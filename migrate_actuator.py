#!/usr/bin/env python3
"""Migrate actuator.rs handlers to ApiResponse format.
Special cases:
- actuator_loggers_set returns -> Response with .into_response()
- actuator_logs_tail is SSE -> keep as-is
- set_api_enabled is a helper fn
- Some success blocks have "ok": true, some don't
- Error blocks have extra diagnostic fields
"""
import re

path = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\actuator.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

original = content

# 1. Add import after 'use crate::GatewayState;'
content = content.replace(
    'use crate::GatewayState;\n',
    'use crate::GatewayState;\nuse mox_api_protocol::{ApiResponse, api_ok, api_error, api_ok_empty};\n'
)

# 2. Change return types: -> Json<Value> -> -> ApiResponse<Value>
# But NOT for the SSE handler (actuator_logs_tail returns -> Response)
# and NOT for actuator_loggers_set (returns -> Response, handled separately)
content = re.sub(r'->\s*Json<Value>', '-> ApiResponse<Value>', content)

# 3. actuator_index: no "ok" field
content = content.replace(
    '    Json(json!({\n        "_links": endpoints,',
    '    api_ok(json!({\n        "_links": endpoints,'
)

# 4. actuator_health: no "ok" field
content = content.replace(
    '    Json(json!({\n        "status": "UP",',
    '    api_ok(json!({\n        "status": "UP",'
)

# 5. actuator_info: no "ok" field
content = content.replace(
    '    Json(json!({\n        "app": {',
    '    api_ok(json!({\n        "app": {'
)

# 6. actuator_mappings: has "ok": true
content = content.replace(
    '    Json(json!({\n        "ok": true,\n        "total": ROUTES.len(),',
    '    api_ok(json!({\n        "total": ROUTES.len(),'
)

# 7. actuator_metrics: no "ok" field
content = content.replace(
    '    Json(json!({\n        "names": ["requests_total",',
    '    api_ok(json!({\n        "names": ["requests_total",'
)

# 8. actuator_env: no "ok" field
content = content.replace(
    '    Json(json!({\n        "config": {',
    '    api_ok(json!({\n        "config": {'
)

# 9. actuator_loggers: no "ok" field
content = content.replace(
    '    Json(json!({\n        "levels": ["TRACE",',
    '    api_ok(json!({\n        "levels": ["TRACE",'
)

# 10. actuator_loggers_set: returns -> Response, special handling
# Change return type
content = content.replace(
    ') -> Response {\n    let level = body.level.to_ascii_uppercase();',
    ') -> ApiResponse<Value> {\n    let level = body.level.to_ascii_uppercase();'
)
# Error branch: (StatusCode::BAD_REQUEST, Json(json!({...}))).into_response()
old_err = '''        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": "level 必须为 TRACE/DEBUG/INFO/WARN/ERROR",
                "got": level,
            })),
        )
            .into_response();'''
new_err = '''        return api_error(400, format!("level 必须为 TRACE/DEBUG/INFO/WARN/ERROR，got: {level}"));'''
content = content.replace(old_err, new_err)
# Success branch: Json(json!({...})).into_response()
old_ok = '''    Json(json!({
        "ok": true,
        "configured_level": state.logs.min_level(),
    }))
    .into_response()'''
new_ok = '''    api_ok(json!({
        "configured_level": state.logs.min_level(),
    }))'''
content = content.replace(old_ok, new_ok)

# 11. actuator_logs: has "ok": true
content = content.replace(
    '    Json(json!({\n        "ok": true,\n        "total": total,',
    '    api_ok(json!({\n        "total": total,'
)

# 12. actuator_logs_clear: has "ok": true, single line
content = content.replace(
    '    Json(json!({ "ok": true, "cleared": cleared }))',
    '    api_ok(json!({ "cleared": cleared }))'
)

# 13. actuator_api_get: match with success and error
# Success branch
content = content.replace(
    '        Some(route) => Json(json!({\n            "ok": true,\n            "id": route.id,',
    '        Some(route) => api_ok(json!({\n            "id": route.id,'
)
# Error branch
old_err2 = '''        None => Json(json!({
            "ok": false,
            "error": format!("未找到 API: {id}"),
            "hint": "可枚举 /actuator/mappings 获取 id",
        })),'''
new_err2 = '''        None => api_error(404, format!("未找到 API: {id}，可枚举 /actuator/mappings 获取 id")),'''
content = content.replace(old_err2, new_err2)

# 14. set_api_enabled helper: change return type (already done by regex)
# Management error
old_err3 = '''                return Json(json!({
                    "ok": false,
                    "error": format!("管理面端点 `{id}` 不允许停用（防自锁）"),
                }));'''
new_err3 = '''                return api_error(403, format!("管理面端点 `{id}` 不允许停用（防自锁"));'''
content = content.replace(old_err3, new_err3)
# Success
content = content.replace(
    '            Json(json!({\n                "ok": true,\n                "id": route.id,',
    '            api_ok(json!({\n                "id": route.id,'
)
# Not found error
old_err4 = '''        None => Json(json!({
            "ok": false,
            "error": format!("未找到 API: {id}"),
        })),'''
new_err4 = '''        None => api_error(404, format!("未找到 API: {id}")),'''
content = content.replace(old_err4, new_err4)

if content == original:
    print("ERROR: No changes made!")
else:
    with open(path, 'w', encoding='utf-8', newline='') as f:
        f.write(content)
    # Verify
    remaining = len(re.findall(r'Json\s*\(\s*json!', content))
    api_ok_count = content.count('api_ok(')
    api_err_count = content.count('api_error(')
    api_response_count = len(re.findall(r'->\s*ApiResponse<', content))
    print(f"actuator.rs migration complete")
    print(f"  Remaining Json(json!): {remaining}")
    print(f"  api_ok calls: {api_ok_count}")
    print(f"  api_error calls: {api_err_count}")
    print(f"  ApiResponse return types: {api_response_count}")
    print(f"  Import added: {'mox_api_protocol' in content}")
    # Check SSE handler is untouched
    print(f"  SSE handler intact: {'actuator_logs_tail' in content and 'Sse::new' in content}")
