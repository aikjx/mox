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

# matplotlib 懒加载：仅当真正导出歌谱图片（export_score）时才 import。
# 原因：matplotlib 顶层 import 会触发 font_manager 全量扫描系统字体，
# 在导入 core.score_sheet 时（gui 启动即触发）可造成数秒阻塞，是启动卡顿主因。
HAS_MPL = False          # 首次调用 _load_mpl() 后置 True
_fm = None               # matplotlib.font_manager
_Figure = None           # matplotlib.figure.Figure
_FigureCanvasPdf = None  # matplotlib.backends.backend_pdf.FigureCanvasPdf


def _load_mpl() -> bool:
    """懒加载 matplotlib（幂等）。返回是否可用。"""
    global HAS_MPL, _fm, _Figure, _FigureCanvasPdf
    if HAS_MPL:
        return True
    try:
        import matplotlib
        matplotlib.use("Agg")
        from matplotlib import font_manager as _fm_mod
        from matplotlib.backends.backend_pdf import FigureCanvasPdf as _FCP
        from matplotlib.figure import Figure as _Fig
        _fm, _Figure, _FigureCanvasPdf = _fm_mod, _Fig, _FCP
        HAS_MPL = True
        return True
    except Exception:  # pragma: no cover - 允许无 GUI/无字体环境降级
        HAS_MPL = False
        return False


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


