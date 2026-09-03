import re

path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kb-svc\src\handlers.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

original = content

# 1. Add import after 'use std::sync::Arc;'
content = content.replace(
    'use std::sync::Arc;\n',
    'use std::sync::Arc;\nuse mox_api_protocol::{ApiResponse, api_ok, api_error};\n'
)

# 2. Remove unused response imports (IntoResponse, Response no longer needed)
content = content.replace('    response::{IntoResponse, Response},\n', '')

# 3. Replace ok() helper
old_ok = '''/// 成功响应（legacy 同款信封）
fn ok<T: serde::Serialize>(data: T) -> Response {
    Json(json!({ "success": true, "data": data })).into_response()
}'''
new_ok = '''/// 成功响应（统一 ApiResponse 信封）
fn ok<T: serde::Serialize>(data: T) -> ApiResponse<Value> {
    api_ok(serde_json::to_value(data).unwrap_or(Value::Null))
}'''
assert old_ok in content, "ok() helper not found!"
content = content.replace(old_ok, new_ok)

# 4. Replace err() helper
old_err = '''/// 错误响应
fn err(status: StatusCode, code: &str, message: &str) -> Response {
    let mut resp = Json(json!({ "success": false, "code": code, "error": message })).into_response();
    *resp.status_mut() = status;
    resp
}'''
new_err = '''/// 错误响应（统一 ApiResponse 信封，code 取 HTTP 状态码）
fn err(status: StatusCode, _code: &str, message: &str) -> ApiResponse<Value> {
    api_error(status.as_u16() as i32, message)
}'''
assert old_err in content, "err() helper not found!"
content = content.replace(old_err, new_err)

# 5. Replace not_found() helper
old_nf = '''/// 文档不存在统一错误
fn not_found(id: &str) -> Response {
    err(StatusCode::NOT_FOUND, "not_found", &format!("文档不存在: {id}"))
}'''
new_nf = '''/// 文档不存在统一错误
fn not_found(id: &str) -> ApiResponse<Value> {
    err(StatusCode::NOT_FOUND, "not_found", &format!("文档不存在: {id}"))
}'''
assert old_nf in content, "not_found() helper not found!"
content = content.replace(old_nf, new_nf)

# 6. Change all handler return types from '-> Response {' to '-> ApiResponse<Value> {'
# But only for async fn handlers (not the helper functions which we already changed)
# The helpers don't have 'async fn' prefix
content = re.sub(r'(async fn \w+\([^)]*\)) -> Response \{', r'\1 -> ApiResponse<Value> {', content)

# Count handlers changed
handler_count = len(re.findall(r'async fn \w+\([^)]*\) -> ApiResponse<Value>', content))
print(f'  Handlers migrated: {handler_count}')

assert content != original, 'No changes made!'

with open(path, 'w', encoding='utf-8-sig', newline='') as f:
    f.write(content)

print('handlers.rs migrated successfully')
print(f'  ApiResponse occurrences: {content.count("ApiResponse")}')
print(f'  api_ok occurrences: {content.count("api_ok(")}')
print(f'  api_error occurrences: {content.count("api_error(")}')
print(f'  Remaining -> Response: {content.count("-> Response")}')
