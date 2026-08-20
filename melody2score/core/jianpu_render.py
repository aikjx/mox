# -*- coding: utf-8 -*-
"""简谱图片渲染后端（现成第三方库）：jianpu-ly + LilyPond。

替换原先的 matplotlib 手绘方案。职责拆分：
    - jianpu-ly  ：成熟的简谱排版预处理器（Silas S. Brown, Apache-2.0），
                   把简谱文本转成 LilyPond 源码。脚本位于 lib/jianpu-ly.py。
    - LilyPond   ：专业乐谱排版引擎，输出 PNG / PDF / SVG。

本模块只做"把 ScoreSheet 适配成 jianpu-ly 文本 + 调用两个现成工具"，
不再手绘任何音符/小节线/下划线 —— 排版全部交给第三方引擎。

外部依赖：
    - LilyPond 需安装（Windows 可用 `winget install LilyPond.LilyPond`）。
    - lib/jianpu-ly.py 已随仓库附带。
"""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
from typing import List, Optional, Tuple

from .score_sheet import ScoreSheet, RenderNote

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
JIANPU_LY = os.path.join(ROOT, "lib", "jianpu-ly.py")


def find_lilypond() -> Optional[str]:
    """定位 lilypond 可执行文件；找不到返回 None。"""
    candidates: List[str] = []

    # 1) PATH
    p = shutil.which("lilypond")
    if p:
        candidates.append(p)

    # 2) winget 默认安装目录（用户级）
    base = os.path.expanduser(r"~\AppData\Local\Microsoft\WinGet\Packages")
    if os.path.isdir(base):
        for d in os.listdir(base):
            if "lilypond" not in d.lower():
                continue
            for cand in (
                os.path.join(base, d, "lilypond-2.24.4", "bin", "lilypond.exe"),
                os.path.join(base, d, "bin", "lilypond.exe"),
            ):
                if os.path.exists(cand):
                    candidates.append(cand)

    # 3) 常见固定路径
    for cand in (
        r"C:\Program Files\LilyPond\bin\lilypond.exe",
        "/usr/bin/lilypond",
        "/usr/local/bin/lilypond",
    ):
        if os.path.exists(cand):
            candidates.append(cand)

    return candidates[0] if candidates else None


def _oct_marks(n: RenderNote) -> str:
    """返回 octave_dot 对应的 jianpu-ly 八度记号（' 高 / , 低）。"""
    if n.octave_dot > 0:
        return "'" * min(n.octave_dot, 3)
    if n.octave_dot < 0:
        return "," * min(-n.octave_dot, 3)
    return ""


# jianpu-ly 可靠时值记号（经实测验证）：
#   四分=1, 八分=q1(0.5), 二分=1 -(2), 全=1 - - -(4), 附点四分=1.(1.5), 附点二分=1 - -(3)
# 为避开 jianpu-ly 对附点八分(0.75)/十六分(0.25)解析不稳定的问题，
# 这里把所有时值安全化为 {0.5, 1, 1.5, 2, 3, 4} 集合（0.25/0.75 并入 0.5）。
def _safe_beats(raw: float) -> float:
    """把原始拍数归一化到 jianpu-ly 可靠时值集。"""
    from .score_sheet import _quantize_duration
    _qdur, label = _quantize_duration(raw)
    beat_map = {
        "whole": 4.0, "dotted-half": 3.0, "half": 2.0,
        "dotted-quarter": 1.5, "quarter": 1.0,
        "dotted-eighth": 0.75, "eighth": 0.5, "sixteenth": 0.25,
    }
    b = beat_map.get(label, 1.0)
    # 0.75 / 0.25 不稳定 -> 并入 0.5（八分），确保可精确表达
    if abs(b - 0.75) < 1e-6 or abs(b - 0.25) < 1e-6:
        return 0.5
    return b


def _dur_token_for(degree: int, oct_marks: str, beats: float) -> str:
    """生成可靠时值集内拍数对应的 jianpu-ly 记号。

    beats<=0 返回空串。degree 为 0 时表示休止符（用 '0' 系列）。
    """
    if beats <= 0:
        return ""
    deg = "0" if degree == 0 else str(degree)
    if abs(beats - 4.0) < 1e-6:
        return f"{deg}{oct_marks} - - -"
    if abs(beats - 3.0) < 1e-6:
        return f"{deg}{oct_marks} - -"
    if abs(beats - 2.0) < 1e-6:
        return f"{deg}{oct_marks} -"
    if abs(beats - 1.5) < 1e-6:
        return f"{deg}{oct_marks}."
    if abs(beats - 1.0) < 1e-6:
        return f"{deg}{oct_marks}"
    if abs(beats - 0.5) < 1e-6:
        return f"q{deg}{oct_marks}"         # 八分（0.5）
    # 其它（理论上不会发生，因已安全化）：回退八分
    return f"q{deg}{oct_marks}"


def _note_token(n: RenderNote) -> Tuple[str, float]:
    """把音符转成 (jianpu-ly 记号, 安全化后拍数)。"""
    if n.octave_dot > 0:
        oct_marks = "'" * min(n.octave_dot, 3)
    elif n.octave_dot < 0:
        oct_marks = "," * min(-n.octave_dot, 3)
    else:
        oct_marks = ""
    beats = _safe_beats(n.dur_beats)
    return _dur_token_for(n.degree, oct_marks, beats), beats


