# -*- coding: utf-8 -*-
"""核心公式的识别与复算：一条链是不是**算术主张**，只在这里判一次。

被两份仪器共用：
  - `scripts/gate/check-api-surface.py` 的 `--ledger`（单文档托管块台账，见 §1.7）
  - `scripts/gate/check-doc-formulas.py`（全 `docs/` 公式普查，扫描集分母在这里量）
任何一侧再自己写一份识别正则＝第二个源（同一本账两把尺子必然漂移）。

判据形状由 2026-10-02 在全库实测归纳（不是手感）：朴素尺子报 171 条主张、112 条"不闭合"，
逐条读原文后**只有仪器是坏的**——`exit=0`／`rc=1`／`failed ≥ 649`／`**加粗**`／`` `code` ``／
`BatchA(T10) 12`／`2.1=10 + 2.2=9`／`鲲鹏920+飞腾2000`／`+1 文件 +9 例` 全都不是分解式加法。
收紧后的口径（主张 19 条 → 误红 2 条 → 0 条）落在四条规则上：

1. **粘连即非数**：数字串紧贴字母（含中文）、数字、`_`、`.`，或右贴 `%`／`*` ⇒ 它是标识符的一部分。
   带小数点／下划线的串（`2.1`、`v1_0`）一并走这条，所以本台账只管整数加法。
2. **等号要全角**：本仓"我在写一条等式"的记法是 `＝` 配 `＋`；ASCII `=` 在这个语料里被
   `key=value`、markdown 与代码占满，只进盲区账（reason=ascii_eq）不判红。
3. **跨不过代码段与句子**：反引号区间、`。；；！？` 与换行都是硬边界；
   跨代码段的链进盲区账（reason=code）。
4. **紧邻的 `+`／`＋` 是增量记号不是加号**：`+1 文件` 表达"本轮多一个文件"，与"总数 ＝ 甲 ＋ 乙"不同阶。

此外：gap 里出现拉丁字母以外的量纲／关系／强调符号（`* _ < > ≥ ≤ ~ ^ | → % -` 等）
或减法记号 ⇒ reason=noise（本台账**只判加法与乘法**的分解式，减法不在射程内，明说而不是猜）。
拉丁字母做标签（`Setup 1 + BatchA 12`）⇒ reason=latin：那是英文任务清单的写法，
与"甲 ＋ 乙 ＝ 丙"的中文分解式不同族，混进来会把清单条目读成求和项。

以上之外一律**按 reason 点名进盲区账**（glued／latin／ascii_eq／increment／code／noise／no_fw_eq），
既不静默放过也不判红。盲区不是缺陷，把盲区读成"没有缺陷"才是缺陷。
"""

from __future__ import annotations

import ast
import re

