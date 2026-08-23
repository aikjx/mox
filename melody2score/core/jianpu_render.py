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
import sys
import tempfile
import threading
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


def _decompose_units(units: int) -> List[int]:
    """把网格单位数贪心拆分为规范时值单位序列（延音线相连，总量分毫不差）。

    单位=0.25 拍（十六分）。16=全音符, 12=附点二分, 8=二分, 6=附点四分,
    4=四分, 3=附点八分, 2=八分, 1=十六分。任意正整数均可精确拆分。
    """
    out: List[int] = []
    u = int(units)
    for s in (16, 12, 8, 6, 4, 3, 2, 1):
        while u >= s:
            out.append(s)
            u -= s
    return out


def _build_jianpu_text(sheet: ScoreSheet) -> str:
    """把 ScoreSheet 构造为 jianpu-ly 输入文本（100% 规范简谱记法）。

    规范要点（区别于旧「切两截+补休止符」的退化写法）：
      1) 统一 0.25 拍网格布局：全部音符起止/间隙先量化到十六分网格（整数
         单位）再排版——单一时间轴。旧实现「原始时间轴切分 + 量化拍数记账」
         双轨并行，两者舍入不一致会使小节记号拍数系统性偏少（实测每小节
         3.5~3.75 拍），累积漂移触发 LilyPond barcheck fail（音符跨小节线）；
      2) 除末小节外每个小节的记号拍数之和恒等于每小节拍数（按构造保证）：
         跨小节长音与非规范总长（如 1.75 拍）均拆为规范时值组合，
         段间以**延音线 tie**（`1 ~ 1`）连接，不补虚假休止符；
      3) 音符间真实间隔以休止符填充（不足十六分的微间隙吸收），
         首小节弱起以休止符前缀填充（简谱弱起规范记法之一）；
      4) 调号区分大/小调：大调 `1=C`，小调 `6=C`（简谱首调唱名规范）；
      5) 曲尾加**终止线** `\\bar "|."`；
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

    # ---- 统一 0.25 拍网格布局（整数单位，单一时间轴） ----
    # 旧实现「原始时间轴切分 + 量化拍数记账」双轨并行：段切分/间隙用原始拍，
    # token 生成/used 累计用量化拍，两者舍入不一致使小节记号拍数系统性偏少
    # （实测 3.5~3.75 拍/小节），累积漂移触发 LilyPond barcheck fail。
    # 现全部量化到十六分网格后再布局：小节拍数按构造精确满额（末小节除外）。
    bar_units = max(1, int(round(beats_per_bar * 4)))   # 每小节网格单位数
    grid_notes = sorted(
        notes, key=lambda n: (float(n.start_beat), float(n.dur_beats)))

    # 事件流：note / rest，(start_u, len_u) 均为整数网格单位
    events: List[tuple] = []          # (kind, start_u, len_u, note|None)
    cursor: Optional[int] = None      # 上一事件结束位置
    for n in grid_notes:
        start_u = int(round(float(n.start_beat) * 4))
        len_u = max(1, int(round(float(n.dur_beats) * 4)))
        if cursor is None:
            if start_u > 0:           # 弱起：首音前以休止符填充（规范记法）
                events.append(("rest", 0, start_u, None))
        elif start_u > cursor:        # 音符间真实间隙 → 休止符事件
            events.append(("rest", cursor, start_u - cursor, None))
        # 重叠保护：start_u < cursor 时紧接上一事件（不产生负休止）
        eff = max(start_u, cursor) if cursor is not None else start_u
        events.append(("note", eff, len_u, n))
        cursor = eff + len_u

    max_bar = (cursor - 1) // bar_units

    # ---- 按小节生成 token（小节拍数按构造精确满额，末小节除外） ----
    note_parts: List[str] = []
    for b in range(max_bar + 1):
        bar_lo, bar_hi = b * bar_units, (b + 1) * bar_units
        tokens: List[str] = []
        for kind, s, l, n in events:
            if s >= bar_hi or s + l <= bar_lo:
                continue
            seg_lo, seg_hi = max(s, bar_lo), min(s + l, bar_hi)
            seg = seg_hi - seg_lo
            if seg <= 0:
                continue
            if kind == "rest":
                for rt in _rest_fill_tokens(seg / 4.0):
                    tokens.append(rt)
                continue
            # 音符片段：跨小节 / 非规范总长 → 规范时值 tie 链
            first_piece = (seg_lo == s)
            last_piece = (seg_hi == s + l)
            decomp = _decompose_units(seg)
            for i, u in enumerate(decomp):
                tok = _dur_token_for(n.degree, _oct_marks(n), u / 4.0)
                if i > 0 or not first_piece:
                    tok = f"~ {tok}"       # 接上一段同音（跨小节或小节内）
                if i == len(decomp) - 1 and not last_piece:
                    tok = f"{tok} ~"       # 续下一段同音
                if first_piece and i == 0 and n.lyric:
                    tok = f"{tok}{n.lyric}"
                if tok.strip():
                    tokens.append(tok)
        note_parts.append(" ".join(t for t in tokens if t.strip()))

    # 拍号行：始终用规范标准 "num/den"。
    # 弱起（首音不在强拍）不依赖 jianpu-ly 的 anacrusis 逗号语法（该语法要求弱起拍
    # 必须严格为 1/denom 整数倍，真实音乐里常不满足而易崩溃）；改用更通用且规范的
    # 写法：首小节弱起音之前以休止符填充（简谱弱起的标准记法之一），配合
    # j2ly_sloppy_bars=1 放宽末小节补足约束。
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


# 进程内执行 jianpu-ly 的串行锁：sys.argv / sys.stdout 是进程全局状态，
# 多线程并发渲染（GUI SheetWorker 与 API 导出同时触发）必须串行化。
_JIANPU_LOCK = threading.Lock()


def _run_jianpu_ly(txt_path: str, ly_path: str) -> str:
    """进程内执行 jianpu-ly.py（txt → LilyPond 源码写入 ly_path）。

    为什么不再用 subprocess ["python", script]：
      1) PyInstaller frozen（绿色发行版）目标电脑**无需安装 Python**——
         PATH 上没有 python 命令，旧调用在发行版里必然 FileNotFoundError，
         简谱图片导出全挂（README 却承诺开箱即用）；
      2) 源码环境下裸 "python" 可能解析到 Microsoft Store 的 stub 或
         版本不一致的解释器。
    进程内 runpy 执行让源码/打包两种模式统一，还省一次解释器冷启动。

    jianpu-ly 输出契约（见其 write_output）：stdout 非 tty 时把 LilyPond
    源码写到 stdout —— 故把 sys.stdout 临时重定向为 ly_path 文件；
    --noStaff 等选项经 sys.argv 传入。警告路径（Incomplete bar 等）
    会 sys.exit 非 0，与旧 subprocess 行为一致：以"是否产出有效 .ly"
    为成败判据（调用方检查文件大小）。

    stderr 兜底（关键，打包后歌谱生成失败的真根因）：
      PyInstaller windowed（console=False）发行版里 sys.stderr 是 None。
      jianpu-ly 有 30+ 处 sys.stderr.write（j2ly_sloppy_bars 末小节警告、
      老版本 tie 警告、errExit 错误路径等），任一触发即
      AttributeError: 'NoneType' object has no attribute 'write'
      → 简谱渲染必败。双击运行 exe（无控制台）100% 复现；从控制台
      启动 exe 时 stderr 句柄有效，故控制台自检发现不了。
      现统一把 stderr 重定向为内存捕获缓冲：不炸、警告文本可回传
      诊断（企业级可观测性）。

    返回：捕获到的 stderr 警告全文（无警告为空串）。
    """
    import io
    import runpy
    import sys as _sys
    captured = io.StringIO()
    with _JIANPU_LOCK:
        argv_backup = _sys.argv
        stdout_backup = _sys.stdout
        stderr_backup = _sys.stderr
        try:
            _sys.argv = [JIANPU_LY, "--noStaff", txt_path]
            with open(ly_path, "w", encoding="utf-8") as fout:
                _sys.stdout = fout
                _sys.stderr = captured
                try:
                    runpy.run_path(JIANPU_LY, run_name="__main__")
                except SystemExit:
                    pass
        finally:
            _sys.argv = argv_backup
            _sys.stdout = stdout_backup
            _sys.stderr = stderr_backup
    return captured.getvalue()


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

        # 1) jianpu-ly: 文本 -> LilyPond 源码（进程内执行，兼容打包发行版）
        #    说明：jianpu-ly 遇到"不完整小节"等会向 stderr 打印警告并以
        #    非 0 退出，但依然能产出可用的 .ly。这里只以"是否产出有效 .ly"
        #    作为成败判据，容忍非致命警告（如 Incomplete bar）。
        #    设置 j2ly_sloppy_bars=1 放宽"末小节必须补足弱起拍"的强制，
        #    使弱起/短曲也能正常出图（对普通谱无副作用）。
        os.environ.setdefault("j2ly_sloppy_bars", "1")
        jianpu_warnings = _run_jianpu_ly(txt_path, ly_path)
        if not os.path.exists(ly_path) or os.path.getsize(ly_path) < 200:
            raise RuntimeError(
                "jianpu-ly 转换失败：未产出有效 LilyPond 源码"
                "（可能是不完整小节等输入问题）"
                + (f"；jianpu-ly 警告：{jianpu_warnings[:300]}" if jianpu_warnings else ""))

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
        # 防御 1：windowed 发行版（console=False）进程的 stdin 是空句柄，
        #         某些外部工具（LilyPond/Guile 启动期探测 tty）可能因此异常——
        #         显式接 DEVNULL 隔离。
        # 防御 2：子进程环境剥离 PyInstaller bootloader 注入到 PATH 头部的
        #         发行版目录（_internal / dist 根），避免外部工具的 DLL/
        #         辅助程序搜索撞上发行版自带的同名文件。
        sub_env = os.environ.copy()
        dist_dirs = {os.path.normcase(os.path.abspath(d)) for d in (
            os.path.dirname(sys.executable),
            os.path.join(os.path.dirname(sys.executable), "_internal"),
        )}
        cleaned = [p for p in sub_env.get("PATH", "").split(os.pathsep)
                   if p and os.path.normcase(os.path.abspath(p)) not in dist_dirs]
        sub_env["PATH"] = os.pathsep.join(cleaned)
        try:
            rc = subprocess.run(cmd, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE,
                                encoding="utf-8", errors="replace",
                                env=sub_env)
        except FileNotFoundError:
            raise RuntimeError(f"LilyPond 可执行文件不存在：{lilypond}")
        produced = f"{out_base}.{ext}"
        if not os.path.exists(produced):
            # eps 后端产出的文件名回退查找（兼容不同后缀）
            for f in os.listdir(tmp):
                if f.startswith("out.") and f.endswith(ext):
                    produced = os.path.join(tmp, f)
                    break
        if rc.returncode != 0 or not os.path.exists(produced):
            # 全量诊断信息（企业级可观测性：rc/stdout/stderr/产物清单/预处理警告）
            raise RuntimeError(
                "LilyPond 渲染失败 rc=%s：stdout=%r stderr=%r files=%s%s"
                % (rc.returncode, (rc.stdout or "")[:200],
                   (rc.stderr or "")[:300], os.listdir(tmp),
                   ("；jianpu-ly 警告：" + jianpu_warnings[:200])
                   if jianpu_warnings else ""))

        os.makedirs(os.path.dirname(os.path.abspath(output_path)) or ".",
                    exist_ok=True)
        shutil.copyfile(produced, output_path)
        return output_path
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
