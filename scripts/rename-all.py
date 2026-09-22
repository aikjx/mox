import os
import sys
import time

# Wait a bit for shell to stabilize
time.sleep(2)

BASE = r"d:\a10\aikjx\gitcode\infotopograph"

renames = [
    # enterprise/ - large files (only filename changes, content already clean)
    (r"docs\enterprise\27-企业级测试与评测主控提示词与mox 模块化系统架构自动化测试报告-V1.0.md",
     r"docs\enterprise\27-企业级测试与评测主控提示词与MOX自动化测试报告-V1.0.md"),
    (r"docs\enterprise\28-mox 模块化系统架构分析与文档归一化报告-V1.0.md",
     r"docs\enterprise\28-MOX分析与文档归一化报告-V1.0.md"),
    (r"docs\enterprise\23-竞品mox 模块化系统架构功能对比与可用性判定报告-V1.0.md",
     r"docs\enterprise\23-竞品MOX功能对比与可用性判定报告-V1.0.md"),
    (r"docs\enterprise\MOX-AI驱动mox 模块化系统架构平台-企业级设计-mox 模块化系统架构分析-v3.0.md",
     r"docs\enterprise\MOX-AI驱动MOX平台-企业级设计-MOX分析-v3.0.md"),
    # modules/
    (r"docs\modules\璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.html",
     r"docs\modules\璇玑-MOX需求业务处理流程图-归一化企业级.html"),
    (r"docs\modules\璇玑-mox 模块化系统架构需求业务处理流程图-归一化企业级.md",
     r"docs\modules\璇玑-MOX需求业务处理流程图-归一化企业级.md"),
    (r"docs\modules\璇玑-mox 模块化系统架构流水线.mmd",
     r"docs\modules\璇玑-MOX流水线.mmd"),
    (r"docs\modules\专家联盟-mox 模块化系统架构业务流程归一化手册-V1.0.md",
     r"docs\modules\专家联盟-MOX业务流程归一化手册-V1.0.md"),
    (r"docs\modules\对话开发系统-mox 模块化系统架构分析与业务流程图.md",
     r"docs\modules\对话开发系统-MOX分析与业务流程图.md"),
    # reports/markdown/
    (r"reports\markdown\mox 模块化系统架构分析报告.md",
     r"reports\markdown\MOX分析报告.md"),
    (r"reports\markdown\专家联盟mox 模块化系统架构开发交付报告.md",
     r"reports\markdown\专家联盟MOX开发交付报告.md"),
    (r"reports\markdown\后端端点mox 模块化系统架构维度审计报告.md",
     r"reports\markdown\后端端点MOX维度审计报告.md"),
    (r"reports\markdown\接口核验-mox 模块化系统架构归一化总控.md",
     r"reports\markdown\接口核验-MOX归一化总控.md"),
    # frontend-ui
    (r"frontend-ui\知识库-璇玑系统mox 模块化系统架构功能图谱.md",
     r"frontend-ui\知识库-璇玑系统MOX功能图谱.md"),
]

results = []
for old_rel, new_rel in renames:
    old_path = os.path.join(BASE, old_rel)
    new_path = os.path.join(BASE, new_rel)
    try:
        if os.path.exists(old_path):
            os.rename(old_path, new_path)
            results.append(f"OK: {old_rel} -> {new_rel}")
        else:
            results.append(f"SKIP (not found): {old_rel}")
    except Exception as e:
        results.append(f"FAIL: {old_rel}: {e}")

# Write results to a file
with open(os.path.join(BASE, "scripts", "rename-results.txt"), "w", encoding="utf-8") as f:
    f.write("\n".join(results))
    f.write(f"\n\nTotal: {len([r for r in results if r.startswith('OK')])} OK, {len([r for r in results if r.startswith('FAIL')])} FAIL, {len([r for r in results if r.startswith('SKIP')])} SKIP")
