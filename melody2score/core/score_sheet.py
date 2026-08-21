# -*- coding: utf-8 -*-
"""标准歌谱生成器：把识别出的音符绘制成可打印/可导出的规范简谱图片。

支持输出：
    - PNG（位图，适合屏幕预览）
    - PDF（矢量，适合打印出版）
    - SVG（矢量，适合二次编辑）

绘制风格贴近常见器乐/声乐简谱：
    - 标题、调号、拍号、速度
    - 小节线分隔
    - 数字音符 + 高音点/低音点
    - 时值下划线（八分、十六分）
    - 歌词/副旋律占位行（可扩展）
"""
from __future__ import annotations

import os
from dataclasses import dataclass
from typing import List, Optional, Tuple

import numpy as np


@dataclass
class RenderNote:
    """渲染层音符，已脱离 pipeline 内部字典。"""
    name: str          # 音名，如 C4 / A#5
    midi: int
    degree: int        # 简谱唱名 1-7（以调式主音为 1）
    octave_dot: int    # >0 高音点，<0 低音点
    start_beat: float  # 以拍为单位的小节内起始
    dur_beats: float   # 以拍为单位的时值
    bar_idx: int       # 所属小节序号
    lyric: str = ""


@dataclass
class ScoreSheet:
    """渲染参数与输出路径集合。"""
    title: str
    key_tonic: str
    key_mode: str
    time_sig: Tuple[int, int]
    bpm: float
    beats_per_bar: int
    notes: List[RenderNote]
    composer: str = ""
    tuning: str = ""


def _midi_to_degree(midi: int, tonic_pc: int) -> int:
    """以指定主音为 1，把 midi 映射到简谱唱名 1-7。"""
    pc = midi % 12
    rel = (pc - tonic_pc) % 12
    # 十二平均律：1=C, 2=D, 3=E, 4=F, 5=G, 6=A, 7=B（自然大调近似）
    mapping = {0: 1, 2: 2, 4: 3, 5: 4, 7: 5, 9: 6, 11: 7}
    return mapping.get(rel, mapping.get(rel - 1, 1))  # 升降号取相邻本位


def _name_to_pitch_class(name: str) -> int:
    """音名 -> 十二平均律 pitch class。"""
    base = {"C": 0, "D": 2, "E": 4, "F": 5, "G": 7, "A": 9, "B": 11}
    n = name.strip().upper()
    if n.startswith("C#") or n.startswith("DB"):
        return 1
    if n.startswith("D#") or n.startswith("EB"):
        return 3
    if n.startswith("F#") or n.startswith("GB"):
        return 6
    if n.startswith("G#") or n.startswith("AB"):
        return 8
    if n.startswith("A#") or n.startswith("BB"):
        return 10
    for k, v in base.items():
        if n.startswith(k):
            return v
    return 0


def _quantize_duration(dur_beats: float) -> Tuple[float, str]:
    """把浮点拍数量化到常见音符时值，返回 (量化拍数, 装饰描述)。"""
    standards = [
        (4.0, "whole"), (3.0, "dotted-half"), (2.0, "half"),
        (1.5, "dotted-quarter"), (1.0, "quarter"),
        (0.75, "dotted-eighth"), (0.5, "eighth"),
        (0.25, "sixteenth"),
    ]
    best, best_label = 1.0, "quarter"
    best_err = abs(dur_beats - 1.0)
    for val, label in standards:
        err = abs(dur_beats - val)
        if err < best_err:
            best_err = err
            best, best_label = val, label
    return best, best_label


