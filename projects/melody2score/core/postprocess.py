# -*- coding: utf-8 -*-
"""MIDI 音符后处理纠错层（缺失的关键一环，错误一路传导到谱面的根因）。

用户痛点链路：
  音高帧 → 初分音符 → segment_notes 已经做了局部毛刺过滤，但：
    1) 旋律整体可能被伴奏低谐波拉偏，出现「一串 C 中夹一个 G」的孤立错音；
    2) 无意义的大跳（相邻音间隔>10 半音且该跳音前后音程都回原位置）；
    3) 人声/乐器音域外的 MIDI（例如 CREPE 把底噪解成 20Hz 的 C1）；
    4) 时长异常：< 60ms 的零散短音符、> 6 秒的不自然长音；
    5) 连续重复极短同音（应该合并成一个长音）。

本模块在 octave_normalize 之后、BPM/调式之前插入，对音符列表做全局
语义级纠错。不修改输入，返回新的 notes 列表 + 诊断统计。

统一输出：{
  "notes": List[Dict]（修正后音符）,
  "dropped_count": int（被丢弃的异常音符数）,
  "merged_count": int（连续短同音被合并的次数）,
  "corrected_jumps": int（被纠正的无意义大跳次数）,
  "kept_range": (min_midi, max_midi),
}
"""
from typing import Dict, List, Tuple

import numpy as np


# 企业级业务约束（可按场景调优）
# 人声/流行旋律合理音域：A2 (45) ~ C6 (84)；哼唱+钢琴覆盖绝大多数情况
_MIDI_MIN_HARD = 36    # C2：再低是贝斯/次低频噪声
_MIDI_MAX_HARD = 96    # C7：再高接近刺耳泛音
# 相邻音程合理跳程：>12 半音（纯八）且左右都回原位 → 视为幻影跳音
_JUMP_OCTAVE_THRESH = 12
# 最短音符绝对下限（秒）：低于此值的孤立音符一律丢弃（比配置层更激进，
# 因为配置层 min_note_dur 是"正常音符过滤"，这里处理"全局语义"）
_DUR_FLOOR_SEC = 0.06
# 最长音符绝对上限（秒）：超过视为持续尾音拖长被误合成一个音，按 4 拍截断
_DUR_CEIL_SEC = 6.0
# 连续同音合并最大间隔（秒）：相邻同音间隔 ≤ 该值 且 每个都短时，合并
_GAP_MERGE_SEC = 0.05


def drop_out_of_range(notes: List[Dict]) -> Tuple[List[Dict], int]:
    """丢弃音域外的孤立音符：返回 (新列表, 丢弃数)。"""
    out = []
    dropped = 0
    for n in notes:
        m = int(round(float(n.get("midi", 60))))
        if _MIDI_MIN_HARD <= m <= _MIDI_MAX_HARD:
            out.append(dict(n))
        else:
            dropped += 1
    return out, dropped


def drop_too_short_or_long(notes: List[Dict]) -> Tuple[List[Dict], int]:
    """丢弃过短孤立、截断过长拖音。"""
    out = []
    dropped = 0
    for n in notes:
        s, e = float(n.get("start", 0.0)), float(n.get("end", 0.0))
        d = max(0.0, e - s)
        if d < _DUR_FLOOR_SEC:
            # 夹在两个同高长音之间的短音稍后由 jump 修复处理；孤立短音直接丢
            dropped += 1
            continue
        item = dict(n)
        if d > _DUR_CEIL_SEC:
            item["end"] = item["start"] + _DUR_CEIL_SEC
        out.append(item)
    return out, dropped


def merge_consecutive_repeats(notes: List[Dict]) -> Tuple[List[Dict], int]:
    """合并相邻同音、时间接近或有微小重叠的重复段。

    真实场景：长音中部被 VAD/conf 切一刀，变成 [C 0.0-0.4][C 0.41-0.8]，
    量化时会出两个四分音符（重复 1 1）而非一个二分音符（1-）。
    合并条件（须同时满足，企业级收紧）：
      1) 音高相同；
      2) 两段之间无静音边界（sep_prev 必须为 False）——segment_notes 已把
         「真实时间空洞/静音」标记为 sep_prev=True，那是独立音符（两个四分
         音符的 1 1 之间常有 60ms 真实间隔），绝不能合并；
      3) 间隙 ≤ _GAP_MERGE_SEC 或两段时间重叠；
      4) 仅当两段都较短（各自 < _MERGE_MAX_SEC，被切断的长音碎片才这么短）
         才合并——完整时长的重复同音（如每段 0.42s 的 5 5）不在此列，
         若误合并会把旋律重复音吞成单音（小星星 14 音塌成 9 音的回归根因）。
    """
    if len(notes) < 2:
        return [dict(n) for n in notes], 0
    # 被切断碎片的上界：约半拍（拍长未知时取 0.25s 保守值）。超过此长度的
    # 同音段必是「有意为之的重复音」，不应合并。
    _MERGE_MAX_SEC = 0.25
    merged = 0
    out: List[Dict] = [dict(notes[0])]
    for n in notes[1:]:
        prev = out[-1]
        same_pitch = int(round(prev["midi"])) == int(round(n["midi"]))
        gap = float(n["start"]) - float(prev["end"])
        has_silence = bool(n.get("sep_prev"))   # 段间有真实静音边界 → 独立音符
        short_prev = (float(prev["end"]) - float(prev["start"])) < _MERGE_MAX_SEC
        short_cur = (float(n["end"]) - float(n["start"])) < _MERGE_MAX_SEC
        if (same_pitch and not has_silence and gap <= _GAP_MERGE_SEC
                and short_prev and short_cur):
            # 合并：取 min start, max end
            prev["start"] = min(float(prev["start"]), float(n["start"]))
            prev["end"] = max(float(prev["end"]), float(n["end"]))
            prev["nfr"] = int(prev.get("nfr", 1)) + int(n.get("nfr", 1))
            merged += 1
        else:
            out.append(dict(n))
    return out, merged


