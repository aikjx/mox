#!/usr/bin/env python3
path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-service-svc\Cargo.toml'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

# Handle both CRLF and LF
import re
old = 'serde_qs = { version = "0.12", optional = true }'
new = 'serde_qs = { version = "0.12", optional = true }\nmox-api-protocol = { workspace = true, optional = true }'

# Find the line and insert after it
lines = content.split('\n')
new_lines = []
for line in lines:
    new_lines.append(line)
    if line.strip().startswith('serde_qs') and 'optional = true' in line:
        # Determine line ending style
        newline = '\r\n' if line.endswith('\r') else '\n'
        new_lines.append('mox-api-protocol = { workspace = true, optional = true }' + ('' if newline == '\n' else ''))

content = '\n'.join(new_lines)
with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

# Verify
with open(path, 'r', encoding='utf-8-sig') as f:
    c = f.read()
print('mox-api-protocol in deps:', 'mox-api-protocol = { workspace = true, optional = true }' in c)
