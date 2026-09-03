import re, os

root = r'D:\a10\aikjx\gitcode\infotopograph'
os.chdir(root)

t = open('Cargo.toml', encoding='utf-8', errors='ignore').read()

# 找独立的 members = [（前面不是 default-）
members = []
for m in re.finditer(r'(?<!default-)members\s*=\s*\[', t):
    depth = 0
    end = m.start()
    for i in range(m.start(), len(t)):
        if t[i] == '[':
            depth += 1
        elif t[i] == ']':
            depth -= 1
            if depth == 0:
                end = i
                break
    block = t[m.start():end+1]
    found = re.findall(r'"([^"]+)"', block)
    if len(found) > 19:
        members = found
        break

print(f'=== workspace members (完整): {len(members)} ===')

# 按顶层目录分类
cats = {}
for mem in members:
    parts = mem.split('/')
    if len(parts) >= 2:
        cat = parts[0] + '/' + parts[1] if parts[0] == 'platform' else parts[0]
    else:
        cat = parts[0]
    cats.setdefault(cat, []).append(mem)

for cat, crates in sorted(cats.items()):
    print(f'\n[{cat}] ({len(crates)}):')
    for c in crates:
        print(f'  {c}')

# 完整性检查
print(f'\n=== 完整性检查 ===')
issues = []
for mem in members:
    cargo_path = os.path.join(mem, 'Cargo.toml')
    src_path = os.path.join(mem, 'src')
    if not os.path.exists(cargo_path):
        issues.append(f'MISSING Cargo.toml: {mem}')
    elif not os.path.exists(src_path):
        issues.append(f'MISSING src/: {mem}')
    else:
        has_lib = os.path.exists(os.path.join(src_path, 'lib.rs'))
        has_main = os.path.exists(os.path.join(src_path, 'main.rs'))
        if not has_lib and not has_main:
            issues.append(f'NO lib.rs/main.rs: {mem}')

if issues:
    print(f'发现 {len(issues)} 个问题:')
    for iss in issues:
        print(f'  WARN: {iss}')
else:
    print('所有 crate 结构完整')

# 保存
with open(r'scripts\_all_members.txt', 'w', encoding='utf-8') as f:
    for mem in members:
        f.write(mem + '\n')
print(f'\n共 {len(members)} 个 crate')
