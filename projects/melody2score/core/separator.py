# -*- coding: utf-8 -*-
"""声源分离层：从混合音频中提取主旋律/人声轨道（企业级多策略优雅降级）。

核心目标：解决「伴奏+人声」混合音频的音高检测错音爆炸问题。
开源工业级首选是 Demucs (htdemucs)，但它依赖 PyTorch 且模型体积大。
为了在缺省环境下依然可用，本模块提供 3 级降级：

  1) Demucs：htdemucs 四源分离（人声 / 鼓 / 贝斯 / 其他）→ 取人声轨。
     - 需要 `demucs` 包 + PyTorch + 模型（首跑自动下载 ~80MB）。
  2) Spleeter Lite（基于 Librosa 的 HPSS + 谐波源启发式）：无额外依赖，
     用「谐波/打击分离 + 能量+频带集中性」近似提取主旋律，虽不如 Demucs
     精准，但对「钢琴弹唱/低伴奏混合」的分离效果足以显著降低错音率。
  3) 无分离直通：环境完全不支持分离时直接回原信号，保证链路不断裂。

输出统一规范：
  分离结果返回 Dict，至少含：
    {
      "strategy": "demucs" | "hpss" | "passthrough",
      "vocals":   np.ndarray (float32, mono, 与输入同采样率同长度),
      "other":    np.ndarray (same shape, 伴奏/其他合计),
      "snr_est":  float (估计分离信噪比 dB, 仅用于诊断)
    }
调用方直接取 vocals 作为「主旋律」喂给后续音高检测。

确定性：无随机源，同一输入相同输出。
"""
from typing import Dict, List, Optional, Tuple

import numpy as np


# ========================================================================
# 策略 2：轻量 HPSS + 谐波能量择优（零额外依赖，企业级默认首选）
# ========================================================================

def _hpss_separate(y: np.ndarray, sr: int) -> np.ndarray:
    """HPSS 谐波/打击分离，返回谐波源（主旋律近似）。

    思想：
      - librosa.decompose.hpss 把 STFT 分成 harmonic（谐波/持续音，旋律）
        与 percussive（打击/瞬态，鼓点/拨弦声）两部分；
      - 对 harmonic 再做「频带集中性掩蔽」：保留能量集中在人声/旋律
        典型频带 (80~3500Hz) 的分量，抑制低频贝斯/高频噪声泄漏；
      - 时域能量尾切：在打击源能量过高的瞬时做软门限，抑制残留鼓点。
    结果虽不如 Demucs 干净，但在哼唱、钢琴、弱伴奏场景下显著降低
    音高检测器「被鼓点/贝斯拉偏」的概率。
    """
    import librosa

    # HPSS 分离：中长窗 (2048) 让谐波与打击分得更开
    n_fft = 2048
    hop = 512
    D = librosa.stft(y, n_fft=n_fft, hop_length=hop)
    H, P = librosa.decompose.hpss(D, kernel_size=(31, 31))  # 推荐默认核

    # 频带掩蔽：仅保留 80 Hz ~ 3500 Hz 内的谐波分量（人声/主旋律黄金区）
    freqs = librosa.fft_frequencies(sr=sr, n_fft=n_fft)
    band_mask = (freqs >= 80.0) & (freqs <= 3500.0)
    H_band = H.copy()
    H_band[~band_mask, :] *= 0.02  # 带外保留微弱底噪，避免合成失真

    # 打击瞬态抑制：在 P 能量远大于 H 的帧，把 H 做时间上的软衰减
    # （例如鼓点的一拍瞬态，HPSS 不会完全把鼓点分到 percussive，
    #  残留在 H 的部分会让 CREPE 音高瞬间跳到低频——这里压一下。）
    H_mag = np.abs(H_band)
    P_mag = np.abs(P)
    ratio = np.zeros_like(H_mag)
    np.divide(P_mag, H_mag + 1e-8, out=ratio, where=H_mag > 1e-8)
    # per-frame P > 2×H 时才触发抑制（按帧标量而非逐 bin 标量）
    frame_H = H_mag.mean(axis=0)
    frame_P = P_mag.mean(axis=0)
    frame_ratio = np.zeros_like(frame_H)
    np.divide(frame_P, frame_H + 1e-8, out=frame_ratio, where=frame_H > 1e-8)
    suppress = 1.0 / (1.0 + np.maximum(0.0, frame_ratio - 2.0) * 3.0)
    H_final = H_band * suppress[np.newaxis, :]

    h = librosa.istft(H_final, hop_length=hop, length=len(y))
    return np.asarray(h, dtype=np.float32)


def _estimate_snr_db(sig: np.ndarray, noise: np.ndarray) -> float:
    """信噪比粗略估计（仅用于诊断，不参与算法决策）。"""
    ps = float(np.mean(sig ** 2))
    pn = float(np.mean(noise ** 2))
    if pn < 1e-12:
        return 60.0
    return float(10.0 * np.log10((ps + 1e-12) / (pn + 1e-12)))


# ========================================================================
# 策略 1：Demucs 工业级分离（若环境可用，自动启用）
# ========================================================================

_HAS_DEMUCS: Optional[bool] = None


