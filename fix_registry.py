path = r'D:\a10\aikjx\gitcode\infotopograph\Cargo.toml'

with open(path, 'r', encoding='utf-8') as f:
    lines = f.readlines()

# 找到 workspace.dependencies 部分，删除错误插入的那一行
result = []
skip_next = False
for i, line in enumerate(lines):
    # 如果是错误插入的 registry-svc 行（在 dependencies 区），跳过
    if 'mox-alliance-registry-svc' in line and i > 300 and i < 320:
        continue
    result.append(line)

with open(path, 'w', encoding='utf-8') as f:
    f.writelines(result)

print('done')
