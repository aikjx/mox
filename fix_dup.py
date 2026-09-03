path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kg-service-svc\Cargo.toml'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    lines = f.readlines()

# Remove duplicate mox-api-protocol lines - keep only the first one
seen = False
new_lines = []
for line in lines:
    if line.strip().startswith('mox-api-protocol ='):
        if not seen:
            new_lines.append(line)
            seen = True
    else:
        new_lines.append(line)

with open(path, 'w', encoding='utf-8-sig', newline='') as f:
    f.writelines(new_lines)

print('Duplicate removed')
with open(path, 'r', encoding='utf-8-sig') as f:
    for i, line in enumerate(f, 1):
        if 'mox-api-protocol' in line:
            print(f'  line {i}: {line.rstrip()}')
