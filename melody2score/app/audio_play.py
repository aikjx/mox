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


def _piano_note(midi: int, dur: float, sr: int = SR) -> np.ndarray:
    """合成单个钢琴音：基频 + 谐波（1:2:3:4:5:6 加权衰减）+ 快起慢落 ADSR。"""
    f0 = 440.0 * 2 ** ((midi - 69) / 12.0)
    n = max(1, int(dur * sr))
    t = np.arange(n) / sr
    harmonics = [1.0, 0.55, 0.38, 0.22, 0.14, 0.09, 0.05]
    sig = np.zeros(n, dtype=np.float64)
    for k, amp in enumerate(harmonics, start=1):
        sig += amp * np.sin(2 * np.pi * f0 * k * t)
    sig /= np.max(np.abs(sig)) + 1e-9

    # ADSR：快起音(5ms) + 衰减 + 持续 + 释放(60ms)
    atk = int(0.005 * sr)
    rel = int(0.06 * sr)
    env = np.ones(n, dtype=np.float64)
    for i in range(min(atk, n)):
        env[i] = i / atk
    if n > rel:
        env[-rel:] = np.linspace(1.0, 0.0, rel)
    # 轻微整体衰减（钢琴自然衰减）
    env *= np.exp(-t * 2.2)
    sig *= env
    return sig.astype(np.float32)


def play_score(notes: list, sr: int = SR, gap: float = 0.0) -> np.ndarray:
    """按音符序列合成整段钢琴曲并播放，返回波形。

    notes: [{'midi','start','end'}...]（start/end 用于保持时序与节拍）。
    无 start/end 时按 dur 顺序拼接。
    """
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
    out = np.ascontiguousarray(out, dtype=np.float32)
    stop()
    t = threading.Thread(target=_play_blocking, args=(out, sr), daemon=True)
    t.start()
    return out
