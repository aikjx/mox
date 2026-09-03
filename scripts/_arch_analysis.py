import re, os, subprocess, sys

root = r'D:\a10\aikjx\gitcode\infotopograph'
os.chdir(root)

t = open('Cargo.toml', encoding='utf-8', errors='ignore').read()
m = re.search(r'\[workspace\][\s\S]*?members\s*=\s*\[([\s\S]*?)\]', t)
members = []
if m:
    members = re.findall(r'"([^"]+)"', m.group(1))

print(f'=== workspace members: {len(members)} ===')
for i, mem in enumerate(members, 1):
    print(f'  {i:3d}. {mem}')

# 检查每个 member 的 Cargo.toml 是否存在，提取 package name
print('\n=== crate 状态检查 ===')
existing = []
missing = []
for mem in members:
    cargo_path = os.path.join(mem, 'Cargo.toml')
    if os.path.exists(cargo_path):
        ct = open(cargo_path, encoding='utf-8', errors='ignore').read()
        nm = re.search(r'^name\s*=\s*"([^"]+)"', ct, re.M)
        name = nm.group(1) if nm else '???'
        # 检查是否有 [[bin]]
        has_bin = '[[bin]]' in ct
        existing.append((mem, name, has_bin))
    else:
        missing.append(mem)

print(f'存在: {len(existing)}, 缺失: {len(missing)}')
if missing:
    print('缺失路径:')
    for m in missing:
        print(f'  - {m}')

print('\n=== 有 bin 的 crate（可启动服务）===')
for mem, name, has_bin in existing:
    if has_bin:
        print(f'  {name}  ({mem})')

# 保存成员列表供后续使用
with open(r'scripts\_workspace_members.txt', 'w', encoding='utf-8') as f:
    for mem, name, has_bin in existing:
        f.write(f'{name}\t{mem}\t{has_bin}\n')
print('\n成员列表已保存到 scripts/_workspace_members.txt')
