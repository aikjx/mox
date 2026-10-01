#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""给 check-view-hex.py 的六面判据上牙：逐枚撤掉一条通道，要求 --selftest 里挂着这条通道的**具名针**必须变红。
覆盖：棘轮（A/B/C）、扫描集边界（D/E）、色板（F/G）、角色串门与文字档做底即判据 5（H/I/J）、
表面阶梯即判据 6（K/L/M）、正文灰阶（N/O）。

为什么用"新增红针"而不是 rc：真语料此刻 --selftest 本就 FAIL=4（两条反咬型覆盖面下限 + 一条排除层
tripwire + 一条故意不回填的台账条目），所以 rc≠0 不是证据。判据＝**基线红针集合之外**新红了哪几格，
且必须是该枚指名的那一格。没有 `SELFTEST PASS=` 判决行一律 INVALID（崩溃只有点名被测守卫的报错才算捕获）。

只读：本驱动不写任何文件。被测闸门唯一会改写自身 BASELINE 的入口是 main() 的 --baseline/--role-baseline
分支，这里**从不调用 main()**，并在前后各取一次闸门文件的 sha256 打印出来做见证。
"""
import hashlib
import importlib.util
import io
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
GATE = os.path.join(HERE, 'check-view-hex.py')
sys.dont_write_bytecode = True


def load():
    spec = importlib.util.spec_from_file_location('cvh_under_test', GATE)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def run_selftest(mod):
    buf = io.StringIO()
    real = sys.stdout
    sys.stdout = buf
    try:
        rc = mod.selftest()
    finally:
        sys.stdout = real
    out = buf.getvalue()
    if 'SELFTEST PASS=' not in out:
        return None, rc, out
    red = set()
    for line in out.split('\n'):
        s = line.strip()
        if s.startswith('FAIL'):
            red.add(s[4:].split('（')[0].strip())
    return red, rc, out


def needle(name):
    """按针名的稳定前缀取（红字尾部的"（得 … 期望 …）"随语料变，不能进 key）。"""
    return name


# ---- 七枚变异体：每枚撤掉一条通道，指名挂着这条通道的针 ----
MUTANTS = [
    ('A', '棘轮从"逐文件比"退化成"只比总量"（别处减 4 抵掉这里加 4）',
     lambda o: (lambda hits, base: (
         (lambda g, n, s: ([] if sum(len(v) for v in hits.values()) <= sum(base.values()) else g, n, s))(
             *o(hits, base)))),
     needle('多一处就 FAIL，且按文件记')),
    ('B', '棘轮不再报 SHRANK（减少量被吞，回填提示消失）',
     lambda o: (lambda hits, base: (lambda t: (t[0], t[1], []))(o(hits, base))),
     needle('低于基线只记 SHRANK')),
    ('C', '首建基线时也判 NEW（空基线 ⇒ 满屏红 ⇒ 下次没人敢建基线）',
     lambda o: (lambda hits, base: (
         lambda t: (t[0], [(r, len(v)) for r, v in hits.items()], t[2])
         if not base else t)(o(hits, base))),
     needle('基线为空 ⇒ 不判 NEW')),
    ('D', 'scan_ok 放过 .test.js/.spec.js（测试断言字面量被当成渲染债）',
     lambda o: (lambda r: o(r) or r.endswith(('.test.js', '.spec.js'))),
     needle('测试文件不进账')),
    ('E', 'scan_ok 把 src/constants/ 踢出扫描集（把字面量搬进单源即可洗账）',
     lambda o: (lambda r: o(r) and not r.replace(os.sep, '/').startswith('src/constants/')),
     needle('constants 目录确实在扫描范围内')),
    ('F', 'palette_problems 对 missing 失明（皮肤少一档不再报）',
     lambda o: (lambda pal: [p for p in o(pal) if p[0] != 'missing']),
     needle('缺档被抓到')),
    ('G', 'palette_problems 对 notcolor 失明（值不是颜色字面量也算填了档）',
     lambda o: (lambda pal: [p for p in o(pal) if p[0] != 'notcolor']),
     needle('值不是颜色字面量也要被抓到')),
    # ---- 判据 5（角色串门 / 文字档做底）与判据 6（表面阶梯 + 正文灰阶）：本轮补齐的八枚 ----
    ('H', 'role_hits 对 fill-as-text 失明（字色读填充档不再算串门）',
     lambda o: (lambda *a, **k: ([h for h in o(*a, **k)[0] if h[0] != 'fill-as-text'],
                                 o(*a, **k)[1])),
     needle('color: 读 --x-fill 被抓到')),
    ('I', 'role_hits 对 text-as-fill 失明（文字档当底 ⇒ 棘轮的分子整个归零）',
     lambda o: (lambda *a, **k: (o(*a, **k)[0],
                                 [h for h in o(*a, **k)[1] if h[0] != 'text-as-fill'])),
     needle('background 单值读文字档被抓为')),
    ('J', 'role_hits 无视 whole_file（纯 .css 没有 <style> 就整份不判）',
     lambda o: (lambda text, fams, whole_file=False: o(text, fams, False)),
     needle('纯 .css 没有')),
    ('K', 'surface_problems 对 surface-collapse 失明（卡片与面板无缝塌陷不再报）',
     lambda o: (lambda tables=None: [p for p in o(tables) if p[0] != 'surface-collapse']),
     needle('两档塌成同一个值')),
    ('L', 'surface_problems 对 surface-alias 失明（组内别名不同值 ⇒ 该档无定义也不再报）',
     lambda o: (lambda tables=None: [p for p in o(tables) if p[0] != 'surface-alias']),
     needle('组内别名不同值')),
    ('M', 'surface_problems 对 surface-cycle 失明（var 成环不再冒充缺档，而是两样都不报）',
     lambda o: (lambda tables=None: [p for p in o(tables) if p[0] != 'surface-cycle']),
     needle('成环')),
    ('N', 'tier_problems 对 tier-dup 失明（正文四级灰阶撞成三级不再报）',
     lambda o: (lambda tiers: [p for p in o(tiers) if p[0] != 'tier-dup']),
     needle('两档撞色被抓到')),
    ('O', 'tier_problems 对 tier-missing 失明（皮肤少一档灰阶只由别的判据兜 ⇒ 此处漏报）',
     lambda o: (lambda tiers: [p for p in o(tiers) if p[0] != 'tier-missing']),
     needle('缺一档被抓到')),
]

TARGETS = {'A': 'ratchet', 'B': 'ratchet', 'C': 'ratchet',
           'D': 'scan_ok', 'E': 'scan_ok',
           'F': 'palette_problems', 'G': 'palette_problems',
           'H': 'role_hits', 'I': 'role_hits', 'J': 'role_hits',
           'K': 'surface_problems', 'L': 'surface_problems', 'M': 'surface_problems',
           'N': 'tier_problems', 'O': 'tier_problems'}


def sha(path):
    with open(path, 'rb') as f:
        return hashlib.sha256(f.read()).hexdigest()[:16]


def main():
    if not os.path.exists(GATE):
        print('HEX-MUTANTS: INVALID 找不到被测闸门 %s' % GATE)
        return 2
    before = sha(GATE)
    mod = load()

    base_red, base_rc, _ = run_selftest(mod)
    if base_red is None:
        print('HEX-MUTANTS: INVALID 未变异基线没有 SELFTEST 判决行（崩在半路，后面的红都不可信）')
        return 2
    print('基线：--selftest rc=%s 已红 %d 格（真语料欠账，非本驱动判据）：%s'
          % (base_rc, len(base_red), '、'.join(sorted(n[:22] for n in base_red)) or '—'))

    bad, lines = [], []
    for mid, desc, factory, want in MUTANTS:
        attr = TARGETS[mid]
        orig = getattr(mod, attr)
        setattr(mod, attr, factory(orig))
        err = None
        try:
            red, rc, out = run_selftest(mod)
        except Exception as exc:      # 崩溃只有点名被测守卫的报错才算捕获，一律记 INVALID
            red, rc, out = 'CRASH', '%s: %s' % (type(exc).__name__, exc), ''
            err = str(exc).split('\n')[0][:90]
        finally:
            setattr(mod, attr, orig)
        if red == 'CRASH':
            lines.append('  [%s] %s -> INVALID 撤这条通道把 --selftest 打崩（%s）' % (mid, desc, err))
            bad.append(mid)
            continue
        if red is None:
            lines.append('  [%s] %s -> INVALID 无 SELFTEST 判决行 rc=%s' % (mid, desc, rc))
            bad.append(mid)
            continue
        newly = sorted(n for n in (red - base_red))
        hit = [n for n in newly if n.startswith(want)]
        ok = bool(hit)
        if not ok:
            bad.append(mid)
        lines.append('  [%s] %s -> rc=%s 指名针%s=%s'
                     '%s\n        该枚新打红 %d 格：%s'
                     % (mid, desc, rc, '命中' if ok else '未命中', want[:28],
                        '' if ok else '  <== 该撤通道却没红到指名那格',
                        len(newly), '、'.join(n[:28] for n in newly) or '—'))
    print('\n'.join(lines))

    after = sha(GATE)
    print('闸门文件 sha256 前/后 = %s / %s %s' % (before, after, '（未改写）' if before == after else '（<== 被改写了！）'))
    if bad or before != after:
        print('HEX-MUTANTS: FAIL（%d/%d 枚没打红指名的针：%s）' % (len(bad), len(MUTANTS), ' '.join(bad)))
        return 1
    print('HEX-MUTANTS: PASS（%d 枚变异体都必须把 --selftest 指名的针打红）' % len(MUTANTS))
    return 0


if __name__ == '__main__':
    sys.exit(main())
