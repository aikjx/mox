"""主题令牌覆盖审计：把「谁消费了哪个令牌」「哪个主题覆写了哪些名字」算成一张可复算的表。

判据全部来自 CSS/源码文本，不依赖运行时（网关停不停都能跑），也不依赖浏览器：
① 消费侧：`var(--x)` 在 src/**（css + vue SFC）里出现的位置数，并区分裸用与带兜底；
② 定义侧：global.css 的 `:root` 与各主题文件的 `html[data-theme=…]` 裸根块里定义的令牌名；
③ 由此算出三类缺陷：无人定义的孤儿名、只在主题里定义因而「默认皮肤（无 data-theme）」读不到的名、
   以及主题从未覆写但外壳一直在读的名字（sky 的浅色页面渲染成深底就是这么来的）。

用法：python scripts/gate/check-theme-tokens.py [--md|--check|--selftest]
  --md        输出 markdown 审计报告（默认输出纯文本报告）
  --check     棘轮闸门：新增孤儿令牌即 exit 1，存量登记在 ORPHAN_BASELINE
  --selftest  用合成语料同时验正例与反例（判据能不能红）
"""
import glob
import io
import os
import re
import sys

UI = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
SRC = os.path.join(UI, 'src')
STYLES = os.path.join(SRC, 'styles')

THEME_FILES = {
    'global': os.path.join(STYLES, 'global.css'),
    'dark': os.path.join(STYLES, 'themes', 'theme-dark.css'),
    'cyberpunk': os.path.join(STYLES, 'themes', 'theme-cyberpunk.css'),
    'sky': os.path.join(STYLES, 'themes', 'theme-sky.css'),
}

# Element Plus 自带的那套 --el-* 变量是别名链的真实终点：theme-chalk 在它的 :root 里把
# --el-color-danger-light-{3,5,7} 这类档位全定义了，而我们只覆写其中几档。不认识这一套的闸门
# 会把 `--el-color-error-light-3: var(--el-color-danger-light-3)` 报成断链（2026-09-25 实测误报 4 个），
# 而浏览器解析得好好的 ⇒ 假红会把闸门训练成"习惯忽略的对象"。
EP_CHALK = os.path.join(UI, 'node_modules', 'element-plus', 'theme-chalk', 'index.css')

DEF_RE = re.compile(r'(--[A-Za-z0-9-]+)\s*:\s*([^;{}]+)')
USE_RE = re.compile(r'var\(\s*(--[A-Za-z0-9-]+)(\s*,)?')
ROOT_BLOCK_RE = re.compile(r'(:root|html\[data-theme="[a-z]+"\])\s*\{', re.M)
HEX_RE = re.compile(r'#([0-9a-fA-F]{6})\b')
ASSIGN_RE = re.compile(r'(--[A-Za-z0-9-]+)\s*:[^()]')
STYLE_KEY_RE = re.compile(r'["\'](--[A-Za-z0-9-]+)["\']\s*:')

# --check 的棘轮基线：2026-09-25 实测的存量断链名 14 个，已由 global.css 的影子名别名层全部接上，
# 所以基线清空 ⇒ 现在任何一处新增的裸用未定义令牌都会让闸门 exit 1。
# 若将来不得不登记存量豁免，写清「谁在读、为什么暂时不修」，别只留名字。
ORPHAN_BASELINE = frozenset()


def strip_comments(text):
    """把 CSS 与模板注释挖空成等长空白：注释里的 `:root {` 会被当成裸根块、注释里的
    `var(--x)` 会被当成消费点，都必须去；保留换行是为了行号仍然指向真行。
    """
    def blank(m):
        return '\n' * m.group(0).count('\n')
    text = re.sub(r'/\*.*?\*/', blank, text, flags=re.S)
    return re.sub(r'<!--.*?-->', blank, text, flags=re.S)


def read(rel):
    with io.open(os.path.join(UI, rel), encoding='utf-8') as fh:
        return strip_comments(fh.read())


def root_defs(css_text):
    """只取根选择器（:root 或 html[data-theme=…]）的裸块里定义的令牌——组件级覆写不算主题令牌。

    选择器后必须紧跟 `{`，所以 `html[data-theme="dark"] body {` 这类派生规则不会被算进来；
    若一份文件里出现多个裸根块，它们的定义合并（CSS 语义也是如此）。
    """
    out = {}
    for m in ROOT_BLOCK_RE.finditer(css_text):
        depth = 0
        start = m.end() - 1
        for i in range(start, len(css_text)):
            if css_text[i] == '{':
                depth += 1
            elif css_text[i] == '}':
                depth -= 1
                if depth == 0:
                    for name, value in DEF_RE.findall(css_text[start + 1:i]):
                        out.setdefault(name, ' '.join(value.split()))
                    break
    return out


