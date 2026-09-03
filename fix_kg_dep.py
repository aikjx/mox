#!/usr/bin/env python3
path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-service-svc\Cargo.toml'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

old = 'serde_qs = { version = "0.12", optional = true }\n\n[features]'
new = 'serde_qs = { version = "0.12", optional = true }\nmox-api-protocol = { workspace = true, optional = true }\n\n[features]'

if old in content:
    content = content.replace(old, new)
    with open(path, 'w', encoding='utf-8', newline='') as f:
        f.write(content)
    print('Fixed: added mox-api-protocol optional dependency')
else:
    print('Pattern not found, checking content...')
    idx = content.find('serde_qs')
    print(repr(content[idx:idx+100]))
