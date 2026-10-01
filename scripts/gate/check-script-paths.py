#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""check-script-paths.py — 代码/CI 里写死的仓内路径必须解析得到（F33 常驻门禁）。

背景：治理 MCP 曾把门禁脚本指向 scripts/，而两枚门禁早已迁到 scripts/gate/（治理文档 §5.61）。
那类"脚本迁走、引用没改"的缺陷，文档侧有 check-doc-links.py 管，代码与 CI 侧没有任何闸门管。
本门禁补这一层，并把自己的覆盖面按当轮实测打印（不判定项也印条数，不硬编）：

  判定
    CI 调用脱钩    .github/workflows/*.yml 的 run: 块里被敲的脚本路径，按仓根解析不到 → ERROR
                   （CI 的 cwd 恒为仓根，因此只有仓根这一种解释）
    执行路径脱钩   代码语料里的路径字面量原样解析不到，但同名文件就在**同一父目录更深处**
                   （scripts/x.py → scripts/gate/x.py＝插了一段目录），且该行不是注释/用法文本 → ERROR
    用法文本脱钩   同上但落在注释、Write-Host 文案或三引号串内 → WARN（给人错的命令，不改行为）
    台账           在册条目＝(file, path, hits, reason)，双向核对：处数不等或条目已不再命中都判 ERROR；
                   收口＝删条目，不是把 hits 写成 0

  不判定（只打印当轮条数当覆盖面声明）
    EXISTS／裸文件名（types.rs 之类不构成仓内路径声明）／同名在完全无关目录（同名撞车）／
    仓内根本没有同名文件（外链示例、待建产物、第三方）。

  分母口径：判定引用＝覆盖面各档之和，另印一行合计核对
    扫描引用（token 循环独立数出）− 显式豁免（跳过不判）必须＝覆盖面各档之和；
    遮蔽不成对是按文件计的形状量，不进分母。分类器多出一档而没进覆盖面表 ⇒ 该行点名它。

用法：
    python scripts/gate/check-script-paths.py                  # 现量体检；有未在册的执行面脱钩则 rc=1
    python scripts/gate/check-script-paths.py --selftest       # 内置微型夹具自检，与真仓语料无关
    python scripts/gate/check-script-paths.py --max-detail 500 # 明细上限（默认 120，超出必印截断告示）
"""
import io
import os
import re
import sys
import tempfile
from bisect import bisect_left
from collections import defaultdict

GATE_NAME = "check-script-paths"
TRUNCATION_MARK = "明细已截断"
IGNORE_MARK = "script-paths: ignore"
MIN_REASON = 20
EXTS = ('.py', '.ps1', '.sh', '.js', '.json', '.md', '.yml', '.yaml', '.toml', '.rs', '.sql', '.csv', '.html')
SKIP_DIRS = {'target', 'node_modules', '.git', 'dist', '.codebuddy', 'ais', 'third_party',
             '.trae', 'generated-images', '.logs', '.runtime', 'workspace', 'log', '__pycache__'}
CODE_DIRS = ('scripts', 'tools', 'frontend-ui/scripts', 'frontend-ui/src/modules')
CI_DIRS = ('.github/workflows',)
CI_EXTRA_SOURCES = ('scripts/gate/check-all.ps1',)
CODE_SUFFIXES = ('.py', '.ps1', '.sh', '.js', '.json', '.yml', '.yaml')

PATH_LIT = re.compile(r'[A-Za-z0-9_@][A-Za-z0-9_\-\./\\@]{2,120}?\.(?:py|ps1|sh|js|json|md|yml|yaml|toml|rs|sql|csv|html)\b')
COMMENT_LINE = re.compile(r'^\s*(?:#|//|\*|/\*|<!--|#>)')
PRINT_LINE = re.compile(r'Write-Host|Write-Output|Write-Information|Write-Warning|echo |print\(|printf|Console\.Write')
GUARD_LINE = re.compile(r'Test-Path|os\.path\.(?:exists|isfile)|\.exists\(\)|Path\.Exists|Path::new')
VITE_ALIAS = re.compile(r'(?<![\w./@-])@/')
URL_SPAN = re.compile(r'\S*://\S*')

# 覆盖面行的档位表＝分母唯一的定义处。classify 若多出一个态而没进这张表，
# "扫描引用 − 显式豁免 ≠ 覆盖面各档之和"当场破（coverage_account 的牙），
# 于是新档位只会进分母、不印出来＝把吞下的东西记在暗账里。
COVER_ORDER = ('EXISTS', 'MOVED', 'CI', 'FILENAME-ONLY', 'NAME-ELSEWHERE', 'NOWHERE')
COVER_LABEL = {'EXISTS': 'EXISTS', 'MOVED': 'MOVED', 'CI': 'CI run 引用',
               'FILENAME-ONLY': '裸文件名', 'NAME-ELSEWHERE': '同名在无关目录', 'NOWHERE': '仓内无同名'}
# 这两个按名另账：显式豁免是"扫描到但跳过不判"的引用，遮蔽不成对是按文件计的形状量、根本不是引用。
BAND_KEYS = ('扫描引用', '显式豁免', '遮蔽不成对')


def repo_root():
    env = os.environ.get("MOX_REPO_ROOT")
    root = os.path.abspath(env) if env else os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    if not os.path.isdir(os.path.join(root, 'scripts')):
        raise SystemExit('[%s] 仓库根定位失败：%s 下没有 scripts/（可用 MOX_REPO_ROOT 显式指定）' % (GATE_NAME, root))
    return root


def to_slash(p):
    return p.replace('\\', '/')


def walk_files(root, rel_dirs, suffixes=CODE_SUFFIXES):
    out = []
    for d in rel_dirs:
        base = os.path.join(root, d)
        if not os.path.isdir(base):
            continue
        for cur, dirs, files in os.walk(base):
            dirs[:] = [x for x in dirs if x not in SKIP_DIRS]
            for fn in files:
                if fn.endswith(suffixes):
                    out.append(to_slash(os.path.relpath(os.path.join(cur, fn), root)))
    return sorted(set(out))


def root_scripts(root):
    """仓根散落的启动/维护脚本（start.sh、根层 *.ps1）也点名入账：
       它们是"真的会被敲下去"的一层，漏掉这一层就等于把最像入口的入口放在扫描集外面。
       实测纳入后当轮不新增欠账（EXISTS 631→662，MOVED 仍 4，rc 仍 0）。"""
    return sorted(fn for fn in os.listdir(root)
                  if fn.endswith(('.py', '.sh', '.ps1')) and os.path.isfile(os.path.join(root, fn)))


def build_index(root):
    """现场枚举 basename → 相对路径集合。索引不信任何清单，包括本门禁自己的语料表。"""
    idx = defaultdict(set)
    n = 0
    for cur, dirs, files in os.walk(root):
        dirs[:] = [x for x in dirs if x not in SKIP_DIRS]
        for fn in files:
            if fn.endswith(EXTS):
                idx[fn].add(to_slash(os.path.relpath(os.path.join(cur, fn), root)))
                n += 1
    return idx, n


def package_roots(root, rel):
    """源文件所在的包根：往上找第一个带 package.json / Cargo.toml 的目录（npm 脚本以包目录为 cwd）。"""
    out = []
    parts = rel.split('/')
    for i in range(len(parts) - 1, 0, -1):
        up = '/'.join(parts[:i])
        if any(os.path.isfile(os.path.join(root, up, m)) for m in ('package.json', 'Cargo.toml')):
            out.append(up)
    return out


def resolve(tok, root, rel):
    """允许四种解释：仓根相对、源文件同目录相对、源文件所在包根相对、以及它们的组合。"""
    tok = to_slash(tok)
    sdir = os.path.dirname(rel)
    cands = [tok]
    if sdir:
        cands.append(sdir + '/' + tok)
    for pr in package_roots(root, rel):
        cands.append(pr + '/' + tok)
    for c in cands:
        if os.path.isfile(os.path.join(root, to_slash(c))):
            return to_slash(c)
    return None


def classify(tok, root, rel, index):
    """返回 (态, 证据)：EXISTS / MOVED / FILENAME-ONLY / NAME-ELSEWHERE / NOWHERE。"""
    tok = to_slash(tok)
    if resolve(tok, root, rel):
        return 'EXISTS', ''
    bn = tok.rsplit('/', 1)[-1]
    cands = index.get(bn, ())
    if not cands:
        return 'NOWHERE', ''
    if '/' not in tok:
        return 'FILENAME-ONLY', ''
    par = tok.rsplit('/', 1)[0] + '/'
    tight = sorted(c for c in cands if c != tok and c.startswith(par))
    if tight:
        return 'MOVED', ' | '.join(tight[:3])
    return 'NAME-ELSEWHERE', ''


def masked_flags(text, ext):
    """逐行标记「落在文档性区间内」：Python 三引号串、PowerShell <# #> 注释块与 @" "@ here-string、
       JS 块注释。遮蔽是承重的——没有它，帮助文案里的命令会被判成执行面脱钩（假阳）；
       遮蔽过度（比如整份文件当注释）又会让真脱钩隐身，所以两个方向各有对照格。"""
    pairs = {'.py': [('"""', '"""'), ("'''", "'''")],
             '.ps1': [('<#', '#>'), ('@"', '"@'), ("@'", "'@")],
             '.js': [('/*', '*/')],
             '.sh': [],
             '.json': []}.get(ext, [])
    lines = text.split('\n')
    flags = [False] * len(lines)
    unpaired = 0
    for opener, closer in pairs:
        idx = 0
        while True:
            s = text.find(opener, idx)
            if s < 0:
                break
            e = text.find(closer, s + len(opener))
            if e < 0:
                # 开不闭：宁可少遮蔽（多报）也不能整份文件当注释（隐身）
                unpaired += 1
                idx = s + len(opener)
                continue
            e += len(closer)
            start_line = text.count('\n', 0, s)
            end_line = text.count('\n', 0, e)
            for i in range(start_line, min(end_line + 1, len(flags))):
                flags[i] = True
            idx = e
    return flags, unpaired


