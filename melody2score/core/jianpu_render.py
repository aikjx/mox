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
try:
    from .paths import resource_path
except Exception:  # 兜底：源码直接运行场景
    def resource_path(*parts: str) -> str:
        return os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))), *parts
        )

# jianpu-ly 脚本（随仓库附带于 lib/，打包后位于 _internal/lib/）。
# 用 resource_path 兼容「源码运行」与「PyInstaller 打包」两种模式。
JIANPU_LY = resource_path("lib", "jianpu-ly.py")


def find_lilypond() -> Optional[str]:
    """定位 lilypond 可执行文件；找不到返回 None。"""
    candidates: List[str] = []

    # 1) PATH
    p = shutil.which("lilypond")
    if p:
        candidates.append(p)

    # 2) winget 默认安装目录（用户级，版本号不写死，动态查找）
    base = os.path.expanduser(r"~\AppData\Local\Microsoft\WinGet\Packages")
    if os.path.isdir(base):
        for d in os.listdir(base):
            if "lilypond" not in d.lower():
                continue
            for root, _dirs, _files in os.walk(os.path.join(base, d)):
                cand = os.path.join(root, "lilypond.exe")
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


# jianpu-ly 规范时值记号（见 lib/jianpu-ly.py 文档：
#   全音符=1 - - -, 二分=1 -, 四分=1, 附点四分=1., 附点二分=1 - -
#   八分=q1, 附点八分=q1., 十六分=s1, 三十二分=d1 等）
# 直接采用规范记号，不做任何时值降格（附点八分/十六分本就被原生支持）。
_RELIABLE_BEATS = {4.0, 3.0, 2.0, 1.5, 1.0, 0.75, 0.5, 0.25, 0.125}


def _safe_beats(raw: float) -> float:
    """把原始拍数量化到 jianpu-ly 规范时值集，原样返回（不降格）。"""
    from .score_sheet import _quantize_duration
    _qdur, label = _quantize_duration(raw)
    beat_map = {
        "whole": 4.0, "dotted-half": 3.0, "half": 2.0,
        "dotted-quarter": 1.5, "quarter": 1.0,
        "dotted-eighth": 0.75, "eighth": 0.5, "sixteenth": 0.25,
        "thirty-second": 0.125,
    }
    b = beat_map.get(label, 1.0)
    return b


def _dur_token_for(degree: int, oct_marks: str, beats: float) -> str:
    """生成规范时值集内拍数对应的 jianpu-ly 记号。

    beats<=0 返回空串。degree 为 0 时表示休止符（用 '0' 系列）。
    采用 jianpu-ly 原生语法：全=1 - - - / 二分=1 - / 四分=1 /
    附点四分=1. / 附点二分=1 - - / 八分=q1 / 附点八分=q1. /
    十六分=s1 / 三十二分=d1，保证与拍号精确对齐。
    """
    if beats <= 0:
        return ""
    deg = "0" if degree == 0 else str(degree)
    if abs(beats - 4.0) < 1e-6:
        return f"{deg}{oct_marks} - - -"
    if abs(beats - 3.0) < 1e-6:
        return f"{deg}{oct_marks} - -"          # 附点二分
    if abs(beats - 2.0) < 1e-6:
        return f"{deg}{oct_marks} -"
    if abs(beats - 1.5) < 1e-6:
        return f"{deg}{oct_marks}."             # 附点四分
    if abs(beats - 1.0) < 1e-6:
        return f"{deg}{oct_marks}"
    if abs(beats - 0.75) < 1e-6:
        return f"q{deg}{oct_marks}."            # 附点八分
    if abs(beats - 0.5) < 1e-6:
        return f"q{deg}{oct_marks}"             # 八分
    if abs(beats - 0.25) < 1e-6:
        return f"s{deg}{oct_marks}"             # 十六分
    if abs(beats - 0.125) < 1e-6:
        return f"d{deg}{oct_marks}"             # 三十二分
    # 其它（跨小节截断出的非标准拍）按比例用 '-' 延长到最接近的四分单位
    # 仅用于补拍兜底，保持小节拍数完整（不允许自绘）。
    if beats > 1.0:
        return f"{deg}{oct_marks}" + " -" * int(round(beats - 1.0))
    return f"q{deg}{oct_marks}"


def _note_token(n: RenderNote) -> Tuple[str, float]:
    """把音符转成 (jianpu-ly 记号, 规范量化后拍数)。"""
    if n.octave_dot > 0:
        oct_marks = "'" * min(n.octave_dot, 3)
    elif n.octave_dot < 0:
        oct_marks = "," * min(-n.octave_dot, 3)
    else:
        oct_marks = ""
    beats = _safe_beats(n.dur_beats)
    return _dur_token_for(n.degree, oct_marks, beats), beats


