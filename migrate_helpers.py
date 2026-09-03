#!/usr/bin/env python3
"""Migrate helper-based handler files to ApiResponse format.
Files: system.rs, monitor.rs, workspace.rs, projects_ext.rs, experts_ext.rs, misc.rs
Pattern: all use fn ok(data: Value) -> Json<Value> { Json(json!({"success": true, "data": data})) }
"""
import re
import sys

BASE = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src'

FILES = ['system.rs', 'monitor.rs', 'workspace.rs', 'projects_ext.rs', 'experts_ext.rs', 'misc.rs']

IMPORT_LINE = 'use mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};\n'

for fname in FILES:
    path = f'{BASE}\\{fname}'
    with open(path, 'r', encoding='utf-8-sig', newline='') as f:
        content = f.read()
    original = content

    # 1. Add import - find a good insertion point (after last 'use ...;' line at top)
    # We'll insert after the first block of use statements
    if 'mox_api_protocol' not in content:
        # Find the last use statement before the first const/fn/struct
        lines = content.split('\n')
        insert_idx = None
        for i, line in enumerate(lines):
            stripped = line.strip()
            if stripped.startswith('use ') and stripped.endswith(';'):
                insert_idx = i
            elif insert_idx is not None and stripped and not stripped.startswith('//') and not stripped.startswith('use '):
                break
        if insert_idx is not None:
            lines.insert(insert_idx + 1, IMPORT_LINE.rstrip('\n'))
            content = '\n'.join(lines)

    # 2. Replace ok() helper
    old_ok = '''fn ok(data: Value) -> Json<Value> {
    Json(json!({ "success": true, "data": data }))
}'''
    new_ok = '''fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}'''
    content = content.replace(old_ok, new_ok)

    # 3. Replace err() helper (system.rs only)
    old_err = '''fn err(msg: &str) -> Json<Value> {
    Json(json!({ "success": false, "code": "IAM_REPO_ERR", "error": msg }))
}'''
    new_err = '''fn err(msg: &str) -> ApiResponse<Value> {
    api_error(500, msg)
}'''
    content = content.replace(old_err, new_err)

    # 4. Replace all -> Json<Value> with -> ApiResponse<Value>
    # But NOT in use statements or type aliases. Only in function return types.
    # Pattern: ") -> Json<Value>" or "-> Json<Value> {"
    content = re.sub(r'->\s*Json<Value>', '-> ApiResponse<Value>', content)

    # 5. Check for any remaining direct Json(json!) calls (outside of ok/err which we already changed)
    remaining_json = re.findall(r'Json\s*\(\s*json!', content)

    if content == original:
        print(f"  {fname}: WARNING - no changes made!")
    else:
        with open(path, 'w', encoding='utf-8', newline='') as f:
            f.write(content)
        handler_count = len(re.findall(r'async\s+fn\s+\w+.*->\s*ApiResponse', content, re.DOTALL))
        print(f"  {fname}: migrated (ApiResponse handlers: ~{handler_count}, remaining Json(json!): {len(remaining_json)})")

print("Done.")
