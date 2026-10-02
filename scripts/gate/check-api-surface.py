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
ARM_PAT_RE = re.compile(r"([A-Za-z_]\w*(?:::[A-Za-z_]\w*)+)\s*(?:\{[^{}]*\})?\s*=>")
ROLE_TAG = "role:"
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
        if CHAIN_STEP_RE.search(expr):
            self.walk_text(expr, base, origin + " [inline]", depth, rel, offset, env, role)
            return
        seen = []
        for _, n in callee_calls(expr):
            if n in self.fns and n not in seen:
                seen.append(n)
        for name in seen:
            self.resolve_fn_ref(name, base, origin, depth, role)
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

    def match_arms(self, text, offset, rel):
        """把 `match x { Enum::A => …, Enum::B | Enum::C => …, _ => … }` 翻成 [(臂起, 臂止, 标签)]。

        标签存**完整枚举路径**（`HostRole::Kg`），因为同一份装配里可能有第二把 match
        （按 tier、按特性开关），只留末段会让两个维度撞名。`_ =>` 无枚举路径 ⇒ 进 role_wild，
        它仍然能"继承外层臂"（臂起点没在册，位置就不落进任何臂区间）。
        """
        out = []
        for m in MATCH_RE.finditer(text):
            ob = text.find("{", m.end() - 1)
            body = span_from(text, ob) if ob >= 0 else None
            if not body:
                continue
            lo, hi = body
            for a_lo, a_hi in self._top_commas(text, lo, hi):
                seg = text[a_lo:a_hi]
                arrow = self._top_arrow(seg)
                pats = ARM_PAT_RE.findall(seg[:arrow + 2] if arrow is not None else seg)
                if pats:
                    label = "+".join(sorted(set(pats)))
                    out.append((a_lo, a_hi, label))
                    self.role_arms.add((rel, offset + a_lo, label))
                elif seg.strip():
                    self.role_wild.add((rel, offset + a_lo))
        return out

    @staticmethod
    def arms_at(arms, pos):
        """位置 pos 落在哪些臂里（外层在前）：嵌套 match 的两维都要带上，丢掉外层就等于把内层读成唯一维。"""
        enclosing = sorted([(lo, hi, lab) for lo, hi, lab in arms if lo <= pos < hi])
        return "+".join(lab for _lo, _hi, lab in enclosing) if enclosing else None

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
        env, free = self.split_stats(text, offset)
        arms = self.match_arms(text, offset, rel)
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
                    step_role = self.arm_label(role, self.arms_at(arms, seg_start + pos - offset))
                    self.apply_step(kind, args, base, at, env, depth, rel, seg_start + pos, step_role)
                continue
            # 头标识符后紧跟 :: 的是模块路径（actuator::build_actuator_router()），
            # 不是同名的 let 变量；把它当变量回查 env 会让 let 自己套自己无限递归。
            if recv_in_env:
                self.walk_text(env[recv.group(0)][1], base, origin, depth + 1, rel, env[recv.group(0)][0], env, role)
                continue
            self.resolve_expr(stripped, base, at, depth, env, rel, seg_start, role)

    def apply_step(self, kind, args, base, at, env, depth, rel, pos, role=None):
        # 角色只进 origin 串，不进 mounted 的键 ⇒ 加这一维不会改变在册集合本身
        tag = (" " + ROLE_TAG + role) if role else ""
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
    m = re.search(r"struct ApiRoute\b[^{]*\{(.*)\}", src, re.S)
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
    """有归属的路径里，无归属读数是否来自**同一个挂载位点**。

    同一个 site 出现两种读数＝这个位点被两条走树路径各到过一次，其中一条不带臂
    （All 走模块注册表那条就是典型）。它说明"角色是路径的属性，不是位点的属性"，
    因此不能把"每条有归属路径都有无归属孪生"直接读成"没有端点专属某个角色"。"""
    same, diff = set(), set()
    for p in agg["labeled"]:
        lab_sites, unlab_sites = set(), set()
        for _raw, _ms, origins in mounted_paths[p]:
            for o in origins:
                site = o.split(" ")[0]
                (unlab_sites if ROLE_TAG not in o else lab_sites).add(site)
        (same if lab_sites & unlab_sites else diff).add(p)
    return same, diff


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
    print("有归属路径的无归属孪生按位点分档：同一 site 两种读数 %d 条 ／ 不同 site %d 条"
          % (len(twins[0]), len(twins[1])))
    print("    同 site ＝ 该位点被两条走树路径各到过一次（一条带臂、一条不带），"
          "读成「角色是路径的属性」；不同 site 才要去看是不是第二处装配。")
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
    print("与 ROUTES 表交叉：挂载未在册的 %d 条里带角色归属 %d 条"
          % (len(set(mounted_paths) - table_paths),
             len([p for p in set(mounted_paths) - table_paths if p in agg["labeled"]])))
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
        ("同 site 与不同 site 两档必须覆盖全部有归属路径（分解不许留余）",
         twins[0] | twins[1] == agg["labeled"] and not (twins[0] & twins[1]),
         "同 %d ＋ 不同 %d 配 有归属 %d，交集 %d" % (
             len(twins[0]), len(twins[1]), len(agg["labeled"]), len(twins[0] & twins[1]))),
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
            "api_route_fields": all_fields,
            "api_route_role_fields": role_fields,
            "labeled_paths": len(agg["labeled"]),
            "unlabeled_paths": len(agg["unlabeled"]),
            "role_only_paths": sorted(agg["role_only"]),
            "same_site_twin_count": len(twins[0]),
            "different_site_twin_paths": sorted(twins[1]),
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
    # 同一棵 kb 子树还被一条**不带臂**的路径走了一次 ⇒ 同一个挂载位点出现两种读数。
    # 这条自由表达式就是真语料里 All 走模块注册表、单角色走 domain_router 的那个形状。
    build_kb_router();
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


