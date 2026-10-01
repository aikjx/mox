#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-frontend-module.py —— 前端模块化治理门禁（FE-MOD-GOV-V1.0 执行门禁）

权威规范：docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md

检查项（frontend-ui/src 下）：
  E1. element-plus 根导入 = 0（`from 'element-plus'` 会使 tree-shake 失效、全量组件进包）
  E2. 跨目录相对导入 = 0（`from '../` / `import '../`；同目录 `./` 允许）
  E3. barrel 导出名冲突 = 0（api/stores/constants/utils/composables/directives 的 export * 链）
  E4. 命名规范：api 下文件必须 `*.api.js`、stores 下必须 `*.store.js`（index.js 除外）
  E5. vite.config.js 必须使用 epSubpathResolver（禁止改回官方 ElementPlusResolver）
  W1. 块注释配对（js/vue 粗检；字符串内误报仅作提示，不阻断）

用法：
  python scripts/gate/check-frontend-module.py            # 全量校验
  python scripts/gate/check-frontend-module.py --repo <路径>   # 指定仓库根

退出码：0 = 无 ERROR；1 = 存在 ERROR（CI 门禁）。
"""
from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SRC = REPO / 'frontend-ui' / 'src'
VITE = REPO / 'frontend-ui' / 'vite.config.js'

BARREL_MODULES = ['api', 'stores', 'constants', 'utils', 'composables', 'directives']

if hasattr(sys.stdout, 'reconfigure'):
    # Windows 控制台默认 GBK：本门禁打印的 '✓' 会让 print() 直接抛 UnicodeEncodeError，
    # 于是 AGENTS.md 里那条文档命令在这台机器上必崩（2026-09-27 实测），门禁形同不存在。
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')

def walk(d: Path):
    for p in d.rglob('*'):
        if p.is_file() and p.suffix in ('.js', '.vue') and 'node_modules' not in p.parts:
            yield p

def is_test_code(p: Path) -> bool:
    """测试文件与 storybook 演示代码不受业务导入规则约束（不参与打包）。"""
    return '.test.' in p.name or '.stories.' in p.name or 'stories' in p.parts

def err(msg):
    print('[ERROR] ' + msg)

def warn(msg):
    print('[WARN ] ' + msg)

def info(msg):
    print('[INFO ] ' + msg)

def strip_comments(c: str) -> str:
    """剥离块注释与行注释（字符串中的 // 不影响 @/ 与 element-plus 判定）。"""
    c = re.sub(r'/\*.*?\*/', '', c, flags=re.S)
    c = re.sub(r'//[^\n]*', '', c)
    return c

# ---------------- E1 / E2 ----------------
def check_imports():
    bad = []
    for p in walk(SRC):
        c = strip_comments(p.read_text(encoding='utf-8', errors='replace'))
        lines = c.splitlines()
        for i, line in enumerate(lines, 1):
            if re.search(r"from\s*['\"]element-plus['\"]", line) or re.search(r"import\s+['\"]element-plus['\"]", line):
                bad.append(f'{p.relative_to(REPO)}:{i}: {line.strip()[:80]}')
    if bad:
        err(f'element-plus 根导入 {len(bad)} 处（必须改为子路径，见 FE-MOD-GOV §2.2）：')
        for b in bad[:10]: err('  ' + b)
    else:
        info('E1 element-plus 根导入：0 处 ✓')
    return bool(bad)

def check_relative():
    bad = []
    for p in walk(SRC):
        if is_test_code(p):
            continue
        c = strip_comments(p.read_text(encoding='utf-8', errors='replace'))
        for m in re.finditer(r"(?:from\s*|import\s*\()\s*['\"](\.\./[^'\"]+)['\"]", c):
            bad.append(f'{p.relative_to(REPO)}: -> {m.group(1)}')
    if bad:
        err(f'跨目录相对导入 {len(bad)} 处（必须用 @/ 别名，见 FE-MOD-GOV §2.1）：')
        for b in bad[:10]: err('  ' + b)
    else:
        info('E2 跨目录相对导入：0 处 ✓')
    return bool(bad)

