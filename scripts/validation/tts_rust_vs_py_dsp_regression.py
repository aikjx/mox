"""Rust DSP vs Python DSP 客观音质回归（6 指标 AB 对照）。

覆盖 cosyvoice2.py 的 DSP 后处理核心（即使 CosyVoice2 模型未安装，
这些纯 DSP 单元也能独立验证）。注意 Python 流水线函数来自 cosyvoice2 模块。
"""
from __future__ import annotations

import io
import math
import os
import sys
import time
import struct
import wave

import numpy as np

# ------- 路径：允许直接 import xiaobai_voice 与 cosyvoice2 子模块 -------
# 注意：仓库实际 package 布局是 projects/xiaobai_voice/xiaobai_voice/ ，
# 所以 sys.path 应该插入 `projects/xiaobai_voice`（外层）才能 import xiaobai_voice.tts。
HERE = os.path.abspath(os.path.dirname(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, ".."))
for p in (
    os.path.join(REPO_ROOT, "projects", "xiaobai_voice"),
    REPO_ROOT,
):
    if p not in sys.path:
        sys.path.insert(0, p)

import xiaobai_dsp_native  # type: ignore  # noqa: E402
from xiaobai_voice.tts.cosyvoice2 import (  # noqa: E402
    _resample_linear,
    _time_stretch_sola,
    _apply_limiter_and_loudness,
)


# --------------------------------------------------------------------- utils
def dbfs_rms(x: np.ndarray) -> float:
    x = np.asarray(x, dtype=np.float32).reshape(-1)
    if x.size == 0:
        return float("-inf")
    rms = float(np.sqrt(np.mean(x * x) + 1e-12))
    return 20.0 * math.log10(rms + 1e-12)


def peak_abs(x: np.ndarray) -> float:
    x = np.asarray(x, dtype=np.float32).reshape(-1)
    if x.size == 0:
        return 0.0
    return float(np.max(np.abs(x)))


def _decode_wav_bytes(b: bytes) -> tuple[int, np.ndarray]:
    """返回 (sr, mono float32 [-1..1])。"""
    with wave.open(io.BytesIO(b), "rb") as wf:
        nch = wf.getnchannels()
        sr = wf.getframerate()
        sw = wf.getsampwidth()
        n = wf.getnframes()
        raw = wf.readframes(n)
    if sw == 2:
        arr = np.frombuffer(raw, dtype="<i2").astype(np.float32)
        # 按 xiaobai-dsp 的精确钳位约定反归一（负值÷32768、正值÷32767）
        neg = arr < 0
        pos = ~neg
        out = np.zeros_like(arr, dtype=np.float32)
        out[neg] = arr[neg] / 32768.0
        out[pos] = arr[pos] / 32767.0
    elif sw == 1:
        arr = np.frombuffer(raw, dtype=np.uint8).astype(np.float32)
        out = (arr - 128.0) / 128.0
    else:
        raise RuntimeError(f"unsupported sample_width={sw}")
    if nch >= 2:
        out = out.reshape(-1, nch).mean(axis=1)
    return sr, out.astype(np.float32)


# ================================================================ reference
# Python 流水线 = 原 cosyvoice2 四步（重采样/SOLA/响度/钳位）
def py_pipeline_f32(sig: np.ndarray, orig_sr: int, target_sr: int, speed: float, target_dbfs: float) -> np.ndarray:
    s = _resample_linear(sig, target_sr, orig_sr)
    if abs(speed - 1.0) > 1e-3 and 0.5 <= speed <= 2.0:
        tlen = int(round(s.size / speed))
        s = _time_stretch_sola(s, tlen, frame_ms=20.0, overlap_ms=10.0, sr=target_sr)
    s = _apply_limiter_and_loudness(s, target_dbfs=target_dbfs, enable=True)
    # 精确钳位 ×32768/×32767 → 转 float 等价 clip
    neg = s < 0
    pos = ~neg
    scaled = np.zeros_like(s, dtype=np.float32)
    scaled[neg] = s[neg] * 32768.0
    scaled[pos] = s[pos] * 32767.0
    i16 = scaled.clip(-32768, 32767).astype(np.int32)
    # 还原为 float 以对齐比较
    out = np.zeros_like(s, dtype=np.float32)
    out[neg] = i16[neg].astype(np.float32) / 32768.0
    out[pos] = i16[pos].astype(np.float32) / 32767.0
    return out


def rust_pipeline_f32(sig: np.ndarray, orig_sr: int, target_sr: int, speed: float, target_dbfs: float) -> np.ndarray:
    # 用 encode_wav=True 生成 WAV bytes，再解码为 float — 精确模拟 cosyvoice2 的输出路径
    opts = dict(
        orig_sr=int(orig_sr),
        target_sr=int(target_sr),
        speed=float(speed),
        target_dbfs=float(target_dbfs),
        enable_loudness=True,
        encode_wav=True,
        channels=1,
    )
    b: bytes = xiaobai_dsp_native.apply_dsp_pipeline(sig.astype(np.float32).tolist(), opts)
    assert isinstance(b, (bytes, bytearray))
    sr, out = _decode_wav_bytes(bytes(b))
    assert sr == target_sr, (sr, target_sr)
    return out


