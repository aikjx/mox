import os, re, hashlib, json

FE = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui'
os.chdir(FE)
CN = re.compile(r'[\u4e00-\u9fff]')
MAP = re.compile(r'(?:const|let)\s+([A-Za-z_][\w]*)\s*=\s*\{([^{}]{20,900})\}', re.S)
PAIR = re.compile(r"([A-Za-z_][\w]*)\s*:\s*(?:'([^']*)'|\"([^\"]*)\"|\[([^\]]*)\])")

entries = []
for root, dirs, files in os.walk('src'):
    for f in files:
        if not f.endswith(('.vue', '.js', '.ts')) or '.test.' in f or f == '_smoke.js':
            continue
        p = os.path.join(root, f).replace(os.sep, '/')
        if p.startswith('src/utils/') or p.startswith('src/constants/') or p.startswith('src/modules/_kernel/'):
            continue
        txt = open(p, encoding='utf-8', errors='replace').read()
        for m in MAP.finditer(txt):
            name, body = m.group(1), m.group(2)
            if not CN.search(body):
                continue
            kv = []
            for pm in PAIR.finditer(body):
                k = pm.group(1)
                v = next((g for g in pm.groups()[1:] if g is not None), '')
                kv.append((k, ' '.join(str(v).split())))
            if len(kv) < 2:
                continue
            ln = txt[:m.start()].count('\n') + 1
            canon = '|'.join('%s=%s' % t for t in sorted(kv))
            entries.append({'file': p, 'line': ln, 'name': name, 'n': len(kv),
                            'keys': sorted(k for k, _ in kv),
                            'vals': sorted(v for _, v in kv),
                            'sha': hashlib.sha256(canon.encode()).hexdigest()[:12],
                            'canon': canon})

print('自写码->文案映射（含中文，键>=2）: %d 条' % len(entries))

# (a) 逐字符相同 = 真副本
grp = {}
for e in entries:
    grp.setdefault(e['sha'], []).append(e)
dup = [v for v in grp.values() if len(v) > 1]
print('\n== (a) 内容逐条相同的真副本组: %d ==' % len(dup))
for v in sorted(dup, key=lambda v: -len(v)):
    print('  x%d  sha=%s  %s' % (len(v), v[0]['sha'], v[0]['canon'][:120]))
    for e in v:
        print('      %s:%d %s' % (e['file'], e['line'], e['name']))

# (b) 键集相同但文案不同 = 口径分裂（不可机械合并）
keygrp = {}
for e in entries:
    keygrp.setdefault(tuple(e['keys']), []).append(e)
split = [v for v in keygrp.values()
         if len(v) > 1 and len(set(x['sha'] for x in v)) > 1]
print('\n== (b) 同键集不同文案（口径分裂，需裁决）: %d ==' % len(split))
for v in split:
    print('  keys=%s' % ','.join(v[0]['keys']))
    for e in v:
        print('      %s:%d %s -> %s' % (e['file'], e['line'], e['name'], e['canon'][:110]))

# (c) 键集部分重叠 >=50%
print('\n== (c) 键集重叠>=2且>=50%% 的对（跨文件）==')
pairs = 0
for i in range(len(entries)):
    for j in range(i + 1, len(entries)):
        a, b = entries[i], entries[j]
        if a['file'] == b['file']:
            continue
        inter = set(a['keys']) & set(b['keys'])
        if len(inter) >= 2 and len(inter) >= 0.5 * min(len(a['keys']), len(b['keys'])):
            pairs += 1
            print('  重叠%d/%d,%d  %s:%d %s | %s:%d %s  同文案=%s' % (
                len(inter), len(a['keys']), len(b['keys']),
                a['file'], a['line'], a['name'], b['file'], b['line'], b['name'],
                a['sha'] == b['sha']))
print('pairs=%d' % pairs)
json.dump(entries, open(r'D:\tmp\vocab-entries.json', 'w', encoding='utf-8'), ensure_ascii=False)
