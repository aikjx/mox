#!/usr/bin/env python3
"""Fix broken proxy entries in actuator.rs ROUTES."""
path = r'platform/gateway/mox-platform-gateway-svc/src/actuator.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    lines = f.readlines()

# Fix line 446 (index 445) - proxy_orchestrator
lines[445] = '    r("platform.proxy_orchestrator", "ANY", "/api/{*path}", "L6", "platform", "ready", "业务域反向代理→编排器（默认 :3001，catch-all）"),\n'
# Fix line 447 (index 446) - proxy_primiflow
lines[446] = '    r("platform.proxy_primiflow", "ANY", "/api/projects/{*path}", "L6", "platform", "ready", "项目域反向代理→PrimiFlow（默认 :8000）"),\n'

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.writelines(lines)

print('Fixed 2 broken proxy entries')
print(f'Line 446: {lines[445].strip()[:120]}')
print(f'Line 447: {lines[446].strip()[:120]}')
