path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-service-svc\Cargo.toml'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    lines = f.readlines()

# Find the mox-framework line and insert mox-api-protocol after it
new_lines = []
for line in lines:
    new_lines.append(line)
    if line.strip().startswith('mox-framework ='):
        new_lines.append('mox-api-protocol = { workspace = true, optional = true }\n')

with open(path, 'w', encoding='utf-8-sig', newline='') as f:
    f.writelines(new_lines)

print('Done. Verifying...')
with open(path, 'r', encoding='utf-8-sig') as f:
    for i, line in enumerate(f, 1):
        if 'mox-api-protocol' in line:
            print(f'  line {i}: {line.rstrip()}')
