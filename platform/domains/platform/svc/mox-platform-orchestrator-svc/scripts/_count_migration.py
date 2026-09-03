#!/usr/bin/env python3
"""统计迁移的 handler 数量和涉及文件"""

from pathlib import Path

ORCH_ROOT = Path(__file__).resolve().parent.parent

files = [
    "src/main.rs",
    "src/handlers/ai_engine.rs",
    "src/handlers/governance.rs",
    "src/handlers/hitl.rs",
    "src/handlers/agent.rs",
    "src/automation.rs",
    "src/market.rs",
    "src/market_dsl.rs",
    "src/market_version.rs",
    "src/routes/ai_engine.rs",
    "src/routes/market.rs",
]

total_handlers = 0
files_with_handlers = []

for f in files:
    path = ORCH_ROOT / f
    with open(path, "r", encoding="utf-8-sig", newline="") as fh:
        content = fh.read()
    count = content.count("-> ApiResponse<")
    if count > 0:
        print(f"  {f}: {count} 个 handler")
        total_handlers += count
        files_with_handlers.append(f)

print(f"\n总计: {total_handlers} 个迁移的 handler")
print(f"涉及文件: {len(files_with_handlers)} 个")
print("\n文件列表:")
for f in files_with_handlers:
    print(f"  - {f}")
