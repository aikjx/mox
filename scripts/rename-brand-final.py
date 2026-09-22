#!/usr/bin/env python3
"""
批量品牌归一化脚本：重命名文件名中的 'mox 模块化系统架构' 为 'MOX'
并替换文件内容中的 'mox 模块化系统架构' 为 'MOX'

用法: python scripts/rename-brand-final.py
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCS = os.path.join(ROOT, "docs")

OLD_BRAND = "mox 模块化系统架构"
NEW_BRAND = "MOX"

# 文件名替换映射（旧名片段 -> 新名片段）
FILENAME_REPLACEMENTS = [
    # enterprise
    ("mox 模块化系统架构愿景核心架构与业务流程总纲", "MOX愿景核心架构与业务流程总纲"),
    ("mox 模块化系统架构分析与文档归一化报告", "MOX分析与文档归一化报告"),
    ("mox 模块化系统架构自动化测试报告", "MOX自动化测试报告"),
    ("竞品mox 模块化系统架构功能对比", "竞品MOX功能对比"),
    ("算子系统mox 模块化系统架构分析", "算子系统MOX分析"),
    ("人人爱用全自动mox 模块化系统架构", "人人爱用全自动MOX"),
    ("MOX-AI驱动mox 模块化系统架构平台-企业级设计-mox 模块化系统架构分析", "MOX-AI驱动MOX平台-企业级设计-MOX分析"),
    # modules
    ("璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级", "璇玑-MOX需求业务处理流程图-归一化企业级"),
    ("璇玑-mox 模块化系统架构流水线", "璇玑-MOX流水线"),
    ("专家联盟-mox 模块化系统架构业务流程归一化手册", "专家联盟-MOX业务流程归一化手册"),
    ("对话开发系统-mox 模块化系统架构分析与业务流程图", "对话开发系统-MOX分析与业务流程图"),
    # _archive (low priority but include for completeness)
    ("璇玑-mox 模块化系统架构分析需求业务处理流程图", "璇玑-MOX分析需求业务处理流程图"),
    ("璇玑-mox 模块化系统架构分析需求-测试分析验证报告", "璇玑-MOX分析需求-测试分析验证报告"),
    ("璇玑-mox 模块化系统架构分析-TraceMatrix-六维绑定追溯", "璇玑-MOX分析-TraceMatrix-六维绑定追溯"),
    ("PrimiFlow-mox 模块化系统架构分析-核心功能补全", "PrimiFlow-MOX分析-核心功能补全"),
]

def fix_filenames(directory):
    """重命名目录中的文件"""
    renamed = []
    for root, dirs, files in os.walk(directory):
        for f in files:
            old_path = os.path.join(root, f)
            new_name = f
            for old_frag, new_frag in FILENAME_REPLACEMENTS:
                if old_frag in new_name:
                    new_name = new_name.replace(old_frag, new_frag)
            if new_name != f:
                new_path = os.path.join(root, new_name)
                if os.path.exists(new_path):
                    print(f"  SKIP (target exists): {f} -> {new_name}")
                    continue
                os.rename(old_path, new_path)
                renamed.append((f, new_name))
                print(f"  RENAMED: {f} -> {new_name}")
    return renamed

def fix_content(directory, skip_archive=True):
    """替换文件内容中的品牌字符串"""
    fixed = []
    for root, dirs, files in os.walk(directory):
        if skip_archive and "_archive" in root:
            continue
        for f in files:
            if not f.endswith(('.md', '.html', '.js', '.json', '.py', '.mmd', '.sql', '.txt')):
                continue
            fpath = os.path.join(root, f)
            try:
                with open(fpath, 'r', encoding='utf-8') as fh:
                    content = fh.read()
            except (UnicodeDecodeError, PermissionError):
                continue
            if OLD_BRAND in content:
                new_content = content.replace(OLD_BRAND, NEW_BRAND)
                with open(fpath, 'w', encoding='utf-8') as fh:
                    fh.write(new_content)
                count = content.count(OLD_BRAND)
                fixed.append((f, count))
                print(f"  CONTENT FIXED: {f} ({count} occurrences)")
    return fixed

def main():
    print("=" * 60)
    print("品牌归一化脚本：'mox 模块化系统架构' -> 'MOX'")
    print("=" * 60)
    
    print("\n[1/2] 替换文件内容（跳过 _archive/）...")
    fixed = fix_content(DOCS, skip_archive=True)
    print(f"  共修复 {len(fixed)} 个文件")
    
    print("\n[2/2] 重命名文件...")
    renamed = fix_filenames(DOCS)
    print(f"  共重命名 {len(renamed)} 个文件")
    
    print("\n" + "=" * 60)
    print(f"完成！内容修复: {len(fixed)} 文件, 文件名重命名: {len(renamed)} 文件")
    print("=" * 60)

if __name__ == "__main__":
    main()
