#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""框架组合式 API（vue / vue-router / pinia 的具名导出）用而未绑探针——EP 反馈 API 探针的同族。

背景同 §11.13：vite 侧没有 auto-import ⇒ `ref / computed / watch / useRouter / defineStore` 这类名字
在脚本里是普通标识符，缺 import 就是运行时 ReferenceError。与 EP 那族不同：**这一族缺 import 会让页面
首屏就崩**，所以实测真语料是干净的（非测试文件 0 处）——本脚本因此按「零容忍不变量」钉，而不是按债务棘轮钉。

    --check / 默认   非测试源文件必须 0 处「用而未绑」；测试文件命中单列一行（已遮蔽正则字面量，仍只作参考），不改 rc
    --selftest       20 枚合成判据针 ＋ 6 个真实文件的"种缺陷"对照（探针失明了就得报红）

两条都是**只读**：不写任何源文件，改动全在内存里。
"""
import os
import re
import sys

FAMILIES = {
    'vue': ['ref', 'reactive', 'readonly', 'computed', 'watch', 'watchEffect', 'watchPostEffect',
            'onMounted', 'onBeforeMount', 'onUpdated', 'onBeforeUpdate', 'onUnmounted', 'onBeforeUnmount',
            'onActivated', 'onDeactivated', 'onErrorCaptured', 'nextTick', 'markRaw', 'markReadonly',
            'toRaw', 'unref', 'isRef', 'toRef', 'toRefs', 'provide', 'inject', 'defineComponent',
            'defineAsyncComponent', 'getCurrentInstance', 'useSlots', 'useAttrs', 'useTemplateRef',
            'createApp'],
    'vue-router': ['useRoute', 'useRouter', 'onBeforeRouteLeave', 'onBeforeRouteUpdate', 'createRouter',
                   'createWebHashHistory', 'createWebHistory', 'useLinkClass', 'useLinkProps'],
    'pinia': ['defineStore', 'storeToRefs', 'createPinia', 'setActivePinia', 'acceptHMRUpdate',
              'mapStores', 'mapState', 'mapActions', 'mapGetters'],
    # §11.14续 第三族：package.json 里其余运行时依赖的常见裸全局名。加进**零容忍**名单的前提是
    # 正则字面量遮蔽先落地（否则 `[A-Za-z]` 里的 `z` 会把干净仓判 FAIL），且名字先经实测为 0 处。
    'third-party': ['axios', 'echarts', 'mermaid', 'THREE', 'MarkdownIt', 'markdownit', 'Vex', 'VexFlow',
                    'Stave', 'StaveNote', 'Voice', 'Accidental', 'ForceGraph3D', 'ForceGraph2D', 'z',
                    'useStorage', 'useDark', 'useToggle', 'useEventListener', 'useWindowSize',
                    'useDebounceFn', 'useThrottleFn', 'useClipboard', 'useElementSize',
                    'useIntersectionObserver', 'useMagicKeys', 'useMouseInElement', 'useFetch',
                    'useLocalStorage', 'useMediaQuery', 'useNow', 'useTimeAgo',
                    'Edit', 'Delete', 'Search', 'Plus', 'Close', 'Check', 'Refresh', 'Setting',
                    'User', 'ArrowRight', 'Warning', 'InfoFilled', 'Loading'],
}
ALL_NAMES = sorted({n for v in FAMILIES.values() for n in v})
IMPORT_RE = re.compile(r'\bimport\b[^;\n]*?from\s*[\'"][^\'"]+[\'"]', re.S)
IMPORT_ML = re.compile(r'\bimport\s*\{[^}]*\}\s*from\s*[\'"][^\'"]+[\'"]', re.S)   # 跨行花括号式
CLAUSE_RE = re.compile(r'\{(?P<clause>[^}]*)\}\s*from', re.S)
DEFAULT_RE = re.compile(r'\bimport\s+(?!\(|\[|\{|\*)(?P<ns>[A-Za-z_$][\w$]*)(?:\s*(?:,|from\b|$))')
NS_RE = re.compile(r'\bimport\s*\*\s*as\s*(?P<ns>[A-Za-z_$][\w$]*)')
SCRIPT_RE = re.compile(r'<script[^>]*>(.*?)</script>', re.S)

# 种缺陷对照用的真实文件（只读，改动只在内存）：任一文件须能被"抹掉该模块的 import"打红。
# 后两条是 §11.14续 的**默认导入**形状（`import axios from 'axios'`）：它们同时是 DEFAULT_RE 那处修复的回归护栏。
FRAMEWORK_MOD = r'vue|vue-router|pinia'
CONTROL_FILES = [('views/graph/GraphView.vue', FRAMEWORK_MOD),
                 ('stores/alliance.store.js', FRAMEWORK_MOD),
                 ('composables/useKnowledgeBase.js', FRAMEWORK_MOD),
                 ('views/project/ProjectsView.vue', FRAMEWORK_MOD),
                 ('api/http.js', r'axios'),
                 ('utils/markdown.js', r'markdown-it')]

UI_ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..'))
SRC = os.path.join(UI_ROOT, 'src')


def blank(out, a, b):
    for k in range(a, b):
        if out[k] != '\n':
            out[k] = ' '


REGEX_OK_AFTER = set('=(,:[!&|?{};+-*%~^<>')
REGEX_OK_KEYWORDS = {'return', 'case', 'in', 'of', 'typeof', 'instanceof', 'do', 'else', 'yield',
                     'await', 'delete', 'void', 'new'}


def regex_starts(text, i):
    """`/` 处是不是正则字面量的开头：只看前一个非空白字符。`4 / 2`、`(a) / (b)` 的前一个是数字/`)`/标识符 ⇒ 除法。"""
    j = i - 1
    while j >= 0 and text[j] in ' \t':
        j -= 1
    if j < 0 or text[j] == '\n':
        return True
    c = text[j]
    if c in REGEX_OK_AFTER:
        return True
    if c.isalnum() or c in '_$)':
        k = j
        while k >= 0 and (text[k].isalnum() or text[k] in '_$'):
            k -= 1
        return text[k + 1:j + 1] in REGEX_OK_KEYWORDS
    return False


def regex_end(text, i):
    """从开 `/` 找到闭 `/`（认字符类 `[...]` 与转义），带 flags；**只在同一行内**，否则放弃遮蔽。"""
    j, n, incls = i + 1, len(text), False
    while j < n:
        c = text[j]
        if c == '\\':
            j += 2
            continue
        if c == '\n':
            return -1
        if incls:
            if c == ']':
                incls = False
        elif c == '[':
            incls = True
        elif c == '/':
            j += 1
            while j < n and text[j].isalpha():
                j += 1
            return j
        j += 1
    return -1


def mask_noise(text):
    """注释、字符串与**正则字面量**改写为空格；长度保持 ⇒ 行号可映射回原文。"""
    out = list(text)
    i, n = 0, len(text)
    while i < n:
        if text.startswith('/*', i) or text.startswith('<!--', i):
            close = '-->' if text.startswith('<!--', i) else '*/'
            end = text.find(close, i)
            end = n if end < 0 else end + len(close)
            blank(out, i, end)
            i = end
            continue
        if text.startswith('//', i):
            end = text.find('\n', i)
            end = n if end < 0 else end
            blank(out, i, end)
            i = end
            continue
        c = text[i]
        if c == '/' and regex_starts(text, i):
            end = regex_end(text, i)
            if end > 0:
                blank(out, i, end)
                i = end
                continue
        if c in '\'"`':
            q, j = c, i + 1
            while j < n:
                if text[j] == '\\':
                    j += 2
                    continue
                if text[j] == '\n' and q != '`':
                    break
                if text[j] == q:
                    j += 1
                    break
                j += 1
            blank(out, i, j)
            i = j
            continue
        i += 1
    return ''.join(out)


def script_mask(src, name):
    """.vue 只留 <script> 段（template/style 不参与判定），位置与原文一一对齐。"""
    if not name.endswith('.vue'):
        return src
    out = list(src)
    pos = 0
    for m in SCRIPT_RE.finditer(src):
        blank(out, pos, m.start(1))
        pos = m.end(1)
    blank(out, pos, len(out))
    return ''.join(out)


def split_specs(clause, alias_wins):
    """`{ a, b as c }` → 绑定名是 c（alias_wins=True）；声明解构 `{ x: local }` → 绑定名是 local。"""
    got = []
    for spec in clause.split(','):
        spec = spec.strip()
        if spec:
            got.append(re.split(r'\s+as\s+', spec)[-1].strip() if alias_wins else re.split(r'\s+as\s+', spec)[0].strip())
    return got


def bindings(clean, body):
    got, spans = set(), []
    for m in list(IMPORT_RE.finditer(body)) + list(IMPORT_ML.finditer(body)):
        a, b = m.start(), m.end()
        spans.append((a, b))
        seg = body[a:b]
        for c in CLAUSE_RE.finditer(seg):
            got.update(split_specs(c.group('clause'), True))
        for rx in (NS_RE, DEFAULT_RE):
            mm = rx.search(seg)
            if mm:
                got.add(mm.group('ns'))
    for m in re.finditer(r'(?:const|let|var)\s*\{([^}]*)\}\s*=', clean, re.S):
        got.update(split_specs(m.group(1), True))
    for m in re.finditer(r'(?:const|let|var)\s*\[([^\]]*)\]\s*=', clean):
        for spec in m.group(1).split(','):
            if spec.strip():
                got.add(spec.strip())
    for x in ALL_NAMES:
        if re.search(r'\b(?:const|let|var|function|class)\s+' + re.escape(x) + r'\b', clean):
            got.add(x)
    return got, spans


def split_top(text):
    """按逗号切形参表，但不在 `{…}`／`[…]` 内部切。"""
    parts, depth, cur = [], 0, []
    for ch in text:
        if ch in '{[(':
            depth += 1
        elif ch in '}])':
            depth -= 1
        if ch == ',' and depth == 0:
            parts.append(''.join(cur))
            cur = []
        else:
            cur.append(ch)
    parts.append(''.join(cur))
    return parts


def bound_in_pattern(spec):
    """解构形参 `{ a, b: local, c }` ⇒ 绑定 a/local/c；键名 `b` 后面紧跟冒号，不算绑定。"""
    out = []
    for m in re.finditer(r'[A-Za-z_$][\w$]*', spec):
        if spec[m.end():].lstrip().startswith(':'):
            continue
        out.append(m.group(0))
    return out


def params(clean):
    got = set()
    pats = (r'\(([^()]*)\)\s*(?:=>|\{)', r'\bfunction\s*[A-Za-z_$\w]*\s*\(([^)]*)\)')
    for p in pats:
        for m in re.finditer(p, clean):
            for spec in split_top(m.group(1)):
                spec = spec.strip()
                if not spec:
                    continue
                if spec[0] in '{[':
                    got.update(bound_in_pattern(spec))
                    continue
                head = re.split(r'[=:\s]', spec)[0].strip()
                if head:
                    got.add(head)
    return got


def uses(clean, x, skip):
    rx = r'(?<![\w$.])' + re.escape(x) + r'\b(?!\s*:)(?![\w$])'
    return [m.start() for m in re.finditer(rx, clean) if not any(a <= m.start() < b for a, b in skip)]


def analyze(src, name):
    body = script_mask(src, name)
    if not body.strip():
        return {}
    clean = mask_noise(body)
    bound, spans = bindings(clean, body)
    bound |= params(clean)
    miss = {}
    for x in ALL_NAMES:
        if x in bound:
            continue
        h = uses(clean, x, spans)
        if h:
            miss[x] = (len(h), clean[:h[0]].count('\n') + 1)
    return miss


def is_test(name):
    return re.search(r'\.(test|spec)\.(js|ts)$', name) is not None


def strip_imports_of(src, mod):
    """种缺陷用（只在内存里）：删掉所有 `... from '<mod>'` 整行（含跨行花括号式）。"""
    out = re.sub(r'^\s*import\s[^;\n]*?from\s*[\'"](?:%s)[\'"];?\s*\n' % mod, '', src, flags=re.M)
    return re.sub(r'^\s*import\s*\{[^}]*\}\s*from\s*[\'"](?:%s)[\'"];?\s*\n' % mod, '', out, flags=re.M | re.S)


def scan():
    app, tests = [], []
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames[:] = [d for d in dirnames if d not in ('node_modules', '.git')]
        for fn in sorted(filenames):
            if not fn.endswith(('.vue', '.js', '.ts')) or '_smoke' in fn:
                continue
            p = os.path.join(dirpath, fn)
            miss = analyze(open(p, encoding='utf-8', errors='replace').read(), fn)
            if miss:
                (tests if is_test(fn) else app).append((os.path.relpath(p, SRC).replace(os.sep, '/'), miss))
    app.sort(key=lambda r: -sum(v[0] for v in r[1].values()))
    return app, tests


def selftest():
    # 每枚针自带文件名：纯脚本片段必须用 .js，否则 .vue 的 script 掩码会把整段抹空 ⇒ 针空转
    cases = [
        ('imported ⇒ clean', "<script setup>\nimport { computed, ref } from 'vue'\nconst a = computed(() => 1)\nconst b = ref(0)\n</script>\n", set(), 'T.vue'),
        ('bare use ⇒ flagged', "<script setup>\nconst a = computed(() => 1)\nwatch(a, f)\n</script>\n", {'computed', 'watch'}, 'T.vue'),
        ('import 子句本身不算使用位点', "<script setup>\nimport { useRoute } from 'vue-router'\n</script>\n", set(), 'T.vue'),
        ('跨行花括号 import ⇒ clean，alias 绑定末端名', "<script setup>\nimport {\n  computed,\n  ref as r,\n} from 'vue'\nconst a = computed(() => 1)\nconst b = r(0)\n</script>\n", set(), 'T.vue'),
        ('对象键不算', "const o = { ref: 1, computed: 2 }\n", set(), 'T.js'),
        ('属性访问不算', "console.log(vm.ref, vm.computed, obj.useRouter)\n", set(), 'T.js'),
        ('字符串/注释不算', "const s = 'computed'\n// useRouter()\n/* watch */\n", set(), 'T.js'),
        ('局部同名声明 ⇒ clean', "const computed = () => 1\nconst x = computed()\n", set(), 'T.js'),
        ('形参同名 ⇒ clean', "function run({ storeToRefs }) { return storeToRefs(s) }\n", set(), 'T.js'),
        ('namespace import 兜住全部名字', "import * as Vue from 'vue'\nconst a = Vue.computed(() => 1)\n", set(), 'T.js'),
        # 形状取自真实语料（api/http.js:2 `import axios from 'axios'`、utils/markdown.js:3 同款）：
        # 默认导入的名字也是绑定，漏绑就会把"其实没缺陷"的文件判成缺陷
        ('default import 必须绑定该名字', "import markRaw from 'vue'\nconst a = markRaw(o)\n", set(), 'T.js'),
        # 正则字面量遮蔽（§11.15）：豁免要同时有"必须豁免得住"和"豁免不许顺手吞掉真缺陷"两枚针
        ('正则字符类里的 z 不算使用位点', "const re = /^[A-Z][A-Za-z]+$/\nconst k = s.replace(/_([a-z])/g, (_, c) => c.toUpperCase())\n", set(), 'T.js'),
        ('正则体被遮蔽但其后代码仍要看', "const m = /ref/.test(s)\nwatch(a, f)\n", {'watch'}, 'T.js'),
        ('除号不是正则：右边的裸名仍要判缺', "const q = total / z / scale\nwatch(q, f)\n", {'z', 'watch'}, 'T.js'),
        # 动态 import / require 绑定形状：这些名字本来就是局部声明绑定的，所以"clean"要靠变异体 K/M
        # 证明是"真的看见了且判为已绑"，不是"整段被看不见"（§11.15续）
        ('动态 import 解构 ⇒ clean', "const { computed } = await import('vue')\nconst a = computed(() => 1)\n", set(), 'T.js'),
        ('动态 import .then 解构形参 ⇒ clean', "import('vue').then(({ ref }) => { const a = ref(0) })\n", set(), 'T.js'),
        ('require 解构 ⇒ clean', "const { watch } = require('vue')\nwatch(a, f)\n", set(), 'T.js'),
        ('动态 import 命名空间：属性访问不算', "const mod = await import('vue')\nconst a = mod.computed(() => 1)\n", set(), 'T.js'),
        ('动态 import 命名空间兜不住裸名 ⇒ 仍要判缺', "const mod = await import('vue')\nconst a = computed(() => 1)\n", {'computed'}, 'T.js'),
        ('template 段不参与', "<template>{{ computed }}<router-view/></template>\n<script>const y = 1</script>\n", set(), 'T.vue'),
    ]
    bad = 0
    for label, src, want, name in cases:
        got = set(analyze(src, name))
        ok = got == want
        bad += 0 if ok else 1
        print('%s %s -> %s (expect %s)' % ('[OK]  ' if ok else '[FAIL]', label, sorted(got), sorted(want)))

    for rel, mod in CONTROL_FILES:
        p = os.path.join(SRC, rel.replace('/', os.sep))
        src = open(p, encoding='utf-8', errors='replace').read()
        clean_now = set(analyze(src, rel))
        gutted = set(analyze(strip_imports_of(src, mod), rel))
        ok = (not clean_now) and bool(gutted)
        bad += 0 if ok else 1
        print('%s 种缺陷对照 %s -> 原样=%s 抹掉 `%s` import 后=%s' % (
            '[OK]  ' if ok else '[FAIL]', rel, sorted(clean_now) or 'clean', mod,
            sorted(gutted) or 'CLEAN(探针失明!)'))
    print('FRAMEWORK-IMPORTS SELFTEST: %s (%d 合成针 + %d 种缺陷对照)' % (
        'PASS' if bad == 0 else 'FAIL', len(cases), len(CONTROL_FILES)))
    return 1 if bad else 0


def main(argv):
    if '--selftest' in argv:
        return selftest()
    app, tests = scan()
    n = sum(v[0] for _, m in app for v in m.values())
    print('非测试源文件「用而未绑」：%d 个 / %d 处（判据＝必须为 0）' % (len(app), n))
    for rel, miss in app:
        print('  %-58s %s' % (rel, {k: '%d@L%d' % v for k, v in miss.items()}))
    tn = sum(v[0] for _, m in tests for v in m.values())
    print('测试文件命中（单列，不计账；正则字面量已遮蔽，见 §11.15）：%d 个 / %d 处' % (len(tests), tn))
    if app:
        print('FRAMEWORK-IMPORTS: FAIL —— 这些名字缺 import 会在浏览器抛 ReferenceError')
        return 1
    print('verdict=PASS (探针：非测试源文件 0 处用而未绑)')
    return 0


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    sys.exit(main(sys.argv))
