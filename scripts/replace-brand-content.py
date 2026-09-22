#!/usr/bin/env python3
"""批量替换所有文本文件中的 'mox 模块化系统架构' 为 'MOX'"""
import os
import sys

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

POLLUTED = "mox 模块化系统架构"
CLEAN = "MOX"

# 需要扫描的目录（递归）
SCAN_DIRS = [
    "docs",
    "reports/markdown",
    "platform",
    "frontend-ui/src",
    "frontend-ui/知识库-璇玑系统mox 模块化系统架构功能图谱.md",  # will be renamed
]

# 需要扫描的文件扩展名
SCAN_EXTS = {".md", ".html", ".json", ".sql", ".py", ".mmd", ".js", ".ts", ".vue", ".rs", ".toml", ".txt", ".yaml", ".yml", ".bat", ".ps1", ".sh"}

# 跳过的目录
SKIP_DIRS = {".git", "node_modules", "target", "_archive", ".trae", ".pytest_cache", "__pycache__", "dist"}

def scan_and_replace():
    total_files = 0
    total_replacements = 0
    modified_files = []
    
    for scan_dir in SCAN_DIRS:
        full_path = os.path.join(BASE, scan_dir.replace("/", os.sep))
        if not os.path.exists(full_path):
            print(f"SKIP (not found): {scan_dir}")
            continue
        
        if os.path.isfile(full_path):
            files = [full_path]
        else:
            files = []
            for root, dirs, filenames in os.walk(full_path):
                # filter skip dirs
                dirs[:] = [d for d in dirs if d not in SKIP_DIRS]
                for fn in filenames:
                    ext = os.path.splitext(fn)[1].lower()
                    if ext in SCAN_EXTS:
                        files.append(os.path.join(root, fn))
        
        for filepath in files:
            try:
                with open(filepath, "r", encoding="utf-8") as f:
                    content = f.read()
            except Exception:
                try:
                    with open(filepath, "r", encoding="utf-8-sig") as f:
                        content = f.read()
                except Exception:
                    try:
                        with open(filepath, "r", encoding="gbk") as f:
                            content = f.read()
                    except Exception as e:
                        print(f"READ ERROR: {filepath}: {e}")
                        continue
            
            count = content.count(POLLUTED)
            if count == 0:
                continue
            
            new_content = content.replace(POLLUTED, CLEAN)
            
            try:
                with open(filepath, "w", encoding="utf-8") as f:
                    f.write(new_content)
                rel = os.path.relpath(filepath, BASE)
                print(f"OK ({count} replacements): {rel}")
                total_files += 1
                total_replacements += count
                modified_files.append((rel, count))
            except Exception as e:
                print(f"WRITE ERROR: {filepath}: {e}")
    
    print(f"\n总计: {total_files} 个文件被修改, {total_replacements} 处替换")
    return modified_files

if __name__ == "__main__":
    scan_and_replace()