def role_of(line_text, in_docstring):
    """EXEC＝这行真会把路径喂给解释器；TEXT＝给人看的；GUARD＝只探测存在性。"""
    if COMMENT_LINE.match(line_text) or in_docstring or PRINT_LINE.search(line_text):
        return 'TEXT'
    if GUARD_LINE.search(line_text):
        return 'GUARD'
    return 'EXEC'


def scan_tokens(text):
    """产出 (行号, 归一后的仓内路径候选)。两重前处理都是承重的：
       ① vite 别名 @/ 展开成 frontend-ui/src/，否则活引用判成断链；
       ② 先把 x://… 整段抹成等长空格（保住行号），否则 https://a/b/c.py 会被正则从 c.py 往前截出
          一个"仓内路径"a/b/c.py——这是假阳，不是覆盖面。"""
    norm = VITE_ALIAS.sub('frontend-ui/src/', text)
    norm = URL_SPAN.sub(lambda m: ' ' * len(m.group(0)), norm)
    nl = [i for i, ch in enumerate(norm) if ch == '\n']
    out = []
    for m in PATH_LIT.finditer(norm):
        tok = to_slash(m.group(0))
        if tok.startswith('http'):
            continue
        out.append((bisect_left(nl, m.start()) + 1, tok))
    return out


