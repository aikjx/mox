#!/usr/bin/env python3
"""Migrate lib.rs handlers to ApiResponse format."""
import re
import sys

path = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\lib.rs'

with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

original = content

# 1. Add import after 'use serde_json::json;'
content = content.replace(
    'use serde_json::json;\n',
    'use serde_json::json;\nuse mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};\n'
)

# 2. status_handler return type
content = content.replace(
    'async fn status_handler(State(state): State<GatewayState>) -> Json<serde_json::Value> {',
    'async fn status_handler(State(state): State<GatewayState>) -> ApiResponse<serde_json::Value> {'
)

# 3. status_handler body: Json(json!({ "ok": true, ... })) -> api_ok(json!({ ... }))
content = content.replace(
    '    Json(json!({\n        "ok": true,\n        "gateway": "rust-axum-enterprise",',
    '    api_ok(json!({\n        "gateway": "rust-axum-enterprise",'
)

# 4. domains_handler return type
content = content.replace(
    'async fn domains_handler() -> Json<serde_json::Value> {',
    'async fn domains_handler() -> ApiResponse<serde_json::Value> {'
)

# 5. domains_handler body
content = content.replace(
    '    Json(json!({\n        "ok": true,\n        "total": routes::DOMAINS.len(),',
    '    api_ok(json!({\n        "total": routes::DOMAINS.len(),'
)

if content == original:
    print("ERROR: No changes made!")
    sys.exit(1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

# Verify
remaining = len(re.findall(r'Json\s*\(\s*json!', content))
api_ok_count = content.count('api_ok(')
api_response_count = content.count('ApiResponse<')
has_import = 'mox_api_protocol' in content

print(f"lib.rs migration complete")
print(f"  Remaining Json(json!): {remaining} (health_handler should keep 1)")
print(f"  api_ok calls: {api_ok_count}")
print(f"  ApiResponse types: {api_response_count}")
print(f"  Import added: {has_import}")
