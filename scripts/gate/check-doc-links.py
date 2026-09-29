#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""文档链接校验器 — docs/ 结构门禁（DOC-GOV-ARC-V1.0 §7）

作用
    扫描文档中的 Markdown 链接、HTML href/src，以及反引号包裹的 `docs/...` 路径引用，
    校验引用目标在仓库中是否真实存在；用于防止目录迁移、重命名后留下死链。

用法
    python scripts/gate/check-doc-links.py                  # 校验 docs/（默认跳过 _archive）
    python scripts/gate/check-doc-links.py --all            # 含 _archive
    python scripts/gate/check-doc-links.py --repo           # 一并校验仓库其它 .md/.html 中对 docs/ 的引用
    python scripts/gate/check-doc-links.py --strict         # 反引号路径引用也视为错误（默认仅告警）
    python scripts/gate/check-doc-links.py --json out.json  # 输出机器可读报告
    python scripts/gate/check-doc-links.py --selftest        # 仪器自检（锚点解析/越界判定/截断告示）

约定
    带行号的引用（形如 docs/a.md:24）按「目标文件存在 + 行号落在该文件行数内」判定，
    不再把整串当文件名——旧写法会把 5 条这类有效引用误判成「引用不存在」；
    明细打印受 --max-detail 限制（默认 120），总体超过上限时另印一行「已截断」告示：
    对两份都被截断的打印集做集合差，会凭空造出一条「消失」的条目，比较总体须先调大上限或读总体行。
    围栏代码块（``` / ~~~）内的链接视为示例，不参与校验；
    JS/Vue 模板插值（${...}）、file:// 绝对路径、glob/占位写法不计为断链；
    历史快照或「旧路径 → 新路径」迁移映射表，可用 HTML 注释显式豁免：
        <!-- check-doc-links:ignore -->              仅豁免本行
        <!-- check-doc-links:ignore-start --> ... <!-- check-doc-links:ignore-end -->  豁免区块
    豁免只影响本行/本区块，正文超链接仍会被校验。

退出码
    0 = 无断链；1 = 存在断链（可直接作为 CI 门禁）
"""

import argparse
import contextlib
import io
import json
import os
import re
import sys
import tempfile
import urllib.parse
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
DOCS_ROOT = REPO_ROOT / "docs"

SKIP_DIRS = {
    "node_modules", "target", ".git", "dist", ".codebuddy", "ais", "third_party",
    ".trae", "generated-images",
}
SKIP_DIR_NAMES_IN_DOCS = {"_archive"}  # 除非 --all

MD_LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
HTML_LINK = re.compile(r"""(?:href|src)\s*=\s*["']([^"']+)["']""", re.IGNORECASE)
CODE_REF = re.compile(r"`(docs/[^`\s]+)`")

# 带行号的引用（docs/a.md:24）：路径部分照常校验，行号另判「越界」，两码事分开报。
# 扩展名不列白名单（任何 1–6 位的点分扩展都算），否则 .mjs/.toml 这一族会退回成假阳。
ANCHOR_REF = re.compile(r"^(.+?\.[A-Za-z0-9]{1,6}):(\d{1,6})$")
# 越界判定的标签常量：判据句与自检共用同一个名，改措辞必须走这里，两边不会各自腐烂
ANCHOR_TAG = "行号越界"

# 围栏代码块（``` / ~~~）：其中的链接是示例或片段，不参与校验
FENCE = re.compile(r"^[ \t]*(`{3,}|~{3,})[^\n]*\n.*?^[ \t]*\1[ \t]*$", re.S | re.M)

EXTERNAL_PREFIXES = ("http://", "https://", "mailto:", "tel:", "data:", "javascript:", "ftp://", "//")

# 历史快照/迁移映射表：显式豁免标记（HTML 注释，渲染时不显示）
IGNORE_LINE = re.compile(r"<!--\s*check-doc-links:ignore\s*-->")
IGNORE_START = re.compile(r"<!--\s*check-doc-links:ignore-start\s*-->")
IGNORE_END = re.compile(r"<!--\s*check-doc-links:ignore-end\s*-->")


def ignored_lines(text: str):
    """返回被显式豁免的行号集合（行内标记或 ignore-start/end 区块）。"""
    ignored, depth = set(), 0
    for lineno, line in enumerate(text.splitlines(), 1):
        if IGNORE_START.search(line):
            depth += 1
            ignored.add(lineno)
            continue
        if IGNORE_END.search(line):
            depth = max(0, depth - 1)
            ignored.add(lineno)
            continue
        if depth or IGNORE_LINE.search(line):
            ignored.add(lineno)
    return ignored


def strip_fences(text: str) -> str:
    """把围栏代码块整体替换为等量换行，保留行号，避免把示例链接判为断链。"""
    return FENCE.sub(lambda m: "\n" * m.group(0).count("\n"), text)


def should_scan(path: Path, include_archive: bool) -> bool:
    parts = set(path.parts)
    if parts & SKIP_DIRS:
        return False
    if not include_archive and "_archive" in parts:
        return False
    return True


def walk_docs(root: Path, include_archive: bool, skip_docs: bool):
    """带剪枝的遍历（os.walk），避免深入 node_modules / target 等巨型目录。"""
    for dirpath, dirnames, filenames in os.walk(str(root), onerror=lambda _e: None):
        dirnames[:] = [
            d for d in dirnames
            if d not in SKIP_DIRS and (include_archive or d not in SKIP_DIR_NAMES_IN_DOCS)
        ]
        base = Path(dirpath)
        if skip_docs:
            try:
                base.relative_to(DOCS_ROOT)
                continue  # docs 内文件由 docs 扫描负责
            except ValueError:
                pass
        for fn in filenames:
            if fn.lower().endswith((".md", ".html")):
                yield base / fn


def collect_files(include_archive: bool, include_repo: bool):
    files = list(walk_docs(DOCS_ROOT, include_archive, skip_docs=False))
    if include_repo:
        files += list(walk_docs(REPO_ROOT, include_archive, skip_docs=True))
    return sorted(set(files))


def extract_targets(text: str):
    """返回 [(lineno, target, kind)]，kind ∈ {link, html, code}"""
    out = []
    for rx, kind in ((MD_LINK, "link"), (HTML_LINK, "html"), (CODE_REF, "code")):
        for m in rx.finditer(text):
            raw = m.group(1).strip()
            if kind == "link":
                if raw.startswith("<"):
                    end = raw.find(">")
                    if end > 0:
                        raw = raw[1:end]
                elif '"' in raw:
                    raw = raw.split('"')[0].strip()
            if not raw:
                continue
            lineno = text.count("\n", 0, m.start()) + 1
            out.append((lineno, raw, kind))
    return out


def normalize_target(raw: str):
    """返回 (可解析路径 或 None, 原因)。None 表示外部/锚点/无法判定。"""
    t = raw.strip().replace("\\", "/")
    if not t or t.startswith("#"):
        return None, "anchor"
    low = t.lower()
    if low.startswith(EXTERNAL_PREFIXES):
        return None, "external"
    t = t.split("#", 1)[0].split("?", 1)[0]
    if not t:
        return None, "anchor"
    t = urllib.parse.unquote(t)
    if t.startswith("file:"):
        return None, "absfile"  # 机器绝对路径链接：不校验，单独统计
    if "${" in t or "{{" in t:
        return None, "placeholder"  # JS/Vue 模板插值（如 ${doc.path}）
    if any(ch in t for ch in '<>"|'):
        return None, "placeholder"  # 形如 docs/xxx/<file> 的占位写法
    if any(ch in t for ch in "*?~"):
        return None, "placeholder"  # glob / 区间 / 省略写法（docs/**、docs/00~17、docs/18-...md）
    if "..." in t:
        return None, "placeholder"
    if any(ch in t for ch in "{}"):
        return None, "placeholder"  # 模板占位（docs/modules/cards/node-{domain}.md）
    if re.search(r"YYYY|MMDD|HHmmss|xxx", t):
        return None, "placeholder"  # 日期/序号模板（test-report-YYYYMMDD-HHmmss/）
    if re.search(r"(^|[-/])(XX|YY|NN)([-/.]|$)", t):
        return None, "placeholder"  # 命名模板（docs/enterprise/cards/doc-XX-module.md）
    if re.fullmatch(r"[\w./-]*/\d{1,3}", t):
        return None, "placeholder"  # 形如 docs/enterprise/15 的简写
    if "." not in t and "/" not in t:
        return None, "placeholder"  # 裸标识符（多为脚本产物/模板占位）
    if re.match(r"^[A-Za-z]:/", t) or t.startswith("/"):
        return Path(t), "absolute"
    if t.startswith("docs/"):
        return (REPO_ROOT / t), "root-relative"
    return Path(t), "relative"  # 由调用方按所在目录拼接


def resolve(path: Path) -> Path:
    return Path(os.path.normpath(str(path)))


def split_anchor(raw: str):
    """`docs/a.md:24` -> ('docs/a.md', 24)；不带行号锚点 -> (原串, None)。"""
    m = ANCHOR_REF.match(raw.strip())
    return (m.group(1), int(m.group(2))) if m else (raw, None)


_LINE_COUNT = {}


def line_count(path: Path) -> int:
    """目标文件的行数；读不到（目录/权限/编码意外）返回 -1，表示「行数未知」而不是 0 行。"""
    key = str(path)
    if key not in _LINE_COUNT:
        try:
            _LINE_COUNT[key] = len(path.read_text(encoding="utf-8", errors="replace").splitlines())
        except OSError:
            _LINE_COUNT[key] = -1
    return _LINE_COUNT[key]


def truncation_notice(total, printed):
    """总体大于打印数时给出截断告示；否则空串（没截断就不许印，印了就是假警报）。"""
    if total <= printed:
        return ""
    return ("\n⚠ 明细已截断：打印 %d 条／总体 %d 条。对两份被截断的打印集做集合差会凭空造出「消失」的条目，"
            "要比较总体请调大 --max-detail 或直接读上面那行总数。" % (printed, total))


def anchor_reason(resolved: Path, anchor):
    """文件确实存在时，对行号锚点做附加判定：返回 None（放行）或越界说明。

    行数未知（-1）按放行处理——「读不到行数」不等于「行号无效」，不许据此判缺。
    """
    if anchor is None:
        return None
    lc = line_count(resolved)
    if lc < 0 or 1 <= anchor <= lc:
        return None
    return "%s（目标 %d 行，引用第 %d 行）" % (ANCHOR_TAG, lc, anchor)


def check(include_archive, include_repo, strict, report_path, max_detail):
    files = collect_files(include_archive, include_repo)
    broken, warnings, scanned_links = [], [], 0
    skipped = {}

    for f in files:
        try:
            text = f.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:  # pragma: no cover
            broken.append({"file": str(f), "line": 0, "target": str(exc), "kind": "io"})
            continue

        rel_file = f.relative_to(REPO_ROOT).as_posix()
        ig = ignored_lines(text)
        for lineno, raw, kind in extract_targets(strip_fences(text)):
            if lineno in ig:
                skipped["ignored"] = skipped.get("ignored", 0) + 1
                continue
            target_str, anchor = split_anchor(raw)
            resolved, mode = normalize_target(target_str)
            if resolved is None:
                if mode in ("absfile", "placeholder", "external"):
                    skipped[mode] = skipped.get(mode, 0) + 1
                continue
            if mode == "absolute":
                continue  # 绝对路径：不判定
            if mode == "relative":
                resolved = f.parent / resolved
            resolved = resolve(resolved)
            scanned_links += 1
            try:
                exists = resolved.exists()
            except OSError:
                exists = False
            reason = None
            if exists:
                reason = anchor_reason(resolved, anchor)
                if reason is None:
                    continue
            item = {"file": rel_file, "line": lineno, "target": raw, "kind": kind}
            if reason:
                item["reason"] = reason
            if kind == "code" and not strict:
                warnings.append(item)
            else:
                broken.append(item)

    print("[check-doc-links] 扫描文件 %d 个，判定引用 %d 条" % (len(files), scanned_links))
    if skipped:
        print("跳过：外部链接 %d 条，file:// 绝对路径链接 %d 条（技术债·应改为仓根相对），占位/非路径 %d 条，显式豁免 %d 行"
              % (skipped.get("external", 0), skipped.get("absfile", 0), skipped.get("placeholder", 0),
                 skipped.get("ignored", 0)))
    if broken:
        per_file = {}
        for it in broken:
            per_file[it["file"]] = per_file.get(it["file"], 0) + 1
        print("\n断链总数 %d，涉及 %d 个文件；TOP 25 文件：" % (len(broken), len(per_file)))
        for name, cnt in sorted(per_file.items(), key=lambda kv: -kv[1])[:25]:
            print("  [%3d] %s" % (cnt, name))
        print("\n断链明细（最多 %d 条，--max-detail 可调）：" % max_detail)
        for it in broken[:max_detail]:
            print("  [BROKEN] %s:%s -> %s  [%s]%s"
                  % (it["file"], it["line"], it["target"], it["kind"],
                     "  （%s）" % it["reason"] if it.get("reason") else ""))
        sys.stdout.write(truncation_notice(len(broken), min(max_detail, len(broken))))
    if warnings:
        print("\n告警·反引号路径引用不存在（%d，加 --strict 可视为错误）：" % len(warnings))
        for it in warnings[:max_detail]:
            print("  [WARN]   %s:%s -> %s%s"
                  % (it["file"], it["line"], it["target"],
                     "  （%s）" % it["reason"] if it.get("reason") else ""))
        sys.stdout.write(truncation_notice(len(warnings), min(max_detail, len(warnings))))

    if report_path:
        Path(report_path).write_text(
            json.dumps(
                {"scanned_files": len(files), "checked_refs": scanned_links,
                 "broken": broken, "warnings": warnings},
                ensure_ascii=False, indent=2,
            ),
            encoding="utf-8",
        )
        print("\n机器可读报告：%s" % report_path)

    if not broken:
        print("\n[OK] 未发现断链")
    return 1 if broken else 0


def selftest():
    """仪器自检：锚点解析、越界判定、行数未知豁免、截断告示。夹具在临时目录，不动仓库。"""
    state = {"pass": 0, "fail": 0}

    def chk(name, cond, detail=""):
        state["pass" if cond else "fail"] += 1
        print("  [%s] %s%s" % ("PASS" if cond else "FAIL", name, ("  ｜" + detail) if detail else ""))

    tmp = Path(tempfile.mkdtemp(prefix="check-doc-links-selftest-"))
    try:
        doc = tmp / "a.md"
        doc.write_text("one\ntwo\nthree\n", encoding="utf-8")

        # 一、锚点解析本身（这形状以前整串当文件名，于是 5 条有效引用被误判）
        chk("A1 带行号的引用拆成路径＋行号", split_anchor("docs/x/a.md:24") == ("docs/x/a.md", 24))
        chk("A2 非 .md 扩展也要认（扩展名不列白名单）", split_anchor("docs/x/a.mjs:7") == ("docs/x/a.mjs", 7))
        chk("A3 不带行号的不误拆", split_anchor("docs/x/a.md") == ("docs/x/a.md", None))
        chk("A4 目录简写不误拆成锚点", split_anchor("docs/enterprise/15") == ("docs/enterprise/15", None))

        # 二、放宽不等于豁免：行号越界必须照判
        chk("B1 行数现量", line_count(doc) == 3, "line_count=%d" % line_count(doc))
        chk("B2 行号在范围内放行", anchor_reason(doc, 3) is None, "reason=%r" % anchor_reason(doc, 3))
        chk("B3 行号越界必须判缺", anchor_reason(doc, 4) is not None, "reason=%r" % anchor_reason(doc, 4))
        chk("B4 行号 0 不是合法锚点", anchor_reason(doc, 0) is not None, "reason=%r" % anchor_reason(doc, 0))
        lc_dir = line_count(tmp)
        chk("B5 读不到行数＝未知（-1），不是 0 行", lc_dir == -1, "line_count(目录)=%d" % lc_dir)
        chk("B6 未知行数时不得凭「越界」判缺", anchor_reason(tmp, 999999) is None,
            "reason=%r" % anchor_reason(tmp, 999999))

        # 三、截断告示：期望由总体那两行现推，不硬编条数
        capped = _run_capture(1)
        full = _run_capture(10 ** 6)
        n_broken = _int_after(r"断链总数 ", full)
        n_warn = _int_after(r"路径引用不存在（", full)
        expect = (1 if n_broken > 1 else 0) + (1 if len(warn_lines(full)) > 1 else 0)
        chk("C1 上限小于总体时必须印截断告示", capped.count("已截断") == expect,
            "总体断链 %d／告警 %d ⇒ 期望告示 %d 条，实得 %d 条"
            % (n_broken, n_warn, expect, capped.count("已截断")))
        chk("C2 上限足够大时不得印截断告示", full.count("已截断") == 0)
        chk("C3 未截断时打印行数＝总体", len(warn_lines(full)) == n_warn,
            "打印 %d／总体 %d" % (len(warn_lines(full)), n_warn))
        anchored_self = re.compile(r":\d{1,6}$")  # 本格自带的针形正则：不借 split_anchor，否则变异体同时改掉尺子和被尺子审的那格
        blind = [l for l in warn_lines(full)
                 if " -> " in l
                 and anchored_self.search(l.split(" -> ", 1)[1].split("  （", 1)[0].strip())
                 and ANCHOR_TAG not in l]
        chk("C4 被判「引用不存在」的不许带行号锚点（锚点只能判越界）", not blind,
            "仍有 %d 条：%s" % (len(blind), [l.split(" -> ", 1)[1][:60] for l in blind[:3]]))
    finally:
        try:
            (tmp / "a.md").unlink()
            tmp.rmdir()
        except OSError:
            pass

    print("\nSELFTEST PASS=%d FAIL=%d" % (state["pass"], state["fail"]))
    return 1 if state["fail"] else 0


def warn_lines(text):
    return [l for l in text.split("\n") if "[WARN]" in l]


def _int_after(marker, text):
    m = re.search(marker + r"(\d+)", text)
    return int(m.group(1)) if m else -1


def _run_capture(max_detail):
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        check(False, False, False, None, max_detail)
    return buf.getvalue()


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    ap = argparse.ArgumentParser(description="docs/ 文档链接校验（DOC-GOV-ARC-V1.0 §7）")
    ap.add_argument("--all", action="store_true", help="包含 docs/_archive/ 内的文件")
    ap.add_argument("--repo", action="store_true", help="同时校验仓库其它 md/html 对 docs/ 的引用")
    ap.add_argument("--strict", action="store_true", help="反引号路径引用也视为错误")
    ap.add_argument("--json", dest="json_path", default=None, help="输出 JSON 报告路径")
    ap.add_argument("--max-detail", dest="max_detail", type=int, default=120,
                    help="明细最多打印条数（总体超过它会另印一行截断告示）")
    ap.add_argument("--selftest", action="store_true", help="自检锚点解析/越界判定/截断告示，不产出断链账")
    args = ap.parse_args()
    if args.selftest:
        sys.exit(selftest())
    sys.exit(check(args.all, args.repo, args.strict, args.json_path, args.max_detail))


if __name__ == "__main__":
    main()
