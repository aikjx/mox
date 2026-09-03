#!/usr/bin/env python3
"""修复 main.rs 中 get_full_status 的 .0 字段访问"""

from pathlib import Path

ORCH_ROOT = Path(__file__).resolve().parent.parent
path = ORCH_ROOT / "src/main.rs"

with open(path, "r", encoding="utf-8-sig", newline="") as f:
    content = f.read()

# 修复 .0 访问为 .data.unwrap_or_default()
replacements = [
    ('"system": basic.0,', '"system": basic.data.unwrap_or_default(),'),
    ('"resources": resources.0,', '"resources": resources.data.unwrap_or_default(),'),
    ('"health": health.0,', '"health": health.data.unwrap_or_default(),'),
    ('"ai_plugins": plugins.0,', '"ai_plugins": plugins.data.unwrap_or_default(),'),
]

count = 0
for old, new in replacements:
    if old in content:
        content = content.replace(old, new)
        count += 1
        print(f"  修复: {old.strip()} -> {new.strip()}")

with open(path, "w", encoding="utf-8", newline="") as f:
    f.write(content)

print(f"\n共修复 {count} 处 .0 访问")
