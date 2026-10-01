#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""check-framework-imports.py 的**常驻变异电池**：12 枚具名变异体 + 3 档遮蔽变体对比。

为什么要有这个文件：闸门自身的判据形状只能靠"还原成坏版本必须报红"来钉住。报告 §11.14 / §11.14续 / §11.15
的证据原先跑在一次性脚本里（临时目录清空即失传）⇒ 落库为可重跑驱动。

    （默认）        12 枚变异体逐一把 `--selftest` 打红；**无 `SELFTEST:` 判决行＝INVALID**（崩溃不冒充打红）
    --variants      三档真语料对比：base / 撤正则遮蔽 / 放宽到"任何 `/` 都当正则开头"
                    判据＝撤遮蔽必须 rc=1 且点到两个活文件；放宽档若与 base **同形**则印"语料盲"（此时闸门
                    对这类放宽唯一的牙是合成针 15），不同形则印出差异（不判 FAIL：语料形状会变，不许把
                    快照钉成棘轮）。

只读：不改任何源文件；变异副本写进本目录并在跑完删除。
"""
import os
import re
import subprocess
import sys

GATE = os.path.dirname(os.path.abspath(__file__))
TARGET = os.path.join(GATE, 'check-framework-imports.py')

MUTANTS = [
    ('A 撤掉 import 子句的位置豁免（子句里的名字被当使用位点）',
     "    return [m.start() for m in re.finditer(rx, clean) if not any(a <= m.start() < b for a, b in skip)]",
     "    return [m.start() for m in re.finditer(rx, clean)]",
     '跨行花括号 import'),
    ('B 撤掉「对象键不算」的前瞻',
     r"rx = r'(?<![\w$.])' + re.escape(x) + r'\b(?!\s*:)(?![\w$])'",
     r"rx = r'(?<![\w$.])' + re.escape(x) + r'\b(?![\w$])'",
     '对象键不算'),
    ('C 不再排除 .vue 的 template/style 段',
     "    if not name.endswith('.vue'):",
     "    if True:",
     'template 段不参与'),
    ('D 不再遮蔽注释与字符串',
     "    clean = mask_noise(body)",
     "    clean = body",
     '字符串/注释不算'),
    ('E 判缺的那半边被短路（探针失明，种缺陷对照必须报红）',
     "        if h:",
     "        if False and h:",
     '种缺陷对照'),
    ('F 不再识别解构形参（把 `{ storeToRefs }` 当未绑定）',
     "                if spec[0] in '{[':",
     "                if False:",
     '形参同名 ⇒ clean'),
    # G：§11.14 落库时 DEFAULT_RE 的原样（只认 `,`/行尾，不认 ` from`）——第三方普查撞出的真缺陷
    ('G 默认导入 `import X from` 不绑定（§11.14 修复前的形状）',
     r"(?P<ns>[A-Za-z_$][\w$]*)(?:\s*(?:,|from\b|$))",
     r"(?P<ns>[A-Za-z_$][\w$]*)\s*(?:,|$)",
     'default import 必须绑定该名字'),
    # H–J：§11.15 加宽（正则字面量遮蔽 / 对照按模块删 import）动到的两条通道
    ('H 撤掉正则字面量遮蔽（假阳复活：字符类里的 z 被当使用位点）',
     "        if c == '/' and regex_starts(text, i):",
     "        if False and c == '/':",
     '正则字符类里的 z 不算使用位点'),
    ('I 任何 `/` 都算正则开头（放宽 ⇒ 顺手吞掉除法右侧的真缺陷）',
     "    c = text[j]\n    if c in REGEX_OK_AFTER:",
     "    c = '='\n    if c in REGEX_OK_AFTER:",
     '除号不是正则：右边的裸名仍要判缺'),
    ('J 种缺陷对照不再按模块删 import（两条默认导入对照就此失明）',
     " % mod, '', src,",
     " % 'never-match', '', src,",
     '种缺陷对照 api/http.js'),
    # K–M：§11.15续 的"动态 import / require 绑定形状"。这五枚新针的 clean 是靠**通用局部声明**与
    # **属性访问豁免**两条通道得到的——若通道其实没看见它们，撤通道也不会报红 ⇒ 那五格就是空转。
    # 所以 K/M 的职责不是"发现缺陷"，而是**证明这五枚针确实挂在通道上**。
    ('K 撤掉「const 解构声明算绑定」通道（动态 import / require 的解构被误判缺）',
     r"""    for m in re.finditer(r'(?:const|let|var)\s*\{([^}]*)\}\s*=', clean, re.S):""",
     "    for m in iter([]):",
     '动态 import 解构 ⇒ clean'),
    ('M 撤掉属性访问豁免（lookbehind）：`mod.computed` 被当使用位点',
     r"rx = r'(?<![\w$.])' + re.escape(x) + r'\b(?!\s*:)(?![\w$])'",
     r"rx = r'' + re.escape(x) + r'\b(?!\s*:)(?![\w$])'",
     '动态 import 命名空间：属性访问不算'),
]

# --variants 用到的两档反向变体（与 H、I 同一处锚点，但看的是**真语料**而不是合成针）
LIVE_EXPECT = ('components/layout/TheSidebar.vue', 'modules/expert-alliance/components/GraphTeamPanel.vue')
VARIANTS = [
    ('base 现状', []),
    ('撤掉正则遮蔽', [("        if c == '/' and regex_starts(text, i):", "        if False and c == '/':")]),
    ('任何 `/` 都算正则开头', [("    c = text[j]\n    if c in REGEX_OK_AFTER:", "    c = '='\n    if c in REGEX_OK_AFTER:")]),
]

COUNT_RE = re.compile(r'(\d+) 个 / (\d+) 处')


def materialise(text, tag):
    path = os.path.join(GATE, '_mutant_%s.py' % tag)
    with open(path, 'w', encoding='utf-8', newline='\n') as f:
        f.write(text)
    return path


def run(path, *args):
    p = subprocess.run([sys.executable, '-X', 'utf8', path] + list(args),
                       capture_output=True, text=True, encoding='utf-8', errors='replace')
    return p.returncode, (p.stdout or '') + (p.stderr or '')


def mutated(base, label, needle, repl):
    """锚点必须**恰好一次**命中：0 次＝中文 needle 静默不命中却报"通过"，>1 次＝改错了地方。"""
    if base.count(needle) != 1:
        raise AssertionError('%s：锚点出现 %d 次（期望 1）' % (label, base.count(needle)))
    return base.replace(needle, repl)


def battery(base):
    bad = 0
    for label, needle, repl, expect_case in MUTANTS:
        try:
            text = mutated(base, label, needle, repl)
        except AssertionError as e:
            print('[INVALID] %s' % e)
            bad += 1
            continue
        path = materialise(text, 'framework_imports')
        try:
            rc, out = run(path, '--selftest')
        finally:
            os.remove(path)
        if 'SELFTEST:' not in out:
            print('[INVALID] %s：无判决行（崩溃冒充打红）\n%s' % (label, out[-400:]))
            bad += 1
            continue
        hit = [l for l in out.split('\n') if expect_case in l and '[FAIL]' in l]
        ok = rc == 1 and bool(hit)
        bad += 0 if ok else 1
        allred = [l.strip()[7:].split(' ->')[0] for l in out.split('\n') if '[FAIL]' in l]
        print('%s %s -> rc=%d 红针=%s\n        该枚共打红 %d 格：%s' % (
            '[OK]  ' if ok else '[FAIL]', label, rc,
            hit[0].strip()[:150] if hit else '（点名的针没红）', len(allred), '、'.join(allred) or '—'))
    print('FRAMEWORK-IMPORTS MUTANTS: %s (%d 枚变异体都必须把 --selftest 打红)' % (
        'PASS' if bad == 0 else 'FAIL', len(MUTANTS)))
    return 1 if bad else 0


def parse_counts(out):
    got = [(int(a), int(b)) for a, b in COUNT_RE.findall(out)]
    return (got + [(None, None), (None, None)])[:2]


def variants(base):
    bad, summary = 0, {}
    for label, pairs in VARIANTS:
        text = base
        for needle, repl in pairs:
            text = mutated(text, label, needle, repl)
        path = materialise(text, 'variant')
        try:
            rc, out = run(path)
        finally:
            os.remove(path)
        app, tst = parse_counts(out)
        hits = sorted({l.strip().split()[0] for l in out.split('\n') if l.startswith('  ') and l.strip()})
        summary[label] = (rc, app, tst)
        print('%-22s rc=%d 非测试=%s 测试=%s 点名命中=%s' % (label, rc, app, tst, hits or '—'))
        if label == 'base 现状' and (rc != 0 or app != (0, 0) or tst != (0, 0)):
            print('[FAIL] base 必须 rc=0 且两桶全 0（否则本节全部前提失效）')
            bad += 1
        if label == '撤掉正则遮蔽':
            if rc != 1 or app[0] < 1:
                print('[FAIL] 撤遮蔽必须把干净仓判红（遮蔽＝承重）')
                bad += 1
            missing = [f for f in LIVE_EXPECT if not any(f in h for h in hits)]
            if missing:
                print('[FAIL] 撤遮蔽没点到预期活文件：%s（语料变了⇒本节口径要重写）' % missing)
                bad += 1
    b, w = summary['base 现状'], summary['任何 `/` 都算正则开头']
    print('放宽档 vs base：%s' % ('语料**同形**⇒真语料对它盲，唯一牙＝合成针 15（变异体 I 必须打红）'
                                  if b == w else '语料可见差异 ⇒ 去查是不是吞掉了真缺陷 %s vs %s' % (b, w)))
    print('FRAMEWORK-IMPORTS VARIANTS: %s (3 档遮蔽变体)' % ('PASS' if bad == 0 else 'FAIL'))
    return 1 if bad else 0


def main(argv):
    base = open(TARGET, encoding='utf-8').read()
    return variants(base) if '--variants' in argv else battery(base)


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    sys.exit(main(sys.argv[1:]))
