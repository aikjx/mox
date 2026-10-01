import os, re

FE = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui'
os.chdir(FE)

# 1) 权威侧：barrel / constants 里已有的"词汇表"出口
auth = {}
for p in ['src/utils/index.js', 'src/utils/knowledgeBase.utils.js', 'src/constants/index.js',
          'src/constants/nav.config.js', 'src/modules/_kernel/nav-icons.js']:
    if not os.path.exists(p):
        print('MISSING authority file', p); continue
    t = open(p, encoding='utf-8', errors='replace').read()
    names = re.findall(r'export\s+(?:function|const)\s+(\w+)', t)
    names += re.findall(r'export\s*\{([^}]*)\}', t)[0].replace('\n', ' ').split(', ') if re.findall(r'export\s*\{([^}]*)\}', t) else []
    auth[p] = sorted(set(n.strip() for n in names if n.strip()))
for k, v in auth.items():
    print('%-42s %d exports' % (k, len(v)))
VOCAB = [n for n in sum(auth.values(), []) if re.search(r'(Label|STATUS|CATEGOR|Map|VOCAB|Colors?|Options)$|^(get|status|category)', n)]
print('vocabulary-ish authority names:', VOCAB)

# 2) 消费侧：视图/模块里自写的"码 -> 中文文案"私有映射
MAP = re.compile(r'(?:const|let)\s+([A-Za-z_][\w]*)\s*=\s*\{([^{}]{20,600})\}', re.S)
CN = re.compile(r'[\u4e00-\u9fff]')
rows = []
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
            keys = re.findall(r"([A-Za-z_][\w]*)\s*:", body)
            if len(keys) < 2:
                continue
            ln = txt[:m.start()].count('\n') + 1
            rows.append((name, len(keys), '%s:%d' % (p, ln)))

by_name = {}
for name, k, where in rows:
    by_name.setdefault(name, []).append(where)
print('\n== 自写"码->中文"映射（按名字聚合，键数=条目数）==')
for name, whs in sorted(by_name.items(), key=lambda kv: -len(kv[1])):
    print('%-26s x%d  %s' % (name, len(whs), '; '.join(whs)))
print('distinct names=%d sites=%d' % (len(by_name), sum(len(v) for v in by_name.values())))