CI_COMMENT = re.compile(r'(?:^|\s)#')


def strip_ci_comment(line):
    """剥掉 shell 的行尾注释：`#` 位于行首或前面是空白才算注释（引号里的 # 剥不掉，宁少不多）。
       整行注释是这条规则的退化情形（cut 落在 0，本体清空），所以不需要另写一条 startswith('#') 守卫。"""
    cut = CI_COMMENT.search(line)
    return line[:cut.start() + 1] if cut else line


def ci_command_tokens(text):
    """只取 run: 块（含 run: | 之后的多行）里"真的被敲下去的脚本路径"。
       承重结构是缩进：run: 块的续行必须比 run: 这一行更深，一旦回到同级或更浅就出块——
       这一步同时把 run 之后的 with:/env: 映射键和下一个 step 挡在外面。
       （曾经还写了一条 "续行不许以 - 开头" 的守卫：合法 YAML 里 step 列表项永远不比自己的
        run: 键更深，缩进判据先把否掉，撤掉它 --check 读数逐字节不变（已实测比对）＝纸面机制，删。）
       注释通道（strip_ci_comment）是承重的，本门禁接进 ci.yml 那一刻自己就撞上了：run 块里的
       说明文字永远不会被执行，不剥掉它，"旧写法 scripts/x.py 已迁走"会被判成"CI 要执行 scripts/x.py"（假阳）。"""
    hits = []
    block_indent = 0
    in_run = False
    for lineno, raw in enumerate(text.split('\n'), 1):
        stripped = raw.strip()
        indent = len(raw) - len(raw.lstrip())
        if stripped.startswith('run:'):
            in_run = True
            block_indent = indent
            body = strip_ci_comment(stripped[4:])
            if body.strip():
                hits.extend(_exec_words(lineno, body))
            continue
        if in_run:
            if stripped and indent > block_indent:
                hits.extend(_exec_words(lineno, strip_ci_comment(stripped)))
            else:
                in_run = False
    return hits


SCRIPT_SUFFIX = ('.py', '.sh', '.ps1', '.js')


def line_kept(lines, lineno):
    """该行是否参与判定（带显式豁免标记的登记性字面量跳过）。
       这条必须是函数而不是自检里重抄一遍——否则自检绿着，真通道的跳过逻辑坏了也没人看见。"""
    return IGNORE_MARK not in lines[lineno - 1]


def _exec_words(lineno, body):
    """两道关卡，各自一格对照、各一枚变异体（第三关曾被写成独立分支，实测被这两关完全覆盖，已删）：
       ① 后缀白名单：`cat reports/data/x.csv`、`rm README.md` 里的文件不是"被执行对象"，
          即便 PATH_LIT 认得它们的路径形状也不进 CI 账。
       ② PATH_LIT 整词匹配（fullmatch，不是 search）：拒绝"路径形状外面还粘着字符"的 token，
          典型是容器里的绝对路径 `/github/workspace/tools/x.py`——子串匹配会从 'g' 起截出
          `github/workspace/tools/x.py` 并判它仓内解析不到（假阳，而且假阳的代价是人绕过闸门）。
       曾经还有 WORD_NOISE 与 -前缀 两关：词法噪声字符（= " ' $ ( ) { } | & ? `）与行首连字符
       都不在 PATH_LIT 的字符集里，② 必然先把它们拒掉，两关永不开火＝纸面机制；
       反斜杠也不成立，因为 to_slash 在 split 之前就把 \\ 译成了 /。删除后当轮真仓读数不变（已复跑比对）。"""
    out = []
    for word in to_slash(body).split():
        if word.endswith(SCRIPT_SUFFIX) and PATH_LIT.fullmatch(word):
            out.append((lineno, word))
    return out


# ── 台账：在册＝已知欠账，必须带处数与 ≥MIN_REASON 字的理由；收口要删条目 ──────────────
LEDGER = [
    {"file": ".github/workflows/graph-gate.yml", "path": "tools/guantu_gate.py", "hits": 1,
     "reason": "该脚本已被搬进 docs/_archive/tools/ 归档，此 CI 步骤在每次 push 上执行一个仓根解析不到的文件；"
               "三种修法（把门禁搬回 tools/、改指归档副本、删该 job）都是架构裁决，未点名前不改 CI 文件"},
    {"file": ".github/workflows/enterprise-ci.yml", "path": "verify_axioms.py", "hits": 1,
     "reason": "仓内不存在同名文件（os.walk 全仓枚举后 verify_axioms* 为 0 命中），"
               "该 job 从落库起就没开过火；删 job 与重建脚本都属裁决，不在本轮范围"},
    {"file": "scripts/tests/Run-T17-EF-All.ps1", "path": "scripts/Run-T17-SDK-All.ps1", "hits": 1,  # script-paths: ignore
     "reason": "该 harness 既未接 CI 也未接 check-all.ps1，同文件还引用仓内已不存在的 platform/backend-node 链；"
               "只改这一行等于把半死的验收链装作能跑，待裁决"},
    {"file": "scripts/tests/Run-T17-EF-All.ps1", "path": "scripts/Run-T19-Regression-706.ps1", "hits": 1,  # script-paths: ignore
     "reason": "同 harness 同一处断链（真身在 scripts/tests/ 下），随整套验收链一起裁决"},
    {"file": "scripts/tests/Run-T17-EF-All.ps1", "path": "scripts/Gray-Warmup.ps1", "hits": 1,  # script-paths: ignore
     "reason": "该行既把路径喂给 Test-Path 又被 & 调用（真身在 scripts/deploy/ 下），"
               "改法要随 harness 一起定，不单点"},
    {"file": "scripts/tests/run_enterprise_7gates.ps1", "path": "scripts/validate_rust_workspace_deps.js",  # script-paths: ignore
     "hits": 1,
     "reason": "该行在 Push-Location platform/backend-node 之后执行，而 platform/backend-node 整目录已不在仓内，"
               "七闸 harness 从入口就断，逐行改路径无意义"},
]