# 引文通道：「…」／“…” 里登记的是"某轮曾印错成什么"，不是本文件的账。
QUOTE_SPAN_RE = re.compile(r"[「“][^」”]*[\u300d\u201d]")
CODE_SPAN_RE = re.compile(r"`[^`\n]*`")
# 句子终止符＝链不许跨句（`… ＝ 2；40~70 ＝ 1` 必须从分号切断）。
CUT_RE = re.compile(r"[。；;！？\n]")
# 数字串按最大取：把 `.1`／`_000` 一起吃进来，好在下一步判粘连。
TOKEN_RE = re.compile(r"\d+(?:[._]\d+)*")
# ×／÷ 是算符不是字母，所以拉丁档要把它们从 À-ÿ 里挖出去（U+00D7／U+00F7）。
_LATIN = "A-Za-z\u00C0-\u00D6\u00D8-\u00F6\u00F8-\u024F"
LETTER_RE = re.compile("[" + _LATIN + r"㐀-鿿]")
LATIN_RE = re.compile("[" + _LATIN + "]")
# 本仓"我在写一条等式"的两个记号，词汇只在这里命名一次。
# `check-doc-formulas.py --recognizer` 用它去问"还有谁在正则里自带这套记号"（第二把尺＝第二个源），
# 所以下面三根针必须从这两个常量拼出，模式串与手写基准逐字节相同由变异体 10 钉住。
EQ_FW = "＝"
ADD_FW = "＋"
EQ_FW_RE = re.compile(EQ_FW)
EQ_HW_RE = re.compile(r"=")
ADD_RE = re.compile("[+" + ADD_FW + "]")
MUL_RE = re.compile(r"×")
# 候选链的准入只要"看见任何算符／关系符"，判决权在 NOISE_RE 与全角等号那两关——
# 关系符留在算符集里正是为了让 `甲 ＝ 乙 ≥ 丙 ＋ 丁` 这种链进盲区账而不是静默断链。
ANY_OP_RE = re.compile("[=" + EQ_FW + "+" + ADD_FW + r"×≥≤<>~→]")
# gap 里出现这些字符＝这段叙述不是纯加法等式（强调、下标、关系符、箭头、量纲、减法）。
NOISE_RE = re.compile(r"[*_<>≥≤~^|→←↑↓%\-－—–]")
# 主张的分母里两个数之间最多隔这么多字符。
MAX_GAP = 40

BLIND_REASONS = ("glued", "latin", "ascii_eq", "increment", "code", "noise", "no_fw_eq")


def _glued(text, m):
    tok = m.group(0)
    # 2026-10-03 实测过「把数＋中文量词（个/层/条/步…）从粘连里放行」：全语料只多认出一条真等式
    # （架构文档里那笔 域×层＝crate 的账），而那一条已改文档写法（数字与量词间补空格＋全角等号）
    # 归一化进复算。为一条账放松唯一算源不划算——改尺子的收益是个位数，改尺子的风险是
    # 六十八格自检、模板有针与同刻对照三条不变式。所以结论钉在这里：
    # **要判的等式去改文档写法，别改尺子**；量词粘连是对的，它是"标识符的一部分"这条真判据。
    if "." in tok or "_" in tok:
        return True
    a, b = text[:m.start()], text[m.end():]
    if a and (LETTER_RE.match(a[-1]) or a[-1].isdigit() or a[-1] == "_"):
        return True
    if b and (LETTER_RE.match(b[0]) or b[0].isdigit() or b[0] in "_%*"):
        return True
    return False


def _adjacent_add(gap, nxt_start, ln):
    """加号若紧贴后面的数字（`+1`／`＋9`，中间没有空格），那是增量记号，不是二元加号。"""
    for m in ADD_RE.finditer(gap):
        if m.end() == len(gap) and ln[nxt_start:nxt_start + 1].isdigit():
            return True
    return False


def _chains(ln, toks):
    """按"任何算符"建最大候选链；被硬边界切断时把已有链收尾。toks 元素＝(m, start, end)。"""
    out, cur = [], []
    for t in toks:
        if not cur:
            cur = [t]
            continue
        gap = ln[cur[-1][2]:t[1]]
        if len(gap) > MAX_GAP or CUT_RE.search(gap) or not ANY_OP_RE.search(gap):
            if len(cur) >= 2:
                out.append(cur)
            cur = [t]
            continue
        cur.append(t)
    if len(cur) >= 2:
        out.append(cur)
    return out


def _spans(code_spans):
    return [(m.start(), m.end()) for m in (c[0] for c in code_spans)]


