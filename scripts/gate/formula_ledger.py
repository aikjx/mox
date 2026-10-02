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


def blind_counts(blind):
    out = {}
    for b in blind:
        out[b["reason"]] = out.get(b["reason"], 0) + 1
    return out
