# -*- coding: utf-8 -*-
import io

def rep(path, pairs, must=True):
    s = open(path, encoding='utf-8').read()
    for a, b in pairs:
        n = s.count(a)
        if n != 1:
            if must:
                raise SystemExit(f'{path}: count={n} for {a[:50]}')
            continue
        s = s.replace(a, b)
    open(path, 'w', encoding='utf-8', newline='').write(s)
    print('OK', path)

rep('AGENTS.md', [
    ('- **workspace 143 crates**', '- **workspace 149 crates**'),
    ('根 `Cargo.toml`（143 members）为准', '根 `Cargo.toml`（149 members）为准'),
])
rep('README.md', [
    ('后端主体（Rust workspace，143 crates）', '后端主体（Rust workspace，149 crates）'),
])
rep(r'docs\CORE-CAPABILITIES.md', [
    ('Rust workspace **143 crates**', 'Rust workspace **149 crates**'),
])
rep(r'docs\architecture\README.md', [
    ('**归一化唯一权威**（v2.0，2026-09-16）：143 crate / 12 域 / 六层 / 四进程 / 网关 :3080',
     '**归一化唯一权威**（v2.1，2026-09-24）：149 crate / 12 域 / 六层 / 四进程 / 网关 :3080'),
])
