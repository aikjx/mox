#!/usr/bin/env python3
path = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\actuator.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    c = f.read()

old = '''            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "ok": false,
                    "code": "API_DISABLED",
                    "id": route.id,
                    "path": path,
                    "message": format!("API `{}` 已被管理端停用，请在 /actuator/api/{} 恢复", route.id, route.id),
                })),
            )
                .into_response();'''

new = '''            return api_error(403, format!("API `{}` 已被管理端停用，请在 /actuator/api/{} 恢复", route.id, route.id)).into_response();'''

if old in c:
    c = c.replace(old, new)
    with open(path, 'w', encoding='utf-8', newline='') as f:
        f.write(c)
    print('Fixed middleware error response')
else:
    print('Pattern not found, trying to locate...')
    idx = c.find('API_DISABLED')
    if idx >= 0:
        print(repr(c[idx-50:idx+200]))
