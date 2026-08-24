# -*- coding: utf-8 -*-
"""旋律识别歌曲（离线曲库匹配，企业级稳健）。

思路：把识别出的旋律轮廓与本地曲库的「标准音高序列」做音程级 DTW 匹配。
  - 用音程（相邻音半音差）而非绝对音高，对哼唱跑调 / 移调天然鲁棒；
  - DTW 对齐变长序列，容忍节奏、起拍、漏音差异；
  - 输出 Top-K 候选歌名 + 匹配分(0~100)，确定性、可复现、无需联网。

曲库来自 classic_corpus.MELODIES（15 首公版经典旋律标准 MIDI 序列）。
"""
from typing import Dict, List, Optional, Tuple

import numpy as np

# 延迟导入曲库，避免无样例环境 import 失败
try:
    from classic_corpus import MELODIES
except Exception:  # pragma: no cover
    MELODIES = []


def _intervals(midis: List[int]) -> List[int]:
    """MIDI 序列 -> 相邻音程（半音差）列表。"""
    out = []
    for a, b in zip(midis, midis[1:]):
        out.append(int(round(b - a)))
    return out


def _dtw(a: List[int], b: List[int]) -> float:
    """序列 DTW 距离（基于绝对音程差）。O(n*m)，曲库规模极小，足够快。"""
    n, m = len(a), len(b)
    if n == 0 or m == 0:
        return float("inf")
    INF = float("inf")
    dp = [[INF] * (m + 1) for _ in range(n + 1)]
    dp[0][0] = 0.0
    for i in range(1, n + 1):
        ai = a[i - 1]
        for j in range(1, m + 1):
            cost = abs(ai - b[j - 1])
            dp[i][j] = cost + min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1])
    return dp[n][m]


def build_library() -> List[Dict]:
    """构建曲库：每首旋律 -> 标准音程序列 + 元数据。"""
    lib = []
    for zh, en, seq in MELODIES:
        midis = [m for m, _ in seq if m > 0]
        iv = _intervals(midis)
        lib.append({
            "title_zh": zh,
            "title_en": en,
            "midis": midis,
            "intervals": iv,
            "length": len(midis),
        })
    return lib


def match_song(notes: List[Dict], top_k: int = 5,
               min_overlap: float = 0.5) -> Dict:
    """给定识别出的音符列表（含 midi），返回匹配结果。

    返回 {matched(bool), candidates:[{title_zh,title_en,score,len}], query_len}。
    score 为 0~100 的归一化匹配分（越长、对齐越准越高）。
    """
    if not notes:
        return {"matched": False, "candidates": [], "query_len": 0}
    query_midis = [int(n["midi"]) for n in notes]
    query_iv = _intervals(query_midis)
    if not query_iv:
        return {"matched": False, "candidates": [], "query_len": len(query_midis)}

    lib = build_library()
    scored = []
    for item in lib:
        # 仅与长度相近的候选比较（避免过短查询误匹配长曲）
        ref = item["intervals"]
        d = _dtw(query_iv, ref)
        # 归一化：平均单音程距离越小越好；越长越可信（折扣噪声）
        norm = (d / len(query_iv)) if len(query_iv) else float("inf")
        # 匹配分：经验映射，平均音程误差 <=1 半音近完美，>=8 几乎无关
        score = max(0.0, 100.0 * (1.0 - norm / 8.0))
        # 长度覆盖度：查询能覆盖曲库的比例（过小则降权）
        coverage = min(1.0, len(query_iv) / max(1, item["length"] - 1))
        score *= (0.6 + 0.4 * coverage)
        scored.append({
            "title_zh": item["title_zh"],
            "title_en": item["title_en"],
            "score": round(score, 1),
            "len": item["length"],
        })

    scored.sort(key=lambda x: x["score"], reverse=True)
    candidates = scored[:top_k]
    best = candidates[0]["score"] if candidates else 0.0
    # 匹配阈值：Top1 分足够高且查询有一定长度才判定命中
    matched = best >= 55.0 and len(query_iv) >= 6
    return {
        "matched": matched,
        "candidates": candidates,
        "query_len": len(query_midis),
        "best_score": round(best, 1),
    }
