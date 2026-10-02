#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""挂载未在册端点的调用者分类器（API-SURFACE-PLAN-V0.1 §1.3 的常驻化）。

输入：`scripts/gate/check-api-surface.py --census` 的工件（只取 `mounted_not_in_table` 那一列）。
语料：`frontend-ui/src` 下的 `.js/.ts/.vue/.jsx/.tsx`，**测试档与非测试档分开**——
禁令台账里的合成样例证明的是"被明令禁止复活"，不是"有人还在调它"，混档会同时造假阳与假阴。

模式：
  --census PATH --json PATH   产账（工件自署 generated_at 与挂载侧权威）
  --check                     只验仪器自身的不变量（语料非空、分解自证闭合、两档嵌套、正对照开火），
                            债务条数按咨询口径打印；**不判"该不该对外"**（那需要裁决点 2/4 的判决）
  --selftest                  夹具＋变异体：每撤一条通道，对应该通道的判决必须变红

本脚本没有接 CI：判决口径要等"140 条里哪些本就不该对外"落定，否则门禁会把待裁决项判成缺陷。
"""
import argparse
import collections
import datetime
import io
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
FRONTEND_SRC = os.path.join(ROOT, 'frontend-ui', 'src')
EXTS = ('.js', '.ts', '.vue', '.jsx', '.tsx')
Q = "'`" + chr(34)
BASE_FORMS = ('/api', '/actuator')
TEST_MARK = ('.test.', '.spec.', '__tests__', '.stories.')
CLASSES = ('decl', 'httpish', 'any', 'bare')
JUDGE_CLASSES = ('decl', 'httpish', 'bare')

RE_BY_CLASS = {
    'decl': r"url\s*:\s*[" + Q + r"]",
    'httpish': r"\b((?:[\w$]*(?:[Hh]ttp|[Aa]xios)|request|service|api|instance))"
               r"\.(?:get|post|put|patch|delete)\(\s*[" + Q + r"]",
    'any': r"\b([\w$.]*)\.(?:get|post|put|patch|delete)\(\s*[" + Q + r"]",
    'bare': r"(?:^|[^\w.])((?:get|post|put|patch|del|fetch))\(\s*[" + Q + r"]",
}
# 参数段在前端写成 `${encodeURIComponent(id)}`：不含 / 与引号，一个"非斜杠非引号"段即可
SEG = r"[^/" + Q + r"]+"
CONST_RE = re.compile(r"(?:const|let|var)\s+([A-Z_][A-Z0-9_]*)\s*=\s*[" + Q + r"](/[A-Za-z0-9_\-/]*)[" + Q + r"]")
NL = chr(10)


def is_test(rel):
    b = rel.lower()
    return any(m in b for m in TEST_MARK)


def expand_constants(text):
    """`${BASE}/x` 型拼接就地展开；只认同文件常量，跨文件导入的常量是登记盲区。"""
    n = 0
    for m in CONST_RE.finditer(text):
        name, val = m.group(1), m.group(2)
        if len(val) <= 1:
            continue
        pat = '${' + name + '}'
        if pat in text:
            n += text.count(pat)
            text = text.replace(pat, val)
    return text, n


def forms_of(path, strip_base=True):
    out = [path]
    if strip_base:
        for base in BASE_FORMS:
            if path.startswith(base):
                out.append(path[len(base):] or '/')
    return sorted(set(out))


def parent_prefix(path):
    o = []
    for s in path.split('/'):
        if s == '{P}':
            break
        o.append(s)
    return '/'.join(o) or '/'


def form_pattern(form, tail_anchor=True):
    segs = [SEG if s == '{P}' else re.escape(s) for s in form.split('/')]
    joined = '/'.join(segs)
    # 尾锚是承重的：没有它，`/kb/documents/${id}/analyze` 会被算成 `/kb/documents/{P}` 的调用者
    return joined + (r"(?:[" + Q + r"]|$)" if tail_anchor else '')


def probe(corpus, path, tier='endpoint', strip_base=True, tail_anchor=True):
    cnt = {k: 0 for k in CLASSES}
    files, evid, forms = [], [], []
    for f in forms_of(path, strip_base):
        if tier == 'parent':
            lit = re.escape(parent_prefix(f))
            rx = {k: re.compile(RE_BY_CLASS[k] + lit) for k in CLASSES}
            gate = parent_prefix(f)
        else:
            rx = {k: re.compile(RE_BY_CLASS[k] + form_pattern(f, tail_anchor)) for k in CLASSES}
            gate = f.split('{P}')[0].rstrip('/')
        for cf, t in corpus.items():
            if gate and gate not in t:
                continue
            lines = t.split(NL)
            hit_here = False
            for k in CLASSES:
                for m in rx[k].finditer(t):
                    cnt[k] += 1
                    hit_here = True
                    if len(evid) < 3:
                        ln = t[:m.start()].count(NL)
                        evid.append('[' + k + '] ' + cf + ':' + str(ln + 1) + ' ' + lines[ln].strip()[:130])
            if hit_here:
                files.append(cf)
                forms.append(f)
    return cnt, sorted(set(files)), sorted(set(forms)), evid


def is_live(hits, judge=JUDGE_CLASSES):
    return bool(any(hits[k] for k in judge))


def classify(paths, src_txt, test_txt, judge=JUDGE_CLASSES, strip_base=True,
             tail_anchor=True, tier='endpoint', merge_test=False):
    rows = []
    for p in paths:
        ce, fe, _, ee = probe(src_txt, p, tier, strip_base, tail_anchor)
        cp, fp, _, ep = probe(src_txt, p, 'parent', strip_base, tail_anchor)
        ct, ft, _, et = probe(test_txt, p, tier, strip_base, tail_anchor)
        corpus_live = dict(ce)
        if merge_test:
            for k in CLASSES:
                corpus_live[k] += ct[k]
        rows.append({
            'path': p,
            'parent_prefix': parent_prefix(p),
            'endpoint_hits': corpus_live, 'endpoint_files': fe, 'endpoint_evidence': ee,
            'parent_hits': cp, 'parent_files': fp, 'parent_evidence': ep[:1],
            'test_endpoint_hits': ct, 'test_endpoint_files': ft, 'test_endpoint_evidence': et[:2],
        })
    ep_live = [r for r in rows if is_live(r['endpoint_hits'], judge)]
    pa_live = [r for r in rows if is_live(r['parent_hits'], judge)]
    tonly = [r['path'] for r in rows
             if not is_live(r['endpoint_hits'], judge) and not is_live(r['parent_hits'], judge)
             and is_live(r['test_endpoint_hits'], judge)]
    dom = collections.Counter()
    dom_ep = collections.Counter()
    dom_pa = collections.Counter()
    for r in rows:
        seg = '/'.join(r['path'].split('/')[1:3])
        dom[seg] += 1
        if is_live(r['endpoint_hits'], judge):
            dom_ep[seg] += 1
        if is_live(r['parent_hits'], judge):
            dom_pa[seg] += 1
    return {
        'rows': rows,
        'ep_live': [r['path'] for r in ep_live],
        'pa_live': [r['path'] for r in pa_live],
        'parent_only': [r['path'] for r in pa_live if not is_live(r['endpoint_hits'], judge)],
        'test_only': tonly,
        'by_domain': {k: [dom[k], dom_ep[k], dom_pa[k]] for k in sorted(dom)},
    }


def invariants(paths, res, src_txt, test_txt):
    """仪器自身的不变量；返回 [(名字, 是否成立, 详情)]，分母一律印实测不硬钉。

    每条都必须能被"另一列读数"证伪：不许出现 `X + (N-X) == N` 这种永真的自证格。
    """
    out = []
    out.append(('语料两档都非空', bool(src_txt) and bool(test_txt),
                'src %d / test %d' % (len(src_txt), len(test_txt))))
    known = [k for k in ('frontend-ui/src/api/auth.api.js', 'frontend-ui/src/api/sso.api.js')
             if k not in src_txt]
    out.append(('已知 api 层源在语料里', not known, '缺 %s' % (known or '-')))
    tot = sum(v[0] for v in res['by_domain'].values())
    out.append(('按域分解自证闭合', tot == len(res['rows']),
                '逐域总数相加 %d 配 行数 %d' % (tot, len(res['rows']))))
    live_by_dom = sum(v[1] for v in res['by_domain'].values())
    out.append(('按域 LIVE 列相加等于 LIVE 行数', live_by_dom == len(res['ep_live']),
                '逐域 LIVE 相加 %d 配 ep_live %d' % (live_by_dom, len(res['ep_live']))))
    parent_by_dom = sum(v[2] for v in res['by_domain'].values())
    out.append(('按域 parent 列相加等于 parent 行数', parent_by_dom == len(res['pa_live']),
                '逐域 parent 相加 %d 配 pa_live %d' % (parent_by_dom, len(res['pa_live']))))
    viol = [r['path'] for r in res['rows'] if not set(r['endpoint_files']) <= set(r['parent_files'])]
    out.append(('调用者文件集合逐行嵌套（两次独立探针交叉核对）', not viol,
                '逐行 endpoint_files ⊆ parent_files，违例 %s；parent %d − endpoint %d ＝ parent_only %d 条 %s' % (
                    viol or '-', len(res['pa_live']), len(res['ep_live']),
                    len(res['parent_only']), res['parent_only'] or '-')))
    out.append(('test_only 与源档判决不相容',
                not (set(res['test_only']) & set(res['ep_live'])),
                'test_only %d 条，与 ep_live 交集 %s' % (
                    len(res['test_only']), sorted(set(res['test_only']) & set(res['ep_live'])) or '-')))
    out.append(('输入清单与产出行数同量', len(paths) == len(res['rows']),
                '输入 %d 配 输出 %d' % (len(paths), len(res['rows']))))
    return out


def positive_control(src_txt):
    c, _f, _m, _e = probe(src_txt, '/api/auth/login', 'endpoint')
    return c, is_live(c)


def cmd_check(args):
    art = json.load(io.open(args.census, encoding='utf-8'))
    paths = art['mounted_not_in_table']
    src_txt, test_txt, exp = walk_corpus_split(FRONTEND_SRC, expand=not args.no_expand)
    res = classify(paths, src_txt, test_txt, tier=args.tier)
    ctrl, fired = positive_control(src_txt)
    print('挂载侧权威 = %s（generated_at %s）' % (args.census, art.get('generated_at', '未署名')))
    print('语料 src/test = %d/%d 个文件，常量展开 src %d／test %d 处'
          % (len(src_txt), len(test_txt), exp['src'], exp['test']))
    print('正对照 /api/auth/login =', ctrl, '开火' if fired else '未开火')
    bad = 0
    for name, ok, detail in invariants(paths, res, src_txt, test_txt):
        print('  %-28s %s  %s' % (name, 'OK' if ok else 'FAIL', detail))
        bad += 0 if ok else 1
    if not fired:
        bad += 1
        print('  正对照未开火 ⇒ 仪器坏，不是语料空')
    print('判决（咨询口径，不改退出码）：endpoint LIVE %d／parent LIVE %d／parent-only 假阳 %d／'
          '仅测试档合成样例 %d／零证据 %d'
          % (len(res['ep_live']), len(res['pa_live']), len(res['parent_only']),
             len(res['test_only']), len(res['rows']) - len(res['ep_live'])))
    if args.show_lists:
        print('  PARENT-ONLY', res['parent_only'])
        print('  TEST-ONLY', res['test_only'])
    print('CHECK %s（仪器不变量失败 %d 项；债务判决未开：等裁决点 2/4）'
          % ('PASS' if bad == 0 else 'FAIL', bad))
    return 0 if bad == 0 else 1


def walk_corpus_split(root_dir, expand=True):
    src_txt, test_txt = {}, {}
    exp = {'src': 0, 'test': 0}
    for dp, _dirs, fns in os.walk(root_dir):
        if 'node_modules' in dp:
            continue
        for f in fns:
            if not f.endswith(EXTS):
                continue
            ap = os.path.join(dp, f)
            rel = os.path.relpath(ap, ROOT).replace(os.sep, '/')
            raw = io.open(ap, encoding='utf-8', errors='replace').read()
            txt, n = expand_constants(raw) if expand else (raw, 0)
            if is_test(rel):
                test_txt[rel] = txt
                exp['test'] += n
            else:
                src_txt[rel] = txt
                exp['src'] += n
    return src_txt, test_txt, exp


def cmd_json(args):
    art = json.load(io.open(args.census, encoding='utf-8'))
    paths = art['mounted_not_in_table']
    src_txt, test_txt, exp = walk_corpus_split(FRONTEND_SRC, expand=not args.no_expand)
    res = classify(paths, src_txt, test_txt)
    now = datetime.datetime.now().astimezone().strftime('%Y-%m-%d %H:%M:%S%z')
    inv = invariants(paths, res, src_txt, test_txt)
    bad = [n for n, ok, _d in inv if not ok]
    if bad:
        print('拒绝产出：仪器不变量失败', bad)
        return 1
    out = {
        'generated_at': now,
        'instrument': 'scripts/gate/classify-unregistered-endpoints.py',
        'revision': 7,
        'lineage': 'rev1–rev6 为会话内 scratch 驱动（未落库＝没牙，缺陷登记在 rev6 工件 revisions 键）；'
                   'rev7 起常驻 scripts/gate/，补回 rev6 丢的 baseurl_relative_rule 键，'
                   '并撤掉一条永真的自证不变量（X+(N−X)==N）换成两次独立探针的文件集交叉核对',
        'authority_for_mounted_side': os.path.relpath(args.census, ROOT).replace(os.sep, '/'),
        'baseurl_relative_rule': 'http.js baseURL=/api、actuator 实例 baseURL=/actuator，'
                                 '源字面量不带该前缀；两种形态都探',
        '语料分档规则': '测试档判据：文件名含 ' + '／'.join(TEST_MARK) + '；判决只取源档，测试档单列 test_only',
        '判据正则': dict(RE_BY_CLASS),
        '判决取哪几档': list(JUDGE_CLASSES),
        'endpoint_pattern_样例': {'form': '/kb/documents/{P}/shares',
                                  'regex': form_pattern('/kb/documents/{P}/shares')},
        'instrument_invariants_passed': len(inv),
        'mounted_not_in_table_len': len(res['rows']),
        'corpus_src_files': len(src_txt),
        'corpus_test_files': len(test_txt),
        'constant_expansions_src': exp['src'],
        'constant_expansions_test': exp['test'],
        'live_endpoint_count': len(res['ep_live']),
        'live_parent_count': len(res['pa_live']),
        'parent_only_paths': res['parent_only'],
        'test_only_paths': res['test_only'],
        'zero_live_endpoint': len(res['rows']) - len(res['ep_live']),
        'by_domain': res['by_domain'],
        'rows': res['rows'],
    }
    with io.open(args.json, 'w', encoding='utf-8', newline='') as fh:
        json.dump(out, fh, ensure_ascii=False, indent=2)
    print('LIVE endpoint %d／parent %d／parent-only %d／test-only %d／零证据 %d（总 %d）'
          % (out['live_endpoint_count'], out['live_parent_count'], len(res['parent_only']),
             len(res['test_only']), out['zero_live_endpoint'], out['mounted_not_in_table_len']))
    print('wrote', args.json, '（自署 generated_at = %s）' % now)
    return 0


# ------------------------------------------------------------------ selftest

FIXTURE_SRC = {
    'frontend-ui/src/api/auth.api.js':
        "export function login(d) { return request({ url: '/auth/login', method: 'post', data: d }) }\n"
        "export function mfaVerify(d) { return request({ url: '/auth/mfa/verify', method: 'post', data: d }) }\n",
    'frontend-ui/src/api/sso.api.js':
        "const BASE = '/enterprise/sso'\n"
        "export const getP = () => http.get(`${BASE}/protocols`)\n"
        "export const ssoLogin = (p) => http.post(`${BASE}/login`, p)\n",
    'frontend-ui/src/api/actuator.api.js':
        "export const enableApi = (id) => actuatorHttp.post(`/api/${encodeURIComponent(id)}/enable`)\n",
    'frontend-ui/src/views/Doc.vue':
        "<script setup>\n"
        "export default { methods: {\n"
        "  analyze(id) { return http.post(`/kb/documents/${id}/analyze`) },\n"
        "  open(id) { return http.get(`/kb/documents/${id}`) },\n"
        "  roles(id) { return http.get(`/system/user/${id}/roles`) },\n"
        "  logs(id) { return http.get(`/alliance/tasks/${id}/logs`) },\n"
        "} }\n"
        "</script>\n",
    'frontend-ui/src/store/kv.js':
        "const kv = new Map()\n"
        "export const peek = () => kv.get('/misc/kv')\n"
        "export const grab = () => fetch('/misc/bare-call')\n",
}
FIXTURE_TEST = {
    'frontend-ui/src/modules/x/contract/forbidden-revival.test.js':
        "const planted = [\n  \"http.post('/api/ai/expert-chat', body)\",\n];\n",
}
FIXTURE_PATHS = [
    '/api/auth/login',
    '/api/auth/mfa/verify',
    '/api/enterprise/sso/protocols',
    '/api/enterprise/sso/login',
    '/actuator/api/{P}/enable',
    '/api/kb/documents/{P}/shares',
    '/api/kb/documents/{P}/analyze',
    '/api/kb/documents/{P}',
    '/api/system/user/{P}/depts',
    '/api/system/user/{P}/roles',
    '/api/alliance/tasks/{P}/qa',
    '/api/alliance/tasks/{P}',
    '/api/ai/expert-chat',
    '/api/misc/kv',
    '/api/misc/bare-call',
    '/api/nope/never-mounted-anywhere',
]


def _expand(d):
    """夹具必须走真通道（walk_corpus_split 里调的就是这个函数），否则模板拼接永不展开。"""
    return {k: expand_constants(v)[0] for k, v in d.items()}


def _base():
    return classify(FIXTURE_PATHS, _expand(FIXTURE_SRC), dict(FIXTURE_TEST))


def _flip(res, path):
    return path in res['ep_live']


class Case(object):
    def __init__(self, cid, kind, label, fn):
        self.cid, self.kind, self.label, self.fn = cid, kind, label, fn


def _cases():
    def regex_shape():
        fails, detail = [], []
        rx_cases = [
            ("url: '/auth/login',", '/auth/login', {'decl': 1, 'httpish': 0, 'any': 0, 'bare': 0}),
            ("http.post('/api/ai/expert-chat', body)", '/api/ai/expert-chat',
             {'decl': 0, 'httpish': 1, 'any': 1, 'bare': 0}),
            ("const WHITE_LIST = ['/login', '/register']", '/login',
             {'decl': 0, 'httpish': 0, 'any': 0, 'bare': 0}),
            ("router.push('/login')", '/login', {'decl': 0, 'httpish': 0, 'any': 0, 'bare': 0}),
            ("kv.get('/misc/kv')", '/misc/kv', {'decl': 0, 'httpish': 0, 'any': 1, 'bare': 0}),
            ("fetch('/misc/bare-call')", '/misc/bare-call', {'decl': 0, 'httpish': 0, 'any': 0, 'bare': 1}),
            # rev6 的针洞：bare 类的字符类被写成 `[^\w.]]`（多一枚字面 ]），
            # 于是只有位于字符串 0 号的裸调用才开火；这枚针在修好前必红。
            ("const f = () => fetch('/misc/bare-call')", '/misc/bare-call',
             {'decl': 0, 'httpish': 0, 'any': 0, 'bare': 1}),
            ("service.get('/enterprise/config/items')", '/enterprise/config/items',
             {'decl': 0, 'httpish': 1, 'any': 1, 'bare': 0}),
            ("url: `/alliance/tasks/${id}/qa`", '/alliance/tasks', {'decl': 1, 'httpish': 0, 'any': 0, 'bare': 0}),
        ]
        for text, lit, want in rx_cases:
            got = {k: len(re.findall(RE_BY_CLASS[k] + re.escape(lit), text)) for k in CLASSES}
            if got != want:
                fails.append(text[:28])
                detail.append('%s got %s want %s' % (text[:28], got, want))
        return (not fails), ('%d 例形状，不符 %s %s' % (len(rx_cases), len(fails), ';'.join(detail[:2])))

    def baseline():
        res = _base()
        want_live = {'/api/auth/login', '/api/auth/mfa/verify', '/api/enterprise/sso/protocols',
                     '/api/enterprise/sso/login', '/actuator/api/{P}/enable', '/api/misc/bare-call',
                     '/api/kb/documents/{P}', '/api/kb/documents/{P}/analyze',
                     '/api/system/user/{P}/roles'}
        want_dead = {'/api/kb/documents/{P}/shares', '/api/system/user/{P}/depts',
                     '/api/alliance/tasks/{P}/qa', '/api/alliance/tasks/{P}',
                     '/api/ai/expert-chat', '/api/misc/kv', '/api/nope/never-mounted-anywhere'}
        got = set(res['ep_live'])
        miss, extra = sorted(want_live - got), sorted(got - want_live)
        uncovered = sorted(set(FIXTURE_PATHS) - (want_live | want_dead))
        detail = 'LIVE %d 条（期望 %d），漏 %s 多 %s，期望表未覆盖 %s' % (
            len(got), len(want_live), miss or '-', extra or '-', uncovered or '-')
        return (not miss and not extra and not uncovered), detail

    def nested():
        res = _base()
        want_po = {'/api/kb/documents/{P}/shares', '/api/system/user/{P}/depts',
                   '/api/alliance/tasks/{P}/qa', '/api/alliance/tasks/{P}'}
        ok = set(res['ep_live']) <= set(res['pa_live']) and set(res['parent_only']) == want_po
        return ok, 'endpoint %d ⊆ parent %d；parent_only 恰点名这 4 条假阳，实得 %s' % (
            len(res['ep_live']), len(res['pa_live']), res['parent_only'])

    def domain_closure():
        res = _base()
        tot = sum(v[0] for v in res['by_domain'].values())
        return tot == len(res['rows']), '逐域相加 %d 配 总数 %d' % (tot, len(res['rows']))

    def no_anchor_blocks_parent():
        res = _base()
        row = [r for r in res['rows'] if r['path'] == '/api/alliance/tasks/{P}'][0]
        return (not is_live(row['endpoint_hits'])), '尾锚生效时 `/alliance/tasks/${id}/logs` 不得算列表端点的调用者：%s' % row['endpoint_hits']

    def fixture_closure():
        return (len(FIXTURE_PATHS) == len(set(FIXTURE_PATHS))), '夹具路径唯一（%d 条，重复会让 ep_live 顶掉彼此的判决）' % len(FIXTURE_PATHS)

    def m_merge_test_corpus():
        res = classify(FIXTURE_PATHS, _expand(FIXTURE_SRC), dict(FIXTURE_TEST), merge_test=True)
        return _flip(res, '/api/ai/expert-chat'), '合档 ⇒ expert-chat 转 LIVE（它今天只在禁令台账里）'

    def m_no_base_form():
        res = classify(FIXTURE_PATHS, _expand(FIXTURE_SRC), dict(FIXTURE_TEST), strip_base=False)
        gone = [p for p in ('/api/auth/login', '/api/auth/mfa/verify', '/api/enterprise/sso/protocols',
                            '/actuator/api/{P}/enable') if not _flip(res, p)]
        want = 4
        return len(gone) == want, '撤 baseURL 双形态 ⇒ 掉回零命中的条数 %d（期望 %d）%s' % (
            len(gone), want, gone or '-')

    def m_no_expand():
        no = classify(FIXTURE_PATHS, dict(FIXTURE_SRC), dict(FIXTURE_TEST))
        yes = classify(FIXTURE_PATHS, _expand(FIXTURE_SRC), dict(FIXTURE_TEST))
        ok = (not _flip(no, '/api/enterprise/sso/protocols')) and _flip(yes, '/api/enterprise/sso/protocols')
        return ok, '常量展开是判决性的：展开 ⇒ LIVE，不展开 ⇒ 零命中'

    def m_parent_tier():
        res = classify(FIXTURE_PATHS, _expand(FIXTURE_SRC), dict(FIXTURE_TEST), tier='parent')
        ok = (not res['parent_only']) and _flip(res, '/api/kb/documents/{P}/shares') \
            and _flip(res, '/api/alliance/tasks/{P}/qa')
        return ok, '退回父前缀判决 ⇒ 4 条假阳被静默收下（parent_only 变空，LIVE %d 对基线 9）：%s' % (
            len(res['ep_live']), res['parent_only'])

    def m_no_tail_anchor():
        res = classify(FIXTURE_PATHS, _expand(FIXTURE_SRC), dict(FIXTURE_TEST), tail_anchor=False)
        ok = _flip(res, '/api/alliance/tasks/{P}')
        return ok, '撤尾锚 ⇒ `/alliance/tasks/${id}/logs` 的调用者被算到列表端点头上'

    def m_any_in_judge():
        res = classify(FIXTURE_PATHS, _expand(FIXTURE_SRC), dict(FIXTURE_TEST), judge=CLASSES)
        ok = _flip(res, '/api/misc/kv')
        return ok, '把 any 档放进判决 ⇒ Map 取值算成调用者（假阳）'

    def probe_synthetic():
        corpus = {'a.js': "export default { url: '/auth/login', }\n", 'b.js': "const x = 1\n",
                  'c.js': "http.get('/auth/login')"}
        c, files, _f, _e = probe(corpus, '/auth/login', 'endpoint')
        ok = (c['decl'] == 1 and c['httpish'] == 1 and c['bare'] == 0 and files == ['a.js', 'c.js'])
        c2, f2, _f2, _e2 = probe(corpus, '/nope/not/there', 'endpoint')
        ok = ok and sum(c2.values()) == 0 and not f2
        return ok, 'probe() 逐文件计数与缺席字面量全 0：%s %s' % (c, files)

    def expansion_count():
        t, n = expand_constants("const BASE = '/enterprise/sso'\n"
                                "const g = () => http.get(`${BASE}/protocols`)\n"
                                "const h = () => http.post(`${BASE}/login`, p)\n")
        ok = n == 2 and '/enterprise/sso/protocols' in t and '/enterprise/sso/login' in t
        return ok, '展开次数 %d（期望 2）' % n

    def expansion_rejects_short():
        _t, n = expand_constants("const P = '/'\nexport const x = () => http.get(`${P}/api`)\n")
        return n == 0, '单段常量不许当 base（会把整棵路径挪走）：展开 %d 处' % n

    return [
        Case('T1', '正对照', '四类正则的形状逐例相符', regex_shape),
        Case('T2', '正对照', '基线判决恰是那 9 条 LIVE', baseline),
        Case('T3', '不变量', '端点档 ⊆ 父前缀档且 parent_only 恰点名 4 条假阳', nested),
        Case('T4', '不变量', '按域分解自证闭合', domain_closure),
        Case('T5', '不变量', '尾锚挡住"父资源调用者冒充子端点"', no_anchor_blocks_parent),
        Case('T6', '正对照', 'probe() 对合成语料逐文件计数＋缺席字面量判 0', probe_synthetic),
        Case('T7', '正对照', '常量展开次数现推', expansion_count),
        Case('T8', '不变量', '单段常量拒绝当 base', expansion_rejects_short),
        Case('T9', '不变量', '夹具路径唯一', fixture_closure),
        Case('M1', '变异体', '合档语料 ⇒ 禁令合成样例转 LIVE', m_merge_test_corpus),
        Case('M2', '变异体', '撤 baseURL 双形态 ⇒ auth/sso 全掉', m_no_base_form),
        Case('M3', '变异体', '撤常量展开 ⇒ sso 模板拼接判零命中', m_no_expand),
        Case('M4', '变异体', '退回父前缀判决 ⇒ 假阳被静默收下', m_parent_tier),
        Case('M5', '变异体', '撤尾锚 ⇒ 邻近子端点算成调用者', m_no_tail_anchor),
        Case('M6', '变异体', '把 any 放进判决 ⇒ Map 取值算成调用', m_any_in_judge),
    ]


def cmd_selftest(_args):
    fails = []
    res_by_case = {}
    for case in _cases():
        try:
            ok, detail = case.fn()
        except Exception as exc:
            ok, detail = False, '抛异常 %r' % (exc,)
        res_by_case[case.cid] = ok
        if not ok:
            fails.append(case.cid)
        print('%s %s[%s] %s  %s' % ('PASS' if ok else 'FAIL', case.cid, case.kind, case.label, detail))
    if set(res_by_case.values()) != {True}:
        missing = [c.cid for c in _cases() if c.kind == '变异体' and not res_by_case[c.cid]]
        print('变异体未开火 ⇒ 针坏，不是账稳：', missing or '-')
    print('SELFTEST 总 %d 例，FAIL %d 例' % (len(res_by_case), len(fails)))
    return 0 if not fails else 1


def main():
    ap = argparse.ArgumentParser(description='挂载未在册端点的调用者分类器')
    ap.add_argument('--census', help='check-api-surface.py --census 的 JSON 工件')
    ap.add_argument('--json', dest='json', help='产账路径')
    ap.add_argument('--check', action='store_true', help='只验仪器不变量，债务按咨询口径打印')
    ap.add_argument('--selftest', action='store_true')
    ap.add_argument('--tier', default='endpoint', choices=('endpoint', 'parent'))
    ap.add_argument('--no-expand', action='store_true', help='关掉常量展开（对照用）')
    ap.add_argument('--show-lists', action='store_true')
    args = ap.parse_args()
    if args.selftest:
        return cmd_selftest(args)
    if not args.census:
        print('要 --census PATH（挂载侧权威）。模式：--check 或 --json PATH')
        return 2
    if args.check:
        return cmd_check(args)
    if not args.json:
        print('--census 要配 --json PATH 或 --check')
        return 2
    return cmd_json(args)


if __name__ == '__main__':
    sys.exit(main())
