# -*- coding: utf-8 -*-
"""音频播放工具：原曲回放 + 按音符序列合成钢琴曲。

- play_raw(y, sr): 播放原始波形（用于试听已选 mp3/wav）。
- play_score(notes, sr): 按识别出的音符序列（MIDI/start/end）合成钢琴音色播放。
  钢琴音色用基频 + 若干谐波 + ADSR 包络合成，轻量且不需外部采样。

关键修复（Windows WASAPI/MME 下「点播放没声音」）：
  sounddevice 的 `sd.play(..., blocking=False)` 返回的 Stream 若不被引用，
  会被 Python GC 立即回收，导致播放被截断/静音。本模块改用「持有 stream 引用
  + 独立守护线程阻塞播放」的方式，确保整段播放完整、且可随时 stop()。
"""
import threading

import numpy as np
import sounddevice as sd

SR = 22050
_lock = threading.Lock()
_stream = None          # 当前持有的 OutputStream（防止被 GC）
_stop_ev = threading.Event()  # 主动停止信号（stop() 置位）


def _ensure_float(y: np.ndarray) -> np.ndarray:
    if y.dtype != np.float32 and y.dtype != np.float64:
        y = y.astype(np.float32)
    if y.ndim > 1:
        y = y.mean(axis=1)
    peak = np.max(np.abs(y)) + 1e-9
    return (y / peak).astype(np.float32)


def stop():
    """停止当前播放（置位停止信号并停止底层流）。"""
    _stop_ev.set()
    try:
        sd.stop()
    except Exception:
        pass


def _play_blocking(y: np.ndarray, sr: int):
    """在调用线程里阻塞播放整段 y；stop() 可中途中断。"""
    global _stream
    _stop_ev.clear()
    try:
        with _lock:
            _stream = sd.OutputStream(samplerate=sr, channels=1, dtype="float32")
            _stream.start()
        # 分块写入，便于响应 stop()
        blk = int(0.05 * sr)  # 50ms 一块
        i = 0
        while i < len(y):
            if _stop_ev.is_set():
                break
            chunk = y[i:i + blk]
            try:
                _stream.write(chunk)
            except Exception:
                break
            i += blk
    finally:
        try:
            _stream.stop()
            _stream.close()
        except Exception:
            pass
        with _lock:
            _stream = None


def play_raw(y: np.ndarray, sr: int):
    """播放原始音频（自动重采样到 SR）。非阻塞启动，后台线程负责完整播放。"""
    stop()
    y = _ensure_float(y)
    if sr != SR:
        import librosa
        y = librosa.resample(y, orig_sr=sr, target_sr=SR)
    # 复制到连续数组，避免底层写入时原数组被释放
    y = np.ascontiguousarray(y, dtype=np.float32)
    t = threading.Thread(target=_play_blocking, args=(y, SR), daemon=True)
    t.start()


_HARMONICS = np.array([1.0, 0.55, 0.38, 0.22, 0.14, 0.09, 0.05], dtype=np.float64)
_HARM_K = np.arange(1, len(_HARMONICS) + 1, dtype=np.float64)


def _piano_note(midi: int, dur: float, sr: int = SR) -> np.ndarray:
    """合成单个钢琴音（向量化）：基频 + 谐波加权 + 快起慢落 ADSR。

    用 np.add.outer 一次性生成所有谐波相位，避免逐样本 Python 循环。
    """
    f0 = 440.0 * 2 ** ((midi - 69) / 12.0)
    n = max(1, int(dur * sr))
    # 相位矩阵：shape (n_samples, n_harmonics)，一次运算替代嵌套循环
    t = np.arange(n, dtype=np.float64) / sr
    phase = 2 * np.pi * f0 * (_HARM_K[:, None] * t[None, :])
    sig = (_HARMONICS[:, None] * np.sin(phase)).sum(axis=0)
    sig /= np.max(np.abs(sig)) + 1e-9

    # ADSR：快起音(5ms) + 释放(60ms) + 轻微自然衰减
    atk = int(0.005 * sr)
    rel = int(0.06 * sr)
    env = np.ones(n, dtype=np.float64)
    if atk < n:
        env[:atk] = np.linspace(0.0, 1.0, atk)
    if n > rel:
        env[-rel:] = np.linspace(1.0, 0.0, rel)
    env *= np.exp(-t * 2.2)
    sig *= env
    return sig.astype(np.float32)


def _synth_score(notes: list, sr: int = SR) -> np.ndarray:
    """后台线程用：把音符序列合成整段钢琴波形（向量化）。"""
    if not notes:
        return np.zeros(1, dtype=np.float32)
    total_dur = float(notes[-1]["end"]) if "end" in notes[-1] else sum(
        float(n.get("dur", 0.3)) for n in notes)
    total_dur = max(total_dur, 0.1) + 0.2
    out = np.zeros(int(total_dur * sr), dtype=np.float32)

    for n in notes:
        m = int(n["midi"])
        if "start" in n and "end" in n:
            s = float(n["start"]); dur = max(0.08, float(n["end"]) - float(n["start"]))
        else:
            s = float(n.get("onset", 0)); dur = max(0.08, float(n.get("dur", 0.3)))
        note = _piano_note(m, dur + 0.08, sr)
        i0 = int(s * sr)
        i1 = min(i0 + len(note), len(out))
        out[i0:i1] += note[: i1 - i0]
    peak = np.max(np.abs(out)) + 1e-9
    out = (out / peak * 0.9).astype(np.float32)
    return np.ascontiguousarray(out, dtype=np.float32)


def play_score(notes: list, sr: int = SR, gap: float = 0.0) -> threading.Thread:
    """在后台线程合成并播放钢琴曲，立即返回，不阻塞调用方（UI 线程）。"""
    def _run():
        out = _synth_score(notes, sr)
        stop()
        _play_blocking(out, sr)
    t = threading.Thread(target=_run, daemon=True)
    t.start()
    return t
