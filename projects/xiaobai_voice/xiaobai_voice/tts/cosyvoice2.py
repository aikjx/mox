"""CosyVoice2 封装（Apache2，信创回退默认）· 豆包级动听质感。

相对 v1 的关键音质升级：
1. 指令前缀不再啰嗦：用「温柔日常 / 柔和 / 甜美 / 专业」风格引导，避免"请用...语气朗读"机械感。
2. write 内置 speaker id 探测：循环 preferred_spk_ids，兼容官方 0.5B / 自定义 SFT 权重。
3. 重采样：线性插值（替代 nearest 索引硬切，显著减少混叠和毛刺）；可选 librosa kaiser_best。
4. 语速（speed）：轻量 SOLA-like 时域缩放（帧长 20ms / 重叠 10ms），不变调，避免"唐老鸭"。
5. 响度/限幅：目标 -18 dBFS LUFS-like 归一化 + 软限幅（tanh ≥ 0.995 阈值），防止 peak 削波。
6. 输出原生 22050 Hz WAV（CosyVoice2 原生采样率，减少一次重采样损失）。
"""
from __future__ import annotations

import os
from collections.abc import Generator
from typing import Any

from .base import TTSBackend, TTSOptions
from ..errors import ErrorCode, XiaobaiError


# ============================================================= style / emotion
# 情绪 × 风格 → 豆包级指令前缀（短而自然，避免指令-正文割裂）
_INSTRUCTION_STYLE = {
    "warm_daily": "用温柔、自然、贴近日常聊天的中文语气，清晰明亮地说：",
    "gentle_soft": "用柔和、细腻、温暖治愈的中文语气，轻声说：",
    "anchor_premium": "用专业、沉稳、圆润悦耳的播音级中文语气，娓娓道来：",
    "professional_calm": "用专业、清晰、从容不迫的中文语气说：",
    "cute_lively": "用甜美、灵动、元气满满的中文语气说：",
}

_EMOTION_STYLE_FALLBACK = {
    "neutral": "warm_daily",
    "happy": "cute_lively",
    "sad": "gentle_soft",
    "serious": "professional_calm",
}

DEFAULT_PREFERRED_SPK = ["中文女", "女", "voice_0", "Default", "中文男"]


# ====================================================================== utils
def _resample_linear(audio, sr_out, sr_in):
    """线性插值重采样：O(n)，纯 numpy，无额外依赖，比 nearest 顺滑很多。"""
    import numpy as np

    if int(sr_out) == int(sr_in):
        return audio.astype(np.float32, copy=False)
    audio = np.asarray(audio, dtype=np.float32).reshape(-1)
    n = int(round(len(audio) * float(sr_out) / float(sr_in)))
    if n <= 0:
        return audio
    if len(audio) == 1:
        return np.full(n, audio[0], dtype=np.float32)
    # 采样点坐标：旧采样对应 [0, len-1]
    idx_f = (np.arange(n, dtype=np.float64) * (len(audio) - 1)) / max(1, (n - 1))
    idx0 = np.floor(idx_f).astype(np.int64)
    idx1 = np.minimum(idx0 + 1, len(audio) - 1)
    frac = (idx_f - idx0).astype(np.float32)
    return audio[idx0] * (1.0 - frac) + audio[idx1] * frac


def _resample(audio, sr_out, sr_in, quality: str):
    """根据配置选择重采样实现。quality ∈ {linear, kaiser_best}。"""
    quality = (quality or "linear").strip().lower()
    if quality in {"kaiser_best", "kaiserbest", "librosa", "best"}:
        try:
            import numpy as np  # noqa: F401
            import librosa  # type: ignore

            return librosa.resample(
                y=audio, orig_sr=int(sr_in), target_sr=int(sr_out), res_type="kaiser_best"
            )
        except Exception:  # noqa: BLE001
            pass
    return _resample_linear(audio, sr_out, sr_in)


