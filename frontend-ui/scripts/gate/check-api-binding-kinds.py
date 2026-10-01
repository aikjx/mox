#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""`@/api` 的**函数型导出**被当对象用（`NAME.member(...)`）——零容忍门禁。

背景（§5.15 缺陷 ①）：`views/auth/Login.vue` 写的是 `import { login as authApi } from '@/api'`
再 `authApi.login({...})`，而 `login` 在 `auth.api.js:18` 是 `export function` ⇒ 每次点登录都抛
`TypeError: authApi.login is not a function`。**全量 vitest（66 文件 946 例全绿）看不见这件事**，
因为 api 层在测试里是 `vi.mock` 的桩；真机点击才暴露。本门禁把那个形状钉成常驻不变量。

判据：具名导入的绑定若在 `src/api/**` 里解析为 function / 箭头函数 / function 表达式，则该绑定是
**可调用值**，对它取属性再调用（`alias.member(…)`）就是运行时 TypeError ⇒ 命中即 FAIL。
与 `check-framework-imports.py` 同族（"首屏即崩"这一类，同样按零容忍钉而不是按债务棘轮钉）。

豁免（每条按名打印站点，且各有"必须豁免得住"＋"不许顺手吞掉真缺陷"两枚针）：
  E-shadow  文件里同名局部声明/形参/解构把导入遮蔽了（`stores/auth.store.js` 的 `refreshToken` 是 ref）
  E-mock    测试文件里的 `.mock*` 成员（`vi.mock` 后检查调用记录是正当用法）
  E-fnprop  `call`/`apply`/`bind`/`length`/`name`/`prototype` —— Function 自身的合法属性

    --check / 默认   函数型导出被当对象用必须 0 处（测试文件同样计入，这一族没有"已知债务"）
    --selftest       16 枚合成针（走真实导出表）＋真实文件的"种缺陷/撤遮蔽"对照
    --mutants        5 枚判据变异体，逐枚点名它打红了哪几格（打不红＝针没牙）

