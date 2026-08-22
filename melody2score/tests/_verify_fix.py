# -*- coding: utf-8 -*-
"""修复验证脚本：NaN 传染 / 共识簇污染 / BPM 反推 / 时值量化。"""
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import numpy as np
import librosa

from core.pipeline import Melody2Score
from core.config import Config

EXPECTED = [60, 60, 67, 67, 69, 69, 67, 65, 65, 64, 64, 62, 62, 60]
EXPECTED_BEATS = [1, 1, 1, 1, 1, 1, 2, 1, 1, 1, 1, 1, 1, 2]

cfg = Config()
cfg.enable_denoise = False
m = Melody2Score(cfg)
y, sr = librosa.load('tests/twinkle.wav', sr=16000, mono=True)
res = m.recognize({'kind': 'array', 'y': y, 'sr': sr, 'cfg': cfg})

got = [n['midi'] for n in res['notes']]
durs = [round(n['dur'], 2) for n in res['notes']]
print('期望 MIDI :', EXPECTED)
print('识别 MIDI :', got)
print('音符时长  :', durs)
print(f"BPM={res['bpm']}  Key={res['key']}  置信度={res['confidence']}")
print('简谱      :', res['jianpu'])
print('perf      :', res['perf'])
print('完全匹配  :', got == EXPECTED)

# 时值检查：1 拍音符 ≈ 0.5s（VAD 切边后 0.4~0.6），2 拍 ≈ 1.0s
if got == EXPECTED:
    ok_dur = all(0.3 <= d <= 0.7 for d, b in zip(durs, EXPECTED_BEATS) if b == 1) and \
             all(0.8 <= d <= 1.3 for d, b in zip(durs, EXPECTED_BEATS) if b == 2)
    print('时值正确  :', ok_dur)
    print('BPM 正确  :', abs(res['bpm'] - 120.0) < 15)