def ledger_audit(ledger, live_by_key):
    """台账与当前命中的双向核对。返回 (未在册的执行面, 台账不一致, 豁免处数)。"""
    ent_by_key = {(e['file'], e['path']): e for e in ledger}
    unledgered, mismatch = [], []
    exempted = 0
    for key, items in sorted(live_by_key.items()):
        ent = ent_by_key.get(key)
        if ent is None:
            unledgered.extend(items)
        elif ent['hits'] != len(items):
            mismatch.append({'file': key[0], 'path': key[1], 'line': items[0].get('line', 0),
                             'role': 'LEDGER', 'why': '台账处数 %s，当前命中 %d' % (ent['hits'], len(items))})
        else:
            exempted += len(items)
    for key, ent in ent_by_key.items():
        if key not in live_by_key:
            mismatch.append({'file': key[0], 'path': key[1], 'line': 0, 'role': 'LEDGER',
                             'why': '在册条目已不再命中——收口后应删掉这一条而不是留着（也防止 hits 被写成 0 蒙过）'})
    bad_reasons = [{'file': e['file'], 'path': e['path'], 'line': 0, 'role': 'REASON',
                    'why': '理由仅 %d 字，不足 %d' % (len(e['reason']), MIN_REASON)}
                   for e in ledger if len(e['reason']) < MIN_REASON]
    return unledgered, mismatch + bad_reasons, exempted


def truncation_notice(total, printed):
    if total <= printed:
        return ""
    return ("⚠ %s：打印 %d 条／总体 %d 条（差额 %d 条未列出，判据按总体算）\n"
            % (TRUNCATION_MARK, printed, total, total - printed))


def coverage_text(counts):
    return "覆盖面（当轮实测，含不判定项）：" + " ｜".join(
        "%s %d" % (COVER_LABEL[k], counts[k]) for k in COVER_ORDER)


def judged_refs(counts):
    """判定引用＝覆盖面各档之和。分母不取 sum(counts)——那会把"跳过不判"和"按文件计的形状量"
       一起算成被判决过的引用，标签与组成不符（本轮由自己的合计核对抓出，见治理文档 §5.62）。"""
    return sum(counts[k] for k in COVER_ORDER)


def unaccounted_buckets(counts):
    return sorted(k for k in counts if k not in COVER_ORDER and k not in BAND_KEYS)


def coverage_account(counts):
    """合计核对行：分母（扫描到的引用）由 token 循环独立数出，覆盖面由各档相加，
       两者对不上就说明有档位进了分母却没进覆盖面行。"""
    extra = unaccounted_buckets(counts)
    if extra:
        return "⚠ 有档位没进覆盖面行：%s（分母含没印出来的判决，覆盖面账不完整）" % '、'.join(extra)
    skipped = counts['扫描引用'] - counts['显式豁免']
    gap = skipped - judged_refs(counts)
    if gap:
        return ("⚠ 覆盖面合计对不上分母：扫描引用 %d − 显式豁免 %d ＝ %d，而覆盖面各档之和 %d（差 %+d）"
                % (counts['扫描引用'], counts['显式豁免'], skipped, judged_refs(counts), gap))
    return ("覆盖面合计核对：扫描引用 %d − 显式豁免 %d（跳过不判）＝覆盖面各档之和＝判定引用 %d"
            "｜遮蔽不成对 %d 处按文件计、不是引用，不进分母"
            % (counts['扫描引用'], counts['显式豁免'], judged_refs(counts), counts['遮蔽不成对']))


def collect(root):
    index, walked = build_index(root)
    code_sources = sorted(set(walk_files(root, CODE_DIRS)) | set(CI_EXTRA_SOURCES) | set(root_scripts(root)))
    ci_sources = walk_files(root, CI_DIRS)

    exec_broken, text_warn, ci_broken = [], [], []
    counts = defaultdict(int)
    for rel in code_sources:
        try:
            text = io.open(os.path.join(root, rel.replace('/', os.sep)), encoding='utf-8', errors='replace').read()
        except OSError:
            continue
        flags, unpaired = masked_flags(text, os.path.splitext(rel)[1])
        counts['遮蔽不成对'] += unpaired
        lines = text.split('\n')
        for lineno, tok in scan_tokens(text):
            counts['扫描引用'] += 1
            if not line_kept(lines, lineno):
                counts['显式豁免'] += 1
                continue
            state, ev = classify(tok, root, rel, index)
            counts[state] += 1
            if state != 'MOVED':
                continue
            role = role_of(lines[lineno - 1], flags[lineno - 1])
            item = {'file': rel, 'line': lineno, 'path': tok, 'where': ev, 'role': role}
            (text_warn if role == 'TEXT' else exec_broken).append(item)
    for rel in ci_sources:
        try:
            text = io.open(os.path.join(root, rel.replace('/', os.sep)), encoding='utf-8', errors='replace').read()
        except OSError:
            continue
        for lineno, tok in ci_command_tokens(text):
            counts['扫描引用'] += 1
            counts['CI'] += 1
            if not os.path.isfile(os.path.join(root, tok.replace('/', os.sep))):
                ci_broken.append({'file': rel, 'line': lineno, 'path': tok, 'role': 'CI'})
    judged = judged_refs(counts)
    return {'index': index, 'walked': walked, 'code_sources': code_sources, 'ci_sources': ci_sources,
            'counts': counts, 'judged': judged, 'exec_broken': exec_broken,
            'text_warn': text_warn, 'ci_broken': ci_broken}


