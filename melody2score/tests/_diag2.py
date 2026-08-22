# -*- coding: utf-8 -*-
"""诊断：生日歌人声的 VAD 有声段 vs 期望音符。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from classic_corpus import MELODIES
from core.config import Config
from core import capture, vad, preprocess
import numpy as np

idx = next(i for i, (t, _, __) in enumerate(MELODIES) if t == "生日歌")
seq = MELODIES[idx][2]
exp = [(m, b) for m, b in seq if m > 0]
BEAT, GAP = 0.42, 0.06

# 重建期望时间轴
t = 0.0
print("期望音符 (midi, start, dur):")
for m, b in exp:
    print(f"  midi={m} start={t:.2f} dur={b*BEAT:.2f}")
    t += b * BEAT + GAP

fpath = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                     f"audio/m{idx:02d}_voice_human_voice.wav")
cfg = Config()
y = capture.load_audio(fpath, cfg.sr)
y = preprocess.preprocess(y, cfg.sr, cfg.enable_denoise)
mask = vad.voice_activity_mask(y, cfg.sr, energy_thresh=cfg.vad_energy_thresh,
    centroid_min=cfg.vad_centroid_min, centroid_max=cfg.vad_centroid_max,
    flatness_max=cfg.vad_flatness_max, hop_ms=cfg.hop, min_voiced_ms=cfg.min_voiced_ms)

segs = []
i = 0
while i < len(mask):
    if mask[i] == 1:
        j = i
        while j < len(mask) and mask[j] == 1:
            j += 1
        segs.append((round(i * 0.01, 2), round(j * 0.01, 2)))
        i = j
    else:
        i += 1
print(f"\nVAD 有声段 ({len(segs)} 段):", segs)