# ---------------- E3 ----------------
def barrel_names(module: str):
    idx = SRC / module / 'index.js'
    if not idx.exists():
        return set()
    c = idx.read_text(encoding='utf-8', errors='replace')
    names = set()
    for m in re.finditer(r"export\s*\{([^}]+)\}\s*from\s*'\./([^']+)'", c):
        for part in m.group(1).split(','):
            part = part.strip()
            asm = re.match(r'^([\w$]+)\s+as\s+([\w$]+)$', part)
            names.add(asm.group(2) if asm else part)
    for m in re.finditer(r"export \* from '\./([^']+)'", c):
        f = SRC / module / (m.group(1) + '.js')
        if not f.exists():
            continue
        fc = f.read_text(encoding='utf-8', errors='replace')
        for nm in re.finditer(r"export\s+(?:async\s+)?(?:function|const|let|var|class)\s+([\w$]+)", fc):
            names.add(nm.group(1))
    return names

def check_barrel_collisions():
    problems = []
    for mod in BARREL_MODULES:
        idx = SRC / mod / 'index.js'
        explicit = set()
        if idx.exists():
            ic = idx.read_text(encoding='utf-8', errors='replace')
            for m in re.finditer(r"export\s*\{([^}]+)\}\s*from\s*'\./([^']+)'", ic):
                for part in m.group(1).split(','):
                    part = part.strip()
                    asm = re.match(r'^([\w$]+)\s+as\s+([\w$]+)$', part)
                    if asm:
                        explicit.add(asm.group(1))  # 原名也被显式处理（别名遮蔽），不构成冲突
                        explicit.add(asm.group(2))
                    else:
                        explicit.add(part)
        seen, dup = {}, []
        for f in (SRC / mod).glob('*'):
            if f.suffix != '.js' or f.name in ('index.js',) or '.test.' in f.name:
                continue
            fc = f.read_text(encoding='utf-8', errors='replace')
            for nm in re.finditer(r"export\s+(?:async\s+)?(?:function|const|let|var|class)\s+([\w$]+)", fc):
                n = nm.group(1)
                if n in explicit:
                    continue  # barrel 已显式遮蔽/别名，不构成 ambiguous
                if n in seen:
                    dup.append(f'{n} ({f.name} + {seen[n]})')
                else:
                    seen[n] = f.name
        if dup:
            problems.append(f'{mod}: ' + '; '.join(dup[:5]))
    if problems:
        err(f'barrel 导出名冲突（export * 会 ambiguous binding，需显式遮蔽或别名）：')
        for p in problems: err('  ' + p)
    else:
        info('E3 barrel 导出名冲突：0 处 ✓')
    return bool(problems)

# ---------------- E6 ----------------
BARRELED_DIRS = ['components', 'api', 'stores', 'constants', 'utils', 'composables', 'directives']

def check_deep_alias():
    """已 barrel 化目录的 @/ 深路径导入 = 0（防深路径回潮，一律走 barrel 出口）。"""
    bad = []
    for p in walk(SRC):
        if is_test_code(p):
            continue
        c = strip_comments(p.read_text(encoding='utf-8', errors='replace'))
        for m in re.finditer(r"(?:from\s*|import\s*\()\s*['\"](@\/(?:%s)\/[^'\"]+)['\"]" % '|'.join(BARRELED_DIRS), c):
            target = m.group(1)
            # 允许：barrel 本身 / barreled 目录内的同目录相对（不走 @/）/ 模块内部自治不受此限
            if re.search(r'/(?:%s)/index\.js$' % '|'.join(BARRELED_DIRS), target):
                continue
            bad.append(f'{p.relative_to(REPO)}: -> {target}')
    if bad:
        err(f'已 barrel 化目录的 @/ 深路径导入 {len(bad)} 处（应走 barrel 出口，见 FE-MOD-GOV §4）：')
        for b in bad[:10]: err('  ' + b)
    else:
        info('E6 barreled 目录 @/ 深路径导入：0 处 ✓')
    return bool(bad)

# ---------------- E7 ----------------
def resolve_alias(spec: str) -> bool:
    """解析 @/ 别名目标：目录+index.js / 文件(.js/.vue)。"""
    p = SRC / spec
    if p.is_dir() and (p / 'index.js').exists():
        return True
    if p.is_file():
        return True
    for ext in ('.js', '.vue'):
        if (SRC / (spec + ext)).exists():
            return True
    return False

