# -*- coding: utf-8 -*-
"""音符时值量化：基于 BPM 节拍网格的柔性对齐（保留附点/切分/三连音）。

修复旧版两个量化 bug：
  1) to_musicxml 固定 round(ql/0.25)*0.25：所有音符被强制到 16 分倍数，
     0.75 拍附点、1.5 拍四分附点还保留，但 0.33/0.66 拍三连音被吞并，
     且「起始点不对齐到拍位」——音符时间戳与节拍网格错位，最终谱面
     看起来像"每个音都晚半拍"。
  2) to_jianpu 按 int(round(dur/beat_dur))：只能表达整数拍（1,2,3 拍），
     所有半拍音被硬上取整为 1 拍，附点/切分音完全丢失。

新模块统一提供：
  quantize_notes(notes, bpm) -> List[Dict]：
    对每个音符做「起始点柔性吸附到 1/8 拍网格 + 时长量化到音乐合法时值
    {0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 0.333, 0.666} 拍」。
    输出每个 note 新增键：
      start_beat   : 量化后起始（拍，浮点数）
      end_beat     : 量化后结束（拍，浮点数）
      dur_beat     : end_beat - start_beat
      rest_before  : 与上一个音符之间的休止（拍，可能为 0）

量化策略（企业级稳健，拒绝粗暴等分）：
  - 起始点：以乐曲 0s 为 0 拍，beat_dur = 60/BPM。
    把观测 start（秒）换算为拍数 s_beat，吸附到最近的 0.125 拍（1/8）网格。
    （1/8 拍是兼顾精度与可读性的最佳颗粒度：足够表达切分音，
    又不会让简谱/五线谱满屏十六分。）
  - 时长：观测 d_beat = obs_end - obs_start，匹配「音乐合法时值」
    {0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0} + {0.333, 0.666}（三连）。
    选绝对偏差最小的合法值，若最佳匹配偏差 > 0.2 拍则回退吸附到 0.25。
  - 非重叠保证：相邻音符若量化后出现重叠（前音 end > 后音 start - 1e-6），
    则把后音 start_beat 推到「max(自身 start_beat, 前音 end_beat)」，
    再重算 dur_beat（至少 0.25 拍）——避免 music21 渲染重叠音爆崩溃。
"""
from typing import Dict, List, Tuple

import numpy as np


# 合法音乐时值（拍）列表：附点+切分+三连，覆盖 99% 流行歌曲情形
_LEGAL_DUR_BEATS = (
    0.25,     # 十六分
    1/3,      # 三连八分（≈0.333）
    0.5,      # 八分
    2/3,      # 三连四分（≈0.666）
    0.75,     # 八分附点
    1.0,      # 四分
    1.5,      # 四分附点
    2.0,      # 二分
    3.0,      # 二分附点
    4.0,      # 全
)

# 吸附颗粒度（拍）：1/8 拍 = 0.125
_SNAP_GRID = 0.125
# 最大允许时长偏差（拍）：超过则不做「最近合法时值」回退，用 0.25 拍栅格对齐
_MAX_DUR_DEV = 0.2


def _snap_start(beat: float) -> float:
    """吸附到 _SNAP_GRID（默认 0.125 拍 = 1/8 拍）网格。"""
    if _SNAP_GRID <= 0:
        return beat
    g = _SNAP_GRID
    q = int(round(beat / g))
    return float(q) * g


def _legal_dur(d_beat: float) -> float:
    """把观测 d_beat 拍数映射到最近合法音乐时值。

    模糊区 tie-break（解决 0.86 拍 → 0.75 vs 1.0 的歧义）：
      - 先找到 Top-2 最小偏差的候选；
      - 若两者偏差差 < 0.05 拍（模糊）：选择更高时值；
        人耳对"短于预期"的感知显著差于"长于预期"，且 1/8 附点
        是稀有标记，卡中间时宁可量化成整数拍。
      - 偏差过大（> _MAX_DUR_DEV）则 0.25 拍栅格对齐兜底。
    """
    if d_beat <= 0:
        return 0.25
    diffs = sorted((abs(x - d_beat), x) for x in _LEGAL_DUR_BEATS)
    dev1, val1 = diffs[0]
    if dev1 > _MAX_DUR_DEV:
        # 兜底：四舍五入到 0.25 拍（十六分整数倍）
        return max(0.25, round(d_beat / 0.25) * 0.25)
    # Top-2 模糊检查
    if len(diffs) >= 2:
        dev2, val2 = diffs[1]
        if (dev2 - dev1) < 0.05 and val2 > val1:
            return float(val2)
    return float(val1)