def fix_spurious_octave_jumps(notes: List[Dict]) -> Tuple[List[Dict], int]:
    """修复「孤立幻影八度跳」：A → B(±oct+) → A' 且 |B-A|>阈值 且 A≈A'，
    把 B 的音高纠正为 A（或按前后最近邻平移 ±12）。这是人声滑音 / 伴奏
    谐波泄漏常见错音：比如相邻 C-C-C 中间被解成 G（十二度），谱面上出现
    一个孤立高音点或低音点，完全不像原曲。"""
    if len(notes) < 3:
        return [dict(n) for n in notes], 0
    fixed = 0
    midi_arr = np.array([int(round(float(n.get("midi", 60)))) for n in notes], dtype=int)
    out = [dict(n) for n in notes]
    # 检测窗口：位置 i，以 i-1 与 i+1 为锚
    for i in range(1, len(notes) - 1):
        a, b, c = midi_arr[i - 1], midi_arr[i], midi_arr[i + 1]
        gap_ab = abs(b - a)
        gap_bc = abs(b - c)
        gap_ac = abs(a - c)
        if (gap_ab >= _JUMP_OCTAVE_THRESH
                and gap_bc >= _JUMP_OCTAVE_THRESH
                and gap_ac <= 4):  # 前后锚接近
            # 幻影：把 b 平移 n×12 让它更靠近 (a+c)/2
            target = (int(a) + int(c)) / 2.0
            shift = int(round((target - b) / 12.0)) * 12
            if shift != 0:
                new_midi = int(b + shift)
                if _MIDI_MIN_HARD <= new_midi <= _MIDI_MAX_HARD:
                    out[i]["midi"] = new_midi
                    midi_arr[i] = new_midi
                    fixed += 1
    return out, fixed


def median_smooth_pitch(notes: List[Dict], win: int = 3) -> Tuple[List[Dict], int]:
    """对 MIDI 音高做"时间加权众数平滑"（不跨大时间跳变）。

    修复 twinkle 过度平滑（69→67、64→65）：
      众数机制在窗口 [67,67,69] 时，67 有 2 票 → 69 被错误抹成 67。
      真实旋律的 5 度跳进（C→G）是常见音程，不可被"邻域投票"抹掉。

    新规则（企业级稳健）：
      1) 只对窗口内众数 ≥ 60% 才修正（避免 2/3 的轻微多数强推 3 度音程）；
      2) 众数与原音差距必须 ≤ 1 半音（只修颤音抖动不修跳进音程）；
         （≥2 半音就是真实旋律变化，不应"平滑"掉）；
      3) 窗口仍限定在 1.5 秒内（跨长间隔不参与）。
    """
    if len(notes) < 3 or win < 1:
        return [dict(n) for n in notes], 0
    corrected = 0
    out = [dict(n) for n in notes]
    n = len(out)
    for i in range(n):
        t0 = float(out[i]["start"])
        neighbors: List[int] = []
        for j in range(max(0, i - win), min(n, i + win + 1)):
            if abs(float(out[j]["start"]) - t0) <= 1.5:
                neighbors.append(int(round(out[j]["midi"])))
        if len(neighbors) < 3:
            continue
        # 众数
        counts: Dict[int, int] = {}
        for m in neighbors:
            counts[m] = counts.get(m, 0) + 1
        total = len(neighbors)
        mode_m, mode_cnt = max(counts.items(), key=lambda kv: kv[1])
        cur = int(round(out[i]["midi"]))
        # 双门槛：1) 众数占比 ≥ 60%；2) 与原音相差 ≤ 1 半音
        if (mode_cnt * 10 >= 6 * total
                and mode_m != cur
                and abs(mode_m - cur) <= 1):
            out[i]["midi"] = mode_m
            corrected += 1
    return out, corrected


def postprocess_notes(notes: List[Dict]) -> Dict:
    """纠错总入口：按顺序组合各子步骤（顺序至关重要）。"""
    if not notes:
        return {"notes": [], "dropped_count": 0, "merged_count": 0,
                "corrected_jumps": 0, "smoothed": 0,
                "kept_range": (60, 72)}

    step1, d1 = drop_out_of_range(notes)
    step2, d2 = drop_too_short_or_long(step1)
    step3, m1 = merge_consecutive_repeats(step2)
    step4, j1 = fix_spurious_octave_jumps(step3)
    step5, s1 = median_smooth_pitch(step4, win=3)

    # 最终确保按 start 排序（下游所有量化/渲染依赖时序）
    step5.sort(key=lambda n: float(n.get("start", 0.0)))

    midi_list = [int(round(float(n.get("midi", 60)))) for n in step5]
    kept_range = (min(midi_list), max(midi_list)) if midi_list else (60, 72)

    return {
        "notes": step5,
        "dropped_count": d1 + d2,
        "merged_count": m1,
        "corrected_jumps": j1,
        "smoothed": s1,
        "kept_range": kept_range,
    }
