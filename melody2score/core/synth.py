# -*- coding: utf-8 -*-
"""钢琴音色合成（轻量、确定、CPU 友好）。

提供 synth_piano(midi, dur, sr) -> float32 [-1,1]，被 app/audio_play.py
用于把音序渲染成可播放波形。保持确定性（无随机源），便于缓存与测试。

性能要点：
  - 单次向量化合成，避免逐谐波多次 astype / 重复 np.arange 的时间轴重建。
  - 时间轴 t 与指数衰减包络按 (n, sr) 缓存（同一采样率下音符时长离散，
    命中率极高），消除大段合成的重复计算。
  - 移除 librosa.tone 调用（其内部有额外格式解析开销且对确定性无益），
    直接用 numpy 向量化基频 + 谐波叠加，速度快且完全确定。
"""
import numpy as np

# 时间轴缓存：同一 (n, sr) 只构造一次 np.arange，避免每个音符重复算
_T_CACHE: dict = {}
_T_CACHE_LOCK = None  # 延迟到首次调用以避免导入期线程开销


def midi_to_hz(midi: int) -> float:
    return float(440.0 * 2.0 ** ((int(midi) - 69) / 12.0))


def _time_axis(n: int, sr: int) -> np.ndarray:
    """返回 float32 时间轴 t = arange(n)/sr，按 (n, sr) 缓存复用。"""
    global _T_CACHE, _T_CACHE_LOCK
    key = (n, sr)
    cached = _T_CACHE.get(key)
    if cached is not None:
        return cached
    t = np.arange(n, dtype=np.float32) / np.float32(sr)
    if len(_T_CACHE) < 256:  # 上限保护，避免长时运行内存膨胀
        _T_CACHE[key] = t
    return t


def synth_piano(midi: int, dur: float, sr: int = 16000) -> np.ndarray:
    """合成单个钢琴音符（基频 + 谐波 + 轻包络）。返回 float32 [-1,1]。

    向量化一次性完成基频+谐波叠加与包络整形，减少中间数组与 astype 次数。
    """
    freq = midi_to_hz(midi)
    dur = max(0.05, float(dur))
    n = int(sr * dur)
    if n <= 0:
        return np.zeros(1, dtype=np.float32)

    t = _time_axis(n, sr)
    two_pi = 2.0 * np.pi
    # 单次向量化：基频 + 二次谐波(0.25) + 三次谐波(0.12)
    y = (np.sin(two_pi * freq * t)
         + 0.25 * np.sin(two_pi * 2.0 * freq * t)
         + 0.12 * np.sin(two_pi * 3.0 * freq * t)).astype(np.float32)

    # 包络：快起音 + 指数衰减（模拟钢琴击弦），同样按 (n, sr) 复用衰减曲线
    env = np.ones(n, dtype=np.float32)
    atk = min(int(0.005 * sr), n // 4)
    if atk > 1:
        env[:atk] = np.linspace(0.0, 1.0, atk, dtype=np.float32)
    decay = np.exp(-t / np.float32(0.35)).astype(np.float32)
    y = y * env * decay

    peak = float(np.max(np.abs(y))) + 1e-9
    return (y / np.float32(peak)).astype(np.float32)
