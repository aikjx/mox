# -*- coding: utf-8 -*-
"""歌谱生成层：music21 生成 musicxml + 简谱数字串 + 标准歌谱图片。"""
from typing import Dict, List, Tuple

import librosa

from core import score_sheet

# 公开标准歌谱导出函数，方便从 core.score 一处入口调用
export_score_sheet = score_sheet.export_score


def to_musicxml(notes: List[Dict], bpm: float, key_name: Tuple[str, str], fp=None):
    # music21 懒加载：其顶层导入耗时 1~2s，仅导出 musicxml 时才需要，
    # 不应拖慢 to_jianpu / API / GUI 的常规识别路径。
    from music21 import note as m21note
    from music21 import stream, key as m21key, tempo as m21tempo

    s = stream.Stream()
    s.append(m21key.Key(key_name[0], key_name[1]))
    # 防御 BPM=0：musicxml 元数据与量化都退化为 BPM=120
    eff_bpm = bpm if (bpm and 30.0 <= bpm <= 300.0) else 120.0
    s.append(m21tempo.MetronomeMark(number=int(round(eff_bpm))))

    ql_per_sec = eff_bpm / 60.0
    for nt in notes:
        ql = nt["end"] - nt["start"]
        ql = max(0.25, round(ql * ql_per_sec / 0.25) * 0.25)  # 量化到 1/4 拍
        n = m21note.Note(midi=nt["midi"])
        n.duration.quarterLength = ql
        s.append(n)

    if fp:
        s.write("musicxml", fp=fp)
    return s


def to_jianpu(notes: List[Dict], key_name: Tuple[str, str], bpm: float = 120.0) -> str:
    """转简谱：数字 1–7 表音级；高八度前加 '.'，低八度后加 '_'；'-' 表延音。

    八度基准自适应（修复哼唱/半频锁定满屏低音点）：
      旧版固定以主音 4 八度（C→C4=60）为「1」——哼唱或检测器 halving 使
      音域偏低时，简谱输出 1_ 6__ 4___ 等满屏低音点。简谱记录相对音级，
      现以 adaptive_tonic_midi（中位音符所在八度的主音）为基准，八度点
      分布居中；与 make_score_sheet 图片渲染共用同一基准。

    离调音（修复就近映射 bug）：
      旧版 dd=rel-s 带符号比较，负值恒小于正值 → 一律偏向 scale 末位
      （F#/G# 全被记成 b7，音高差 4 半音）。现按绝对距离就近（等距取
      下方），偏差记 '#'/b'，与 score_sheet._degree_accidental 一致。
    """
    from core.score_sheet import adaptive_tonic_midi, _name_to_pitch_class, _scale_for

    tonic_pc = _name_to_pitch_class(key_name[0])
    tonic_midi = adaptive_tonic_midi(notes, tonic_pc)
    scale = list(_scale_for(key_name[1]))

    out = []
    for nt in notes:
        m = int(nt["midi"])
        d = m - tonic_midi
        oct_shift = d // 12
        rel = d % 12
        if rel in scale:
            deg = str(scale.index(rel) + 1)
        else:
            # 离调音：绝对距离就近映射（等距取下方音级，升下方音是记谱惯例）
            best, best_dist, best_dev = scale[0], 99, 0
            for s in scale:
                dev = (rel - s) % 12
                dist = min(dev, 12 - dev)
                if dist < best_dist:
                    best_dist, best, best_dev = dist, s, dev
            acc = "#" if best_dev <= 2 else "b"
            deg = acc + str(scale.index(best) + 1)
        if oct_shift > 0:
            deg = "." * oct_shift + deg
        elif oct_shift < 0:
            deg = deg + "_" * (-oct_shift)
        # BPM 不可靠（哼唱常检测为 0）时退化为按固定 0.25s/拍量化，避免除零崩溃
        beat_dur = 60.0 / bpm if bpm and bpm > 0 else 0.25
        # 时值量化：按拍数四舍五入到最近整数拍（文本简谱的 "-" 只能表达整数拍，
        # 精确时值由 LilyPond 专业渲染负责）。旧公式 round(dur/beat/0.25)-1
        # 会把 0.86 拍量化成 3 拍（"1--"），严重失真。
        beats = max(1, int(round((nt["end"] - nt["start"]) / beat_dur)))
        out.append(deg + "-" * (beats - 1))
    return " ".join(out)