三条命令都是**只读**：不写任何源文件，改写全在内存里。
"""
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
UI_ROOT = os.path.normpath(os.path.join(HERE, '..', '..'))
SRC = os.path.join(UI_ROOT, 'src')
API_DIR = os.path.join(SRC, 'api')

CALLABLE, DATA, CLASS, UNKNOWN = 'callable', 'data', 'class', 'unknown'
FN_PROPS = {'call', 'apply', 'bind', 'length', 'name', 'prototype'}

_spec = importlib.util.spec_from_file_location(
    'framework_imports', os.path.join(HERE, 'check-framework-imports.py'))
_fi = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_fi)
mask_noise, script_mask, bindings, params = (_fi.mask_noise, _fi.script_mask, _fi.bindings, _fi.params)

RE_FN2 = re.compile(r'(?m)^(?P<ex>export\s+)?(?:async\s+)?function\s*\*?\s*(?P<name>[A-Za-z_$][\w$]*)')
RE_VAR2 = re.compile(r'(?m)^(?P<ex>export\s+)?(?:const|let|var)\s+(?P<name>[A-Za-z_$][\w$]*)\s*=(?P<init>[^\n]*)')
RE_CLASS2 = re.compile(r'(?m)^(?P<ex>export\s+)?class\s+(?P<name>[A-Za-z_$][\w$]*)')
RE_ALIAS_ONLY = re.compile(r'^\s*(?P<target>[A-Za-z_$][\w$]*)\s*,?\s*$')
RE_DEFAULT_IDENT = re.compile(r'^export\s+default\s+([A-Za-z_$][\w$]*)\s*$', re.M)
RE_DEFAULT_FN = re.compile(r'^export\s+default\s+(?:async\s+)?function', re.M)
RE_NAMED_FROM = re.compile(r'^export\s*\{(?P<clause>[^}]*)\}\s*from\s*[\'"](?P<mod>[^\'"]+)[\'"]', re.M | re.S)
RE_NAMED_LOCAL = re.compile(r'^export\s*\{(?P<clause>[^}]*)\}\s*(?:;|$)', re.M)
RE_STAR = re.compile(r'^export\s*\*\s*(?:as\s+[A-Za-z_$][\w$]*\s*)?from\s*[\'"](?P<mod>[^\'"]+)[\'"]', re.M)
RE_ARROWISH = re.compile(r'^\s*(?:async\s*)?(?:function\b|\([^)]*\)\s*=>|[A-Za-z_$][\w$]*\s*=>)')
IMPORT_NAMED = re.compile(r'\bimport\s*\{(?P<clause>[^}]*)\}\s*from\s*[\'"](?P<mod>[^\'"]+)[\'"]', re.S)
LOCAL_DECL = re.compile(r'\b(?:const|let|var|function|class)\s+([A-Za-z_$][\w$]*)')


DESTRUCT_OBJ = re.compile(r'\b(?:const|let|var)\s*\{(?P<pat>[^{}]*)\}\s*=', re.S)
DESTRUCT_ARR = re.compile(r'\b(?:const|let|var)\s*\[(?P<pat>[^\[\]]*)\]\s*=')


def parse_specs(clause):
    """`a, b as c` → [(a, a), (b, c)]。别名必须按**末端绑定名**查站点（缺陷 ① 就是别名形状），
    `_fi.split_specs` 的 alias_wins=False 只回 orig，用它会把别名整段丢掉。"""
    out = []
    for raw in clause.split(','):
        raw = raw.strip()
        if raw:
            parts = [p.strip() for p in re.split(r'\s+as\s+', raw)]
            out.append((parts[0], parts[-1]))
    return out


def resolve(mod, importer):
    """说明符 → src 内的真实文件；解析不到或不在 src 内返回 None。"""
    if mod.startswith('@/'):
        base = os.path.join(SRC, mod[2:].replace('/', os.sep))
    elif mod.startswith('.'):
        base = os.path.normpath(os.path.join(os.path.dirname(importer), mod.replace('/', os.sep)))
    else:
        return None
    if not os.path.normpath(base).startswith(SRC + os.sep):
        return None
    for cand in (base + '.js', base + '.mjs', base + '.ts', os.path.join(base, 'index.js')):
        if os.path.isfile(cand):
            return cand
    return None


def own_decls(text):
    """文件内每个声明 → 种类（导出的与未导出的都要，未导出的用于解 `export default http` 这类间接）。

    三种真实形状都必须落对，否则判据会**静默少判**（种类成 UNKNOWN 就不判）：
      `export const listArtifacts = getArtifacts`  ⇒ 别名，继承右端名字的种类；
      `export default http`（http.js:306）          ⇒ 右端是文件内**未导出**的 const；
      `export const getFoo = (p) => http.get(…)`    ⇒ 箭头函数＝可调用。
    """
    exported, decl = set(), {}

    def put(name, kind, is_ex):
        if is_ex:
            exported.add(name)
        decl.setdefault(name, kind)

    for m in RE_FN2.finditer(text):
        put(m.group('name'), CALLABLE, bool(m.group('ex')))
    for m in RE_CLASS2.finditer(text):
        put(m.group('name'), CLASS, bool(m.group('ex')))
    aliases = {}
    for m in RE_VAR2.finditer(text):
        init = m.group('init')
        ali = RE_ALIAS_ONLY.match(init)
        if ali:
            aliases[m.group('name')] = ali.group(1)
            put(m.group('name'), UNKNOWN, bool(m.group('ex')))
        else:
            put(m.group('name'), CALLABLE if RE_ARROWISH.match(init) else DATA, bool(m.group('ex')))
    for _ in range(3):
        for name, target in aliases.items():
            if decl.get(name) != CALLABLE and decl.get(target) == CALLABLE:
                decl[name] = CALLABLE
    default = DATA
    m = RE_DEFAULT_IDENT.search(text)
    if m:
        default = decl.get(m.group(1), UNKNOWN)
    elif RE_DEFAULT_FN.search(text):
        default = CALLABLE
    return {n: decl[n] for n in exported if n in decl}, default


class ExportTable:
    """src/api/** 的具名导出 → 种类；桶（`export * from` / `export {a as b} from`）顺着链解析。"""

    def __init__(self):
        self.cache = {}

    def of(self, path, stack=()):
        path = os.path.normpath(path)
        if path in self.cache:
            return self.cache[path]
        if path in stack:
            return {}
        raw = open(path, encoding='utf-8', errors='replace').read()
        text = mask_noise(script_mask(raw, path))
        kinds, default = own_decls(text)
        out = dict(kinds)
        out['default'] = default
        # re-export 的说明符在**字符串里** ⇒ 必须从未遮蔽的原文取（遮蔽后 `from '…'` 整体消失，
        # 桶就成了空表，判据会退化成"什么都不判"而照样全绿）。
        for m in RE_STAR.finditer(raw):
            tgt = resolve(m.group('mod'), path)
            if tgt and os.path.normpath(tgt).startswith(API_DIR + os.sep):
                for k, v in self.of(tgt, stack + (path,)).items():
                    if k != 'default':
                        out.setdefault(k, v)
        for m in RE_NAMED_FROM.finditer(raw):
            tgt = resolve(m.group('mod'), path)
            tk = self.of(tgt, stack + (path,)) if tgt else {}
            for orig, bound in parse_specs(m.group('clause')):
                out[bound] = tk.get(orig, UNKNOWN)
        for m in RE_NAMED_LOCAL.finditer(text):
            for orig, bound in parse_specs(m.group('clause')):
                if orig != bound:
                    out[bound] = kinds.get(orig, UNKNOWN)
        self.cache[path] = out
        return out

    def kind(self, mod_file, name):
        if not mod_file:
            return UNKNOWN
        return self.of(mod_file).get(name, UNKNOWN)


TABLE = ExportTable()
KINDS = TABLE.of(os.path.join(API_DIR, 'index.js'))
CALLABLE_N = sum(1 for v in KINDS.values() if v == CALLABLE)
# 分母守卫：桶链解析塌缩时表会退化成"谁的种类都不知道"⇒ 全局 clean，与"没有缺陷"同形。
if len(KINDS) < 200 or CALLABLE_N < 20:
    raise SystemExit('INVALID：api 导出表退化（%d 具名 / %d 函数型），判据已失去分母' % (len(KINDS), CALLABLE_N))


def analyze(src_text, path, rel):
    """→ (hits, exempt, stats)。hits: [(line, alias, orig, member, mod)]；exempt: {通道名: [站点]}；
    stats 记录判集规模（`specs`＝检过的 src/api 具名导入数），用于把"0 命中"和"没东西可判"区分开。"""
    name = os.path.basename(path)
    body = script_mask(src_text.replace('\r\n', '\n'), name)
    if not body.strip():
        return [], {}, {'specs': 0, 'callable_specs': 0}
    clean = mask_noise(body)
    _, spans = bindings(clean, body)
    # E-shadow 只看**局部**绑定（形参/局部声明/解构），绝不能把 import 绑定本身算进去
    # ——那样每个导入都会自我豁免，判集恒空（探针通电但永远看不见东西）。
    shadowed = set(params(clean))
    shadowed |= {m.group(1) for m in LOCAL_DECL.finditer(clean)}
    for rx in (DESTRUCT_OBJ, DESTRUCT_ARR):
        for m in rx.finditer(clean):
            shadowed |= {p for p in _fi.split_specs(m.group('pat'), True) if re.fullmatch(r'[A-Za-z_$][\w$]*', p)}
    hits = []
    exempt = {'shadow': [], 'mock': [], 'fnprop': []}
    stats = {'specs': 0, 'callable_specs': 0}
    for m in IMPORT_NAMED.finditer(body):
        mod_file = resolve(m.group('mod'), path)
        if not mod_file or not os.path.normpath(mod_file).startswith(API_DIR + os.sep):
            continue
        for orig, alias in parse_specs(m.group('clause')):
            if not re.fullmatch(r'[A-Za-z_$][\w$]*', orig or ''):
                continue
            stats['specs'] += 1
            if TABLE.kind(mod_file, orig) != CALLABLE:
                continue
            stats['callable_specs'] += 1
            if alias in shadowed:
                exempt['shadow'].append('%s:%s' % (rel, alias))
                continue
            rx = re.compile(r'(?<![\w$.])' + re.escape(alias) + r'\s*\.\s*(?P<member>[A-Za-z_$][\w$]*)')
            for u in rx.finditer(clean):
                if any(a <= u.start() < b for a, b in spans):
                    continue
                line = clean[:u.start()].count('\n') + 1
                member = u.group('member')
                if member in FN_PROPS:
                    exempt['fnprop'].append('%s:L%d:%s.%s' % (rel, line, alias, member))
                elif _fi.is_test(name) and member.startswith('mock'):
                    exempt['mock'].append('%s:L%d:%s.%s' % (rel, line, alias, member))
                else:
                    hits.append((line, alias, orig, member, m.group('mod')))
    return hits, exempt, stats


def scan():
    rows = []
    ex = {'shadow': [], 'mock': [], 'fnprop': []}
    stats = {'specs': 0, 'callable_specs': 0}
    files = 0
    for dirpath, dirnames, filenames in os.walk(SRC):
        dirnames[:] = [d for d in dirnames if d not in ('node_modules', '.git')]
        for fn in sorted(filenames):
            if not fn.endswith(('.vue', '.js', '.ts')) or '_smoke' in fn:
                continue
            p = os.path.join(dirpath, fn)
            files += 1
            rel = os.path.relpath(p, SRC).replace(os.sep, '/')
            hits, exempt, st = analyze(open(p, encoding='utf-8', errors='replace').read(), p, rel)
            for k in ex:
                ex[k].extend(exempt[k])
            for k in stats:
                stats[k] += st[k]
            rows.extend((rel,) + h for h in hits)
    rows.sort()
    return rows, ex, files, stats


# ---- 判据表：合成针走**真实导出表**（login/http/register… 都是 src/api 里的真名字） ----
NEEDLES = [
    ('函数导出取属性 ⇒ 命中', "import { login } from '@/api'\nlogin.check()\n", 1, set(), 'A.js'),
    # .vue 针必须自带 <script> 标签：script_mask 会把标签外的内容整段抹空 ⇒ 裸片段针是空转针（§31 同形）
    ('缺陷 ① 原形：别名 + 同名成员（.vue 的 script 段）',
     "<script setup>\nimport { login as authApi } from '@/api'\nawait authApi.login({})\n</script>\n", 1, set(), 'A.vue'),
    ('直接调用 ⇒ clean', "import { login } from '@/api'\nawait login({})\n", 0, set(), 'A.js'),
    ('对象型导出取属性 ⇒ clean（http 是 axios 实例）', "import { http } from '@/api'\nhttp.get('/x')\n", 0, set(), 'A.js'),
    ('别名导入的对象型导出取属性 ⇒ clean（M4 的反向见证）', "import { http as req } from '@/api'\nreq.get('/x')\n", 0, set(), 'A.js'),
    ('默认导出的对象取属性 ⇒ clean（auth.api.js 的 export default）', "import authApi from '@/api/auth.api'\nauthApi.login({})\n", 0, set(), 'A.js'),
    ('局部同名声明遮蔽 ⇒ 豁免 E-shadow', "import { refreshToken } from '@/api'\nconst refreshToken = ref('')\nrefreshToken.value = 'x'\n", 0, {'shadow'}, 'A.js'),
    ('测试里的 .mock 检查 ⇒ 豁免 E-mock', "import { register } from '@/api'\nexpect(register.mock.calls.length).toBe(1)\n", 0, {'mock'}, 'A.test.js'),
    ('测试里的真缺陷不许被 mock 豁免吞掉', "import { register } from '@/api'\nexpect(register.wasCalled).toBe(true)\n", 1, set(), 'A.test.js'),
    ('Function 自带属性 ⇒ 豁免 E-fnprop', "import { logout } from '@/api'\nlogout.call(null)\n", 0, {'fnprop'}, 'A.js'),
    ('跨行 import 子句也要看得到', "import {\n  mfaVerify as mfa,\n} from '@/api'\nmfa.state()\n", 1, set(), 'A.js'),
    ('子路径导入同样判（按需从模块导入是 api/index.js 的官方建议）', "import { getCurrentUser } from '@/api/auth.api'\ngetCurrentUser.foo()\n", 1, set(), 'A.js'),
    ('相对路径导入同样判', "import { logout } from '../../api/auth.api'\nlogout.bar()\n", 1, set(), 'A.js'),
    ('别名按末端绑定名查（裸 orig 取属性不算本站点）', "import { logout as bye } from '@/api'\nbye()\n", 0, set(), 'A.js'),
    ('namespace 导入取属性 ⇒ clean（那是真对象）', "import * as api from '@/api'\napi.foo()\n", 0, set(), 'A.js'),
    ('注释里的形状不算', "// import { login } from '@/api'\n// login.foo()\n", 0, set(), 'A.js'),
    ('template 段不参与', "<template><div @click=\"login.foo()\"/></template>\n<script>const q = 1</script>\n", 0, set(), 'A.vue'),
]

# 对照：(相对路径, [(old→new)…] 内存改写, 期望, 为什么)
CONTROLS = [
    ('views/auth/Login.vue', [('login as apiLogin', 'login as authApi'), ('apiLogin({', 'authApi.login({')],
     'hit', '还原缺陷 ①（HEAD 里的真实形状）'),
    ('stores/auth.store.js', [('const refreshToken = ref(', 'const refreshTokenShadow = ref(')],
     'hit', '撤 E-shadow 遮蔽：原样 clean 必须是因为看见了遮蔽'),
    ('stores/auth.store.js', [('await authApi.login({', 'await authApi.check()')],
     'clean', '局部对象 authApi 不是导入绑定 ⇒ 不该误判'),
]

PROBE_DIR = os.path.join(SRC, 'modules', '_kernel')

# 导出表本身也是观测量：种类解析错一个（别名/默认导出/箭头函数）判据就静默少判，
# 所以把四个真实名字的种类钉死，而不是只靠针的"clean"（clean 可能来自看不见）。
MAP_FACTS = [
    ('login', CALLABLE, 'auth.api.js:18 export function login（缺陷 ① 的当事名字）'),
    ('listArtifacts', CALLABLE, 'ai.api.js:31 export const listArtifacts = getArtifacts（别名要继承种类）'),
    ('getLlmPresets', CALLABLE, 'llm.api.js:7 export const getLlmPresets = getLlmProviderPresets'),
    ('http', DATA, 'http.js:306 export default http（axios 实例，取属性合法）'),
    ('ROLE_TEMPLATES', DATA, 'system.api.js 的对象型导出'),
]


def run_map_facts():
    bad = []
    unknown = [k for k, v in KINDS.items() if v == UNKNOWN]
    if len(unknown) > 20:
        bad.append('导出表 %d 个名字里有 %d 个种类未知 ⇒ 桶链/声明解析塌缩' % (len(KINDS), len(unknown)))
    print('  导出表 %d 个具名导出 / 函数型 %d / 未知 %d（未知上界 20）' % (
        len(KINDS), CALLABLE_N, len(unknown)))
    for name, want, why in MAP_FACTS:
        got = KINDS.get(name)
        if got != want:
            bad.append('%s 种类=%r（期望 %r，依据：%s）' % (name, got, want, why))
        print('  表事实 %s %-22s %-9s %s' % ('[OK]  ' if got == want else '[FAIL]', name, got, why))
    return bad


def run_needles():
    bad = []
    for label, src, want_hits, want_ex, name in NEEDLES:
        hits, exempt, _ = analyze(src, os.path.join(PROBE_DIR, name), name)
        got_ex = {k for k, v in exempt.items() if v}
        if len(hits) != want_hits or got_ex != want_ex:
            bad.append('%s -> hits=%d%s exempt=%s（期望 hits=%d exempt=%s）' % (
                label, len(hits), hits[:2], sorted(got_ex), want_hits, sorted(want_ex)))
    return bad


def run_controls():
    bad = []
    for rel, edits, want, why in CONTROLS:
        p = os.path.join(SRC, rel.replace('/', os.sep))
        src = open(p, encoding='utf-8', errors='replace').read()
        base, _, _ = analyze(src, p, rel)
        mut_src = src
        for old, new in edits:
            assert old in mut_src, '对照锚点在 %s 里找不到：%r' % (rel, old)
            mut_src = mut_src.replace(old, new, 1)
        mut, _, _ = analyze(mut_src, p, rel)
        ok = bool(mut) if want == 'hit' else not mut
        if not ok:
            bad.append('%s（%s）base=%d mut=%d 期望 %s' % (rel, why, len(base), len(mut), want))
        print('  对照 %s %-46s 原样=%d 改写后=%d（期望 %s）' % (
            '[OK]  ' if ok else '[FAIL]', rel + ' · ' + why, len(base), len(mut), want))
    return bad


def selftest():
    nbad = run_needles()
    for b in nbad:
        print('[FAIL] 针 %s' % b)
    print('合成针：%d 枚，失败 %d 枚' % (len(NEEDLES), len(nbad)))
    mbad = run_map_facts()
    cbad = run_controls()
    print('API-BINDING-KINDS SELFTEST: %s (%d 合成针 + %d 导出表事实 + %d 对照)' % (
        'PASS' if not (nbad or mbad or cbad) else 'FAIL', len(NEEDLES), len(MAP_FACTS), len(CONTROLS)))
    return 1 if (nbad or mbad or cbad) else 0


MUTANTS = [
    ('M1 过度判：对象型导出也当函数',
     'if TABLE.kind(mod_file, orig) != CALLABLE:',
     'if TABLE.kind(mod_file, orig) not in (CALLABLE, DATA):',
     '对象型导出取属性 ⇒ clean（http 是 axios 实例）'),
    ('M2 撤 E-shadow 遮蔽', 'if alias in shadowed:', 'if False and alias in shadowed:',
     '局部同名声明遮蔽 ⇒ 豁免 E-shadow'),
    ('M3 过度豁免 mock', "elif _fi.is_test(name) and member.startswith('mock'):", 'elif _fi.is_test(name):',
     '测试里的真缺陷不许被 mock 豁免吞掉'),
    ('M4 站点按 orig 找（别名丢形）',
     "rx = re.compile(r'(?<![\\w$.])' + re.escape(alias) + r'\\s*\\.\\s*(?P<member>[A-Za-z_$][\\w$]*)')",
     "rx = re.compile(r'(?<![\\w$.])' + re.escape(orig) + r'\\s*\\.\\s*(?P<member>[A-Za-z_$][\\w$]*)')",
     '缺陷 ① 原形：别名 + 同名成员'),
    ('M5 只认桶本身（放过子路径/相对导入）',
     'if not mod_file or not os.path.normpath(mod_file).startswith(API_DIR + os.sep):',
     "if not mod_file or os.path.normpath(mod_file) != os.path.normpath(os.path.join(API_DIR, 'index.js')):",
     "子路径导入同样判（按需从模块导入是 api/index.js 的官方建议）"),
]


def _code_only(src):
    """变异体锚点要在**去掉 MUTANTS 表本身**的代码里数：表里就抄着这些锚点原文，
    连表一起数会永远得到 2 次（自己的引文进了期望，§仪器看不见自己要测的结果）。"""
    a, b = src.index('\nMUTANTS = [\n'), src.index('\ndef mutants():\n')
    return src[:a] + src[b:]


def mutants():
    here = os.path.abspath(__file__)
    base_src = _code_only(open(here, encoding='utf-8').read())
    base_bad = run_needles()
    print('未变异基线：合成针失败 %d 枚' % len(base_bad))
    if base_bad:
        for b in base_bad:
            print('  %s' % b)
        print('INVALID：基线不干净，不测针')
        return 2
    bad = 0
    for label, old, new, expect in MUTANTS:
        n = base_src.count(old)
        if n != 1:
            print('[FAIL] %s：锚点在门禁自身源码里出现 %d 次（期望 1）' % (label, n))
            bad += 1
            continue
        ns = {'__name__': 'mutant_gate', '__file__': here}
        exec(compile(base_src.replace(old, new, 1), '<mutant>', 'exec'), ns)
        got = ns['run_needles']()
        named = [b for b in got if b.startswith(expect)]
        ok = bool(got) and bool(named)
        bad += 0 if ok else 1
        print('%s %s -> 打红 %d 格%s' % ('[OK]  ' if ok else '[FAIL]', label, len(got),
                                         ('，点名：%s' % expect) if ok else '（无人点名＝针没牙）'))
    print('API-BINDING-KINDS MUTANTS: %s (%d 枚)' % ('PASS' if bad == 0 else 'FAIL', len(MUTANTS)))
    return 1 if bad else 0


def main(argv):
    if '--selftest' in argv:
        return selftest()
    if '--mutants' in argv:
        return mutants()
    rows, ex, files, stats = scan()
    print('导出表：%d 个 api 模块 / %d 个具名导出 / 其中 %d 个是函数型（判据分子的上界）' % (
        len(TABLE.cache), len(KINDS), CALLABLE_N))
    print('判集规模：%d 个文件被扫 / 检出 %d 处 src/api 具名导入，其中函数型 %d 处' % (
        files, stats['specs'], stats['callable_specs']))
    if stats['callable_specs'] < 200:
        print('INVALID：函数型导入站点仅 %d 处（下界 200）⇒ 导入解析塌缩，"0 命中"不算证据' % stats['callable_specs'])
        return 2
    print('结论：%d 处「函数型导入绑定取属性」（判据＝必须为 0）' % len(rows))
    for k in ('shadow', 'mock', 'fnprop'):
        v = sorted(set(ex[k]))
        print('  豁免 E-%-8s %d 处%s' % (k, len(ex[k]), ('：' + ', '.join(v[:6])) if v else ''))
    for rel, line, alias, orig, member, mod in rows:
        print('  FAIL %s:L%d  `%s.%s(…)` —— `%s` 自 %r 导入，在 src/api 里是函数' % (rel, line, alias, member, orig, mod))
    if rows:
        print('API-BINDING-KINDS: FAIL —— 函数当对象用会在浏览器抛 TypeError（§5.15 缺陷 ① 同形）')
        return 1
    print('verdict=PASS（零容忍：函数型导出被当对象用 0 处）')
    return 0


if __name__ == '__main__':
    sys.stdout.reconfigure(encoding='utf-8')
    sys.exit(main(sys.argv))