def _ensure_font() -> Optional[str]:
    """尝试找一个能显示中文的字体；找不到则返回 None，matplotlib 会回退。"""
    candidates = [
        # Windows 常见中文字体
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simsun.ttc",
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msgothic.ttc",
        # macOS
        "/System/Library/Fonts/PingFang.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
        # Linux
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    for p in candidates:
        if os.path.exists(p):
            try:
                prop = _fm.FontProperties(fname=p)
                return prop
            except Exception:
                continue
    return None


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


def _format_jianpu_symbol(degree: int, octave_dot: int, duration_label: str) -> str:
    """生成一个简谱符号的“字符层”表示，供绘图层使用。"""
    body = str(degree)
    dots = ""
    if octave_dot > 0:
        dots = "·" * min(octave_dot, 3)
    elif octave_dot < 0:
        dots = "," * min(abs(octave_dot), 3)

    # 下划线数量：eighth=1, sixteenth=2（仅示意，不精确附点）
    under = ""
    if "eighth" in duration_label and "dotted" not in duration_label:
        under = "_"
    elif "sixteenth" in duration_label:
        under = "="
    elif "dotted-quarter" == duration_label:
        under = "."  # 用下点表示附点，实际绘制再处理
    return body, dots, under


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


def _group_by_bar(notes: List[RenderNote]) -> List[List[RenderNote]]:
    """按小节索引分组并排序。空小节不绘制（避免首行密集后续空行）。"""
    if not notes:
        return []
    max_bar = max(n.bar_idx for n in notes)
    bars: List[List[RenderNote]] = [[] for _ in range(max_bar + 1)]
    for n in notes:
        bars[n.bar_idx].append(n)
    for b in bars:
        b.sort(key=lambda x: x.start_beat)
    # 过滤空小节，但保留相对位置用于节拍标号稳定（用 None 占位）
    non_empty: List[List[RenderNote]] = [b for b in bars if b]
    return non_empty


def _layout_bars(bars: List[List[RenderNote]], bars_per_row: int = 4) -> List[List[List[RenderNote]]]:
    """把小节排成多行。"""
    rows = []
    for i in range(0, len(bars), bars_per_row):
        rows.append(bars[i : i + bars_per_row])
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


def draw_score_sheet(
    sheet: ScoreSheet,
    output_path: str,
    bars_per_row: int = 4,
    width_px: int = 1200,
    dpi: int = 150,
) -> str:
    """绘制标准歌谱并保存为 PNG/PDF/SVG。

    Args:
        sheet: 已构造好的歌谱数据结构。
        output_path: 输出文件路径，后缀决定格式（.png/.pdf/.svg）。
        bars_per_row: 每行小节数。
        width_px: 图片宽度（像素，仅 PNG 有效）。
        dpi: 分辨率。

    Returns:
        保存后的文件路径。
    """
    if not _load_mpl():
        raise RuntimeError("绘制歌谱需要 matplotlib，请执行：pip install matplotlib")

    ext = os.path.splitext(output_path)[1].lower()
    is_pdf = ext == ".pdf"

    # PDF 用英寸尺寸，PNG 用像素换算
    width_in = width_px / dpi
    rows = _layout_bars(_group_by_bar(sheet.notes), bars_per_row=bars_per_row)
    # 预估行高：标题 + 调号 + 每行谱表 + 间距
    row_height_in = 1.2
    header_height_in = 1.3
    height_in = header_height_in + len(rows) * row_height_in + 0.6

    fig = _Figure(figsize=(width_in, height_in), dpi=dpi)
    fig.patch.set_facecolor("white")
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_xlim(0, width_px)
    ax.set_ylim(0, height_in * dpi)
    ax.invert_yaxis()
    ax.axis("off")

    font_prop = _ensure_font()
    title_font = _fm.FontProperties(family="sans-serif", size=26, weight="bold")
    if font_prop:
        title_font = _fm.FontProperties(fname=font_prop.get_file(), size=26, weight="bold")

    info_font = _fm.FontProperties(family="sans-serif", size=12)
    if font_prop:
        info_font = _fm.FontProperties(fname=font_prop.get_file(), size=12)

    note_font = _fm.FontProperties(family="monospace", size=20)
    if font_prop:
        note_font = _fm.FontProperties(fname=font_prop.get_file(), size=20)

    # 1. 标题
    y_cursor = 50
    ax.text(width_px / 2, y_cursor, sheet.title, fontproperties=title_font,
            ha="center", va="top", color="black")
    y_cursor += 50

    # 2. 副标题信息
    mode_str = "大调" if sheet.key_mode == "major" else "小调"
    info_line = f"1={sheet.key_tonic}    {sheet.time_sig[0]}/{sheet.time_sig[1]}    速度={int(round(sheet.bpm))}"
    if sheet.tuning:
        info_line += f"    定弦：{sheet.tuning}"
    if sheet.composer:
        info_line += f"    编曲：{sheet.composer}"
    ax.text(60, y_cursor, info_line, fontproperties=info_font,
            ha="left", va="top", color="#333333")
    y_cursor += 50

    # 3. 谱表绘制
    left_margin = 60
    right_margin = 40
    row_top = y_cursor
    usable_width = width_px - left_margin - right_margin
    row_gap = 30
    bar_width = usable_width / bars_per_row

    def draw_bar(x, y, bar, idx, first_in_row=False):
        h_staff = 70
        # 小节线
        ax.plot([x, x], [y, y + h_staff], color="black", linewidth=1.2)
        if first_in_row:
            # 首小节左侧加终止线风格的双纵线（可选）
            ax.plot([x, x], [y, y + h_staff], color="black", linewidth=2.5)
        # 终止线
        ax.plot([x + bar_width, x + bar_width], [y, y + h_staff], color="black", linewidth=1.2)

        if not bar:
            return

        total_beats = sheet.beats_per_bar
        for n in bar:
            qdur, label = _quantize_duration(n.dur_beats)
            x_note = x + (n.start_beat / total_beats) * bar_width + 12
            y_note = y + h_staff / 2 + 6

            body, dots, under = _format_jianpu_symbol(n.degree, n.octave_dot, label)

            # 数字
            ax.text(x_note, y_note, body, fontproperties=note_font,
                    ha="center", va="center", color="black")

            # 高音点
            if dots and "," not in dots:
                ax.text(x_note, y_note - 24, dots, fontproperties=note_font,
                        ha="center", va="center", color="black")
            # 低音点（用下标逗号）
            if "," in dots:
                ax.text(x_note, y_note + 22, dots, fontproperties=note_font,
                        ha="center", va="center", color="black")

            # 下划线（八分/十六分）
            if "_" in under:
                ax.plot([x_note - 10, x_note + 10], [y_note + 16, y_note + 16],
                        color="black", linewidth=1.5)
            elif "=" in under:
                ax.plot([x_note - 10, x_note + 10], [y_note + 16, y_note + 16],
                        color="black", linewidth=1.5)
                ax.plot([x_note - 10, x_note + 10], [y_note + 22, y_note + 22],
                        color="black", linewidth=1.5)
            elif "." in under:
                # 附点
                ax.plot(x_note + 12, y_note + 4, marker=".", markersize=6, color="black")

    for row_idx, row in enumerate(rows):
        y = row_top + row_idx * (80 + row_gap)
        for col, bar in enumerate(row):
            x = left_margin + col * bar_width
            draw_bar(x, y, bar, row_idx * bars_per_row + col,
                     first_in_row=(col == 0))

    # 页脚
    footer_y = height_in * dpi - 20
    ax.text(width_px / 2, footer_y, "— Melody2Score 自动生成 —",
            fontproperties=info_font, ha="center", va="bottom", color="#888888")

    os.makedirs(os.path.dirname(os.path.abspath(output_path)) or ".", exist_ok=True)

    if is_pdf:
        # PDF 使用矢量后端，保持 figsize 英寸
        canvas = _FigureCanvasPdf(fig)
        canvas.print_figure(output_path, dpi=dpi)
    elif ext == ".svg":
        fig.savefig(output_path, format="svg", dpi=dpi, facecolor="white")
    else:
        fig.savefig(output_path, format="png", dpi=dpi, facecolor="white")

    return output_path


def export_score(
    notes: List[dict],
    key: dict,
    bpm: float,
    output_path: str,
    title: str = "未命名旋律",
    time_sig: Tuple[int, int] = (4, 4),
    composer: str = "",
    tuning: str = "",
    bars_per_row: int = 4,
    width_px: int = 1200,
    dpi: int = 150,
) -> str:
    """一站式导出：直接接受 pipeline notes 字典，生成标准歌谱图片。

    渲染后端优先使用现成第三方库（jianpu-ly + LilyPond），
    仅在第三方依赖缺失或失败时降级到内置的 matplotlib 手绘。
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
    try:
        from . import jianpu_render
        return jianpu_render.render_score_sheet(
            sheet=sheet,
            output_path=output_path,
            dpi=dpi,
        )
    except Exception as exc:  # pragma: no cover - 第三方依赖缺失时的兜底
        # 记录原因后回退到 matplotlib 手绘，保证可用性
        import sys
        print(f"[score_sheet] 第三方简谱渲染不可用，降级 matplotlib：{exc}",
              file=sys.stderr)
        return draw_score_sheet(
            sheet=sheet,
            output_path=output_path,
            bars_per_row=bars_per_row,
            width_px=width_px,
            dpi=dpi,
        )
