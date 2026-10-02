import re

path = r'D:\a10\aikjx\gitcode\infotopograph\Cargo.toml'

with open(path, 'r', encoding='utf-8') as f:
    lines = f.readlines()

result = []
for line in lines:
    result.append(line)
    if 'mox-alliance-scheduler-svc' in line and 'svc' in line:
        result.append('    "platform/domains/alliance/svc/mox-alliance-registry-svc",\n')

with open(path, 'w', encoding='utf-8') as f:
    f.writelines(result)

print('done')
