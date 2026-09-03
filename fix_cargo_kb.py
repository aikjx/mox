path = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kb-svc\Cargo.toml'
with open(path, 'r', encoding='utf-8-sig', newline='') as f:
    lines = f.readlines()

# Insert mox-api-protocol after the mox-ai-expert-proto line
new_lines = []
for line in lines:
    new_lines.append(line)
    if line.strip().startswith('mox-ai-expert-proto ='):
        new_lines.append('mox-api-protocol = { workspace = true }\n')

with open(path, 'w', encoding='utf-8-sig', newline='') as f:
    f.writelines(new_lines)

print('Cargo.toml updated')
