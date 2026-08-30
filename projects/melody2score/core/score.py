# -*- coding: utf-8 -*-
"""歌谱生成层：music21 生成 musicxml + 简谱数字串 + 标准歌谱图片。

v2 改进：
  - 所有量化走 core.quantize（柔性 1/8 拍吸附 + 保留附点/切分/三连）；
  - 禁止粗暴等分（旧 0.25 拍 round / 整数拍 round），否则 BPM 一错
    整条谱节奏跟着完全跑偏。
"""
from typing import Dict, List, Tuple

from core import score_sheet
from core.quantize import quantize_notes, quarter_length, jianpu_dur_tokens, optimize_rhythm

# 公开标准歌谱导出函数，方便从 core.score 一处入口调用
export_score_sheet = score_sheet.export_score


def to_musicxml(notes: List[Dict], bpm: float, key_name: Tuple[str, str], fp=None):
    """musicxml 导出（用柔性拍网格量化，保留附点/切分/三连音）。

    旧版 bug 修复：
      - 不再用「(end-start)*ql_per_sec / 0.25 → round → ×0.25」粗暴等分，
        所有时值附点/切分会被吃掉；
      - 相邻重叠音自动后推起始位，避免 music21 在并行单声部流里崩溃。
    """
    # music21 懒加载：其顶层导入耗时 1~2s，仅导出 musicxml 时才需要
    from music21 import note as m21note
    from music21 import stream, key as m21key, tempo as m21tempo

    eff_bpm = bpm if (bpm and 30.0 <= bpm <= 300.0) else 120.0
    qnotes = quantize_notes(notes, eff_bpm)
    # 节奏记谱优化：合并同音短休止、吸收微休止，减少谱面细碎感
    qnotes, _ = optimize_rhythm(qnotes)

    s = stream.Stream()
    s.append(m21key.Key(key_name[0], key_name[1]))
    s.append(m21tempo.MetronomeMark(number=int(round(eff_bpm))))

    for qn in qnotes:
        ql = quarter_length(qn["dur_beat"])
        # 休止（与上一音之间的间隙）以 Rest 插入——让 musicxml 节拍正确
        if qn.get("rest_before_beat", 0) > 0.02:
            r = m21note.Rest()
            r.duration.quarterLength = quarter_length(qn["rest_before_beat"])
            s.append(r)
        n = m21note.Note(midi=int(qn["midi"]))
        n.duration.quarterLength = ql
        s.append(n)

    if fp:
        s.write("musicxml", fp=fp)
    return s


def to_jianpu(notes: List[Dict], key_name: Tuple[str, str],
               bpm: float = 120.0, unicode_octave: bool = False) -> str:
    """转简谱 v2：量化到合法音乐时值，附点/半拍/三连音可视化表达。

    旧版只会输出整数拍的 "-"，导致所有附点/切分都丢了。新版：
      - 下划线 `__`（十六分）、`_`（八分）表达短于一拍；
      - 右侧 `.` 表示附点（时值 × 1.5）；
      - 右侧 `-` 表示延音（整数拍延续）；
      - 前缀 `3` 表示三连音八分近似。

    v3 新增 unicode_octave 模式（出版级规范，解决高低音点与减时线混淆）：
      - 高音点：Unicode 组合上点 U+0307（如 1̇ 2̇），写在数字上方
      - 低音点：Unicode 组合下点 U+0323（如 1̣ 2̣），写在数字下方
      - 减时线：仍用 `_` 前缀，与高低音点彻底区分
      - 附点：仍用 `.` 后缀，在数字右侧，与高低音点不重叠
    """
    from core.score_sheet import adaptive_tonic_midi, _name_to_pitch_class, _scale_for

    tonic_pc = _name_to_pitch_class(key_name[0])
    tonic_midi = adaptive_tonic_midi(notes, tonic_pc)
    scale = list(_scale_for(key_name[1]))

    eff_bpm = bpm if (bpm and bpm > 0) else 120.0
    qnotes = quantize_notes(notes, eff_bpm)
    # 节奏记谱优化：合并同音短休止、吸收微休止，减少谱面细碎感
    qnotes, _ = optimize_rhythm(qnotes)

    out = []
    for qn in qnotes:
        m = int(qn["midi"])
        d = m - tonic_midi
        oct_shift = d // 12
        rel = d % 12
        if rel in scale:
            deg = str(scale.index(rel) + 1)
        else:
            # 离调音：绝对距离就近映射（等距取下方）
            best, best_dist, best_dev = scale[0], 99, 0
            for s in scale:
                dev = (rel - s) % 12
                dist = min(dev, 12 - dev)
                if dist < best_dist:
                    best_dist, best, best_dev = dist, s, dev
            acc = "#" if best_dev <= 2 else "b"
            deg = acc + str(scale.index(best) + 1)

        # 八度点
        if unicode_octave:
            # Unicode 规范模式：上点/下点组合字符，与减时线/附点彻底无歧义
            if oct_shift > 0:
                # 高音点：在数字后加组合上点（多个八度加多个点）
                deg = deg + "\u0307" * oct_shift
            elif oct_shift < 0:
                # 低音点：在数字后加组合下点（多个八度加多个点）
                deg = deg + "\u0323" * (-oct_shift)
        else:
            # 兼容模式：高音前缀 '.'，低音后缀 '_'
            # （注意：'_' 同时用作减时线前缀，易混淆——推荐使用 unicode_octave=True）
            if oct_shift > 0:
                deg = "." * oct_shift + deg
            elif oct_shift < 0:
                deg = deg + "_" * (-oct_shift)

        # 时值修饰（v2：附点+半拍+三连 显式）
        prefix, underscores, dots, dashes = jianpu_dur_tokens(qn["dur_beat"])
        # 下划线（减时线）：统一放在最前面，与高低音点区分
        core = deg
        if underscores == 2:
            core = "__" + core
        elif underscores == 1:
            core = "_" + core
        core = prefix + core
        core += "." * dots        # 附点（数字右侧）
        core += "-" * dashes      # 延音线
        out.append(core)
    return " ".join(out)
