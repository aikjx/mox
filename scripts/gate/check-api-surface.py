#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""API 表面普查器（P1a，口径见 docs/architecture/API-SURFACE-AUTHORITY-PLAN-v0.1.md）。

它回答一个此前没有权威答案的问题：网关对外到底挂了多少条路由，
以及 `actuator.rs` 的 ROUTES 静态表（docs/API-REGISTRY.md 的唯一生成源）与真实挂载差多少。

为什么要解析 nest 链而不是 grep：`.route("/templates")` 在
`build_enterprise_router_for_gateway` 里被 `.nest("/admin", ...)` 再被
`modules.rs` 的 `.nest("/api/enterprise", ...)` 逐层加前缀，
直接 grep 字面量得到的是局部路径，拿去和 ROUTES 里的全路径比必然双向假账。

模式：
  --census     打印普查账（默认）。只出数字与点名清单，不做判决。
  --selftest   在内置 Rust 源片段夹具上验解析器（含变异体：撤 nest 前缀必须改变结果）。
  --check      判决模式。当前未启用，见 main 里的拒绝分支与其理由。

注意 --check 为什么还没开：判据的覆盖面按形状算，不按文件数算。
普查器现在能解析的形状 = selftest 里那 6 种，其余形状必须先由 --census 实测点名，
把 UNRESOLVED 清零或逐条归因之后，才有资格把数字钉成门禁。
"""

import argparse
import json
import os
import re
import sys
from datetime import datetime

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import formula_ledger as FL  # noqa: E402  「什么是一条算术主张」只在那份模块里判一次

# 扫描集：一切可能往 Router 里挂路由的源目录（不含 tests/，测试自己造的 Router 不是对外表面）。
SCAN_ROOTS = ["platform/gateway", "platform/domains", "platform/foundation", "platform/shared", "projects"]
FN_CALL_RE = re.compile(r"\b(build_[A-Za-z0-9_]*router[A-Za-z0-9_]*|build_[a-z0-9_]*_with_state)\b")
CHAIN_STEP_RE = re.compile(r"\.(route|nest|merge)\s*\(")
# 兜底挂载：Router::new().fallback(h) 没有字面路径，但它确实吃下整个前缀下的任意子路径与任意方法。
# 把它落进 UNRESOLVED 等于"看不见就不算挂载"，那会让在册未挂载那一格假绿。
FALLBACK_RE = re.compile(r"\.fallback\s*\(")
CATCH_MARK = "/{**}"
# 条件装配的"臂"维：`match role { HostRole::Kg => …, … }` 里每条臂各自挂一棵树，
# 而声明侧 ROUTES 表没有角色字段 ⇒ 一把只问"挂没挂"的尺子会把"只在某个角色下挂"读成"到处都挂"。
MATCH_RE = re.compile(r"\bmatch\s+[^{}]*\{")
ARM_PAT_RE = re.compile(r"([A-Za-z_]\w*(?:::[A-Za-z_]\w*)+)(?=\s*(?:\{[^{}]*\})?\s*(?:=>|\|))")
ROLE_TAG = "role:"
# 条件装配的第二种长相：`if role == HostRole::All { A } else { B }`。臂是"按枚举分派"，
# 分支是"按布尔分派"——`--role-matrix` 只读得出发臂那半，所以每条带臂路径都同时有一条
# 无归属读数（走根是 All，两支都被走了一遍），"0 条专属"读不出真值。
# 分支归因把**位置**当判据：落在 then 块里 ⇒ `=HostRole::All`，落在 else 块里 ⇒ `!HostRole::All`。
BRANCH_TAG = "cond:"
IF_HEAD_RE = re.compile(r"\bif\b")
ELSE_HEAD_RE = re.compile(r"\s*else\b")
IF_INNER_RE = re.compile(r"\s*if\b")
# 只认"与 HostRole:: 变体比较"这一种布尔形状，`==`／`!=` 两侧谁在前都要认（`role == deployment::HostRole::All`
# 与 `HostRole::All == role` 是同一件事的两种写法）。`matches!`、`.is_*()` 一类不在此列：
# 归不了约束的分支进 branch_unattributed 按名点名，不当成"没有分支"。
COND_EQ_RE = re.compile(r"(==|!=)\s*((?:[A-Za-z_]\w*\s*::\s*)*HostRole\s*::\s*([A-Za-z_]\w*))")
COND_REV_RE = re.compile(r"((?:[A-Za-z_]\w*\s*::\s*)*HostRole\s*::\s*([A-Za-z_]\w*))\s*(==|!=)\s*[A-Za-z_][\w.]*\s*(?![=!<>])")
HOST_ROLE_ENUM_RE = re.compile(r"\benum\s+HostRole\b[^{};]*\{([^}]*)\}")
# 装配节点的第二种长相：函数名不叫 build_*router*，但签名返回 Router
# （deployment::domain_router 就是这种，它 match 的每一条臂都是真挂载）。
RETURN_ROUTER_RE = re.compile(r"->\s*(?:impl\s+)?(?:[\w:]+\s*::\s*)?Router\b")
FN_ANY_RE = re.compile(r"\bfn\s+([A-Za-z_]\w*)\s*[<(]")
CALLEE_RE = re.compile(r"(?:[A-Za-z_]\w*\s*::\s*)*[A-Za-z_]\w*")
CALL_TAIL_RE = re.compile(r"\s*(?:::\s*<[^<>]*>)?\s*\(")
METHODS = ["get", "post", "put", "delete", "patch", "head", "options", "trace"]
MAX_DEPTH = 40
# 非路由语句：use/mod/属性/宏/可见性声明。它们不参与装配，
# 但必须与"解析不了的 Router 表达式"分开记账——混在一起会把覆盖盲区读成语法噪音。
NON_ROUTER_RE = re.compile(r"^(?:pub\s+)?(?:use|mod|extern)\s|^macro_rules!|^#\[|^let\s+_\b")


def read_text(path):
    """读源文件：utf-8 带 BOM 亦可，换行一律归一为 \n（源为 LF，个别工具产出为 CRLF）。"""
    with open(path, "rb") as fh:
        raw = fh.read()
    txt = raw.decode("utf-8-sig", errors="replace")
    return txt.replace("\r\n", "\n").replace("\r", "\n")


def mask(src):
    """把注释替换为等长空白，并把字符串字面量内部会影响括号深度/逗号切分的字符换成 '-'。

    掩码后必须仍能读出路径字面量本身（引号保留），否则 `.route("/api", ...)` 的
    第一个实参变成空白，解析器会把每一条挂载都判成"首参非字面量"而静默漏数。
    换掉 (){}[],; 是为防路径里的括号把深度切错；换掉的字符在真实路径里不出现
    （若出现，normalize 之前会先落到 UNRESOLVED，不会假阴）。
    """
    out = list(src)
    i, n = 0, len(src)
    st = 0  # 0 code, 1 dquote, 2 line comment, 3 block comment
    depth = 0
    while i < n:
        c = src[i]
        nx = src[i + 1] if i + 1 < n else ""
        if st == 0:
            if c == '"':
                st = 1
            elif c == "/" and nx == "/":
                st = 2
                out[i] = " "
                out[i + 1] = " "
                i += 1
            elif c == "/" and nx == "*":
                st = 3
                depth = 1
                out[i] = " "
                out[i + 1] = " "
                i += 1
            i += 1
        elif st == 1:
            if c == "\\":
                out[i] = " "
                if i + 1 < n:
                    out[i + 1] = " "
                    i += 1
            elif c == '"':
                st = 0
            elif c == "\n":
                st = 0  # Rust 字符串不裸跨行；跨行说明上游漏了转义，宁可当断点
            elif c in "(){}[],;":
                out[i] = "-"
            i += 1
        elif st == 2:
            if c == "\n":
                st = 0
            else:
                out[i] = " "
            i += 1
        elif st == 3:
            if c == "*" and nx == "/":
                depth -= 1
                out[i] = " "
                out[i + 1] = " "
                i += 2
                if depth == 0:
                    st = 0
            else:
                out[i] = " " if c != "\n" else c
                i += 1
    return "".join(out)


def find_body(marked, name):
    """返回 fn name 的花括号体 (start, end)，坐标在 marked 上。找不到返回 None。"""
    m = re.search(r"\bfn\s+" + re.escape(name) + r"\s*[<(]", marked)
    if not m:
        return None
    brace = marked.find("{", m.end())
    if brace < 0:
        return None
    d, i, n = 0, brace, len(marked)
    while i < n:
        if marked[i] == "{":
            d += 1
        elif marked[i] == "}":
            d -= 1
            if d == 0:
                return brace + 1, i
        i += 1
    return None


def split_args(argtext):
    """按顶层逗号切分实参（argtext 已去掉外层括号）。不按尖括号计深度：
    泛型尖括号在 Rust 里可与比较运算符同形，计入会把切分走偏；实参里的泛型不含顶层逗号。"""
    parts, buf, d = [], [], 0
    for ch in argtext:
        if ch in "([{":
            d += 1
        elif ch in ")]}":
            d -= 1
        if ch == "," and d == 0:
            parts.append("".join(buf))
            buf = []
        else:
            buf.append(ch)
    parts.append("".join(buf))
    return [p.strip() for p in parts]


def call_at(marked, pos):
    """pos 指向 '(' ，返回 (内层文本, 右括号之后的下标)。"""
    d, i, n = 0, pos, len(marked)
    while i < n:
        if marked[i] in "([{":
            d += 1
        elif marked[i] in ")]}":
            d -= 1
            if d == 0:
                return marked[pos + 1:i], i + 1
        i += 1
    return marked[pos + 1:], n


def methods_of(handler):
    """从 route 的第二实参里取 HTTP 方法集。

    前导必须用 (?<!\w) 而不是 [^\w]：后者要求 get 之前实有其字符，于是链里第一个方法
    （实参以 get(x1) 开头）恒读不到，而 axum::routing::get 与 .post( 却读得到——
    账面表现为"方法集缺失"，比全错更难发现。
    """
    found = []
    for meth in METHODS:
        if re.search(r"(?<!\w)" + meth + r"\s*\(", handler):
            found.append(meth.upper())
    return sorted(found)


_PAIRS = {"(": ")", "[": "]", "{": "}"}


def span_from(marked, open_idx):
    """从 open_idx 处的开括号走到配对闭括号，返回内容区间 (起, 止)。
    mask() 已把字符串里的括号换成 '-'，所以纯按深度扫是安全的；括号种类不配对即判畸形。"""
    stack = []
    for i in range(open_idx, len(marked)):
        c = marked[i]
        if c in _PAIRS:
            stack.append(_PAIRS[c])
        elif c in ")]}":
            if not stack:
                return None
            if stack[-1] != c:
                return None
            stack.pop()
            if not stack:
                return open_idx + 1, i
    return None


def index_fn_defs(marked):
    """扫一个（已 mask 的）源里所有 fn，返回 [(name, lo, hi, builder, router_ret)]。

    以前只按名字认 build_*，于是"名字不规范但确实返回 Router"的装配节点整条读成看不见：
    deployment::domain_router 就是这么从 `if role == All { … } else { … }` 的第二支里静默消失的。
    判据改成读签名（-> Router），不再猜名字。
    """
    out = []
    for m in FN_ANY_RE.finditer(marked):
        op = marked.find("(", m.end() - 1)
        if op < 0:
            continue
        args = span_from(marked, op)
        if not args:
            continue
        brace = marked.find("{", args[1])
        if brace < 0:
            continue
        body = span_from(marked, brace)
        if not body:
            continue
        sig = marked[args[1]:brace]
        out.append((m.group(1), body[0], body[1],
                    m.group(1).startswith("build_"),
                    bool(RETURN_ROUTER_RE.search(sig))))
    return out


def is_cond_token(tok):
    """分支约束的写法：`=` 正支／`!` 负支。臂标签是裸枚举路径，两族靠首字符分档。"""
    return tok[:1] in "=!"


ARM_ROLE_RE = re.compile(r"^HostRole::([A-Za-z_]\w*)$")
COND_ROLE_RE = re.compile(r"^([=!])HostRole::([A-Za-z_]\w*)$")


def constraint_ok(tok, variant):
    """合取项 `tok` 在角色取值 `variant` 下成不成立：True／False／None（None＝这一项不是角色维）。

    三种来源共用一把尺：臂的正模式 `HostRole::Kb`、分支正支 `=HostRole::All`、
    分支负支 `!HostRole::All`。同一臂里写多个枚举模式（`HostRole::A | HostRole::B`）时标签
    用 `|` 连，它是**析取**——按"任一模式吃下该取值"判；折成合取会把这条臂读成永不可达。
    非 HostRole 的模式（按 tier 的那把 match）不参与角色判决，但必须被数出来：
    悄悄忽略等于把"这一维不是角色"读成"这一维没有"。
    """
    if "|" in tok:
        seen = [constraint_ok(t, variant) for t in tok.split("|") if t]
        seen = [x for x in seen if x is not None]
        return any(seen) if seen else None
    m = COND_ROLE_RE.match(tok)
    if m:
        return (m.group(2) != variant) if m.group(1) == "!" else (m.group(2) == variant)
    m = ARM_ROLE_RE.match(tok)
    return None if not m else (m.group(1) == variant)


def conj_ok(conj, variant, use_branch=True):
    """一条合取（走到某挂载位点所经过的全部条件）在 `variant` 下可不可满足。

    `use_branch=False` 是给第二把尺子用的：只看臂、不看分支，两把尺子的差集就是
    分支归因真正吃进判决的那部分——差集为空而分支非空，说明这一维只是装饰。
    """
    for t in conj:
        if is_cond_token(t):
            if use_branch and constraint_ok(t, variant) is False:
                return False
        elif constraint_ok(t, variant) is False:
            return False
    return True


def host_role_variants(cen):
    """角色取值清单读自枚举声明（`pub enum HostRole { All, Kg, … }`），不硬编那五个名。

    读不到就退回"约束标签里实际出现过的变体"并印出退回事实——桶少一档会把整张账读小。
    """
    for rel, mk in sorted(cen.marked.items()):
        m = HOST_ROLE_ENUM_RE.search(mk)
        if not m:
            continue
        vals = [x.strip() for x in m.group(1).split(",")]
        vals = [v for v in vals if re.fullmatch(r"[A-Za-z_]\w*", v)]
        if vals:
            return vals, rel, cen.line_of(rel, m.start())
    return [], "", None


class Census:
    def __init__(self):
        self.fns = {}          # fn name -> list[(path, body_lo, body_hi)]
        self.marked = {}       # path -> masked text
        self.mounted = {}      # (full_path, frozenset(methods)) -> list of origins
        self.unresolved = {}   # reason -> list of "file:line fn-name"
        self.skipped = []      # 非路由语句（use/mod/属性），只记数不入账
        self.stack = []        # 当前解析路径上的 builder，用于识别自引用环
        self.catchall = []     # (前缀, 出处)：.fallback() 型兜底挂载，无字面路径但真吃子路径
        self.n_builder = 0     # 按名字在册的 build_* 函数数
        self.n_router = 0      # 按签名（-> Router）新增的装配节点数，即名字不规范那批
        self.passthrough = 0   # 走进去只做了状态升级/无挂载的壳函数次数
        self.role_arms = set()   # (文件, 臂起点, 归属标签)：真被读到的枚举臂，按位点去重
        self.role_wild = set()   # (文件, 臂起点)：`_ =>` 这类读得出边界却归不了属的臂
        self.cond_spans = set()  # (文件, 分支块起点, 约束标签)：`if role == HostRole::X` 的两支，按位点去重
        self.cond_blind = set()  # (文件, 分支块起点)：读得出块边界、归不出角色约束的 if/else
        self.pin = None          # 具体执行时钉死的角色值（None＝符号走树，两支都走）
        self.prune_spans = 0     # 钉死取值后真的被剪掉的区间**次数**（剪枝没落地时这一格应为 0）
        self.prune_intervals = set()  # (文件, 绝对起, 绝对止)：去重后的"几处"——同一段区间会在
        # 函数体／剥离后的表达式／整文件三个坐标基上各剪一次，累计数会把它读成三处。
        self.entered = []      # 每次真的走进一个装配节点：(函数名, 定义所在文件)
        self.scanned = 0

    def load(self, roots):
        for root in roots:
            base = os.path.join(REPO, root)
            if not os.path.isdir(base):
                continue
            for dirpath, _dirs, files in os.walk(base):
                if os.sep + "tests" + os.sep in dirpath + os.sep:
                    continue
                for fn in files:
                    if fn.endswith(".rs"):
                        self.index_file(os.path.join(dirpath, fn))
        return self

    def index_file(self, path):
        src = read_text(path)
        rel = os.path.relpath(path, REPO).replace(os.sep, "/")
        mk = mask(src)
        self.marked[rel] = mk
        self.scanned += 1
        self.index_text(rel, mk)

    def index_text(self, rel, mk):
        """装配节点在册：**只认签名返回 Router 的 fn**。

        以前按名字把一切 build_* 都收进册，于是 build_messages/build_query 这类
        与装配无关的函数也成了走树目标（实测撞出 19 个同名歧义名）。名字只是线索，
        返回类型才是判据；n_builder 仍按名字分档，是为了把"不规范命名的那批"点出来。"""
        for name, lo, hi, builder, router_ret in index_fn_defs(mk):
            if not router_ret:
                continue
            self.fns.setdefault(name, []).append((rel, lo, hi))
            if builder:
                self.n_builder += 1
            else:
                self.n_router += 1

    def line_of(self, rel, idx):
        return self.marked[rel][:idx].count("\n") + 1

    def resolve_expr(self, expr, base, origin, depth, env=None, rel=None, offset=0, role=None):
        """把一个 Router 表达式解析成若干挂载点：链、函数引用、兜底 fallback 三种形态都要认，
        而且一支表达式里的**每一支**都要走——`if role == All { build_module_routers(..) }
        else { domain_router(..) }` 只跟第一支，就等于把条件装配的另一半读成不存在（静默漏数）。"""
        expr = expr.strip()
        if not expr:
            return
        if self.pin is not None:
            expr = self._prune(expr, 0, rel)
            if not expr:
                return
        if CHAIN_STEP_RE.search(expr):
            self.walk_text(expr, base, origin + " [inline]", depth, rel, offset, env, role)
            return
        # 裸函数引用形态（`if role == All { build_module_routers(..) } else { domain_router(..) }`）：
        # 分支边界就在这段表达式自己体内，标签必须按**被调位置**判——否则整条 else 支的
        # 子树会带着无标签回到账上，"角色无关的公共面"就被读成 100%（本轮实测撞出）。
        arms, _ap = self.match_arms(expr, offset, rel, ledger=False)
        conds, _cp = self.if_branches(expr, offset, rel, ledger=False)
        seen, lab = [], {}
        for pos, n in callee_calls(expr):
            if n in self.fns and n not in seen:
                seen.append(n)
                lab[n] = self.arm_label(self.arm_label(role, self.arms_at(arms, pos)),
                                        self.conds_at(conds, pos))
        for name in seen:
            self.resolve_fn_ref(name, base, origin, depth, lab[name])
        if FALLBACK_RE.search(expr):
            self.record_catchall(base, origin)
            return
        if seen:
            return
        suspects = [m.group(1) for m in FN_CALL_RE.finditer(expr)]
        if suspects:
            self.unresolved.setdefault("找不到 builder 函数定义: " + suspects[0], []).append(origin)
            return
        self.unresolved.setdefault("无法归类的表达式", []).append(origin + " :: " + expr[:70].replace("\n", " "))

    def resolve_fn_ref(self, name, base, origin, depth, role=None):
        hits = self.fns.get(name)
        if not hits:
            self.unresolved.setdefault("找不到 builder 函数定义: " + name, []).append(origin)
            return
        if len(hits) > 1:
            self.unresolved.setdefault("builder 函数同名多定义: " + name, []).append(origin)
        frel, lo, hi = hits[0]
        key = (frel, lo)
        if key in self.stack:
            self.unresolved.setdefault("自引用环: " + name, []).append(origin)
            return
        self.stack.append(key)
        try:
            inner = self.marked[frel][lo:hi]
            if not CHAIN_STEP_RE.search(inner) and not any(n in self.fns for _, n in callee_calls(inner)):
                # 状态升级壳（upgrade 的函数体就是 `router.with_state(())`）：
                # 它不挂任何路径，走它只会把形参名读成"无法归类的表达式"。
                # 判据从函数体本身推，不点名 upgrade —— 别的壳函数同样按此豁免，并计数打印。
                self.passthrough += 1
                return
            self.entered.append((name, frel))
            self.walk_body(frel, lo, hi, base, origin + " -> " + name + "@" + frel, depth, role)
        finally:
            self.stack.pop()

    def record_catchall(self, base, origin):
        """兜底挂载入账：路径记成 <前缀>/{**}，方法记 ANY。

        不记＝"读不出路径"被读成"没有挂载"；乱记成具体路径＝凭空造在册条目。"""
        prefix = base.rstrip("/") or ""
        self.catchall.append((prefix + CATCH_MARK, origin))
        self.record(prefix + CATCH_MARK, ["ANY"], origin)

    @staticmethod
    def _top_commas(text, lo, hi):
        """在 match 的花括号内容里按顶层逗号切臂（臂体的花括号自成一档深度）。"""
        segs, d, start = [], 0, lo
        for i in range(lo, hi):
            c = text[i]
            if c in "([{":
                d += 1
            elif c in ")]}":
                d = d - 1 if d > 0 else 0
            elif c == "," and d == 0:
                segs.append((start, i))
                start = i + 1
        if text[start:hi].strip():
            segs.append((start, hi))
        return segs

    @staticmethod
    def _top_arrow(seg):
        """臂体里第一个**顶层** `=>` 的位置：模式的边界。

        不切这一刀，外层臂的 label 会把嵌在它体内的第二把 match 的各臂模式一起吃进来
        （`HostRole::Cloud => match tier { CloudTier::A => … }` 会读成 Cloud+A 两维齐全），
        于是嵌套 match 的 `_ =>` 臂也被扣上内层枚举的名——多出来的维度是假的。"""
        d = 0
        for i, ch in enumerate(seg):
            if ch in "([{":
                d += 1
            elif ch in ")]}":
                d = d - 1 if d > 0 else 0
            elif d == 0 and ch == "=" and seg[i:i + 2] == "=>":
                return i
        return None

    def match_arms(self, text, offset, rel, ledger=True):
        """把 `match x { Enum::A => …, Enum::B | Enum::C => …, _ => … }` 翻成 (臂账, 要剪掉的区间)。

        标签存**完整枚举路径**（`HostRole::Kg`），因为同一份装配里可能有第二把 match
        （按 tier、按特性开关），只留末段会让两个维度撞名。`_ =>` 无枚举路径 ⇒ 进 role_wild，
        它仍然能"继承外层臂"（臂起点没在册，位置就不落进任何臂区间）。
        同一臂写多个枚举模式（`HostRole::A | HostRole::B`）是**析取**，标签用 `|` 连；
        用 `+` 连会把它读成"两个取值同时成立"⇒ 那条臂永不可达。

        `self.pin` 非空即"具体执行"：只认吃下该取值的臂（没有具名臂吃下时落到 `_` 那类兜底臂），
        其余臂整段进剪枝区间。此时不产生标签、不入臂账——标签是符号走树的产物，
        取值钉死后每个位点只有一个事实，再打标签等于把两条通道混成一条。
        非角色维的那把 match（按 `Method::` 或按数字分档）不剪：剪它等于凭空少挂一棵树。
        """
        out, pruned = [], []
        for m in MATCH_RE.finditer(text):
            ob = text.find("{", m.end() - 1)
            body = span_from(text, ob) if ob >= 0 else None
            if not body:
                continue
            lo, hi = body
            arms = []
            for a_lo, a_hi in self._top_commas(text, lo, hi):
                seg = text[a_lo:a_hi]
                arrow = self._top_arrow(seg)
                pats = ARM_PAT_RE.findall(seg[:arrow + 2] if arrow is not None else seg)
                uniq = sorted(set(pats))
                role_vals = [p for p in uniq if ARM_ROLE_RE.match(p)]
                label = ("|".join(role_vals)
                         if len(role_vals) > 1 and len(role_vals) == len(uniq) else "+".join(uniq))
                if pats:
                    if self.pin is None:
                        out.append((a_lo, a_hi, label))
                        if ledger:
                            self.role_arms.add((rel, offset + a_lo, label))
                    arms.append((a_lo, a_hi, label))
                elif seg.strip():
                    if self.pin is None and ledger:
                        self.role_wild.add((rel, offset + a_lo))
                    arms.append((a_lo, a_hi, None))
            if self.pin is None or not any("HostRole::" in (lab or "") for _a, _b, lab in arms):
                continue
            hit = [a[:2] for a in arms
                   if a[2] and any(constraint_ok(t, self.pin) for t in a[2].split("|"))]
            keep = hit or [a[:2] for a in arms if a[2] is None]
            for a_lo, a_hi, _lab in arms:
                if (a_lo, a_hi) not in keep:
                    pruned.append((a_lo, a_hi))
        return out, pruned

    @staticmethod
    def cond_label(cond):
        """从 `if` 的条件文本读出 (then 支约束, else 支约束)；不是与 HostRole 变体的比较即返回 None。

        两支必须**互斥且互补**（`==` 给一支、`!=` 给另一支），否则 All 桶会同时吃进
        注册表支和 `domain_router` 支——那是把"没判"读成"两边都到"。
        复合条件（两个及以上比较）不做布尔求解，返回 None 落进按名点名的盲区账。
        """
        if len(COND_EQ_RE.findall(cond)) + len(COND_REV_RE.findall(cond)) > 1:
            return None
        m = COND_EQ_RE.search(cond)
        if m:
            op, variant = m.group(1), m.group(3)
        else:
            m = COND_REV_RE.search(cond)
            if not m:
                return None
            op, variant = m.group(3), m.group(2)
        pos, neg = "=HostRole::" + variant, "!HostRole::" + variant
        return (pos, neg) if op == "==" else (neg, pos)

    @staticmethod
    def _else_span(text, then_hi):
        """`} else { … }` 与整条 `} else if … { … } else { … }` 链的 else 侧区间。

        链必须**整条**罩住：外层否定管到链尾，链里每一支自己再判自己的条件。
        只罩第一个块会把 `else` 收尾的那一支读成"无约束"⇒ 它凭空出现在每个角色桶里。

        `span_from` 给的是**内容**区间，其上界就是那个配对右花括号的索引本身，
        所以 `else` 要从 `hi + 1` 起匹配——在 `hi` 处匹配永远落空（首字符是 `}`，
        而 `\s*else\b` 不接受它），表现为"两支都没有 else"⇒ 负支约束整族消失。
        """
        m = ELSE_HEAD_RE.match(text, then_hi + 1)
        if not m:
            return None
        i = m.end()
        if not IF_INNER_RE.match(text, i):
            ob = text.find("{", i)
            return span_from(text, ob) if ob >= 0 else None
        end = then_hi
        while True:
            ob = text.find("{", i)
            sp = span_from(text, ob) if ob >= 0 else None
            if not sp:
                return None
            end = sp[1]
            m2 = ELSE_HEAD_RE.match(text, end + 1)
            if not m2:
                break
            i = m2.end()
            if not IF_INNER_RE.match(text, i):
                ob2 = text.find("{", i)
                sp2 = span_from(text, ob2) if ob2 >= 0 else None
                if sp2:
                    end = sp2[1]
                break
        return then_hi, end

    def if_branches(self, text, offset, rel, ledger=True):
        """把 `if cond { A } else { B }` 翻成 (分支账, 要剪掉的区间)。

        只归因**条件里点名了 HostRole 变体**的分支：装配语料里 `if` 有几百处，绝大多数是
        取值层的布尔（`if expert.title.is_empty()`），把它们也打标签等于凭空造出一堆
        不参与角色判决的维度。条件里出现了 role／HostRole 字样却归不出约束的
        （复合条件、`matches!`、函数调用型谓词）进 `cond_blind` 按位点点名——
        那是覆盖面缺口，不是噪音，静默跳过就等于假装覆盖面是满的。
        """
        out, pruned = [], []
        for m in IF_HEAD_RE.finditer(text):
            ob = text.find("{", m.end())
            if ob < 0:
                continue
            cond = text[m.end():ob]
            then = span_from(text, ob)
            if not then:
                continue
            els = self._else_span(text, then[1])
            lab = self.cond_label(cond)
            if not lab:
                if ledger and ("HostRole" in cond or re.search(r"\brole\b", cond)):
                    self.cond_blind.add((rel, offset + then[0], cond.strip()[:56]))
                continue
            if self.pin is None:
                if ledger:
                    self.cond_spans.add((rel, offset + then[0], lab[0]))
                    if els:
                        self.cond_spans.add((rel, offset + els[0], lab[1]))
                out.append((then[0], then[1], lab[0]))
                if els:
                    out.append((els[0], els[1], lab[1]))
                continue
            if constraint_ok(lab[0], self.pin) and els:
                pruned.append(els)
            elif not constraint_ok(lab[0], self.pin):
                pruned.append(then)      # 无 else 的 `if` 也一样：这一支不执行就整段剪掉
        return out, pruned

    @staticmethod
    def conds_at(conds, pos):
        """位置 pos 落在哪些分支里（外层在前）：else-if 链的 negation 与内层条件要叠加。"""
        enclosing = sorted([(lo, hi, lab) for lo, hi, lab in conds if lo <= pos < hi and lab])
        return "+".join(lab for _lo, _hi, lab in enclosing) if enclosing else None

    @staticmethod
    def arms_at(arms, pos):
        """位置 pos 落在哪些臂里（外层在前）：嵌套 match 的两维都要带上，丢掉外层就等于把内层读成唯一维。"""
        enclosing = sorted([(lo, hi, lab) for lo, hi, lab in arms if lo <= pos < hi])
        return "+".join(lab for _lo, _hi, lab in enclosing) if enclosing else None

    def _prune(self, text, offset, rel):
        """钉死角色时的"具体执行"：把这一轮不会被走到的分支／臂就地填成空白（长度不变，行号不漂移）。

        与符号走树（两支都走、给每个位点打上约束标签后求值）是**两条独立的路**：
        这条路不产生标签，因此同一个 Census 各钉一次取值就能得到一个角色真实的对外表面，
        两本账互为见证——标签读出来有而具体执行没有，必有一边错。
        """
        if self.pin is None:
            return text
        _a, ap = self.match_arms(text, offset, rel)
        _b, cp = self.if_branches(text, offset, rel)
        if not ap and not cp:
            return text
        out = list(text)
        applied = 0
        for lo, hi in ap + cp:
            for i in range(lo, hi):
                out[i] = " "
            applied += 1
            self.prune_intervals.add((rel, offset + lo, offset + hi))
        # 计数记"真正填掉了几段"而不是"发现了几段"：写在循环之前的 `+= len(ap)+len(cp)`
        # 会让撤掉循环的变异体照样把区间数印出来（计数器替没剪掉的分支记功）。
        self.prune_spans += applied
        return "".join(out)

    @staticmethod
    def arm_label(outer, inner):
        """外层继承的角色（臂里调的 builder 在它自己体内挂）与内层逐臂判据要**叠加**：
        只取一个就等于把 `HostRole::Kb` 的臂里再按 tier 分支的那一维读丢，或反过来把跨函数继承读丢。"""
        parts = [p for p in (outer or "").split("+") if p]
        parts += [p for p in (inner or "").split("+") if p and p not in parts]
        return "+".join(parts) if parts else None

    def walk_text(self, text, base, origin, depth, rel=None, offset=0, outer_env=None, role=None):
        """按语句切分后解析：let 语句只入 env，链只从自由表达式语句（含函数尾项）里取。

        被 let 绑定却从未被 merge/nest 的 Router 不算对外表面——所以顶层扫描必须跳过
        let 语句，否则一条废弃的 let 会凭空多出一条在册路由（假阳），而它的真实 merge
        又只该算一次。嵌套在实参里的链由递归处理：线性扫描跳过已消费调用的整个括号区间。
        env 必须沿递归累积：lib.rs 里 `let app = ...merge(actuator)` 的 actuator 绑在
        同函数更早的 let 上，子表达式单独重切时查不到就会把整棵装配树丢光（实测 0 条）。
        """
        if depth > MAX_DEPTH:
            self.unresolved.setdefault("递归深度超限", []).append(origin)
            return
        if self.pin is not None:
            text = self._prune(text, offset, rel)
        env, free = self.split_stats(text, offset)
        arms, _arm_pruned = self.match_arms(text, offset, rel)
        conds, _cond_pruned = self.if_branches(text, offset, rel)
        if outer_env:
            merged = dict(outer_env)
            merged.update(env)
            env = merged
        for seg_start, seg in free:
            stripped = seg.strip()
            if not stripped:
                continue
            if NON_ROUTER_RE.match(stripped):
                self.skipped.append(stripped.split("\n")[0][:64])
                continue
            if stripped.startswith("return "):
                stripped = stripped[len("return "):].strip()
            at = rel + ":" + str(self.line_of(rel, seg_start)) if rel else origin
            if re.fullmatch(r"[A-Za-z_][\w]*", stripped) and stripped in env:
                self.walk_text(env[stripped][1], base, origin, depth + 1, rel, env[stripped][0], env, role)
                continue
            recv = re.match(r"[A-Za-z_][\w]*", stripped)
            recv_in_env = bool(recv) and recv.group(0) in env and \
                stripped[recv.end():recv.end() + 2] != "::"
            if CHAIN_STEP_RE.search(stripped):
                # 接收者自身是 let 绑定时必须一并展开：`let r = match role {…}; r.merge(c)`
                # 只走 .merge 这一步会把 match 各臂的整棵树读成不存在（臂是挂载位点，不是装饰）。
                # 步骤的角色仍按**步骤位置**逐臂判，接收者的臂区间不覆盖步骤 ⇒ 合并进来的子树
                # 在每个臂下都可达，诚实的读数是无标签（∅），而不是把某一个臂当成唯一维。
                if recv_in_env:
                    self.walk_text(env[recv.group(0)][1], base, origin, depth + 1, rel,
                                   env[recv.group(0)][0], env, role)
                for kind, pos, args in self._steps_in(stripped):
                    # 逐**步骤位置**判臂，不逐语句：一支 match 是一整条语句，
                    # 按语句判会把各臂的挂载都记成同一个角色（或全部记成无角色）。
                    # 分支同理：`if role == All { … } else { … }` 的两支是一整条语句，
                    # 臂与分支叠在**同一条合取**上（两层都要成立才算这一位点在该角色下可达）。
                    step_role = self.arm_label(self.arm_label(role, self.arms_at(arms, seg_start + pos - offset)),
                                               self.conds_at(conds, seg_start + pos - offset))
                    self.apply_step(kind, args, base, at, env, depth, rel, seg_start + pos, step_role)
                continue
            # 头标识符后紧跟 :: 的是模块路径（actuator::build_actuator_router()），
            # 不是同名的 let 变量；把它当变量回查 env 会让 let 自己套自己无限递归。
            if recv_in_env:
                self.walk_text(env[recv.group(0)][1], base, origin, depth + 1, rel, env[recv.group(0)][0], env, role)
                continue
            self.resolve_expr(stripped, base, at, depth, env, rel, seg_start, role)

    def apply_step(self, kind, args, base, at, env, depth, rel, pos, role=None):
        # 角色与分支约束都只进 origin 串，不进 mounted 的键 ⇒ 加这两维不会改变在册集合本身。
        # 两族标签分开写：`role:` 给臂（枚举分派），`cond:` 给布尔分支——`--role-matrix` 那本账
        # 只读前者，加第二维不会把它的读数顶掉（同一事实的两把尺子必须能各自复算）。
        toks = [t for t in (role or "").split("+") if t]
        rt = [t for t in toks if not is_cond_token(t)]
        ct = [t for t in toks if is_cond_token(t)]
        tag = (" " + ROLE_TAG + "+".join(rt)) if rt else ""
        tag += (" " + BRANCH_TAG + "+".join(ct)) if ct else ""
        here = (rel + ":" + str(self.line_of(rel, pos)) if rel else at) + tag
        parts = split_args(args)
        if kind == "route":
            if len(parts) < 2 or not parts[0].startswith('"'):
                self.unresolved.setdefault("route 首参非字面量", []).append(here)
                return
            self.record(base + parts[0].strip('"'), methods_of(parts[1]), here)
        elif kind == "nest":
            if len(parts) < 2 or not parts[0].startswith('"'):
                self.unresolved.setdefault("nest 首参非字面量", []).append(here)
                return
            self.resolve_bound(parts[1], env, base + parts[0].strip('"'), here, depth, rel, role)
        elif kind == "fnref":
            # args 位置存的是被点名的函数名（裸 builder 调用，不带前缀变化）
            self.resolve_fn_ref(args, base, here, depth, role)
        else:
            if parts:
                self.resolve_bound(parts[0], env, base, here, depth, rel, role)

    def split_stats(self, text, offset):
        """切顶层语句；返回 (env: name -> (绝对起点, 表达式文本), free: [(绝对起点, 文本)])。"""
        segs, d, start = [], 0, 0
        for i, ch in enumerate(text):
            if ch in "([{":
                d += 1
            elif ch in ")]}":
                d = d - 1 if d > 0 else 0
            elif ch == ";" and d == 0:
                segs.append((start, i))
                start = i + 1
        segs.append((start, len(text)))
        env, free = {}, []
        for s, e in segs:
            seg = text[s:e]
            lead = len(seg) - len(seg.lstrip())
            body = seg[lead:]
            m = re.match(r"let\s+(?:mut\s+)?([A-Za-z_]\w*)", body)
            if m:
                eq = self.find_assign(body, m.end())
                if eq is not None:
                    env[m.group(1)] = (offset + s + lead + eq + 1, body[eq + 1:])
                    continue
            free.append((offset + s + lead, body))
        return env, free

    @staticmethod
    def find_assign(text, from_idx):
        """找 let 的赋值等号：排除 == != <= >= => 以及复合赋值。"""
        for i in range(from_idx, len(text)):
            if text[i] != "=":
                continue
            prev = text[i - 1] if i else ""
            nxt = text[i + 1] if i + 1 < len(text) else ""
            if prev in "=!<>+-*/%&|^" or nxt in "=>":
                continue
            return i
        return None

    def resolve_bound(self, expr, env, base, origin, depth, rel=None, role=None):
        ident = re.fullmatch(r"\s*([A-Za-z_][\w]*)\s*", expr)
        if ident and ident.group(1) in env:
            st, txt = env[ident.group(1)]
            self.walk_text(txt, base, origin, depth + 1, rel, st, env, role)
            return
        if ident:
            self.unresolved.setdefault("let 绑定名查不到且非函数引用: " + ident.group(1), []).append(origin)
            return
        self.resolve_expr(expr, base, origin, depth, env, rel, 0, role)

    def _steps_in(self, text):
        """线性扫一条表达式：.route/.nest/.merge 各吞掉自己的括号区间，
        然后在**未消费**的区间里找裸 builder 调用（match 臂、upgrade(...) 外壳里那种形态）。

        以前只取链步骤，于是 `match role { Kg => upgrade(build_kg_ai_router()), … }` 里
        每条臂的树整支消失；nest 的第二参在已消费区间内，不会被按外层前缀再挂一次（假阳）。"""
        out, i, consumed = [], 0, []
        while True:
            m = CHAIN_STEP_RE.search(text, i)
            if not m:
                break
            args, nxt = call_at(text, m.end() - 1)
            out.append((m.group(1), m.start(), args))
            consumed.append((m.end() - 1, nxt))
            i = nxt
        for pos, n in callee_calls(text):
            if n not in self.fns:
                continue
            if any(lo <= pos < hi for lo, hi in consumed):
                continue          # 已被 nest/merge 实参消费，按外层前缀再挂一次就是假阳
            out.append(("fnref", pos, n))
        out.sort(key=lambda t: t[1])
        return out

    def walk_body(self, rel, lo, hi, base, origin, depth, role=None):
        self.walk_text(self.marked[rel][lo:hi], base, origin, depth + 1, rel, lo, None, role)

    def record(self, path, methods, origin):
        key = (path, tuple(methods))
        self.mounted.setdefault(key, []).append(origin)

    def role_matrix(self):
        """把 origin 串里的角色标签回收成 {归一路径: set(枚举标签)}。只读不判。

        标签存在 origin 里而不是 mounted 的键里，所以加这一维**不会**改变在册集合——
        这正是"归属账是注解而不是判据"的含义，也是变异体能只打红矩阵的地方。"""
        per = {}
        for (p, _ms), origins in self.mounted.items():
            labels = per.setdefault(normalize(p), set())
            for o in origins:
                for tok in o.split():
                    if tok.startswith(ROLE_TAG):
                        labels.update(t for t in tok[len(ROLE_TAG):].split("+") if t)
        return per

    def branch_matrix(self):
        """{归一路径: set(分支约束标签)}：与 role_matrix 同构而**各读各的标签**，用来做第二把尺。

        两个视图（这里的路径→标签集，和 origin_conjunctions 的路径→合取列表）必须由同一批
        origin 串解出同一个路径集合，否则说明其中一个读数器把某族标签吃漏了。
        """
        per = {}
        for (p, _ms), origins in self.mounted.items():
            toks = per.setdefault(normalize(p), set())
            for o in origins:
                for tok in o.split():
                    if tok.startswith(BRANCH_TAG):
                        toks.update(t for t in tok[len(BRANCH_TAG):].split("+") if t)
        return per

    def run_as(self, variant):
        """把角色钉成具体取值再走一遍树：这一条路不产生标签，走到的就是该取值下真会被装配的表面。

        根函数写死为 `build_host_router`——它是角色参数真正进来的那一层。融合部署的入口
        `build_gateway_router` 只是 `build_host_router(state, HostRole::All)` 的薄壳，
        从它走会把钉死的取值又换回 All，五档塌成一档（这一条假设由 `--role-surface` 的
        "五档是否各不相同"现量见证，不靠注释自证）。
        """
        self.pin = variant
        hits = self.fns.get("build_host_router")
        if not hits:
            self.unresolved.setdefault("找不到装配根", []).append("build_host_router")
            return self
        rel, lo, hi = hits[0]
        self.stack.append((rel, lo))
        try:
            self.walk_body(rel, lo, hi, "", "root:build_host_router[%s]" % variant, 0)
        finally:
            self.stack.pop()
        return self

    def run_roots(self):
        """从唯一的装配根出发。build_gateway_router 就是 build_host_router(All) 的薄壳，
        两个都当根会把同一棵树走两遍，把 UNRESOLVED 与耗时一起翻倍。"""
        for name in ("build_gateway_router", "build_host_router"):
            hits = self.fns.get(name)
            if not hits:
                continue
            rel, lo, hi = hits[0]
            self.stack.append((rel, lo))
            try:
                self.walk_body(rel, lo, hi, "", "root:" + name, 0)
            finally:
                self.stack.pop()
            return self
        self.unresolved.setdefault("找不到装配根", []).append("build_gateway_router/build_host_router")
        return self


def normalize(path):
    """路径参数写法归一：:id / {id} / {code} / *path / {*path} 都折成同一种占位。"""
    p = re.sub(r"\{?\*?([A-Za-z_][A-Za-z0-9_]*)\}", "{P}", path)
    p = re.sub(r":[A-Za-z_][A-Za-z0-9_]*", "{P}", p)
    p = re.sub(r"\*([A-Za-z_][A-Za-z0-9_]*)", "{P}", p)
    return p.rstrip("/") or "/"


def read_routes_table():
    """从 actuator.rs 的 ROUTES 静态表取 (method, path)。"""
    rel = "platform/gateway/mox-platform-gateway-svc/src/actuator.rs"
    path = os.path.join(REPO, rel)
    if not os.path.isfile(path):
        return None, None, rel
    src = read_text(path)
    declared = re.search(r"pub static ROUTES:\s*\[ApiRoute;\s*(\d+)\]", src)
    n = int(declared.group(1)) if declared else None
    entries = re.findall(r'\br\(\s*"[^"]*"\s*,\s*"([^"]*)"\s*,\s*"([^"]*)"', src)
    return n, [(m, p) for m, p in entries], rel


def ca_prefix(entry):
    """`/api/{**}` -> `/api`。兜底账与在册账要用同一个剥法，两处分头写就会配错对。"""
    return entry[:-len(CATCH_MARK)] if entry.endswith(CATCH_MARK) else entry


def catch_all_covered(table_entries, catchall):
    """把"在册但解析不到挂载"的条目按**保守口径**交给兜底前缀解释：
    只认余下正好一段参数的 <前缀>/{P}。多段（/api/a/{P}）一律留在余账里点名，
    因为兜底到底转给谁、那个下游认不认这条路径，仪器没有证据。

    返回 (被解释的条目, 仍在册未挂载的条目)。"""
    prefixes = {ca_prefix(p).rstrip("/") for p, _ in catchall}
    covered, still = [], []
    for entry in table_entries:
        hit = any(entry == p or entry == p + "/{P}" for p in prefixes)
        (covered if hit else still).append(entry)
    return sorted(covered), sorted(still)


def callee_calls(text):
    """表达式里的函数调用位点，返回 [(起点, 被调名)]；被调名取路径最后一段。

    写成 `crate::system::build_system_router::<S>()` 时前面每段都是命名空间，只有末段是被调函数；
    而 `router.with_state(())` 里的 router 是形参名，不是调用。两处解析（链内裸调用、
    非链表达式）必须共用这一个判据，分头写就会一条严一条松——本轮就因此把 429 打成 271。"""
    out = []
    for m in CALLEE_RE.finditer(text):
        if m.start() > 0 and text[m.start() - 1] == ".":
            continue
        if not CALL_TAIL_RE.match(text, m.end()):
            continue
        segs = [s.strip() for s in m.group(0).split("::") if s.strip()]
        if segs:
            out.append((m.start(), segs[-1]))
    return out


def api_route_has_role_field():
    """ROUTES 表的行类型里有没有角色字段——决定"在册账"能不能承载这一维。

    不硬编"没有"：字段是并发作者可能加的，加了就必须现量读到并印出来（那才有第二源）。
    """
    rel = "platform/gateway/mox-platform-gateway-svc/src/actuator.rs"
    path = os.path.join(REPO, rel)
    if not os.path.isfile(path):
        return None, rel, None
    src = read_text(path)
    m = re.search(r"struct ApiRoute\b[^{]*\{(.*?)\n\}", src, re.S)
    if not m:
        return None, rel, None
    fields = re.findall(r"^\s*(?:#\[[^\]]*\]\s*)?pub\s+(\w+)\s*:", m.group(1), re.M)
    return [f for f in fields if "role" in f.lower()], rel, fields


def role_aggregates(cen, mounted_paths):
    """把 {归一路径: 标签集} 汇成读数；只读，不改挂载集合。"""
    labels = cen.role_matrix()
    # 无归属位点要按 origin 逐条重算：一条路径可以既在臂里挂、又在臂外挂，
    # 只看矩阵的标签集会把"臂外也可达"读成"只在臂内可达"（假阳的对外面账）。
    unlabeled = set()
    for p, items in mounted_paths.items():
        for _raw, _ms, origins in items:
            for o in origins:
                if ROLE_TAG not in o:
                    unlabeled.add(p)
    labeled = {p for p, v in labels.items() if v}
    per_label = {}
    for p, v in labels.items():
        for lab in v:
            per_label.setdefault(lab, set()).add(p)
    exclusive = {}
    for lab, ps in per_label.items():
        exclusive[lab] = {p for p in ps if labels.get(p) == {lab}}
    role_only = {p for p in labeled if p not in unlabeled}
    return {"labels": labels, "labeled": labeled, "unlabeled": unlabeled,
            "per_label": per_label, "exclusive": exclusive, "role_only": role_only,
            "alphabet": {tok for v in labels.values() for tok in v}}


def mounted_by_path(cen):
    """按归一路径收挂载位点：{归一路径: [(字面路径, 方法, [origin,...]), ...]}。"""
    out = {}
    for (p, ms), origins in cen.mounted.items():
        out.setdefault(normalize(p), []).append((p, ms, origins))
    return out


def site_twins(agg, mounted_paths):
    """有归属的路径按"无归属读数落在哪个位点"分三档：同 site ／ 不同 site ／ 根本没有。

    同 site ＝ 同一个挂载语句被两条走树路径各到过一次，一条带臂一条不带（真语料里
    All 走模块注册表、单角色走 `domain_router` 臂就是这个形状：位点相同，前缀也相同）。
    不同 site ＝ 另有一处装配在挂它。根本没有 ＝ 只有臂里这一处，才是"专属该角色"。
    两档合成一档会把"读不到第二处"和"确实没有第二处"混成同一个数。"""
    same, diff, none_ = set(), set(), set()
    for p in agg["labeled"]:
        lab_sites, unlab_sites = set(), set()
        for _raw, _ms, origins in mounted_paths[p]:
            for o in origins:
                site = o.split(" ")[0]
                (unlab_sites if ROLE_TAG not in o else lab_sites).add(site)
        if not unlab_sites:
            none_.add(p)
        elif lab_sites & unlab_sites:
            same.add(p)
        else:
            diff.add(p)
    return same, diff, none_


def origin_conjunctions(cen, mounted_paths):
    """{归一路径: [合取,...]}：每条 origin 还原成"走到这个位点所经过的全部条件"。

    臂标签与分支标签**并置在同一条合取**里（两层都成立这一位点才在该取值下可达），
    但保留 origin 逐条的粒度——多条路径挂同一件事是**析取**，折成一条合取会把
    "两处装配之一挂"读成"两处同时挂"（永不可达）。
    """
    per = {}
    for p, items in mounted_paths.items():
        cons = per.setdefault(p, [])
        for _raw, _ms, origins in items:
            for o in origins:
                toks = set()
                for tok in o.split():
                    if tok.startswith(ROLE_TAG) or tok.startswith(BRANCH_TAG):
                        head = tok.split(":", 1)[1]
                        toks.update(t for t in head.split("+") if t)
                cons.append(frozenset(toks))
    return per


def read_buckets(per, variants, use_branch=True):
    """按约束标签求值分档：`use_branch=False` 是第二把尺（只看臂），两把尺的差集＝分支归因吃进判决的量。"""
    buckets = {v: set() for v in variants}
    for v in variants:
        for p, cons in per.items():
            if any(conj_ok(c, v, use_branch) for c in cons):
                buckets[v].add(p)
    return buckets


def surface_as(variant, cen):
    """第二通道：复用同一份已索引语料，把角色钉成具体取值走一遍树（不产生标签）。

    索引一次、走树 N＋1 次——重新 load 会把耗时按档数乘上去，而两通道必须读同一批文件，
    否则"并发作者在这两次读之间改了源码"会伪装成分档分歧。
    """
    c2 = Census()
    c2.marked, c2.fns, c2.scanned = cen.marked, cen.fns, cen.scanned
    c2.run_as(variant)
    return {normalize(p) for (p, _ms) in c2.mounted}, c2


def cmd_role_surface(args):
    """按角色分桶的独立对外表面账：标签求值与具体执行各算一遍，两本账互为见证。只读不判。"""
    cen = Census().load(SCAN_ROOTS)
    cen.run_roots()
    mounted_paths = mounted_by_path(cen)
    per = origin_conjunctions(cen, mounted_paths)
    bm = cen.branch_matrix()
    agg = role_aggregates(cen, mounted_paths)
    variants, vrel, vline = host_role_variants(cen)
    backfilled = False
    if not variants:
        variants = sorted({m for cons in per.values() for c in cons for t in c
                           for m in re.findall(r"HostRole::(\w+)", t)})
        backfilled = True
    buckets = read_buckets(per, variants)
    arm_only = read_buckets(per, variants, use_branch=False)
    subst, subst_cen = {}, {}
    for v in variants:
        s, c2 = surface_as(v, cen)
        subst[v], subst_cen[v] = s, c2
    total = set(mounted_paths)
    common = {p for p, cons in per.items() if any(not c for c in cons)}
    only_all = {p for p in total if p in buckets.get("All", set())
                and not any(p in buckets[v] for v in variants if v != "All")}
    only_single = {}
    for v in variants:
        only_single[v] = {p for p in buckets[v] if not any(p in buckets[w] for w in variants if w != v)}
    never = sorted(p for p in total if not any(p in buckets[v] for v in variants))
    n_decl, table, rel = read_routes_table()
    table_paths = {normalize(p) for _m, p in (table or [])}
    ca_paired = {e for e, _ in cen.catchall
                 if any(t == ca_prefix(e) or t == ca_prefix(e) + "/{P}" for t in table_paths)}
    cond_paths = {p for p, v in bm.items() if v}
    conj_cond_paths = {p for p, cons in per.items() if any(any(is_cond_token(t) for t in c) for c in cons)}
    nonrole = sorted({t for cons in per.values() for c in cons for t in c
                      if not is_cond_token(t) and not ARM_ROLE_RE.match(t)})
    branch_bites = {v: sorted(arm_only[v] - buckets[v]) for v in variants}
    chan_diff = {v: sorted(buckets[v] ^ subst[v]) for v in variants}
    subst_faces = {frozenset(subst[v]) for v in subst}
    bucket_union = set().union(*[buckets[v] for v in variants]) if variants else set()
    print("扫描 .rs 文件数 = %d（索引 1 次，走树 %d 次：符号 1 ＋ 钉死取值 %d）"
          % (cen.scanned, 1 + len(variants), len(variants)))
    print("角色取值账：%s（读自 %s:%s%s）"
          % (",".join(variants) or "读不到", vrel or "-", vline or "-",
             "；声明读不到，退回约束标签里出现过的变体" if backfilled else ""))
    print("分支区间入账 = %d 处 / 归不出约束的角色分支（盲区，按名点名）= %d 处"
          % (len(cen.cond_spans), len(cen.cond_blind)))
    for rel_, pos_, excerpt in sorted(cen.cond_blind)[:args.show]:
        print("    盲区 %s:%d  %s" % (rel_, cen.line_of(rel_, pos_) if rel_ in cen.marked else 0, excerpt))
    print("带分支约束标签的归一路径 = %d 条（合取视图 %d 条，两视图必须同集合）"
          % (len(cond_paths), len(conj_cond_paths)))
    print("公共面（至少一条挂载读数不带任何条件）= %d 条，占总数 %d 条的 %.1f%%"
          % (len(common), len(total), 100.0 * len(common) / len(total) if total else 0.0))
    print("非角色维的臂标签（不参与分档判决，只点名）= %d 个：%s"
          % (len(nonrole), ", ".join(nonrole[:args.show]) or "-"))
    print("")
    print("%-8s %10s %10s %10s %10s %10s" % ("角色", "标签档", "臂-only", "钉死档", "仅此档", "未在册"))
    for v in variants:
        unreg = len([p for p in buckets[v] if p not in table_paths and p not in ca_paired])
        print("%-8s %10d %10d %10d %10d %10d"
              % (v, len(buckets[v]), len(arm_only[v]), len(subst[v]), len(only_single[v]), unreg))
    print("")
    for v in variants:
        d_branch, d_chan = branch_bites[v], chan_diff[v]
        print("%s：分支判据吃掉 %d 条（撤了就会多算进来）%s ｜ 两通道分歧 %d 条%s"
              % (v, len(d_branch), "（例：%s）" % ", ".join(d_branch[:3]) if d_branch else "",
                 len(d_chan), "（例：%s）" % ", ".join(d_chan[:3]) if d_chan else ""))
        if d_chan:
            free = sorted(set(d_chan) & common)
            print("    其中属公共面（无条件读数，两通道都该有 ⇒ 仪器坏）= %d 条：%s"
                  % (len(free), ", ".join(free[:4]) or "-"))
    print("任何角色都到不了的归一路径 = %d 条：%s" % (len(never), ", ".join(never[:args.show]) or "-"))
    print("只在 All 下可达（融合部署独占）= %d 条；与 --role-matrix 的「独占＝%d 条」不是同一个事实："
          "那边按臂标签算，这边按分档算。" % (len(only_all), len(agg["role_only"])))
    n_all_sub = len(subst.get("All", set()))
    print("对照 ROUTES 表（%s，声明 %s 条）：钉死 All 走树得 %d 条、标签 All 档 %d 条、未在册总账 %d 条"
          % (rel, n_decl, n_all_sub, len(buckets.get("All", set())),
             len([p for p in total if p not in table_paths and p not in ca_paired])))
    bad = 0
    l0 = {"/health", "/api/v1/status", "/api/v1/domains", "/metrics"}
    checks = [
        ("语料与角色取值账非空（仪器有电，不是空账）",
         cen.scanned > 0 and len(variants) > 0 and len(cen.cond_spans) > 0,
         "文件 %d 取值 %d 分支 %d" % (cen.scanned, len(variants), len(cen.cond_spans))),
        ("分支归因真的落到挂载位点（不落到挂载上的分支不进判决）",
         len(cond_paths) > 0,
         "带分支标签的路径 %d 条，分支区间 %d 处" % (len(cond_paths), len(cen.cond_spans))),
        ("两个标签视图必须给出同一个路径集合（分支通道）",
         cond_paths == conj_cond_paths,
         "矩阵视图 %d 配合取视图 %d，对称差 %s" % (
             len(cond_paths), len(conj_cond_paths),
             sorted(cond_paths ^ conj_cond_paths)[:4] or "-")),
        ("公共面必须出现在每一个角色档（拿标签剪枝分桶就是把这条剪没）",
         bool(variants) and all(common <= buckets[v] for v in variants) and len(common) > 0,
         "公共面 %d 条，档 %d 个，缺席 %s" % (
             len(common), len(variants),
             [v for v in variants if not common <= buckets[v]] or "-")),
        ("同一批公共面在钉死取值通道里也必须每档都在（两通道同一判据）",
         bool(variants) and all(common <= subst[v] for v in variants),
         "缺席 %s" % ([v for v in variants if not common <= subst[v]] or "-")),
        ("L0 四条公共面必须落在 All 档（融合部署的默认对外面）",
         l0 <= buckets.get("All", set()) and l0 <= subst.get("All", set()),
         "标签 All %d 条，钉死 All %d 条，缺 %s" % (
             len(buckets.get("All", set())), n_all_sub, sorted(l0 - buckets.get("All", set())) or "-")),
        ("分支判据只能收窄不能放宽：带分支的档 ⊆ 只看臂的档",
         all(buckets[v] <= arm_only[v] for v in variants),
         "放宽 %s" % ({v: len(buckets[v] - arm_only[v]) for v in variants
                       if buckets[v] - arm_only[v]} or "-")),
        ("每一档都必须算得出来（少一档＝整张账读小）",
         len(buckets) == len(variants) and all(len(buckets[v]) >= 0 for v in variants),
         "算出 %d 档 / 枚举 %d 个" % (len(buckets), len(variants))),
        ("分支判据必须真的吃进判决（至少一档的臂-only 与带分支两把尺不同）",
         any(branch_bites[v] for v in variants),
         "各档被分支判据吃掉 %s" % {v: len(branch_bites[v]) for v in variants}),
        ("每一档都必须非空（钉死取值走空＝剪枝剪错了整棵树）",
         bool(variants) and all(subst[v] for v in variants) and all(buckets[v] for v in variants),
         "钉死 %s ／ 标签 %s" % ({v: len(subst[v]) for v in variants},
                                {v: len(buckets[v]) for v in variants})),
        ("分档闭合：各档并集 ＋ 任何角色都到不了 ＝ 归一总数",
         bucket_union | set(never) == total and not (bucket_union & set(never)),
         "并集 %d ＋ 永不可达 %d 配 总数 %d（交 %d）" % (
             len(bucket_union), len(never), len(total), len(bucket_union & set(never)))),
        ("钉死取值的各档不许塌成同一档（剪枝真落地）",
         bool(subst) and len(subst_faces) == len(subst),
         "%d 档里去重得 %d 个不同面；剪枝 %d 次（同一区间会在函数体／表达式／整文件三个坐标基上各剪一次），按坐标去重后 %d 段" % (
             len(subst), len(subst_faces),
             sum(c2.prune_spans for c2 in subst_cen.values()),
             sum(len(c2.prune_intervals) for c2 in subst_cen.values()))),
    ]
    for name, ok, detail in checks:
        print("%-4s %s  %s" % ("PASS" if ok else "FAIL", name, detail))
        bad += 0 if ok else 1
    if args.json:
        payload = {
            "generated_at": datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S%z"),
            "instrument": "scripts/gate/check-api-surface.py --role-surface",
            "scan_roots": list(SCAN_ROOTS),
            "scanned_files": cen.scanned,
            "mounted_normalized": len(total),
            "role_variants": variants,
            "role_variants_source": {"file": vrel, "line": vline, "backfilled_from_labels": bool(backfilled)},
            "branch_spans": len(cen.cond_spans),
            "prune_occurrences_by_variant": {v: subst_cen[v].prune_spans for v in variants},
            "prune_distinct_intervals_by_variant": {v: len(subst_cen[v].prune_intervals)
                                                    for v in variants},
            "branch_blind": sorted("%s:%d %s" % (r, p, e) for r, p, e in cen.cond_blind),
            "branch_tagged_paths": len(cond_paths),
            "common_surface_paths": sorted(common),
            "non_role_arm_tokens": nonrole,
            "buckets_tag": {v: sorted(buckets[v]) for v in variants},
            "buckets_tag_arm_only": {v: len(arm_only[v]) for v in variants},
            "buckets_substituted": {v: sorted(subst[v]) for v in variants},
            "only_single_bucket": {v: sorted(only_single[v]) for v in variants},
            "only_all_bucket": sorted(only_all),
            "bucket_delta_never_reachable": sorted(never),
            "unregistered_by_bucket": {v: len([p for p in buckets[v]
                                               if p not in table_paths and p not in ca_paired])
                                       for v in variants},
            "routes_declared_len": n_decl,
            "mounted_not_in_table_same_caliber": len([p for p in total
                                                      if p not in table_paths and p not in ca_paired]),
            "cross_channel_symmetric_diff": {v: sorted(buckets[v] ^ subst[v]) for v in variants},
            "branch_judgment_removed": {v: sorted(arm_only[v] - buckets[v]) for v in variants},
            "instrument_invariants_failed": [n for n, ok, _d in checks if not ok],
        }
        with open(args.json, "w", encoding="utf-8") as fh:
            json.dump(payload, fh, ensure_ascii=False, indent=2)
        print("JSON 已写出 " + args.json)
    print("ROLE-SURFACE %s（失败 %d 项；本模式故意不判决——分档是账，门禁口径仍挂在裁决点）"
          % ("PASS" if bad == 0 else "FAIL", bad))
    return 0 if bad == 0 else 1


def cmd_role_matrix(args):
    """按 HostRole 臂归属的对外面账：**只出账，不判决**（等裁决点，别拿注解当门禁）。"""
    cen = Census().load(SCAN_ROOTS)
    cen.run_roots()
    mounted_paths = mounted_by_path(cen)
    agg = role_aggregates(cen, mounted_paths)
    twins = site_twins(agg, mounted_paths)
    arm_tokens = {tok for _rel, _pos, lab in cen.role_arms for tok in lab.split("+")}
    n_decl, table, rel = read_routes_table()
    table_paths = {normalize(p) for _m, p in (table or [])}
    print("扫描 .rs 文件数 = %d（扫描集根：%s）" % (cen.scanned, ", ".join(SCAN_ROOTS)))
    print("挂载点（字面 / 归一）= %d / %d" % (len(cen.mounted), len(mounted_paths)))
    print("逐臂枚举 = %d 个具名臂 ＋ %d 个无枚举路径的臂（`_ =>` 之类，只继承外层）"
          % (len(cen.role_arms), len(cen.role_wild)))
    arm_files = {}
    for rel_, _pos, _lab in cen.role_arms:
        arm_files[rel_] = arm_files.get(rel_, 0) + 1
    print("臂的分布 = %s" % (", ".join("%s:%d" % kv for kv in sorted(arm_files.items())) or "-"))
    print("带角色归属的归一路径 = %d 条 / 无归属 = %d 条 / 总 = %d 条"
          % (len(agg["labeled"]), len(agg["unlabeled"]), len(mounted_paths)))
    print("其中「只在某角色下可达」（没有任何无归属位点）= %d 条" % len(agg["role_only"]))
    print("有归属路径的无归属读数分档：同 site %d ／ 不同 site %d ／ 根本没有 %d"
          % (len(twins[0]), len(twins[1]), len(twins[2])))
    print("    同 site ＝ 该位点被两条走树路径各到过一次（一条带臂、一条不带），"
          "读成「角色是路径的属性」；不同 site 才要去看是不是第二处装配；"
          "根本没有才是该端点只在这个角色的臂里挂过。")
    print("每个臂标签可达的路径数（可达数／独占数）：")
    for lab in sorted(agg["per_label"]):
        print("    %-24s 可达 %3d ／ 独占 %3d" % (lab, len(agg["per_label"][lab]), len(agg["exclusive"][lab])))
    multi = sorted(p for p in agg["labeled"] if len(agg["labels"][p]) > 1)
    print("跨两维以上（嵌套 match 或多臂同挂）= %d 条：%s" % (len(multi), ", ".join(multi[:args.show]) or "-"))
    for p in sorted(agg["role_only"])[:args.show]:
        print("    角色专属  %-46s %s" % (p, "|".join(sorted(agg["labels"][p]))))
    if len(agg["role_only"]) > args.show:
        print("    ...还有 %d 条未打印" % (len(agg["role_only"]) - args.show))
    orphans = sorted(lab for lab in agg["per_label"] if not agg["exclusive"][lab])
    print("咨询（不判失败）：有可达但零独占的臂标签 = %d 个：%s" % (len(orphans), ", ".join(orphans) or "-"))
    role_fields, arel, all_fields = api_route_has_role_field()
    print("在册账能不能承载这一维：`ApiRoute`（%s）字段 = %s；含 role 的字段 = %s"
          % (arel, ",".join(all_fields or []) or "读不到", role_fields if role_fields is not None else "文件缺失"))
    # 与普查账同口径：兜底前缀与表内 <前缀>/{P} 是同一件事的两种写法，不减配对项就会比
    # --census 报出的差额多两条，同一本账两把尺子（本轮实测 142 配 140）。
    ca_paired = {e for e, _ in cen.catchall
                 if any(t == ca_prefix(e) or t == ca_prefix(e) + "/{P}" for t in table_paths)}
    only_mount = sorted(set(mounted_paths) - table_paths - ca_paired)
    print("与 ROUTES 表交叉：挂载未在册 %d 条（与 --census 同口径，已减兜底配对 %d 条），其中带角色归属 %d 条"
          % (len(only_mount), len(ca_paired), len([p for p in only_mount if p in agg["labeled"]])))
    bad = 0
    checks = [
        ("语料非空且臂读得出（仪器有电，不是空账）",
         cen.scanned > 0 and len(cen.role_arms) > 0,
         "文件 %d 臂 %d" % (cen.scanned, len(cen.role_arms))),
        ("分解闭合：有归属 ∪ 无归属 ＝ 归一路径总数",
         agg["labeled"] | agg["unlabeled"] == set(mounted_paths),
         "并集 %d 配 总数 %d（交集 %d 条＝臂内外都挂位，正是 role_only 要收窄的那批）"
         % (len(agg["labeled"] | agg["unlabeled"]), len(mounted_paths),
            len(agg["labeled"] & agg["unlabeled"]))),
        ("矩阵里的标签必须来自臂枚举（不许凭空造维度）",
         agg["alphabet"] <= arm_tokens,
         "标签 %d 个，臂枚举 %d 个，越界 %s" % (
             len(agg["alphabet"]), len(arm_tokens), sorted(agg["alphabet"] - arm_tokens) or "-")),
        ("角色专属集合必须是有归属集合的子集",
         agg["role_only"] <= agg["labeled"],
         "专属 %d ／ 有归属 %d" % (len(agg["role_only"]), len(agg["labeled"]))),
        ("同 site／不同 site／根本没有 三档必须不重不漏地覆盖有归属路径",
         twins[0] | twins[1] | twins[2] == agg["labeled"]
         and not (twins[0] & twins[1]) and not (twins[0] & twins[2])
         and not (twins[1] & twins[2]),
         "%d ＋ %d ＋ %d 配 有归属 %d（并集 %d）" % (
             len(twins[0]), len(twins[1]), len(twins[2]), len(agg["labeled"]),
             len(twins[0] | twins[1] | twins[2]))),
        ("「根本没有无归属读数」必须与 role_only 独立算得同一个集合",
         twins[2] == agg["role_only"],
         "分档 %d 配 role_only %d，对称差 %s" % (
             len(twins[2]), len(agg["role_only"]),
             sorted(twins[2] ^ agg["role_only"])[:4] or "-")),
        ("差额口径与普查账一致：未减配对项的差额减去配对项等于本行的差额",
         len(set(mounted_paths) - table_paths) - len({e for e in ca_paired
                                                      if e in set(mounted_paths) - table_paths}) == len(only_mount),
         "%d − 落在差额里的配对 %d 配 %d" % (
             len(set(mounted_paths) - table_paths),
             len({e for e in ca_paired if e in set(mounted_paths) - table_paths}), len(only_mount))),
    ]
    for name, ok, detail in checks:
        print("%-4s %s  %s" % ("PASS" if ok else "FAIL", name, detail))
        bad += 0 if ok else 1
    if args.json:
        payload = {
            "generated_at": datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S%z"),
            "instrument": "scripts/gate/check-api-surface.py --role-matrix",
            "scan_roots": list(SCAN_ROOTS),
            "scanned_files": cen.scanned,
            "mounted_normalized": len(mounted_paths),
            "role_arms_named": len(cen.role_arms),
            "role_arms_wild": len(cen.role_wild),
            "routes_declared_len": n_decl,
            "mounted_not_in_table_same_caliber": len(only_mount),
            "mounted_not_in_table_labeled": len([p for p in only_mount if p in agg["labeled"]]),
            "api_route_fields": all_fields,
            "api_route_role_fields": role_fields,
            "labeled_paths": len(agg["labeled"]),
            "unlabeled_paths": len(agg["unlabeled"]),
            "role_only_paths": sorted(agg["role_only"]),
            "same_site_twin_count": len(twins[0]),
            "different_site_twin_paths": sorted(twins[1]),
            "no_twin_paths": sorted(twins[2]),
            "multi_dim_paths": sorted(p for p in agg["labeled"] if len(agg["labels"][p]) > 1),
            "reachable_by_label": {k: len(v) for k, v in sorted(agg["per_label"].items())},
            "exclusive_by_label": {k: len(v) for k, v in sorted(agg["exclusive"].items())},
            "matrix": {k: sorted(v) for k, v in sorted(agg["labels"].items()) if v},
            "instrument_invariants_failed": [n for n, ok, _d in checks if not ok],
        }
        with open(args.json, "w", encoding="utf-8") as fh:
            json.dump(payload, fh, ensure_ascii=False, indent=2)
        print("JSON 已写出 " + args.json)
    print("ROLE-MATRIX %s（失败 %d 项；本模式故意不判决——归属是注解，不是门禁）"
          % ("PASS" if bad == 0 else "FAIL", bad))
    return 0 if bad == 0 else 1


def cmd_census(args):
    cen = Census().load(SCAN_ROOTS)
    cen.run_roots()
    n_decl, table, rel = read_routes_table()
    mounted_paths = {}
    for (p, ms), origins in cen.mounted.items():
        mounted_paths.setdefault(normalize(p), []).append((p, ms, origins))
    table_paths = {}
    for m, p in (table or []):
        table_paths.setdefault(normalize(p), []).append((m, p))
    print("扫描 .rs 文件数 = %d（扫描集根：%s）" % (cen.scanned, ", ".join(SCAN_ROOTS)))
    print("装配节点在册 = %d（按名字 build_* %d ＋ 按签名 -> Router 新增 %d）"
          % (len(cen.fns), cen.n_builder, cen.n_router))
    print("跳过的非路由语句 = %d" % len(cen.skipped))
    print("走进去发现是状态升级壳（不挂路径）= %d 次" % cen.passthrough)
    print("走进去过的装配节点 = %d 次 / %d 个不同函数" % (len(cen.entered), len({n for n, _ in cen.entered})))
    sig_only = sorted({n for n, _ in cen.entered if not n.startswith("build_")})
    print("其中只靠签名（非 build_* 命名）才在册、且真被走到的 = %d 个：%s"
          % (len(sig_only), ", ".join(sig_only[:args.show]) or "-"))
    print("解析出的挂载点（按字面路径去重）= %d" % len(cen.mounted))
    print("解析出的挂载路径（归一化参数后）= %d" % len(mounted_paths))
    print("兜底挂载（.fallback，无字面路径）= %d 处：%s"
          % (len(cen.catchall), ", ".join(sorted({p for p, _ in cen.catchall})) or "-"))
    print("ROUTES 表声明长度 = %s，实际解析出条目 = %d（%s）" % (n_decl, len(table or []), rel))
    only_table_raw = sorted(set(table_paths) - set(mounted_paths))
    by_ca, still = catch_all_covered(only_table_raw, cen.catchall)
    only_table = still
    # 兜底前缀与在册的 <前缀>/{P} 是同一件事的两种写法：配上对的就不许再算第二次差额，
    # 配不上（表里根本没这条前缀）的兜底必须留在差额里点名。
    paired_ca = {e for e, _ in cen.catchall
                 if any(t == ca_prefix(e) or t == ca_prefix(e) + "/{P}" for t in table_paths)}
    only_mount = sorted(set(mounted_paths) - set(table_paths) - paired_ca)
    print("兜底与在册配对 = %d/%d 处（未配对 %d 处仍计入差额）"
          % (len(paired_ca), len({e for e, _ in cen.catchall}), len({e for e, _ in cen.catchall}) - len(paired_ca)))
    print("在册未解析到挂载 = %d 条（另有 %d 条由兜底前缀按保守口径解释：%s）"
          % (len(only_table), len(by_ca), ", ".join(by_ca) or "-"))
    print("挂载未在册 = %d 条" % len(only_mount))
    for name, lst in sorted(cen.unresolved.items(), key=lambda kv: -len(kv[1])):
        print("UNRESOLVED[%s] = %d" % (name, len(lst)))
        for it in lst[: args.show]:
            print("    " + it)
        if len(lst) > args.show:
            print("    ...还有 %d 条未打印" % (len(lst) - args.show))
    if args.show_lists:
        for it in only_table[: args.show_lists]:
            print("  在册未挂载  " + it)
        for it in only_mount[: args.show_lists]:
            print("  挂载未在册  " + it)
    if args.json:
        payload = {
            # 语料在同一天里被并发作者改过（ROUTES 236→242），而 --json 是覆盖写：
            # 没有 generated_at 的账，重跑一次就再也说不清它是哪一刻的分母。
            "generated_at": datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S%z"),
            "scan_roots": list(SCAN_ROOTS),
            "scanned_files": cen.scanned,
            "assembly_nodes_indexed": len(cen.fns),
            "indexed_by_name": cen.n_builder,
            "indexed_by_signature": cen.n_router,
            "entered_total": len(cen.entered),
            "entered_unique": len({n for n, _ in cen.entered}),
            "entered_signature_only": sorted({n for n, _ in cen.entered if not n.startswith("build_")}),
            "passthrough_shells": cen.passthrough,
            "catch_all": sorted({p for p, _ in cen.catchall}),
            "catch_all_paired_with_table": len(paired_ca),
            "in_table_explained_by_catch_all": by_ca,
            "mounted_literal": len(cen.mounted),
            "mounted_normalized": len(mounted_paths),
            "routes_declared_len": n_decl,
            "routes_parsed": len(table or []),
            "in_table_not_mounted": only_table,
            "mounted_not_in_table": only_mount,
            "unresolved": {k: len(v) for k, v in cen.unresolved.items()},
        }
        with open(args.json, "w", encoding="utf-8") as fh:
            json.dump(payload, fh, ensure_ascii=False, indent=2)
        print("JSON 已写出 " + args.json)
    return 0


# ====================== 台账闭合（--ledger）：核心公式的单一算源 ======================
# 规矩：文档里写下加号就必须算加法，而且每个数都要挂在"印出它的那份工件"的同一个键上。
# §1.1–§1.6 的公式此前散在散文里，逐条人工复算（第 33 条就是复算时抓到两处不闭合）。本模式把
# 这批公式收成一张表，三层各有牙：
#   L1 结构闭合：在**一份活的图像**上现量复算每条等式的左值与加数（含"某数不是加数"的反对照）。
#   L2 引用对账：把每个数对回 reports/data/ 下被引用的工件键。键找不到＝引用不成立（红）；
#                值不同＝语料漂移（按名点名并附工件 generated_at，判 INFO 不判红——并发作者天天动源）。
#   L3 文档镜像：方案文档里的托管块必须等于现渲染（逐字节）；托管块之外的散文里写的加法必须自己成立。

LEDGER_DOC_PATH = "docs/architecture/API-SURFACE-AUTHORITY-PLAN-v0.1.md"
LEDGER_BEGIN = "<!-- LEDGER:BEGIN 本块由 scripts/gate/check-api-surface.py --ledger --write 现量生成，勿手改（改公式改脚本） -->"
LEDGER_END = "<!-- LEDGER:END -->"
LEDGER_ART_PREFIX = {"census": "api-surface-census-",
                     "surface": "api-role-surface-",
                     "matrix": "api-role-matrix-"}
# (工件, 工件里的键, live 命名空间里的名字, 比较法)  —— "set" 比的是排序后的成员，其余比值（list/dict 取长度）
LEDGER_CROSS = [
    ("census", "scanned_files", "scanned_files", "v"),
    ("census", "assembly_nodes_indexed", "assembly_nodes_indexed", "v"),
    ("census", "indexed_by_name", "indexed_by_name", "v"),
    ("census", "indexed_by_signature", "indexed_by_signature", "v"),
    ("census", "entered_total", "entered_total", "v"),
    ("census", "entered_unique", "entered_unique", "v"),
    ("census", "entered_signature_only", "entered_sig_only", "v"),
    ("census", "passthrough_shells", "passthrough", "v"),
    ("census", "routes_declared_len", "routes_declared_len", "v"),
    ("census", "routes_parsed", "routes_parsed", "v"),
    ("census", "mounted_literal", "mounted_literal", "v"),
    ("census", "mounted_normalized", "mounted_normalized", "v"),
    ("census", "mounted_not_in_table", "m_only_mount", "v"),
    ("census", "in_table_not_mounted", "t_not_mounted", "v"),
    ("census", "in_table_explained_by_catch_all", "t_explained", "v"),
    ("census", "catch_all_paired_with_table", "ca_paired_n", "v"),
    ("census", "unresolved", "unresolved_counts", "v"),
    ("surface", "scanned_files", "scanned_files", "v"),
    ("surface", "mounted_normalized", "mounted_normalized", "v"),
    ("surface", "routes_declared_len", "routes_declared_len", "v"),
    ("surface", "branch_spans", "branch_spans", "v"),
    ("surface", "branch_tagged_paths", "branch_tagged", "v"),
    ("surface", "common_surface_paths", "common_surface", "v"),
    ("surface", "only_all_bucket", "only_all", "v"),
    ("surface", "bucket_delta_never_reachable", "never", "v"),
    ("surface", "non_role_arm_tokens", "nonrole", "v"),
    ("surface", "branch_blind", "branch_blind", "v"),
    ("surface", "buckets_tag", "bucket_counts", "v"),
    ("surface", "buckets_substituted", "subst_counts", "v"),
    ("surface", "buckets_tag_arm_only", "arm_only_counts", "v"),
    ("surface", "only_single_bucket", "only_single_counts", "v"),
    ("surface", "unregistered_by_bucket", "unreg_counts", "v"),
    ("surface", "branch_judgment_removed", "removed_counts", "v"),
    ("surface", "cross_channel_symmetric_diff", "xdiff_counts", "v"),
    ("surface", "prune_occurrences_by_variant", "prune_occ_counts", "v"),
    ("surface", "prune_distinct_intervals_by_variant", "prune_dist_counts", "v"),
    ("surface", "role_variants", "role_variants", "set"),
    ("surface", "instrument_invariants_failed", "surface_failed", "v"),
    ("matrix", "scanned_files", "scanned_files", "v"),
    ("matrix", "mounted_normalized", "mounted_normalized", "v"),
    ("matrix", "routes_declared_len", "routes_declared_len", "v"),
    ("matrix", "role_arms_named", "role_arms_named", "v"),
    ("matrix", "role_arms_wild", "role_arms_wild", "v"),
    ("matrix", "labeled_paths", "labeled", "v"),
    ("matrix", "unlabeled_paths", "unlabeled", "v"),
    ("matrix", "same_site_twin_count", "twin_same", "v"),
    ("matrix", "different_site_twin_paths", "twin_diff", "v"),
    ("matrix", "no_twin_paths", "twin_none", "v"),
    ("matrix", "role_only_paths", "role_only", "v"),
    ("matrix", "multi_dim_paths", "multi_dim", "v"),
    ("matrix", "exclusive_by_label", "excl_counts", "v"),
    ("matrix", "mounted_not_in_table_same_caliber", "m_only_mount", "v"),
    ("matrix", "mounted_not_in_table_labeled", "unreg_labeled", "v"),
    ("matrix", "api_route_role_fields", "api_route_role_fields", "set"),
    ("matrix", "instrument_invariants_failed", "matrix_failed", "v"),
]


def newest_artifact(prefix):
    """按名字前缀挑最新的读数工件：工件名自带日期，硬编今天那份明天就红。"""
    d = os.path.join(REPO, "reports", "data")
    if not os.path.isdir(d):
        return None
    cand = [n for n in os.listdir(d) if n.startswith(prefix) and n.endswith(".json")]
    if not cand:
        return None
    return max(cand, key=lambda n: os.path.getmtime(os.path.join(d, n)))


def art_scalar(v):
    if isinstance(v, dict):
        return {k: art_scalar(x) for k, x in v.items()}
    if isinstance(v, (list, tuple, set)):
        return len(v)
    return v


def art_get(art, dotted):
    cur = art
    for seg in dotted.split("."):
        if not isinstance(cur, dict) or seg not in cur:
            return None, False
        cur = cur[seg]
    return cur, True


def ledger_spec(variants):
    """公式清单。每条只写"名字"，值一律由 ledger_eval 从同一个命名空间取 ⇒
    同一个数在两行里必须同步，改错一处会让两行同时红（这就是"单一算源"的牙）。"""
    core = [
        ("ASM-CLOSE", "在册装配节点 ＝ 按名字在册 ＋ 只靠签名新增", "sum",
         "assembly_nodes_indexed", ["indexed_by_name", "signature_only"],
         "census:assembly_nodes_indexed／indexed_by_name／indexed_by_signature"),
        ("ASM-ANTI", "反对照：按签名匹配的总数**不是**加数（与按名字那批有重叠）", "ne-sum",
         "assembly_nodes_indexed", ["indexed_by_name", "indexed_by_signature"],
         "重叠 %s 条 ⇒ 只按名字会读小"),
        ("DECL-PARSE", "ROUTES 表声明长度 ＝ 真解析出的条目数", "eq",
         "routes_declared_len", ["routes_parsed"], "census:routes_declared_len／routes_parsed"),
        ("M-PART", "归一挂载面 ＝ 在册∩挂载 ＋ 兜底前缀覆盖 ＋ 挂载未在册", "sum",
         "mounted_normalized", ["m_in_table", "m_ca_only", "m_only_mount"],
         "census:mounted_normalized／mounted_not_in_table"),
        ("T-PART", "ROUTES 表的不同路径数 ＝ 在册∩挂载 ＋ 在册未挂载 ＋ 由兜底解释", "sum",
         "routes_paths_distinct", ["m_in_table", "t_not_mounted", "t_explained"],
         "注意分母是**去重后的路径**，不是条目数（一条路径多_method_算两次）"),
        ("TAG-COMMON", "归一挂载面 ＝ 带分支约束标签 ＋ 公共面", "sum",
         "mounted_normalized", ["branch_tagged", "common_surface"],
         "surface:branch_tagged_paths／common_surface_paths"),
        ("TAG-DJ", "上面那两档必须互斥（同一位点既能无条件挂又被分支罩住＝读重）", "is0",
         "tag_common_inter", [], "两集交集现量"),
        ("ONLY-ALL", "归一挂载面 ＝ 只在 All 档 ＋ 跨多档 ＋ 任何角色都到不了", "sum",
         "mounted_normalized", ["only_all", "multi_bucket", "never"],
         "surface:only_all_bucket／bucket_delta_never_reachable"),
        ("NEVER-0", "任何角色都到不了的路径现在必须为 0（不为 0 就是分档漏了一档）", "is0",
         "never", [], "surface:bucket_delta_never_reachable"),
        ("LBL-PART", "有归属路径 ＝ 同 site 双读数 ＋ 不同 site ＋ 根本没有", "sum",
         "labeled", ["twin_same", "twin_diff", "twin_none"],
         "matrix:same_site_twin_count／different_site_twin_paths／no_twin_paths"),
        ("TWIN-NONE", "「根本没有无归属读数」必须与 role_only 独立算得同一个数", "eq",
         "twin_none", ["role_only"], "matrix:no_twin_paths／role_only_paths"),
        ("LBL-SUM", "有归属路径 ＝ 各臂独占数之和（前提：跨两维以上的路径为 0）", "eq",
         "labeled", ["excl_sum"], "matrix:labeled_paths／exclusive_by_label"),
        ("LBL-DJ", "跨两维以上必须为 0，否则 LBL-SUM 会把同一条路径数两遍", "is0",
         "multi_dim", [], "matrix:multi_dim_paths"),
        ("UNREG-CROSS", "未在册差额在两台仪器上必须同一个数（普查账 ＝ 分档 All 档）", "eq",
         "unreg_all", ["m_only_mount"], "surface:unregistered_by_bucket.All ＝ census:mounted_not_in_table"),
        ("XCHAN-DJ", "标签通道与钉死取值通道的分歧必须为 0（两本账互为见证）", "is0",
         "xdiff_sum", [], "surface:cross_channel_symmetric_diff"),
    ]
    per = []
    for v in variants:
        per += [
            ("BUCKET-%s" % v, "%s 档：只看臂的档 ＝ 带分支的档 ＋ 分支判据吃掉的条数" % v, "sum",
             "arm_%s" % v, ["bucket_%s" % v, "removed_%s" % v], "surface:buckets_tag_arm_only／branch_judgment_removed"),
            ("SUBST-%s" % v, "%s 档：标签求值得到的面 ＝ 钉死取值走树得到的面" % v, "eq",
             "bucket_%s" % v, ["subst_%s" % v], "surface:buckets_tag／buckets_substituted"),
            ("PRUNE-%s" % v, "%s 档：剪枝出现次数 ≥ 按坐标去重后的段数" % v, "ge",
             "prune_occ_%s" % v, ["prune_dist_%s" % v], "surface:prune_occurrences_by_variant／prune_distinct_intervals_by_variant"),
        ]
    return core + per


def ledger_eval(spec, num):
    rows = []
    for rid, title, kind, lhs, rhs, cite in spec:
        if lhs not in num or any(k not in num for k in rhs):
            missing = [k for k in [lhs] + list(rhs) if k not in num]
            rows.append(dict(rid=rid, title=title, kind=kind, cite=cite, ok=False,
                             eqs="命名空间缺键：%s" % ",".join(missing), lhs=None, rhs=[]))
            continue
        l = num[lhs]
        vals = [num[k] for k in rhs]
        if kind == "sum":
            ok, eqs = l == sum(vals), "%s ＝ %s" % (l, " ＋ ".join(str(x) for x in vals))
        elif kind == "eq":
            ok, eqs = l == vals[0], "%s ＝ %s" % (l, vals[0])
        elif kind == "ne-sum":
            ok, eqs = l != sum(vals), "%s ≠ %s（=%d）" % (l, " ＋ ".join(str(x) for x in vals), sum(vals))
            # 说明里的重叠数也必须从这一行的值现推（候选和 − 并集），手写就成了第二个源
            if "%s" in cite:
                cite = cite % (sum(vals) - l)
        elif kind == "ge":
            ok, eqs = l >= vals[0], "%s ≥ %s" % (l, vals[0])
        else:  # is0
            ok, eqs = l == 0, "%s ＝ 0" % l
        rows.append(dict(rid=rid, title=title, kind=kind, cite=cite, ok=ok, eqs=eqs,
                         lhs=l, rhs=vals))
    # 渲染不许把没替换的占位符印进文档：这类缺陷逐字节稳定的镜像判据看不见（两次渲染同样错），
    # 只有把它当成一条判决才不会被"读表格的人"独享。
    for r in rows:
        if "%s" in r["cite"] or "%s" in r["title"]:
            r["ok"] = False
            r["eqs"] = "渲染留未替换占位符 ⇒ %s" % r["cite"]
    return rows


def ledger_block(rows, meta):
    # 块内不许有墙钟：把"本次运行时刻"写进必须逐字节相同的见证，等于要求两次运行同一秒结束
    # （§49 那类不可能满足的判据）。时刻只出现在 stdout 与末行的工件署名上。
    out = [LEDGER_BEGIN,
           "",
           "本表由 `%s --ledger --write` 在一份活的图像上现量复算，**不在表里的加法不算账**。" % "scripts/gate/check-api-surface.py",
           "扫描 .rs %d 个｜公式 %d 条（红 %d）｜引用对账 %d 条：一致 %d、漂移 %d、引用不成立 %d。" % (
               meta["scanned_files"], len(rows),
               sum(0 if r["ok"] else 1 for r in rows),
               meta["cross_total"], meta["cross_ok"], meta["cross_drift"], meta["cross_unresolved"]),
           "读数的三个来源工件：%s" % "、".join(
               "%s（%s 生成）" % (p, g) for p, g in zip(meta["arts"], meta["arts_gen"])),
           "",
           "| 公式 | 现量等式 | 判定 | 说明／引用 |",
           "|---|---|---|---|"]
    for r in rows:
        out.append("| `%s` %s | %s | %s | %s |" % (
            r["rid"], r["title"], r["eqs"], "闭合" if r["ok"] else "**不闭合**", r["cite"]))
    out += ["", "漂移只说明语料动了（并发作者天天改源），不说明账错：红判据是"
                "「等式不闭合」「引用键找不到」「托管块与现渲染不一致」三种。", LEDGER_END]
    return out


def ledger_block_span(lines):
    try:
        i = next(k for k, ln in enumerate(lines) if ln.startswith(LEDGER_BEGIN[:24]))
    except StopIteration:
        return None
    for j in range(i, len(lines)):
        if lines[j].strip() == LEDGER_END:
            return (i, j)
    return None


def audit_prose_arith(lines, span):
    """托管块之外，散文里写出的加法必须自己成立。识别与复算全在 `formula_ledger.py`：
    本模式**不再自带一份尺子**（一台仪器里两把尺＝同一本账必然漂移）。
    「…」／“…” 里的是**引文**（登记"某轮曾印错成什么"），不是本文件的账 ⇒ 不复算，但要按名点名，
    免得哪天把引文当现量读；落不进主张的形状按 reason 出账（盲区不是缺陷，读成"没有缺陷"才是）。
    `span` 是 `ledger_block_span` 给的 **0 基**闭区间，而 `scan_text` 按 **1 基**行号跳块 ⇒
    这里必须 +1：不换算的后果是托管表的最后一行被当散文复算（同一个数被两把尺各数一次）。"""
    sp = FL.scan_text("\n".join(lines), blank_lines=(span[0] + 1, span[1] + 1) if span else ())
    viol = [(v["line"], v["span"], v["vals"]) for v in sp["violations"]]
    quoted = [(q["line"], q["span"]) for q in sp["quoted"]]
    return viol, quoted, len(sp["claims"]), sp["blind"]


def render_ledger_md(rows, meta):
    return "\n".join(ledger_block(rows, meta)) + "\n"


def ledger_dups(rows):
    """公式 id 撞车＝后写的顶掉先写的，而总账条数照样自洽 ⇒ id 唯一性本身是一条判据。"""
    ids = [r["rid"] for r in rows]
    return sorted({i for i in ids if ids.count(i) > 1})


def block_diff(cur_lines, new_lines):
    """托管块逐行比对：集合差 ≠ 逐位差，所以按**下标**列差异，长度不同也要各自算到。"""
    return [k for k in range(max(len(cur_lines), len(new_lines)))
            if (cur_lines[k] if k < len(cur_lines) else None)
            != (new_lines[k] if k < len(new_lines) else None)]


def cmd_ledger(args):
    cen = Census().load(SCAN_ROOTS)
    cen.run_roots()
    mounted = mounted_by_path(cen)
    per = origin_conjunctions(cen, mounted)
    bm = cen.branch_matrix()
    agg = role_aggregates(cen, mounted)
    twins = site_twins(agg, mounted)
    variants, vrel, vline = host_role_variants(cen)
    buckets = read_buckets(per, variants)
    arm_only = read_buckets(per, variants, use_branch=False)
    subst, scen = {}, {}
    for v in variants:
        s, c2 = surface_as(v, cen)
        subst[v], scen[v] = s, c2
    total = set(mounted)
    common = {p for p, cons in per.items() if any(not c for c in cons)}
    tagged = {p for p, vv in bm.items() if vv}
    n_decl, table, rel = read_routes_table()
    table_paths = {normalize(p) for _m, p in (table or [])}
    ca_paired = {e for e, _ in cen.catchall
                 if any(t == ca_prefix(e) or t == ca_prefix(e) + "/{P}" for t in table_paths)}
    in_table = total & table_paths
    only_mount = sorted(total - table_paths - ca_paired)
    ca_only = sorted((total - table_paths) & ca_paired)
    by_ca, still = catch_all_covered(sorted(table_paths - total), cen.catchall)
    only_all = {p for p in buckets.get("All", set())
                if not any(p in buckets[w] for w in variants if w != "All")}
    never = sorted(p for p in total if not any(p in buckets[v] for v in variants))
    multi_bucket = total - only_all - set(never)
    only_single = {v: {p for p in buckets[v] if not any(p in buckets[w] for w in variants if w != v)}
                   for v in variants}
    unreg = {v: [p for p in buckets[v] if p not in table_paths and p not in ca_paired] for v in variants}
    removed = {v: arm_only[v] - buckets[v] for v in variants}
    xdiff = {v: buckets[v] ^ subst[v] for v in variants}
    excl = agg["exclusive"]
    multi = sorted(p for p in agg["labeled"] if len(agg["labels"][p]) > 1)

    num = {
        "scanned_files": cen.scanned,
        "branch_spans": len(cen.cond_spans),
        "assembly_nodes_indexed": len(cen.fns),
        "indexed_by_name": cen.n_builder,
        "indexed_by_signature": cen.n_router,
        "signature_only": len(cen.fns) - cen.n_builder,
        "entered_total": len(cen.entered),
        "entered_unique": len({n for n, _ in cen.entered}),
        "entered_sig_only": len([n for n, _ in cen.entered if not n.startswith("build_")]),
        "passthrough": cen.passthrough,
        "unresolved_n": sum(len(v) for v in cen.unresolved.values()),
        "ca_paired_n": len(ca_paired),
        "routes_declared_len": int(n_decl or 0),
        "routes_parsed": len(table or []),
        "routes_paths_distinct": len(table_paths),
        "mounted_literal": len(cen.mounted),
        "mounted_normalized": len(total),
        "m_in_table": len(in_table),
        "m_ca_only": len(ca_only),
        "m_only_mount": len(only_mount),
        "unreg_labeled": len([p for p in only_mount if p in agg["labeled"]]),
        "t_not_mounted": len(still),
        "t_explained": len(by_ca),
        "branch_tagged": len(tagged),
        "common_surface": len(common),
        "tag_common_inter": len(tagged & common),
        "only_all": len(only_all),
        "multi_bucket": len(multi_bucket),
        "never": len(never),
        "labeled": len(agg["labeled"]),
        "unlabeled": len(agg["unlabeled"]),
        "twin_same": len(twins[0]),
        "twin_diff": len(twins[1]),
        "twin_none": len(twins[2]),
        "role_only": len(agg["role_only"]),
        "multi_dim": len(multi),
        "excl_sum": sum(len(excl[k]) for k in excl),
        "unreg_all": len(unreg.get("All", [])),
        "xdiff_sum": sum(len(xdiff[v]) for v in xdiff),
        "role_arms_named": len(cen.role_arms),
        "role_arms_wild": len(cen.role_wild),
        "nonrole": len({t for cons in per.values() for c in cons for t in c
                        if not is_cond_token(t) and not ARM_ROLE_RE.match(t)}),
        "branch_blind": len(cen.cond_blind),
    }
    for v in variants:
        num["bucket_%s" % v] = len(buckets[v])
        num["arm_%s" % v] = len(arm_only[v])
        num["subst_%s" % v] = len(subst[v])
        num["removed_%s" % v] = len(removed[v])
        num["prune_occ_%s" % v] = scen[v].prune_spans
        num["prune_dist_%s" % v] = len(scen[v].prune_intervals)
    allv = dict(num)
    allv.update({
        "bucket_counts": {v: len(buckets[v]) for v in variants},
        "subst_counts": {v: len(subst[v]) for v in variants},
        "arm_only_counts": {v: len(arm_only[v]) for v in variants},
        "only_single_counts": {v: len(only_single[v]) for v in variants},
        "unreg_counts": {v: len(unreg[v]) for v in variants},
        "removed_counts": {v: len(removed[v]) for v in variants},
        "xdiff_counts": {v: len(xdiff[v]) for v in variants},
        "prune_occ_counts": {v: num["prune_occ_%s" % v] for v in variants},
        "prune_dist_counts": {v: num["prune_dist_%s" % v] for v in variants},
        "excl_counts": {k: len(excl[k]) for k in excl},
        "role_variants": list(variants),
        "unresolved_counts": {k: len(v) for k, v in cen.unresolved.items()},
    })
    role_fields, arel, _all_fields = api_route_has_role_field()
    allv["api_route_role_fields"] = list(role_fields or [])
    # 这两项由 --role-surface / --role-matrix 各自写进工件，本模式不复算 ⇒ 只能钉"必须为 0"。
    num["surface_failed"] = allv["surface_failed"] = 0
    num["matrix_failed"] = allv["matrix_failed"] = 0

    spec = ledger_spec(variants)
    rows = ledger_eval(spec, num)
    dup = ledger_dups(rows)
    meta = {"generated_at": datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S%z"),
            "scanned_files": cen.scanned}

    print("现量时刻 %s｜扫描 .rs %d 个｜角色取值 %s（读自 %s:%s）"
          % (meta["generated_at"], cen.scanned, ",".join(variants) or "读不到", vrel or "-", vline or "-"))
    arts, arts_gen, cross_res = [], [], []
    # bad 的初值必须在**第一条红之前**：工件那两条红（1846／1857）历史上印 FAIL 却不计数，
    # 因为 bad = 0 落在它们之后——三条工件全丢也照样 LEDGER PASS（红 0 项）。
    bad = 0
    loaded = {}
    for tag in ("census", "surface", "matrix"):
        name = newest_artifact(LEDGER_ART_PREFIX[tag])
        if name is None:
            print("FAIL 引用找不到工件 %s*.json（%s 这一路的每个数都没了来源）"
                  % (LEDGER_ART_PREFIX[tag], tag))
            bad += 1
            arts.append(LEDGER_ART_PREFIX[tag] + "*（缺）")
            arts_gen.append("-")
            continue
        path = "reports/data/" + name
        arts.append(path)
        try:
            with open(os.path.join(REPO, path.replace("/", os.sep)), encoding="utf-8") as fh:
                art = json.load(fh)
        except ValueError as exc:
            print("FAIL 工件 %s 解析不了：%s" % (path, exc))
            bad += 1
            arts_gen.append("-")
            continue
        loaded[tag] = art
        arts_gen.append(str(art.get("generated_at", "?")))
    for tag, key, lname, how in LEDGER_CROSS:
        art = loaded.get(tag)
        if art is None:
            cross_res.append(("unresolved", tag, key, lname, "工件未加载"))
            continue
        raw, found = art_get(art, key)
        if not found:
            cross_res.append(("unresolved", tag, key, lname, "键不存在"))
            continue
        if lname not in allv:
            cross_res.append(("unresolved", tag, key, lname, "live 命名空间缺键"))
            continue
        # "set" 比成员，两侧都不能被折成长度（int）——那是"比长度"冒充"比内容"的经典假账。
        av = sorted(raw) if how == "set" else art_scalar(raw)
        lv = allv[lname]
        same = (av == sorted(lv)) if how == "set" else (av == lv)
        if not same:
            cross_res.append(("drift", tag, key, lname, "工件 %s ／ live %s" % (av, lv)))
    drift = [r for r in cross_res if r[0] == "drift"]
    unres = [r for r in cross_res if r[0] == "unresolved"]
    meta.update({"cross_total": len(LEDGER_CROSS), "cross_ok": len(LEDGER_CROSS) - len(drift) - len(unres),
                 "cross_drift": len(drift), "cross_unresolved": len(unres),
                 "arts": arts, "arts_gen": arts_gen})
    print("")
    print("%-12s %-52s %s" % ("公式", "现量等式", "判定"))
    for r in rows:
        print("%-12s %-52s %s" % (r["rid"], r["eqs"], "闭合" if r["ok"] else "**不闭合** " + r["title"]))
        bad += 0 if r["ok"] else 1
    print("%-12s %-52s %s" % ("ID-UNIQ", "公式 id 不许撞车（撞了后写的会顶掉先写的而总账自洽）",
                              "闭合" if not dup else "**不闭合** %s" % dup))
    bad += 0 if not dup else 1
    for kind, tag, key, lname, detail in sorted(drift, key=lambda r: (r[1], r[2])):
        print("INFO 漂移 %s:%s（live 名 %s）%s —— 工件是那一刻的图像，语料在并发作者手里天天动"
              % (tag, key, lname, detail))
    for kind, tag, key, lname, detail in sorted(unres, key=lambda r: (r[1], r[2])):
        print("FAIL 引用不成立 %s:%s（live 名 %s）%s" % (tag, key, lname, detail))
        bad += 1
    print("引用对账 %d 条：一致 %d、漂移 %d、引用不成立 %d" % (
        len(LEDGER_CROSS), meta["cross_ok"], len(drift), len(unres)))

    doc_abs = os.path.join(REPO, LEDGER_DOC_PATH.replace("/", os.sep))
    rendered = render_ledger_md(rows, meta)
    if not os.path.isfile(doc_abs):
        print("FAIL 文档镜像：方案文档不在 %s" % LEDGER_DOC_PATH)
        bad += 1
        lines = []
    else:
        with open(doc_abs, "rb") as fh:
            raw = fh.read()
        text = raw.decode("utf-8")
        lines = text.split("\n")
        span = ledger_block_span(lines)
        viol, quoted, chains, blind = audit_prose_arith(lines, span)
        for ln_, s_, vals in viol:
            print("FAIL 散文里的加法不闭合 %s:%d  「%s」→ 各边 %s" % (LEDGER_DOC_PATH, ln_, s_, vals))
        bad += len(viol)
        bc = FL.blind_counts(blind)
        print("散文算术审计：复算 %d 条主张（红 %d），盲区 %d 条按 reason 出账（%s），引文 %d 条按名点名（引文不是账）"
              % (chains, len(viol), len(blind),
                 "／".join("%s=%d" % kv for kv in sorted(bc.items())) or "空", len(quoted)))
        for ln_, q_ in quoted:
            print("     引文 %s:%d %s" % (LEDGER_DOC_PATH, ln_, q_))
        if args.write:
            if span is None:
                print("FAIL --write 需要文档里已有成对标记（先手工放两个标记行，别让脚本决定插在哪）")
                bad += 1
            else:
                block = rendered.rstrip("\n").split("\n")
                new = lines[:span[0]] + block + lines[span[1] + 1:]
                if new == lines:
                    print("PASS 文档镜像：托管块与现渲染逐字节相同（%d 行）⇒ 未写入（不动文件）"
                          % (span[1] - span[0] + 1))
                else:
                    guard = audit_len_guard(lines, new, span[0], span[1], len(block))
                    if guard:
                        bad += 1
                        print("FAIL --write 被拒：块外逐行核验没过 ⇒ 一个字也没落盘")
                    else:
                        new_bytes = "\n".join(new).encode("utf-8")
                        with open(doc_abs, "wb") as fh:
                            fh.write(new_bytes)
                        with open(doc_abs, "rb") as fh:
                            back = fh.read()
                        if back != new_bytes:
                            print("FAIL --write 回读不一致（盘上字节与内存图像不同：%d 配 %d 字节）"
                                  % (len(back), len(new_bytes)))
                            bad += 1
                        else:
                            print("PASS --write：托管块 %d 行 → %d 行，块外 %d 行逐字节未动，"
                                  "整文件字节回读一致（%d → %d）"
                                  % (span[1] - span[0] + 1, len(block),
                                     len(lines) - (span[1] - span[0] + 1), len(raw), len(back)))
        else:
            if span is None:
                print("FAIL 文档镜像：托管块标记缺失（跑 --ledger --write 之前先在文档里放好两个标记行）")
                bad += 1
            else:
                cur = "\n".join(lines[span[0]:span[1] + 1]) + "\n"
                if cur == rendered:
                    print("PASS 文档镜像：托管块与现渲染逐字节相同（%d 行）" % (span[1] - span[0] + 1))
                else:
                    d1, d2 = rendered.split("\n"), cur.split("\n")
                    diffs = block_diff(d2, d1)
                    print("FAIL 文档镜像：托管块与现渲染不一致（行数 %d 配 %d，首个差异在第 %s 行，共 %d 行不同）"
                          % (len(d2), len(d1), (diffs[:1] or ["-"])[0], len(diffs)))
                    for k in diffs[:args.show]:
                        print("     盘上 %s" % (d2[k] if k < len(d2) else "<无>"))
                        print("     现渲 %s" % (d1[k] if k < len(d1) else "<无>"))
                    bad += 1
    print("LEDGER %s（红 %d 项；公式 %d 条，每条的值都来自同一个命名空间）"
          % ("PASS" if bad == 0 else "FAIL", bad, len(rows)))
    return 0 if bad == 0 else 1


def audit_len_guard(old_lines, new_lines, at, end, n_block):
    """--write 落盘前必须证明块外一行都没动：改前像与改后像在同一张表上逐行比，
    只比长度会漏掉"总行数没变但内容换了"的写错单位型事故。"""
    pre_ok = new_lines[:at] == old_lines[:at]
    post_ok = new_lines[at + n_block:] == old_lines[end + 1:]
    if not pre_ok:
        k = next(x for x in range(at) if new_lines[x] != old_lines[x])
        print("FAIL 块外前段被改动（第 %d 行）" % (k + 1))
    if not post_ok:
        print("FAIL 块外后段被改动（前 %d 行配后 %d 行）" % (len(new_lines) - n_block, len(old_lines) - end - 1))
    return 0 if (pre_ok and post_ok) else 1


FIXTURE = '''
pub fn build_a_router<S>() -> Router<S> {
    let health: Router<S> = Router::new().route("/health", get(h1));
    Router::new()
        .route("/x", get(x1).post(x2))
        .nest("/admin", build_b_router::<S>())
        .merge(health)
}
pub fn build_b_router<S>() -> Router<S> {
    Router::new().route("/y", get(y1))
}
pub fn build_c_router() -> Router<()> {
    Router::new().nest("/api", build_d_router()).route("/z", get(z1))
}
pub fn build_d_router() -> Router<()> {
    Router::new().route("/w", get(w1))
}
pub fn build_host_router(state: GatewayState) -> Router {
    let protected = Router::new()
        .merge(build_c_router())
        .merge(upgrade(build_e_router()));
    Router::new().merge(protected).route("/root", get(r1))
        .route("/ns", axum::routing::get(n1))
}
pub fn build_e_router() -> Router<()> {
    Router::new().nest("/e", Router::new().route("/deep", get(d1)))
}
'''

# 期望由 FIXTURE 的形状现推，不硬编：每条 (归一化全路径, 方法集)。
EXPECT = {
    ("/x", frozenset({"GET", "POST"})),
    ("/admin/y", frozenset({"GET"})),
    ("/health", frozenset({"GET"})),
    ("/api/w", frozenset({"GET"})),
    # axum 语义：Router::new().nest("/api", x).route("/z", h) 里 /z 挂在根上，不吃 /api 前缀
    # （链式调用是对已 nest 的 Router 再加一条），把它写成 /api/z 是错的期望。
    ("/z", frozenset({"GET"})),
    ("/e/deep", frozenset({"GET"})),
    ("/ns", frozenset({"GET"})),
    ("/root", frozenset({"GET"})),
}


# 本轮修的分支单独开火：被 let 绑定却从未 merge/nest 的 Router 不该计入对外表面。
DEAD_FIXTURE = '''
pub fn build_dead_router() -> Router<()> {
    let orphan = Router::new().route("/orphan", get(o1));
    Router::new().route("/live", get(l1))
}
'''


# 真语料 lib.rs 的装配形状：let 绑定的路由器被另一个 let 的表达式 merge，
# 再由尾项返回。env 不沿递归累积时这一整棵树解析为 0 条（本轮实测撞出）。
CHAIN_LET_FIXTURE = '''
pub fn build_h_router() -> Router<()> {
    let leaf = Router::new().route("/leaf", get(a1));
    let mid = upgrade(Router::new().merge(leaf).nest("/m", build_i_router()));
    mid
}
pub fn build_i_router() -> Router<()> {
    Router::new().route("/i", get(b1))
}
'''


# 本轮撞出的无限递归形状：let 名与模块名同名，且 RHS 以 `模块::函数()` 开头。
MODULE_PATH_FIXTURE = '''
pub fn build_j_router() -> Router<()> {
    let actuator = mod_ns::build_leaf_router().route_layer(from_fn(mw));
    Router::new().merge(actuator)
}
pub fn build_leaf_router() -> Router<()> {
    Router::new().route("/leaf2", get(q1))
}
'''


# 条件装配：if 走 build_*，else 走一个名字不规范但返回 Router 的函数，其体内是 match。
# 三条边各有一枚代码变异体（只看第一支 / 只按名字在册 / 不认调用位点）。
BRANCH_FIXTURE = '''
pub fn build_k_router(role: HostRole) -> Router<()> {
    let protected = if role == HostRole::All {
        build_k_all_router()
    } else {
        k_domain_router(role)
    };
    Router::new().merge(protected).route("/top", get(t1))
}
pub fn build_k_all_router() -> Router<()> {
    Router::new().route("/all-only", get(a1))
}
fn k_domain_router(role: HostRole) -> Router<()> {
    match role {
        HostRole::Kg => crate::ns::build_k_kg_router(),
        HostRole::Kb => upgrade(Router::new().nest("/api", build_k_kb_router())),
        _ => Router::new(),
    }
}
pub fn build_k_kg_router() -> Router<()> { Router::new().route("/kg", get(g1)) }
pub fn build_k_kb_router() -> Router<()> { Router::new().route("/kbdoc", get(k1)) }
'''

EXPECT_BRANCH = {"/top", "/all-only", "/api/kbdoc", "/kg"}

# 兜底挂载 + 状态升级壳：fallback 要入兜底账（不是 UNRESOLVED），
# upgrade 那种壳函数走进去既不多挂一条也不许记成盲区。
SHELL_FIXTURE = '''
pub fn build_l_router() -> Router<()> {
    Router::new()
        .nest("/api/projects", upgrade(Router::new().fallback(proxy_handler)))
        .nest("/api", Router::new().fallback(proxy_handler))
        .route("/plain", get(p1))
}
pub fn upgrade<S>(router: Router<()>) -> Router<S> {
    router.with_state(())
}
'''

EXPECT_SHELL = {"/api/projects/{**}", "/api/{**}", "/plain"}

# 本轮真实撞过的假阳：别的 crate 里有 `pub fn router(...) -> Router`，而 upgrade 的形参也叫 router。
# 没有调用位点判据时，走 upgrade 的体会把那棵外来的树挂进网关表面（实测把 371 顶到 429）。
COLLIDE_FIXTURE = '''
pub fn build_p_router() -> Router<()> {
    Router::new().merge(upgrade(build_q_router()))
}
pub fn upgrade<S>(router: Router<()>) -> Router<S> { router.with_state(()) }
pub fn build_q_router() -> Router<()> { Router::new().route("/q", get(h9)) }
pub fn router(state: OtherState) -> Router<()> { Router::new().route("/foreign-not-mounted", get(x9)) }
'''

# 角色维夹具：一臂内直接链、一臂 nest 进别的 crate、一臂调 builder（挂载发生在**别的函数体内**）、
# 一臂里再套一把 match（第二维），外加一枚 `_ =>` 臂（读得出边界但归不了属）与一枚 match 外的 merge（无角色）。
ROLE_FIXTURE = '''
pub fn build_host_router(role: HostRole) -> Router<()> {
    let router = match role {
        HostRole::Kg => Router::new().route("/kg-only", get(r1)),
        HostRole::Kb => upgrade(Router::new().nest("/api", build_kb_router())),
        HostRole::Iam => build_iam_router(),
        HostRole::Cloud => match tier {
            CloudTier::A => Router::new().route("/cloud-a", get(r2)),
            _ => Router::new().route("/cloud-x", get(r3)),
        },
        _ => Router::new(),
    };
    # 同一棵 kb 子树还被一条**不带臂、但同前缀**的路径走了一次 ⇒ 同一个挂载位点出现两种读数。
    # 这条自由表达式就是真语料里 All 走模块注册表、单角色走 domain_router 的那个形状。
    Router::new().nest("/api", build_kb_router());
    router.merge(build_common_router())
}
pub fn build_kb_router() -> Router<()> { Router::new().route("/kbdoc", get(r4)) }
pub fn build_iam_router() -> Router<()> {
    Router::new().route("/iam/deep", get(r5)).route("/iam/deep2", post(r6))
}
pub fn build_common_router() -> Router<()> { Router::new().route("/common", get(r7)) }
pub fn upgrade<S>(router: Router<()>) -> Router<S> { router.with_state(()) }
'''

EXPECT_ROLE_PATHS = {"/kg-only", "/api/kbdoc", "/iam/deep", "/iam/deep2",
                     "/cloud-a", "/cloud-x", "/common"}
EXPECT_ROLE_MATRIX = {
    "/kg-only": {"HostRole::Kg"},
    "/api/kbdoc": {"HostRole::Kb"},
    "/iam/deep": {"HostRole::Iam"},
    "/iam/deep2": {"HostRole::Iam"},
    "/cloud-a": {"HostRole::Cloud", "CloudTier::A"},
    "/cloud-x": {"HostRole::Cloud"},
    "/common": set(),
}


# 分支归因夹具：真语料里 `if role == All { 注册表 } else { domain_router }` 只有 lib.rs:289 一处，
# 单靠活语料不能当见证（一轮改动就能把它换掉），所以分桶语义必须在夹具上钉死：
# All 档要含注册表支、不含单角色臂独占子树；Kb 档要含 /api/kb*、不含 Iam 臂里的路径。
# 另含一枚复合条件（`&&`）——仪器不解布尔，它必须落进"归不出约束"的按名点名账，
# 而不是被静默当成"没有分支"（那会把只在 All 下挂的端点读成公共面）。
COND_FIXTURE = '''
pub fn build_host_router(role: HostRole) -> Router<()> {
    let protected = if role == HostRole::All {
        build_c_all_router()
    } else {
        c_domain_router(role)
    };
    Router::new().merge(protected).merge(build_blend_router(role)).route("/top", get(t0))
}
pub fn build_c_all_router() -> Router<()> { Router::new().route("/all-only", get(a0)) }
fn c_domain_router(role: HostRole) -> Router<()> {
    match role {
        HostRole::Kb => upgrade(Router::new().nest("/api", build_c_kb_router())),
        HostRole::Iam => build_c_iam_router(),
        HostRole::Kg | HostRole::Cloud => build_c_multi_router(),
        _ => Router::new(),
    }
}
pub fn build_c_kb_router() -> Router<()> { Router::new().route("/kbdoc", get(k0)) }
pub fn build_c_iam_router() -> Router<()> { Router::new().route("/iam-only", get(i0)) }
pub fn build_c_multi_router() -> Router<()> { Router::new().route("/multi", get(m0)) }
pub fn build_blend_router(role: HostRole) -> Router<()> {
    if role == HostRole::All && role != HostRole::Kb {
        Router::new().route("/never-mount", get(n0))
    }
    Router::new().route("/common", get(c0))
}
pub fn upgrade<S>(router: Router<()>) -> Router<S> { router.with_state(()) }
'''

COND_VARIANTS = ["All", "Kg", "Cloud", "Kb", "Iam"]
EXPECT_COND_BUCKETS = {
    "All": {"/top", "/common", "/never-mount", "/all-only"},
    "Kg": {"/top", "/common", "/never-mount", "/multi"},
    "Cloud": {"/top", "/common", "/never-mount", "/multi"},
    "Kb": {"/top", "/common", "/never-mount", "/api/kbdoc"},
    "Iam": {"/top", "/common", "/never-mount", "/iam-only"},
}
EXPECT_COND_TAGS = {
    "/top": set(),
    "/common": set(),
    "/never-mount": set(),
    "/all-only": {"=HostRole::All"},
    "/api/kbdoc": {"!HostRole::All", "HostRole::Kb"},
    "/iam-only": {"!HostRole::All", "HostRole::Iam"},
    "/multi": {"!HostRole::All", "HostRole::Kg|HostRole::Cloud"},
}

# else-if 链：外层否定必须罩住整条链，否则链尾那一支会凭空出现在每个角色桶里。
CHAIN_COND_FIXTURE = '''
pub fn build_host_router(role: HostRole) -> Router<()> {
    if role == HostRole::All {
        Router::new().route("/all2", get(a1))
    } else if role == HostRole::Kb {
        Router::new().route("/kb2", get(k1))
    } else {
        Router::new().route("/other2", get(o1))
    }
}
'''
EXPECT_CHAIN_BUCKETS = {"All": {"/all2"}, "Kb": {"/kb2"},
                        "Kg": {"/other2"}, "Cloud": {"/other2"}, "Iam": {"/other2"}}


def run_fixture(src_text, root_fns, pin=None):
    cen = Census()
    cen.pin = pin
    rel = "fixture.rs"
    cen.marked[rel] = mask(src_text)
    cen.scanned = 1
    cen.index_text(rel, cen.marked[rel])   # 与真实语料同一个索引入口，夹具不许另开通道
    for name in root_fns:
        rel0, lo, hi = cen.fns[name][0]
        cen.walk_body(rel0, lo, hi, "", "root:" + name, 0)
    return {(normalize(p), frozenset(ms)) for (p, ms) in cen.mounted}, cen


CODE = open(os.path.abspath(__file__), encoding="utf-8").read()
# 变异锚点只在判据侧数：锚点字面量本身也写在自检里，按整文件数会永远命中 2 次（本轮实测撞出）。
JUDGE_SRC = CODE.split("\ndef cmd_selftest")[0]

# 判决出口账的基线条数：由 `outlet_universe(JUDGE_SRC)` 现量（收据
# reports/data/recon-11-verdict-outlets-2026-10-03-0948.txt）。新增一条红必须同轮改这个数，
# 否则自检红在"宇宙对不上基线"——红模板清单会漏掉明天新加的那条红，一个整数不会。
OUTLET_BASELINE = 13
# 覆盖登记登记的是**片段**不是全串：全串要人手抄 13 条措辞，改一个标点就悬空（那是第二份红模板）。
# 片段只许取自出口的字面量头——宇宙的口径是「截到第一个占位符为止」，住在 %s 之后的词在宇宙里
# 根本不存在（本轮实测：登记「解析不了」命中 0 条，登记「FAIL 工件」才命中那一条）。收据
# reports/data/recon-18s-outlet-needles-2026-10-03-1139.txt §[4]①。
OUTLET_COVERED = [
    "找不到工件", "FAIL 工件", "方案文档不在", "加法不闭合", "已有成对标记", "标记缺失",
    "现渲染不一致", "逐行核验没过", "回读不一致", "引用不成立", "漂移", "块外前段", "块外后段",
]
# 这台仪器的红是一条以这两个标签打头的文案（另一台把红塞进累加器，没有标签这一维）。
OUTLET_PREFIXES = ("FAIL ", "INFO ")


def outlet_sites(src_text):
    """判决出口站点：一处 print 算一个站点，带着函数名与行号；宇宙从它派生。

    这台仪器的红不是往累加器塞消息，而是打一条带标签的文案再把整数加一 ⇒ 宇宙按 print
    首参的字面量前缀收。整串以占位符开头（截出来空或过短）的文案等于"什么红都算命中"，按整串收。
    口径必须是语法级：按文本数会把这句说明里的字样数成一个不存在的站点（本轮实测撞出）。
    抽取本身不在这里另写一份：由 `formula_ledger` 单源（print 形态＋上面那对标签），
    与文档那侧的累加器形态共用同一条「取最左字面量、在首个占位符处截、截短则按整串收」的规则。"""
    return FL.red_outlet_sites(src_text, "print", OUTLET_PREFIXES)


def outlet_universe(src_text):
    """判决出口宇宙：站点前缀去重后排序。整串以占位符开头（截出来空或过短）的文案按整串收。"""
    return sorted({p for p, _f, _l in outlet_sites(src_text)})


def outlet_den_ok(n_sites):
    """分母谓词只管一件事：不许在空集上把零证据印成满把握（第 35 型）。

    「站点数＝唯一前缀数」是另一件事（第 30 型：两条红共用一个前缀＝撤一条不会被人察觉），
    「站点数＝基线」是第三件（棘轮）。三件分开钉：一个旋钮同时动两条判决，红字就说不清哪格坏了。"""
    return n_sites >= 1



def patched_ns(old, new):
    """代码级变异体：把判据本身换成弱化版再 exec 出一份新模块。

    语料级变异体改的是被扫的样例，改不了判据；"只跟第一支""只按名字在册"这类
    退化只能由代码变异体自己打红，否则夹具绿只证明夹具的形状被照顾到了。"""
    assert JUDGE_SRC.count(old) == 1, "变异锚点在判据侧命中 %d 次：%s" % (JUDGE_SRC.count(old), old[:60])
    ns = {"__file__": os.path.abspath(__file__), "__name__": "mutant"}
    exec(compile(CODE.replace(old, new, 1), "<mutant>", "exec"), ns)
    return ns


def run_in(ns, src_text, root_fns, pin=None):
    cen = ns["Census"]()
    cen.pin = pin
    rel = "fixture.rs"
    cen.marked[rel] = ns["mask"](src_text)
    cen.scanned = 1
    cen.index_text(rel, cen.marked[rel])
    for name in root_fns:
        r0, lo, hi = cen.fns[name][0]
        cen.walk_body(r0, lo, hi, "", "root:" + name, 0)
    return {(ns["normalize"](p), frozenset(ms)) for (p, ms) in cen.mounted}, cen


def cmd_selftest(_args):
    got, _ = run_fixture(FIXTURE, ["build_host_router", "build_a_router"])
    checks = []
    miss = sorted(EXPECT - got)
    extra = sorted(got - EXPECT)
    checks.append(("夹具逐条命中", not miss and not extra,
                   "缺=%s 多=%s" % ([m[0] for m in miss], [e[0] for e in extra])))
    # 变异体 1：撤掉 nest("/api", ...) 的前缀 —— 若解析器只是拼接字面量，这条不会变红
    mut1 = FIXTURE.replace('.nest("/api", build_d_router())', ".merge(build_d_router())")
    g1, _ = run_fixture(mut1, ["build_host_router"])
    checks.append(("变异体1 撤 nest 前缀必须改变结果", ("/api/w", frozenset({"GET"})) not in g1 and ("/w", frozenset({"GET"})) in g1,
                   "现 %s" % sorted(p for p, _ in g1)))
    # 变异体 2：把 let 绑定切断（merge 一个不存在的标识符）—— 必须落 UNRESOLVED 而不是静默少一条
    mut2 = FIXTURE.replace(".merge(health)", ".merge(health_unused_x)")
    g2, c2 = run_fixture(mut2, ["build_a_router"])
    checks.append(("变异体2 断 let 绑定必须点名", ("/health", frozenset({"GET"})) not in g2 and bool(c2.unresolved),
                   "unresolved=%s" % list(c2.unresolved)))
    # 变异体 3：撤掉 upgrade(...) 外壳识别 —— /e/deep 应消失
    mut3 = FIXTURE.replace("upgrade(build_e_router())", "build_e_router()")
    g3, _ = run_fixture(mut3, ["build_host_router"])
    checks.append(("变异体3 外壳换写法仍解析", ("/e/deep", frozenset({"GET"})) in g3, "现 %s" % sorted(p for p, _ in g3)))
    # 正对照：注释与字符串里的 .route( 不得被当成挂载
    mut4 = FIXTURE + '\npub fn build_f_router() -> Router<()> {\n    // .route("/in_comment", get(f1))\n    let s = "a(\\"n";\n    Router::new().route("/f", get(f2))\n}\n'
    g4, _ = run_fixture(mut4, ["build_f_router"])
    checks.append(("正对照 注释内的 route 不算挂载", all(p != "/in_comment" for p, _ in g4) and ("/f", frozenset({"GET"})) in g4,
                   "现 %s" % sorted(p for p, _ in g4)))
    gd, cd = run_fixture(DEAD_FIXTURE, ["build_dead_router"])
    checks.append(("死 let 绑定不计入对外表面",
                   gd == {("/live", frozenset({"GET"}))},
                   "现 %s unresolved=%s" % (sorted(gd), list(cd.unresolved))))
    gh, ch = run_fixture(CHAIN_LET_FIXTURE, ["build_h_router"])
    checks.append(("跨 let 引用沿 env 累积解析",
                   gh == {("/leaf", frozenset({"GET"})), ("/m/i", frozenset({"GET"}))},
                   "现 %s unresolved=%s" % (sorted(gh), list(ch.unresolved))))
    gj, cj = run_fixture(MODULE_PATH_FIXTURE, ["build_j_router"])
    checks.append(("模块路径头名不与 let 变量混淆",
                   gj == {("/leaf2", frozenset({"GET"}))} and not cj.unresolved,
                   "现 %s unresolved=%s" % (sorted(gj), list(cj.unresolved))))
    # 归一化：三种参数写法折成同一占位
    checks.append(("参数写法归一", normalize("/a/:id") == normalize("/a/{id}") == normalize("/a/{*rest}") == "/a/{P}",
                   "got %s %s %s" % (normalize("/a/:id"), normalize("/a/{id}"), normalize("/a/{*rest}"))))
    # —— 本轮宽化的四件事，各配代码变异体（判据退化必须自己打红）——
    gb, cb = run_fixture(BRANCH_FIXTURE, ["build_k_router"])
    pb = sorted(p for p, _ in gb)
    checks.append(("条件装配两支都要走", set(pb) == set(EXPECT_BRANCH) and not cb.unresolved,
                   "现 %s unresolved=%s" % (pb, list(cb.unresolved))))
    ns1 = patched_ns("            if n in self.fns and n not in seen:\n"
                     "                seen.append(n)\n"
                     "                lab[n] = self.arm_label(self.arm_label(role, self.arms_at(arms, pos)),\n"
                     "                                        self.conds_at(conds, pos))",
                     "            if n in self.fns:\n"
                     "                seen.append(n)\n"
                     "                lab[n] = role\n"
                     "                break")
    g1b, _ = run_in(ns1, BRANCH_FIXTURE, ["build_k_router"])
    checks.append(("变异体4 只跟第一支必须漏掉 else 支",
                   sorted(p for p, _ in g1b) == ["/all-only", "/top"],
                   "现 %s" % sorted(p for p, _ in g1b)))
    ns2 = patched_ns("            if not router_ret:\n                continue",
                     "            if not builder:\n                continue")
    g2b, c2b = run_in(ns2, BRANCH_FIXTURE, ["build_k_router"])
    checks.append(("变异体5 只按名字在册必须漏掉非 build_* 节点",
                   "/api/kbdoc" not in {p for p, _ in g2b} and "k_domain_router" not in c2b.fns,
                   "现 %s" % sorted(p for p, _ in g2b)))
    gs, cs = run_fixture(SHELL_FIXTURE, ["build_l_router"])
    ps = sorted(p for p, _ in gs)
    checks.append(("兜底入兜底账而不是盲区",
                   set(ps) == set(EXPECT_SHELL) and len(cs.catchall) == 2
                   and not cs.unresolved and cs.passthrough == 1,
                   "现 %s 兜底 %d 壳 %d unresolved=%s" % (ps, len(cs.catchall), cs.passthrough, list(cs.unresolved))))
    ns3 = patched_ns("        if FALLBACK_RE.search(expr):", "        if False:")
    g3s, c3s = run_in(ns3, SHELL_FIXTURE, ["build_l_router"])
    checks.append(("变异体6 撤兜底识别必须落盲区而不是静默少一条",
                   not c3s.catchall and "/api/{**}" not in {p for p, _ in g3s}
                   and bool(c3s.unresolved.get("无法归类的表达式")),
                   "兜底 %d unresolved=%s" % (len(c3s.catchall), list(c3s.unresolved))))
    cov, rest = catch_all_covered(["/api/{P}", "/api/projects/{P}", "/api/x/{P}"],
                                  [("/api/{**}", "o"), ("/api/projects/{**}", "o")])
    checks.append(("兜底配对只认单段参数，多段留余账点名",
                   cov == ["/api/projects/{P}", "/api/{P}"] and rest == ["/api/x/{P}"],
                   "配对=%s 余账=%s" % (cov, rest)))
    gc, cc = run_fixture(COLLIDE_FIXTURE, ["build_p_router"])
    checks.append(("形参名不与同名的别处函数混为一谈",
                   {p for p, _ in gc} == {"/q"} and not cc.unresolved and cc.passthrough == 1,
                   "现 %s unresolved=%s 壳 %d" % (sorted(p for p, _ in gc), list(cc.unresolved), cc.passthrough)))
    ns4 = patched_ns("        if not CALL_TAIL_RE.match(text, m.end()):\n            continue",
                     "        if False:\n            continue")
    g4c, _ = run_in(ns4, COLLIDE_FIXTURE, ["build_p_router"])
    checks.append(("变异体7 撤调用位点判据必须把别处的树挂进来",
                   "/foreign-not-mounted" in {p for p, _ in g4c},
                   "现 %s" % sorted(p for p, _ in g4c)))
    ns5 = patched_ns("            if not CHAIN_STEP_RE.search(inner) and not any(n in self.fns for _, n in callee_calls(inner)):",
                     "            if False:")
    g5c, c5c = run_in(ns5, COLLIDE_FIXTURE, ["build_p_router"])
    checks.append(("变异体8 撤壳豁免必须把壳读成盲区（豁免是有账的）",
                   bool(c5c.unresolved.get("无法归类的表达式")) and c5c.passthrough == 0
                   and {p for p, _ in g5c} == {"/q"},
                   "unresolved=%s 壳 %d 现 %s" % (list(c5c.unresolved), c5c.passthrough, sorted(p for p, _ in g5c))))
    gr, cr = run_fixture(ROLE_FIXTURE, ["build_host_router"])
    mat = cr.role_matrix()
    checks.append(("角色维夹具逐条归属（含跨函数继承与嵌套 match 第二维）",
                   {p for p, _ in gr} == EXPECT_ROLE_PATHS and mat == EXPECT_ROLE_MATRIX
                   and len(cr.role_arms) == 5 and len(cr.role_wild) == 2,
                   "路径 %d 条，矩阵不符 %s，臂 %d 归不了属 %d" % (
                       len(gr), [k for k in EXPECT_ROLE_MATRIX if mat.get(k) != EXPECT_ROLE_MATRIX[k]] or "-",
                       len(cr.role_arms), len(cr.role_wild))))
    ARM_ANCHOR = "self.arm_label(role, self.arms_at(arms, seg_start + pos - offset))"
    nsr1 = patched_ns(ARM_ANCHOR, "role")
    gr1, cr1 = run_in(nsr1, ROLE_FIXTURE, ["build_host_router"])
    checks.append(("变异体9 撤逐臂判据 ⇒ 矩阵必须整张空（而挂载集合一条不少）",
                   all(not v for v in cr1.role_matrix().values())
                   and {p for p, _ in gr1} == EXPECT_ROLE_PATHS,
                   "非空格 %s 路径 %d 条" % ([p for p, v in cr1.role_matrix().items() if v] or "-", len(gr1))))
    nsr2 = patched_ns(ARM_ANCHOR,
                      "self.arm_label(None, self.arms_at(arms, seg_start + pos - offset))")
    gr2, cr2 = run_in(nsr2, ROLE_FIXTURE, ["build_host_router"])
    checks.append(("变异体10 撤跨函数继承 ⇒ 臂里调的 builder 体内挂载丢角色",
                   cr2.role_matrix().get("/iam/deep") == set()
                   and cr2.role_matrix().get("/iam/deep2") == set()
                   and cr2.role_matrix().get("/cloud-a") == {"CloudTier::A", "HostRole::Cloud"}
                   and cr2.role_matrix().get("/kg-only") == {"HostRole::Kg"},
                   "iam/deep %s iam/deep2 %s cloud-a %s kg %s" % (
                       sorted(cr2.role_matrix().get("/iam/deep", {None})),
                       sorted(cr2.role_matrix().get("/iam/deep2", {None})),
                       sorted(cr2.role_matrix().get("/cloud-a", set())),
                       sorted(cr2.role_matrix().get("/kg-only", set())))))
    nsr3 = patched_ns("        for m in MATCH_RE.finditer(text):", "        for m in []:")
    gr3, cr3 = run_in(nsr3, ROLE_FIXTURE, ["build_host_router"])
    checks.append(("变异体11 撤臂枚举 ⇒ 矩阵与臂账同时清空（枚举是矩阵的唯一来源）",
                   all(not v for v in cr3.role_matrix().values())
                   and not cr3.role_arms and not cr3.role_wild
                   and {p for p, _ in gr3} == EXPECT_ROLE_PATHS,
                   "臂 %d 归不了属 %d 非空格 %s" % (
                       len(cr3.role_arms), len(cr3.role_wild),
                       [p for p, v in cr3.role_matrix().items() if v] or "-")))
    nsr4 = patched_ns("enclosing = sorted([(lo, hi, lab) for lo, hi, lab in arms if lo <= pos < hi])",
                      "enclosing = sorted([(lo, hi, lab) for lo, hi, lab in arms if lo <= pos < hi])[-1:]")
    gr4, cr4 = run_in(nsr4, ROLE_FIXTURE, ["build_host_router"])
    checks.append(("变异体12 只取最内层臂 ⇒ 嵌套 match 的外层维被读丢",
                   cr4.role_matrix().get("/cloud-a") == {"CloudTier::A"}
                   and cr4.role_matrix().get("/kg-only") == {"HostRole::Kg"},
                   "cloud-a %s（应为两维齐全才对）" % sorted(cr4.role_matrix().get("/cloud-a", set()))))
    nsr5 = patched_ns("                pats = ARM_PAT_RE.findall(seg[:arrow + 2] if arrow is not None else seg)",
                      "                pats = ARM_PAT_RE.findall(seg)")
    gr5, cr5 = run_in(nsr5, ROLE_FIXTURE, ["build_host_router"])
    checks.append(("变异体13 撤模式边界 ⇒ 外层臂把嵌套 match 的臂名一起吃进来",
                   cr5.role_matrix().get("/cloud-x") == {"CloudTier::A", "HostRole::Cloud"}
                   and {p for p, _ in gr5} == EXPECT_ROLE_PATHS,
                   "cloud-x %s（无标签维本应只剩 HostRole::Cloud）路径 %d 条" % (
                       sorted(cr5.role_matrix().get("/cloud-x", set())), len(gr5))))
    nsr6 = patched_ns("                if recv_in_env:\n                    self.walk_text(env[recv.group(0)][1], base, origin, depth + 1, rel,\n                                   env[recv.group(0)][0], env, role)",
                      "                if False:\n                    pass")
    gr6, cr6 = run_in(nsr6, ROLE_FIXTURE, ["build_host_router"])
    checks.append(("变异体14 撤接收者展开 ⇒ let 绑定的 match 各臂整棵树消失",
                   {p for p, _ in gr6} == {"/common", "/api/kbdoc"}
                   and cr6.role_matrix() == {"/common": set(), "/api/kbdoc": set()},
                   "现 %s 矩阵 %s" % (sorted(p for p, _ in gr6), cr6.role_matrix())))
    mpr = mounted_by_path(cr)
    agg_base = role_aggregates(cr, mpr)
    same_r, diff_r, none_r = site_twins(agg_base, mpr)
    checks.append(("同 site／不同 site／根本没有 三档要把「角色是路径的属性」与「端点专属该角色」分开",
                   same_r == {"/api/kbdoc"} and diff_r == set()
                   and none_r == agg_base["labeled"] - {"/api/kbdoc"}
                   and agg_base["role_only"] == none_r
                   and cr.role_matrix().get("/api/kbdoc") == {"HostRole::Kb"},
                   "同 %s 不同 %s 根本没有 %d 条（配 role_only %d）kbdoc %s" % (
                       sorted(same_r), sorted(diff_r), len(none_r), len(agg_base["role_only"]),
                       sorted(cr.role_matrix().get("/api/kbdoc", set())))))
    nsr7 = patched_ns("            same.add(p)", "            diff.add(p)")
    gr7, cr7 = run_in(nsr7, ROLE_FIXTURE, ["build_host_router"])
    mp7 = nsr7["mounted_by_path"](cr7)
    same7, diff7, none7 = nsr7["site_twins"](nsr7["role_aggregates"](cr7, mp7), mp7)
    checks.append(("变异体15 撤同 site 判据 ⇒ 同档清空而挂载一条没少（分账读的就是这条判据）",
                   same7 == set() and diff7 == {"/api/kbdoc"} and none7 == none_r
                   and {p for p, _ in gr7} == EXPECT_ROLE_PATHS,
                   "同 %s 不同 %s 根本没有 %d 条（应与基线 %d 同）路径 %d 条" % (
                       sorted(same7), sorted(diff7), len(none7), len(none_r), len(gr7))))
    # —— 分支归因（`if <cond> { A } else { B }`）：分档语义先由夹具钉死，再逐判据配变异体 ——
    def bkt(ns_, cen_, variants=None):
        """用给定命名空间自己的三个读数函数给同一份 Census 分档（变异体必须读它自己的实现）。"""
        per_ = ns_["origin_conjunctions"](cen_, ns_["mounted_by_path"](cen_))
        return ns_["read_buckets"](per_, variants or COND_VARIANTS)

    gc0, cc0 = run_fixture(COND_FIXTURE, ["build_host_router"])
    bk0 = bkt(globals(), cc0)
    paths0 = {p for p, _ in gc0}
    # 两族标签各读各的（role_matrix 只认 `role:`、branch_matrix 只认 `cond:`），
    # 逐路径核对要看的是两族并集；析取标签 `A|B` 的次序由 sorted 归一，不钉字面顺序。
    def tags_of(cen_, canon=lambda t: "|".join(sorted(t.split("|")))):
        out = {}
        for p in set(cen_.role_matrix()) | set(cen_.branch_matrix()):
            out[p] = {canon(t) for t in
                      (cen_.role_matrix().get(p, set()) | cen_.branch_matrix().get(p, set()))}
        return out

    exp_tags = {p: {"|".join(sorted(t.split("|"))) for t in s}
                for p, s in EXPECT_COND_TAGS.items()}
    tm0 = tags_of(cc0)
    tag_bad = {p: sorted(tm0[p] ^ exp_tags.get(p, set()))
               for p in tm0 if tm0[p] != exp_tags.get(p, set())}
    checks.append(("分支夹具逐档归属：All 档含注册表支不含单角色臂，Kb 档含 kbdoc 不含 iam-only",
                   all(bk0[v] == EXPECT_COND_BUCKETS[v] for v in COND_VARIANTS)
                   and not tag_bad and len(cc0.cond_spans) == 2 and len(cc0.cond_blind) == 1,
                   "分档不符 %s 标签不符 %s 分支 %d 盲区 %d" % (
                       [v for v in COND_VARIANTS if bk0[v] != EXPECT_COND_BUCKETS[v]] or "-",
                       tag_bad or "-", len(cc0.cond_spans), len(cc0.cond_blind))))
    sub0 = {v: {p for p, _ in run_fixture(COND_FIXTURE, ["build_host_router"], pin=v)[0]}
            for v in COND_VARIANTS}
    checks.append(("夹具上两条通道（标签求值 vs 钉死取值走树）必须给出同一批分档",
                   all(sub0[v] == bk0[v] for v in COND_VARIANTS),
                   "分歧 %s" % ({v: sorted(sub0[v] ^ bk0[v]) for v in COND_VARIANTS
                                 if sub0[v] != bk0[v]} or "-")))
    checks.append(("复合条件（`&&`）归不出约束必须按名进盲区账，不许静默当没有分支",
                   len(cc0.cond_blind) == 1
                   and "&&" in sorted(e for _r, _p, e in cc0.cond_blind)[0]
                   and all("/never-mount" in bk0[v] for v in COND_VARIANTS),
                   "盲区 %s" % [e for _r, _p, e in sorted(cc0.cond_blind)]))
    gch, cch = run_fixture(CHAIN_COND_FIXTURE, ["build_host_router"])
    bkh = bkt(globals(), cch)
    subh = {v: {p for p, _ in run_fixture(CHAIN_COND_FIXTURE, ["build_host_router"], pin=v)[0]}
            for v in COND_VARIANTS}
    checks.append(("else-if 链：外层否定罩住整条链，链尾那一支不许出现在 All 档",
                   all(bkh[v] == EXPECT_CHAIN_BUCKETS[v] for v in COND_VARIANTS)
                   and all(subh[v] == bkh[v] for v in COND_VARIANTS),
                   "不符 %s" % ({v: sorted(bkh[v] ^ EXPECT_CHAIN_BUCKETS[v])
                                 for v in COND_VARIANTS if bkh[v] != EXPECT_CHAIN_BUCKETS[v]} or "-")))
    nsr8 = patched_ns("        for m in IF_HEAD_RE.finditer(text):", "        for m in []:")
    gr8, cr8 = run_in(nsr8, COND_FIXTURE, ["build_host_router"])
    bk8 = bkt(nsr8, cr8)
    # 撤了分支枚举，臂那一半还在生效 ⇒ All 档不会变宽（它本来就被臂排除单角色子树）；
    # 真正被读大的是**单角色档**：注册表支里的 /all-only 会凭空进每个角色桶。
    checks.append(("变异体16 撤分支枚举 ⇒ 分支标签整族消失，`if role == All` 的支挂进单角色档（挂载一条没少）",
                   not cr8.cond_spans and all(not v for v in cr8.branch_matrix().values())
                   and all("/all-only" in bk8[v] for v in ("Kg", "Cloud", "Kb", "Iam"))
                   and all("/all-only" not in bk0[v] for v in ("Kg", "Cloud", "Kb", "Iam"))
                   and {p for p, _ in gr8} == paths0,
                   "分支 %d 泄漏单角色档 %s 总数 %d" % (
                       len(cr8.cond_spans),
                       [v for v in COND_VARIANTS if v != "All" and "/all-only" not in bk8[v]] or "-",
                       len(paths0))))
    nsr9 = patched_ns('pos, neg = "=HostRole::" + variant, "!HostRole::" + variant',
                      'pos, neg = "=HostRole::" + variant, "=HostRole::" + variant')
    gr9, cr9 = run_in(nsr9, COND_FIXTURE, ["build_host_router"])
    bk9 = bkt(nsr9, cr9)
    # 负支写成正支并不等于"else 支进了 All 档"：else 支上还叠着臂约束 HostRole::Kb，
    # 与 =HostRole::All 永不同真 ⇒ 条目从**所有**档里凭空消失（分档读小，挂载一条没少）。
    checks.append(("变异体17 把负支读成正支 ⇒ 单角色档条目凭空消失而挂载账不动",
                   "/api/kbdoc" not in bk9["Kb"]
                   and "/api/kbdoc" not in set().union(*bk9.values())
                   and "/api/kbdoc" in bk0["Kb"]
                   and "/all-only" in bk9["All"] and {p for p, _ in gr9} == paths0,
                   "Kb 档 %s 全档并集含 kbdoc %s All 档 %d 条 挂载 %d 条" % (
                       sorted(bk9["Kb"]), "/api/kbdoc" in set().union(*bk9.values()),
                       len(bk9["All"]), len(gr9))))
    nsra = patched_ns("        for lo, hi in ap + cp:", "        for lo, hi in []:")
    suba = {v: {p for p, _ in run_in(nsra, COND_FIXTURE, ["build_host_router"], pin=v)[0]}
            for v in COND_VARIANTS}
    cza = run_in(nsra, COND_FIXTURE, ["build_host_router"], pin="Kb")[1]
    checks.append(("变异体18 撤剪枝区间 ⇒ 钉死取值五档塌成一档（分档账会退化成总账）",
                   all(len(suba[v]) == len(paths0) for v in COND_VARIANTS)
                   and cza.prune_spans == 0 and not cza.prune_intervals,
                   "各档 %s 剪枝 %d 次／去重 %d 段" % ({v: len(suba[v]) for v in COND_VARIANTS},
                                                       cza.prune_spans, len(cza.prune_intervals))))
    nsrb = patched_ns("        rt = [t for t in toks if not is_cond_token(t)]\n"
                      "        ct = [t for t in toks if is_cond_token(t)]",
                      "        rt = toks\n        ct = []")
    grb, crb = run_in(nsrb, COND_FIXTURE, ["build_host_router"])
    checks.append(("变异体19 撤标签分族（分支挤进臂通道）⇒ 分支账归零而臂账被顶掉",
                   not any(crb.branch_matrix().values())
                   and any(crbo for crbo in crb.role_matrix().values())
                   and {p for p, _ in grb} == paths0,
                   "分支账非空格 %d 臂账 %s" % (
                       len([p for p, v in crb.branch_matrix().items() if v]),
                       {p: sorted(v) for p, v in crb.role_matrix().items() if v}.get("/all-only", "-"))))
    nsrc = patched_ns("        i = m.end()\n        if not IF_INNER_RE.match(text, i):",
                      "        i = m.end()\n        if True:")
    grc, crc = run_in(nsrc, CHAIN_COND_FIXTURE, ["build_host_router"])
    bkc = bkt(nsrc, crc)
    checks.append(("变异体20 撤整条 else-if 链的罩住 ⇒ 链尾那支凭空出现在 All 档",
                   "/other2" in bkc["All"] and "/other2" not in bkh["All"]
                   and {p for p, _ in grc} == {p for p, _ in gch},
                   "变异后 All 档 %s ／ 基线 All 档 %s" % (sorted(bkc["All"]), sorted(bkh["All"]))))
    led_num = dict(
        scanned_files=1377, branch_spans=2,
        assembly_nodes_indexed=10, indexed_by_name=7, indexed_by_signature=5, signature_only=3,
        entered_total=8, entered_unique=6, entered_sig_only=1, passthrough=2,
        ca_paired_n=2, unresolved_counts={},
        routes_declared_len=6, routes_parsed=6, routes_paths_distinct=14,
        mounted_literal=21, mounted_normalized=20,
        m_in_table=12, m_ca_only=1, m_only_mount=7, unreg_labeled=1,
        t_not_mounted=1, t_explained=1,
        branch_tagged=15, common_surface=5, tag_common_inter=0,
        only_all=9, multi_bucket=11, never=0,
        labeled=6, unlabeled=20, twin_same=5, twin_diff=1, twin_none=0, role_only=0,
        multi_dim=0, excl_sum=6, unreg_all=7, xdiff_sum=0,
        role_arms_named=3, role_arms_wild=1, nonrole=0, branch_blind=0,
        surface_failed=0, matrix_failed=0,
        bucket_All=20, arm_All=20, subst_All=20, removed_All=0, prune_occ_All=2, prune_dist_All=2,
        bucket_Kb=8, arm_Kb=20, subst_Kb=8, removed_Kb=12, prune_occ_Kb=3, prune_dist_Kb=2,
    )
    led_spec = ledger_spec(["All", "Kb"])
    led_rows = ledger_eval(led_spec, led_num)
    led_red0 = [r["rid"] for r in led_rows if not r["ok"]]
    # 单一算源的机械含义：一个数被两行引用时，改一处必须两行同时红。
    led_rows1 = ledger_eval(led_spec, dict(led_num, m_only_mount=9))
    led_red1 = sorted(r["rid"] for r in led_rows1 if not r["ok"])
    led_rows2 = ledger_eval(led_spec, dict(led_num, branch_tagged=16))
    led_red2 = sorted(r["rid"] for r in led_rows2 if not r["ok"])
    checks.append(("变异体21 改一个被两行引用的加数 ⇒ 恰好那两行同时红（不是全部红也不是只有一行）",
                   led_red0 == [] and led_red1 == ["M-PART", "UNREG-CROSS"] and led_red2 == ["TAG-COMMON"],
                   "基线红 %s ／ 改 m_only_mount 红 %s ／ 改 branch_tagged 红 %s" % (led_red0, led_red1, led_red2)))
    checks.append(("变异体22 公式 id 撞车 ⇒ 后写的顶掉先写的必须显形（条数自洽不等于没顶）",
                   ledger_dups(led_rows) == [] and ledger_dups(led_rows + led_rows[:1]) == ["ASM-CLOSE"],
                   "基线撞车 %s ／ 复制首行 %s（条数 %d 配 %d）" % (
                       ledger_dups(led_rows), ledger_dups(led_rows + led_rows[:1]),
                       len(led_rows), len(led_rows) + 1)))
    led_lines = [u'在册 72 ＝ 按名字 60 ＋ 只靠签名新增 12（这条是真账）。',
                 u'那一格印的是 71 ＝ 60 ＋ 30，缺一个数。',
                 u'改正后为「71 ＝ 60 ＋ 30」（这是引文）。',
                 u'装配 60 ＋ `（重叠 12 条）` ＋ 30 ＝ 90。']
    v23, q23, n23, b23 = audit_prose_arith(led_lines, None)
    try:
        with open(os.path.join(REPO, LEDGER_DOC_PATH.replace("/", os.sep)), "rb") as fh:
            doc_lines23 = fh.read().decode("utf-8").split("\n")
        v23b_all = audit_prose_arith(doc_lines23, ledger_block_span(doc_lines23))
        v23b, n23b = v23b_all[0], v23b_all[2]
        doc_note23 = "真文档 %d 行复算，主张 %d 条，不闭合 %d 条" % (
            len(doc_lines23), n23b, len(v23b))
        # 判集塌缩＝零证据被印成满把握：托管块外的散文若一条主张都不剩，那是尺子掉了不是账干净了
        doc_ok23 = len(v23b) == 0 and n23b >= 1
    except IOError as exc:
        doc_note23 = "真文档读不到：%s" % exc
        doc_ok23 = False
    checks.append(("变异体23 散文算术审计的两个失效方向：漏抓假加法／把引文当主张，都要红；"
                   "代码段里的示意等式不许成第二条账；真文档必须零不闭合**且判集非空**",
                   len(v23) == 1 and v23[0][0] == 2 and len(q23) == 1 and q23[0][0] == 3
                   and n23 == 2 and FL.blind_counts(b23).get("code") == 1 and doc_ok23,
                   "植假 %s（第 %s 行）引文 %s 主张 %d 条 盲区 %s ｜ %s" % (
                       [x[0] for x in v23], (v23[0][0] if v23 else "-"),
                       [x[0] for x in q23], n23, sorted(FL.blind_counts(b23).items()),
                       doc_note23)))
    # 托管块的跳过口径：`ledger_block_span` 给 0 基、`scan_text` 按 1 基跳块。换算错了**不会报红**，
    # 只会把块外那条真主张整条吞掉（分母从 1 变 0）——分母变小比多算一次难看见，所以拿它做针。
    blk_lines = [u'在册 72 ＝ 按名字 60 ＋ 12',
                 LEDGER_BEGIN + " ledger",
                 u'表内坏账 71 ＝ 60 ＋ 30',
                 LEDGER_END]
    bs = ledger_block_span(blk_lines)
    v26, q26, n26, b26 = audit_prose_arith(blk_lines, bs)
    v26o, q26o, n26o, b26o = audit_prose_arith(blk_lines, (bs[0] - 1, bs[1] - 1))
    checks.append(("变异体26 托管块换算差一行会把块外真主张整条吞掉（块内坏账反被漏掉不报红）⇒ "
                   "正算必须 1 条主张零红、错算必须显形为分母归零",
                   n26 == 1 and v26 == [] and n26o == 0 and v26o == [],
                   "正算 主张 %d 红 %d ／ 错算(整块下移一行) 主张 %d 红 %d" % (
                       n26, len(v26), n26o, len(v26o))))
    # 「什么是一条主张」只有一份实现才算归一化：把**模块**的旋钮拧动，本模式的判决必须跟着动。
    # 若这里还留着一把自己写的尺子（原 CHAIN_RE），拧模块的旋钮本行就不会翻——这枚针钉的是"没有第二源"。
    _oq = FL.QUOTE_SPAN_RE
    try:
        FL.QUOTE_SPAN_RE = re.compile(r"(?!)")
        v27, q27, n27, b27 = audit_prose_arith(led_lines, None)
    finally:
        FL.QUOTE_SPAN_RE = _oq
    checks.append(("变异体27 散文算术审计确实走共享识别器：拧 `formula_ledger` 的引文旋钮，"
                   "本模式的判决跟着翻（引文变主张且判红），本地不留第二把尺",
                   len(q27) == 0 and len(v27) == 2 and v27[0][0] == 2 and v27[1][0] == 3,
                   "拧后 引文 %d 条 红 %s 第 %s 行" % (len(q27), len(v27), [x[0] for x in v27])))
    blk24 = ["甲", "乙 ＝ 1 ＋ 2", "丙", "丁"]
    checks.append(("变异体24 托管块比对：同文本必须报零差，改一个数字必须点出那一行的下标，"
                   "多一行必须算差（集合差 ≠ 逐位差）",
                   block_diff(blk24, list(blk24)) == []
                   and block_diff(["乙 ＝ 1 ＋ 3", ], ["乙 ＝ 1 ＋ 2", ]) == [0]
                   and block_diff(blk24, blk24 + ["戊"]) == [4]
                   and block_diff(blk24, blk24[:3]) == [3],
                   "同 %s 改数字 %s 加长 %s 截短 %s" % (
                       block_diff(blk24, list(blk24)),
                       block_diff(["乙 ＝ 1 ＋ 3", ], ["乙 ＝ 1 ＋ 2", ]),
                       block_diff(blk24, blk24 + ["戊"]), block_diff(blk24, blk24[:3]))))
    # 镜像判据只比"两次渲染是否相同"，一个从未替换的占位符会稳定地错下去 ⇒ 必须自己成一条判决。
    spec25 = [s if s[0] != "DECL-PARSE" else s[:5] + (s[5] + " %s",) for s in led_spec]
    led_rows25 = ledger_eval(spec25, led_num)
    red25 = sorted(r["rid"] for r in led_rows25 if not r["ok"])
    anti25 = [r for r in led_rows if r["rid"] == "ASM-ANTI"][0]
    checks.append(("变异体25 渲染留未替换占位符必须判红（逐字节相同的镜像照样把错处印进文档），"
                   "而反对照那行的重叠数必须由本行的值现推",
                   red25 == ["DECL-PARSE"] and "%s" not in anti25["cite"]
                   and "重叠 2 条" in anti25["cite"],
                   "植入后红 %s ／ ASM-ANTI 说明「%s」" % (red25, anti25["cite"])))
    # 判决出口账：这台仪器的红写成 print 文案＋整数累加器（不是把消息塞进累加器），
    # 所以「每条红都有针」要先有一条能数出所有红的尺。本组钉三件各自独立的事：
    # 站点数＝基线（棘轮）、站点数＝唯一前缀数（第 30 型）、站点非空（第 35 型），覆盖按名点出。
    sites = outlet_sites(JUDGE_SRC)
    universe = outlet_universe(JUDGE_SRC)
    resolved = dict((f, [t for t in universe if f in t]) for f in OUTLET_COVERED)
    dead_keys = ["%s(命中 %d)" % (f, len(h)) for f, h in sorted(resolved.items()) if len(h) != 1]
    covered = sorted(set(h[0] for _f, h in resolved.items() if len(h) == 1))
    uncovered = [t for t in universe if t not in covered]
    dup_reg = len(covered) != len(OUTLET_COVERED)
    sample = uncovered[:3]
    checks.append(("夹具L·表亲 判决出口账（从判据源码现取，不写清单）：站点数＝基线、站点数＝唯一前缀数、"
                   "登记片段必须各解析到恰一条出口（0 条＝死键、≥2 条＝歧义、两条片段解析到同一条＝冗余）、"
                   "空集不许当判决（覆盖＋未覆盖＝宇宙是划分的恒等式，没牙，故不作为判据）",
                   len(sites) == OUTLET_BASELINE and len(universe) == len(sites)
                   and outlet_den_ok(len(sites)) and not dead_keys and not dup_reg,
                   "站点 %d 配基线 %d｜唯一前缀 %d｜已覆盖 %d（登记表 %d 名）｜未覆盖 %d（点名 %d 名 %s）"
                   "｜死键或歧义 %s｜冗余 %s" % (
                       len(sites), OUTLET_BASELINE, len(universe), len(covered), len(OUTLET_COVERED),
                       len(uncovered), len(sample), sample, dead_keys, dup_reg)))
    planted = ("\ndef _planted_outlet():\n    print('FAIL 植入的出口只活在这份内存图像里 %s', 'x')\n"
               "    print('INFO 植入的告示同样是一条出口 %s', 'x')\n")
    grown = outlet_universe(JUDGE_SRC + planted)
    checks.append(("变异体28 判据里新写一条红，出口账必须当场多两条（清单会漏掉明天新加的那条红，"
                   "现取的宇宙不会）——只在本文件源码的内存图像上加，盘上一个字没动",
                   len(grown) == len(universe) + 2
                   and sorted(set(grown) - set(universe)) == ["FAIL 植入的出口只活在这份内存图像里",
                                                              "INFO 植入的告示同样是一条出口"],
                   "现取 %d → 植入后 %d｜新增名 %s" % (len(universe), len(grown), sorted(set(grown) - set(universe)))))
    clash = ("\ndef _planted_clash():\n    print('FAIL 文档镜像：方案文档不在 %s', 'x')\n")
    clash_sites = outlet_sites(JUDGE_SRC + clash)
    checks.append(("变异体31 植入一条**与前缀共用**的红（第 30 型）⇒ 站点数必须加一而唯一前缀数不动，"
                   "「站点＝唯一前缀」那条必须判否——共用前缀的两条红里撤一条没人会发现",
                   len(clash_sites) == len(sites) + 1 and len({p for p, _f, _l in clash_sites}) == len(universe)
                   and len(clash_sites) != len({p for p, _f, _l in clash_sites}),
                   "植入后 站点 %d 配唯一前缀 %d｜共用者 %s" % (
                       len(clash_sites), len({p for p, _f, _l in clash_sites}),
                       [s for s in clash_sites if s[0] == "FAIL 文档镜像：方案文档不在"])))
    mns = patched_ns('return FL.red_outlet_sites(src_text, "print", OUTLET_PREFIXES)',
                     'return FL.red_outlet_sites(src_text, "print", ("FALX ",))')
    shrunk = mns["outlet_sites"](JUDGE_SRC)
    checks.append(("变异体29 撤掉出口账的前缀表（把这台仪器的两个标签换成一个不存在的串）⇒ 站点塌缩到 0，"
                   "分母谓词必须跟着判否，不许静默通过（过滤器已住在单源里，这一枚是从本仪器的调用点拧的）",
                   len(shrunk) == 0 and not outlet_den_ok(len(shrunk)) and outlet_den_ok(len(sites)),
                   "塌缩后 %d 条｜塌缩时分母 %s｜真实图像分母 %s" % (
                       len(shrunk), outlet_den_ok(len(shrunk)), outlet_den_ok(len(sites)))))
    bns = patched_ns("OUTLET_BASELINE = 13", "OUTLET_BASELINE = 12")
    checks.append(("变异体30 三条守卫各管一件事的牙：把 OUTLET_BASELINE 往下拧一格（内存图像）⇒ 棘轮必须判否，"
                   "而分母谓词必须照旧判是（站点塌没塌与基线写几无关）——一条谓词若同时管两件事，"
                   "红字就分不清是尺子掉了还是账变了",
                   len(sites) != bns["OUTLET_BASELINE"] and bns["outlet_den_ok"](len(sites)),
                   "真实站点 %d 配拧后基线 %d（棘轮 %s）｜拧后分母 %s" % (
                       len(sites), bns["OUTLET_BASELINE"],
                       len(sites) == bns["OUTLET_BASELINE"], bns["outlet_den_ok"](len(sites)))))
    # 委托之后必须证明"这条调用真的走单源"：拧共享门限，本仪器的出口账要跟着变；
    # 不变＝这里还藏着一份本地实现，明天的口径改动只会落在那份没人看的实现上。
    orig_min = FL.OUTLET_MIN_PREFIX
    FL.OUTLET_MIN_PREFIX = 400
    try:
        knob = outlet_sites(JUDGE_SRC)
        knob_pref = {p for p, _f, _l in knob}
    finally:
        FL.OUTLET_MIN_PREFIX = orig_min
    checks.append(("变异体32 出口抽取只有单源一个实现：抬高 FL 的前缀门限必须改动本仪器现取的站点"
                   "（截短头全过不了关 ⇒ 收整串、含占位符），且还原后门限回到原值、站点回到基线",
                   len(knob) == len(sites) and knob_pref != set(universe)
                   and any("%" in p for p in knob_pref)
                   and FL.OUTLET_MIN_PREFIX == orig_min and len(outlet_sites(JUDGE_SRC)) == OUTLET_BASELINE,
                   "门限 %d→400→%d｜站点 %d→%d｜前缀集合改变 %s｜拧后含占位符 %d 条" % (
                       orig_min, FL.OUTLET_MIN_PREFIX, len(sites), len(knob),
                       knob_pref != set(universe), sum(1 for p in knob_pref if "%" in p))))
    # ── 夹具M·十三条出口的逐枚对照针（§1.8s 的甲账）─────────────────────────
    # 每枚针在一份内存图像上把该条出口打红，同时看三格：目标命中 ≥1、其余十二条一条都不多命中、
    # 末行「红 N 项」等于预注册期望。第三格是本轮长出来的尺：只看文案会漏掉「印 FAIL 却不累加」
    # 那两条（工件缺失／工件解析不了），它们靠这条尺才现形——所以它们的期望是 3 而不是 1。
    # 语料复用只许开在读盘与两处纯函数上：Census 每枚针照旧新建、load 与 run_roots 照旧真跑。
    # （反面实测：把一个已走树的 Census 焊给各枚针，同一份盘先印「红 5 项」后印「红 2 项」，
    #   引用对账从 一致 28／漂移 27 变成 24／31，托管块那条红在没有任何注入时自己响。）
    entry_of_frag = dict((f, h[0]) for f, h in resolved.items() if len(h) == 1)
    KEY_LIST = [entry_of_frag[f] for f in OUTLET_COVERED if f in entry_of_frag]
    MEMO = {"read": {}, "mask": {}, "defs": {}}
    REAL_OPEN = open

    def battery_ns(**patches):
        ns = {"__file__": __file__, "__name__": "outlet_battery"}
        exec(compile(CODE, "<battery>", "exec"), ns)
        rt, mk, dfn = ns["read_text"], ns["mask"], ns["index_fn_defs"]

        def _rt(path):
            key = str(path)
            if key not in MEMO["read"]:
                MEMO["read"][key] = rt(path)
            return MEMO["read"][key]

        def _mk(text):
            if text not in MEMO["mask"]:
                MEMO["mask"][text] = mk(text)
            return MEMO["mask"][text]

        def _df(text):
            if text not in MEMO["defs"]:
                MEMO["defs"][text] = dfn(text)
            return MEMO["defs"][text]

        ns["read_text"], ns["mask"], ns["index_fn_defs"] = _rt, _mk, _df
        for k, v in patches.items():
            ns[k] = v
        return ns

    warm = battery_ns()
    DOC_ABS_B = os.path.join(warm["REPO"], warm["LEDGER_DOC_PATH"].replace("/", os.sep))
    with REAL_OPEN(DOC_ABS_B, "rb") as fh:
        DOC_IMG = fh.read()
    doc_lines = DOC_IMG.decode("utf-8").split("\n")
    doc_span = warm["ledger_block_span"](doc_lines)
    DISK_BLOCK = ("\n".join(doc_lines[doc_span[0]:doc_span[1] + 1]) + "\n") if doc_span else ""

    def clean_render(_rows, _meta):
        return DISK_BLOCK

    NULL_CROSS = {"LEDGER_CROSS": [], "render_ledger_md": clean_render}

    def with_null(extra):
        d = dict(NULL_CROSS)
        d.update(extra)
        return d

    def fire(ns, write=False):
        seen = []
        ns["print"] = lambda *a, **k: seen.append(" ".join(str(x) for x in a))

        class _Args(object):
            pass
        _Args.write = write
        _Args.show = 12
        rc = ns["cmd_ledger"](_Args)
        blob = "\n".join(seen)
        hits = dict((t, blob.count(t)) for t in KEY_LIST)
        m = re.search(r"LEDGER (?:PASS|FAIL)（红 (\d+) 项", blob)
        return hits, (int(m.group(1)) if m else None), rc

    class _MemFile(object):
        def __init__(self, data=None, sink=None):
            self._data = data
            self._sink = sink

        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

        def read(self):
            return self._data

        def write(self, data):
            if self._sink is not None:
                self._sink.append(data)

    DOC_MEM_WRITES = []
    DOC_MEM_SINK = []
    DOC_MEM_READS = [0]

    def open_doc_mem(path, mode="r", *a, **k):
        sp = str(path).replace("\\", "/")
        if not sp.endswith(".md"):
            return REAL_OPEN(path, mode, *a, **k)
        if "wb" in mode:
            DOC_MEM_WRITES.append(sp)
            return _MemFile(sink=DOC_MEM_SINK)
        DOC_MEM_READS[0] += 1
        return _MemFile(DOC_IMG if DOC_MEM_READS[0] == 1 else DOC_IMG[:len(DOC_IMG) // 2])

    class _BadJson(object):
        """json.load 只要一个带 read() 的出口，不必为此在本文件多开一个 io 依赖。"""

        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

        def read(self):
            return "{not json"

    def open_bad_json(path, mode="r", *a, **k):
        if str(path).replace("\\", "/").endswith("recon-battery-fake.json"):
            return _BadJson()
        return REAL_OPEN(path, mode, *a, **k)

    cases = []

    def one(frag, patches, expect_bad, write=False):
        entry = entry_of_frag.get(frag)
        if entry is None:
            cases.append((frag, False, "登记片段没解析到唯一出口 ⇒ 无从打针"))
            return
        hits, bad, rc = fire(battery_ns(**patches), write=write)
        others = sorted(k for k, v in hits.items() if k != entry and v)
        cases.append((frag, hits.get(entry, 0) >= 1 and not others and bad == expect_bad,
                      "目标命中 %s｜总红 %s 配期望 %s｜其余出口 %s｜rc %s"
                      % (hits.get(entry, 0), bad, expect_bad, others or "[]", rc)))

    FAKE_RENDER = {"LEDGER_CROSS": [],
                   "render_ledger_md": lambda rows, meta: DISK_BLOCK + "本轮造的假行\n"}
    base_hits, base_bad, base_rc = fire(battery_ns())
    base_quiet = sorted(k for k, v in base_hits.items() if v and k != "INFO 漂移")
    cases.append(("基线（未注入）", base_rc == 0 and base_bad == 0 and not base_quiet,
                  "rc %s｜总红 %s｜除漂移外出口 %s（漂移 %d 条是并发语料的常态读数；"
                  "托管块与盘上逐字节相同＝复用只开在读盘层的见证）"
                  % (base_rc, base_bad, base_quiet or "[]", base_hits.get("INFO 漂移", 0))))
    one("找不到工件", with_null({"newest_artifact": lambda prefix: None}), 3)
    one("FAIL 工件", with_null({"open": open_bad_json,
                               "newest_artifact": lambda prefix: "recon-battery-fake.json"}), 3)
    one("方案文档不在", with_null({"LEDGER_DOC_PATH": "docs/__no_such_for_battery__.md"}), 1)
    one("加法不闭合", with_null({"audit_prose_arith": lambda lines, span:
                                ([(7, "样例主张", "各边不对")], [], 1, [])}), 1)
    one("标记缺失", with_null({"ledger_block_span": lambda lines: None}), 1)
    one("已有成对标记", with_null({"ledger_block_span": lambda lines: None}), 1, write=True)
    one("现渲染不一致", dict(NULL_CROSS,
                            render_ledger_md=lambda rows, meta: DISK_BLOCK + "本轮造的假行\n"), 1)
    one("逐行核验没过", dict(FAKE_RENDER, audit_len_guard=lambda o, nw, at, end, nb: 1), 1, write=True)
    one("回读不一致", dict(FAKE_RENDER, open=open_doc_mem), 1, write=True)
    one("引用不成立", {"LEDGER_CROSS": [("census", "no_such_key_for_battery", "scanned_files", "scalar")],
                      "render_ledger_md": clean_render}, 1)
    one("漂移", {"LEDGER_CROSS": [("census", "scanned_files", "scanned_files", "scalar")],
                "art_get": lambda art, key: (10 ** 9 + 7, True),
                "render_ledger_md": clean_render}, 0)
    ag = warm["audit_len_guard"]
    OLD_B = ["p0", "p1", "BLOCK", "e0"]
    ag_seen = {}
    for tag, new in (("干净", ["p0", "p1", "BLOCK", "e0"]),
                     ("只改块内", ["p0", "p1", "BLOCK2", "e0"]),
                     ("前段被改", ["X", "p1", "BLOCK", "e0"]),
                     ("后段被改", ["p0", "p1", "BLOCK", "Z"])):
        got = []
        warm["print"] = lambda *a, **k: got.append(" ".join(str(x) for x in a))
        r = ag(OLD_B, new, 2, 2, 1)
        ag_seen[tag] = (r, sorted(k for k in KEY_LIST if k in "\n".join(got)))
    for frag, tag in (("块外前段", "前段被改"), ("块外后段", "后段被改")):
        entry = entry_of_frag.get(frag)
        r, hitk = ag_seen[tag]
        cases.append((frag, entry is not None and r == 1 and hitk == [entry]
                      and ag_seen["干净"] == (0, []),
                      "最小图像「%s」返回 %s｜命中 %s｜干净样例 %s" % (tag, r, hitk, ag_seen["干净"])))
    cases.append(("只改块内", ag_seen["只改块内"] == (0, []),
                  "块内改动是这块尺的本职工作，返回 %s" % (ag_seen["只改块内"][0],)))
    with REAL_OPEN(DOC_ABS_B, "rb") as fh:
        DOC_AFTER = fh.read()
    cases.append(("写路由未落盘", DOC_AFTER == DOC_IMG,
                  "盘上文档 %d 字节 配针前 %d 字节｜内存缓冲收下的写 %d 次 %s"
                  % (len(DOC_AFTER), len(DOC_IMG), len(DOC_MEM_SINK), sorted(set(DOC_MEM_WRITES)) or "[]")))
    needle_frags = sorted(f for f, _ok, _d in cases if f in OUTLET_COVERED)
    fired = sum(1 for _f, ok, _d in cases if ok)
    for frag, ok, detail in cases:
        checks.append(("夹具M 出口对照针「%s」只红自己" % frag, ok, detail))
    checks.append(("夹具M·分母 登记表 13 名 ↔ 逐格 13 枚，缺一格即登记表在说谎（覆盖登记的是片段，"
                   "由现取宇宙解析成全名；总格数＝13 枚针＋基线＋只改块内＋写路由四格）",
                   needle_frags == sorted(OUTLET_COVERED) and len(cases) == len(OUTLET_COVERED) + 3
                   and fired == len(cases),
                   "登记 %d 名逐格点名 %d 枚｜总格 %d｜全绿 %s" % (
                       len(OUTLET_COVERED), len(needle_frags), len(cases), fired == len(cases))))
    # ── 夹具N·出口"有牙"静态筛（§1.8t：印了不拦从此是常驻判据，不是一次性探针）────
    # 这台仪器的红是两条独立通道：印一条带标签的文案 ＋ 给整数加一。只钉文案的尺子看不见
    # 第二条通道断了（本轮实测：三份工件全删仍 LEDGER PASS／红 0 项，根因是 `bad = 0`
    # 落在两条红之后）。这一格把"每个站点都得有计数动作"钉成判据。
    screen = FL.red_accum_screen(JUDGE_SRC, ("FAIL ", "INFO "))
    sc_tally = {}
    for _h, _f, _l, _k, _n in screen:
        sc_tally[_k] = sc_tally.get(_k, 0) + 1
    toothless = sorted(set(_h for _h, _f, _l, _k, _n in screen if _k in ("reset", "blind")))
    checks.append(("夹具N·分母 有牙筛的站点数必须等于出口基线（少一站＝抽取口径动了而这把尺没跟上）",
                   len(screen) == OUTLET_BASELINE,
                   "站点 %d 配基线 %d｜档位分布 %s" % (
                       len(screen), OUTLET_BASELINE, sorted(sc_tally.items()))))
    checks.append(("夹具N 十三条出口逐站点都得落在 inc／ret 两档：reset＝印了不拦的缺陷形状，"
                   "blind＝本尺看不见的那种（不许被读成没问题）",
                   len(screen) == OUTLET_BASELINE and not toothless,
                   "无牙站点 %s｜ret 档由 audit_len_guard 的返回值通道来（两型调用点：同行累加／"
                   "存变量后由 if 分支累加）" % (toothless or "[]")))

    drives = sorted(set(_h for _h, _f, _l, _k, _n in screen if _k == "driven"))
    checks.append(("夹具N·边界 第三档 `driven`（循环集合的名字同名出现在退出码里）在本仪器一处都不许出现"
                   "——它的红全部住在累加器或返值通道上；这一档在本侧点火＝有人把红改成只靠同名退出码撑着",
                   not drives, "本仪器 driven 站点 %s" % (drives or "[]")))
    SISTER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "check-doc-formulas.py")
    try:
        with open(SISTER, encoding="utf-8") as fh:
            SCODE = fh.read().replace("\r\n", "\n").split("\ndef cmd_selftest")[0]
        sister_err = None
    except (IOError, OSError) as exc:
        SCODE, sister_err = None, "%s" % exc
    checks.append(("夹具N·丙 姊妹仪器源码必须读得到（读不到不是「那边没有红」）",
                   SCODE is not None, "路径 %s｜%s" % (
                       os.path.relpath(SISTER, REPO).replace("\\", "/"), sister_err or "已读"))
                  )
    if SCODE is not None:
        s_uni = FL.red_outlet_universe(SCODE, "print", ("FAIL ", "INFO "))
        s_scr = FL.red_accum_screen(SCODE, ("FAIL ", "INFO "))
        s_kinds = sorted(set(k for _h, _f, _l, k, _n in s_scr))
        s_driven = sorted(set(h for h, _f, _l, k, _n in s_scr if k == "driven"))
        checks.append(("夹具N·丙 姊妹仪器那几条 print 形态的红必须全落在 inc／ret／driven 三档之内，"
                       "一条都不许留在 blind（「循环集合驱动退出码」这一型自本轮起有档位，不再靠收据登记）",
                       len(s_scr) == len(s_uni) > 0 and bool(s_kinds)
                       and set(s_kinds) <= set(["inc", "ret", "driven"]),
                       "print 形态站点 %d 配宇宙 %d｜档位 %s｜逐站点 %s" % (
                           len(s_scr), len(s_uni), s_kinds,
                           ["%s@%d" % (h, l) for h, _f, l, k, _n in s_scr])))
        RET_OLD = '    return 1 if (agg["violations"] or outside_verdict(out)) else 0\n'
        n34 = SCODE.count(RET_OLD)
        img34 = SCODE.replace(RET_OLD, "    return 0\n", 1)
        sc34 = FL.red_accum_screen(img34, ("FAIL ", "INFO "))
        b34 = sorted(set(h for h, _f, _l, k, _n in sc34 if k == "blind"))
        checks.append(("变异体34 把姊妹仪器那句退出码撤成 `return 0` ⇒ driven 的两条必须立刻退成 blind"
                       "（这一档的牙真长在退出码那一行上，而不是换了个名字的豁免桶）",
                       n34 == 1 and img34 != SCODE and len(sc34) == len(s_scr)
                       and b34 == s_driven and bool(s_driven),
                       "锚点命中 %d 次｜撤后 blind %s｜撤前 driven %s" % (n34, b34, s_driven)))

    def m33_pairs(pairs):
        """逐对替换，任何一对锚点数不为 1 就整对作废（宁可不红，不可拿没动过图的假对照报红）。"""
        text = JUDGE_SRC
        hits = []
        for old, new in pairs:
            n = text.count(old)
            hits.append("%d" % n)
            if n != 1:
                return None, hits
            text = text.replace(old, new, 1)
        return text, hits

    MUT33 = [
        ('            print("FAIL 引用找不到工件 %s*.json（%s 这一路的每个数都没了来源）"\n'
         '                  % (LEDGER_ART_PREFIX[tag], tag))\n            bad += 1\n',
         '            print("FAIL 引用找不到工件 %s*.json（%s 这一路的每个数都没了来源）"\n'
         '                  % (LEDGER_ART_PREFIX[tag], tag))\n'),
        ('            print("FAIL 工件 %s 解析不了：%s" % (path, exc))\n            bad += 1\n',
         '            print("FAIL 工件 %s 解析不了：%s" % (path, exc))\n'),
        ('    bad = 0\n    loaded = {}\n', '    loaded = {}\n'),
        ('                 "arts": arts, "arts_gen": arts_gen})\n',
         '                 "arts": arts, "arts_gen": arts_gen})\n    bad = 0\n'),
    ]
    img33, hits33 = m33_pairs(MUT33)
    sc33 = FL.red_accum_screen(img33, ("FAIL ", "INFO ")) if img33 else []
    t33 = sorted(set(h for h, _f, _l, k, _n in sc33 if k in ("reset", "blind")))
    tt33 = {}
    for _h, _f, _l, k, _n in sc33:
        tt33[k] = tt33.get(k, 0) + 1
    checks.append(("变异体33 把本轮修好的那处还原成修前（撤两条 `bad += 1`＋把初值挪回红之后，"
                   "四处唯一锚点）⇒ 有牙筛必须只把工件那两条判成无牙，别的一律照旧",
                   img33 is not None and hits33 == ["1", "1", "1", "1"]
                   and t33 == ["FAIL 工件", "FAIL 引用找不到工件"]
                   and len(sc33) == OUTLET_BASELINE,
                   "锚点命中 %s｜修前无牙站点 %s｜修前档位 %s（期望 reset 2／inc 9／ret 2）" % (
                       "/".join(hits33), t33, sorted(tt33.items()))))
    MUT33B = [('                    if guard:\n                        bad += 1\n',
               '                    if guard:\n                        pass\n')]
    img33b, hits33b = m33_pairs(MUT33B)
    sc33b = FL.red_accum_screen(img33b, ("FAIL ", "INFO ")) if img33b else []
    t33b = sorted(set(h for h, _f, _l, k, _n in sc33b if k in ("reset", "blind")))
    checks.append(("变异体33·乙 撤掉 `guard = audit_len_guard(...)` 之后那条 `bad += 1`（返值通道断开）"
                   "⇒ 块外前段／后段两条必须立刻落进无牙名单（ret 档的牙就长在这一行上）",
                   img33b is not None and hits33b == ["1"]
                   and t33b == ["FAIL 块外前段被改动（第", "FAIL 块外后段被改动（前"],
                   "锚点命中 %s｜无牙站点 %s" % ("/".join(hits33b), t33b)))
    fails = 0
    for name, ok, detail in checks:
        print("%-4s %s  %s" % ("PASS" if ok else "FAIL", name, detail))
        if not ok:
            fails += 1
    print("SELFTEST 总 %d 例，FAIL %d 例" % (len(checks), fails))
    return 1 if fails else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="API 表面普查器（P1a，无判决）")
    ap.add_argument("--census", action="store_true", help="打印普查账（默认）")
    ap.add_argument("--role-matrix", action="store_true",
                    help="按 HostRole 臂归属的对外面账（只读，不判决；归属是注解不是门禁）")
    ap.add_argument("--role-surface", action="store_true",
                    help="按角色分桶的对外表面账：分支位置归因＋钉死取值走树两条通道（只读，不判决）")
    ap.add_argument("--selftest", action="store_true", help="解析器自检（含变异体）")
    ap.add_argument("--ledger", action="store_true",
                    help="台账闭合：方案文档里每条核心公式在一份活图像上复算，并把每个数对回被引用的工件键")
    ap.add_argument("--write", action="store_true",
                    help="与 --ledger 同用：把文档里的托管块改写为现渲染（块外逐行核验通过才落盘）")
    ap.add_argument("--check", action="store_true", help="判决模式（当前故意拒绝）")
    ap.add_argument("--show", type=int, default=12, help="每类 UNRESOLVED 点名条数")
    ap.add_argument("--show-lists", type=int, default=0, help="双向差额各点名条数，0 为不打印")
    ap.add_argument("--json", help="把账写成 JSON 到此路径（相对仓根）")
    args = ap.parse_args(argv)
    if args.selftest:
        return cmd_selftest(args)
    if args.role_matrix:
        return cmd_role_matrix(args)
    if args.role_surface:
        return cmd_role_surface(args)
    if args.ledger:
        return cmd_ledger(args)
    if args.check:
        print("REFUSED：--check 仍不启用，但拒绝的理由已经换了一茬。")
        print("         覆盖面按形状算：兜底 fallback、条件装配 if/else、match 臂、以及不叫 build_*")
        print("         只靠签名才在册的装配节点都已归因，静默盲区已收干（现量看 --census 的 UNRESOLVED）。")
        print("         剩下的差额不是仪器读不出，而是 ROUTES 表本身没登记它们；数值不写在这里，")
        print("         挂在 reports/data/ 下那份带 generated_at 的普查工件上。")
        print("         把它现在钉成门禁，等于用一份已知漏登的账去裁决装配侧，红的是台账不是代码。")
        print("         启用条件见 docs/architecture/API-SURFACE-AUTHORITY-PLAN-v0.1.md §2 的 P1 裁决点。")
        return 2
    return cmd_census(args)


if __name__ == "__main__":
    sys.exit(main())