def verdict_line(counts, exec_broken, text_warn, ci_broken, unledgered):
    """判决分流行：把"每条判决通道各数出多少"恒印出来（0 也要印）。
       小节标题是"非空才 dump"的，只靠段落缺席来表达 0，和"这条通道压根没通电"长得一模一样。
       MOVED 必须等于执行面＋用法文本两支之和——这支等式是本函数唯一能自证的地方，留着当分母。"""
    exec_un = [i for i in unledgered if i['role'] != 'CI']
    ci_un = [i for i in unledgered if i['role'] == 'CI']
    if counts['MOVED'] != len(exec_broken) + len(text_warn):
        return ("⚠ 分流不等式破了：MOVED %d ≠ 执行面 %d＋用法文本 %d（有一支在丢件）"
                % (counts['MOVED'], len(exec_broken), len(text_warn)))
    return ("判决分流：MOVED %d＝执行面 %d（未在册 %d）＋用法文本 %d ｜CI run 判缺 %d（未在册 %d）"
            "｜显式豁免标记 %d 处（行尾 %s）｜遮蔽区间不成对 %d 处（少遮蔽不多吞）"
            % (counts['MOVED'], len(exec_broken), len(exec_un), len(text_warn),
               len(ci_broken), len(ci_un), counts['显式豁免'], IGNORE_MARK, counts['遮蔽不成对']))


def check(root, max_detail):
    data = collect(root)
    counts, judged = data['counts'], data['judged']
    live = defaultdict(list)
    for it in data['ci_broken'] + data['exec_broken']:
        live[(it['file'], it['path'])].append(it)
    unledgered, mismatch, exempted = ledger_audit(LEDGER, live)

    print("[%s] 仓根 %s" % (GATE_NAME, root))
    print("枚举文件 %d 份（%d 个不同 basename）｜代码语料 %d 份｜CI 语料 %d 份｜判定引用 %d 条"
          % (data['walked'], len(data['index']), len(data['code_sources']), len(data['ci_sources']), judged))
    print(coverage_text(counts))
    print(coverage_account(counts))
    print(verdict_line(counts, data['exec_broken'], data['text_warn'], data['ci_broken'], unledgered))

    def dump(title, items, render):
        print("\n%s（%d，明细最多 %d 条，--max-detail 可调）：" % (title, len(items), max_detail))
        for it in items[:max_detail]:
            print("  " + render(it))
        sys.stdout.write(truncation_notice(len(items), min(max_detail, len(items))))

    if unledgered:
        dump("执行面脱钩·未在册", unledgered,
             lambda it: "[BROKEN] %s:%s -> %s （%s；盘上同名在 %s）"
                        % (it['file'], it['line'], it['path'], it['role'], it.get('where') or '无'))
    if mismatch:
        dump("台账不一致", mismatch,
             lambda it: "[LEDGER] %s -> %s：%s" % (it['file'], it['path'], it.get('why', '')))
    if exempted:
        print("\n已在册豁免 %d 处（逐条见 LEDGER；收口＝删条目）" % exempted)
    if data['text_warn']:
        dump("告警·用法文本里的脱钩命令（给人错的命令，不改行为）", data['text_warn'],
             lambda it: "[WARN]   %s:%s -> %s（盘上同名在 %s）" % (it['file'], it['line'], it['path'], it['where']))
    if not unledgered and not mismatch:
        print("\n[OK] 执行面路径全部解析得到或已在册，台账与当前命中一致")
    return 1 if (unledgered or mismatch) else 0


# ── 自检：微型夹具自造，绝不读真仓语料（语料随别人的提交变，自检不能） ──────────────────
FIXTURE = {
    'scripts/gate/real_gate.py': '# gate\n',
    'scripts/gate/keep.py': '# 供"相对源文件目录"这一格用\n',
    'scripts/keep.py': 'import os\n',
    'start_probe.sh': '#!/bin/sh\nbash scripts/keep.py\n',
    'tools/types.rs': '',
    'frontend-ui/package.json': '{}',
    'frontend-ui/src/api/http.js': '',
    'docs/_archive/tools/guantu_gate.py': '',
    'scripts/gate/twoface.py': ('run scripts/real_gate.py  # script-paths: ignore\n'
                                'run scripts/real_gate.py\n'),
    # 三档"不判定"的形状必须由 collect 真的判到（M29 第一轮在夹具上全绿，就是因为夹具只吐出 EXISTS/MOVED
    # 两档——空桶上的合计核对照样闭合，覆盖面表漏登记一档也没人看见）。
    'scripts/states.py': ('cat types.rs\n'
                          'run tools/keep.py\n'
                          'python docs/_archive/tools/gone-9f3q.py\n'),
    '.github/workflows/demo.yml': (
        'name: demo\non: push\njobs:\n  a:\n    steps:\n'
        '      - name: ok\n        run: python scripts/keep.py\n'
        '      - name: broken\n        run: python3 tools/guantu_gate.py\n'
        '      - name: moved\n        run: |\n          python scripts/real_gate.py  # 旧写法 scripts/gone_gate.py 已迁走\n'
        '          # 整行注释里的 scripts/also_gone.py 同样不执行\n'
        '          pip install numpy\n'
        # run 块结束之后紧跟的 with:/env: 映射不属于被敲下去的命令：缩进比 run: 更深，
        # 只有"缩进承重点"这一条判据能把它挡在外面（变异体 M12 就是拆它）。
        '        with:\n          script: scripts/map_only.py\n'),
}


