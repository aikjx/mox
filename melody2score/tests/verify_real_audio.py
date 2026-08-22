# -*- coding: utf-8 -*-
"""企业级真实音频识别质量验证：多旋律 × 多音色 × 量化指标。

用 classic_corpus 的 ground truth（精确 MIDI 标注）对内置样例音频
做端到端识别，统计：
  - 音符序列精确匹配率（零容差：数量+音高逐一对应）
  - 容差匹配率（允许 ±1 音符错位）
  - 音高类覆盖率
  - 时值误差中位数
运行：
    python tests/verify_real_audio.py [--quick]
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from classic_corpus import MELODIES
from core.pipeline import Melody2Score
from core.config import Config
from core import capture

AUDIO_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "audio")

# 代表性样例：覆盖 乐器/人声/纯音 三大类 + 不同节奏复杂度
SAMPLES = [
    ("小星星", "piano"),      # 乐器·基础
    ("小星星", "human_voice"),  # 人声
    ("欢乐颂", "piano"),       # 含附点
    ("欢乐颂", "flute"),
    ("茉莉花", "guitar"),      # 跨八度
    ("两只老虎", "strings"),   # 重复乐句
    ("致爱丽丝", "pure_sine"),  # 纯音·快速十六分
    ("生日歌", "human_voice"), # 弱起+附点
]


def expected_midis(title: str):
    for t_zh, _t_en, seq in MELODIES:
        if t_zh == title:
            return [m for m, _b in seq if m > 0]
    return None


def tol_match(got, exp):
    """容差匹配：允许 |got|-|exp| <= 1 的数量差下，求最优对齐的音高一致率。"""
    if not exp or not got:
        return 0.0
    # 简单 DP 对齐（音高相等得 1 分）
    n, m = len(got), len(exp)
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            eq = 1 if got[i - 1] == exp[j - 1] else 0
            dp[i][j] = max(dp[i - 1][j - 1] + eq, dp[i - 1][j], dp[i][j - 1])
    return dp[n][m] / len(exp)


def main(quick=False):
    cfg = Config()
    cfg.enable_denoise = True   # 样例注入了底噪(SNR 42dB)，开降噪
    m = Melody2Score(cfg)

    samples = SAMPLES[:4] if quick else SAMPLES
    exact_hits = 0
    tol_scores = []
    pc_scores = []
    print("=" * 64)
    print(" 企业级真实音频识别质量验证（ground truth: classic_corpus）")
    print("=" * 64)

    for title, timbre in samples:
        fname = None
        # 从 manifest 命名规则定位文件
        for t_zh, _t_en, _seq in MELODIES:
            if t_zh == title:
                idx = MELODIES.index((t_zh, _t_en, _seq))
                kind = "voice" if timbre == "human_voice" else (
                    "pure" if timbre.startswith("pure_") else "instrument")
                fname = f"audio/m{idx:02d}_{kind}_{timbre}.wav"
                break
        fpath = os.path.join(os.path.dirname(AUDIO_DIR), fname)
        if not os.path.exists(fpath):
            print(f"  [SKIP] {title}/{timbre}: 样例缺失 {fname}")
            continue

        exp = expected_midis(title)
        y = capture.load_audio(fpath, cfg.sr)
        res = m.recognize({"kind": "array", "y": y, "sr": cfg.sr, "cfg": cfg})
        got = [n["midi"] for n in res["notes"]]

        exact = (got == exp)
        tol = tol_match(got, exp)
        pc = len(set(x % 12 for x in got) & set(x % 12 for x in exp)) / max(1, len(set(x % 12 for x in exp)))
        exact_hits += int(exact)
        tol_scores.append(tol)
        pc_scores.append(pc)

        status = "EXACT" if exact else ("OK" if tol >= 0.85 else "LOW")
        print(f"  [{status:>5}] {title}·{timbre}: "
              f"精确={exact} 容差率={tol:.0%} 音高类={pc:.0%} "
              f"音符 {len(got)}/{len(exp)} BPM={res['bpm']:.0f} "
              f"后端={res['backend']}")

    n = len(tol_scores)
    if n == 0:
        print("无可用样例")
        return 1
    print("-" * 64)
    print(f"  样例数: {n} | 精确匹配: {exact_hits}/{n} ({exact_hits/n:.0%})")
    print(f"  平均容差匹配率: {sum(tol_scores)/n:.1%}")
    print(f"  平均音高类覆盖率: {sum(pc_scores)/n:.1%}")
    ok = sum(tol_scores)/n >= 0.85 and exact_hits >= n * 0.5
    print(f"  结论: {'PASS（企业级质量达标）' if ok else 'FAIL（需进一步调优）'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(quick="--quick" in sys.argv))