def _build_jianpu_text(sheet: ScoreSheet) -> str:
    """把 ScoreSheet 构造为 jianpu-ly 输入文本。

    关键约束：jianpu-ly 要求每个小节拍数必须精确等于拍号，
    否则报错 "Incomplete bar" 且不产出 .ly。因此这里对每个小节做：
      1) 逐音符按"安全化拍数"截断到本小节剩余拍（处理跨小节长音）；
      2) 末音/休止精确补满到 beats_per_bar（拍数均为可靠集合元素）。
    """
    ts = sheet.time_sig
    tonic = sheet.key_tonic or "C"
    key_line = f"1={tonic}"

    bpm_int = int(round(sheet.bpm)) if sheet.bpm else 0
    tempo_line = f"4={bpm_int}" if bpm_int else ""

    beats_per_bar = sheet.beats_per_bar

    # 按小节分组，组内按起始拍排序
    bars: List[List[RenderNote]] = []
    if sheet.notes:
        max_bar = max(n.bar_idx for n in sheet.notes)
        bars = [[] for _ in range(max_bar + 1)]
        for n in sheet.notes:
            bars[n.bar_idx].append(n)
    for b in bars:
        b.sort(key=lambda x: x.start_beat)

    note_parts: List[str] = []
    for b in bars:
        if not b:
            note_parts.append(" ".join(["0"] * beats_per_bar))
            continue

        tokens: List[str] = []
        used = 0.0
        for n in b:
            remain = beats_per_bar - used
            if remain <= 1e-6:
                break
            tok, eff = _note_token(n)
            # 截断到本小节剩余拍（跨小节长音切分）
            if eff > remain + 1e-6:
                eff = remain
                tok = _dur_token_for(n.degree, _oct_marks(n), eff)
            if tok:
                tokens.append(tok)
                used += eff
        # 补满本小节（用最后音符的唱名占位）
        missing = round(beats_per_bar - used, 4)
        if missing >= 0.2:
            last = b[-1]
            fill = _dur_token_for(last.degree, _oct_marks(last), missing)
            if fill:
                tokens.append(fill)
        note_parts.append(" ".join(tokens))

    body = " | ".join(note_parts)

    # 歌词行
    lyrics = " ".join(n.lyric for n in sheet.notes if n.lyric)
    lyric_line = f"L: {lyrics}" if lyrics else ""

    lines = [f"{ts[0]}/{ts[1]}", key_line]
    if tempo_line:
        lines.append(tempo_line)
    lines.append(body)
    if lyric_line:
        lines.append(lyric_line)
    return "\n".join(lines) + "\n"


def render_score_sheet(
    sheet: ScoreSheet,
    output_path: str,
    dpi: int = 150,
) -> str:
    """用 jianpu-ly + LilyPond 渲染简谱图片。

    与 score_sheet.draw_score_sheet 同语义（接受 ScoreSheet，写出 output_path）。
    失败时抛出 RuntimeError，由调用方降级到 matplotlib。
    """
    lilypond = find_lilypond()
    if not lilypond or not os.path.exists(JIANPU_LY):
        raise RuntimeError(
            "缺少简谱渲染依赖：LilyPond 未安装或 lib/jianpu-ly.py 缺失。"
            " 请先 `winget install LilyPond.LilyPond`。"
        )

    ext = os.path.splitext(output_path)[1].lower().lstrip(".")
    if ext not in ("png", "pdf", "svg"):
        ext = "png"

    text = _build_jianpu_text(sheet)

    tmp = tempfile.mkdtemp(prefix="jianpu_")
    try:
        txt_path = os.path.join(tmp, "score.txt")
        ly_path = os.path.join(tmp, "score.ly")
        with open(txt_path, "w", encoding="utf-8") as f:
            f.write(text)

        # 1) jianpu-ly: 文本 -> LilyPond 源码
        #    说明：jianpu-ly 遇到"不完整小节"等会向 stderr 打印警告并以
        #    非 0 退出，但依然能产出可用的 .ly。这里只以"是否产出有效 .ly"
        #    作为成败判据，容忍非致命警告（如 Incomplete bar）。
        with open(ly_path, "w", encoding="utf-8") as fout, \
             open(txt_path, "r", encoding="utf-8") as fin:
            rc = subprocess.run(
                ["python", JIANPU_LY, "--noStaff", txt_path],
                stdin=fin, stdout=fout,
                stderr=subprocess.PIPE, text=True,
            )
        if not os.path.exists(ly_path) or os.path.getsize(ly_path) < 200:
            raise RuntimeError(f"jianpu-ly 转换失败：{rc.stderr[:500]}")

        # 2) LilyPond: 源码 -> 图片
        out_base = os.path.join(tmp, "out")
        if ext == "png":
            cmd = [lilypond, "-dbackend=eps", "--png",
                   f"-dresolution={dpi}", "-o", out_base, ly_path]
        else:
            cmd = [lilypond, f"-dbackend={ext}", "-o", out_base, ly_path]
        rc = subprocess.run(cmd, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, text=True)
        produced = f"{out_base}.{ext}"
        # eps 后端会额外产出 .eps；png 时实际文件名是 out.png
        if not os.path.exists(produced):
            # eps 后端 png 文件名可能带后缀，回退查找
            for f in os.listdir(tmp):
                if f.startswith("out.") and f.endswith(ext):
                    produced = os.path.join(tmp, f)
                    break
        if rc.returncode != 0 or not os.path.exists(produced):
            raise RuntimeError(f"LilyPond 渲染失败：{rc.stderr[:500]}")

        os.makedirs(os.path.dirname(os.path.abspath(output_path)) or ".",
                    exist_ok=True)
        shutil.copyfile(produced, output_path)
        return output_path
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