def classify(ln, chain, code_spans):
    """候选链 → dict(kind=claim|blind, reason, span, vals, ok)。链元素＝(m, start, end)。"""
    def blind(reason):
        return dict(kind="blind", reason=reason, span=span, vals=None, ok=None)

    span = ln[chain[0][1]:chain[-1][2]]
    gaps = [ln[chain[i][2]:chain[i + 1][1]] for i in range(len(chain) - 1)]
    if any(_glued(ln, item[0]) for item in chain):
        return blind("glued")
    a, b = chain[0][1], chain[-1][2]
    for cs, ce in code_spans:
        if cs < b and ce > a:
            return blind("code")
    if any(LATIN_RE.search(g) for g in gaps):
        return blind("latin")
    if any(NOISE_RE.search(g) for g in gaps):
        return blind("noise")
    if any(EQ_HW_RE.search(g) for g in gaps):
        return blind("ascii_eq")
    if not any(EQ_FW_RE.search(g) for g in gaps):
        return blind("no_fw_eq")
    if not any(ADD_RE.search(g) or MUL_RE.search(g) for g in gaps):
        return blind("no_fw_eq")
    # 取值：全角 ＝ 切成若干侧，侧内按 ＋／× 折叠
    groups = [[]]
    for idx, item in enumerate(chain):
        groups[-1].append((idx, item))
        if idx < len(gaps) and EQ_FW_RE.search(gaps[idx]):
            groups.append([])
    vals = []
    for grp in [g for g in groups if g]:
        total, run, prev = 0, None, None
        for idx, item in grp:
            v = int(item[0].group(0))
            if prev is None:
                run = v
            else:
                gap = gaps[prev]
                if _adjacent_add(gap, item[1], ln):
                    return blind("increment")
                if MUL_RE.search(gap) and not ADD_RE.search(gap):
                    run *= v
                elif ADD_RE.search(gap):
                    total += run
                    run = v
                else:
                    return blind("increment")
            prev = idx
        vals.append(total + run)
    return dict(kind="claim", span=span, vals=vals, ok=len(set(vals)) == 1, reason=None)


def scan_text(text, blank_lines=()):
    """全文扫描 ⇒ claims／violations／blind（带 reason）／quoted。
    `blank_lines`＝(起, 止) 行号闭区间：仪器托管块内的表由 `--ledger` 自己保证，不在散文里复算。"""
    claims, viol, blind, quoted = [], [], [], []
    lines = text.split("\n")
    for i, ln in enumerate(lines):
        no = i + 1
        if blank_lines and blank_lines[0] <= no <= blank_lines[1]:
            continue
        qsp = [(m.start(), m.end()) for m in QUOTE_SPAN_RE.finditer(ln)]
        for q in qsp:
            seg = ln[q[0]:q[1]]
            if EQ_FW_RE.search(seg) or (EQ_HW_RE.search(seg) and ADD_RE.search(seg)):
                quoted.append(dict(line=no, span=seg))
        code_spans = [(m, None, None) for m in CODE_SPAN_RE.finditer(ln)]
        csp = _spans(code_spans) + qsp
        toks = []
        for m in TOKEN_RE.finditer(ln):
            if any(a <= m.start() and m.end() <= b for a, b in csp):
                continue
            toks.append((m, m.start(), m.end()))
        for ch in _chains(ln, toks):
            if any(a <= ch[0][1] and ch[-1][2] <= b for a, b in csp):
                continue
            r = classify(ln, ch, csp)
            rec = dict(line=no, span=r["span"], vals=r.get("vals"), reason=r.get("reason"))
            if r["kind"] == "blind":
                blind.append(rec)
                continue
            claims.append(rec)
            if not r["ok"]:
                viol.append(rec)
    return dict(claims=claims, violations=viol, blind=blind, quoted=quoted)


def _acc_subscript(node, acc_names):
    """`agg["key"]` 这类下标读写点：返回 (宿主名, 键名)，否则 None。"""
    if (isinstance(node, ast.Subscript) and isinstance(node.value, ast.Name)
            and node.value.id in acc_names and isinstance(node.slice, ast.Index)):
        k = node.slice.value
        if isinstance(k, ast.Str):
            return node.value.id, k.s
    if (isinstance(node, ast.Subscript) and isinstance(node.value, ast.Name)
            and node.value.id in acc_names and isinstance(node.slice, ast.Str)):
        return node.value.id, node.slice.s
    return None


