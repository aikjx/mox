# -*- coding: utf-8 -*-
"""定位 3717 与 3010 全量引用（排除第三方库/数据/历史报告）。"""
import os, re, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8", errors="replace")
ROOT = r"D:\a10\aikjx\gitcode\infotopograph"
SKIP_DIRS = {"node_modules","target","dist","ais","third_party",".git","__pycache__","build","release-pkg","venv",".venv","temp","models","downloads","_data"}
SKIP_EXT = {".png",".jpg",".jpeg",".gif",".ico",".woff",".woff2",".ttf",".pyc",".exe",".dll",".pdb",".map",".pdf",".wav",".mp3",".zip",".7z",".gz",".whl",".db",".sqlite",".lock",".min.js",".min.css",".log",".err",".out"}
for target in ("3717", "3010"):
    print("=" * 90)
    print("搜索端口:", target)
    print("=" * 90)
    rows = []
    for dp, dn, fn in os.walk(ROOT):
        dn[:] = [d for d in dn if d not in SKIP_DIRS and not d.startswith(".")]
        for f in fn:
            ext = os.path.splitext(f)[1].lower()
            if ext in SKIP_EXT:
                continue
            fp = os.path.join(dp, f)
            rel = os.path.relpath(fp, ROOT).replace("\\","/")
            # 排除数据/导出/历史报告
            if rel.startswith(("data/","docs/working-reports/","docs/_archive/","docs/specifications/tasks/","prototypes/","my_projects/","mox-workspace/")) and not rel.startswith(("mox-workspace/.env",)):
                # 但保留规范/权威文档
                if rel.startswith(("docs/standards/","docs/ports/")):
                    pass
                else:
                    continue
            try:
                t = open(fp, encoding="utf-8", errors="replace").read()
            except Exception:
                continue
            if target not in t:
                continue
            for ln_no, ln in enumerate(t.splitlines(), 1):
                if target in ln:
                    rows.append((rel, ln_no, ln.strip()[:160]))
    seen = set()
    for rel, no, ln in rows:
        key = (rel, no)
        if key in seen:
            continue
        seen.add(key)
        print(f"  {rel}:{no} :: {ln}")
    print("  共", len(rows), "处")