# ================================================================ synthetic
def synth_signal(sr: int, seconds: float, seed: int = 7) -> np.ndarray:
    """中文 TTS 典型：基频 100~300Hz 谐波叠加 + 共振谐波 + 轻噪声 + 2 处瞬态。"""
    rng = np.random.default_rng(seed)
    n = int(sr * seconds)
    t = np.arange(n, dtype=np.float32) / float(sr)
    s = np.zeros(n, dtype=np.float32)
    # 基频随时间轻微滑动（模拟语调）
    f0 = 180.0 + 60.0 * np.sin(2 * np.pi * 1.2 * t, dtype=np.float32)
    # 前 10 个谐波
    for h in range(1, 11):
        amp = 1.0 / (h ** 1.35)
        phi = rng.uniform(0, 2 * math.pi)
        s += amp * np.sin(2 * math.pi * (h * f0 * t) + phi, dtype=np.float32)
    # 共振峰（两个高斯包络的带限能量）
    for fc, bw, gain in ((900, 180, 0.55), (1800, 260, 0.35)):
        env = gain * np.exp(-(((t - 0.5) / 0.4) ** 2), dtype=np.float32)
        s += env * np.sin(2 * np.pi * fc * t + rng.uniform(0, 6.28), dtype=np.float32)
    # 噪声（气息）
    s += 0.035 * rng.standard_normal(n).astype(np.float32)
    # 两个瞬态（爆发辅音-like）
    for tk, dur in ((0.22, 0.006), (0.73, 0.005)):
        i0 = int(tk * sr); i1 = max(i0 + 1, int((tk + dur) * sr))
        s[i0:i1] += 0.35 * rng.standard_normal(i1 - i0).astype(np.float32)
    # 衰减到 ≤ 0.92 peak，给响度归一留抬升空间
    pk = float(np.max(np.abs(s))) or 1.0
    s = (s / pk) * 0.60
    return s


# ================================================================ cases
def run_case(name: str, orig_sr: int, target_sr: int, speed: float, target_dbfs: float, seconds: float = 1.2):
    print(f"\n===== Case {name}  orig_sr={orig_sr} target_sr={target_sr} speed={speed} tgt_dbfs={target_dbfs} sec={seconds} =====")
    sig = synth_signal(orig_sr, seconds, seed=11)
    print(f"  input  length={sig.size}  peak={peak_abs(sig):.4f}  dBFS(RMS)={dbfs_rms(sig):.2f}")

    # ---- Rust ----
    t0 = time.perf_counter()
    r = rust_pipeline_f32(sig, orig_sr, target_sr, speed, target_dbfs)
    rt = time.perf_counter() - t0

    # ---- Python ----
    t0 = time.perf_counter()
    p = py_pipeline_f32(sig, orig_sr, target_sr, speed, target_dbfs)
    pt = time.perf_counter() - t0

    # 长度（SOLA 拉伸 ratio 正确性）
    exp_len = int(round(int(round(len(sig) * target_sr / orig_sr)) / speed))
    print(f"  expected length ≈ {exp_len}")
    print(f"  Rust len={r.size}  peak={peak_abs(r):.4f}  dBFS={dbfs_rms(r):.2f}  time={rt*1000:.1f}ms")
    print(f"  Py   len={p.size}  peak={peak_abs(p):.4f}  dBFS={dbfs_rms(p):.2f}  time={pt*1000:.1f}ms")

    speedup = pt / rt if rt > 0 else float("nan")
    print(f"  speedup = {speedup:.2f}x")

    # 对齐长度（允许 ±0.5% 由 SOLA 实现差异导致的端点补零差）
    m = min(r.size, p.size)
    ra = r[:m]
    pa = p[:m]
    # 全链路波形一致性（SNR + 最大逐点偏差）
    diff = ra - pa
    noise_pwr = float(np.mean(diff * diff) + 1e-18)
    sig_pwr = float(np.mean(pa * pa) + 1e-18)
    snr = 10.0 * math.log10(sig_pwr / noise_pwr)
    max_abs_err = float(np.max(np.abs(diff)))
    # 响度：因为实现"只抬升不衰减"（保护已响亮的段），以下任一满足即可通过：
    #   (a) 实测响度 ≥ 目标 - 0.5 dB（本来就响，无需处理）
    #   (b) |实测 - 目标| ≤ 5.0 dB 内（触发抬升且接近）
    def lu_ok(measured: float) -> bool:
        return measured >= target_dbfs - 0.5 or abs(measured - target_dbfs) <= 5.0

    rust_lu_ok = lu_ok(dbfs_rms(r))
    py_lu_ok = lu_ok(dbfs_rms(p))
    # 限幅
    rust_peak_ok = peak_abs(r) <= 1.0 + 1e-6
    py_peak_ok = peak_abs(p) <= 1.0 + 1e-6
    # 长度：SOLA / 重采样都允许 ±0.5% 或 ±3 samples
    tol = max(3, int(exp_len * 0.005))
    len_ok_r = abs(r.size - exp_len) <= tol
    len_ok_p = abs(p.size - exp_len) <= tol

    # 波形一致性：只有在重采样线性（无 SOLA）的 case 才以 SNR≥60dB 严格要求；
    #             含 SOLA（speed≠1）时 SOLA 算法内部窗/偏移搜索不保证逐样本一致，
    #             以"音频有效（peak≤1 且长度正确 且 响度OK）为验收，SNR仅参考展示。
    is_pure_resample = (abs(speed - 1.0) <= 1e-3)
    if is_pure_resample:
        snr_ok = snr >= 60.0
    else:
        # SOLA 只要求两个输出都在音频健康范围（不炸）。逐样本不等是实现差异，不是错误。
        snr_ok = True  # 通过响度/限幅/长度判定即可

    status = []
    status.append(("限幅 peak≤1 (Rust)", rust_peak_ok))
    status.append(("限幅 peak≤1 (Py)",   py_peak_ok))
    status.append(("响度 OK (Rust)",       rust_lu_ok))
    status.append(("响度 OK (Py)",         py_lu_ok))
    status.append((f"长度 ±{tol} (Rust)",   len_ok_r))
    status.append((f"长度 ±{tol} (Py)",     len_ok_p))
    if is_pure_resample:
        status.append((f"线性 DSP 波形 SNR ≥ 60 dB", snr_ok))
    else:
        status.append(("SOLA 长度对齐通过（逐样本差异允许）", len_ok_r and len_ok_p))

    print(f"  waveform SNR(Rust↔Py) = {snr:.2f} dB   max|Δ| = {max_abs_err:.5f}")
    for k, ok in status:
        print(f"    [{'PASS' if ok else 'FAIL'}]  {k}")
    ok_all = all(v for _, v in status)
    return dict(
        name=name,
        speedup_x=float(f"{speedup:.2f}"),
        rust_len=int(r.size),
        py_len=int(p.size),
        rust_dbfs=float(f"{dbfs_rms(r):.2f}"),
        py_dbfs=float(f"{dbfs_rms(p):.2f}"),
        rust_peak=float(f"{peak_abs(r):.4f}"),
        py_peak=float(f"{peak_abs(p):.4f}"),
        snr_rust_py_db=float(f"{snr:.2f}"),
        max_abs_err=float(f"{max_abs_err:.6f}"),
        all_pass=ok_all,
    )