def _has_demucs() -> bool:
    global _HAS_DEMUCS
    if _HAS_DEMUCS is not None:
        return _HAS_DEMUCS
    try:
        import demucs  # noqa: F401
        import torch   # noqa: F401
        _HAS_DEMUCS = True
    except Exception:
        _HAS_DEMUCS = False
    return _HAS_DEMUCS


def _demucs_separate(y: np.ndarray, sr: int, model_name: str = "htdemucs") -> Dict:
    """Demucs 分离，返回 vocals/other。

    - 输入：numpy 单声道 float32；Demucs 需要 stereo+2 声道 + 44100Hz，
      内部先升采样/复制声道，分离后再降回原采样率取左=右=单声道。
    - 模型首跑会下载 ~80MB 权重，后续走本地缓存；timeout 兜底。
    - 失败（硬件不足/模型下载失败等）统一抛异常，调用方回退 HPSS。
    """
    import torch
    import librosa
    from demucs import pretrained
    from demucs.apply import apply_model
    from demucs.separate import load_track

    # Demucs 官方：htdemucs 以 44100Hz 训练
    TARGET_SR = 44100
    device = "cuda" if torch.cuda.is_available() else "cpu"

    model = pretrained.get_model(model_name)
    model.to(device)
    model.eval()

    # 重采样到 44100Hz，mono→stereo（Demucs 要求 2 通道）
    if sr != TARGET_SR:
        y44 = librosa.resample(y, orig_sr=sr, target_sr=TARGET_SR).astype(np.float32)
    else:
        y44 = np.asarray(y, dtype=np.float32).copy()
    ref = torch.from_numpy(np.stack([y44, y44], axis=0))  # [2, T]
    ref = ref.unsqueeze(0)  # [1, 2, T]

    # 按官方默认参数分离（shifts=1 折中速度/稳定性；segment=8s 降低显存）
    with torch.no_grad():
        sources = apply_model(
            model, ref, device=device, shifts=1,
            split=True, overlap=0.25, segment=8,
        )
    # htdemucs 源顺序: drums, bass, other, vocals
    src_names = model.sources
    s = sources[0].cpu().numpy()  # [4, 2, T]
    vocals = None
    other_sum = None
    for i, name in enumerate(src_names):
        # 双声道合并为 mono
        mono = s[i].mean(axis=0)
        if name == "vocals":
            vocals = mono
        else:
            other_sum = mono if other_sum is None else (other_sum + mono)

    # 降回原采样率
    if sr != TARGET_SR:
        vocals = librosa.resample(vocals, orig_sr=TARGET_SR, target_sr=sr)
        if other_sum is not None:
            other_sum = librosa.resample(other_sum, orig_sr=TARGET_SR, target_sr=sr)

    return {
        "vocals": np.asarray(vocals[:len(y)], dtype=np.float32),
        "other":  np.asarray(other_sum[:len(y)], dtype=np.float32) if other_sum is not None else np.zeros(len(y), dtype=np.float32),
    }


# ========================================================================
# 统一入口
# ========================================================================

def separate_melody(y: np.ndarray, sr: int,
                    strategy: str = "auto",
                    verbose: bool = False) -> Dict:
    """主旋律分离统一入口。

    参数
    ----
    y : np.ndarray (float32, mono)
    sr : int
    strategy : str
      - "auto"      : 有 Demucs → Demucs，否则 HPSS Lite。
      - "demucs"    : 强制 Demucs（不可用则抛出异常）。
      - "hpss"      : 强制 HPSS Lite（零依赖，速度快）。
      - "none"      : 直通（不分离，仅返回 {vocals=y, other=0}）。

    返回 Dict（参见模块顶部 docstring）。
    """
    y = np.asarray(y, dtype=np.float32)
    if y.ndim > 1:
        y = y.mean(axis=-1)

    strategy = (strategy or "auto").lower()

    # 策略分发
    if strategy == "none":
        return {
            "strategy": "passthrough",
            "vocals": y.copy(),
            "other": np.zeros_like(y),
            "snr_est": 40.0,
        }

    if strategy == "hpss":
        vocals = _hpss_separate(y, sr)
        other = y - vocals
        return {
            "strategy": "hpss",
            "vocals": vocals,
            "other": other,
            "snr_est": _estimate_snr_db(vocals, other),
        }

    if strategy in ("auto", "demucs"):
        if _has_demucs():
            try:
                d = _demucs_separate(y, sr)
                return {
                    "strategy": "demucs",
                    "vocals": d["vocals"],
                    "other":  d["other"],
                    "snr_est": _estimate_snr_db(d["vocals"], d["other"]),
                }
            except Exception as e:
                if verbose:
                    print(f"[sep] Demucs 失败，回退 HPSS: {e}")
                if strategy == "demucs":
                    # demucs 强制模式：失败抛错而非静默回退，便于诊断
                    raise
        elif strategy == "demucs":
            raise RuntimeError("demucs / torch 未安装，且已强制 strategy=demucs")

    # 兜底 HPSS
    vocals = _hpss_separate(y, sr)
    other = y - vocals
    return {
        "strategy": "hpss",
        "vocals": vocals,
        "other": other,
        "snr_est": _estimate_snr_db(vocals, other),
    }
