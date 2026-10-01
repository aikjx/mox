# -*- coding: utf-8 -*-
p = r'docs\enterprise\38-企业级管理系统架构与业务处理流程文档-V2.1.md'
s = open(p, encoding='utf-8').read()
pairs = [
    ('共 **13 个业务域、143 个 Crate**', '共 **13 个业务域、149 个 Crate**'),
    ('`platform/domains/`（13 域 143 crate）', '`platform/domains/`（13 域 149 crate）'),
    ('模块清单更新为 13 域 143 crate 归一化结构', '模块清单更新为 13 域 149 crate 归一化结构'),
]
for a, b in pairs:
    n = s.count(a)
    assert n == 1, f'count={n} for {a[:40]}'
    s = s.replace(a, b)
open(p, 'w', encoding='utf-8', newline='').write(s)
print('38 doc ok')