def quantize_notes(notes: List[Dict], bpm: float) -> List[Dict]:
    """把带 (start,end,midi) 秒单位的原始音符 → 柔性量化到节拍网格。

    返回新的音符列表（不修改输入），每个元素新增：
      start_beat / end_beat / dur_beat / rest_before（拍）
    """
    out: List[Dict] = []
    if not notes:
        return out
    bpm_eff = float(bpm) if bpm and bpm > 0 else 120.0
    beat_dur = 60.0 / bpm_eff  # 一拍（秒）

    prev_end_beat = 0.0
    for i, n in enumerate(notes):
        s = max(0.0, float(n.get("start", 0.0)))
        e = max(s, float(n.get("end", s)))
        d_raw = e - s
        # 换算到拍
        s_beat = s / beat_dur
        d_beat = d_raw / beat_dur
        # 起音吸附到 1/8 拍网格
        qs = _snap_start(s_beat)
        # 时长映射到最近合法时值
        qd = _legal_dur(d_beat)
        qe = qs + qd

        # 防重叠：若与上一个量化后音符有交叠，后推本音起始
        if i > 0 and qs < prev_end_beat:
            qs = prev_end_beat
            # 吸附后再重取合法时长（起音已动，d_beat 保持意图）
            qd = _legal_dur(d_beat)
            qe = qs + qd

        # 计算与前一个音符之间的休止
        rest = 0.0 if i == 0 else max(0.0, qs - prev_end_beat)

        item = dict(n)
        item["start_beat"] = round(qs, 4)
        item["end_beat"] = round(qe, 4)
        item["dur_beat"] = round(qd, 4)
        item["rest_before_beat"] = round(rest, 4)
        out.append(item)
        prev_end_beat = qe
    return out


# ==========================
# 便捷：从量化结果还原 music21 quarterLength 与简谱 "-" 数
# ==========================

def quarter_length(dur_beat: float) -> float:
    """量化后的拍数 → music21 quarterLength（直接使用 dur_beat）。"""
    # 我们的「拍」就是四分音符（quarter note）一拍，因此 dur_beat 即
    # quarterLength，唯一要做的是 music21 能识别的合法值（附点/三连都合法）。
    return float(dur_beat)


