fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\gateway\mox-platform-gateway-svc\src\actuator.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    c = f.read()

old = 'return api_error(403, format!("API `{}` 已被管理端停用，请在 /actuator/api/{} 恢复", route.id, route.id)).into_response();'
new = 'return ApiResponse::<Value>::error(403, format!("API `{}` 已被管理端停用，请在 /actuator/api/{} 恢复", route.id, route.id)).into_response();'

if old in c:
    c = c.replace(old, new, 1)
    print('actuator.rs: fixed type annotation')
else:
    print('actuator.rs: pattern not found, checking...')
    # Try to find the line
    for i, line in enumerate(c.split('\n')):
        if 'api_error(403' in line and '已被管理端停用' in line:
            print(f'Found at line {i+1}: {line.strip()[:80]}')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(c)
print('Done')