def make_score_sheet(
    notes: List[dict],
    key: dict,
    bpm: float,
    title: str = "未命名旋律",
    time_sig: Tuple[int, int] = (4, 4),
    composer: str = "",
    tuning: str = "",
) -> ScoreSheet:
    """把 pipeline 输出的 notes 转换为规范歌谱数据结构。"""
    tonic = key.get("tonic", "C")
    mode = key.get("mode", "major")
    tonic_pc = _name_to_pitch_class(tonic)

    beats_per_bar = time_sig[0]
    beat_unit = time_sig[1]
    # 稳健 beat_dur：
    #   1) bpm 可信(30-300) → 用 bpm；
    #   2) 否则用音符中位时长自洽推导（一拍 ≈ 一个中位时长音符）；
    #   3) 还不行（无 notes）→ 退到 0.5s/拍。
    if bpm and 30.0 <= float(bpm) <= 300.0:
        beat_dur = 60.0 / float(bpm)
    else:
        durs = [float(n.get("dur", 0.0)) for n in notes]
        durs = [d for d in durs if 0.05 < d < 4.0]
        beat_dur = float(np.median(durs)) if durs else 0.5

    render_notes: List[RenderNote] = []
    for n in notes:
        start = float(n.get("start", 0.0))
        dur = float(n.get("dur", 0.0))
        midi = int(round(float(n.get("midi", 0))))
        name = n.get("name", "")
        degree = _midi_to_degree(midi, tonic_pc)

        # 计算相对于 C4 的八度点：C4 为中音 1
        ref_midi = 60  # C4
        octave_steps = midi - ref_midi
        # 每个八度 12 半音；唱名 1(C) 位于每八度底部
        octave = octave_steps // 12
        # 若余数为负，多降一个八度
        if octave_steps < 0 and (midi % 12) < (ref_midi % 12):
            octave -= 1
        octave_dot = octave  # >0 高音点，<0 低音点

        start_beat = start / beat_dur
        dur_beats = max(0.05, dur / beat_dur)  # 下限保护，避免零长度黑块
        bar_idx = int(start_beat // beats_per_bar)

        render_notes.append(RenderNote(
            name=name or "",
            midi=midi,
            degree=degree,
            octave_dot=octave_dot,
            start_beat=start_beat,
            dur_beats=dur_beats,
            bar_idx=bar_idx,
        ))

    return ScoreSheet(
        title=title,
        key_tonic=tonic,
        key_mode=mode,
        time_sig=time_sig,
        bpm=bpm,
        beats_per_bar=beats_per_bar,
        notes=render_notes,
        composer=composer,
        tuning=tuning,
    )


def _layout_rows(notes: List[RenderNote], notes_per_row: int = 16) -> List[List[RenderNote]]:
    """把音符按每行近似数量切成多行（仅供 jianpu-ly 文本分段参考）。"""
    rows = []
    for i in range(0, len(notes), notes_per_row):
        rows.append(notes[i : i + notes_per_row])
    return rows


def _split_long_notes(bar_notes: List[RenderNote], beats_per_bar: int) -> List[RenderNote]:
    """跨小节长音切分，保证每小节内拍数正确。"""
    out: List[RenderNote] = []
    for n in bar_notes:
        start = n.start_beat % beats_per_bar
        remaining = n.dur_beats
        cur_bar = n.bar_idx
        while remaining > 0:
            room = beats_per_bar - (start if cur_bar == n.bar_idx else 0)
            take = min(remaining, room)
            out.append(RenderNote(
                name=n.name,
                midi=n.midi,
                degree=n.degree,
                octave_dot=n.octave_dot,
                start_beat=start if cur_bar == n.bar_idx else 0.0,
                dur_beats=take,
                bar_idx=cur_bar,
            ))
            remaining -= take
            cur_bar += 1
            start = 0.0
    return out


def export_score(
    notes: List[dict],
    key: dict,
    bpm: float,
    output_path: str,
    title: str = "未命名旋律",
    time_sig: Tuple[int, int] = (4, 4),
    composer: str = "",
    tuning: str = "",
    dpi: int = 150,
) -> str:
    """一站式导出：直接接受 pipeline notes 字典，生成标准歌谱图片。

    渲染后端使用规范第三方工具链（jianpu-ly 简谱记法 + LilyPond 排版），
    不自写渲染器。若 LilyPond/jianpu-ly 缺失则直接报错，提示安装。
    """
    sheet = make_score_sheet(
        notes=notes,
        key=key,
        bpm=bpm,
        title=title,
        time_sig=time_sig,
        composer=composer,
        tuning=tuning,
    )
    from . import jianpu_render
    return jianpu_render.render_score_sheet(
        sheet=sheet,
        output_path=output_path,
        dpi=dpi,
    )
