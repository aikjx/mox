"""对 check-view-hex.py 的**扫描集**（判据 1 的覆盖面）与**表面阶梯**（判据 6）做变异检验。

覆盖面是账本的地基：它错了，后面所有判定都建立在缺账的语料上，且报告照样绿。
判据 6 是新增的纯静态判定（跟 var 链、比组内别名、比组间塌陷、对豁免台账），
没有真机后果可依赖，只能靠变异体证明那十几条合成用例真的在看着它。
每个变异体必须被 --selftest 里的用例打破；基线不绿则全部作废。跑完即删，`_` 前缀不进 CI。
"""
import os
import re
import subprocess
import sys

GATE_DIR = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\scripts\gate'
SRC = os.path.join(GATE_DIR, 'check-view-hex.py')
ORIG = open(SRC, encoding='utf-8').read()

if hasattr(sys.stdout, 'reconfigure'):
    # Windows 控制台默认 GBK，变异体名字里的 ⇒ 会直接 UnicodeEncodeError 打断整跑
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')

SCAN_OK = '''    if rel.startswith(SCAN_EXCLUDE):
        return False
    if rel.endswith('.vue'):
        return True
    # .test.js/.spec.js 不参与：见开头说明
    return rel.endswith('.js') and not rel.endswith(('.test.js', '.spec.js'))'''

MUTANTS = [
    ('S1 拆掉 SCAN_EXCLUDE ⇒ modules 进账（当前 0 处，静默无感）',
     '    if rel.startswith(SCAN_EXCLUDE):\n        return False\n', ''),
    ('S2 白名单复辟（只认旧三根）⇒ stores/ 与新目录逃到账外',
     SCAN_OK,
     '''    if not rel.startswith(('src/views/', 'src/constants/', 'src/components/')):
        return False
    if rel.endswith('.vue'):
        return True
    return rel.endswith('.js') and not rel.endswith(('.test.js', '.spec.js'))'''),
    ('S3 丢掉 .test.js/.spec.js 过滤 ⇒ 测试字面量混进账',
     "    return rel.endswith('.js') and not rel.endswith(('.test.js', '.spec.js'))",
     "    return rel.endswith('.js')"),
    ('S4 排除项写成 os.sep 坐标（Windows 下 rel 已归一成 / ⇒ 永不命中，排除静默失效）',
     "SCAN_EXCLUDE = ('src/modules/',)",
     "SCAN_EXCLUDE = ('src' + os.sep + 'modules' + os.sep,)"),

    # ---- 判据 6：表面阶梯（var 链 / 组内别名 / 组间塌陷 / 豁免对表）----
    ('T1 不跟 var 链（别名层直接当字面值）',
     '        m = VAR_ALIAS_RE.match(raw)',
     '        m = None'),
    ('T2 断链丢掉兜底值（浏览器回退语义被无视 ⇒ 假报缺档）',
     "            return _norm_color(rest.split(',', 1)[1]) if ',' in rest else None",
     '            return None'),
    ('T3 成环报成缺档（红字指错病灶，且和真缺档混成一片）',
     "            return '\u2205\u73af:' + '\u2192'.join(chain[chain.index(base):])",
     '            return None'),
    ('T4 环判定拆掉（互相 var 的两名 ⇒ while True 无限绕）',
     '        if base in chain:',
     '        if False:'),
    ('T5 豁免按皮名整片放行（换个档对塌陷照样绿）',
     '        if (skin, frozenset(pair)) not in ok:',
     '        if skin not in {s for s, _ in ok}:'),
    ('T6 豁免键不做档名归一（书写顺序与字典序一翻脸就假红）',
     '    return {(s, frozenset(p)) for s, *p in SURFACE_COLLAPSE_OK}',
     '    return {(s, tuple(p)) for s, *p in SURFACE_COLLAPSE_OK}'),
    ('T7 拆掉组内别名判定（别名层写错没人管）',
     '            if len(vals) > 1:',
     '            if False:'),
    ('T8 拆掉组间塌陷判定（两层同值 = 层级消失，无人拦）',
     '                if va == vb:',
     '                if False:'),
    # T9 写成"跳过上报但继续跑"：直接把 `if resolved[n] is None` 换成 False 会让下一行
    # elif 在 None 上 .startswith 崩掉，崩在代码里而不是崩在断言上 ⇒ 那不算打破（没有判决行）。
    ('T9 拆掉缺档上报（漏覆写一个别名 ⇒ 静默）',
     "                if resolved[n] is None:\n                    problems.append(('surface-missing', skin, n))",
     "                if resolved[n] is None:\n                    pass"),
    ('T10 豁免对表返回常量（"每条豁免还对应真账"变成哨兵值自比）',
     '    return sorted({(s, tuple(sorted(p))) for s, p, _ in collapses if (s, frozenset(p)) in ok})',
     '    return sorted({(s, tuple(sorted(p))) for s, *p in SURFACE_COLLAPSE_OK})'),
]


def run(path, arg):
    """T4（拆掉环判定）会把 selftest 变成死循环 ⇒ 超时也算被打破，但要说清是哪种死法。"""
    try:
        p = subprocess.run([sys.executable, path, arg], capture_output=True, cwd=GATE_DIR,
                           timeout=120)
        return p.returncode, (p.stdout or b'').decode('utf-8', 'replace')
    except subprocess.TimeoutExpired:
        return -1, 'TIMEOUT：变异体让闸门转不起来了（对棘轮类判据来说，转不动＝用例走不完＝红）'


def failed_labels(out):
    return [m.group(1).strip() for m in
            re.finditer(r'^\s+FAIL\s+(.*?)（得', out, re.M)]


def main():
    code, out = run(SRC, '--selftest')
    base = re.search(r'SELFTEST PASS=(\d+) FAIL=(\d+)', out)
    print('基线（未变异）：%s rc=%d' % (base.group(0) if base else '没有判决行', code))
    if code != 0 or not base or base.group(2) != '0':
        print('基线不绿 ⇒ 变异结果全部作废')
        print(out[-1500:])
        return 1
    base_n = int(base.group(1))
    survivors = []
    for i, (name, needle, repl) in enumerate(MUTANTS):
        n = ORIG.count(needle)
        if n != 1:
            print('%-58s 钉不上（命中 %d 次）⇒ 该变异体无效，需要重写' % (name, n))
            survivors.append((name, 'needle x%d' % n))
            continue
        path = os.path.join(GATE_DIR, '_mut_scan_%d.py' % (i + 1))
        with open(path, 'w', encoding='utf-8', newline='') as fh:
            fh.write(ORIG.replace(needle, repl))
        try:
            code, out = run(path, '--selftest')
            hung = out.startswith('TIMEOUT')
            m = re.search(r'SELFTEST PASS=(\d+) FAIL=(\d+)', out)
            labels = failed_labels(out)
            killed = hung or (code != 0 and m and int(m.group(2)) > 0)
            print('%-58s %s  %s' % (name, '被打破' if killed else '★存活★',
                                    ('｜转不动（死循环）' if hung else
                                     ('｜打破者：' + ' ; '.join(l[:40] for l in labels[:3]))
                                     if labels else
                                     ('（selftest 崩在别处：%s）' % out.strip()[-90:]
                                      if not killed else ''))))
            if not killed:
                survivors.append((name, m.group(0) if m else 'no-verdict'))
        finally:
            os.remove(path)
    print('\n基线用例 %d 条；变异体 %d 个，存活 %d 个' % (base_n, len(MUTANTS), len(survivors)))
    for s in survivors:
        print('  存活：%s → %s' % s)
    return 1 if survivors else 0


if __name__ == '__main__':
    sys.exit(main())
