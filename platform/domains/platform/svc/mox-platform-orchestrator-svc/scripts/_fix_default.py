#!/usr/bin/env python3
"""修复 main.rs 中 get_full_status 的 Default trait 问题"""

from pathlib import Path

ORCH_ROOT = Path(__file__).resolve().parent.parent
path = ORCH_ROOT / "src/main.rs"

with open(path, "r", encoding="utf-8-sig", newline="") as f:
    content = f.read()

replacements = [
    ('resources.data.unwrap_or_default()', 'resources.data.unwrap()'),
    ('health.data.unwrap_or_default()', 'health.data.unwrap()'),
    ('plugins.data.unwrap_or_default()', 'plugins.data.unwrap()'),
]

count = 0
for old, new in replacements:
    if old in content:
        content = content.replace(old, new)
        count += 1
        print(f"  修复: {old} -> {new}")

with open(path, "w", encoding="utf-8", newline="") as f:
    f.write(content)

print(f"\n共修复 {count} 处")
