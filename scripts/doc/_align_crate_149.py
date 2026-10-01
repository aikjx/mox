# -*- coding: utf-8 -*-
p = r'docs\architecture\NORMALIZED_ARCHITECTURE.md'
s = open(p, encoding='utf-8').read()
a = '| `48/60+/73 crate` → `143` | 13 处 | 改 143 |'
b = '| `48/60+/73 crate` → `149` | 13 处 | 改 149（2026-09-24 按 cargo metadata 复点；v2.0 原记 143） |'
if b in s:
    print('already done')
elif s.count(a) == 1:
    open(p, 'w', encoding='utf-8', newline='').write(s.replace(a, b))
    print('footer ok')
else:
    raise SystemExit(f'a count={s.count(a)}')
