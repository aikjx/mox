fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\domains\kg\svc\mox-kb-svc\src\handlers.rs'
with open(fpath, encoding='utf-8-sig', errors='ignore') as f:
    content = f.read()

bad = "}' + @'"
n = content.count(bad)
print(f'损坏字符串出现 {n} 次')
content = content.replace(bad, '}')

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(content)
print('已修复保存')

with open(fpath, encoding='utf-8', errors='ignore') as f:
    lines = f.readlines()
print(f'363行: {repr(lines[362])}')
print(f'总行数: {len(lines)}')
