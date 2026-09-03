#!/usr/bin/env python3
"""Fix actuator.rs ROUTES: convert RouteInfo{} to ApiRoute via r() constructor."""
import re

path = r'platform/gateway/mox-platform-gateway-svc/src/actuator.rs'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    content = f.read()

# Find the ROUTES section
routes_start = content.find('pub static ROUTES:')
routes_end = content.find('];', routes_start) + 2
routes_section = content[routes_start:routes_end]

def parse_entry(entry_text):
    fields = {}
    pattern = r'(\w+):\s*((?:"[^"]*")|(?:true)|(?:false))'
    for m in re.finditer(pattern, entry_text):
        fields[m.group(1)] = m.group(2)
    return fields

def convert_entry(entry_text):
    fields = parse_entry(entry_text)
    rid = fields.get('id', '""')
    method = fields.get('methods', '"GET"')
    rpath = fields.get('path', '""')
    layer = fields.get('layer', '"L0"')
    domain = fields.get('domain', '""')
    status = '"ready"'
    desc = fields.get('desc', '""')
    return f'r({rid}, {method}, {rpath}, {layer}, {domain}, {status}, {desc})'

# Replace all entries
new_routes = routes_section
entries = list(re.finditer(r'RouteInfo\s*\{[^}]+\}', routes_section))
print(f'Converting {len(entries)} entries...')

for m in reversed(entries):
    old = m.group(0)
    new = convert_entry(old)
    new_routes = new_routes[:m.start()] + new + new_routes[m.end():]

# Fix the type declaration
new_routes = new_routes.replace('&[RouteInfo; 98]', '&[ApiRoute; 98]')

content = content[:routes_start] + new_routes + content[routes_end:]

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(content)

print('Done! ROUTES converted to ApiRoute with r() constructor')
idx = new_routes.find('r(')
print(f'First entry: {new_routes[idx:idx+150]}')
