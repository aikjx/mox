#!/usr/bin/env python3
"""
P0-1 品牌串污染修复脚本 | DOC-FIX-P01-V1.0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
目标：docs/ L0-L8 governed 树内 `mox 模块化系统架构` 归一化
原则：
  - 作为产品/平台主体名 → "MOX"
  - 作为维度描述前缀  → "MOX全维"
  - _archive/ 跳过（L8 只读）
  - reports/ 跳过（独立治理域）
  - 文件名含该词的同步 rename
"""

import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"

# 跳过的目录（相对 docs/ 的顶层）
SKIP_DIRS = {"_archive"}

# 匹配模式
TERM = "mox 模块化系统架构"


def classify_line(line: str) -> str:
    """判断一行中该术语的语境类别。"""
    # 维度描述
    if re.search(r"mox\s*模块化系统架构\s*(维度|工程视图|分析|框架)", line):
        return "dimension"
    # 作为产品/平台名（标题、句首）
    return "product"


def scan_file(path: Path) -> list[dict]:
    """扫描文件，返回命中行信息。"""
    hits = []
    try:
        text = path.read_text(encoding="utf-8")
    except Exception:
        return hits
    for i, line in enumerate(text.splitlines(), 1):
        if TERM in line:
            hits.append({"line": i, "ctx": classify_line(line), "text": line.rstrip()[:120]})
    return hits


def plan() -> dict:
    """生成修复计划。"""
    plan_map: dict[str, list[dict]] = {}
    for dirpath, dirnames, filenames in os.walk(DOCS):
        # 跳过 _archive 等
        rel = Path(dirpath).relative_to(DOCS).parts
        if rel and rel[0] in SKIP_DIRS:
            dirnames.clear()
            continue
        for fn in filenames:
            if not fn.endswith((".md", ".html")):
                continue
            p = Path(dirpath) / fn
            hits = scan_file(p)
            if hits:
                file_needs_rename = TERM in fn
                plan_map[str(p.relative_to(ROOT))] = {
                    "hits": hits,
                    "rename": file_needs_rename,
                    "new_name": fn.replace(TERM, "MOX") if file_needs_rename else None,
                }
    return plan_map


def dry_run():
    plan_map = plan()
    prod_files = sum(1 for v in plan_map.values() if any(h["ctx"] == "product" for h in v["hits"]))
    dim_files = sum(1 for v in plan_map.values() if any(h["ctx"] == "dimension" for h in v["hits"]))
    rename_files = [k for k, v in plan_map.items() if v["rename"]]
    total_hits = sum(len(v["hits"]) for v in plan_map.values())

    print("=" * 72)
    print(" P0-1 品牌串污染修复 · DRY-RUN 预览")
    print("=" * 72)
    print(f"  命中文件总数：{len(plan_map)}")
    print(f"  命中行总数  ：{total_hits}")
    print(f"  产品名语境  ：{prod_files} 文件（→ MOX）")
    print(f"  维度描述语境：{dim_files} 文件（→ MOX全维）")
    print(f"  需改名文件  ：{len(rename_files)}")
    print()
    if rename_files:
        print("── 文件 rename 清单 ──")
        for rp in rename_files:
            new = plan_map[rp]["new_name"]
            old_name = Path(rp).name
            new_name = Path(new).name if new else old_name
            print(f"  {old_name}")
            print(f"    → {new_name}")
    print()
    print("── 首 10 文件 hit 详情 ──")
    for i, (rp, info) in enumerate(plan_map.items()):
        if i >= 10:
            break
        tag = "[更名]" if info["rename"] else ""
        print(f"  {rp}  ({len(info['hits'])} 行) {tag}")
        for h in info["hits"][:2]:
            print(f"    L{h['line']} [{h['ctx']}] {h['text']}")
    print()
    print("=" * 72)
    print(" 以上为预览，未改动任何文件。")
    print(" 确认后执行：python scripts/p01-brand-normalize.py --apply")
    print("=" * 72)


def apply_fix():
    """执行修复。"""
    plan_map = plan()
    changed = 0
    renamed = 0
    for rp, info in plan_map.items():
        p = ROOT / rp
        text = p.read_text(encoding="utf-8")
        new_text = text.replace(TERM, "MOX")
        if new_text != text:
            p.write_text(new_text, encoding="utf-8")
            changed += 1
        # rename file
        if info["rename"] and info["new_name"]:
            new_path = p.parent / info["new_name"]
            if new_path != p:
                p.rename(new_path)
                renamed += 1
    print(f"✓ 正文修复 {changed} 文件")
    print(f"✓ 文件改名 {renamed} 个")
    print(" 提示：需手动确认跨文件引用并提交（git mv 可由 git 自动追踪）")


if __name__ == "__main__":
    # 自包含输出到文件，避免终端超时丢失
    import io
    buf = io.StringIO()
    old_stdout = sys.stdout
    sys.stdout = buf
    if "--apply" in sys.argv:
        apply_fix()
    else:
        dry_run()
    sys.stdout = old_stdout
    out = buf.getvalue()
    print(out, end="")
    out_path = ROOT / "reports" / "data" / "p01-dryrun.txt"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(out, encoding="utf-8")
    print(f"\n[报告已写入 {out_path}]")
