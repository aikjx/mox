# -*- coding: utf-8 -*-
"""诊断：多音符样例的识别序列对比。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from classic_corpus import MELODIES
from core.pipeline import Melody2Score
from core.config import Config
from core import capture

cfg = Config()
m = Melody2Score(cfg)

for title, fname in [("两只老虎", "m03_instrument_strings.wav"),
                     ("小星星", "m00_voice_human_voice.wav")]:
    idx = next(i for i, (t, _, __) in enumerate(MELODIES) if t == title)
    exp = [mm for mm, _b in MELODIES[idx][2] if mm > 0]
    fpath = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                         "audio", fname)
    y = capture.load_audio(fpath, cfg.sr)
    res = m.recognize({"kind": "array", "y": y, "sr": cfg.sr, "cfg": cfg})
    got = [n["midi"] for n in res["notes"]]
    durs = [round(n["dur"], 2) for n in res["notes"]]
    print(f"\n{title} ({fname}):")
    print("  期望:", exp)
    print("  识别:", got)
    print("  时长:", durs)
