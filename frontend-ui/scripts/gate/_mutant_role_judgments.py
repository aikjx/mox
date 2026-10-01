"""对 check-view-hex.py 新增判据做变异检验：每个变异体必须被 --selftest 里的用例打破。

变异体写在 scripts/gate/ 目录下（闸门用自身路径反推 UI 根，放别处会变成空跑），
跑完即删。基线（未变异）必须先 PASS=… FAIL=0，否则后面的「红」说明不了任何事。
"""
import os
import re
import subprocess
import sys

GATE_DIR = r'D:\a10\aikjx\gitcode\infotopograph\frontend-ui\scripts\gate'
SRC = os.path.join(GATE_DIR, 'check-view-hex.py')
ORIG = open(SRC, encoding='utf-8').read()

MUTANTS = [
    ('M1 拆掉 fill-unpaired 判定',
     'if colors and not (on_fams & want):', 'if False:'),
    ('M2 别家的 --on-x 也算配套（.m 该红）',
     'if colors and not (on_fams & want):', 'if colors and not on_fams:'),
    ('M3 @ 规则不再下钻（子规则并成一块）',
     "        if ch == '{':", "        if ch == '{' and not stack:"),
    ('M4 拆掉 text-as-fill 判定',
     "                elif not val.startswith(('linear-gradient', 'radial', 'repeating')):",
     '                elif False:'),
    ('M5 纯 .css 不扫',
     '    if whole_file:\n        return [(0, len(text))]',
     '    if whole_file:\n        return []'),
    ('M6 .vue 没有 <style> 就退化成扫整份',
     '    if whole_file:\n        return [(0, len(text))]\n'
     '    return [(m.start(1), m.end(1)) for m in STYLE_BODY_RE.finditer(text)]',
     '    return [(0, len(text))]'),
    ('M7 家族过滤丢掉（--nomesh-fill 也当家族，.n 该不误报）',
     'want = {f for f in FILL_IN_VALUE_RE.findall(val) if f in fams}',
     'want = set(FILL_IN_VALUE_RE.findall(val))'),
    ('M8 文字档做底不看家族（--surface 会被抓）',
     "if t and t.group(1)[2:] in fams and val.count('var(') == 1:",
     'if t:'),
    ('M9 行号在切片上数（回归：报告指向别的行）',
     "text[:at].count('\\n') + 1", "text[start:at].count('\\n') + 1"),
]


def run(path, arg):
    p = subprocess.run([sys.executable, path, arg], capture_output=True, cwd=GATE_DIR)
    return p.returncode, (p.stdout or b'').decode('utf-8', 'replace')


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
    for name, needle, repl in MUTANTS:
        n = ORIG.count(needle)
        if n != 1:
            print('%-52s 钉不上（命中 %d 次）⇒ 该变异体无效，需要重写' % (name, n))
            survivors.append((name, 'needle x%d' % n))
            continue
        path = os.path.join(GATE_DIR, '_mut_%d.py' % (MUTANTS.index((name, needle, repl)) + 1))
        with open(path, 'w', encoding='utf-8', newline='') as fh:
            fh.write(ORIG.replace(needle, repl))
        try:
            code, out = run(path, '--selftest')
            m = re.search(r'SELFTEST PASS=(\d+) FAIL=(\d+)', out)
            labels = failed_labels(out)
            killed = code != 0 and m and int(m.group(2)) > 0
            print('%-52s %s  %s' % (name, '被打破' if killed else '★存活★',
                                    ('｜打破者：' + ' ; '.join(l[:34] for l in labels[:3]))
                                    if labels else ('（selftest 崩在别处：%s）' % out.strip()[-90:] if not killed else '')))
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
