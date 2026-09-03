#!/usr/bin/env python3
"""修复 automation.rs 中 api_error 调用的 .into() 类型推断问题"""

from pathlib import Path

ORCH_ROOT = Path(__file__).resolve().parent.parent
path = ORCH_ROOT / "src/automation.rs"

with open(path, "r", encoding="utf-8-sig", newline="") as f:
    content = f.read()

# 移除 api_error 调用中字符串字面量的 .into()
# api_error(403, "xxx".into()) -> api_error(403, "xxx")
import re

# 匹配 api_error(CODE, "string".into())
pattern = r'(api_error\(\d+,\s*"[^"]*")\.into\(\)'
content, count = re.subn(pattern, r'\1', content)

with open(path, "w", encoding="utf-8", newline="") as f:
    f.write(content)

print(f"修复了 {count} 处 .into() 调用")
