#!/usr/bin/env python3
"""Fix remaining direct Json(json!) error responses in monitor.rs and experts_ext.rs."""
import re

BASE = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src'

# --- monitor.rs: 3 remaining error responses ---
path = f'{BASE}\\monitor.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

# All 3 are the same single-line pattern
old_err = 'Json(json!({ "success": false, "error": format!("alert rule not found: {id}") }))'
new_err = 'api_error(404, format!("alert rule not found: {id}"))'
count = content.count(old_err)
content = content.replace(old_err, new_err)
print(f"monitor.rs: replaced {count} error responses")

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

# --- experts_ext.rs: 2 remaining error responses ---
path = f'{BASE}\\experts_ext.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

# 1. Multi-line error response in cancel_booking
old_err1 = '''            return Json(json!({
                "success": false,
                "error": format!("预约 {} 当前状态为 {}，无法取消", id, b.status),
            }));'''
new_err1 = '''            return api_error(400, format!("预约 {} 当前状态为 {}，无法取消", id, b.status));'''
if old_err1 in content:
    content = content.replace(old_err1, new_err1)
    print("experts_ext.rs: replaced multi-line error response")
else:
    print("experts_ext.rs: WARNING - multi-line pattern not found, trying alternate...")
    # Try with different indentation
    old_err1b = '''return Json(json!({
                "success": false,
                "error": format!("预约 {} 当前状态为 {}，无法取消", id, b.status),
            }));'''
    if old_err1b in content:
        content = content.replace(old_err1b, new_err1)
        print("experts_ext.rs: replaced multi-line error response (alt indent)")

# 2. Single-line error response
old_err2 = 'Json(json!({ "success": false, "error": format!("booking not found: {id}") }))'
new_err2 = 'api_error(404, format!("booking not found: {id}"))'
count2 = content.count(old_err2)
content = content.replace(old_err2, new_err2)
print(f"experts_ext.rs: replaced {count2} single-line error responses")

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

# Verify
for fname in ['monitor.rs', 'experts_ext.rs']:
    with open(f'{BASE}\\{fname}', 'r', encoding='utf-8-sig') as f:
        c = f.read()
    jj = len(re.findall(r'Json\s*\(\s*json!', c))
    print(f"  {fname}: remaining Json(json!) = {jj}")