def _build_jianpu_text(sheet: ScoreSheet) -> str:
    """把 ScoreSheet 构造为 jianpu-ly 输入文本（100% 规范简谱记法）。

    规范要点（区别于旧「切两截+补休止符」的退化写法）：
      1) 基于绝对时间轴精确布局：每音符用 {bar_idx*beats_per_bar + 段内相对拍}
         还原真实起止，小节内有休止间隔时显式写休止符（而非吞掉/错位）；
      2) 跨小节长音用**延音线 tie**（`1 ~ 1`）连接两段，不再补虚假休止符；
      3) 首小节若不足拍数按**弱起 anacrusis** 处理（拍号写 `4/4,8` 形），
         规范且与之匹配的最后小节自动少一拍；
      4) 调号区分大/小调：大调 `1=C`，小调 `6=C`（简谱首调唱名规范）；
      5) 曲尾加**终止线** `\bar "|."`；
      6) 逐音符带歌词（`1你 2好`），字位严格对齐。
    """
    ts = sheet.time_sig
    tonic = sheet.key_tonic or "C"
    mode = (sheet.key_mode or "major").lower()
    # 简谱规范：大调用主音 1 唱名，小调用主音 6 唱名（首调记谱）。
    key_line = f"6={tonic}" if mode == "minor" else f"1={tonic}"

    bpm_int = int(round(sheet.bpm)) if sheet.bpm else 0
    tempo_line = f"4={bpm_int}" if bpm_int else ""

    beats_per_bar = sheet.beats_per_bar
    notes = list(sheet.notes)
    if not notes:
        # 空旋律：放一个全休止符小节，仍产出合法谱表
        lines = [f"{ts[0]}/{ts[1]}", key_line]
        if tempo_line:
            lines.append(tempo_line)
        lines.append("0 - - -")
        lines.append(r'\bar "|."')
        return "\n".join(lines) + "\n"

    # 把每音符按绝对拍切成"落在各小节内的片段"，同音跨小节片段间用 tie 连接。
    # 段： (bar_idx, start_in_bar, dur, degree, oct_marks, lyric, is_tie_start, is_tie_end)
    segments: List[dict] = []
    for n in notes:
        abs_start = float(n.start_beat)
        abs_end = abs_start + max(0.05, float(n.dur_beats))
        deg = n.degree
        oct_marks = _oct_marks(n)
        lyric = n.lyric or ""
        cur = abs_start
        first_seg = True
        while cur < abs_end - 1e-6:
            b_idx = int(cur // beats_per_bar)
            bar_start = b_idx * beats_per_bar
            seg_end_in_bar = min(abs_end, bar_start + beats_per_bar)
            seg_dur = seg_end_in_bar - cur
            if seg_dur <= 1e-6:
                break
            in_bar = round(cur - bar_start, 6)
            last_seg = abs(seg_end_in_bar - abs_end) < 1e-6
            segments.append({
                "bar": b_idx, "in_bar": in_bar, "dur": seg_dur,
                "deg": deg, "oct": oct_marks, "lyric": lyric if first_seg else "",
                "tie_prev": not first_seg, "tie_next": not last_seg,
            })
            cur = seg_end_in_bar
            first_seg = False
    segments.sort(key=lambda s: (s["bar"], s["in_bar"]))

    max_bar = max(s["bar"] for s in segments)
    # 弱起（anacrusis）：仅当整曲第一音不在强拍（首小节起始拍 > 0）才算弱起，
    # 否则短曲首小节"未满拍"只是曲子短，不应标弱起（避免误判）。jianpu-ly 弱起
    # 语法 "4/4,N" 中 N = 首小节剩余拍数 = beats_per_bar - 第一音起始拍。
    first_occ = [s for s in segments if s["bar"] == 0]
    first_start = min((s["in_bar"] for s in first_occ), default=0.0)
    is_anacrusis = first_start > 1e-6
    anacrusis_span = round(beats_per_bar - first_start, 4) if is_anacrusis else beats_per_bar

    # 构建各小节 token 串
    note_parts: List[str] = []
    for b in range(max_bar + 1):
        segs = [s for s in segments if s["bar"] == b]
        if not segs:
            note_parts.append(" ".join(["0"] * beats_per_bar))
            continue
        tokens: List[str] = []
        used = 0.0
        for s in segs:
            # 段前休止间隔（小节内留白）
            gap = round(s["in_bar"] - used, 6)
            if gap > 1e-6:
                for rt in _rest_fill_tokens(gap):
                    tokens.append(rt)
                used += gap
            elif gap < -1e-6:
                # 重叠：夹断保护，不写负休止
                used = s["in_bar"]
            beats = _safe_beats(s["dur"])
            tok = _dur_token_for(s["deg"], s["oct"], beats)
            # 逐音符歌词：规范写法 "1你"（字紧接数字后）
            if s["lyric"]:
                tok = f"{tok}{s['lyric']}"
            # 延音线：段首接上一小节同音 → 前置 '~'；段末续下一小节 → 后置 '~'
            if s["tie_prev"]:
                tok = f"~ {tok}"
            if s["tie_next"]:
                tok = f"{tok} ~"
            if tok.strip():
                tokens.append(tok)
            used += beats
        # 末小节前的完整小节：仅当不是弱起的"补偿小节"才补休止
        # 但 jianpu-ly 要求每小节满拍；弱起时最后一小节会自动少拍，此处末小节不补。
        if b < max_bar and not (is_anacrusis and b == 0):
            missing = round(beats_per_bar - used, 4)
            if missing > 1e-6:
                for rt in _rest_fill_tokens(missing):
                    tokens.append(rt)
        note_parts.append(" ".join(tokens))

    # 拍号行：弱起用 "num/den,首小节剩余拍数" 语法
    if is_anacrusis:
        ts_line = f"{ts[0]}/{ts[1]},{anacrusis_span}"
    else:
        ts_line = f"{ts[0]}/{ts[1]}"

    body = " | ".join(note_parts)
    # 终止线：规范乐曲结尾用双细线 "|."，而非普通单竖线
    body = body + " " + r'\bar "|."'

    # 歌词仍保留 L: 整行作为后备（逐音符已带字则优先；二者并存 jianpu-ly 以逐音符为准）
    all_lyrics = " ".join(n.lyric for n in sheet.notes if n.lyric)
    lyric_line = f"L: {all_lyrics}" if all_lyrics else ""

    lines = [ts_line, key_line]
    if tempo_line:
        lines.append(tempo_line)
    lines.append(body)
    if lyric_line:
        lines.append(lyric_line)
    return "\n".join(lines) + "\n"


def _rest_fill_tokens(beats: float) -> List[str]:
    """把剩余拍数精确拆分为 jianpu-ly 休止符记号序列（贪心从大到小）。

    返回如 ['0 -', 'q0'] 表示 2.5 拍休止；保证记号拍数之和 ≈ beats。
    """
    standards = [4.0, 3.0, 2.0, 1.5, 1.0, 0.75, 0.5, 0.25]
    rest = round(beats, 4)
    out: List[str] = []
    for s in standards:
        while rest >= s - 1e-6:
            out.append(_dur_token_for(0, "", s))
            rest = round(rest - s, 4)
    return out


def render_score_sheet(
    sheet: ScoreSheet,
    output_path: str,
    dpi: int = 150,
) -> str:
    """用 jianpu-ly + LilyPond 渲染规范简谱图片（标准第三方工具链，不自写渲染器）。

    接受 ScoreSheet，写出 output_path（支持 png/pdf/svg）。
    缺少 LilyPond 或 lib/jianpu-ly.py 时抛出 RuntimeError 提示安装。
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
                stderr=subprocess.PIPE, encoding="utf-8", errors="replace",
            )
        if not os.path.exists(ly_path) or os.path.getsize(ly_path) < 200:
            raise RuntimeError(f"jianpu-ly 转换失败：{rc.stderr[:500]}")

        # 2) LilyPond: 源码 -> 图片
        #    说明：LilyPond 2.24 在 Windows 上的 `-dbackend=pdf` 因缺少 Ghostscript
        #    绑定会报 `Unbound variable: output-stencils`。规范且可靠的做法是
        #    统一走 `-dbackend=eps` 后端：它本就会同时产出 .pdf（矢量）与 .eps，
        #    png 额外加 --png，svg 单独走 -dbackend=svg。
        out_base = os.path.join(tmp, "out")
        if ext == "png":
            cmd = [lilypond, "-dbackend=eps", "--png",
                   f"-dresolution={dpi}", "-o", out_base, ly_path]
        elif ext == "svg":
            cmd = [lilypond, "-dbackend=svg", "-o", out_base, ly_path]
        else:  # pdf / 其它 -> eps 后端会产出 .pdf
            cmd = [lilypond, "-dbackend=eps", "-o", out_base, ly_path]
            ext = "pdf"
        rc = subprocess.run(cmd, stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE,
                            encoding="utf-8", errors="replace")
        produced = f"{out_base}.{ext}"
        if not os.path.exists(produced):
            # eps 后端产出的文件名回退查找（兼容不同后缀）
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