def run_fixture(src_text, root_fns):
    cen = Census()
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


def patched_ns(old, new):
    """代码级变异体：把判据本身换成弱化版再 exec 出一份新模块。

    语料级变异体改的是被扫的样例，改不了判据；"只跟第一支""只按名字在册"这类
    退化只能由代码变异体自己打红，否则夹具绿只证明夹具的形状被照顾到了。"""
    assert JUDGE_SRC.count(old) == 1, "变异锚点在判据侧命中 %d 次：%s" % (JUDGE_SRC.count(old), old[:60])
    ns = {"__file__": os.path.abspath(__file__), "__name__": "mutant"}
    exec(compile(CODE.replace(old, new, 1), "<mutant>", "exec"), ns)
    return ns


def run_in(ns, src_text, root_fns):
    cen = ns["Census"]()
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
    ns1 = patched_ns("            if n in self.fns and n not in seen:\n                seen.append(n)",
                     "            if n in self.fns:\n                seen.append(n)\n                break")
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
    same_r, diff_r = site_twins(role_aggregates(cr, mpr), mpr)
    checks.append(("同一挂载位点的两种读数要按「路径的属性」归到有标签那一侧",
                   same_r == {"/api/kbdoc"} and not diff_r
                   and cr.role_matrix().get("/api/kbdoc") == {"HostRole::Kb"},
                   "同 site %s 不同 site %s kbdoc %s" % (
                       sorted(same_r), sorted(diff_r), sorted(cr.role_matrix().get("/api/kbdoc", set())))))
    nsr7 = patched_ns("        (same if lab_sites & unlab_sites else diff).add(p)", "        diff.add(p)")
    gr7, cr7 = run_in(nsr7, ROLE_FIXTURE, ["build_host_router"])
    mp7 = nsr7["mounted_by_path"](cr7)
    same7, diff7 = nsr7["site_twins"](nsr7["role_aggregates"](cr7, mp7), mp7)
    checks.append(("变异体15 撤同 site 判据 ⇒ 同档清空而挂载一条没少（分账读的是这条判据）",
                   same7 == set() and diff7 == {"/api/kbdoc"} and {p for p, _ in gr7} == EXPECT_ROLE_PATHS,
                   "同 %s 不同 %s 路径 %d 条" % (sorted(same7), sorted(diff7), len(gr7))))
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
    ap.add_argument("--selftest", action="store_true", help="解析器自检（含变异体）")
    ap.add_argument("--check", action="store_true", help="判决模式（当前故意拒绝）")
    ap.add_argument("--show", type=int, default=12, help="每类 UNRESOLVED 点名条数")
    ap.add_argument("--show-lists", type=int, default=0, help="双向差额各点名条数，0 为不打印")
    ap.add_argument("--json", help="把账写成 JSON 到此路径（相对仓根）")
    args = ap.parse_args(argv)
    if args.selftest:
        return cmd_selftest(args)
    if args.role_matrix:
        return cmd_role_matrix(args)
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
