#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Element Plus 函数式 API（ElMessage / ElMessageBox / ElNotification / ElLoading）缺 import 探针。

背景：vite 侧只有 unplugin-vue-components（解析模板 <el-*>），没有 auto-import ⇒ 脚本里这些名字是
普通标识符，必须逐文件 import，否则运行到该行必抛 ReferenceError。而 `_smoke.js` 等测试里的
`globalThis.ElMessage = {…}` 桩会把这件事掩盖成绿灯。

本脚本是**探针不是棘轮**：债务多寡不影响 rc（清账只会让它更好），只有「判据自身失效」才打红：
  --check     跑 7 条真实文件对照（6 枚必须不 flag ＋ 1 枚必须 flag），任一失配 rc=1
  --selftest  跑 4 枚合成变异针（两行式 import／缺 import／动态解构／vi.mock 对象键）
  默认/--json 只报账（rc=0）
"""
import json
import os
import re
import sys

NAMES = ['ElMessage', 'ElMessageBox', 'ElNotification', 'ElLoading']
IMPORT_ALL = re.compile(r'import\s*\{([^}]*)\}\s*from\s*[\'"]element-plus[^\'"]*[\'"]', re.S)
DYN_ALL = re.compile(r'(?:const|let|var)\s*\{([^}]*)\}\s*=\s*(?:await\s+)?import\s*\(\s*[\'"]element-plus[^\'"]*[\'"]\s*\)')

UI_ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
SRC = os.path.join(UI_ROOT, 'src')

MUST_BE_CLEAN = [
    'main.js',
    'composables/useKnowledgeBase.js',
    'modules/admin-lowcode/composables/useCrudPage.js',
    'modules/admin-lowcode/pages/tenant.page.js',
    'modules/admin-lowcode/pages/access.page.js',
    'utils/message.utils.js',
]
# flag 侧对照不许钉在"仓库还得留着这笔债"的具体文件上（债务清零后该控制会反咬自己，
# 且它证明的是语料没变而不是尺子还认得出形状）。改喂一段合成样例给同一个 classify。
MUST_BE_FLAGGED = None  # 已退役：见 SYNTH_FLAG
SYNTH_FLAG = ("import { getToken } from '@/utils'\nElMessage.error(msg)\nElMessage.error(msg2)\n", {'ElMessage'})


def imported_names(src):
    got = set()
    for m in list(IMPORT_ALL.finditer(src)) + list(DYN_ALL.finditer(src)):
        for spec in m.group(1).split(','):
            spec = spec.strip()
            if spec:
                got.add(re.split(r'\s+as\s+', spec)[0].strip())
    return got


def decl_lines(src):
    out = set()
    for m in list(IMPORT_ALL.finditer(src)) + list(DYN_ALL.finditer(src)):
        out.add(src[:m.start()].count('\n') + 1)
    return out


def _code_lines(src):
    out, in_block = [], False
    for i, line in enumerate(src.split('\n'), 1):
        s = line.strip()
        if in_block:
            if '*/' in s:
                in_block = False
            continue
        if s.startswith('/*'):
            if '*/' not in s:
                in_block = True
            continue
        if s.startswith('//') or s.startswith('*') or s.startswith('<!--'):
            continue
        out.append((i, line))
    return out


def classify(src):
    """-> {name: (use_count, first_line)} for names used in code but never imported"""
    imp = imported_names(src)
    decl = decl_lines(src)
    miss = {}
    for x in NAMES:
        rx = re.compile(r'(?<![\w$.\'"])' + x + r'\b')
        hits = []
        for i, line in _code_lines(src):
            if i in decl or 'vi.mock' in line or not rx.search(line):
                continue
            if re.search(r'\b' + x + r'\s*:', line):        # 对象键位置，不是使用位点
                continue
            stripped = re.sub(r'[\'"][^\'"]*[\'"]', "''", line)
            if rx.search(stripped):
                hits.append(i)
        if hits and x not in imp:
            miss[x] = (len(hits), hits[0])
    return miss


def is_test(path):
    return re.search(r'\.(test|spec)\.(js|ts)$', path) is not None


def scan():
    app, tests = [], []
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames[:] = [d for d in dirnames if d not in ('node_modules', '.git')]
        for fn in filenames:
            if not fn.endswith(('.vue', '.js', '.ts')) or '_smoke' in fn:
                continue
            p = os.path.join(dirpath, fn)
            rel = os.path.relpath(p, SRC).replace(os.sep, '/')
            src = open(p, encoding='utf-8', errors='replace').read()
            miss = classify(src)
            if miss:
                (tests if is_test(fn) else app).append((rel, miss))
    app.sort(key=lambda r: -sum(v[0] for v in r[1].values()))
    return app, tests


def report(app, tests):
    n = sum(v[0] for _, m in app for v in m.values())
    print('缺 import 的非测试文件：%d 个 / %d 处；测试文件另计 %d 个（不参与账目）' % (len(app), n, len(tests)))
    for rel, miss in app:
        print('  %-58s %s' % (rel, {k: '%d@L%d' % v for k, v in miss.items()}))
    return n


def main(argv):
    mode = argv[1] if len(argv) > 1 else ''
    if mode == '--selftest':
        cases = [
            ('two-line subpath imports -> clean',
             "import { ElMessage } from 'element-plus/es/components/message/index'\n"
             "import { ElMessageBox } from 'element-plus/es/components/message-box/index'\n"
             "ElMessage.ok(); await ElMessageBox.confirm('x')\n", set()),
            ('missing import -> flagged',
             "import { getToken } from '@/utils'\nElMessage.error(msg)\nElMessage.error(msg2)\n", {'ElMessage'}),
            ('dynamic destructure import -> clean',
             "const { ElMessage } = await import('element-plus')\nElMessage.success('ok')\n", set()),
            ('vi.mock object key -> not a use site',
             "vi.mock('element-plus', () => ({ ElMessage: { error: vi.fn() } }))\n", set()),
        ]
        bad = 0
        for label, src, want in cases:
            got = set(classify(src))
            ok = got == want
            bad += 0 if ok else 1
            print('%s %s -> %s (expect %s)' % ('[OK]  ' if ok else '[FAIL]', label, sorted(got), sorted(want)))
        print('EP-IMPORT SELFTEST: %s (%d/%d)' % ('PASS' if bad == 0 else 'FAIL', len(cases) - bad, len(cases)))
        return 1 if bad else 0

    app, tests = scan()
    if mode == '--json':
        print(json.dumps({'files': [{'path': p, 'missing': {k: list(v) for k, v in m.items()}}
                                    for p, m in app],
                          'test_files': [p for p, _ in tests]}, ensure_ascii=False, indent=1))
    else:
        report(app, tests)
    by_rel = {p for p, _ in app}
    synth_miss = set(classify(SYNTH_FLAG[0]))
    flag_ok = synth_miss == SYNTH_FLAG[1]
    bad = [p for p in MUST_BE_CLEAN if p in by_rel] + ([] if flag_ok else ['flag 侧合成样例未被判缺＝尺子认不出这个形状了'])
    print('对照：clean 侧 %d/%d 通过，flag 侧 %s（合成样例判出 %s；语料现量缺 import %d 个文件／%d 处）' % (
        len(MUST_BE_CLEAN) - len([p for p in bad if p in MUST_BE_CLEAN]), len(MUST_BE_CLEAN),
        '通过' if flag_ok else '未通过', sorted(synth_miss), len(app),
        sum(v[0] for _, m in app for v in m.values())))
    if bad:
        print('EP-IMPORT CONTROLS: FAIL -> ' + ', '.join(bad))
        return 1
    print('verdict=PASS (探针：债务多少不改 rc)')
    return 0


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    sys.exit(main(sys.argv))
