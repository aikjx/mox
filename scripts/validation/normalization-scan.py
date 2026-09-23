#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
全维归一化体检（Normalization Scan）
===================================

扫描 Rust 源码中的**公开符号重复定义**，量化架构归一化状态。

检查项：
- `pub enum` / `pub struct` / `pub const` / `pub type` 同名符号在两个及以上 crate 中定义
- 重复即意味着该概念没有单一真源（SSOT），跨 crate 使用时需要转换层或存在漂移风险

设计：
- 只统计 `pub` 符号（模块内部私有类型不产生跨 crate 契约，不算债）
- 按 crate 归属判定「重复」：同一 crate 内同名（如不同模块各定义一份）同样计入，
  因为那也是同 crate 内的重复真源（历史上 TaskStatus 就在同一 crate 两份）
- 注释与字符串内的匹配会被误计，故用行首锚定 + 排除 `//` 注释行

用法：
    python scripts/normalization-scan.py                 # 输出体检报告
    python scripts/normalization-scan.py --top 30        # 只看前 30 项
    python scripts/normalization-scan.py --kinds enum    # 只看枚举
    python scripts/normalization-scan.py --json out.json # 导出 JSON

退出码：始终为 0（体检工具只报告，不做门禁；门禁由 arch-test 承担）。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path

try:
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except (AttributeError, ValueError):
    pass

ROOT = Path(__file__).resolve().parents[1]
SCAN_ROOTS = ["platform"]
SKIP_DIRS = {"target", "node_modules", ".git", "third_party", "ais", "90_历史归档"}

SYMBOL_PATTERNS = {
    "enum": re.compile(r"^\s*pub\s+enum\s+([A-Za-z_]\w*)"),
    "struct": re.compile(r"^\s*pub\s+struct\s+([A-Za-z_]\w*)"),
    "const": re.compile(r"^\s*pub\s+const\s+([A-Za-z_]\w*)"),
    "type": re.compile(r"^\s*pub\s+type\s+([A-Za-z_]\w*)"),
}

# 再导出（pub use）不算新定义，用于把「已归一」的符号排除在重复之外
REEXPORT_PATTERN = re.compile(r"^\s*pub\s+use\s+.*\b([A-Za-z_]\w*)\s*;")

_crate_cache: dict[Path, str] = {}


def crate_of(path: Path) -> str:
    """向上查找最近的 Cargo.toml 所在目录名作为 crate 名。"""
    for parent in path.parents:
        cached = _crate_cache.get(parent)
        if cached:
            return cached
        if (parent / "Cargo.toml").exists():
            name = parent.name
            _crate_cache[parent] = name
            return name
    return "unknown"


def is_comment(line: str) -> bool:
    s = line.strip()
    return s.startswith("//") or s.startswith("///") or s.startswith("//!") or s.startswith("*")


def scan() -> dict[str, list[dict]]:
    """返回 {kind: [ {name, count, crates, locations} ]} 按次数降序。"""
    found: dict[str, dict[str, list[tuple[str, str, int]]]] = {
        k: defaultdict(list) for k in SYMBOL_PATTERNS
    }

    for scan_root in SCAN_ROOTS:
        base = ROOT / scan_root
        if not base.exists():
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            dirnames[:] = [d for d in dirnames if d not in SKIP_DIRS]
            for fn in filenames:
                if not fn.endswith(".rs"):
                    continue
                p = Path(dirpath) / fn
                try:
                    lines = p.read_text(encoding="utf-8", errors="replace").splitlines()
                except OSError:
                    continue
                crate = crate_of(p)
                rel = p.relative_to(ROOT).as_posix()
                for i, line in enumerate(lines, start=1):
                    if is_comment(line):
                        continue
                    for kind, pat in SYMBOL_PATTERNS.items():
                        m = pat.match(line)
                        if m:
                            found[kind][m.group(1)].append((crate, rel, i))
                            break

    result: dict[str, list[dict]] = {}
    for kind, mapping in found.items():
        items = []
        for name, locs in mapping.items():
            if len(locs) < 2:
                continue
            crates = sorted({c for c, _, _ in locs})
            items.append(
                {
                    "name": name,
                    "count": len(locs),
                    "crates": crates,
                    "cross_crate": len(crates) > 1,
                    "locations": [f"{r}:{ln}" for _, r, ln in locs],
                }
            )
        items.sort(key=lambda x: (-x["count"], x["name"]))
        result[kind] = items
    return result


def main() -> int:
    ap = argparse.ArgumentParser(description="全维归一化体检：公开符号重复定义扫描")
    ap.add_argument("--top", type=int, default=40, help="每类最多展示条数")
    ap.add_argument("--kinds", nargs="*", default=list(SYMBOL_PATTERNS), help="检查的符号类别")
    ap.add_argument("--json", metavar="PATH", help="导出 JSON 报告")
    ap.add_argument(
        "--baseline-txt",
        metavar="PATH",
        help="导出基线文本（每行 kind::name|crate1,crate2，含副本 crate 集合，仅跨 crate 重复项），供 arch-test 门禁做扩散检测",
    )
    ap.add_argument("--cross-crate-only", action="store_true", help="只展示跨 crate 重复")
    args = ap.parse_args()

    data = scan()

    total_dup = 0
    cross_dup = 0
    print("=" * 78)
    print("全维归一化体检 · 公开符号重复定义")
    print("=" * 78)

    for kind in args.kinds:
        items = data.get(kind, [])
        if args.cross_crate_only:
            items = [i for i in items if i["cross_crate"]]
        if not items:
            continue
        print()
        print(f"── pub {kind} 重复定义：{len(items)} 项 ──")
        for it in items[: args.top]:
            flag = "跨crate" if it["cross_crate"] else "同crate"
            print(
                f"  [{flag}] {it['name']}  ×{it['count']}  crates={','.join(it['crates'])}"
            )
            for loc in it["locations"][:4]:
                print(f"        {loc}")
            if it["count"] > 4:
                print(f"        ... 另 {it['count'] - 4} 处")
        total_dup += len(items)
        cross_dup += sum(1 for i in items if i["cross_crate"])

    print()
    print("-" * 78)
    print(f"重复符号合计：{total_dup} 项（其中跨 crate：{cross_dup} 项）")
    print("说明：本工具只体检，不做门禁；门禁规则见 platform/arch-test。")
    print("-" * 78)

    if args.json:
        out = Path(args.json)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
        print(f"JSON 报告已导出：{out}")

    if args.baseline_txt:
        out = Path(args.baseline_txt)
        out.parent.mkdir(parents=True, exist_ok=True)
        # 格式：kind::name|crate1,crate2 —— 副本 crate 集合使 arch-test 能检测
        # 「重复扩散」（已登记的重复又被第三个 crate 复制时门禁即失败）
        lines = sorted(
            f"{kind}::{it['name']}|{','.join(it['crates'])}"
            for kind in SYMBOL_PATTERNS
            for it in data.get(kind, [])
            if it["cross_crate"]
        )
        out.write_text("\n".join(lines) + "\n", encoding="utf-8")
        print(f"基线文本已导出：{out}（{len(lines)} 项，含副本 crate 集合）")

    return 0


if __name__ == "__main__":
    sys.exit(main())
