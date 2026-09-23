#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
术语门禁（Term Guard）
=====================

禁止历史批量替换事故产生的污染术语再次进入「可执行面」：
代码字符串字面量、配置字段、测试断言、脚本命令。

设计取舍：
- 只扫可执行面，跳过注释行（`//`、`///`、`//!`、`#`、`*`）。
  理由：注释层的历史存量较大，一次性批量回退容易产生新的错误术语；
  先卡住可执行面，注释层按文件逐步清理。
- 术语表与扫描范围集中在本文件顶部，便于随治理进展增补。

用法：
    python scripts/gate/check-forbidden-terms.py            # 检查，命中则退出码 1
    python scripts/gate/check-forbidden-terms.py --verbose  # 打印已扫描文件数

对齐：docs/working-reports/_norm_research/ai-engine-modularization-normalization.md SSOT-6
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

# Windows GBK 控制台下打印 emoji / 特殊字符会抛 UnicodeEncodeError（历史踩坑），
# 必须显式把标准输出切到 UTF-8 且容错替换，否则门禁脚本自身崩溃。
try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, ValueError):
    pass

# ── 禁用术语表 ──────────────────────────────────────────────────
# 来源：历史批量替换把「架构分析」类术语整体替换为该串，污染了注释、
# Cargo.toml description、路由关键词表与测试断言。
FORBIDDEN_TERMS = [
    "mox 模块化系统架构",
]

# ── 扫描范围 ────────────────────────────────────────────────────
SCAN_ROOTS = [
    "platform",
    "frontend-ui/src",
    "scripts",
    "config",
    "deploy",
]

SCAN_EXTS = {
    ".rs", ".toml", ".yml", ".yaml", ".json",
    ".js", ".ts", ".vue", ".py", ".sh", ".ps1", ".sql",
}

SKIP_DIRS = {
    "target", "node_modules", "dist", ".git", ".idea",
    "third_party", "ais", "90_历史归档", "__pycache__",
}

# 注释行前缀（strip 后判断），命中则跳过整行
COMMENT_PREFIXES = ("//", "///", "//!", "#", "*", "/*", "<!--")


def is_comment_line(line: str) -> bool:
    stripped = line.strip()
    return any(stripped.startswith(p) for p in COMMENT_PREFIXES)


def scan_file(path: Path) -> list[tuple[int, str, str]]:
    """返回 (行号, 命中术语, 行内容) 列表。"""
    hits: list[tuple[int, str, str]] = []
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return hits

    for lineno, line in enumerate(text.splitlines(), start=1):
        if is_comment_line(line):
            continue
        for term in FORBIDDEN_TERMS:
            if term in line:
                hits.append((lineno, term, line.strip()))
                break
    return hits


def iter_files(root: Path):
    # 门禁脚本自身持有禁用术语的定义，必须跳过，否则必然自命中
    self_path = Path(__file__).resolve()
    for scan_root in SCAN_ROOTS:
        base = root / scan_root
        if not base.exists():
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
            for name in filenames:
                p = Path(dirpath) / name
                if p.resolve() == self_path:
                    continue
                if p.suffix.lower() in SCAN_EXTS:
                    yield p


def main() -> int:
    parser = argparse.ArgumentParser(description="术语门禁：禁止污染术语进入可执行面")
    parser.add_argument("--verbose", action="store_true", help="打印扫描统计")
    parser.add_argument(
        "--stats",
        action="store_true",
        help="打印污染术语的上下文模式频次（用于制定批量回退方案）",
    )
    args = parser.parse_args()

    root = Path(__file__).resolve().parents[2]
    total = 0
    failures: list[tuple[Path, int, str, str]] = []

    for path in iter_files(root):
        total += 1
        for lineno, term, content in scan_file(path):
            failures.append((path, lineno, term, content))

    if failures and args.stats:
        print("[STATS] 污染术语上下文模式频次（前 6 字 | 术语 | 后 6 字）：")
        patterns: dict[tuple[str, str], int] = {}
        for _, _, term, content in failures:
            idx = content.find(term)
            before = content[max(0, idx - 6):idx]
            after = content[idx + len(term):idx + len(term) + 6]
            patterns[(before, after)] = patterns.get((before, after), 0) + 1
        for (before, after), cnt in sorted(patterns.items(), key=lambda kv: -kv[1]):
            print("  " + str(cnt).rjust(3) + "  [" + before + "|" + after + "]")
        print()

    if failures:
        print("[FAIL] 术语门禁未通过，命中 " + str(len(failures)) + " 处：")
        for path, lineno, term, content in failures:
            rel = path.relative_to(root).as_posix()
            print("  " + rel + ":" + str(lineno) + "  [" + term + "]")
            print("      " + content[:160])
        print()
        print("修复方式：将污染术语回退为正确业务术语（见 SSOT-6 术语表）。")
        return 1

    print("[OK] 术语门禁通过：扫描 " + str(total) + " 个文件，0 处命中")
    if args.verbose:
        print("     禁用术语：" + ", ".join(FORBIDDEN_TERMS))
    return 0


if __name__ == "__main__":
    sys.exit(main())
