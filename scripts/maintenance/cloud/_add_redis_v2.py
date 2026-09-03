fpath = r'D:\a10\aikjx\gitcode\infotopograph\Cargo.toml'
with open(fpath, 'rb') as f:
    data = f.read()
text = data.decode('utf-8', errors='replace')
lines = text.split('\n')

# Find the base64 line after reqwest
insert_idx = None
for i, line in enumerate(lines):
    if line.strip().startswith('base64 =') and i > 200:
        insert_idx = i + 1
        break

if insert_idx:
    print(f'在第 {insert_idx+1} 行后插入')
    lines.insert(insert_idx, '')
    lines.insert(insert_idx+1, '# Redis client (cloud-filer-svc real Redis backend)')
    lines.insert(insert_idx+2, 'redis = { version = "0.26", features = ["tokio-comp"] }')
    with open(fpath, 'w', encoding='utf-8', newline='') as f:
        f.write('\n'.join(lines))
    print('已添加 redis 依赖')
else:
    print('未找到 base64 行')
