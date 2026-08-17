# -*- coding: utf-8 -*-
"""歌谱生成层：music21 生成 musicxml + 简谱数字串。"""
from typing import Dict, List, Tuple

import numpy as np
import librosa
from music21 import note as m21note
from music21 import stream, key as m21key, tempo as m21tempo


def to_musicxml(notes: List[Dict], bpm: float, key_name: Tuple[str, str], fp=None):
    s = stream.Stream()
    s.append(m21key.Key(key_name[0], key_name[1]))
    s.append(m21tempo.MetronomeMark(number=int(round(bpm))))

    ql_per_sec = bpm / 60.0
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
        deg = str(scale.index(rel) + 1) if rel in scale else "#"  # 离调音近似标 #
        if oct_shift > 0:
            deg = "." * oct_shift + deg
        elif oct_shift < 0:
            deg = deg + "_" * (-oct_shift)
        ext = max(0, int(round((nt["end"] - nt["start"]) / (60.0 / bpm) / 0.25)) - 1)
        out.append(deg + "-" * ext)
    return " ".join(out)
