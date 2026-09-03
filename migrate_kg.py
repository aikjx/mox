import re

path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-service-svc\src\http_adapter.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

original = content

# 1. Add import after 'use std::sync::Arc;'
content = content.replace(
    'use std::sync::Arc;\n',
    'use std::sync::Arc;\nuse mox_api_protocol::{ApiResponse, api_ok, api_error};\n'
)

# 2. Return types: Json<Value> -> ApiResponse<Value>
content = content.replace(') -> Json<Value> {', ') -> ApiResponse<Value> {')

# 3. Handler error case in ai_analyze
old_error = '''        return Json(json!({
            "ok": false,
            "elapsed_ms": now_ms() - t0,
            "error": "entity not found",
            "query": {"entity_id": req.entity_id, "depth": req.depth},
        }));'''
new_error = '        return api_error(404, "entity not found");'
assert old_error in content, "Handler error block not found!"
content = content.replace(old_error, new_error)

# 4. Bulk: Json(json!({ -> api_ok(json!({
content = content.replace('Json(json!({', 'api_ok(json!({')

# 5. Remove '"ok": true,' lines (8-space indent inside json! macros)
content = content.replace('        "ok": true,\n', '')

# 6. ERROR TEST: replace before Json(resp) bulk replacement
old_err_test = '''        let Json(resp) = ai_analyze(State(state.clone()), Json(req)).await;

        assert_eq!(resp["ok"], json!(false));
        assert_eq!(resp["error"], json!("entity not found"));'''
new_err_test = '''        let resp = ai_analyze(State(state.clone()), Json(req)).await;
        assert_ne!(resp.code, 0);
        assert_eq!(resp.message, "entity not found");'''
assert old_err_test in content, "Error test block not found!"
content = content.replace(old_err_test, new_err_test)

# 7. SUCCESS TESTS: 'let Json(resp) = ' -> 'let ApiResponse { data, .. } = '
content = content.replace('let Json(resp) = ', 'let ApiResponse { data, .. } = ')
# After each ApiResponse destructure statement ending with .await;, add unwrap
content = re.sub(
    r'(let ApiResponse \{ data, \.\. \} = .+?\.await;)',
    r'\1\n        let resp = data.unwrap();',
    content
)

# 8. Handler Json(resp) -> api_ok(resp) (tests no longer have Json(resp))
content = content.replace('Json(resp)', 'api_ok(resp)')

# 9. Remove assert_eq!(resp["ok"], json!(true)); lines (8-space indent)
content = content.replace('        assert_eq!(resp["ok"], json!(true));\n', '')

assert content != original, 'No changes made!'

with open(path, 'w', encoding='utf-8-sig', newline='') as f:
    f.write(content)

print('http_adapter.rs migrated successfully')
print(f'  ApiResponse occurrences: {content.count("ApiResponse")}')
print(f'  api_ok occurrences: {content.count("api_ok(")}')
print(f'  api_error occurrences: {content.count("api_error(")}')
print(f'  Remaining Json<Value>: {content.count("Json<Value>")}')
print(f'  Remaining Json(json!: {content.count("Json(json!")}')
ok_count = content.count('resp["ok"]')
print(f'  Remaining resp["ok"]: {ok_count}')