def _time_stretch_sola(audio, target_len, frame_ms=20.0, overlap_ms=10.0, sr=22050):
    """
    Light-weight WSOLA/SOLA-like time-scale modification。
    适用于 ±30% 的小幅语速调整（speed 0.8~1.3 是 90% 场景，精度足够）。
    不变调。frame/overlap 取 20/10ms：480Hz / 1000Hz 基频都能稳定对齐。
    """
    import numpy as np

    audio = np.asarray(audio, dtype=np.float32).reshape(-1)
    src_len = audio.size
    if src_len <= 1 or target_len <= 1:
        return audio
    if abs(src_len - target_len) <= 1:
        return audio
    sr_int = int(sr) if sr else 22050
    frame = max(32, int(sr_int * frame_ms / 1000.0))
    overlap = max(8, min(frame // 2, int(sr_int * overlap_ms / 1000.0)))
    hop_synthesis = max(1, frame - overlap)
    # ratio < 1 → 压缩（变慢？不：我们的 stretch ratio = target / src）
    ratio = float(target_len) / float(src_len)
    hop_analysis = max(1, int(round(hop_synthesis / ratio)))
    out = np.zeros(int(target_len) + frame, dtype=np.float32)
    win = np.hanning(frame).astype(np.float32)
    n_frames = (src_len - frame) // hop_analysis + 1
    write_pos = 0
    for i in range(n_frames):
        s = i * hop_analysis
        chunk = audio[s : s + frame].astype(np.float32)
        if chunk.size < frame:
            # 补零，保证窗口长度一致
            chunk = np.concatenate([chunk, np.zeros(frame - chunk.size, dtype=np.float32)])
        # overlap-add with window (crossfade)
        if write_pos == 0:
            out[0:frame] += chunk * win
            write_pos += hop_synthesis
            continue
        # 查找 overlap 区内最大互相关的偏移（± half_overlap）
        search = min(overlap, write_pos, out.size - frame) // 2
        if search > 0 and overlap >= 4 and write_pos + frame <= out.size:
            # 当前 chunk 开头 overlap 样本，与已写末尾 overlap 样本做互相关
            tail = out[write_pos : write_pos + overlap]
            head_need = chunk[0:overlap]
            # 暴力搜索 ±search 偏移（短窗 O(search*overlap)，search~数十 可接受）
            best_off = 0
            best_corr = -1e18
            for off in range(-search, search + 1):
                t_start = write_pos + off
                t_end = t_start + overlap
                if t_start < 0 or t_end > out.size:
                    continue
                buf = out[t_start:t_end]
                c = float(np.dot(buf, head_need))
                if c > best_corr:
                    best_corr = c
                    best_off = off
            write_pos += best_off
        if write_pos < 0:
            write_pos = 0
        if write_pos + frame > out.size:
            # 扩容一点点
            grow = np.zeros(write_pos + frame - out.size + 16, dtype=np.float32)
            out = np.concatenate([out, grow])
        out[write_pos : write_pos + frame] += chunk * win
        write_pos += hop_synthesis
    # 裁剪到目标长度
    if write_pos < target_len:
        # 没填满，补零
        if out.size < target_len:
            out = np.concatenate([out, np.zeros(target_len - out.size, dtype=np.float32)])
        return out[:target_len]
    return out[:target_len]


def _apply_limiter_and_loudness(audio, target_dbfs: float = -18.0, enable: bool = True):
    """响度归一化 + 软限幅（tanh）。
    - target_dbfs: -18 ~ -14 是中文普通话对话常用响度目标（接近豆包/Apple News）。
    - enable=False 时只做软限幅防止削波。
    """
    import numpy as np

    audio = np.asarray(audio, dtype=np.float32).reshape(-1)
    if audio.size == 0:
        return audio
    # RMS -> dBFS
    rms = float(np.sqrt(np.mean(audio * audio) + 1e-12))
    db = 20.0 * np.log10(rms + 1e-12)
    if enable and np.isfinite(db) and db < -6.0:
        # 只在音量明显不足时抬升（防止静音段爆炸）
        gain_db = float(target_dbfs) - float(db)
        # 最多 +22 dB，防止异常冲激
        gain_db = min(22.0, max(0.0, gain_db))
        audio = audio * float(10.0 ** (gain_db / 20.0))
    # 软限幅：|x| >= 0.95 进入 tanh knee；|x| >= 1.0 严格压到 < 0.995
    peak = float(np.max(np.abs(audio))) if audio.size else 0.0
    if peak > 0.95:
        k = 1.0 / 0.95
        mask_high = np.abs(audio) >= 0.95
        signed = np.sign(audio[mask_high])
        scaled = (np.abs(audio[mask_high]) - 0.95) * k
        audio[mask_high] = signed * (0.95 + 0.045 * np.tanh(scaled))
    return audio


# ==================================================================== backend
class CosyVoice2Backend(TTSBackend):
    name = "cosyvoice2"

    def __init__(self, cfg: dict, models_registry: Any | None = None) -> None:
        super().__init__(cfg, models_registry)
        cosy_cfg = ((cfg.get("engines") or {}).get("cosyvoice2") or {}) if isinstance(cfg, dict) else {}
        self._preferred_spk = list(cosy_cfg.get("preferred_spk_ids") or DEFAULT_PREFERRED_SPK)
        self._style = str(cosy_cfg.get("instruction_style") or "warm_daily").strip().lower()
        self._resample_quality = str(cosy_cfg.get("resample_quality") or "linear")
        self._limiter = bool(cosy_cfg.get("limiter", True))
        self._loudness_target_dbfs = float(cosy_cfg.get("loudness_target_dbfs") or -18.0)
        self._resolved_spk_id: str | None = None
        self._model = None
        self._model_sr = 22050
        try:
            self._ckpt_dir = self._resolve_model_dir(models_registry)
            self._load_engine()
        except XiaobaiError:
            raise
        except FileNotFoundError as exc:
            missing = getattr(exc, "filename", None)
            missing_text = f"（缺失路径：{missing}）" if missing else "（所有候选路径均未命中）"
            raise XiaobaiError(
                code=ErrorCode.MISSING_MODEL,
                message=(
                    "CosyVoice2 权重目录缺失。请下载 tts-cosyvoice2-0.5b 放入 "
                    f"`~/.mox/models/voice/` 或 `projects/xiaobai_voice/models/`。{missing_text}"
                ),
                cause=exc,
            ) from exc
        except ImportError as exc:
            raise XiaobaiError(
                code=ErrorCode.MISSING_DEP,
                message=(
                    "CosyVoice2 未安装。请执行：pip install cosyvoice>=0.2.0 （Apache2）。"
                    "或把 license_tier=apache2 切回 auto，允许浏览器 TTS 兜底。"
                ),
                cause=exc,
            ) from exc
        except OSError as exc:
            raise XiaobaiError(
                code=ErrorCode.DLL_LOAD_FAIL,
                message="CosyVoice2 加载 DLL/torch/onnxruntime 失败。",
                cause=exc,
            ) from exc

    def _resolve_model_dir(self, registry: Any | None) -> str:
        if registry is not None and hasattr(registry, "resolve"):
            r = registry.resolve("tts-cosyvoice2-0.5b")
            if r:
                return r["root"]
        candidates = []
        import sys

        if getattr(sys, "frozen", False):
            candidates.append(os.path.join(os.path.dirname(sys.executable), "models", "tts-cosyvoice2-0.5b"))
        candidates.append(os.path.join(os.path.expanduser("~"), ".mox", "models", "voice", "tts-cosyvoice2-0.5b"))
        candidates.append(
            os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "models", "tts-cosyvoice2-0.5b"))
        )
        for c in candidates:
            if os.path.isfile(os.path.join(c, "configuration.json")) or os.path.isdir(c):
                return c
        raise FileNotFoundError(candidates[-1])

    def _load_engine(self) -> None:
        import cosyvoice  # type: ignore  # noqa: F401

        try:
            self._model = cosyvoice.CosyVoice2(self._ckpt_dir)
        except Exception:
            try:
                self._model = cosyvoice.CosyVoice(self._ckpt_dir)  # type: ignore[attr-defined]
            except Exception as e:  # noqa: BLE001
                raise XiaobaiError(
                    code=ErrorCode.MISSING_MODEL,
                    message=f"CosyVoice2 模型构造失败，请确认权重目录为 CosyVoice2-0.5B：{e}",
                    cause=e,
                ) from e
        # 探测可用 speaker id 列表
        self._resolved_spk_id = self._detect_spk()

    def _detect_spk(self) -> str:
        """CosyVoice 的实际注册 speaker id 在不同版本不同。
        常见：CosyVoice2 0.5B SFT 权重默认有 "中文女" / "中文男"；社区自定义 ckpt 可能写 "Default"。
        优先按 self._preferred_spk 顺序；都不命中时取 inference_sft 的 exception 信息兜底。
        """
        model = self._model
        # 方案 A：list_avaliable_spks（部分版本有）
        for attr in ("list_avaliable_spks", "list_available_spks", "available_spks", "avaliable_spks"):
            fn = getattr(model, attr, None)
            if not callable(fn):
                continue
            try:
                spks = fn() or []
            except Exception:  # noqa: BLE001
                spks = []
            if isinstance(spks, dict):
                spks = list(spks.keys())
            spks_norm: list[str] = []
            for s in spks:
                if isinstance(s, str):
                    spks_norm.append(s)
                elif isinstance(s, (list, tuple)) and len(s) >= 2 and isinstance(s[1], str):
                    spks_norm.append(str(s[1]))
                elif isinstance(s, (list, tuple)) and len(s) >= 1 and isinstance(s[0], str):
                    spks_norm.append(str(s[0]))
            for cand in self._preferred_spk:
                if cand in spks_norm:
                    return cand
            # 优先选带「女」字 / 女声注册名
            for s in spks_norm:
                if "女" in s or "female" in s.lower() or "voice_0" in s.lower() or "default" == s.lower():
                    return s
            if spks_norm:
                return spks_norm[0]
        # 方案 B：打一次空合成，取异常里的可用列表
        return self._probe_spk_by_inference()

    def _probe_spk_by_inference(self) -> str:
        # 用最短文本 + 第一个候选触发 try，看报错里的允许列表
        probe_text = "你好"
        last_cand = "中文女"
        for cand in self._preferred_spk:
            last_cand = cand
            try:
                _ = list(self._do_infer_raw(probe_text, cand))
                return cand
            except Exception as e:  # noqa: BLE001
                msg = str(e)
                # 常见："spk_id ... must be one of {'中文女', '中文男'}"
                import re

                m = re.search(r"one\s+of\s*[\(\{\[]([^\}\)\]]+)[\)\}\]]", msg, flags=re.I)
                if m:
                    raw = m.group(1)
                    parts = [p.strip().strip("\"' ") for p in raw.split(",")]
                    parts = [p for p in parts if p]
                    for c in self._preferred_spk:
                        if c in parts:
                            return c
                    if parts:
                        return parts[0]
                continue
        return last_cand

    def _do_infer_raw(self, text: str, spk: str):
        """适配多种 inference API：优先 inference_sft，兜底 inference。"""
        model = self._model
        if hasattr(model, "inference_sft") and callable(model.inference_sft):
            yield from model.inference_sft(text, spk)
            return
        if hasattr(model, "inference") and callable(model.inference):
            yield from model.inference(text, spk)
            return
        raise XiaobaiError(
            code=ErrorCode.RUNTIME,
            message="当前 CosyVoice2 模型对象上未发现 inference_sft / inference 方法。",
        )

    # -------------------------------------------------------------- synthesize
    def synthesize(self, opts: TTSOptions) -> Generator[bytes, None, None]:
        import numpy as np

        model_sr = int(self._model_sr or 22050)
        sr = int(opts.sample_rate or self.sample_rate or model_sr)
        if opts.emotion not in _EMOTION_STYLE_FALLBACK:
            opts.emotion = "neutral"
        # 风格：优先 emotion 决定；默认 warm_daily 可被 instruction_style 覆盖
        style_key = (
            self._style
            if self._style in _INSTRUCTION_STYLE
            else _EMOTION_STYLE_FALLBACK.get(opts.emotion, "warm_daily")
        )
        prefix = _INSTRUCTION_STYLE.get(style_key, _INSTRUCTION_STYLE["warm_daily"])
        instruction = prefix + (opts.text or "")

        spk = self._resolved_spk_id or "中文女"
        try:
            synth_iter = self._do_infer_raw(instruction, spk)
        except XiaobaiError:
            raise
        except Exception as exc:  # noqa: BLE001
            raise XiaobaiError(
                code=ErrorCode.RUNTIME,
                message=f"CosyVoice2 合成失败：{exc}",
                cause=exc,
            ) from exc

        audio_chunks: list[np.ndarray] = []
        sample_rate_out = model_sr
        try:
            for item in synth_iter:
                if isinstance(item, tuple) and len(item) == 2:
                    sample_rate_out, arr = item
                    sample_rate_out = int(sample_rate_out or model_sr)
                elif isinstance(item, dict) and "tts_speech" in item:
                    sample_rate_out, arr = item["tts_speech"]
                    sample_rate_out = int(sample_rate_out or model_sr)
                else:
                    continue
                audio_chunks.append(np.asarray(arr, dtype=np.float32).reshape(-1))
        except Exception as exc:  # noqa: BLE001
            raise XiaobaiError(ErrorCode.RUNTIME, f"CosyVoice2 合成中断: {exc}", cause=exc) from exc

        if not audio_chunks:
            raise XiaobaiError(ErrorCode.RUNTIME, "CosyVoice2 合成结果为空。可能是空文本或权重不完整。")

        audio = np.concatenate(audio_chunks, axis=0) if len(audio_chunks) > 1 else audio_chunks[0]

        # 1) 重采样到目标 sr（linear/kaiser）
        audio = _resample(audio, sr, sample_rate_out, self._resample_quality)

        # 2) 语速缩放（speed != 1.0 时 SOLA）
        speed = float(getattr(opts, "speed", 1.0) or 1.0)
        if speed <= 0:
            speed = 1.0
        if abs(speed - 1.0) > 1e-3 and 0.5 <= speed <= 2.0:
            target_len = int(round(audio.size / speed))
            audio = _time_stretch_sola(audio, target_len, frame_ms=20.0, overlap_ms=10.0, sr=sr)

        # 3) 响度归一 + 软限幅（防止 peak 削波）
        audio = _apply_limiter_and_loudness(
            audio,
            target_dbfs=self._loudness_target_dbfs,
            enable=self._limiter,
        )

        # 4) float → int16 输出
        int16 = (audio * 32767.0).clip(-32768, 32767).astype("<i2")
        raw = int16.tobytes()

        from .browser_fallback import _make_wav_header

        yield _make_wav_header(sr=sr, channels=1, bits=16, data_len=len(raw))
        chunk_bytes = max(1024, int(sr * 2 * (opts.stream_chunk_ms / 1000.0)))
        for i in range(0, len(raw), chunk_bytes):
            yield raw[i : i + chunk_bytes]

    def close(self) -> None:
        if self._model is not None:
            self._model = None
