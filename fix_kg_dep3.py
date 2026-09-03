#!/usr/bin/env python3
path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-service-svc\Cargo.toml'
with open(path, 'rb') as f:
    raw = f.read()

# Decode, preserving BOM awareness
text = raw.decode('utf-8-sig')

# Insert after serde_qs line
marker = 'serde_qs = { version = "0.12", optional = true }'
insertion = '\r\nmox-api-protocol = { workspace = true, optional = true }'

if marker in text:
    text = text.replace(marker, marker + insertion, 1)
    with open(path, 'wb') as f:
        f.write(text.encode('utf-8'))
    print('Fixed: added mox-api-protocol optional dependency')
else:
    print('Marker not found')

# Verify
with open(path, 'r', encoding='utf-8-sig') as f:
    c = f.read()
print('Verify:', 'mox-api-protocol = { workspace = true, optional = true }' in c)