def _mk_fixture(root):
    for rel, body in FIXTURE.items():
        p = os.path.join(root, rel.replace('/', os.sep))
        os.makedirs(os.path.dirname(p) or root, exist_ok=True)
        io.open(p, 'w', encoding='utf-8').write(body)


def selftest_cells(root, index):
    cells = []

    def chk(name, ok, detail=''):
        cells.append((name, bool(ok), detail))

    probe = 'scripts/gate/probe.py'
    for tok, rel, want in [('scripts/gate/real_gate.py', probe, 'EXISTS'),
                           ('scripts/real_gate.py', probe, 'MOVED'),
                           ('types.rs', probe, 'FILENAME-ONLY'),
                           ('tools/guantu_gate.py', probe, 'NAME-ELSEWHERE'),
                           ('nope/nosuch-9f3q.py', probe, 'NOWHERE'),
                           ('keep.py', 'scripts/gate/probe.py', 'EXISTS'),
                           ('src/api/http.js', 'frontend-ui/src/modules/x/index.js', 'EXISTS')]:
        got, ev = classify(tok, root, rel, index)
        chk('分类 %s' % tok, got == want, '期望 %s 实得 %s｜%s' % (want, got, ev))

    for ln, doc, want in [('    # 用法: bash scripts/zz-smoke.sh', False, 'TEXT'),
                          ('    python scripts/zz-ci.py  # 全量', True, 'TEXT'),
                          ('  $CMD = "& (Join-Path $ROOT \'scripts\\X.ps1\')"', False, 'EXEC'),
                          ('  $GRAY = Join-Path $ROOT "scripts\\Gray.ps1"', False, 'EXEC'),
                          ('  if (Test-Path $GRAY) { $ok = 1 }', False, 'GUARD'),
                          ('Write-Host "run scripts/zz-start.ps1"', False, 'TEXT')]:
        got = role_of(ln, doc)
        chk('角色 %s' % ln.strip()[:26], got == want, '期望 %s 实得 %s' % (want, got))

    # 遮蔽双向对照：撤遮蔽打红"串内"格，过度遮蔽（连串外也吞）打红"串外"格
    body = '"""\npython scripts/a.py\n"""\ncd scripts/b.py\n'
    fl, unp = masked_flags(body, '.py')
    roles = [role_of(l, f) for l, f in zip(body.split('\n'), fl)]
    chk('三引号串内的用法行判 TEXT', roles[1] == 'TEXT' and unp == 0, '实得 %s un=%d' % (roles, unp))
    chk('三引号串外的同形态行判 EXEC', roles[3] == 'EXEC', '实得 %s' % roles)
    ps = ('<#\n.EXAMPLE\n  .\\scripts\\zz-start.ps1 Stop\n#>\n'
          '$b = @"\n  run scripts/zz-start.ps1 Stop\n"@\n& .\\scripts\\zz-start.ps1 Stop\n')
    pfl, punp = masked_flags(ps, '.ps1')
    plines = ps.split('\n')
    chk('PS 注释块 <# #> 内行判 TEXT', role_of(plines[2], pfl[2]) == 'TEXT', '实得 %s' % pfl)
    chk('PS here-string 内行判 TEXT', role_of(plines[5], pfl[5]) == 'TEXT', '实得 %s' % pfl)
    chk('PS 块注释与 here-string 之后的行判 EXEC', role_of(plines[7], pfl[7]) == 'EXEC' and punp == 0,
        '实得 %s un=%d' % (pfl, punp))
    # 开不闭不许一路遮蔽到文件尾——那会让真脱钩整份隐身，宁可少遮蔽多报
    dang, dunp = masked_flags('<#\npython scripts/a.py\n', '.ps1')
    chk('遮蔽不成对时不吞后文并计数', dunp == 1 and dang[1] is False, '实得 %s un=%d' % (dang, dunp))

    # 显式豁免指令：登记性字面量（本门禁的台账就写着坏路径）用行尾标记豁免，且按名打印条数。
    # 这里必须调生产函数 line_kept，不许在自检里重抄一遍判据——重抄的那份绿着，
    # 真通道（collect）的跳过逻辑坏了也没人看见（变异电池第一轮就是这么露馅的）。
    ig = 'run scripts/a.py  # script-paths: ignore\nrun scripts/b.py\n'
    kept = [(l, t) for l, t in scan_tokens(ig) if line_kept(ig.split('\n'), l)]
    chk('带 ignore 标记的行被跳过，同文件不带标记的行仍判', [t for _l, t in kept] == ['scripts/b.py'],
        '实得 %s' % kept)
    chk('line_kept 对无标记行返回真、对行尾带标记的行返回假',
        line_kept(['plain text'], 1) is True and line_kept(['x  # ' + IGNORE_MARK], 1) is False, '判据反了')

    # 端到端：collect 真的按上面那条通道跳过（夹具 twoface.py 同一行内容两写，只应剩一枚执行面脱钩）
    e2e = collect(root)
    faces = [it for it in e2e['exec_broken'] if it['file'] == 'scripts/gate/twoface.py']
    chk('collect 端到端：带 ignore 的那行没进执行面，另一行按 MOVED/EXEC 进',
        len(faces) == 1 and faces[0]['line'] == 2 and faces[0]['path'] == 'scripts/real_gate.py',
        '实得 %s' % faces)
    chk('collect 端到端：跳过量按名计数（不许记在暗账里）', e2e['counts']['显式豁免'] == 1,
        '实得 %s' % e2e['counts']['显式豁免'])
    chk('仓根散落的启动脚本进扫描集（入口层不许放在覆盖面外）',
        root_scripts(root) == ['start_probe.sh'] and 'start_probe.sh' in e2e['code_sources'],
        '实得 rs=%s 根层=%s' % (root_scripts(root), [s for s in e2e['code_sources'] if '/' not in s]))

    # 覆盖面账（本轮仪器自捉）：判定引用曾 = sum(counts.values())，把 12 条"跳过不判"的引用和
    # 13 处"按文件计"的遮蔽形状量都算成被判决过的引用——标签与组成不符，且没有任何一行看得住。
    # 于是分母改由 token 循环独立数出（扫描引用），覆盖面行按 COVER_ORDER 印，两者必须闭合。
    cov = coverage_text(e2e['counts'])
    nums = [int(x) for x in re.findall(r'\d+', cov)]
    chk('覆盖面行印出的各数之和＝判定引用（行与分母同源，漏印一档即红）',
        len(nums) == len(COVER_ORDER) and sum(nums) == judged_refs(e2e['counts']),
        '实得 行=%s 判定=%d' % (nums, judged_refs(e2e['counts'])))
    chk('合计核对行在真夹具上必须闭合（不许 ⚠）',
        coverage_account(e2e['counts']).startswith('覆盖面合计核对'), '实得 %s' % coverage_account(e2e['counts']))
    stray = coverage_account(dict(e2e['counts'], **{'SOME-NEW-STATE': 7}))
    chk('多出一档没登记进覆盖面表时必须点名该档（新档位不许只进分母不进账）',
        stray.startswith('⚠') and 'SOME-NEW-STATE' in stray, '实得 %s' % stray)
    # 上面两格的牙都取决于"夹具真的把每一档都判到"——空桶上的合计核对照样闭合（M29 第一版就这样全绿溜过去）。
    emitted = set()
    for rel in e2e['code_sources']:
        try:
            body = io.open(os.path.join(root, rel.replace('/', os.sep)),
                           encoding='utf-8', errors='replace').read()
        except OSError:
            continue
        lines = body.split('\n')
        for lineno, tok in scan_tokens(body):
            if line_kept(lines, lineno):
                emitted.add(classify(tok, root, rel, e2e['index'])[0])
    want_states = set(COVER_ORDER) - {'CI'}
    chk('夹具 collect 必须把分类器每一档都判到（档位表与分类器实吐一比一，CI 档另由 demo.yml 供）',
        emitted == want_states and all(e2e['counts'][k] > 0 for k in COVER_ORDER),
        '实得 emitted=%s 表=%s 各档=%s' % (sorted(emitted), sorted(want_states),
                                        {k: e2e['counts'][k] for k in COVER_ORDER}))

    # vite 别名展开：不展开就把活引用判成断链；URL 遮蔽：不遮蔽就把外链尾段判成仓内路径（假阳）
    toks = [t for _l, t in scan_tokens("import x from '@/api/http.js';\n")]
    chk('别名 @/ 展开成 frontend-ui/src/', toks == ['frontend-ui/src/api/http.js'], '实得 %s' % toks)
    ext = [t for _l, t in scan_tokens("fetch('https://a.example.com/pkg/c.js')\n")]
    chk('URL 整段遮蔽后不留仓内候选', ext == [], '实得 %s' % ext)

    # CI run 块提取与存在性判定
    text = io.open(os.path.join(root, '.github', 'workflows', 'demo.yml'), encoding='utf-8').read()
    hits = ci_command_tokens(text)
    chk('run: 块与 run: | 多行都提取到（3 条）', [t for _l, t in hits] ==
        ['scripts/keep.py', 'tools/guantu_gate.py', 'scripts/real_gate.py'], '实得 %s' % hits)
    missing = sorted(t for _l, t in hits if not os.path.isfile(os.path.join(root, t.replace('/', os.sep))))
    chk('按仓根解析不到的 2 条判缺', missing == sorted(['tools/guantu_gate.py', 'scripts/real_gate.py']), '实得 %s' % missing)
    # 注释通道（本门禁接进 ci.yml 时自己踩的：run 块里的说明文字不是被执行的命令）
    chk('CI 注释：run: | 块里整行 # 的路径不进执行面',
        'scripts/also_gone.py' not in [t for _l, t in hits], '实得 %s' % hits)
    chk('CI 注释：命令行尾 ` #…` 之后的路径不进执行面',
        'scripts/gone_gate.py' not in [t for _l, t in hits], '实得 %s' % hits)
    inline = ci_command_tokens('  run: python scripts/keep.py  # 别执行 scripts/dead.py\n')
    chk('CI 注释：行尾注释剥离后命令本体仍留', [t for _l, t in inline] == ['scripts/keep.py'], '实得 %s' % inline)
    # CI 词法（真仓踩过：license-compliance.yml 的 --exclude='pnpm-lock.yaml' 曾被判成"CI 要执行它"）
    w1 = _exec_words(1, "EXCLUDES=(--exclude='*.md' --exclude='pnpm-lock.yaml')")
    chk('CI 词法②：排除模式与带引号 token 不当作执行面', w1 == [], '实得 %s' % w1)
    w2 = _exec_words(1, 'python3 tools/guantu_gate.py --strict')
    chk('CI 词法：真正的脚本实参仍被提取', w2 == [(1, 'tools/guantu_gate.py')], '实得 %s' % w2)
    # 关卡①：后缀白名单——去掉它，`cat x.csv` / `rm README.md` 会被记成"CI 要执行数据与文档"
    w3 = _exec_words(1, 'cat reports/data/audit.csv && rm docs/README.md')
    chk('CI 词法①：数据/文档文件不算被执行对象', w3 == [], '实得 %s' % w3)
    # 关卡②：整词匹配——换成子串匹配，容器绝对路径会被截出一段"仓内相对路径"判成断链（假阳）
    w4 = _exec_words(1, 'python3 /github/workspace/tools/guantu_gate.py')
    chk('CI 词法②：绝对路径不被切片成仓内候选', w4 == [], '实得 %s' % w4)
    # run 块的"缩进承重点"：run 之后紧跟的更深缩进映射键（with:/env:）不是被敲下去的命令
    cb = ci_command_tokens('steps:\n- name: a\n  run: python scripts/keep.py\n  with:\n    script: scripts/map_only.py\n')
    chk('CI 承重点：run 块后的更深缩进映射键不当作执行面', [t for _l, t in cb] == ['scripts/keep.py'], '实得 %s' % cb)

    # 台账双向
    item = lambda f, p: {'file': f, 'path': p, 'line': 1, 'role': 'CI', 'why': ''}
    un, mm, ex = ledger_audit([{'file': 'a', 'path': 'p', 'hits': 2, 'reason': 'x' * MIN_REASON}],
                              {('a', 'p'): [item('a', 'p'), item('a', 'p')]})
    chk('处数相符的在册项不报错且计入豁免', un == [] and mm == [] and ex == 2, '实得 un=%s mm=%s ex=%s' % (un, mm, ex))
    un, mm, ex = ledger_audit([{'file': 'a', 'path': 'p', 'hits': 1, 'reason': 'x' * MIN_REASON}],
                              {('a', 'p'): [item('a', 'p'), item('a', 'p')]})
    chk('处数少了要判台账不一致', len(mm) == 1 and ex == 0, '实得 mm=%s ex=%s' % (mm, ex))
    un, mm, ex = ledger_audit([], {('b', 'q'): [item('b', 'q')]})
    chk('未在册的执行面必须进 BROKEN 集', len(un) == 1, '实得 un=%s' % un)
    un, mm, ex = ledger_audit([{'file': 'z', 'path': 'gone.py', 'hits': 1, 'reason': 'x' * MIN_REASON}], {})
    chk('在册却已不再命中的条目要判过期', len(mm) == 1, '实得 mm=%s' % mm)
    un, mm, ex = ledger_audit([{'file': 'a', 'path': 'p', 'hits': 1, 'reason': '太短'}],
                              {('a', 'p'): [item('a', 'p')]})
    chk('理由字数不足要判台账不合格', len(mm) == 1 and mm[0]['role'] == 'REASON', '实得 mm=%s' % mm)

    # 真仓台账形状（只钉结构，不钉命中数——命中数属语料，随别人提交变）
    chk('台账每条四键齐全', all(set(e) == {'file', 'path', 'hits', 'reason'} for e in LEDGER),
        '不齐 %d 条' % len([e for e in LEDGER if set(e) != {'file', 'path', 'hits', 'reason'}]))
    chk('台账键不重复', len({(e['file'], e['path']) for e in LEDGER}) == len(LEDGER), '有重复')
    chk('台账 hits 必须为正（不许写 0 蒙过）', all(int(e['hits']) >= 1 for e in LEDGER),
        '非正 %d 条' % len([e for e in LEDGER if int(e['hits']) < 1]))

    # 判决分流行（0 也必须印得出来；两支之和必须等于 MOVED，否则是丢件而不是"没命中"）
    it_exec = {'role': 'EXEC'}
    it_ci = {'role': 'CI'}
    vl = verdict_line({'MOVED': 3, '显式豁免': 5, '遮蔽不成对': 7},
                      [it_exec, it_exec], [it_exec], [it_ci, it_ci], [it_exec, it_exec, it_ci])
    chk('分流行把两支未在册按 role 分开数（CI 不许混进执行面）',
        '执行面 2（未在册 2）＋用法文本 1 ｜CI run 判缺 2（未在册 1）' in vl, '实得 %r' % vl)
    chk('两支之和≠MOVED 时必须印不等式而不是照常分流',
        verdict_line({'MOVED': 5, '显式豁免': 0, '遮蔽不成对': 0}, [it_exec], [it_exec], [], [])
        .startswith('⚠'), '实得 %r' % verdict_line({'MOVED': 5, '显式豁免': 0, '遮蔽不成对': 0},
                                                   [it_exec], [it_exec], [], []))
    chk('全零语料仍要印出判决分流行（段落缺席≠通道通电）',
        verdict_line({'MOVED': 0, '显式豁免': 0, '遮蔽不成对': 0}, [], [], [], []).startswith('判决分流：MOVED 0'),
        '实得 %r' % verdict_line({'MOVED': 0, '显式豁免': 0, '遮蔽不成对': 0}, [], [], [], []))

    # 截断告示
    n1, n2 = truncation_notice(43, 3), truncation_notice(3, 3)
    chk('上限小于总体时必须印截断告示', TRUNCATION_MARK in n1 and '打印 3 条／总体 43 条' in n1, '实得 %r' % n1)
    chk('未截断时不得印截断告示', n2 == '', '实得 %r' % n2)
    return cells


def selftest():
    fails = 0
    with tempfile.TemporaryDirectory() as tmp:
        _mk_fixture(tmp)
        index, walked = build_index(tmp)
        cells = selftest_cells(tmp, index)
        print('[selftest] 夹具枚举文件 %d 份（%d 个不同 basename）｜格 %d 枚'
              % (walked, len(index), len(cells)))
        for name, ok, detail in cells:
            print('  [%s] %s%s' % ('PASS' if ok else 'FAIL', name, '' if ok else '  ｜' + detail))
            fails += 0 if ok else 1
    print('SELFTEST PASS=%d FAIL=%d' % (len(cells) - fails, fails))
    return 1 if fails else 0


def main(argv):
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass
    if '--selftest' in argv:
        return selftest()
    max_detail = 120
    for i, a in enumerate(argv):
        if a == '--max-detail' and i + 1 < len(argv):
            max_detail = int(argv[i + 1])
    return check(repo_root(), max_detail)


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