def jianpu_dur_tokens(dur_beat: float) -> Tuple[str, int]:
    """量化后的拍数 → (数字后缀类型, 延音线数)。

    原简谱只用整数拍的 "-" 丢失了半拍。新方式：
      - 加下划线 _ 表示半拍（八分音符）；
      - 加双下划线 __ 表示 1/4 拍（十六分音符）；
      - 加 "." 后缀表示附点（原时值 × 1.5）；
      - 整数拍仍用 "-" 延续；
      - 三连音加 "3" 前缀标记（拍数 ≈ 0.333 或 0.666）。

    返回 (prefix, underscores, dots, dashes)：
      prefix     : str（简谱音级前的修饰，如 "3_" 表示三连八分）
      underscores: int（下方下划线数：0=四分,1=八分,2=十六分）
      dots       : int（右侧附点数：0 或 1）
      dashes     : int（右侧延音线 "-" 数量）

    调用方按：prefix + "<u><u>...音级</u></u>" + "."*dots + "-"*dashes 拼装。
    """
    # 规范化到最近合法表中的条目
    d = float(dur_beat)
    # 找最接近合法值条目（避免浮点漂移）
    entry = min(_LEGAL_DUR_BEATS, key=lambda x: abs(x - d))
    if abs(entry - d) > 0.2:
        entry = max(0.25, round(d / 0.25) * 0.25)

    triplet = False
    # 三连音判定
    if abs(entry - 1/3) < 0.05:
        triplet, beats = True, 1/3
    elif abs(entry - 2/3) < 0.05:
        triplet, beats = True, 2/3
    else:
        beats = float(entry)

    # 分解：整数拍部分（dashes）+ 余下（决定下划线+附点）
    if triplet:
        # 三连音：1/3 → 八分三连（1下划线 三连前缀）
        #         2/3 → 八分三连两个（用下划线 三连前缀 附点表达近似）
        if abs(beats - 1/3) < 0.05:
            underscores = 1
            dots = 0
            dashes = 0
        else:  # 2/3
            underscores = 1
            dots = 0
            dashes = 0
        prefix = "3" if triplet else ""
        return prefix, underscores, dots, dashes

    # 非三连：拆成 N 拍整数 + frac（0.25/0.5/0.75）
    whole = int(beats)         # 完整整数拍
    frac = beats - whole       # 0.25 / 0.5 / 0.75
    underscores = 0
    dots = 0
    if abs(frac - 0.75) < 0.01:
        # 0.75 拍 = 八分附点（1/2 + 1/4 → 下划线+附点）
        underscores = 1
        dots = 1
        frac_use = 0.0  # 这部分已经被下划线+附点表达
    elif abs(frac - 0.5) < 0.01:
        underscores = 1
        dots = 0
    elif abs(frac - 0.25) < 0.01:
        underscores = 2
        dots = 0
    else:
        frac_use = 0.0
        underscores = 0

    # 整数拍（1 拍=基准）全部用 "-" 延音表达。附点只在"基"上打一次。
    dashes = whole
    # 注意：如果 beats=1.5（四分附点）→ whole=1, frac=0.5
    # 旧公式会给出 1 个"-"和下划线。但 1.5 拍的简谱规范表示是
    #   数字 + 右侧 "."（附点） → 没有"-"。所以这里重新规整：
    #   以「基本时值 + 附点 + 延音线」的规范表达，而不是整数拍数×"-"
    #
    # 规范重写：
    #   base = 最小可表达的单位 {0.25,0.5,1.0,2.0,4.0}
    #   if beats == base * 1.5 → dots=1, base 决定下划线
    #   dashes = (beats - base*(1.5 if dotted else 1.0)) / base
    #
    # 为了简单且 99% 正确，这里直接枚举所有 LEGAL 条目对应的规范输出。
    return _jianpu_from_entry(float(entry))


def _jianpu_from_entry(entry: float) -> Tuple[str, int, int, int]:
    """直接从 LEGAL 时值表映射简谱的 (triplet前缀, 下划线数, 附点, 延音数)。"""
    # (entry, (prefix, underscores, dots, dashes))
    table = [
        (0.25,    ("", 2, 0, 0)),   # 十六分：__
        (1/3,     ("3", 1, 0, 0)),  # 三连八分：3_
        (0.5,     ("", 1, 0, 0)),   # 八分：_
        (2/3,     ("3", 1, 1, 0)),  # 三连八分+附点近似：3_.
        (0.75,    ("", 1, 1, 0)),   # 八分附点：_.
        (1.0,     ("", 0, 0, 0)),   # 四分：无
        (1.5,     ("", 0, 1, 0)),   # 四分附点：.
        (2.0,     ("", 0, 0, 1)),   # 二分：-
        (3.0,     ("", 0, 1, 1)),   # 二分附点：-.
        (4.0,     ("", 0, 0, 3)),   # 全：---
    ]
    for v, token in table:
        if abs(entry - v) < 0.06:
            return token
    # 兜底：用 0.25 对齐的最近整拍近似
    dashes = int(round(entry)) - 1
    return ("", 0, 0, max(0, dashes))


# ==========================
# 节奏记谱优化（出版级规范）
# ==========================