def driven_provenance(src_text, prefixes=(), acc_names=("agg",)):
    """`driven` 档的键级复算：那条红真由退出码读的那个键驱动吗。

    `red_accum_screen` 的 `driven` 只到名字一级（循环与 return 都写到 `agg` 就算）。这里补两型更强的同源：
      "same-iter"：驱动这条红的 `for` 的可迭代表达式，与 return 表达式里某一段**逐字相同**
                  （`ast.get_source_segment` 取原文，不靠 ast.dump 的写法）；
      "pair"：红的循环走的是 `agg[键A]`（一张明细表），而 `agg[键B]` 的累加恰是"同一个来源表达式的长度"
             ——两处写在同一个 `for` 里 ⇒ 键B 是键A 的计数，return 读键B 就等于读这条红的同一批数据。
    两型都不成立就退回 "name-only"（＝名字级，正是 `red_accum_screen` 已经说过的那一层），
    名单照旧按名点名：计数与明细分家、或 return 换了别的键，这一格当场红。
    只读源码图像，不执行任何东西。"""
    lines = src_text.split("\n")
    tree = ast.parse(src_text)
    funcs = [n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef)]

    def seg(node):
        got = ast.get_source_segment(src_text, node)
        return " ".join(got.split()) if got else ""

    def innermost_for(fn, line):
        cand = [n for n in ast.walk(fn) if isinstance(n, ast.For)
                and n.lineno <= line <= n.end_lineno]
        return min(cand, key=lambda n: n.end_lineno - n.lineno) if cand else None

    def host_fn(name):
        for f in funcs:
            if f.name == name:
                return f
        return None

    # 每张明细表（append 进 agg[键]）与每个计数（agg[键] += len(来源)）的构造点
    lists, counts = {}, {}
    for n in ast.walk(tree):
        if (isinstance(n, ast.Expr) and isinstance(n.value, ast.Call)
                and isinstance(n.value.func, ast.Attribute) and n.value.func.attr == "append"):
            got = _acc_subscript(n.value.func.value, set(acc_names))
            if got:
                lists.setdefault(got[1], []).append(n.lineno)
        elif isinstance(n, (ast.AugAssign, ast.Assign)):
            tgt = n.target if isinstance(n, ast.AugAssign) else n.targets[0]
            got = _acc_subscript(tgt, set(acc_names))
            if got and isinstance(n.value, ast.Call) and isinstance(n.value.func, ast.Name) \
                    and n.value.func.id == "len" and n.value.args:
                counts.setdefault(got[1], []).append((n.lineno, seg(n.value.args[0])))

    def for_chain(lineno):
        return set(x.lineno for x in ast.walk(tree) if isinstance(x, ast.For)
                   and x.lineno <= lineno <= x.end_lineno)

    def innermost_for_of_line(lineno):
        cand = [x for x in ast.walk(tree) if isinstance(x, ast.For)
                and x.lineno <= lineno <= x.end_lineno]
        return min(cand, key=lambda x: x.end_lineno - x.lineno) if cand else None

    out = []
    for head, fname, ln, kind, _note in red_accum_screen(src_text, prefixes, ("bad",)):
        if kind != "driven":
            continue
        fn = host_fn(fname)
        drv = innermost_for(fn, ln) if fn is not None else None
        if drv is None:
            out.append((head, fname, ln, "name-only", "driven 却找不到宿主 for（判据自相矛盾）"))
            continue
        iter_seg = seg(drv.iter)
        ret_segs = [" ".join(s.split()) for s in
                    (seg(n.value) for n in ast.walk(fn) if isinstance(n, ast.Return) and n.value)]
        hit_ret = [s for s in ret_segs if iter_seg and iter_seg in s]
        got = _acc_subscript(drv.iter, set(acc_names))
        if hit_ret:
            out.append((head, fname, ln, "same-iter", "循环集合 %s 逐字出现在 return（%d 行）"
                        % (iter_seg, ln)))
        elif got:
            key = got[1]
            pairs = []
            for ck, sites in counts.items():
                for cl, src_seg in sites:
                    for ll in lists.get(key, []):
                        host = innermost_for_of_line(ll)
                        if host is not None and for_chain(cl) & for_chain(ll) \
                                and src_seg and src_seg == seg(host.iter):
                            pairs.append((ck, cl, ll))
            if pairs:
                ck, cl, ll = pairs[0]
                keys_in_ret = set()
                for n in ast.walk(fn):
                    if isinstance(n, ast.Return) and n.value is not None:
                        for m in ast.walk(n.value):
                            g = _acc_subscript(m, set(acc_names))
                            if g:
                                keys_in_ret.add(g[1])
                if ck in keys_in_ret:
                    out.append((head, fname, ln, "pair", "明细 %s（append@%d）的计数键 %s 被 return 读"
                                % (key, ll, ck)))
                else:
                    out.append((head, fname, ln, "name-only",
                                "计数键 %s 不在 return 读的键集 %s 里" % (ck, sorted(keys_in_ret))))
            else:
                out.append((head, fname, ln, "name-only",
                            "明细表 %s 没有同循环的长度累加点（append@%s）" % (key, lists.get(key))))
        else:
            out.append((head, fname, ln, "name-only", "循环集合 %s 不逐字出现在任何 return" % iter_seg))
    return out


