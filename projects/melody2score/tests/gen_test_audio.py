# -*- coding: utf-8 -*-
"""生成已知旋律的测试音频（小星星主旋律），用于离线端到端自测。

用法：
    python tests/gen_test_audio.py
生成 tests/twinkle.wav（16kHz 单声道），并打印期望音高（MIDI）。
"""
import os

import numpy as np
import soundfile as sf

SR = 16000
BPM = 120
SPB = 60.0 / BPM  # 每拍秒数

# 小星星主旋律（C 大调），MIDI 音号
MELODY = [60, 60, 67, 67, 69, 69, 67, 65, 65, 64, 64, 62, 62, 60]
BEATS = [1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 2]


def midi2freq(m: int) -> float:
    return 440.0 * 2 ** ((m - 69) / 12.0)


def gen(out_path: str = "tests/twinkle.wav"):
    segs = []
    for m, b in zip(MELODY, BEATS):
        dur = b * SPB
        n = int(SR * dur)
        t = np.linspace(0, dur, n, endpoint=False)
        sig = np.sin(2 * np.pi * midi2freq(m) * t)
        # 简单 attack/release 包络，更接近真实哼唱
        env = np.ones(n)
        atk = min(int(0.01 * SR), n // 4)
        rel = min(int(0.02 * SR), n // 4)
        env[:atk] = np.linspace(0, 1, atk)
        env[-rel:] = np.linspace(1, 0, rel)
        segs.append(sig * env * 0.8)
        segs.append(np.zeros(int(0.09 * SR)))  # 音符间 90ms 静音（>min_voiced，可分辨相邻同音）
    y = np.concatenate(segs).astype(np.float32)
    sf.write(out_path, y, SR)
    print(f"[gen] 已生成 {out_path}  (sr={SR})")
    print("[gen] 期望 MIDI 音高:", MELODY)
    return out_path, MELODY


if __name__ == "__main__":
    base = os.path.dirname(os.path.abspath(__file__))
    out = os.path.join(base, "twinkle.wav")
    gen(out)