def element_plus_defs():
    """Element Plus 的 :root 变量表；文件缺失时返回 None（调用方必须把它当"证据无效"，不是"没有定义"）。"""
    if not os.path.exists(EP_CHALK):
        return None
    with io.open(EP_CHALK, encoding='utf-8', errors='replace') as fh:
        return root_defs(strip_comments(fh.read()))


def usages():
    """令牌名 → 消费点列表（file:line + 是否写了兜底值），扫描 css / vue SFC。

    按整文件文本匹配而不是逐行：`var(换行--x换行)` 这类写法也要能抓到兜底逗号。
    不扫 js：实测 `src/**!(*.test).js` 里 `var(--` 出现 0 次，加上只会把测试样例文本混进来。
    """
    used = {}
    for ext in ('css', 'vue'):
        for path in glob.glob(os.path.join(SRC, '**', '*.' + ext), recursive=True):
            rel = os.path.relpath(path, UI).replace(os.sep, '/')
            with io.open(path, encoding='utf-8', errors='replace') as fh:
                for name, site, fallback in scan_text(rel, strip_comments(fh.read())):
                    used.setdefault(name, []).append((site, fallback))
    return used


def scan_text(rel, text):
    """(令牌名, 'file:line', 是否带兜底) 序列——单独抽出来是为了能被合成语料直接喂。"""
    out = []
    for m in USE_RE.finditer(text):
        # 兜底写法 var(--x, #fff) 不是"拿不到值"，而是"永远拿同一个硬编码值"——两种病因不同
        out.append((m.group(1),
                    f'{rel}:{text[:m.start()].count(chr(10)) + 1}',
                    bool(m.group(2))))
    return out


def luminance(hexval):
    """WCAG 相对亮度（只看 6 位 hex；渐变/rgba 跳过）。"""
    r, g, b = (int(hexval[i:i + 2], 16) / 255.0 for i in (0, 2, 4))
    f = lambda c: c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b)


def bg_lum(defs):
    vals = [v for k, v in defs.items() if k.startswith('--bg-')]
    hexes = [h for v in vals for h in HEX_RE.findall(v)]
    if not hexes:
        return None, 0
    return sum(luminance(h) for h in hexes) / len(hexes), len(hexes)


def selftest():
    """判据自己得能被证伪：合成语料里既有"必须被抓到"也有"必须不被抓到"的两类样本。

    只有正例的自检等于空转——root_defs 把派生选择器也算进定义面时，正例照样全绿。
    """
    css = '''
    /* a { color: var(--in-comment); } :root { --in-comment: #000000; } */
    :root { --ok: #112233; }
    html[data-theme="x"] { --theme-only: #000000; }
    html[data-theme="x"] body { --not-a-root-token: #ffffff; background: var(--also-nope); }
    a { color: var(--nope); }
    b { color: var(--ok); }
    c { color: var(--nope2, #fff); }
    d { color: var(
          --multiline); }
    '''
    body = strip_comments(css)
    defs = root_defs(body)
    found = scan_text('synthetic.css', body)
    sites = {n for n, _, _ in found}
    fb = {n: flag for n, _, flag in found}
    line_of = {n: s for n, s, _ in found}
    checks = [
        ('注释里的 :root 不算定义', '--in-comment' not in defs),
        ('注释里的 var() 不算消费点', '--in-comment' not in sites),
        ('裸根块里的名字算定义', '--ok' in defs and '--theme-only' in defs),
        ('派生选择器里的定义不算主题令牌', '--not-a-root-token' not in defs),
        ('未知名被抓为消费点', '--nope' in sites and '--also-nope' in sites),
        ('换行写法也算消费点', '--multiline' in sites),
        ('兜底位被识别', fb.get('--nope2') is True and fb.get('--nope') is False),
        ('行号指向真行', line_of['--nope'] == 'synthetic.css:6'),
        ('EP 的 :root 档位名会被认成定义（不认就假报断链）',
         '--el-color-danger-light-3' in root_defs(strip_comments(':root{--el-color-danger-light-3:#f78989;}'))),
    ]
    real_ep = element_plus_defs()
    if real_ep is None:
        print('  SKIP EP 变量表缺失（node_modules 未安装）⇒ 下面那条真数据判据没有跑')
    else:
        checks.append(('EP 变量表在真数据里给得出 light-3/danger 两档',
                       '--el-color-danger-light-3' in real_ep and '--el-color-danger' in real_ep))
    bad = [name for name, ok in checks if not ok]
    for name, ok in checks:
        print(('  ok   ' if ok else '  FAIL ') + name)
    print(f'SELFTEST PASS={len(checks) - len(bad)} FAIL={len(bad)}')
    return 1 if bad else 0


