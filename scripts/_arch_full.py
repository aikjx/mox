import re, os

root = r'D:\a10\aikjx\gitcode\infotopograph'
os.chdir(root)

t = open('Cargo.toml', encoding='utf-8', errors='ignore').read()

# 提取 members（不是 default-members）
# 找到 "members = [" 开始，到匹配的 "]" 结束
idx = t.find('members = [')
if idx == -1:
    print('未找到 members')
    exit()

# 从 idx 开始找匹配的 ]
depth = 0
end = idx
for i in range(idx, len(t)):
    if t[i] == '[':
        depth += 1
    elif t[i] == ']':
        depth -= 1
        if depth == 0:
            end = i
            break

members_block = t[idx:end+1]
members = re.findall(r'"([^"]+)"', members_block)
print(f'=== workspace members (完整): {len(members)} ===')
for i, mem in enumerate(members, 1):
    print(f'  {i:3d}. {mem}')

# 按域分类
domains = {}
for mem in members:
    parts = mem.split('/')
    if len(parts) >= 3 and parts[0] == 'platform':
        domain = parts[2] if parts[1] == 'domains' else parts[1]
        domains.setdefault(domain, []).append(mem)
    else:
        domains.setdefault('other', []).append(mem)

print(f'\n=== 按域分类 ===')
for domain, crates in sorted(domains.items()):
    print(f'\n[{domain}] ({len(crates)}):')
    for c in crates:
        print(f'  {c}')

# 检查每个 crate 的 Cargo.toml 和 src
print(f'\n=== crate 完整性检查 ===')
issues = []
for mem in members:
    cargo_path = os.path.join(mem, 'Cargo.toml')
    src_path = os.path.join(mem, 'src')
    if not os.path.exists(cargo_path):
        issues.append(f'MISSING Cargo.toml: {mem}')
    elif not os.path.exists(src_path):
        issues.append(f'MISSING src/: {mem}')
    else:
        # 检查 src/lib.rs 或 src/main.rs
        has_lib = os.path.exists(os.path.join(src_path, 'lib.rs'))
        has_main = os.path.exists(os.path.join(src_path, 'main.rs'))
        if not has_lib and not has_main:
            issues.append(f'NO lib.rs/main.rs: {mem}')

if issues:
    print(f'发现 {len(issues)} 个问题:')
    for iss in issues:
        print(f'  ⚠️  {iss}')
else:
    print('所有 crate 结构完整')

# 保存
with open(r'scripts\_all_members.txt', 'w', encoding='utf-8') as f:
    for mem in members:
        f.write(mem + '\n')
print(f'\n共 {len(members)} 个 crate，已保存到 scripts/_all_members.txt')
