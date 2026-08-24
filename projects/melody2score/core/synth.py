# -*- coding: utf-8 -*-
"""钢琴音色合成（轻量、确定、CPU 友好 · V2 线程安全+短音包络版）。

提供 synth_piano(midi, dur, sr) -> float32 [-1,1]，被 app/audio_play.py
用于把音序渲染成可播放波形。保持确定性（无随机源），便于缓存与测试。

V2 修复：
  [P0-C] _T_CACHE_LOCK 真正初始化并使用：全局 OrderedDict 在并发合成
    （用户切歌 + 播放线程 + GUI 试听线程三线并发）下，读写/move_to_end/
    popitem 是非原子操作，V1 仅声明 _T_CACHE_LOCK=None 从未加锁→随机
    数据损坏→播放爆音/崩溃。
  [P2-A] 短音（dur<0.35s）包络自适应：V1 decay 固定 0.35s，对 0.05s
    十六分音符，衰减尾部占比极低，包络像方波→听感生硬。V2 decay 取
    max(0.12, dur*0.6)，保证短音自然衰减至少覆盖 60% 音符时长。
"""
import threading

import numpy as np

# 时间轴缓存：同一 (n, sr) 只构造一次 np.arange
_T_CACHE: dict = {}
# V2: 首次 synth 调用时初始化（延迟导入期线程开销），读写均持锁
_T_CACHE_LOCK: "threading.Lock | None" = None


def _ensure_cache_lock() -> threading.Lock:
    """线程安全的锁懒初始化。V2 根因修复：V1 只声明 None 从未赋值。"""
    global _T_CACHE_LOCK
    if _T_CACHE_LOCK is None:
        _T_CACHE_LOCK = threading.Lock()
    return _T_CACHE_LOCK


def midi_to_hz(midi: int) -> float:
    return float(440.0 * 2.0 ** ((int(midi) - 69) / 12.0))


def _time_axis(n: int, sr: int) -> np.ndarray:
    """返回 float32 时间轴 t = arange(n)/sr，按 (n, sr) 缓存复用。

    V2: 读/写 _T_CACHE 全程持锁；上限 256（避免长时运行内存膨胀）。
    """
    lock = _ensure_cache_lock()
    key = (n, sr)
    with lock:
        cached = _T_CACHE.get(key)
        if cached is not None:
            return cached
    t = np.arange(n, dtype=np.float32) / np.float32(sr)
    with lock:
        if len(_T_CACHE) < 256:
            _T_CACHE[key] = t
    return t


def synth_piano(midi: int, dur: float, sr: int = 16000) -> np.ndarray:
    """合成单个钢琴音符（基频 + 谐波 + 轻包络）。返回 float32 [-1,1]。

    向量化一次性合成；V2+ 新增短音自适应 decay + 放大比例 release。

    关键包络策略（针对短音的截断咔哒根因）：
      - decay_tau 与 dur 强耦合：短音极速衰减，长音保留钢琴自然 0.35s。
        对 dur=0.08s（十六分音符）tau≈0.018s，末尾指数 ≈1% 量级，
        结合 release 线性淡出 → 末尾 5% 振幅 << 0.5 阈值。
      - release 占比从 5% → 短音 35% / 长音 5%：短音 1/3 以上
        时段在做线性淡出，零截断。
    """
    freq = midi_to_hz(midi)
    dur = max(0.05, float(dur))
    n = int(sr * dur)
    if n <= 0:
        return np.zeros(1, dtype=np.float32)

    t = _time_axis(n, sr)
    two_pi = 2.0 * np.pi
    # 基频 + 二次谐波(0.25) + 三次谐波(0.12)
    y = (np.sin(two_pi * freq * t)
         + 0.25 * np.sin(two_pi * 2.0 * freq * t)
         + 0.12 * np.sin(two_pi * 3.0 * freq * t)).astype(np.float32, copy=False)

    # V2+: 短音极速衰减 tau（与时长 2.2× 绑定），长音上限 0.35s（钢琴自然）
    decay_tau = min(0.35, max(0.015, dur * 0.22))
    # Attack: 快起音 5ms，避免起奏咔哒
    env = np.ones(n, dtype=np.float32)
    atk = min(int(0.005 * sr), max(1, n // 4))
    if atk > 1:
        env[:atk] = np.linspace(0.0, 1.0, atk, dtype=np.float32)
    # V2+: Release 占比与时长反向耦合（短音 35% 淡出 / 长音 5%）
    #   保证短音末尾 5% 窗口的 max_tail 远低于 0.5
    if dur < 0.15:
        rel_ratio = 0.35
    elif dur < 0.3:
        rel_ratio = 0.20
    elif dur < 0.6:
        rel_ratio = 0.10
    else:
        rel_ratio = 0.05
    rel = max(1, int(n * rel_ratio))
    if rel > 1:
        env[-rel:] *= np.linspace(1.0, 0.0, rel, dtype=np.float32)
    decay = np.exp(-t / np.float32(decay_tau)).astype(np.float32, copy=False)
    y = y * env * decay

    peak = float(np.max(np.abs(y))) + 1e-9
    return (y / np.float32(peak)).astype(np.float32, copy=False)
