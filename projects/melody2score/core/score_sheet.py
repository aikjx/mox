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
    accidental: str = ""   # 离调音升降记号 '#'|'b'|''（图片渲染不丢半音）


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


# 大/小调音级（半音偏移，相对主音）。degree 就近映射共用此表，
# 保证文本简谱（score.to_jianpu）与图片渲染（make_score_sheet）一致。
_MAJOR_SCALE = (0, 2, 4, 5, 7, 9, 11)
_MINOR_SCALE = (0, 2, 3, 5, 7, 8, 10)


def _scale_for(mode: str):
    return _MINOR_SCALE if str(mode).lower().startswith("min") else _MAJOR_SCALE


def _degree_accidental(midi: int, tonic_pc: int, mode: str = "major"):
    """midi → (唱名 1-7, 升降记号 '#'|'b'|'')。

    调内音：直接音级。离调音：绝对距离最近的调内音级（等距取下方——
    升下方音是记谱惯例），偏差 ≤2 半音记 '#'、≥10 记 'b'（黑键距
    调内白键必 ≤1，无中间值）。替代旧版「负距离偏向 scale 末位」的
    错误映射（F# 曾被记成 b7，音高差 4 半音）。
    """
    rel = (int(midi) - tonic_pc) % 12
    scale = _scale_for(mode)
    if rel in scale:
        return scale.index(rel) + 1, ""
    best, best_dist, best_dev = scale[0], 99, 0
    for s in scale:
        dev = (rel - s) % 12
        dist = min(dev, 12 - dev)
        if dist < best_dist:          # 严格小于：等距保留先扫到的下方音
            best_dist, best, best_dev = dist, s, dev
    acc = "#" if best_dev <= 2 else "b"
    return scale.index(best) + 1, acc


def _midi_to_degree(midi: int, tonic_pc: int) -> int:
    """以指定主音为 1，把 midi 映射到简谱唱名 1-7（大调；兼容旧签名）。"""
    return _degree_accidental(midi, tonic_pc, "major")[0]


def adaptive_tonic_midi(notes: List[dict], tonic_pc: int) -> int:
    """按旋律实际音域自适应选择 tonic 基准八度（修复哼唱低八度满屏低音点）。

    旧版基准固定为主音 4 八度（C→C4）：哼唱/检测音域偏低时简谱满屏 '_'
    （低音点），且检测器半频锁定时叠加成 1__ 6___ 等。简谱记录相对音级，
    绝对八度不承载乐义——以「中位音符所在八度的主音」为基准，中位音符
    恒落在无点中音区，八度点分布居中，符合人声记谱惯例。

    to_jianpu 与 make_score_sheet 共用此基准，保证文本与图片一致。
    """
    if not notes:
        return 60 + (tonic_pc % 12)
    mids = sorted(int(round(float(n.get("midi", 60)))) for n in notes)
    med = mids[len(mids) // 2]
    return med - ((med - tonic_pc) % 12)


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
    # 自适应 tonic 基准（循环外一次计算）：中位音符所在八度的主音为无点区
    base_midi = adaptive_tonic_midi(notes, tonic_pc)
    for n in notes:
        start = float(n.get("start", 0.0))
        dur = float(n.get("dur", 0.0))
        midi = int(round(float(n.get("midi", 0))))
        name = n.get("name", "")
        degree, accidental = _degree_accidental(midi, tonic_pc, mode)

        # 八度点：以「自适应 tonic 基准」（中位音符所在八度的主音）为无点
        # 中音区——与 to_jianpu 共用 adaptive_tonic_midi，文本与图片一致。
        # 旧版固定 C4 参考：哼唱/检测音域偏低时图片满屏低音点。
        octave_dot = (midi - base_midi) // 12  # floor：负数向下，数学正确

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
            accidental=accidental,
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
                accidental=n.accidental,
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
    dpi: int = 200,
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