def assignments():
    """任何位置给某个名字赋过值的集合：组件级块里的 `--x: …`、以及 :style 里动态设的 `'--x': …`。

    这类名字不在根块里 ⇒ 不参与换肤，但**不是断链**。把它们算成缺陷会让闸门变成噪声机。
    """
    out = set()
    for ext in ('css', 'vue', 'js'):
        for path in glob.glob(os.path.join(SRC, '**', '*.' + ext), recursive=True):
            rel = os.path.relpath(path, UI).replace(os.sep, '/')
            if rel.endswith('.test.js'):
                continue
            text = strip_comments(io.open(path, encoding='utf-8', errors='replace').read())
            out.update(ASSIGN_RE.findall(text))
            out.update(STYLE_KEY_RE.findall(text))
    return out


def main():
    # Windows 控制台默认 GBK，✓ 这类字符会直接抛 UnicodeEncodeError（闸门得能被跑起来才行）
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    if '--selftest' in sys.argv:
        return selftest()
    defs = {}
    raw = {}
    for k, v in THEME_FILES.items():
        rel = os.path.relpath(v, UI).replace(os.sep, '/')
        text = read(rel)
        defs[k] = root_defs(text)
        raw[k] = len(ROOT_BLOCK_RE.findall(text))
    ep = element_plus_defs()
    if ep is None:
        print(f'证据无效：Element Plus 的变量表不在（{EP_CHALK}），定义面缺第三套真值 ⇒ 断链判据不可用')
        return 2
    defs['element-plus'] = ep
    used = usages()
    themes = ['dark', 'cyberpunk', 'sky']

    orphan = {n: p for n, p in used.items() if not any(n in defs[k] for k in defs)}
    assigned = assignments()
    # 断链 = 不在任何根块里、也没在任何别处赋过值、且至少有一处是裸用（无兜底）
    broken = {n: p for n, p in orphan.items()
              if n not in assigned and any(not fb for _, fb in p)}
    local = sorted(set(orphan) & assigned)
    hooked = sorted(n for n in orphan if n not in assigned and all(fb for _, fb in orphan[n]))
    theme_only = {n: p for n, p in used.items()
                  if n not in defs['global'] and any(n in defs[k] for k in themes)}
    shell_read = sorted(used, key=lambda n: -len(used[n]))
    not_overridden = {k: [n for n in shell_read if n not in defs[k]] for k in themes}
    # global 里写成 var() 的名字是别名，主题覆写的是它指向的那个真值名 ⇒ 不算缺口
    alias = {n for n, v in defs['global'].items() if v.startswith('var(')}

    if '--check' in sys.argv:
        new = sorted(set(broken) - ORPHAN_BASELINE)
        gone = sorted(ORPHAN_BASELINE - set(broken))
        print(f'断链令牌：现存 {len(broken)} / 基线 {len(ORPHAN_BASELINE)}，新增 {len(new)}，已清 {len(gone)}')
        print(f'（不计入：组件本地赋值 {len(local)} 个、只带兜底的钩子 {len(hooked)} 个；'
              f'定义面含 EP 自带变量表 {len(defs["element-plus"])} 个名字）')
        for n in new:
            print(f'  NEW  {n}  ← {broken[n][0][0]}')
        for n in gone:
            print(f'  GONE {n}  （已清掉，记得从 ORPHAN_BASELINE 删名，否则同名复发无人拦）')
        print(f'CHECK new={len(new)} verdict={"PASS" if not new else "FAIL"}')
        return 1 if new else 0

    lines = []
    emit = lines.append
    emit(f'# 主题令牌覆盖审计（自动算，勿手抄）')
    emit('')
    emit(f'- 消费点扫描：`src/**/*.(css|vue)`，共 **{len(used)}** 个不同令牌名被 `var()` 读到')
    emit(f'- 定义面：global `:root` **{len(defs["global"])}** 个；' +
         '；'.join(f'{k} **{len(defs[k])}** 个' for k in themes) +
         f'；element-plus **{len(defs["element-plus"])}** 个（node_modules 里 EP 自带的 `--el-*` 变量表，'
         '是我们别名链的终点，不计入换肤靶子）')
    emit(f'- 裸根块个数（定义面唯一的取值处）：' +
         '；'.join(f'{k} {raw[k]}' for k in ['global'] + themes))
    for k in ['global'] + themes:
        if raw[k] == 0:
            emit(f'- !! `{k}` 没有裸根块，定义面为空 → 它下面的缺口/孤儿数字全部不可信')
        elif raw[k] > 1:
            emit(f'- !! `{k}` 有 {raw[k]} 个裸根块，定义面按并集计（同名以先出现的为准）')
    emit('')
    emit('## 1. 断链令牌（不在任何根块、也没在别处赋值，且至少一处裸用）')
    emit('')
    emit('> 裸用 `var(--x)` 取不到值 ⇒ 该声明在计算值时失效：`color` 退成继承来的颜色、')
    emit('> `background` 退成 transparent。**带兜底** `var(--x, #fff)` 不是断链，而是"永远拿那个')
    emit('> 硬编码值、换肤不变色"——它同时是一处裸 hex（任务 #17），列在第 1b 节。')
    emit('')
    if not broken:
        emit('无。')
    else:
        emit('| 令牌 | 裸用处数 | 消费点合计 | 首处 |')
        emit('|------|---------|-----------|------|')
        for n in sorted(broken, key=lambda x: -sum(1 for _, fb in broken[x] if not fb)):
            bare = sum(1 for _, fb in broken[n] if not fb)
            emit(f'| `{n}` | {bare} | {len(broken[n])} | `{broken[n][0][0]}` |')
    emit('')
    emit(f'## 1b. 不算断链的两类（共 {len(local) + len(hooked)} 个，别拿去修）')
    emit('')
    emit(f'- 组件自己赋过值（含 `:style` 动态设的）：{len(local)} 个 —— ' +
         ', '.join(f'`{n}`' for n in local) + '')
    emit(f'- 每一处都写了兜底值的可选钩子：{len(hooked)} 个 —— ' +
         ', '.join(f'`{n}`' for n in hooked) + '')
    emit('')
    emit('## 2. 只在主题里定义、global `:root` 没有的名字（默认皮肤读不到）')
    emit('')
    emit(f'共 **{len(theme_only)}** 个，按消费点数排序（前 20）：')
    emit('')
    emit('| 令牌 | 消费点数 | dark | cyberpunk | sky | 首处 |')
    emit('|------|---------|------|-----------|-----|------|')
    for n in sorted(theme_only, key=lambda x: -len(theme_only[x]))[:20]:
        marks = ['✓' if n in defs[k] else '—' for k in themes]
        emit(f'| `{n}` | {len(theme_only[n])} | {marks[0]} | {marks[1]} | {marks[2]} | `{theme_only[n][0][0]}` |')
    emit('')
    emit('## 3. 各主题从未覆写、但被读到的名字')
    emit('')
    emit('> 只有**硬编码**的名字才是缺口（主题不改它，换肤后它就还是老值）。global 里写成')
    emit('> `var(--…)` 的是别名，跟着它指向的真值名走，不算缺口；两边都没有的名字见第 1/2 节。')
    emit('')
    for k in themes:
        names = not_overridden[k]
        hard = [n for n in names if n in defs['global'] and n not in alias]
        via = [n for n in names if n in alias]
        orph = [n for n in names if n not in defs['global']]
        emit(f'- **{k}**：{len(names)} 个 = 硬编码未覆写 **{len(hard)}**（'
             + ', '.join(f'`{n}`' for n in hard) + '）'
             + f' + 别名自动跟随 {len(via)} + global 里就没有的名字 {len(orph)}')
    emit('')
    emit('## 4. 主题明度自证（`--bg-*` 的平均相对亮度）')
    emit('')
    emit('| 皮肤 | --bg-* hex 个数 | 平均亮度 | 判读 |')
    emit('|------|----------------|---------|------|')
    for k in ['global'] + themes:
        lum, cnt = bg_lum(defs[k])
        if lum is None:
            emit(f'| {k} | 0 | — | 无可比的 6 位 hex |')
            continue
        tone = '浅色' if lum > 0.5 else '深色'
        emit(f'| {k} | {cnt} | {lum:.3f} | {tone} |')
    emit('')
    emit('> 判读口径：`--bg-*` 平均亮度 > 0.5 记浅色。若某皮肤判读为浅色而它**不覆写**外壳在读的')
    emit('> `--bg-primary/--bg-secondary/--bg-card`，页面底就仍然是另一套皮肤的深色——这正是名字')
    emit('> 分裂的症状，不是取值错误（第 3 节的缺口数即为此）。')

    emit('')
    emit('## 5. 附录：global 里硬编码、三套主题都没覆写的名字（换肤不会跟着变的靶子）')
    emit('')
    union = sorted({n for k in themes for n in not_overridden[k]
                    if n in defs['global'] and n not in alias},
                   key=lambda x: -len(used[x]))
    emit(f'共 **{len(union)}** 个：')
    emit('')
    emit('| 令牌 | 消费点数 | global 现值 | 未覆它的主题 |')
    emit('|------|---------|------------|-------------|')
    for n in union:
        who = ', '.join(k for k in themes if n not in defs[k])
        emit(f'| `{n}` | {len(used[n])} | `{defs["global"][n]}` | {who} |')

    text = '\n'.join(lines) + '\n'
    if '--md' in sys.argv:
        sys.stdout.buffer.write(text.encode('utf-8'))
    else:
        print(text)
    return 0


if __name__ == '__main__':
    sys.exit(main())