def check_alias_targets():
    """所有 @/ 别名导入必须能解析到真实目标（防路径断裂回归）。"""
    bad = []
    for f in walk(SRC):
        if is_test_code(f):
            continue
        c = strip_comments(f.read_text(encoding='utf-8', errors='replace'))
        for m in re.finditer(r'(?:from\s*|import\s*\()\s*["\'](@/[^"\']+)["\']', c):
            spec = m.group(1)[2:]  # 去掉 '@/'
            if not resolve_alias(spec):
                bad.append(f'{f.relative_to(REPO)}: -> @/{spec}')
    if bad:
        err(f'@/ 别名导入目标不可解析 {len(bad)} 处（路径断裂会直接构建失败）：')
        for b in bad[:10]: err('  ' + b)
    else:
        info('E7 @/ 别名导入目标：全部可解析 ✓')
    return bool(bad)

# ---------------- E4 ----------------
def check_naming():
    bad = []
    for mod, suffix in (('api', '.api.js'), ('stores', '.store.js')):
        for f in (SRC / mod).glob('*'):
            if f.suffix != '.js' or f.name == 'index.js' or '.test.' in f.name:
                continue
            if f.name == 'http.js':
                continue  # 有意例外：axios 实例与拦截器基础设施（FE-MOD-GOV §3）
            if not f.name.endswith(suffix):
                bad.append(f'{f.relative_to(REPO)}（应为 *{suffix}）')
    if bad:
        err('命名规范违规：')
        for b in bad: err('  ' + b)
    else:
        info('E4 api/store 命名规范：全部符合 ✓')
    return bool(bad)

# ---------------- E5 ----------------
def check_vite():
    if not VITE.exists():
        err('frontend-ui/vite.config.js 不存在')
        return True
    c = VITE.read_text(encoding='utf-8', errors='replace')
    # 只检查代码行，忽略注释（注释可能解释 ElementPlusResolver 的弃用原因）
    code_lines = []
    in_block = False
    for line in c.splitlines():
        if '/*' in line and '*/' not in line:
            in_block = True
            continue
        if in_block:
            if '*/' in line:
                in_block = False
            continue
        stripped = line.strip()
        if stripped.startswith('//'):
            continue
        code_lines.append(line)
    code = '\n'.join(code_lines)
    if 'epSubpathResolver' not in code:
        err('vite.config.js 缺少 epSubpathResolver（element-plus 按需解析器，见 FE-MOD-GOV §2.2）')
        return True
    if 'ElementPlusResolver' in code:
        err('vite.config.js 仍引用官方 ElementPlusResolver（根 barrel，tree-shake 失效源，禁止）')
        return True
    info('E5 vite.config.js epSubpathResolver 在位 ✓')
    return False

# ---------------- W1 ----------------
def check_comments():
    bad = []
    for p in walk(SRC):
        c = p.read_text(encoding='utf-8', errors='replace')
        o, cl = len(re.findall(r'/\*', c)), len(re.findall(r'\*/', c))
        if o != cl:
            bad.append(f'{p.relative_to(REPO)} (open={o} close={cl})')
    if bad:
        warn(f'块注释配对异常 {len(bad)} 处（含字符串/CSS 误报可能，需人工甄别）：')
        for b in bad[:8]: warn('  ' + b)
    else:
        info('W1 块注释配对：全部平衡 ✓')
    return False

def main():
    ap = argparse.ArgumentParser(description='前端模块化治理门禁')
    ap.add_argument('--repo', type=str, default=str(REPO), help='仓库根目录')
    args = ap.parse_args()
    repo = Path(args.repo).resolve()
    global SRC, VITE
    SRC = repo / 'frontend-ui' / 'src'
    VITE = repo / 'frontend-ui' / 'vite.config.js'
    if not SRC.exists():
        print('[ERROR] frontend-ui/src 不存在：' + str(SRC))
        return 1
    results = [
        check_imports(), check_relative(), check_barrel_collisions(),
        check_naming(), check_vite(), check_deep_alias(), check_alias_targets(),
    ]
    check_comments()
    n_err = sum(1 for r in results if r)
    print(f'--- 前端模块化门禁：ERROR={n_err} ---')
    return 1 if n_err else 0

if __name__ == '__main__':
    sys.exit(main())