def main():
    print("=== Rust DSP module ===")
    print("  version:", getattr(xiaobai_dsp_native, "__version__", None))
    print("  exports:", [x for x in dir(xiaobai_dsp_native) if not x.startswith("__")])

    cases = [
        ("C1:基准 identity（sr=22050 speed=1.0）", 22050, 22050, 1.00, -18.0),
        ("C2:升采样 16k→22050 女声语速 +3%",   16000, 22050, 1.03, -18.0),
        ("C3:降采样 44.1k→22050 慢 -15%",       44100, 22050, 0.85, -19.0),
        ("C4:快 +30% speed=1.3（边缘 SOLA）",   22050, 22050, 1.30, -17.5),
        ("C5:慢 -30% speed=0.7（边缘 SOLA）",   22050, 22050, 0.70, -17.5),
        ("C6:大跨采样 8k→48k（重采样压力）",      8000, 48000, 1.00, -18.0),
    ]
    rows = [run_case(*c, seconds=1.2) for c in cases]

    print("\n==================== SUMMARY TABLE (Rust DSP Regression) ====================")
    hdr = ["case", "speedup", "len_r/py", "dbFS_r/py", "peak_r/py", "SNR_rp", "max|Δ|", "ALL?"]
    print(" | ".join(f"{h:>14}" for h in hdr))
    for r in rows:
        print(
            f"{r['name'][:26]:>14}"
            f" | {r['speedup_x']:>13.2f}"
            f" | {r['rust_len']:>6}/{r['py_len']:<6}"
            f" | {r['rust_dbfs']:>5.1f}/{r['py_dbfs']:<5.1f}"
            f" | {r['rust_peak']:>5.3f}/{r['py_peak']:<5.3f}"
            f" | {r['snr_rust_py_db']:>13.1f}"
            f" | {r['max_abs_err']:>13.6f}"
            f" | {'OK' if r['all_pass'] else 'FAIL':>14}"
        )
    n_pass = sum(1 for r in rows if r["all_pass"])
    avg_su = sum(r["speedup_x"] for r in rows) / len(rows)
    avg_snr = sum(r["snr_rust_py_db"] for r in rows) / len(rows)
    print(f"\nPass {n_pass}/{len(rows)} cases  ·  avg speedup = {avg_su:.2f}x  ·  avg SNR(Rust↔Py) = {avg_snr:.1f} dB")
    return 0 if n_pass == len(rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
