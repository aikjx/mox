#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""文档链接校验器 — docs/ 结构门禁（DOC-GOV-ARC-V1.0 §7）

作用
    扫描文档中的 Markdown 链接、HTML href/src，以及反引号包裹的 `docs/...` 路径引用，
    校验引用目标在仓库中是否真实存在；用于防止目录迁移、重命名后留下死链。

用法
    python scripts/check-doc-links.py                  # 校验 docs/（默认跳过 _archive）
    python scripts/check-doc-links.py --all            # 含 _archive
    python scripts/check-doc-links.py --repo           # 一并校验仓库其它 .md/.html 中对 docs/ 的引用
    python scripts/check-doc-links.py --strict         # 反引号路径引用也视为错误（默认仅告警）
    python scripts/check-doc-links.py --json out.json  # 输出机器可读报告

约定
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
import json
import os
import re
import sys
import urllib.parse
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DOCS_ROOT = REPO_ROOT / "docs"

SKIP_DIRS = {
    "node_modules", "target", ".git", "dist", ".codebuddy", "ais", "third_party",
    ".trae", "generated-images",
}
SKIP_DIR_NAMES_IN_DOCS = {"_archive"}  # 除非 --all

MD_LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
HTML_LINK = re.compile(r"""(?:href|src)\s*=\s*["']([^"']+)["']""", re.IGNORECASE)
CODE_REF = re.compile(r"`(docs/[^`\s]+)`")

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
            resolved, mode = normalize_target(raw)
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
            if exists:
                continue
            item = {"file": rel_file, "line": lineno, "target": raw, "kind": kind}
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
            print("  [BROKEN] %s:%s -> %s  [%s]" % (it["file"], it["line"], it["target"], it["kind"]))
    if warnings:
        print("\n告警·反引号路径引用不存在（%d，加 --strict 可视为错误）：" % len(warnings))
        for it in warnings[:max_detail]:
            print("  [WARN]   %s:%s -> %s" % (it["file"], it["line"], it["target"]))

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
    ap.add_argument("--max-detail", dest="max_detail", type=int, default=120, help="明细最多打印条数")
    args = ap.parse_args()
    sys.exit(check(args.all, args.repo, args.strict, args.json_path, args.max_detail))


if __name__ == "__main__":
    main()
