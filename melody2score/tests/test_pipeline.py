# -*- coding: utf-8 -*-
"""端到端自测：合成旋律 → 跑流水线 → 断言恢复音高类匹配。

运行：
    pytest tests/test_pipeline.py -q
或直接：
    python tests/test_pipeline.py
"""
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from core.pipeline import Melody2Score
from core.config import Config

EXPECTED = [60, 60, 67, 67, 69, 69, 67, 65, 65, 64, 64, 62, 62, 60]


def _ensure_audio():
    base = os.path.dirname(os.path.abspath(__file__))
    wav = os.path.join(base, "twinkle.wav")
    if not os.path.exists(wav):
        import subprocess
        subprocess.run([sys.executable, os.path.join(base, "gen_test_audio.py")], check=True)
    return wav


def test_twinkle_pipeline():
    audio = _ensure_audio()
    cfg = Config()
    cfg.enable_denoise = False  # 合成音频本就干净，关降噪加速
    m = Melody2Score(cfg)
    res = m.run(audio_path=audio)

    got = [n["midi"] for n in res["notes"]]
    print("\n[test] 恢复 MIDI:", got)
    print("[test] 简谱:", res["jianpu"])
    print(f"[test] BPM={res['bpm']:.1f} Key={res['key']}")

    # 1) 恢复音符数量不应明显偏少
    assert len(got) >= len(EXPECTED) - 3, f"音符数偏少: {got}"

    # 2) 音高类（pitch class）匹配率应较高（允许 ±1 半音近似）
    exp_pc = set(m % 12 for m in EXPECTED)
    got_pc = set(m % 12 for m in got)
    inter = exp_pc & got_pc
    rate = len(inter) / len(exp_pc)
    print(f"[test] 音高类匹配率: {rate:.2f}")
    assert rate >= 0.7, f"音高类匹配率过低: {rate}"


if __name__ == "__main__":
    # 无 pytest 时也能直接跑
    audio = _ensure_audio()
    cfg = Config()
    cfg.enable_denoise = False
    m = Melody2Score(cfg)
    res = m.run(audio_path=audio)
    m.print_summary(res)
    print("恢复 MIDI:", [n["midi"] for n in res["notes"]])