def blind_counts(blind):
    out = {}
    for b in blind:
        out[b["reason"]] = out.get(b["reason"], 0) + 1
    return out


OUTLET_MIN_PREFIX = 4


def first_const(node):
    """取一条输出语句里最左边的字面量：那条串的头就是这条红的名字。"""
    if isinstance(node, ast.Constant) and isinstance(node.value, str):
        return node.value
    if isinstance(node, ast.BinOp):
        return first_const(node.left)
    if isinstance(node, ast.JoinedStr):
        for v in node.values:
            got = first_const(v)
            if got is not None:
                return got
    return None


def red_outlet_sites(src_text, form, prefixes=()):
    """判决出口的站点：一处输出算一个站点，带着它落在哪个函数、哪一行；宇宙从它派生。

    两份仪器的红有两种形状，共用同一套取头／截断／嵌套递归的规则——规则写两遍就是两个口径，
    而这两本账必须能对上同一条红：
      form="append"：把消息塞进名为 `bad` 的列表累加器（红是数据）；
      form="print"：打一条以 `prefixes` 打头的文案，再给整数加一（红是文案）。
    截断后短于 `OUTLET_MIN_PREFIX` 的头等于"什么红都算命中"，按整串收。"""
    found = []

    def hit(node):
        if form == "append":
            return (isinstance(node.func, ast.Attribute) and node.func.attr == "append"
                    and isinstance(node.func.value, ast.Name) and node.func.value.id == "bad")
        return isinstance(node.func, ast.Name) and node.func.id == "print"

    def scan(fn):
        for n in ast.walk(fn):
            if not (isinstance(n, ast.Call) and n.args and hit(n)):
                continue
            head = first_const(n.args[0])
            if head is None:
                continue
            if form == "print" and not head.startswith(prefixes):
                continue
            cut = head.split("%")[0].rstrip()
            found.append((cut if len(cut) >= OUTLET_MIN_PREFIX else head, fn.name, n.lineno))
        for n in ast.walk(fn):
            if isinstance(n, ast.FunctionDef) and n is not fn:
                scan(n)

    for n in ast.parse(src_text).body:
        if isinstance(n, ast.FunctionDef):
            scan(n)
    return found


def red_outlet_universe(src_text, form, prefixes=()):
    return sorted({p for p, _f, _l in red_outlet_sites(src_text, form, prefixes)})