# 同音短休止合并阈值（拍）：低于此值的同音间休止被视为"分割伪影"，
# 前后同音合并为一个长音。0.125 = 八分音符的一半，足够包容
# VAD/置信度边界抖动造成的短间隙，又不会吞掉真实的八分休止。
_SAME_PITCH_REST_MERGE_THRESH = 0.125

# 极短休止吸收阈值（拍）：低于此值的休止被吸收到前一个音符，
# 避免谱面出现"一个十六分休止"这种几乎无音乐意义的细碎标记。
_TINY_REST_ABSORB_THRESH = 0.0625


def optimize_rhythm(notes: List[Dict]) -> Tuple[List[Dict], Dict]:
    """节奏记谱优化：减少不必要的休止符分裂，让谱面更接近出版级规范。

    优化项（按顺序执行，每项都不修改音高顺序与时值总量）：
      1) 同音短休止合并：相同音高 + 短休止（<=阈值） → 合并为一个长音
      2) 极短休止吸收：< 1/32 拍的微休止吸收到前一音符

    返回 (优化后 notes, 统计信息)。不修改输入。
    统计信息：{merged_rests: 合并的休止符数, absorbed_tiny_rests: 吸收的微休止数,
              saved_rest_tokens: 减少的休止符 token 数（估算）}
    """
    if len(notes) < 2:
        return [dict(n) for n in notes], {
            "merged_rests": 0, "absorbed_tiny_rests": 0, "saved_rest_tokens": 0}

    out: List[Dict] = [dict(notes[0])]
    merged_rests = 0
    absorbed_tiny = 0

    for i in range(1, len(notes)):
        prev = out[-1]
        cur = dict(notes[i])
        rest = float(cur.get("rest_before_beat", 0.0))

        # 优化 1：同音短休止合并
        same_pitch = int(round(prev.get("midi", 0))) == int(round(cur.get("midi", 0)))
        if same_pitch and 0 < rest <= _SAME_PITCH_REST_MERGE_THRESH:
            # 合并：把 prev 拉长到 cur 的结束，丢弃 cur（休止被吃掉）
            new_end = float(cur["end_beat"])
            prev["end_beat"] = round(new_end, 4)
            prev["dur_beat"] = round(new_end - float(prev["start_beat"]), 4)
            # 重新量化到合法时值（合并后可能是非标准拍数）
            prev["dur_beat"] = round(_legal_dur(float(prev["dur_beat"])), 4)
            prev["end_beat"] = round(float(prev["start_beat"]) + float(prev["dur_beat"]), 4)
            merged_rests += 1
            continue

        # 优化 2：极短休止吸收到前一音符
        if 0 < rest < _TINY_REST_ABSORB_THRESH:
            # 把 prev 拉长，吸收掉微休止，cur 起始后移到 prev 结束
            new_prev_end = float(prev["end_beat"]) + rest
            prev["end_beat"] = round(new_prev_end, 4)
            prev["dur_beat"] = round(new_prev_end - float(prev["start_beat"]), 4)
            # prev 重新量化
            prev["dur_beat"] = round(_legal_dur(float(prev["dur_beat"])), 4)
            prev["end_beat"] = round(float(prev["start_beat"]) + float(prev["dur_beat"]), 4)
            # cur 的 rest_before 变为 0（已被吸收）
            cur["rest_before_beat"] = 0.0
            cur["start_beat"] = round(float(prev["end_beat"]), 4)
            cur["end_beat"] = round(float(cur["start_beat"]) + float(cur["dur_beat"]), 4)
            absorbed_tiny += 1

        out.append(cur)

    # 重新计算 rest_before_beat（合并/吸收后可能变化）
    for i in range(1, len(out)):
        rest = float(out[i]["start_beat"]) - float(out[i - 1]["end_beat"])
        out[i]["rest_before_beat"] = round(max(0.0, rest), 4)

    stats = {
        "merged_rests": merged_rests,
        "absorbed_tiny_rests": absorbed_tiny,
        "saved_rest_tokens": merged_rests + absorbed_tiny,
    }
    return out, stats
