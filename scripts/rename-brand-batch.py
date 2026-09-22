#!/usr/bin/env python3
"""批量重命名文件名中包含 'mox 模块化系统架构' 的文件，替换为 'MOX'"""
import os
import sys

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# 所有需要重命名的文件 (relative_path, new_filename)
RENAMES = [
    # enterprise/
    ("docs/enterprise/27-企业级测试与评测主控提示词与mox 模块化系统架构自动化测试报告-V1.0.md",
     "27-企业级测试与评测主控提示词与MOX自动化测试报告-V1.0.md"),
    ("docs/enterprise/28-mox 模块化系统架构分析与文档归一化报告-V1.0.md",
     "28-MOX分析与文档归一化报告-V1.0.md"),
    ("docs/enterprise/23-竞品mox 模块化系统架构功能对比与可用性判定报告-V1.0.md",
     "23-竞品MOX功能对比与可用性判定报告-V1.0.md"),
    ("docs/enterprise/MOX-AI驱动mox 模块化系统架构平台-企业级设计-mox 模块化系统架构分析-v3.0.md",
     "MOX-AI驱动MOX平台-企业级设计-MOX分析-v3.0.md"),
    ("docs/enterprise/14-mox 模块化系统架构愿景核心架构与业务流程总纲.html",
     "14-MOX愿景核心架构与业务流程总纲.html"),
    ("docs/enterprise/15-产品规范标准-人人爱用全自动mox 模块化系统架构-V1.0.html",
     "15-产品规范标准-人人爱用全自动MOX-V1.0.html"),
    # modules/
    ("docs/modules/璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.html",
     "璇玑-MOX需求业务处理流程图-归一化企业级.html"),
    ("docs/modules/璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.md",
     "璇玑-MOX需求业务处理流程图-归一化企业级.md"),
    ("docs/modules/璇玑-mox 模块化系统架构流水线.mmd",
     "璇玑-MOX流水线.mmd"),
    ("docs/modules/专家联盟-mox 模块化系统架构业务流程归一化手册-V1.0.md",
     "专家联盟-MOX业务流程归一化手册-V1.0.md"),
    ("docs/modules/对话开发系统-mox 模块化系统架构分析与业务流程图.md",
     "对话开发系统-MOX分析与业务流程图.md"),
    # reports/markdown/
    ("reports/markdown/mox 模块化系统架构分析报告.md",
     "MOX分析报告.md"),
    ("reports/markdown/专家联盟mox 模块化系统架构开发交付报告.md",
     "专家联盟MOX开发交付报告.md"),
    ("reports/markdown/后端端点mox 模块化系统架构维度审计报告.md",
     "后端端点MOX维度审计报告.md"),
    ("reports/markdown/接口核验-mox 模块化系统架构归一化总控.md",
     "接口核验-MOX归一化总控.md"),
    # _archive/
    ("docs/_archive/2026-08-16/璇玑-mox 模块化系统架构分析需求业务处理流程图.md",
     "璇玑-MOX分析需求业务处理流程图.md"),
    ("docs/_archive/2026-08-16/璇玑-mox 模块化系统架构分析需求-测试分析验证报告.md",
     "璇玑-MOX分析需求-测试分析验证报告.md"),
    ("docs/_archive/2026-08-16/璇玑-mox 模块化系统架构分析-TraceMatrix-六维绑定追溯.md",
     "璇玑-MOX分析-TraceMatrix-六维绑定追溯.md"),
    ("docs/_archive/2026-08-16/PrimiFlow-mox 模块化系统架构分析-核心功能补全-20260816.md",
     "PrimiFlow-MOX分析-核心功能补全-20260816.md"),
    ("docs/_archive/prototypes/data-vis/全维分析流程/mox 模块化系统架构分析需求业务处理流程图.html",
     "MOX分析需求业务处理流程图.html"),
    # frontend-ui
    ("frontend-ui/知识库-璇玑系统mox 模块化系统架构功能图谱.md",
     "知识库-璇玑系统MOX功能图谱.md"),
]

success = 0
skipped = 0
failed = 0

for old_rel, new_name in RENAMES:
    old_path = os.path.join(BASE, old_rel.replace("/", os.sep))
    dir_name = os.path.dirname(old_path)
    new_path = os.path.join(dir_name, new_name)
    
    if not os.path.exists(old_path):
        print(f"SKIP (not found): {old_rel}")
        skipped += 1
        continue
    
    if old_path == new_path:
        print(f"SKIP (same): {old_rel}")
        skipped += 1
        continue
    
    try:
        os.rename(old_path, new_path)
        print(f"OK: {old_rel} -> {new_name}")
        success += 1
    except Exception as e:
        print(f"FAIL: {old_rel} -> {new_name}: {e}")
        failed += 1

print(f"\n总计: {success} 成功, {skipped} 跳过, {failed} 失败")