def _acc_uses(node, name):
    for n in ast.walk(node):
        if isinstance(n, ast.Name) and n.id == name:
            return True
    return False


def red_accum_screen(src_text, prefixes=(), acc_names=("bad",)):
    """print 形态的出口：文案印出去之后，那里到底有没有一条把它算进判决的动作。

    这台仪器的红是"打一条带标签的文案，再把整数加一"——两条通道各自独立，只钉文案时
    "印了不拦"永远隐形（本轮实测：三份工件全删仍 LEDGER PASS／rc=0，根因是 `bad = 0`
    这条初值落在两条红之后，累加无处可加）。逐站点给四档，档位不许被读成"没问题"：
      "inc"   同一函数内、这条文案之后先遇到累加器动作（`A += ...` 或 `A = A + ...`）；
      "ret"   本函数自己不累加，但 `return` 一个非零整数，且调用点把返回值加进累加器
              （同行 `bad += f(...)` 或 `x = f(...)` 后由 `if x:` 分支加一，两型都认）；
      "reset" 这条文案之后先遇到 `A = ...`（无牙的那个形状）；
      "driven" 文案住在一个 `for` 里，而本函数的退出码读的是同一个名字（弱判据：只证同名，
              键级同源要另读构造处；它不豁免任何人，只把"看不见"换成"看得见并按名点名"）；
      "blind" 以上三档都不成立，函数里此后连累加器都不碰。
    这一档不许被读成"没问题"，但也不许被读成"有缺陷"：本尺认累加器、返回值、循环集合三种通道，
    第三种最早在 `check-doc-formulas.py` 的 `cmd_census` 上撞到（两条 FAIL 由
    `agg["details"]`／`outside_verdict(out)` 驱动，退出码读的是同一批名字），于是建了 `driven`。
    `driven` 只到名字一级 ⇒ 名单要按名点名，键级同源（`violations` 恰是 `details` 的计数）无尺；
    真落不进三档的才叫 blind，逐站点印出来说明"这一处本尺没看见"，绝不折叠成一条绿。
    只读源码图像，不执行任何东西。调用点的查找按函数名匹配（同名多处的语言在这里没有）。"""
    acc = set(acc_names)
    tree = ast.parse(src_text)
    src_lines = src_text.split("\n")
    funcs = [n for n in ast.walk(tree) if isinstance(n, ast.FunctionDef)]

    def innermost(line):
        cand = [f for f in funcs if f.lineno <= line <= f.end_lineno]
        if not cand:
            return None
        return min(cand, key=lambda f: f.end_lineno - f.lineno)

    def events(fn):
        out = []
        for n in ast.walk(fn):
            if isinstance(n, ast.AugAssign) and isinstance(n.target, ast.Name) and n.target.id in acc:
                out.append((n.lineno, "inc", n.target.id))
            elif isinstance(n, ast.Assign):
                for t in n.targets:
                    if isinstance(t, ast.Name) and t.id in acc:
                        out.append((n.lineno, "inc" if _acc_uses(n.value, t.id) else "reset", t.id))
        return sorted(out)

    def returns_nonzero(fn):
        """本函数有没有可能返回非零：`return` 的表达式子树里出现非零整数字面量即算。

        只认 `return 1` 会把 `return 0 if ok else 1`（本仓 `audit_len_guard` 的真写法）判成
        无返回值通道 ⇒ 有牙的红被读成 blind。放宽到有字面量即可，方向是保守的：
        它只会少报 blind，不会把真的无牙形状放过（无牙的那型是整条通道都不碰累加器）。"""
        for n in ast.walk(fn):
            if not isinstance(n, ast.Return) or n.value is None:
                continue
            for m in ast.walk(n.value):
                if isinstance(m, ast.Constant) and isinstance(m.value, int) and m.value:
                    return True
        return False

    def call_site_accum(fn):
        """调用点有没有把返回值算进累加器。实测有两种写法，只认同行那种会把好代码判成 blind：
          "inline"：`bad += fn(...)` 同一行；
          "guarded"：`x = fn(...)` 存进变量，随后 `if x:`（或 `if not x:`）的分支里给累加器加一
                    ——本轮在本仓的 `audit_len_guard` 上撞到的正是这一型（1940 存、1941 判、1942 加）。
        两型都只按行文本匹配并点名行号，档位由人复核；匹配不到就退回 blind，不许静默过。"""
        inline = re.compile(r"\b(" + "|".join(sorted(acc)) + r")\s*\+=.*\b" + re.escape(fn.name) + r"\s*\(")
        for i, s in enumerate(src_lines):
            if inline.search(s):
                return i + 1, "inline"
        assign = re.compile(r"^\s*(\w+)\s*=\s*" + re.escape(fn.name) + r"\s*\(")
        for i, s in enumerate(src_lines):
            m = assign.match(s)
            if not m:
                continue
            var = m.group(1)
            tester = re.compile(r"^\s*if\s+(not\s+)?" + re.escape(var) + r"\b")
            bump = re.compile(r"\b(" + "|".join(sorted(acc)) + r")\s*[-+]?=")
            for j in range(i + 1, min(i + 24, len(src_lines))):
                if tester.match(src_lines[j]):
                    for k in range(j + 1, min(j + 5, len(src_lines))):
                        if bump.search(src_lines[k]):
                            return k + 1, "guarded"
                    break
        return None, None

    def names_of(node):
        return {n.id for n in ast.walk(node) if isinstance(n, ast.Name)}

    def enclosing_for(fn, line):
        """包着这条文案的最里层 `for`：第三型通道的判据是"文案由哪个集合驱动"。"""
        cand = [n for n in ast.walk(fn) if isinstance(n, (ast.For, ast.AsyncFor))
                and n.lineno <= line <= n.end_lineno]
        if not cand:
            return None
        return min(cand, key=lambda n: n.end_lineno - n.lineno)

    def return_names(fn):
        out = set()
        for n in ast.walk(fn):
            if isinstance(n, ast.Return) and n.value is not None:
                out |= names_of(n.value)
        return out

    out = []
    for head, fname, ln in red_outlet_sites(src_text, "print", prefixes):
        fn = innermost(ln)
        if fn is None:
            out.append((head, fname, ln, "blind", "找不到宿主函数"))
            continue
        nxt = [e for e in events(fn) if e[0] > ln]
        kind = nxt[0][1] if nxt else None
        note = "%s@%d" % (nxt[0][1], nxt[0][0]) if nxt else "本函数内此后不碰累加器"
        if kind != "inc":
            at, shape = call_site_accum(fn)
            if at and returns_nonzero(fn):
                kind, note = "ret", "%s 返回值在 %d 行被累加" % (shape, at)
            elif kind is None:
                # 第三型：文案住在一个 `for` 里，而本函数的退出码读的是同一个名字
                # （`check-doc-formulas.py` 的 `cmd_census`：`for v in agg["details"]` 与
                #  `return 1 if (agg["violations"] or outside_verdict(out))` 共用 `agg`／`out`）。
                # 这一档是**弱**判据——它只证明"驱动这条红的那个名字也出现在退出码里"，
                # 键级同源（violations 是 details 的计数）要另读构造处；所以它不豁免任何人，
                # 只把"看不见"换成"看得见但按名点名"。
                drv = enclosing_for(fn, ln)
                shared = sorted(names_of(drv.iter) & return_names(fn)) if drv is not None else []
                if shared and returns_nonzero(fn):
                    kind, note = "driven", "循环集合 %s 同名出现在本函数 return（%s）" % (
                        "/".join(shared), fname)
                else:
                    kind = "blind"
        out.append((head, fname, ln, kind, note))
    return out
