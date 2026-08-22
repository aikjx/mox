# -*- coding: utf-8 -*-
"""诊断：生日歌人声识别细节 + BPM 时长分布。"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from classic_corpus import MELODIES
from core.pipeline import Melody2Score
from core.config import Config
from core import capture

idx = next(i for i, (t, _, __) in enumerate(MELODIES) if t == "生日歌")
seq = MELODIES[idx][2]
exp = [m for m, _b in seq if m > 0]
exp_beats = [b for m, b in seq if m > 0]
print("期望 MIDI :", exp)
print("期望拍数 :", exp_beats)

fpath = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                     f"audio/m{idx:02d}_voice_human_voice.wav")
cfg = Config()
m = Melody2Score(cfg)
y = capture.load_audio(fpath, cfg.sr)
res = m.recognize({"kind": "array", "y": y, "sr": cfg.sr, "cfg": cfg})
got = [n["midi"] for n in res["notes"]]
durs = [n["dur"] for n in res["notes"]]
print("识别 MIDI :", got)
print("识别时长 :", [round(d, 2) for d in durs])
print(f"BPM={res['bpm']} beat_dur={60/res['bpm']:.3f}s")
print("识别拍数 :", [round(d * res['bpm'] / 60, 2) for d in durs])
print("简谱:", res["jianpu"])

# 诊断小星星 piano 的 BPM 偏差
idx2 = next(i for i, (t, _, __) in enumerate(MELODIES) if t == "小星星")
fpath2 = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                      f"audio/m{idx2:02d}_instrument_piano.wav")
y2 = capture.load_audio(fpath2, cfg.sr)
res2 = m.recognize({"kind": "array", "y": y2, "sr": cfg.sr, "cfg": cfg})
durs2 = [n["dur"] for n in res2["notes"]]
print("\n小星星·piano 识别时长:", [round(d, 2) for d in durs2])
print(f"BPM={res2['bpm']}（真值 {60/0.42:.1f}） 音符真值时长 0.42s")
