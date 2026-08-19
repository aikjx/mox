# -*- coding: utf-8 -*-
"""歌谱生成层：music21 生成 musicxml + 简谱数字串 + 标准歌谱图片。"""
from typing import Dict, List, Tuple

import numpy as np
import librosa
from music21 import note as m21note
from music21 import stream, key as m21key, tempo as m21tempo

from core import score_sheet

# 公开标准歌谱导出函数，方便从 core.score 一处入口调用
export_score_sheet = score_sheet.export_score


def to_musicxml(notes: List[Dict], bpm: float, key_name: Tuple[str, str], fp=None):
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

    以 tonic 的 4 八度音（如 C→C4=60）为简谱「1」的基准八度：
      oct_shift = (midi - tonic_midi) // 12  → 相对基准的八度偏移；
      rel = (midi - tonic_midi) % 12         → 在调内音级索引。
    这样既保证调内音级正确，又保证八度点标记符合记谱习惯。
    """
    tonic_midi = int(librosa.note_to_midi(key_name[0] + "4"))  # 含八度，如 C4=60
    if key_name[1] == "minor":
        scale = [0, 2, 3, 5, 7, 8, 10]
    else:
        scale = [0, 2, 4, 5, 7, 9, 11]

    out = []
    for nt in notes:
        m = int(nt["midi"])
        d = m - tonic_midi
        oct_shift = d // 12
        rel = d % 12
        if rel in scale:
            deg = str(scale.index(rel) + 1)
        else:
            # 离调音：回退到最近的调内音级，并用升降记号标出偏离，避免直接显示 '#'
            best, best_d = 0, 99
            for s in scale:
                dd = rel - s
                dd = min(dd, 12 - dd) if dd > 6 else dd
                if dd < best_d:
                    best_d, best = dd, s
            acc = "#" if (rel - best) % 12 in (1, 3, 6, 8, 10) else "b"
            deg = acc + str(scale.index(best) + 1)
        if oct_shift > 0:
            deg = "." * oct_shift + deg
        elif oct_shift < 0:
            deg = deg + "_" * (-oct_shift)
        # BPM 不可靠（哼唱常检测为 0）时退化为按固定 0.25s/拍量化，避免除零崩溃
        beat_dur = 60.0 / bpm if bpm and bpm > 0 else 0.25
        ext = max(0, int(round((nt["end"] - nt["start"]) / beat_dur / 0.25)) - 1)
        out.append(deg + "-" * ext)
    return " ".join(out)
