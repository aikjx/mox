#!/usr/bin/env python3
"""P0-1 文件名 rename — 扫描 docs/ 中含品牌串的文件名并执行 os.rename + git add。"""
import os, re, subprocess, sys

ROOT = os.path.join(os.path.dirname(__file__), '..', 'docs')
OLD = 'mox 模块化系统架构'
FLAG = os.path.join(os.path.dirname(__file__), '..', '.runtime', 'p01-rename-done.flag')

def new_name(fn):
    fn = re.sub(r'^(\d+)-mox 模块化系统架构需求明确书\.md$', r'\1-MOX需求明确书.md', fn)
    fn = re.sub(r'^(\d+)-mox 模块化系统架构自动化处理明确书\.md$', r'\1-MOX自动化处理明确书.md', fn)
    fn = re.sub(r'^(\d+)-企业级mox 模块化系统架构维度完成归档\.md$', r'\1-企业级MOX维度完成归档.md', fn)
    fn = re.sub(r'^(\d+)-mox 模块化系统架构测试验证优化修复报告\.md$', r'\1-MOX测试验证优化修复报告.md', fn)
    fn = re.sub(r'^(\d+)-mox 模块化系统架构愿景核心架构与业务流程总纲\.md$', r'\1-MOX愿景核心架构与业务流程总纲.md', fn)
    fn = re.sub(r'^(\d+)-产品规范标准-人人爱用全自动mox 模块化系统架构-(V[\d.]+)\.md$', r'\1-产品规范标准-人人爱用全自动MOX-\2.md', fn)
    fn = re.sub(r'^(\d+)-算子系统mox 模块化系统架构分析与归一化设计\.md$', r'\1-算子系统MOX分析与归一化设计.md', fn)
    fn = re.sub(r'^(\d+)-竞品mox 模块化系统架构功能对比与可用性判定报告-(V[\d.]+)\.md$', r'\1-竞品MOX功能对比与可用性判定报告-\2.md', fn)
    fn = re.sub(r'^(\d+)-企业级测试与评测主控提示词与mox 模块化系统架构自动化测试报告-(V[\d.]+)\.md$', r'\1-企业级测试与评测主控提示词与MOX自动化测试报告-\2.md', fn)
    fn = re.sub(r'^(\d+)-mox 模块化系统架构分析与文档归一化报告-(V[\d.]+)\.md$', r'\1-MOX分析与文档归一化报告-\2.md', fn)
    fn = re.sub(r'^(\d+)-企业级运维部署mox 模块化系统架构停机时间优化与灰度发布-(V[\d.]+)\.md$', r'\1-企业级运维部署MOX停机时间优化与灰度发布-\2.md', fn)
    fn = re.sub(r'^璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级\.md$', '璇玑-MOX需求业务处理流程图-归一化企业级.md', fn)
    return fn

def scan():
    renames = []
    for dp, _, fns in os.walk(ROOT):
        for fn in fns:
            if OLD in fn:
                old_path = os.path.join(dp, fn)
                nn = new_name(fn)
                if nn == fn:
                    continue
                new_path = os.path.join(dp, nn)
                renames.append((old_path, new_path))
    return renames

def main():
    renames = scan()
    # 写报告
    report_path = os.path.join(os.path.dirname(__file__), '..', 'reports', 'data', 'p01-rename-dryrun.txt')
    os.makedirs(os.path.dirname(report_path), exist_ok=True)
    with open(report_path, 'w', encoding='utf-8') as f:
        for old, new in renames:
            f.write(f'RENAME: {old}\n    -> {new}\n')
        f.write(f'\n总计: {len(renames)} 个文件\n')

    if '--apply' not in sys.argv:
        # 写 flag 文件表示 dry-run 完成
        with open(FLAG, 'w') as f:
            f.write(f'DRY_RUN:{len(renames)}\n')
        sys.exit(0)

    for old, new in renames:
        os.rename(old, new)
        subprocess.run(['git', 'add', old, new], check=True)
    with open(FLAG, 'w') as f:
        f.write(f'APPLIED:{len(renames)}\n')

if __name__ == '__main__':
    main()
