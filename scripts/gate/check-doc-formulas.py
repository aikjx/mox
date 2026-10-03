# -*- coding: utf-8 -*-
"""全 `docs/` 核心公式普查：散文里写出的每一条分解式加法都要被复算一次。

为什么要这台仪器：`docs/architecture/API-SURFACE-AUTHORITY-PLAN-v0.1.md` §1.7 把**那一份文件**的
公式收成了单一算源台账（`--ledger`），但"某某 ＝ 某某 ＋ 某某"这类账在全库还有几十处，
它们既没有算源、也没有尺子读过。识别与复算的逻辑在 `scripts/gate/formula_ledger.py`——
**"什么是一条算术主张"只在那里判一次**，本文件只管扫描集、判决与牙齿。

模式：
  --census [--json <路径>]   只读数、可写工件，不改任何被扫文件（违反判据 ⇒ rc=1）
                             同时现量扫描集**之外**那一圈（docs/ 以外的 .md 用同一把尺复算）
  --dircheck                 扫描集自身的牙齿（进来的合不合格＋该进来的有没有进来＋被挡掉的有没有出账＋集外边界现量）
  --dir-account              目录账那半条：目录账＋集外宇宙登记账＋让出面账（不读字形表 ⇒ 不待裁决点 6，可单独接线）
  --recognizer               仪器侧的账：算术尺只许有一个归宿文件（§1.8b）
  --selftest                 夹具＋变异体（期望 FAIL 0）
默认（无模式）               同上 --census，只是不写工件

判据的形状（2026-10-02 由实测归纳，勿凭手感改，理由逐条写在 `formula_ledger.py` 文档串里）：
  主张（claim）    必须闭合。不闭合＝红，按 `文件:行` 点名并打印各侧取值。
  盲区（blind）    粘连／拉丁标签／ASCII 等号／增量记号／跨代码段／非等式符号／无全角等号
                   ⇒ 不是主张，但**按 reason 点名出账**。
  引文（quoted）   `「…」`／`“…”` 里的是"某轮曾印错成什么"，不复算，按名点名。
盲区不是缺陷，把盲区读成"没有缺陷"才是缺陷——所以每种盲区各自有编号的夹具与变异体。
"""

from __future__ import annotations

import argparse
import ast
import glob
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time

# Windows 控制台默认 GBK，而判决行里带 ⇒／＝ 这类字符：不设这道守卫，崩的是仪器自己而不是被测面
# （rc=1 会被读成"有红"，实为 print 抛 UnicodeEncodeError）。仓内另外五份门禁已有同款守卫，照同一写法。
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import formula_ledger as FL  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ROOTS = ["docs"]
# 目录出账：每条都要有名字与理由，理由不许空、不许短于 8 字。
# §1.8n 起条目名**不只是文字**：最后一段（去掉 `**` 段）就是它声称排除的目录名，由 prune_dirs()
# 派生给两条读路用；首段若是目录则是它声明的顶层桶（`**/` 开头＝全局）。名字与实现各写一遍＝两个源，
# 从前 `_archive` 在 walk_md／outside_walk／dircheck_verdict 里各抄了一次，而另两条出账条目
# 声称排除却一份也没排（实测读集里仍有 5 份 _verification 与 1 份 _data）。
DIR_ACCOUNT = [
    ("docs/_archive", "归档文档不是活账，其中的旧错值不做复算对象，且体量会把分母冲淡"),
    ("**/_verification", "验证产物由生成器写出，属派生副本，其权威在源文档；复算两遍＝两个源"),
    ("docs/**/_data", "报告数据目录，多为机器生成与第三方截图说明，登记但只出 INFO，不判红"),
]
LEDGER_BEGIN = "<!-- LEDGER:BEGIN"
LEDGER_END = "<!-- LEDGER:END -->"


def entry_dirname(name):
    """条目名最后一段（跳过 `**` 段）＝它声称排除的目录名。剪枝集只从目录账派生。"""
    segs = [s for s in name.split("/") if s and s != "**"]
    return segs[-1] if segs else name


def entry_scope(name):
    """条目名首段声明的顶层桶；`**/` 开头或只有一段者声明全局（None＝不比范围）。"""
    parts = name.split("/")
    if parts[0] == "**" or len(parts) == 1:
        return None
    return parts[0]


def prune_dirs(account=None):
    """两条读路共用的剪枝集：由 DIR_ACCOUNT 的条目名现推，不另立名单（名单＝第二个源）。"""
    acc = list(DIR_ACCOUNT if account is None else account)
    return frozenset(entry_dirname(n) for n, _ in acc)


def walk_md(roots, prune=None):
    pr = prune_dirs() if prune is None else prune
    out = []
    for r in roots:
        base = os.path.join(REPO, r)
        for dirpath, dirnames, filenames in os.walk(base):
            rel = os.path.relpath(dirpath, REPO).replace("\\", "/")
            parts = rel.split("/")
            if any(x in pr for x in parts):
                dirnames[:] = []
                continue
            dirnames[:] = [d for d in dirnames
                           if not d.startswith(".") and d not in pr]
            for f in filenames:
                if f.endswith(".md"):
                    out.append(os.path.join(dirpath, f))
    return sorted(out)


def block_span(text):
    """文档里仪器托管的表格块（`--ledger --write` 写的）不参与散文复算。"""
    lines = text.split("\n")
    b = e = None
    for i, ln in enumerate(lines):
        if ln.startswith(LEDGER_BEGIN):
            b = i + 1
        elif LEDGER_END in ln and b is not None:
            e = i + 1
            break
    return (b, e) if (b and e) else None


def _rel(path, repo=REPO):
    """仓内路径按仓根相对化；跨盘（Windows 上 tempfile 在 C: 而仓在 D:）relpath 会抛 ValueError，
    此时按原路径点名——探针不许因为「夹具放在了另一个盘」而崩溃成没有判决。"""
    try:
        return os.path.relpath(path, repo).replace("\\", "/")
    except ValueError:
        return path.replace("\\", "/")


def _scan_file(path):
    with io.open(path, encoding="utf-8", errors="replace") as fh:
        text = fh.read()
    rel = _rel(path)
    return rel, FL.scan_text(text, blank_lines=block_span(text))


def _scan_paths(paths):
    agg = dict(files=len(paths), claims=0, violations=0, blind=0, quoted=0,
               blind_counts={}, per_doc={}, details=[], claim_files=[])
    for p in paths:
        rel, sp = _scan_file(p)
        bc = FL.blind_counts(sp["blind"])
        agg["claims"] += len(sp["claims"])
        agg["violations"] += len(sp["violations"])
        agg["blind"] += len(sp["blind"])
        agg["quoted"] += len(sp["quoted"])
        for k, v in bc.items():
            agg["blind_counts"][k] = agg["blind_counts"].get(k, 0) + v
        if any(sp[k] for k in ("claims", "blind", "quoted")):
            agg["per_doc"][rel] = dict(claims=len(sp["claims"]),
                                       violations=len(sp["violations"]),
                                       blind=bc, quoted=len(sp["quoted"]))
        if sp["claims"]:
            agg["claim_files"].append((rel, len(sp["claims"])))
        for v in sp["violations"]:
            agg["details"].append(dict(path=rel, line=v["line"], span=v["span"], vals=v["vals"]))
    return agg


def census(roots):
    return _scan_paths(walk_md(roots))


# ─────────────────── 扫描集的边界（集外那一圈是量出来的，不是声明的） ───────────────────
# ROOTS 只有 docs，而「所有核心公式」这句话的覆盖面取决于**集外有没有等式**：10-02 现量证实集外
# 确有（reports/markdown/ 里 1 条）。所以集外不是「不看」，是「用同一把尺复算、只判它自不闭合」——
# 集外没有台账，等式的每一项无从指认权威，硬要复算＝给派生面凭空造第二个源。
# 排除目录按名点名并印出跳过了多少个：`walk_md` 原先靠「目录名以 . 开头」顺手跳过了 pnpm 的
# node_modules/.pnpm（约七百份第三方 README），那是**巧合不是口径**，此处的名单才是。
OUTSIDE_SKIP = frozenset(["docs", "target", "data", "log", "workspace", "ais", "third_party",
                          "node_modules", "dist", "build", "coverage"])
OUTSIDE_NESTED = frozenset(["node_modules", "dist", "build", "target", "coverage",
                             "ais", "third_party", "screenshots"])


def outside_top(repo=REPO):
    dirs, mds = [], []
    for e in sorted(os.listdir(repo)):
        if e.startswith(".") or e in OUTSIDE_SKIP:
            continue
        if os.path.isdir(os.path.join(repo, e)):
            dirs.append(e)
        elif e.endswith(".md"):
            mds.append(e)
    return dirs, mds


def outside_walk(repo, dirs, root_files, nested=None, prune=None):
    """集外侧的读路：剪枝集与 walk_md 同源（都取自 DIR_ACCOUNT 的条目名），
       再叠一份只针对代码族的 OUTSIDE_NESTED——那一圈在跟踪面里一份 .md 也没有，由 夹具W 现印。
       嵌套名单当参数喂进来（§1.8p 第十本账的逐名反事实要能撤掉一名再走一遍），不改全局＝仪器不留痕迹。"""
    pr = prune_dirs() if prune is None else prune
    ne = OUTSIDE_NESTED if nested is None else nested
    got, skip_dirs = [], 0
    for d in dirs:
        for dirpath, dirnames, filenames in os.walk(os.path.join(repo, d)):
            rel = os.path.relpath(dirpath, repo).replace("\\", "/")
            parts = rel.split("/")
            if any(x in pr for x in parts):
                dirnames[:] = []
                continue
            drop = [n for n in dirnames
                    if n.startswith(".") or n in ne or n in pr]
            skip_dirs += len(drop)
            dirnames[:] = [n for n in dirnames if n not in drop]
            got.extend(os.path.join(dirpath, f) for f in filenames if f.endswith(".md"))
    got.extend(os.path.join(repo, f) for f in root_files)
    return sorted(got), skip_dirs


# ─────────────────── 集外宇宙登记账（"该进来的有没有进来"） ───────────────────
# 目录账从前只判"进来的目录合不合格"，而 ROOTS 只有 docs 一根——"所有核心公式"这句话的覆盖面
# 取决于没被登记的那一圈有多大，此前没有任何仪器知道这个数是几（§1.8k 末段登记的盲侧）。
# 10-02 现量：集外 11 个桶装着 310 份 .md，桶＝顶层目录＋仓根，口径就是集外账那把
# outside_top／outside_walk，不另起第三种切法。第二口径（git 索引）逐桶对照后只有三处差：
# my-projects 读到 0 份＝整体未入库，projects／reports 的差是未跟踪的导出物与报告。
# 形状取"双向登记账"而不是"装满 N 份才判"：阈值要人猜＝第二个源，而"每个装活文档的桶都必须有
# 一句理由"不要。撤掉登记＝漏登记红，登记了而桶空了＝失效条目红（按规矩删条目，不是留理由）。
# 这条通道只用路径形状（os.walk 给的目录名与文件名），不调 `_scan_paths` ⇒ 不读字形表，
# 因此它跟着目录账那半条一起可单独接线，不等裁决点 6。
ROOT_BUCKET = "(仓根)"
OUTSIDE_ACCOUNT = [
    ("(仓根)", "README／AGENTS 等仓根文档由 CI 声称账与脚本路径账按名接管，再复算＝两个源"),
    ("deploy", "部署交付面的运维文档自成一套索引，不参与仓内等式，纳入只会稀释分母"),
    ("examples", "示例目录一份 README，权威在示例代码本身"),
    ("frontend-ui", "前端模块 README 与源码同目录，权威在代码与前端治理规范"),
    ("my-projects", "子项目工作区语料属本地运行态且整体未入库，不是仓内活账"),
    ("platform", "逐 crate 的迁移与评估说明，权威在 Rust 源码与各 crate 目录本身"),
    ("projects", "各子项目自带手册与导出物，文档权威在子项目内部，跨项目无从同一台账"),
    ("reports", "报告是派生面，等式已由集外账用同一把尺复算且只判自不闭合，再判＝凭空造第二个源"),
    ("scripts", "门禁与工具脚本的一份 README，权威在各脚本头注释与治理文档"),
    ("tests", "回归用例的 README 与验证报告属运行态产物，权威在对应用例与日志"),
    ("tools", "工具自带 README／SKILL 与脚本同目录，权威在脚本自身"),
]


def outside_buckets(repo=REPO):
    """集外宇宙按顶层粒度现量：返回 [(桶名, .md 份数)]，只走路径，不读字形。"""
    dirs, root_files = outside_top(repo)
    paths, _skip = outside_walk(repo, dirs, root_files)
    counts = {}
    for p in paths:
        bucket = top_bucket(os.path.relpath(p, repo).replace("\\", "/"))
        counts[bucket] = counts.get(bucket, 0) + 1
    return sorted(counts.items())


def universe_verdict(buckets, account=None):
    """双向对账：桶未登记＝漏登记；条目在本轮宇宙里读不到＝失效；理由不许空、不许短于 8 字。
       全称判据在空宇宙上恒真，所以"齐备"这句话必须由调用方的塌缩红来兜（第 35 型）。"""
    acc = list(OUTSIDE_ACCOUNT if account is None else account)
    seen = dict(buckets)
    named = set(n for n, _ in acc)
    bad = []
    for name, why in acc:
        if name not in seen:
            bad.append("集外出账条目已失效：%s（本轮在集外宇宙里读到 0 份 .md——目录已消失，"
                       "或它被加进了 ROOTS／跳过名单，那种情况要删条目而不是留理由）" % name)
        elif reason_bad(why):
            bad.append("集外出账理由不合格：%s" % name)
    for name in sorted(seen):
        if name not in named:
            bad.append("集外宇宙漏登记：%s（%d 份 .md 在扫描集外，既没进 ROOTS 也没出账）"
                       % (name, seen[name]))
    return bad


# ─────────────────── 让出面账（被顶层规则挡掉的那一圈自己也要复算） ───────────────────
# §1.8l 印出的盲侧④：`outside_top` 用 OUTSIDE_SKIP ∪ 点前缀把 19 个顶层目录挡在宇宙外，
# 而"被跳过"这件事过去没有任何仪器复核过——名单与形状定义了覆盖面，覆盖面就没有下界。
# 10-02 现量：被挡掉的桶里确实装着 git 跟踪的 .md（ais 1／log 1／.trae 1／.workbuddy 4，合计 7 份），
# 其中两个住在**没有名字的点前缀规则**后面。docs 那 415 份在 ROOTS 内，属"已接管"不属"已让出"。
# 形状沿用第五本账的双向登记，但域只取"真让出过跟踪文档"的桶：没让出东西的桶不许有条目，
# 于是账不会烂成一张名字清单；让出了却没说＝红。阈值一个都不设，桶数／份数／条目数每轮现量。
# 跟踪面只走路径与 `git ls-files` 清单，仍不读字形表 ⇒ 接线账一条命令管住三本账。
GIVEN_UP_ACCOUNT = [
    ("ais", "第三方参考树的一份 README，虽被 git 跟踪，权威仍在第三方源码本身"),
    ("log", "运行态日志目录的一份 README，属运行时产物而非仓内活文档"),
    (".trae", "IDE 工作区数据下的实施计划草稿，作者与归宿都在工具侧，不参与仓内等式"),
    (".workbuddy", "代理记忆暂存面的 Markdown，属会话运行态导出物，非仓内活账"),
]


def tracked_md(repo=REPO):
    """跟踪面读取：本文件唯一的 git 调用出口。返回 (路径表, None) 或 (None, 原因)。
       失败绝不返回空表——空表会让"漏登记"在真实缺口上恒绿，把"没读到"印成"没有"（第 35 型）。"""
    try:
        r = subprocess.run(["git", "ls-files", "-z"], cwd=repo, capture_output=True)
    except (OSError, ValueError) as exc:
        return None, "git 调用失败 %s" % type(exc).__name__
    if r.returncode != 0:
        return None, "git ls-files rc=%d" % r.returncode
    return sorted(x for x in r.stdout.decode("utf-8", "replace").split("\0")
                  if x.endswith(".md")), None


def top_bucket(rel):
    """顶层粒度：路径第一段，仓根文件归 (仓根)。宇宙账与让出面账共用这一把切法。"""
    return rel.split("/")[0] if "/" in rel else ROOT_BUCKET


def given_up_partition(md_paths, universe_names, roots=None):
    """让出面现量：跟踪 .md 里既不在 ROOTS、顶层桶又不在集外宇宙名单的那些，按桶归堆。"""
    rt = set(ROOTS if roots is None else roots)
    univ = set(universe_names)
    seen = {}
    for rel in md_paths:
        b = top_bucket(rel)
        if b in rt or b in univ:
            continue
        seen.setdefault(b, []).append(rel)
    return seen


def given_up_verdict(seen, account=None):
    """双向对账三条红：让出了却没出账／条目在本轮让出面里读到 0／理由不合格。
       空让出面上的全称判据恒真，所以"齐备"必须靠调用方的塌缩红兜（第 35 型）。"""
    acc = list(GIVEN_UP_ACCOUNT if account is None else account)
    named = set(n for n, _ in acc)
    bad = []
    for name, why in acc:
        if name not in seen:
            bad.append("让出面条目已失效：%s（本轮让出面里没有它的跟踪 .md——要么它归了 ROOTS／宇宙，"
                       "要么目录已消失，那种情况要删条目而不是留理由）" % name)
        elif reason_bad(why):
            bad.append("让出面理由不合格：%s" % name)
    for name in sorted(seen):
        if name not in named:
            bad.append("让出面漏登记：%s（%d 份 git 跟踪的 .md 被顶层规则挡在宇宙外，既没进 ROOTS 也没出账）"
                       % (name, len(seen[name])))
    return bad


def outside_census(repo=REPO):
    dirs, root_files = outside_top(repo)
    paths, skip_dirs = outside_walk(repo, dirs, root_files)
    agg = _scan_paths(paths)
    agg.update(dirs=len(dirs), root_files=len(root_files), skip_dirs=skip_dirs)
    return agg


def outside_verdict(agg):
    bad = []
    if agg["files"] == 0:
        bad.append("集外探针读到 0 份 .md（分母塌了，不是集外干净）")
    for v in agg["details"]:
        bad.append("集外主张不闭合 %s:%d 「%s」各侧取值 %s"
                   % (v["path"], v["line"], v["span"], v["vals"]))
    return bad


def _outside_head(agg):
    return ("扫描集外 .md %d 个（顶层目录 %d 个＋仓根 %d 份，按名跳过 %d 个第三方/构建目录）"
            "｜主张 %d 条（不闭合 %d）｜盲区 %d 条｜引文 %d 条"
            % (agg["files"], agg["dirs"], agg["root_files"], agg["skip_dirs"],
               agg["claims"], agg["violations"], agg["blind"], agg["quoted"]))


def _outside_json(agg):
    return {k: agg[k] for k in ("files", "dirs", "root_files", "skip_dirs", "claims",
                                "violations", "blind", "quoted", "claim_files")}



def _head(agg):
    return ("扫描 .md %d 个｜主张 %d 条（不闭合 %d）｜盲区 %d 条｜引文 %d 条"
            % (agg["files"], agg["claims"], agg["violations"], agg["blind"], agg["quoted"]))


def cmd_census(args):
    agg = census(args.root)
    out = outside_census()
    print(_head(agg))
    print("盲区分档 " + "／".join("%s=%d" % (k, agg["blind_counts"][k])
                                 for k in sorted(agg["blind_counts"])))
    # 主张按文件点名：谁持有"甲 ＝ 乙 ＋ 丙"这类账，决定了单源化下一站该搬哪份文档
    for rel in sorted(agg["per_doc"]):
        d = agg["per_doc"][rel]
        if d["claims"]:
            print("主张 %s：%d 条（不闭合 %d）" % (rel, d["claims"], d["violations"]))
    for v in agg["details"]:
        print("FAIL 主张不闭合 %s:%d 「%s」各侧取值 %s" % (v["path"], v["line"], v["span"], v["vals"]))
    # 集外边界自署：这句话的覆盖面是量出来的，不是"我们只管 docs/"
    print(_outside_head(out))
    for rel, n in out["claim_files"]:
        print("集外主张 %s：%d 条" % (rel, n))
    for b in outside_verdict(out):
        print("FAIL %s" % b)
    if args.json:
        art = dict(generated_at=time.strftime("%Y-%m-%d %H:%M:%S%z"),
                   scan_roots=list(args.root),
                   recognizer="scripts/gate/formula_ledger.py",
                   **{k: agg[k] for k in ("files", "claims", "violations", "blind", "quoted")},
                   blind_reasons=agg["blind_counts"],
                   per_doc=agg["per_doc"], unclosed=agg["details"],
                   outside=_outside_json(out))
        with io.open(args.json, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(json.dumps(art, ensure_ascii=False, indent=1))
        print("JSON 已写出 %s" % args.json)
    return 1 if (agg["violations"] or outside_verdict(out)) else 0


REASON_MIN = 8  # 目录账／集外账／让出面账共用一把理由尺：各写一遍 `< 8` 就是三条尺


def reason_bad(why):
    """空理由与短理由同罪。抽成函数前这两条判据在两个口径里各写了一遍。"""
    return (not why) or len(why) < REASON_MIN


def dircheck_verdict(dirs, account=None):
    """目录账的两条判决：抽成函数之前它们是写在 `cmd_dircheck` 里、自检碰不到的红。
    一条判决若在内存里造不出来，就没有任何针能保证它会在真语料上造得出来。
    §1.8n：第二条从前只认字面量 `_archive`（walk_md／outside_walk／这里各抄了一遍），
    现在认的是条目名派生出的那批目录名——一条出账声称排除了谁，扫描集里就不许再出现谁。"""
    bad = []
    acc = list(DIR_ACCOUNT if account is None else account)
    for name, why in acc:
        if reason_bad(why):
            bad.append("理由不合格：%s" % name)
    want = {}
    for name, _ in acc:
        want.setdefault(entry_dirname(name), []).append(name)
    hits = {}
    for d in dirs:
        for n in want.get(d.split("/")[-1], []):
            hits[n] = hits.get(n, 0) + 1
    for n in sorted(hits):
        bad.append("出账目录出现在扫描集里：%s（目录账声称排除它，扫描集仍扫进 %d 个目录——"
                   "这条理由今天只是文字，不是剪枝）" % (n, hits[n]))
    return bad


# 针的等式**不许手写**——源码里出现 `6 ＝ 1 ＋ 4 ＋ 1` 这种整串，就是第二把尺（夹具K 会拿它当别家命中）。
# 所以等式从唯一算源的两个常量现拼：形状照样是等式，源码里却看不到记号。
_NEEDLE_CLOSED = "6 %s 1 %s 4 %s 1" % (FL.EQ_FW, FL.ADD_FW, FL.ADD_FW)
_NEEDLE_UNCLOSED = "9 %s 1 %s 4 %s 1" % (FL.EQ_FW, FL.ADD_FW, FL.ADD_FW)


def _suspect_key(rel, b):
    """嫌疑的身份＝「哪份文档、第几行、前 40 字」。读账给判决时按这个键认人，不许按条数认。"""
    return "%s:%d:%s" % (rel, b.get("line"), (b.get("span") or "")[:40])


def blind_read_corpus_verdict(total, files):
    """第九本账的分母兜底（第 35 型）：全集读到盲区 0 条时，「盲区读账干净」是空集上的全称判据，
    不构成判决。这一格**只红一次**——按文档红会把每份本就没有盲区项的干净文档都打成红，
    那不是兜底，是把分母当成了红。"""
    bad = []
    if total:
        return bad
    bad.append("盲区读账分母塌缩：全集 %d 份 .md 读到盲区 0 条（0 条时「读账干净」不构成判决——"
               "先查扫描集是否为空，别读成仓里没有未判项）" % len(files))
    return bad


def _unblind_variants(span):
    """只摘遮蔽物（加粗／反引号／引号／ASCII 等号），**不动数字、不动算符、不下判决**——
    动数字或自己写判据就是第二把尺；摘完之后仍由唯一算源 `formula_ledger.scan_text` 判，
    所以这一本账没有第二源，只是把同一把尺举到遮蔽物外面去。"""
    s = span
    for ch in ("*", "`", '"', "“", "”", "「", "」", "『", "』"):
        s = s.replace(ch, "")
    return [span, s, s.replace("=", FL.EQ_FW)]


def blind_read_verdict(rel, blind, adjudicated=None):
    """第九本账（盲区读账）：盲区不是缺陷，把盲区读成「没有缺陷」才是缺陷。
    所以这一本只做三件事：
      ① 分母塌缩兜底（第 35 型：盲区 0 条时「读账干净」不构成判决）；
      ② 把「长得像等式」（带 ＝ 且带算符）的盲区项逐条点名，并给一个判决——
         解除遮蔽后交回唯一算源再判一次，判得出主张就是真算术，判不出就是非算术；
      ③ 真算术而不闭合 ⇒ 那是盲区里藏着的错账，红。
    给不出判决的嫌疑不许静默（未点名＝没读），也不许拿「盲区 N 条」当成读过。"""
    adj = {} if adjudicated is None else dict(adjudicated)
    bad = []
    for b in blind:
        span = b.get("span") or ""
        if not FL.EQ_FW_RE.search(span) or not (FL.ADD_RE.search(span) or FL.MUL_RE.search(span)):
            continue  # 结构项按 reason 进分档账，不进这一本
        key = _suspect_key(rel, b)
        verdict = adj.get(key)
        if verdict is None:
            bad.append("盲区嫌疑未点名：%s（%s 这条盲区项带 ＝ 且带算符，读账却没给它判决——"
                       "不许把嫌疑汇总成一个数）" % (span[:60], rel))
            continue
        if verdict != "真算术":
            continue
        hit = None
        for v in _unblind_variants(span):
            r = FL.scan_text(v)
            if r["claims"]:
                hit = r
                break
        if hit is not None and hit["violations"]:
            bad.append("盲区嫌疑未闭合：%s（%s 解除遮蔽后仍是算术主张且不闭合，各侧取值 %s——"
                       "盲区里藏着的错账）" % (span[:60], rel, hit["violations"][0]["vals"]))
    return bad


def exclusion_verdict(md_paths, dirs, account=None):
    """第七本账（出账执行账）：一条出账条目要同时满足两件事，各一条红，外加一格塌缩自证。
       ① 它真的排掉了东西（跟踪面里 0 命中＝这条目今天什么都不排，要删条目而不是留理由）；
       ② 它声明的范围与实际命中面相符（名字写 docs 而命中落在别家＝条目名里的范围也是一句没复算的判决）；
       ③ 跟踪面读到 0 份时"每条都在执行"不构成判决（第 35 型）。
       输入全部显式（跟踪名单＋扫描目录集＋可替换的账），所以两个口径共用这一份实现；
       只看路径形状，一个字节不读字形表 ⇒ 与目录账那半条同属可单独接线的通道。"""
    acc = list(DIR_ACCOUNT if account is None else account)
    bad = []
    if not md_paths:
        bad.append("出账执行账塌缩：git 跟踪的 .md 读到 0 份（读到 0 时『每条出账都在执行』"
                   "不构成判决——先查 git 是否可用，别读成排除全都生效）")
        return bad
    for name, _ in acc:
        d = entry_dirname(name)
        hits = [r for r in md_paths if d in r.split("/")]
        if not hits:
            bad.append("出账条目已失效：%s（本轮 git 跟踪的 .md 里 0 份命中目录名 %s——"
                       "它没排掉任何东西，要删条目而不是留理由）" % (name, d))
            continue
        scope = entry_scope(name)
        beyond = sorted(set(top_bucket(r) for r in hits) - set([scope])) if scope else []
        if beyond:
            bad.append("出账范围不符：%s 声明顶层桶 %s，而命中落在 %s"
                       "（条目名里的范围也是一句判决，不许比实现宽也不许比实现窄）"
                       % (name, scope, "／".join(beyond)))
    return bad


def dir_account_audit(dirs, n_files, account=None, universe=None, uaccount=None,
                      gmd=None, gaccount=None, gerr=None):
    """目录账口径的三件套：逐条判决＋分母行＋"分母塌了"这条独立红。
       分母断言是这条口径自己的半边牙齿（第 35 型：扫描集塌缩时 `dircheck_verdict` 的两条判据
       都在空集上恒真，会把"什么都没扫到"打印成"目录账全过"）。
       这条口径一个字节也不读 `formula_ledger` 的字形表 ⇒ 不待裁决点 6，可以单独接线；
       从前它与集外账捆在 --dircheck 里，于是要接就只能整块等字形裁决。
       §1.8l 长出第五本账（集外宇宙登记账），§1.8m 长出第六本账（让出面账）：
       进来的合不合格、该进来的有没有进来、被挡掉的有没有出账，同属"扫描集边界"且都不读字形。
       git 不可用时让出面整格 withheld 并把原因印进分母行——withheld 不是红，但绝不许被读成齐备，
       所以它自己有一枚与"确有红"配对的针（变异体31·①）。"""
    acc = list(DIR_ACCOUNT if account is None else account)
    univ = outside_buckets() if universe is None else list(universe)
    uacc = list(OUTSIDE_ACCOUNT if uaccount is None else uaccount)
    gacc = list(GIVEN_UP_ACCOUNT if gaccount is None else gaccount)
    real_read = gmd is None and gerr is None
    if real_read:
        gmd, gerr = tracked_md()
    gseen = None if gerr else given_up_partition(gmd, [n for n, _ in univ])
    bad = list(dircheck_verdict(dirs, acc)) + universe_verdict(univ, uacc)
    if gseen is None:
        g_head = "让出面账 withheld（%s）：这一本今天没判，别读成让出面干净" % gerr
    else:
        bad += given_up_verdict(gseen, gacc)
        # 第七本账（§1.8n 出账执行账）要的是 git 那份跟踪名单，合成格喂的是让出面专用的假名单，
        # 拿它判"每条出账都排掉了东西"会把假名单判成缺陷——所以这一本只在 git 真读时参与，
        # 而"合成格没参与"这件事由 夹具W 的见证行印出来，不写成静默跳过。
        if real_read:
            bad += exclusion_verdict(gmd, dirs, acc)
        g_head = ("让出桶 %d 个／跟踪让出 %d 份／出账 %d 条"
                  % (len(gseen), sum(len(v) for v in gseen.values()), len(gacc)))
    n_out = sum(n for _, n in univ)
    shared = sorted(set(ROOTS) & set(n for n, _ in univ))
    if shared:
        bad.append("扫描集与集外宇宙桶名重叠：%s（同一份文档被两本账各算一次＝两个源；"
                   "OUTSIDE_SKIP 里 docs 那一项的作用今天第一次被量出来）" % "／".join(shared))
    if not dirs or n_files == 0:
        bad.append("目录账分母塌缩：扫描目录 %d 个／文件 %d 个（扫描集为空时『目录账 %d 条』不构成判决）"
                   % (len(dirs), n_files, len(acc)))
    if not univ:
        bad.append("集外宇宙塌缩：顶层桶 %d 个／集外 .md %d 份（读到 0 时『登记齐备』不构成判决——"
                   "一根集外目录也没读到，先查 outside_top 的名单与仓根，别把它读成集外干净）"
                   % (len(univ), n_out))
    if gseen is not None and not gmd:
        bad.append("让出面宇宙塌缩：git 跟踪的 .md 读到 0 份（读到 0 时『让出面齐备』不构成判决——"
                   "先查 git 是否可用，别读成仓里没有让出的文档）")
    return bad, ("扫描目录 %d 个｜目录账 %d 条｜分母现量 文件 %d 个"
                  "｜集外桶 %d 个／.md %d 份／出账 %d 条｜%s"
                  % (len(dirs), len(acc), n_files, len(univ), n_out, len(uacc), g_head))


def outside_emit(agg):
    """集外账的出口（那半条要读字形，整块待裁决点 6）。行格式只在这里写一次。"""
    lines = [_outside_head(agg)]
    for rel, n in agg["claim_files"]:
        lines.append("     集外主张 %s：%d 条（集外无台账，只判自不闭合）" % (rel, n))
    return lines, outside_verdict(agg)


def dircheck_emit(dirs, n_files, out=None, account_only=False, universe=None, uaccount=None,
                  gmd=None, gaccount=None, gerr=None):
    """一个实现两个口径：account_only=True 只出目录账那半条（含第五、第六本账），False 追加集外账。
       分母行与 FAIL 行格式都只有一份——各写各的，接哪条就只验哪本账。"""
    bad, head = dir_account_audit(dirs, n_files, universe=universe, uaccount=uaccount,
                                  gmd=gmd, gaccount=gaccount, gerr=gerr)
    lines = [head]
    if not account_only:
        out_lines, out_bad = outside_emit(out)
        lines += out_lines
        bad += out_bad
    for b in bad:
        lines.append("FAIL 扫描集 %s" % b)
    return lines, bad


_GLYPH_PROBE = "在册 12 ＝ 6 ＋ 6，另有 ascii 口径 12 = 6 + 6"


def _planted_glyph_verdict():
    """植入的『依赖字形的判决』：真的读全局 FL 的两把等号尺，专给下面的反事实咬它。
       它不是本门禁的判据，是一枚示形探针——中性化若连它都不改变，那条『目录账不变』就是空转读出来的假话。"""
    return sorted(["fw=%d" % len(FL.EQ_FW_RE.findall(_GLYPH_PROBE)),
                   "hw=%d" % len(FL.EQ_HW_RE.findall(_GLYPH_PROBE))])


def _neutral_ledger():
    """词汇表的中性化替身：两把等号尺都换成永不开火的模式（＝裁决点 6 里『ASCII 连底迁全角』那条路的
       极端情形——半角尺从此认不出任何等号）。只在内存里换 `globals()["FL"]`，一个字节也不写盘。"""
    class _N(object):
        EQ_FW_RE = re.compile(r"(?!)")
        EQ_HW_RE = re.compile(r"(?!)")
    return _N()


def channel_glyph_invariance(dirs, account, neutralize=_neutral_ledger):
    """给『这条判决待哪个裁决点』这种打包主张定价：把词汇表中性化，两件事必须同时成立——
       ① `dircheck_verdict` 的输出逐字节不变（目录账这半条通道不读字形，
          所以『--census／--dircheck／--recognizer 三条整块待裁决点 6』是过度主张，得按通道点名）；
       ② 那枚真读 FL 的植入判决的输出必须变（不变＝中性化在空转，①就成了恒真的废判据）。
       返回 (目录账是否不变, 植入判决是否变, 中性化前后四份读数)。"""
    orig = globals()["FL"]
    try:
        base_dir = dircheck_verdict(dirs, account)
        base_plant = _planted_glyph_verdict()
        globals()["FL"] = neutralize()
        alt_dir = dircheck_verdict(dirs, account)
        alt_plant = _planted_glyph_verdict()
    finally:
        globals()["FL"] = orig
    return base_dir == alt_dir, base_plant != alt_plant, (base_dir, alt_dir, base_plant, alt_plant)


# ─────────────────────── CI 接线账（"接了 CI" 的宇宙取自 CI，不取自声明） ───────────────────────
# 「这台仪器有针」由 --selftest 判；「这根针每天有人踩」是另一件事，此前只写在散文里＝声明不是测量，
# 与 §1.8c 那圈集外账同病。判据不许自写第二把尺：**"什么是一条真会被敲下去的 run 命令"是
# check-script-paths.py 的既有判据**（靠缩进出/入块、shell 注释必须剥掉，两处都是承重的），
# 这里只调它，另补一枚"行尾反斜杠续行"的拼接——CI 里参数可能落在下一物理行，不拼就会把合法写法判成缺陷
# （假阳的代价是人绕过闸门）。名单只两台、且都只吃 --selftest：把 --census/--dircheck/--recognizer
# 接进 CI 要先裁裁决点 6 的字形口径，接错了这条账也会当场点名"接线未带 --selftest"。
CI_WORKFLOW = ".github/workflows/ci.yml"
CI_SELFTEST_ARG = "--selftest"
CI_WIRED = ["scripts/gate/check-doc-formulas.py", "scripts/gate/check-api-surface.py"]


def _csp_instrument():
    """按路径装载兄弟门禁（模块名带连字符，只能走 importlib）。装载失败必须报出来，
    不许退化成"读不到命令 ⇒ 名单为空 ⇒ 没有红"。"""
    import importlib.util
    p = os.path.join(os.path.dirname(os.path.abspath(__file__)), "check-script-paths.py")
    spec = importlib.util.spec_from_file_location("mox_check_script_paths", p)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def _cmd_text_at(lines, lineno):
    """取 lineno 这条命令的完整文本：shell 的行尾 `\\` 是续行，参数可能落在下一物理行。"""
    i = lineno - 1
    parts = []
    while 0 <= i < len(lines):
        raw = lines[i].rstrip()
        cont = raw.endswith("\\")
        parts.append(raw[:-1] if cont else raw)
        if not cont:
            break
        i += 1
    return " ".join(parts)


CI_PRINTER_CMDS = ("echo", "printf")


def ci_exec_tokens(tokens, lines, printer=CI_PRINTER_CMDS):
    """把复用尺给的宇宙**收窄**到"这条命令真的在执行那份脚本"。
    复用来的 `ci_command_tokens` 按空白切词、只认脚本后缀，它分不出 `python x.py` 与
    `echo "python x.py"`——后一种此刻就在本仓 ci.yml 里（三条 `BLOCKED` 行把未接线的门禁打进日志，
    于是 `check-view-hex.py` 这类"其实没在跑"的脚本会出现在执行宇宙里）。接线账若直接吃原始宇宙，
    **一行 echo 就能把『已接线』糊出来**，而"判成已接线"恰是这台仪器唯一不许犯的错。
    收窄只看**命令首词**（不是"整串里有没有 echo"）：`cd x && python y` 这类复合命令首词是 `cd`，照旧算执行；
    被撤下的 token 一律按名交回调用方印出来——悄悄少数的宇宙与没数过同形。"""
    kept, dropped = [], []
    for lineno, word in tokens:
        head = _cmd_text_at(lines, lineno).lstrip().split(" ")[0].lower()
        if head in printer:
            dropped.append((lineno, word, head))
        else:
            kept.append((lineno, word))
    return kept, dropped


def ci_wiring_verdict(tokens, lines, want=CI_WIRED, arg=CI_SELFTEST_ARG, printer=CI_PRINTER_CMDS):
    """三条红，逐条走 `bad.append`——这样 夹具L 会自动向每条新红要一枚内存对照，
    今天这条判决有针，明天加的判决也跑不掉。"""
    kept, dropped = ci_exec_tokens(tokens, lines, printer)
    bad = []
    if not kept:
        bad.append("CI 扫描集塌缩：可执行脚本 0 条（原始 token %d 条，其中 %d 条住在打印语句里，"
                   "全塌或全被 echo 吞掉都判红）" % (len(tokens), len(dropped)))
        return bad
    by_path = {}
    for lineno, word in kept:
        by_path.setdefault(word, []).append(lineno)
    for path in want:
        got = by_path.get(path, [])
        if not got:
            bad.append("CI 未接线 %s：名单里的仪器没出现在任何一条 run 命令里" % path)
        elif not any(arg in _cmd_text_at(lines, ln) for ln in got):
            bad.append("CI 接线未带 %s %s：接进去的是别的模式（或参数写丢了）" % (arg, path))
    return bad


# ─────────────────── 文档声称的"CI 门禁"也要对账（名单取自文档，不是手抄） ───────────────────
# `CI_WIRED` 是**我选的名单**，它只能证明我列的那两台；而 `AGENTS.md` 里每句"CI 门禁"都是维护者读到的口径声明。
# 这一格把名单的来源换成文档自己：同一行既写着标记、又挂着脚本路径 ⇒ 一条声称，必须在某份 workflow 的真执行 token 里出现。
# 宇宙仍复用兄弟门禁的 `scan_tokens`/`ci_command_tokens` 与本文件的 `ci_exec_tokens`（echo 不算），不自写第三种切词。
WORKFLOW_DIR = ".github/workflows"
WORKFLOW_SUFFIX = (".yml", ".yaml")
AGENTS_DOC = "AGENTS.md"
CI_CLAIM_MARK = "CI 门禁"


def ci_workflow_texts(repo=REPO):
    """所有 workflow 文件的 (相对路径, 文本) 并集。
       边界必须自称：某份 workflow 到底会不会被触发**不在这格的口径里**（那是触发器那本账的事），
       所以这里的"已接线"是"有人写过它"的下界，不是"每天真跑"的上界。"""
    d = os.path.join(repo, WORKFLOW_DIR)
    out = []
    if os.path.isdir(d):
        for name in sorted(os.listdir(d)):
            if name.endswith(WORKFLOW_SUFFIX):
                out.append(("%s/%s" % (WORKFLOW_DIR, name),
                            _read_code(os.path.join(d, name))))
    return out


def doc_ci_claims(lines, tokens, suffixes, mark=CI_CLAIM_MARK):
    """把文档声称收成账。返回 (claims, 无脚本路径可核的标记行号, 被后缀口径挡下的 token)。
       后缀口径是**复用来的**（`check-script-paths.py` 的 `SCRIPT_SUFFIX`，不含 `.mjs`），
       所以挂在同一行的 `.md`/`.mjs` 会落进 `non_script` 并按名印出来——挡下的必须可见，否则就是静默少数。"""
    by_line = {}
    for lineno, tok in tokens:
        by_line.setdefault(lineno, []).append(tok)
    claims, no_path, non_script = [], [], []
    for lineno, line in enumerate(lines, 1):
        if mark not in line:
            continue
        hit = [t for t in by_line.get(lineno, []) if t.endswith(tuple(suffixes))]
        non_script += [(lineno, t) for t in by_line.get(lineno, []) if not t.endswith(tuple(suffixes))]
        if hit:
            claims += [(lineno, t) for t in hit]
        else:
            no_path.append(lineno)
    return claims, no_path, non_script


def ci_claim_verdict(claims, exec_paths, no_path, non_script):
    """两条红：声称账塌缩／声称未接线。偏差方向与 §1.8e 同——只许读成"没接线"（响亮地红），
    不许读成"已接"（静默地绿）。没有路径可挂的声称（`cargo clippy` 那一类）不判红，但必须按行号印出来，
    否则"这一格全绿"会被读成"所有声称都核实了"，而它对那种形状根本是盲的。"""
    bad = []
    if not claims:
        bad.append("CI 声称账塌缩：挂着脚本路径的「%s」行 0 条（标记命中却无路径 %d 行／后缀挡下 %d 处），"
                   "分母塌了必须自己成一条红" % (CI_CLAIM_MARK, len(no_path), len(non_script)))
    for lineno, path in claims:
        if path not in exec_paths:
            bad.append("CI 声称未接线 %s：文档写着标记却没有一条 run 命令执行它（文档行 %d）"
                       % (path, lineno))
    return bad


GATE_DIR_MARK = "/gate/"


def gate_scripts_unlisted(exec_paths, agents_text):
    """反向账（夹具Q）：CI 每次真在跑、且住在 `*/gate/` 目录下的脚本，必须在维护指南里点得名。
    正向账（夹具O）问"文档声称的有没有真跑"，本格问"真跑的有没有写进文档"——两侧都要有牙，
    只钉一侧时另一侧的漂移是静默的：指南少一格，新来的人（和每个 AI 代理）就照着少一格的口径跑，
    而那条门禁本身照样天天绿。
    "住在 gate 目录"是按目录名取的口径，边界要印出来：不在该目录下的执行对象（`scripts/doc/*`、
    `tools/*`、`verify_axioms.py` 那类）本格不管，只数它们一份、印进细节里，免得读者把 10 读成 15。"""
    gate = sorted(p for p in exec_paths if GATE_DIR_MARK in p)
    bad = []
    if not gate:
        bad.append("反向账塌缩：CI 执行宇宙里没有一份住在 %s 目录下的脚本（分母塌了，不是门禁齐备）"
                   % GATE_DIR_MARK)
    for p in gate:
        if p not in agents_text:
            bad.append("门禁脚本未登记在维护指南 %s：CI 天天执行它，AGENTS.md 里却点不到" % p)
    return bad, gate


ONE_CLICK_DOC = "scripts/gate/check-all.ps1"
ONE_CLICK_TOTAL = "$Total"
ONE_CLICK_LABEL = re.compile(r"\[(\d+)/([^\]\s]+)\]")
ONE_CLICK_TOTAL_DEF = re.compile(r"\$Total\s*=\s*(\d+)")
ONE_CLICK_ITEMS = re.compile(r"（(\d+) 项")


def one_click_audit(gate_scripts, ps1_text, agents_text):
    """第三本账（夹具R）：`AGENTS.md` 宣传的『一键质量检查』必须与 CI 真跑的 gate 集合同账，
    而且它的**项数只许有一个算源**。为什么把两件事钉在同一格里：那份 .ps1 原先把分母写成字面量
    `[i/7]` 共 11 处——加一步就得记着改 11 个地方，漏改的那几个不是报错而是安静地印错数；
    这正是本文件反复在收的"同一个事实有多个出口"，而它的后果恰好就是"本地一键全绿、CI 却红"。
    匹配按**整条带引号的仓内路径**取（不按 basename：basename 会撞同名，那是另一类假阳）。"""
    bad = []
    quoted = dict((p, ('"%s"' % p) in ps1_text) for p in gate_scripts)
    invoked = sorted(p for p, ok in quoted.items() if ok)
    if gate_scripts and not invoked:
        bad.append("一键账塌缩：CI 的 gate 宇宙有 %d 份，而「一键质量检查」里一条都没执行（分母塌了，不是覆盖齐全）"
                   % len(gate_scripts))
    for p in gate_scripts:
        if not quoted.get(p):
            bad.append("CI 门禁未进一键 %s：CI 每次真跑它，本地一键却敲不到（本地全绿可以藏着一条 CI 红）" % p)
    labels = ONE_CLICK_LABEL.findall(ps1_text)
    dens = sorted({d for _i, d in labels})
    defs = ONE_CLICK_TOTAL_DEF.findall(ps1_text)
    total = None
    if not labels:
        bad.append("一键分母账塌缩：%s 里解析不到任何 [i/N] 段标签（脚本换了形状，分母从此不可核）" % ONE_CLICK_DOC)
    elif dens != [ONE_CLICK_TOTAL]:
        bad.append("一键分母不合一：段标签的分母是 %s，必须是唯一一个变量 %s（同一个数写 N 次＝N 个第二源）"
                   % (dens, ONE_CLICK_TOTAL))
    elif len(defs) != 1:
        bad.append("一键分母不合一：变量 %s 的定义应有且只有 1 处，实得 %d 处" % (ONE_CLICK_TOTAL, len(defs)))
    else:
        total = int(defs[0])
        steps = sorted({int(i) for i, _d in labels})
        if steps != list(range(1, total + 1)):
            bad.append("一键分母不合一：%s＝%d 而段标签的分子是 %s（有一步没打标签、或标签写错号）"
                       % (ONE_CLICK_TOTAL, total, steps))
    claim = None
    for line in agents_text.split("\n"):
        if ONE_CLICK_DOC in line:
            mm = ONE_CLICK_ITEMS.search(line)
            if mm:
                claim = int(mm.group(1))
    if total is not None and claim is not None and claim != total:
        bad.append("一键项数与文档不符：%s 写着 %d 项而脚本 %s＝%d" % (AGENTS_DOC, claim, ONE_CLICK_TOTAL, total))
    return bad, invoked, total, claim


CMD_SEPARATORS = frozenset(("&&", "||", ";", "|"))


def _fmt_args(args):
    return " ".join(args) if args else "(裸跑，无参数)"


def _arg_segments(words, path):
    """同一条路径在一行里可能被敲两次（`python a.py --selftest && python a.py`）：每次出现出一段参数，
       段止于下一个分隔词。把行尾整串都当成参数会造出不存在的模式，而假造的参数会让本该红的
       『不合一』变绿——那一格就成了它自己要防的东西。"""
    out = []
    for i, w in enumerate(words):
        if w != path:
            continue
        tail = []
        for x in words[i + 1:]:
            if x in CMD_SEPARATORS:
                break
            tail.append(x)
        out.append(tuple(tail))
    return out


def ci_gate_invocations(csp, rel, wtext):
    """把 CI 的执行宇宙从『路径』升到『(路径, 参数组)』。
       名单仍完全来自复用尺：`ci_command_tokens` 给 run 块里的脚本路径，`ci_exec_tokens` 按命令首词
       撤掉 echo/printf（一行 echo 骗不出参数账，与 夹具M 同一道闸）。本格只在路径之后的文本上做派生，
       不另起第三种切词——参数是第二个源的话，『同账』就又是散文了。"""
    out = []
    if not csp or not wtext:
        return out
    lines = wtext.split("\n")
    kept, _dropped = ci_exec_tokens(csp.ci_command_tokens(wtext), lines)
    seen = set()
    for lineno, path in kept:
        words = _cmd_text_at(lines, lineno).split()
        for args in _arg_segments(words, path):
            # 复用尺是按词给的：同一行里同一条路径出现两次会交来两个 token，而段是从整行取的，
            # 不去重就把同一组参数数两遍——组数是这格的总分母，虚高的分母与少数同罪。
            if (path, args, lineno) not in seen:
                seen.add((path, args, lineno))
                out.append((path, args, rel, lineno))
    return out


def one_click_argsets(ps1_text, path):
    """一键侧『以某组参数执行这条路径』的取法与 夹具R 保持同一个口径（认**双引号里的整条仓内路径**）：
       路径之后的词即参数，遇分隔词截断。每行只取第一次出现——那种一行敲两次的写法在本仓不存在，
       真出现时宁可判成『不合一』响亮地红。"""
    lit = '"%s"' % path
    out = []
    for line in ps1_text.split("\n"):
        i = line.find(lit)
        if i < 0:
            continue
        tail = []
        for x in line[i + len(lit):].split():
            if x in CMD_SEPARATORS:
                break
            tail.append(x)
        out.append(tuple(tail))
    return out


def one_click_arg_parity(ci_inv, oc_text, gate_scripts, invoked):
    """第四维（夹具S）：CI 与一键不但要跑**同一批** gate，还要**以同一组参数**跑。
    只比路径时，一键里的 `X --selftest`（只测仪器自己，永远不看仓库）可以顶掉 CI 里的
    `X --check`（判仓库），本地全绿而 CI 红——与 夹具R 防的是同一类缺陷，只是这次藏在参数里。
    未进一键的路径不在这里重复报（那是 夹具R 的红）：同一个缺陷出两条红时，改其一会让人以为两条都好了。"""
    bad = []
    want = {}
    for path, args, rel, lineno in ci_inv:
        if path in set(gate_scripts) and path in set(invoked):
            want.setdefault((path, args), []).append((rel, lineno))
    if gate_scripts and invoked and not want:
        bad.append("一键参数账塌缩：CI 执行的 gate 有 %d 份、一键收录 %d 份，却一条参数组都没解析出来"
                   "（分母塌了，不是参数同账）" % (len(set(gate_scripts)), len(invoked)))
    for (path, args) in sorted(want):
        sets = one_click_argsets(oc_text, path)
        if args not in sets:
            rel, lineno = want[(path, args)][0]
            bad.append("一键参数不合一 %s：CI 以「%s」执行它（%s 行 %d），一键里同路径只有 %s"
                       "（本地全绿藏着一条 CI 红：CI 真跑的那组参数在本地没人敲过）"
                       % (path, _fmt_args(args), rel, lineno,
                          " ／ ".join(_fmt_args(s) for s in sets) or "(一条都没有)"))
    return bad, want


def cmd_dircheck(args):
    """扫描集自身的牙齿：目录账逐条验，归档目录不许走进来，分母现量。
       一个命令体两个口径：--dircheck 全量（目录账＋集外账）／--dir-account 只目录账。"""
    files = walk_md(args.root)
    dirs = sorted({os.path.relpath(os.path.dirname(p), REPO).replace("\\", "/") for p in files})
    # 集外账要读字形（裁决点 6 未裁），目录账不读；只有真需要它时才扫那一圈。
    out = None if args.dir_account else outside_census()
    # 集外宇宙按路径现量一次就交给两个口径；各量一次＝同一本账两把尺，必然漂移。
    lines, bad = dircheck_emit(dirs, len(files), out, args.dir_account,
                               outside_buckets())
    for line in lines:
        print(line)
    return 1 if bad else 0


# ─────────────────────── 识别器单归宿账（对象是仪器，不是文档） ───────────────────────
# 「什么是一条算术主张」只许在 scripts/gate/formula_ledger.py 判一次。任何一份仪器在**正则位点**
# 上自带这套记号，就是第二把尺——同一本账两把尺必然漂移（本轮收掉的一处真账：
# check-api-surface.py 的 --ledger 曾自写 CHAIN_RE／QUOTE_SPAN_RE，比文档里的托管块多算 12 条）。
# 本尺的词汇取自 FL（它自己再抄一遍 ＝／＋ 就成了第三条尺），收窄与盲区逐格点名。
RECO_GLOBS = ("scripts/gate/*.py", "frontend-ui/scripts/gate/*.py", "tools/**/*.py")
# 非 Python 的仪器（前端门禁里有 .mjs）走一把**弱尺**：全文找记号，不分正则位点。
# 弱尺必须自称是弱尺——否则"命中 0"会被读成"那里没有第二把尺"，而真实意思是"我没往那儿看"。
RECO_GLOBS_OTHER = ("scripts/gate/*.mjs", "scripts/gate/*.js", "scripts/gate/*.ts",
                    "frontend-ui/scripts/gate/*.mjs", "frontend-ui/scripts/gate/*.js",
                    "frontend-ui/scripts/gate/*.ts")
RECO_HOME = "scripts/gate/formula_ledger.py"
RECO_FUNCS = frozenset(["compile", "search", "match", "fullmatch", "findall",
                        "finditer", "sub", "subn", "split"])
# `RX.search(x)` 这种「已编译模式」的调用里，模式在**接收者**身上而不是第一个参数：
# 早先一版把 arg0（`x`）当模式，于是 115 个位点全落盲区、归宿账照样"零第二把尺"——
# 那是针没穿通道，不是语料干净。
RECO_PMETHOD = RECO_FUNCS - frozenset(["compile"])


def _fold(expr, pats):
    """把 `"[" + 名字 + "]"` 这类串接折成完整字面量；折不出（f-string、函数返回值）就交 None 进盲区。
    不折这一层的话，归宿文件自己 `ADD_RE = re.compile("[" + ADD_FW + "]")` 就成了读不出的模式——
    把词汇收成一处命名反而让尺子变盲，这是本轮实测到的事（命中 12 → 1）。"""
    if isinstance(expr, ast.Constant):
        return expr.value if isinstance(expr.value, str) else None
    if isinstance(expr, ast.Name):
        return pats.get(expr.id)
    if isinstance(expr, ast.BinOp) and isinstance(expr.op, ast.Add):
        left, right = _fold(expr.left, pats), _fold(expr.right, pats)
        return None if left is None or right is None else left + right
    return None


def reco_pattern_src(tree):
    """名字 → 字面量模式串：`X = r'…'`、`X = re.compile(r'…')`，含串接，按依赖两轮解到不动点。"""
    pending = []
    for n in ast.walk(tree):
        if isinstance(n, ast.Assign):
            for tg in n.targets:
                if isinstance(tg, ast.Name):
                    pending.append((tg.id, n.value))
    pats = {}
    for _ in range(3):
        changed = False
        for name, v in pending:
            if name in pats:
                continue
            lit = None
            if isinstance(v, ast.Constant) and isinstance(v.value, str):
                lit = v.value
            elif (isinstance(v, ast.Call) and isinstance(v.func, ast.Attribute)
                  and isinstance(v.func.value, ast.Name) and v.func.value.id == "re"
                  and v.func.attr == "compile" and v.args):
                lit = _fold(v.args[0], pats)
            if lit is not None:
                pats[name] = lit
                changed = True
        if not changed:
            break
    return pats


def reco_sites(src):
    """[(行号, 模式串或 None, 通道名)]：解得出字面量的进判决集，解不出的进盲区。"""
    tree = ast.parse(src)
    pats = reco_pattern_src(tree)
    out = []
    for n in ast.walk(tree):
        if not isinstance(n, ast.Call) or not n.args:
            continue
        fn = n.func
        if not isinstance(fn, ast.Attribute) or fn.attr not in RECO_FUNCS:
            continue
        recv = fn.value
        if not isinstance(recv, ast.Name):
            continue
        if recv.id == "re":
            a = n.args[0]
            tag = "re-call:" + (a.id if isinstance(a, ast.Name) else type(a).__name__)
            out.append((n.lineno, _fold(a, pats), tag))
        elif fn.attr in RECO_PMETHOD:
            out.append((n.lineno, pats.get(recv.id), "method:" + recv.id))
    return out


def reco_is_arith(pat):
    """算术尺：正则里出现全角 ＝／＋ 即算。ASCII 那一半只认「转义加号＋等号＋数字」三者同现。"""
    if FL.EQ_FW in pat or FL.ADD_FW in pat:
        return True
    return ("\\+" in pat and FL.EQ_HW_RE.pattern in pat and "\\d" in pat)


def reco_is_eqshape(pat):
    """等式形但本尺不判：含等号与 \\d 的 key=value／计数尺（端口、SELFTEST PASS=n FAIL=m）。"""
    return (FL.EQ_FW in pat or FL.EQ_HW_RE.pattern in pat) and "\\d" in pat


def reco_items(repo=REPO):
    """扫描集由 glob 现量，不写文件名清单——清单会漏掉明天新加的仪器。"""
    out, seen = [], set()
    for g in RECO_GLOBS + RECO_GLOBS_OTHER:
        for p in sorted(glob.glob(os.path.join(repo, g.replace("/", os.sep)), recursive=True)):
            rel = os.path.relpath(p, repo).replace("\\", "/")
            if rel in seen:
                continue
            seen.add(rel)
            out.append((rel, _read_code(p)))
    return out


def _read_code(path):
    """按 utf-8-sig 读：本仓有带 BOM 的脚本（`scripts/ci/ci.py` 就是），而 BOM 留在串首会让
    `ast.parse` 报 "invalid character in identifier" ⇒ 那台仪器整份落进"解析失败"，
    它自带的第二把尺从此隐身。读码不许假定没有 BOM。"""
    with io.open(path, encoding="utf-8-sig", errors="replace") as fh:
        return fh.read()


# ───────────────── 识别器扫描集自己的边界（"其余 30 份仪器无一把自带"的宇宙是量出来的） ─────────────────
# `RECO_GLOBS` 只覆盖三处门禁目录，所以"别家 0 处"原本只在**在册的 31 份**上成立；
# 而仪器这个词的真实外延是"仓里所有写得动正则的代码"。本通道把同一套判据打在扫描集外的
# 候选仪器上，命中即判"第二把尺在集外"（点名到文件与行号），并印出自己跳过了哪些目录。
RECO_OUTSIDE_ROOTS = ("scripts", "tools", "frontend-ui/scripts", "platform", "domains", ".github")
RECO_GENERATED_NESTED = frozenset(["node_modules", "dist", "build", "target", "coverage",
                                 "__pycache__", "assets"])
# 从前这里手写了一份 `_verification`（10-02 22:15 普查 nested-skip-recon-2026-10-02-2215.txt：
# 目录账派生名与它恰好只重一名，就是它）。现在那份排除只由 DIR_ACCOUNT 条目名派生——
# 目录账撤条目时代码语料跟着放出来，不会留下一份静默的副本；合并对今天的语料零代价（候选集相同）。
RECO_OUTSIDE_NESTED = RECO_GENERATED_NESTED | prune_dirs()
RECO_OUTSIDE_EXT = (".py", ".mjs", ".js", ".ts")


def reco_outside_paths(repo=REPO, nested=None):
    ne = RECO_OUTSIDE_NESTED if nested is None else nested
    have = {rel for rel, _ in reco_items(repo)}
    cand, skip_dirs = set(), 0
    for r in RECO_OUTSIDE_ROOTS:
        base = os.path.join(repo, r)
        if not os.path.isdir(base):
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            drop = [d for d in dirnames if d.startswith(".") or d in ne]
            skip_dirs += len(drop)
            dirnames[:] = [d for d in dirnames if d not in drop]
            for f in filenames:
                if f.endswith(RECO_OUTSIDE_EXT):
                    cand.add(_rel(os.path.join(dirpath, f), repo))
    for e in sorted(os.listdir(repo)):
        p = os.path.join(repo, e)
        if os.path.isfile(p) and e.endswith(RECO_OUTSIDE_EXT):
            cand.add(e)
    return sorted(cand - have), skip_dirs


def reco_outside_audit(repo=REPO, items=None, skip_dirs=0):
    if items is None:
        paths, skip_dirs = reco_outside_paths(repo)
        items = [(rel, _read_code(os.path.join(repo, rel))) for rel in paths]
    rep = reco_audit([(RECO_HOME, _read_code(os.path.join(repo, RECO_HOME)))] + list(items),
                     home=RECO_HOME)
    bad = []
    if not items:
        bad.append("集外候选读到 0 份代码文件（分母塌了，不是集外干净）")
    for h in rep["foreign"]:
        bad.append("第二把尺在扫描集外 %s:%d 通道 %s" % (h["path"], h["line"], h["channel"]))
    rep["out_files"] = len(items)
    rep["skip_dirs"] = skip_dirs
    rep["bad"] = bad
    return rep


def reco_exclusion_verdict(rows, out_files, nested, generated):
    """第八本账（嵌套排除的代价）：名单每名都要有反事实读数，读出的代价要能点名隐身。

    三条塌缩红＋一条隐身红＋一条第二源红。全部只吃参数与 prune_dirs()，不碰磁盘，
    因此 夹具L 能从 bad 累加器里取到模板并由内存对照逐条打红。
    """
    bad = []
    dup = sorted(set(generated) & set(prune_dirs()))
    if dup:
        bad.append("代码名单重打了目录账的名字 %s（一个事实一个源：那份排除只能由 DIR_ACCOUNT 条目名派生）"
                   % "／".join(dup))
    if not nested:
        bad.append("嵌套排除名单塌缩（0 个名字＝没有边界声明，代价账无从谈起）")
    elif len(rows) != len(set(nested)):
        bad.append("排除代价账塌缩（名单 %d 名／反事实读数 %d 行，缺的那几名今天没有代价记录）"
                   % (len(set(nested)), len(rows)))
    if out_files < 1:
        bad.append("排除代价账分母塌缩（集外候选读到 0 份，此时逐名读数与隐身判据都在空集上恒真）")
    for name, exposed, hits, where, dirs in rows:
        if hits:
            bad.append("嵌套排除挡住了自带算术记号的候选 %s（放出 %d 份、命中 %d 处：%s）"
                       % (name, exposed, hits, where))
    return bad


def reco_exclusion_account(repo=REPO):
    """逐名反事实：撤掉名单里的一名再走一遍集外代码语料，量它挡住几份候选、其中有没有算术记号。

    不改全局（名单当参数喂进 reco_outside_paths），所以这台仪器不会留下被自己改写过的痕迹。
    """
    base, skip0 = reco_outside_paths(repo)
    base_set = set(base)
    nested = set(RECO_OUTSIDE_NESTED)
    rows = []
    for name in sorted(nested):
        paths, skip = reco_outside_paths(repo, frozenset(nested - {name}))
        exposed = sorted(set(paths) - base_set)
        hits, where = 0, ""
        if exposed:
            rep = reco_audit([(p, _read_code(os.path.join(repo, p))) for p in exposed])
            hits = len(rep["foreign"])
            where = "／".join("%s:%d" % (h["path"], h["line"]) for h in rep["foreign"][:3])
        rows.append((name, len(exposed), hits, where, skip0 - skip))
    return rows, len(base), skip0


def outside_exclusion_account(repo=REPO):
    """第十本账（§1.8p）：集外侧 .md 的嵌套名单逐名反事实——撤掉一名再走一遍集外语料，
       量它新放出几份 .md、少走几个目录。与 第八本账 同一形状，所以两本账印在同一份输出里
       （跨账恒等式要两侧同刻现量，不许一侧是本轮读数、另一侧是回忆）。

       名单当参数喂进 outside_walk ⇒ 不改全局，仪器不在盘上留痕迹。少走目录是两次计数的差，
       撤掉一名可能让**新进入的子树**里去数别的跳过名，差可以是负的——符号不是判据，逐名读数才是。
       每行第五格是「撤掉这名后放出的路径里落在 docs/ 下的条数」：集外侧的剪枝名单本不该伸进扫描集，
       一旦伸进去，这份账报的就不再是「集外多读了几个文件」而是「集内被挡了几个文件」，两者不同价。
    """
    dirs, root_files = outside_top(repo)
    base, skip0 = outside_walk(repo, dirs, root_files)
    base_set = set(base)
    nested = set(OUTSIDE_NESTED)
    rel = lambda p: os.path.relpath(p, repo).replace(os.sep, "/")
    rows = []
    for name in sorted(nested):
        paths, skip = outside_walk(repo, dirs, root_files, frozenset(nested - {name}))
        exposed = sorted(rel(p) for p in set(paths) - base_set)
        rows.append((name, len(exposed), skip0 - skip, "／".join(exposed[:2]),
                     sum(1 for p in exposed if p.startswith("docs/"))))
    return rows, len(base), skip0


def outside_exclusion_verdict(rows, out_md, nested, skip_names):
    """六条红：名单塌缩／逐名读数缺行／集外 .md 分母塌缩（此时逐名读数与恒等式都在空集上恒真）／
       嵌套名单重打了目录账的名字（第二源）／一名既无代价又不与顶层名单重叠（没有任何读数支持它留在集）／
       反事实放出的路径落进 docs/（那份名单越过了集内外边界，报的不再是集外代价而是集内缺口）。

       只吃参数与 prune_dirs()，不碰磁盘 ⇒ 夹具能在内存里逐条打红（承 第八本账 那条路子）。
    """
    bad = []
    sk = set(skip_names)
    if not nested:
        bad.append("集外 .md 嵌套名单塌缩（0 个名字＝没有边界声明，代价账无从谈起）")
    elif len(rows) != len(set(nested)):
        bad.append("集外 .md 排除代价账塌缩（名单 %d 名／反事实读数 %d 行，缺的那几名没有代价记录）"
                   % (len(set(nested)), len(rows)))
    if out_md < 1:
        bad.append("集外 .md 排除代价账分母塌缩（集外读到 0 份，此时逐名读数与恒等式在空集上恒真）")
    dup = sorted(set(nested) & set(prune_dirs()))
    if dup:
        bad.append("集外 .md 名单重打了目录账的名字 %s（一个事实一个源：那份排除只能由 DIR_ACCOUNT 条目名派生）"
                   % "／".join(dup))
    orphan = sorted(n for n, files, dirs, _s, _ud in rows if not files and not dirs and n not in sk)
    if orphan:
        bad.append("集外 .md 名单里有 %d 名既不放文件也不减目录、顶层名单也不含它 %s"
                   "（既不点火也不与任何在册口径共用＝没有读数支持它继续留在集）"
                   % (len(orphan), "／".join(orphan)))
    bleed = sorted(set(n for n, _f, _d, _s, ud in rows if ud))
    if bleed:
        bad.append("集外 .md 反事实放出的路径里有集内目录名下的 %d 名 %s"
                   "（集外侧的剪枝名单伸进了扫描集：本账报的从「集外多读几份」变成「集内被挡几份」，两件事不同价）"
                   % (len(bleed), "／".join(bleed)))
    return bad


# 六条通道一张表（前缀, 变异锚点, 内存对照的入参）：变异体34 用它逐条打红、夹具Z 用它逐条撤分支、
# BATTERY 用它当针。三份对照各写一遍＝改一条红要记得改三处，那正是「清单会漏掉明天新加的那条红」的自家版本。
OUT_EX_CASES = [
    ("集外 .md 嵌套名单塌缩（0 个名字＝没有边界声明，代价账无从谈起）",
     '    if not nested:\n        bad.append("集外 .md 嵌套名单塌缩',
     ([], 5, [], OUTSIDE_SKIP)),
    ("集外 .md 排除代价账塌缩（名单",
     '    elif len(rows) != len(set(nested)):\n        bad.append("集外 .md 排除代价账塌缩',
     ([("a", 1, 0, "x/y.md", 0)], 5, ["a", "b"], [])),
    ("集外 .md 排除代价账分母塌缩（集外读到 0 份，此时逐名读数与恒等式在空集上恒真）",
     '    if out_md < 1:\n        bad.append("集外 .md 排除代价账分母塌缩',
     ([("a", 1, 0, "x/y.md", 0)], 0, ["a"], [])),
    ("集外 .md 名单重打了目录账的名字",
     '    if dup:\n        bad.append("集外 .md 名单重打了目录账的名字',
     ([("_archive", 1, 0, "x/y.md", 0)], 5, ["_archive"], [])),
    ("集外 .md 名单里有",
     '    if orphan:\n        bad.append("集外 .md 名单里有',
     ([("ghost", 0, 0, "", 0)], 5, ["ghost"], [])),
    ("集外 .md 反事实放出的路径里有集内目录名下的",
     '    if bleed:\n        bad.append("集外 .md 反事实放出的路径里有集内目录名下的',
     ([("a", 3, 0, "docs/x.md", 3)], 5, ["a"], [])),
]


OUT_EX_HOME_FN = "outside_exclusion_verdict"


def _ex_witness(src=None):
    """夹具Z 的机器：把第十本账的判决函数从源码里现抠出来，逐条分支在内存里 `and False`，
       要求撤第 k 条只让第 k 枚针哑、其余五枚照红。

       默认读盘上那份（跑起来时盘＝运行图像，见证是对着真身跑的）；`src=` 是留给牙的——
       一个只会「读自己那份磁盘图像」的见证没法在不改仓内文件的前提下证明它会红，
       所以把图像当参数喂进来，撤分支／抠不到函数／锚点不唯一三种坏法都能只坏在内存里
       （坏法本身由 夹具AA 常驻复跑，不留在临时脚本里）。
       累加器故意不叫 bad：这些消息是「见证自己坏了」，不是判决通道，不该进红模板台账。
    """
    notes = []
    src = _read_code(os.path.abspath(__file__)) if src is None else src
    fns = [n for n in ast.parse(src).body if isinstance(n, ast.FunctionDef)
           and n.name == OUT_EX_HOME_FN]
    if len(fns) != 1:
        return ["判决函数在本文件源码里不是恰好 1 个（现量 %d）＝抠不出可变异的源码，见证作废"
                % len(fns)], "抠取失败"
    seg = ast.get_source_segment(src, fns[0]) or ""
    n_tpl = seg.count('bad.append("集外 .md ')
    if n_tpl != len(OUT_EX_CASES):
        notes.append("判决源码里的红模板现量 %d 条 ≠ 通道表 %d 条＝有条道没被见证（新增判决要同轮补通道）"
                     % (n_tpl, len(OUT_EX_CASES)))
    ns0 = {"prune_dirs": prune_dirs}
    exec(compile(seg, "<ex-base>", "exec"), ns0)
    base = {p: len(ns0["outside_exclusion_verdict"](*args)) for p, _a, args in OUT_EX_CASES}
    if sorted(base.values()) != [1] * len(OUT_EX_CASES):
        notes.append("未变异基线不是每枚各红 1 条 %s＝先修对照再来撤分支" % base)
        return notes, "基线 %s" % base
    per = []
    for i, (p, anch, _args) in enumerate(OUT_EX_CASES):
        if seg.count(anch) != 1:
            notes.append("锚点第 %d 条（%s）在判决源码里命中 %d 次（不唯一＝可能撤错分支或一条没撤）"
                         % (i + 1, p[:14], seg.count(anch)))
            continue
        mut = seg.replace(anch, anch.replace(":\n", " and False:\n", 1), 1)
        if mut == seg:
            notes.append("锚点第 %d 条撤不动（源码里没有 `:\\n` 那个形状＝分支写法变了）" % (i + 1))
            continue
        ns = {"prune_dirs": prune_dirs}
        exec(compile(mut, "<ex-mut-%d>" % i, "exec"), ns)
        got = {q: len(ns["outside_exclusion_verdict"](*ar)) for q, _a2, ar in OUT_EX_CASES}
        want = dict(base)
        want[p] = 0
        ok = got == want
        per.append((i + 1, "哑自己＋其余照红" if ok else got))
        if not ok:
            notes.append("撤掉第 %d 条（%s）后各针红数 %s ≠ 期望 %s" % (i + 1, p[:14], got, want))
    return notes, "红模板现量 %d／基线每枚 1 条／逐条撤分支 %s" % (n_tpl, per)


def reco_weak_sites(src):
    """弱尺（非 Python 仪器）：整行含全角记号即算，不区分它是不是正则位点。"""
    return [(i, ln.strip()[:64]) for i, ln in enumerate(src.splitlines(), 1)
            if FL.EQ_FW in ln or FL.ADD_FW in ln]


def reco_audit(items, home=RECO_HOME):
    hits, blind_dyn, eq_unflagged, unparsable = [], [], [], []
    judged = other_files = 0
    for path, src in items:
        if not path.endswith(".py"):
            other_files += 1
            for lineno, ln in reco_weak_sites(src):
                hits.append({"path": path, "line": lineno, "channel": "weak-text", "pattern": ln})
            continue
        try:
            sites = reco_sites(src)
        except SyntaxError as exc:
            unparsable.append((path, exc.lineno))
            continue
        for lineno, pat, how in sites:
            judged += 1
            if pat is None:
                blind_dyn.append((path, lineno, how))
            elif reco_is_arith(pat):
                hits.append({"path": path, "line": lineno, "channel": how, "pattern": pat[:64]})
            elif reco_is_eqshape(pat):
                eq_unflagged.append((path, lineno, how, pat[:48]))
    norm = lambda p: p.replace("\\", "/")
    return dict(files=len(items), judged=judged, other_files=other_files, hits=hits,
                foreign=[h for h in hits if norm(h["path"]) != home],
                home_count=sum(1 for h in hits if norm(h["path"]) == home),
                blind_dynamic=blind_dyn, eq_unflagged=eq_unflagged, unparsable=unparsable,
                home_seen=any(norm(p) == home for p, _ in items))


def reco_verdict(rep):
    """红四种：别家自带算术正则／归宿文件不在扫描集／判决集塌缩／有仪器解析失败。"""
    bad = []
    if rep["foreign"]:
        bad.append("第二把尺 %d 处" % len(rep["foreign"]))
    if not rep["home_seen"]:
        bad.append("归宿文件不在扫描集（此时 foreign 必为空，全称判据是空的）")
    if rep["judged"] < 1:
        bad.append("判决集塌缩（一个模式位点都没看见）")
    if rep["unparsable"]:
        bad.append("解析失败 %d 份，不许把读不出报成没有问题" % len(rep["unparsable"]))
    return bad


def cmd_recognizer(args):
    rep = reco_audit(reco_items())
    bad = reco_verdict(rep)
    out = reco_outside_audit()
    print("识别器单归宿：扫描仪器 %d 份（含归宿 1 份 ⇒ 判分母 %d 份；Python %d 份走 AST／其余 %d 份走弱尺：整行含全角记号即算）"
          "｜模式位点 %d 处（盲区 %d 处：模式在运行时构造或跨模块导入）"
          % (rep["files"], rep["files"] - 1, rep["files"] - rep["other_files"], rep["other_files"],
             rep["judged"], len(rep["blind_dynamic"])))
    print("    算术记号命中 %d 处：归宿文件 %d ／ 别家 %d" % (len(rep["hits"]), rep["home_count"], len(rep["foreign"])))
    for h in rep["foreign"][:10]:
        print("     第二把尺 %s:%d 通道 %s 「%s」" % (h["path"], h["line"], h["channel"], h["pattern"]))
    for p, ln in rep["unparsable"]:
        print("     解析失败 %s:%s" % (p, ln))
    print("    等式形但不判 %d 处（ASCII key=value／计数尺；本尺认全角记号，这是假阴方向，登记不隐藏）"
          % len(rep["eq_unflagged"]))
    for p, ln, how, pat in rep["eq_unflagged"][:4]:
        print("     出账 %s:%d %s 「%s」" % (p, ln, how, pat))
    # 扫描集自己的边界：上面那句"其余 N 份无一把自带"的宇宙是这三条 glob，集外那些写得动正则的代码
    # 此前没有任何读数——本行把它量出来，命中即判"第二把尺在集外"。
    print("    集外仪器候选 %d 份（根 %s／按名跳过 %d 个构建与派生目录）"
          "｜集外位点 %d 处、盲区 %d 处｜集外自带算术记号 %d 处"
          % (out["out_files"], "／".join(RECO_OUTSIDE_ROOTS), out["skip_dirs"],
             out["judged"], len(out["blind_dynamic"]), len(out["foreign"])))
    for h in out["foreign"][:10]:
        print("     第二把尺（集外）%s:%d 通道 %s 「%s」"
              % (h["path"], h["line"], h["channel"], h["pattern"]))
    for p, ln in out["unparsable"][:5]:
        print("     集外解析失败（出账，不判红：集外文件不是在册仪器）%s:%s" % (p, ln))
    rows, n_base, n_skip = reco_exclusion_account()
    ex_bad = reco_exclusion_verdict(rows, n_base, RECO_OUTSIDE_NESTED, RECO_GENERATED_NESTED)
    dead = sorted(r[0] for r in rows if not r[1] and not r[4])
    ign = ["%s:%d 份/%d 处命中" % (r[0], r[1], r[2]) for r in rows if r[1]]
    print("    嵌套排除代价账 %d 名（每名撤掉再走一遍）｜真挡住候选文件的 %s｜从不点火（既不放文件也不减目录）%s"
          % (len(rows), " ".join(ign) or "无", " ".join(dead) or "无"))
    print("    名单来源：构建派生字面量 %d 个 + 目录账派生名 %d 个（prune_dirs 现推，代码侧不再手写第二份）｜集外基线 %d 份｜少走目录 %d 个"
          % (len(RECO_GENERATED_NESTED), len(prune_dirs()), n_base, n_skip))
    # 第十本账（§1.8p）：集外侧 .md 那一圈自己的嵌套名单。第八本账量的是**代码**语料的排除代价，
    # 而 .md 语料用的是另一份名单（OUTSIDE_NESTED），此前只在普查里量过点火面、没进判决。
    _tc = time.perf_counter()
    o_rows, o_base, o_skip = outside_exclusion_account()
    o_cost = time.perf_counter() - _tc
    o_bad = outside_exclusion_verdict(o_rows, o_base, OUTSIDE_NESTED, OUTSIDE_SKIP)
    o_fire = ["%s：%d 份/目录 %+d" % (r[0], r[1], r[2]) for r in o_rows if r[1] or r[2]]
    o_dead = [r[0] for r in o_rows if not r[1] and not r[2]]
    o_bleed = sum(r[4] for r in o_rows)
    print("    集外 .md 嵌套排除代价账 %d 名（每名撤掉再走一遍，逐名反事实耗时 %.1f s）｜"
          "点火的 %s｜从不点火（防御项，按名出账，撤与不撤本账不判）%s｜放出里落在集内的 %d 条（本账只许算集外）"
          % (len(o_rows), o_cost, " ".join(o_fire) or "无", " ".join(o_dead) or "无", o_bleed))
    for r in o_rows:
        if r[3]:
            print("     放出样例 %s → %s" % (r[0], r[3]))
    print("    跨账恒等式（两本名单同一份输出里现量）：NESTED %d 名／SKIP %d 名／交集 %d 名（%s）｜"
          "只在 NESTED（%s）｜只在 SKIP（%s）｜集外 .md 基线 %d 份／少走目录 %d 个"
          % (len(OUTSIDE_NESTED), len(OUTSIDE_SKIP), len(set(OUTSIDE_NESTED) & set(OUTSIDE_SKIP)),
             " ".join(sorted(set(OUTSIDE_NESTED) & set(OUTSIDE_SKIP))) or "无",
             " ".join(sorted(set(OUTSIDE_NESTED) - set(OUTSIDE_SKIP))) or "无",
             " ".join(sorted(set(OUTSIDE_SKIP) - set(OUTSIDE_NESTED))) or "无",
             o_base, o_skip))
    bad = bad + out["bad"] + ex_bad + o_bad
    if bad:
        print("RECOGNIZER FAIL：%s" % "／".join(bad))
        return 1
    print("RECOGNIZER PASS（算术尺只在 %s 里定义，除归宿外在册 %d 份与集外候选 %d 份无一把自带；"
          "两本分母同一次现推、按构造不相交）"
          % (RECO_HOME, rep["files"] - 1, out["out_files"]))
    return 0


# ────────────────────────────── 夹具与变异体 ──────────────────────────────

# 真语料里"看起来像加法但不是主张"的七型，逐枚取自 2026-10-02 读原文的结案（假阳＝人绕过闸门）
LEGAL_GLUED = [
    "- 06-api-spec.md §2 自列 50 条（2.1=10 + 2.2=9 + 2.3=6 + 2.4=6 + 2.5=8 + 2.6=6 + 2.7=5 = 50）",
    "| 2 | 鲲鹏920+飞腾2000+/海光7285 | 国产化验证 |",
    "| T-集成-05 | 企业版神道 V2.0 = 激活扩散 A5 + 拓扑排序 A8 CPM | 命中数 ≥ 90% |",
    "> **与既有迭代对应**：M0 = 原「迭代 1+2」的归一化收口 + 文档化；M1 = 原「迭代 3」增强",
    "- **Deliverables**: 4 个 registry 文件；`GET /atlas/verify` W1=30+16=46 DOMAINS 全 PASS",
    "| E3=业务→L5+体道；E3=算法→L3+神道；E3=应用→L6+部署 | 映射表 |",
    "| TR15.4 | 节约 >70% = 2；40~70% = 1；<40% = 0。阈值 ≥ 1 |",
]
# 主张：必须判成 claim 且闭合（全角 ＝／＋ 配中文标签，是本仓写等式的记法）
LEGAL_CLAIMS = [
    "在册装配节点 72 ＝ 按名字 60 ＋ 只靠签名新增 12",
    "计算表 360 ＝ 36 × 2 × 1 ＋ 48 × 6 ＝ 72 ＋ 288",
    "归一挂载 375 ＝ 带分支标签 359 ＋ 公共面 16",
]
BROKEN_CLAIM = "归一挂载 374 ＝ 带分支标签 359 ＋ 公共面 16"
QUOTED_LINE = "旧值登记：「71 ＝ 60 ＋ 30」，右边相加其实是 90。"
# 每种盲区各一枚形状，reason 必须是它自己那一格（盲区不许挤成一格，否则等于没出账）
FIX_BLIND = [
    ("第 A1 ＝ 3 ＋ 4 项", "glued"),
    ("本 tasks.md 包含：Setup 1 + BatchA 12 + BatchB 11 + Gate 4 = 共 54 项任务", "latin"),
    ("在册装配节点 72 = 60 + 12", "ascii_eq"),
    ("本轮 946 ＝ +1 文件 ＋9 例", "increment"),
    ("装配 60 ＋ `（重叠 12 条）` ＋ 30 ＝ 90", "code"),
    ("净增 50 ＝ 甲 ≥ 20 ＋ 30", "noise"),
    ("总数 72 ＋ 60 ＋ 12", "no_fw_eq"),
]
# 反引号里写的是"公式长什么样"的示意，不许被读成这条散文的第二条活账
FIX_CODE_ONLY = "口径 `375 ＝ 359 ＋ 16` 只是示意，真账另见台账"
# 句子终止符专用的反例：撤掉终止符后两句会被接成一条链并误红
CUT_FIXTURE = "口径：甲 ＝ 60 ＋ 12；乙 ＝ 90 ＋ 8"


def _fix(reason):
    """按 reason 取夹具：位置索引会在增删盲区型时静默错位（变异体打到别人家的形状上）。"""
    return next(ln for ln, r in FIX_BLIND if r == reason)


def _kinds(text):
    sp = FL.scan_text(text)
    return sp, dict(claims=len(sp["claims"]), viol=len(sp["violations"]),
                    blind=len(sp["blind"]), quoted=len(sp["quoted"]),
                    reasons=sorted(FL.blind_counts(sp["blind"])))


def cmd_selftest(args):
    checks = []
    # 1) 七枚真语料形状：必须判盲区或不成链，绝不许判红
    viol_any, not_claim = [], []
    gl_reasons = {}
    for ln in LEGAL_GLUED:
        sp, t = _kinds(ln)
        if t["viol"]:
            viol_any.append((ln[:28], sp["violations"][0]["span"]))
        if t["claims"]:
            not_claim.append(ln[:28])
        for b in sp["blind"]:
            gl_reasons[b["reason"]] = gl_reasons.get(b["reason"], 0) + 1
    checks.append(("夹具A 真语料七型标识符/行号/型号/分号表格全不判红也不成主张（假阳＝人绕过闸门的代价）",
                   not viol_any and not not_claim,
                   "误红 %s ／ 误成主张 %s ／ 落档 %s" % (viol_any, not_claim, sorted(gl_reasons))))
    # 2) 三枚真主张：必须判成 claim 且闭合
    miss, bad = [], []
    for ln in LEGAL_CLAIMS:
        sp, t = _kinds(ln)
        if t["claims"] != 1:
            miss.append((ln[:24], t["claims"], t["reasons"]))
        if sp["violations"]:
            bad.append(sp["violations"][0]["span"])
    checks.append(("夹具B 三枚真主张（带标签七项／带乘法两段／三段和）必须各认成 1 条主张且闭合",
                   not miss and not bad, "未认成 %s ／ 误判不闭合 %s" % (miss, bad)))
    # 3) 植一处真错：必须红，且只红这一条
    sp3, t3 = _kinds(BROKEN_CLAIM)
    checks.append(("夹具C 把 375 改成 374 ⇒ 唯一一条不闭合必须点出来（不是全红也不是没红）",
                   t3["viol"] == 1 and t3["claims"] == 1,
                   "主张 %d 违反 %d 「%s」取值 %s" % (t3["claims"], t3["viol"],
                                                   (sp3["violations"][0]["span"] if sp3["violations"] else "-"),
                                                   (sp3["violations"][0]["vals"] if sp3["violations"] else "-"))))
    # 4) 引文与主张的界碑
    sp4, t4 = _kinds(QUOTED_LINE)
    checks.append(("夹具D 「…」 里的旧错值是引文：按名点名，绝不进违反账",
                   t4["quoted"] == 1 and t4["viol"] == 0 and t4["claims"] == 0,
                   "引文 %d 主张 %d 违反 %d" % (t4["quoted"], t4["claims"], t4["viol"])))
    # 5) 六种盲区各自落档，且都不成主张、不判红
    wrong = []
    for ln, want in FIX_BLIND:
        sp, t = _kinds(ln)
        got = FL.blind_counts(sp["blind"])
        if t["viol"] or t["claims"] or want not in got:
            wrong.append((want, ln[:20], t["claims"], t["reasons"]))
    checks.append(("夹具E 七型盲区各自落进自己那一格，且每型恰有一枚夹具（盲区型与夹具数不等＝有一型从没被复验过）",
                   not wrong and sorted([r for _, r in FIX_BLIND]) == sorted(FL.BLIND_REASONS),
                   "错档 %s ／ 夹具 %d 注册 %d" % (wrong, len(FIX_BLIND), len(FL.BLIND_REASONS))))
    # 5b) 代码段里的示意公式不许成为第二条活账
    spG, tG = _kinds(FIX_CODE_ONLY)
    checks.append(("夹具G 反引号内的示意等式不出账也不判红（否则同一个事实被两把尺各数一次）",
                   tG["claims"] == 0 and tG["viol"] == 0,
                   "主张 %d 违反 %d 落档 %s" % (tG["claims"], tG["viol"], tG["reasons"])))
    # 6) 分母不许凭手感：reason 闭集＋扫描集非空
    agg = census(["docs"])
    used = set(agg["blind_counts"])
    checks.append(("夹具F 扫描集非空且盲区 reason 全在注册表内（空集上的全称判据会把零证据印成满把握）",
                   agg["files"] > 100 and agg["claims"] >= len(LEGAL_CLAIMS)
                   and used <= set(FL.BLIND_REASONS),
                   "文件 %d 主张 %d 违反 %d 盲区 %d 引文 %d ／ 未注册 reason %s" % (
                       agg["files"], agg["claims"], agg["violations"], agg["blind"],
                       agg["quoted"], sorted(used - set(FL.BLIND_REASONS)))))
    # 7) 变异体：每撤一条判据都要看见判决翻转，否则那根针根本没穿这条通道
    def flip(name, attr, value, text, want):
        orig = getattr(FL, attr)
        setattr(FL, attr, value)
        try:
            sp, t = _kinds(text)
        finally:
            setattr(FL, attr, orig)
        checks.append((name, want(t, sp), "撤后 主张 %d 违反 %d 落档 %s 「%s」" % (
            t["claims"], t["viol"], t["reasons"],
            (sp["violations"][0]["span"] if sp["violations"] else "-"))))

    NEG = re.compile(r"(?!)")
    flip("变异体1 撤「粘连即非数」 ⇒ 编号 `A1` 的 1 被当成操作数并误红（标识符读成数＝假阳，代价是人绕过闸门）",
         "_glued", lambda text, m: False, _fix("glued"),
         lambda t, sp: t["viol"] == 1)
    flip("变异体2 撤句子终止符 ⇒ 分号两侧被接成一条链并误红（跨句连读必然算错）",
         "CUT_RE", NEG, CUT_FIXTURE,
         lambda t, sp: t["viol"] == 1)
    flip("变异体3 撤引文通道 ⇒ 登记旧错值的那句被判成主张不闭合（分不清引文与主张时主张不可信）",
         "QUOTE_SPAN_RE", NEG, QUOTED_LINE,
         lambda t, sp: t["viol"] >= 1)
    flip("变异体4 撤乘号通道 ⇒ 带 × 的真主张要么碎裂要么取值错（乘法不是装饰）",
         "MUL_RE", NEG, LEGAL_CLAIMS[1],
         lambda t, sp: bool(sp["violations"]) or t["claims"] != 1)
    flip("变异体5 把链内跨度限到 6 字符 ⇒ 带标签的长链被切碎（分母会静默变小）",
         "MAX_GAP", 6, LEGAL_CLAIMS[2],
         lambda t, sp: t["claims"] < 1 or bool(sp["violations"]))
    flip("变异体6 撤「紧邻加号＝增量记号」 ⇒ 「946 ＝ +1 文件 ＋9 例」被当成主张并误红",
         "_adjacent_add", lambda gap, ns, ln: False, _fix("increment"),
         lambda t, sp: t["viol"] == 1)
    flip("变异体7 撤代码段遮蔽 ⇒ 跨反引号的链把示例里的数字当操作数并误红（拆出的数比账上多一个 12）",
         "CODE_SPAN_RE", NEG, _fix("code"),
         lambda t, sp: bool(sp["violations"]))
    flip("变异体9 撤代码段遮蔽（同一条通道，另一种失效方向）⇒ 示意等式被登成第二条活账，同一个事实两个源",
         "CODE_SPAN_RE", NEG, FIX_CODE_ONLY,
         lambda t, sp: t["claims"] == 1)
    flip("变异体8 撤非等式符号遮蔽 ⇒ 「50 ＝ 甲 ≥ 20 ＋ 30」被判成一条闭合主张（假阴：不闭合也读不出来）",
         "NOISE_RE", NEG, _fix("noise"),
         lambda t, sp: t["claims"] == 1 and t["viol"] == 0)
    # 8) 识别器单归宿账：真扫描集零别家，且每根针都要能被打红
    real_items = reco_items()
    rep0 = reco_audit(real_items)
    bad0 = reco_verdict(rep0)
    checks.append(("夹具H 仪器目录现量：算术尺只住在归宿文件里，判决集非空、无解析失败、归宿自己在扫描集内",
                   bad0 == [] and rep0["home_seen"] and rep0["judged"] >= 1 and rep0["files"] >= 1,
                   "仪器 %d 份 位点 %d 处 算术命中 %d（归宿 %d／别家 %d）盲区 %d 解析失败 %d ｜ 红因 %s" % (
                       rep0["files"], rep0["judged"], len(rep0["hits"]), rep0["home_count"],
                       len(rep0["foreign"]), len(rep0["blind_dynamic"]), len(rep0["unparsable"]), bad0)))
    checks.append(("夹具I 词汇只命名一次：三根针的模式串由 FL.EQ_FW／ADD_FW 拼出后仍与手写基准逐字节相同",
                   FL.EQ_FW_RE.pattern == "＝" and FL.ADD_RE.pattern == "[+＋]"
                   and FL.ANY_OP_RE.pattern == "[=＝+＋×≥≤<>~→]",
                   "EQ %r ADD %r ANY %r" % (FL.EQ_FW_RE.pattern, FL.ADD_RE.pattern, FL.ANY_OP_RE.pattern)))
    SECOND_FW = ("import re\n"
                 "def f(x):\n"
                 "    return re.search(r'(\\d+) ＝ (\\d+) ＋ (\\d+)', x)\n")
    SECOND_NAMED = ("import re\n"
                    "EQ2 = re.compile(r'(\\d+) ＝ (\\d+)')\n"
                    "def f(x):\n"
                    "    return EQ2.search(x)\n")
    SECOND_ASCII = ("import re\n"
                    "EQ3 = re.compile(r'(\\d+) = (\\d+) \\+ (\\d+)')\n"
                    "def f(x):\n"
                    "    return EQ3.findall(x)\n")
    BENIGN = ("import re\n"
              "PORT = re.compile(r'(?:^|[^A-Z])PORT\\w*\\s*[:=]\\s*(\\d{2,5})')\n"
              "CNT = re.compile(r'SELFTEST PASS=(\\d+) FAIL=(\\d+)')\n"
              "def f(x):\n"
              "    return PORT.search(x), CNT.search(x)\n")
    HOME_STUB = (RECO_HOME, "")
    r_fw = reco_audit([HOME_STUB, ("scripts/gate/second_fw.py", SECOND_FW)])
    r_named = reco_audit([HOME_STUB, ("scripts/gate/second_named.py", SECOND_NAMED)])
    r_ascii = reco_audit([HOME_STUB, ("scripts/gate/second_ascii.py", SECOND_ASCII)])
    r_ben = reco_audit([HOME_STUB, ("scripts/gate/benign.py", BENIGN)])
    checks.append(("变异体10 别家在三处不同位置自造算术尺（re-call 字面量／具名常量再 compile／ASCII 转义加号）"
                   "都要点名到文件与行号，而端口尺与计数尺不许误报（误报的代价是人绕过闸门）",
                   {h["path"] for h in r_fw["foreign"]} == {"scripts/gate/second_fw.py"}
                   and {h["path"] for h in r_named["foreign"]} == {"scripts/gate/second_named.py"}
                   and {h["path"] for h in r_ascii["foreign"]} == {"scripts/gate/second_ascii.py"}
                   and not r_ben["foreign"] and len(r_ben["eq_unflagged"]) >= 2,
                   "全角 %s 具名 %s ASCII %s ｜  benign 误报 %d 出账 %d ｜ 红因 %s" % (
                       sorted({h["path"] for h in r_fw["foreign"]}),
                       sorted({h["path"] for h in r_named["foreign"]}),
                       sorted({h["path"] for h in r_ascii["foreign"]}), len(r_ben["foreign"]),
                       len(r_ben["eq_unflagged"]), reco_verdict(r_ben))))
    checks.append(("变异体11 把归宿文件从扫描集里撤掉：全称判据会立刻变成空集上的真话（别家 0 处却什么都没测），"
                   "所以归宿在不在场必须自己成一条判决",
                   reco_verdict(r_ben) == [] and "归宿文件不在扫描集"
                   in " ".join(reco_verdict(reco_audit([("scripts/gate/second_fw.py", SECOND_FW)]))),
                   "无归宿时的红因 %s" % reco_verdict(reco_audit([("scripts/gate/second_fw.py", SECOND_FW)]))))
    orig_arith = globals()["reco_is_arith"]
    globals()["reco_is_arith"] = lambda pat: False
    try:
        r_novic = reco_audit([HOME_STUB, ("scripts/gate/second_fw.py", SECOND_FW)])
    finally:
        globals()["reco_is_arith"] = orig_arith
    checks.append(("变异体12 撤词汇判定 ⇒ 别家照样「零红」：证明红来自 FL 的词汇而不是文件清单",
                   r_novic["foreign"] == [] and r_novic["judged"] >= 1,
                   "撤后 命中 %d 别家 %d 位点 %d" % (len(r_novic["hits"]), len(r_novic["foreign"]),
                                                   r_novic["judged"])))
    RECV_FIX = ("import re\n"
                "RX = re.compile(r'(\\\\d+)')\n"
                "def f(x):\n"
                "    return RX.search(x)\n")
    r_recv = reco_audit([HOME_STUB, ("scripts/gate/recv.py", RECV_FIX)])
    orig_pm = globals()["RECO_PMETHOD"]
    globals()["RECO_PMETHOD"] = frozenset()
    try:
        r_norecv = reco_audit([HOME_STUB, ("scripts/gate/recv.py", RECV_FIX)])
    finally:
        globals()["RECO_PMETHOD"] = orig_pm
    checks.append(("变异体13 撤「已编译模式走接收者」这条通道：判决集必须真的少一个位点。"
                   "这条通道买的是盲区诚实（早先一版把 arg0 当模式，115 处全落盲区而归宿账照样全绿），"
                   "不是新增红",
                   r_recv["judged"] == 2 and r_norecv["judged"] == 1,
                   "有通道 %d 位点／撤后 %d 位点（撤后盲区 %d）" % (
                       r_recv["judged"], r_norecv["judged"], len(r_norecv["blind_dynamic"]))))
    orig_fold = globals()["_fold"]
    globals()["_fold"] = (lambda e, p: e.value if isinstance(e, ast.Constant)
                          and isinstance(e.value, str) else None)
    try:
        r_nofold = reco_audit(real_items)
    finally:
        globals()["_fold"] = orig_fold
    checks.append(("变异体14 撤串接折叠 ⇒ 归宿文件自己的 ADD_RE／ANY_OP_RE（模式串由词汇常量拼出）读不出，"
                   "算术命中必须真的变少：把词汇收成一处命名曾让这把尺变盲，这条通道就是为此补的",
                   r_nofold["home_count"] < rep0["home_count"],
                   "折叠 %d 处／撤后 %d 处（撤后盲区 %d 配 %d）" % (
                       rep0["home_count"], r_nofold["home_count"],
                       len(r_nofold["blind_dynamic"]), len(rep0["blind_dynamic"]))))
    SECOND_JS = ("const EQ = /(\\d+) ＝ (\\d+) ＋ (\\d+)/;\n"
                 "export function f(s) { return s.match(EQ); }\n")
    BENIGN_JS = "export const g = (s) => s.match(/PORT=(\\d+)/);\n"
    r_js = reco_audit([HOME_STUB, ("frontend-ui/scripts/gate/second.mjs", SECOND_JS)])
    r_js_ok = reco_audit([HOME_STUB, ("frontend-ui/scripts/gate/ok.mjs", BENIGN_JS)])
    checks.append(("变异体15 弱尺通道（非 Python 仪器整行找记号）：.mjs 里自造等式尺必须点名，"
                   "而 ASCII key=value 形态不许误报；真语料那几份非 Python 仪器命中 0 是『看过了』不是『没看』",
                   {h["path"] for h in r_js["foreign"]} == {"frontend-ui/scripts/gate/second.mjs"}
                   and r_js["foreign"][0]["channel"] == "weak-text"
                   and not r_js_ok["foreign"] and rep0["other_files"] >= 1
                   and not [h for h in rep0["hits"] if h["channel"] == "weak-text"],
                   "命中 %s ／ benign %d ／ 真语料非 Python %d 份弱尺命中 %d" % (
                       [h["path"] for h in r_js["foreign"]], len(r_js_ok["foreign"]),
                       rep0["other_files"],
                       len([h for h in rep0["hits"] if h["channel"] == "weak-text"]))))
    out0 = outside_census()
    docs_paths = {os.path.relpath(p, REPO).replace("\\", "/") for p in walk_md(ROOTS)}
    checks.append(("夹具J 扫描集边界自署：集外探针必须真的读到东西（顶层目录＋仓根 .md），"
                   "集内集外不许重叠（重叠＝同一本账复算两遍，两份都会漂），集外只在「自己不闭合」时判红",
                   out0["files"] > 0 and out0["dirs"] >= 10 and out0["root_files"] >= 1
                   and outside_verdict(out0) == []
                   and not (set(d for d, _ in out0["claim_files"]) & docs_paths)
                   and all(not rel.startswith("docs/") for rel, _ in out0["claim_files"]),
                   "集外文件 %d（目录 %d／仓根 %d／按名跳过 %d）主张 %d 不闭合 %d 点名 %s" % (
                       out0["files"], out0["dirs"], out0["root_files"], out0["skip_dirs"],
                       out0["claims"], out0["violations"], [r for r, _ in out0["claim_files"]])))
    tmp = tempfile.mkdtemp(prefix="mox-outside-")
    try:
        bad_p = os.path.join(tmp, "unclosed.md")
        ok_p = os.path.join(tmp, "closed.md")
        with io.open(bad_p, "w", encoding="utf-8", newline="\n") as fh:
            fh.write("在册装配节点 374 ＝ 带分支标签 359 ＋ 公共面 16\n")
        with io.open(ok_p, "w", encoding="utf-8", newline="\n") as fh:
            fh.write("在册装配节点 375 ＝ 带分支标签 359 ＋ 公共面 16\n")
        agg_t = _scan_paths([ok_p, bad_p])
        red_t = outside_verdict(agg_t)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    # 按** basename 全等**取被点名的那份：`unclosed.md` 含 `closed.md` 这个子串，
    # 用 `in` 判"另一份没被点"会永远判错（去前缀针是带前缀针的子串，同族第二形态）。
    # 取路径要按 `:(\d+) ` 收尾而不是 split(':')[0]——Windows 绝对路径自己就带盘冒号。
    m_t = re.search(r"不闭合 (.+?):(\d+) ", red_t[0]) if len(red_t) == 1 else None
    named = os.path.basename(m_t.group(1)) if m_t else "-"
    checks.append(("变异体16 集外真文件（写在仓库外的临时目录，跑完即删）一条闭合一条不闭合："
                   "两份都必须被读到（claims=2，否则红可能来自没读），而红只许点不闭合那一份",
                   agg_t["files"] == 2 and agg_t["claims"] == 2 and len(red_t) == 1
                   and named == "unclosed.md",
                   "主张 %d 红 %d 点名 %s" % (agg_t["claims"], len(red_t), named)))
    orig_top = globals()["outside_top"]
    globals()["outside_top"] = lambda repo=REPO: ([], [])
    try:
        out_empty = outside_verdict(outside_census())
    finally:
        globals()["outside_top"] = orig_top
    checks.append(("变异体17 把集外枚举清空（模拟第三方目录被顺手跳过／列名写错那种塌缩）："
                   "全称判据在空集上恒真，所以分母塌了必须自己成一条红",
                   out_empty and "0 份" in " ".join(out_empty),
                   "塌缩红因 %s ／ 现量红因 %s" % (out_empty, outside_verdict(out0))))
    out0 = reco_outside_audit()
    inter = {rel for rel, _ in reco_items()} & set(reco_outside_paths()[0])
    checks.append(("夹具K 识别器扫描集**自己**的边界：『其余 N 份无一把自带』的宇宙是三条 glob，"
                   "集外那些写得动正则的代码要按同一套判据复算——候选分母要非零、按名跳过的目录数要印出来、"
                   "集外命中必须为 0，而归宿文件在这条通道里也要在场（否则又是空集上的全称判据）；"
                   "两本分母还须不相交（相交＝同一份仪器在册与集外各数一遍，PASS 句的和就是假总数）",
                   out0["out_files"] > 0 and out0["skip_dirs"] > 0 and out0["foreign"] == []
                   and out0["bad"] == [] and out0["home_seen"] and out0["home_count"] >= 1
                   and not inter,
                   "集外候选 %d 份／按名跳过目录 %d 个／集外位点 %d 盲区 %d／归宿命中 %d 别家 %d／集内集外交集 %d 份" % (
                       out0["out_files"], out0["skip_dirs"], out0["judged"],
                       len(out0["blind_dynamic"]), out0["home_count"], len(out0["foreign"]), len(inter))))
    o18 = reco_outside_audit(items=[("scripts/maintenance/probes/planted.py", SECOND_FW)])
    checks.append(("变异体18 在**真实集外目录形状**里放一枚自造等式尺（`scripts/**` 下的 .py，不在三条 glob 内）："
                   "集外通道必须点名到文件，否则夹具K 那句『集外 0 处』只是没看",
                   [h["path"] for h in o18["foreign"]] == ["scripts/maintenance/probes/planted.py"]
                   and len(o18["bad"]) == 1 and "planted.py" in o18["bad"][0] and o18["out_files"] == 1,
                   "点名 %s ／ 红 %s ｜ 塌缩对照 %s" % (
                       [h["path"] for h in o18["foreign"]], o18["bad"],
                       reco_outside_audit(items=[])["bad"])))
    BOM = u"\ufeff"
    tmp2 = tempfile.mkdtemp(prefix="mox-bom-")
    try:
        bom_p = os.path.join(tmp2, "bom_gate.py")
        with io.open(bom_p, "w", encoding="utf-8-sig", newline="\n") as fh:
            fh.write(SECOND_FW)
        src_bom = _read_code(bom_p)
        rep_bom = reco_audit([(RECO_HOME, ""), ("scripts/gate/bom_gate.py", src_bom)])
    finally:
        shutil.rmtree(tmp2, ignore_errors=True)
    checks.append(("变异体19 带 BOM 的仪器必须读得动（读码走 utf-8-sig）：BOM 留在串首时 `ast.parse` 报"
                   "「invalid character in identifier」⇒ 那台仪器整份落进『解析失败』，它自带的第二把尺从此隐身"
                   "（本仓真有这样一份：`scripts/ci/ci.py` 首字节是 BOM）",
                   not src_bom.startswith(BOM) and rep_bom["unparsable"] == []
                   and [h["path"] for h in rep_bom["foreign"]] == ["scripts/gate/bom_gate.py"],
                   "BOM 已剥离 %s ／ 解析失败 %d ／ 点名 %s" % (
                       not src_bom.startswith(BOM), len(rep_bom["unparsable"]),
                       [h["path"] for h in rep_bom["foreign"]])))
    def _tmpl_prefixes():
        """红模板从**本文件自己的源码**现取，不写清单——清单会漏掉明天新加的那条红。
        抽取口径不在这里另写一份：由 `formula_ledger` 单源（本机把红写成 `bad.append(模板)`，
        即 `form="append"`；普查器那侧写成 `print("FAIL …")`，即 `form="print"`），
        两侧共用同一条「取最左字面量、在首个占位符处截、截短则按整串收」的规则。"""
        return FL.red_outlet_universe(_read_code(os.path.abspath(__file__)), "append")

    tmpls = _tmpl_prefixes()

    def _battery_run(f):
        try:
            return list(f()), []
        except Exception as exc:
            return [], ["%s: %s" % (type(exc).__name__, exc)]

    def _doc_outside_collapse():
        orig_top2 = globals()["outside_top"]
        globals()["outside_top"] = lambda repo=REPO: ([], [])
        try:
            return outside_verdict(outside_census())
        finally:
            globals()["outside_top"] = orig_top2

    csp_err = ""
    csp = None
    try:
        csp = _csp_instrument()
    except Exception as exc:
        csp_err = "%s: %s" % (type(exc).__name__, exc)
    ci_p = os.path.join(REPO, CI_WORKFLOW)
    ci_text = _read_code(ci_p) if os.path.isfile(ci_p) else ""
    ci_lines = ci_text.split("\n")
    ci_tokens = csp.ci_command_tokens(ci_text) if (csp and ci_text) else []
    real_red = ci_wiring_verdict(ci_tokens, ci_lines)
    # 正对照：参数落在续行上的合法写法不许判红。只测真语料永远不知道这条判据会不会咬错人。
    cont_ok = ci_wiring_verdict(
        [(1, CI_WIRED[0]), (3, CI_WIRED[1])],
        ["python %s \\" % CI_WIRED[0], "  " + CI_SELFTEST_ARG,
         "python %s %s" % (CI_WIRED[1], CI_SELFTEST_ARG)])
    ci_kept, ci_echoed = ci_exec_tokens(ci_tokens, ci_lines)
    # 正对照：复合命令的首词不是打印语句（`cd x && python y`），必须照旧算执行——
    # 把收窄写成"整串含 echo 就丢"会咬错这种写法，而假阳的代价是人绕过闸门。
    compound_ok = ci_wiring_verdict(
        [(1, CI_WIRED[0]), (1, CI_WIRED[1])],
        ["cd frontend-ui && python %s %s && python %s %s"
         % (CI_WIRED[0], CI_SELFTEST_ARG, CI_WIRED[1], CI_SELFTEST_ARG)])
    checks.append(("夹具M CI 接线账：『接了 CI』的宇宙取自 check-script-paths.py 的 run 块判据（不自写第二把尺），"
                   "真 ci.yml 必须让名单里两台仪器都带上 --selftest；分母从对象推（可执行脚本种类 ≥ 名单长度），"
                   "兄弟门禁装载失败要印出来而不是安静退化成没有红；另钉两枚正对照（续行写法／复合命令写法）",
                   not csp_err and ci_text != "" and real_red == [] and cont_ok == [] and compound_ok == []
                   and len(set(w for _, w in ci_kept)) >= len(CI_WIRED),
                   "run 块 token %d 条（收窄后执行 %d 条／打印语句里 %d 条）／脚本种类 %d／名单 %d 台／"
                   "真语料红 %s／续行对照 %s／复合命令对照 %s／装载 %s" % (
                       len(ci_tokens), len(ci_kept), len(ci_echoed), len(set(w for _, w in ci_kept)),
                       len(CI_WIRED), real_red, cont_ok, compound_ok, csp_err or "OK")))
    note_text = ("        run: |\n"
                 "          python scripts/gate/other_gate.py %s\n"
                 "          # 这里曾写着 python %s %s\n" % (CI_SELFTEST_ARG, CI_WIRED[0], CI_SELFTEST_ARG))
    note_tokens = csp.ci_command_tokens(note_text) if csp else []
    note_red = ci_wiring_verdict(note_tokens, note_text.split("\n"))
    note_missing = [r for r in note_red if r.startswith("CI 未接线")]
    checks.append(("变异体20 注释里的写法不算接线：名单仪器只写进 run 块的 shell 注释时，复用来的注释剥离通道"
                   "必须让它落到『未接线』（否则一行注释就能把『接了 CI』骗过去），且两台都不许隐身",
                   len(note_tokens) == 1 and note_tokens[0][1] == "scripts/gate/other_gate.py"
                   and len(note_missing) == len(CI_WIRED),
                   "剥注释后可执行 token %d 条／未接线红 %d 条／全部红 %s" % (
                       len(note_tokens), len(note_missing), note_red)))
    echo_text = ("        run: |\n"
                 "          python scripts/gate/other_gate.py %s\n"
                 "          echo \"NOT-WIRED python %s %s\"\n" % (CI_SELFTEST_ARG, CI_WIRED[0], CI_SELFTEST_ARG))
    echo_tokens = csp.ci_command_tokens(echo_text) if csp else []
    echo_lines = echo_text.split("\n")
    echo_kept, echo_dropped = ci_exec_tokens(echo_tokens, echo_lines)
    echo_red = ci_wiring_verdict(echo_tokens, echo_lines)
    # 撤掉收窄这条通道，同一个宇宙就必须读成"已接线"——针不红而撤通道也不红，说明针根本没穿这条通道。
    echo_unfiltered = ci_wiring_verdict(echo_tokens, echo_lines, want=[CI_WIRED[0]], printer=())
    checks.append(("变异体21 `echo` 里的命令串不算接线（本仓 ci.yml 三条 BLOCKED 行正是这个形状）："
                   "复用尺按空白切词、分不出打印与执行，不收窄则一行 echo 就能把『已接线』糊出来；"
                   "被撤下的 token 要按名印出来，且同一宇宙在**撤掉收窄**后必须读成已接线（证明针穿的是这条通道）",
                   len(echo_tokens) == 2 and len(echo_kept) == 1 and len(echo_dropped) == 1
                   and echo_dropped[0][1] == CI_WIRED[0] and echo_dropped[0][2] == "echo"
                   and ("CI 未接线 %s：名单里的仪器没出现在任何一条 run 命令里" % CI_WIRED[0]) in echo_red
                   and echo_unfiltered == [],
                   "原始 token %d／执行 %d／打印里 %d（%s）／红 %d 条／撤窄对照 %s" % (
                       len(echo_tokens), len(echo_kept), len(echo_dropped),
                       [d[1] for d in echo_dropped], len(echo_red), echo_unfiltered)))
    inv, plant_moved, quad = channel_glyph_invariance(["docs/architecture"], DIR_ACCOUNT)
    checks.append(("夹具N 通道依赖实测：宣称一条判决『待裁决点 6』之前，先把托管词汇表中性化看它会不会变——"
                   "目录账（扫描集牙齿）必须不变，同机植入的那枚真读字形表的判决必须变；"
                   "后者不变则前者一文不值（反事实只在内存里换 globals()[FL]，不写盘）",
                   inv and plant_moved,
                   "目录账不变 %s／植入判决变 %s／中性化前 %s → 后 %s" % (
                       inv, plant_moved, quad[2], quad[3])))
    inv2, plant2, _q2 = channel_glyph_invariance(
        ["docs/architecture"], DIR_ACCOUNT, neutralize=lambda: globals()["FL"])
    checks.append(("变异体22 把中性化换成恒等（替身＝同一个 FL）：植入判决从此不会变，夹具N 的『必须变』那半边失守 "
                   "⇒ 那一格必须判红。这枚钉的是『反事实通道有没有牙』，不是目录账自己",
                   inv2 and not plant2,
                   "恒等替身：目录账不变 %s／植入判决变 %s" % (inv2, plant2)))
    wf_texts = ci_workflow_texts()
    exec_paths, exec_where = set(), {}
    for rel, wtext in wf_texts:
        kept_w, _drop_w = (ci_exec_tokens(csp.ci_command_tokens(wtext), wtext.split("\n"))
                           if csp else ([], []))
        for _ln, word in kept_w:
            exec_paths.add(word)
            exec_where.setdefault(word, []).append(rel)
    ag_path = os.path.join(REPO, AGENTS_DOC)
    ag_text = _read_code(ag_path) if os.path.isfile(ag_path) else ""
    ag_lines = ag_text.split("\n")
    ag_tokens = csp.scan_tokens(ag_text) if (csp and ag_text) else []
    sfx = csp.SCRIPT_SUFFIX if csp else ()
    claims, no_path, non_script = doc_ci_claims(ag_lines, ag_tokens, sfx)
    claim_red = ci_claim_verdict(claims, exec_paths, no_path, non_script)
    checks.append(("夹具O 文档声称的『CI 门禁』逐条对账：名单来源换成 AGENTS.md 自己（同一行既有标记又挂着脚本路径），"
                   "每条声称都必须在某份 workflow 的真 run 命令里出现；分母塌了自成一条红；"
                   "没有脚本路径可挂的标记行按行号印出来——那种形状本格是盲的，不许让『全绿』冒充『都核实了』。"
                   "『已接线』是下界：只保证某份 workflow 里有一条 run 命令执行它，不保证那份 workflow 在当前分支或 "
                   "`paths:` 过滤下真会触发——落点按名印出来，触发条件去读那个文件",
                   bool(wf_texts) and ag_text != "" and not csp_err and bool(claims) and claim_red == [],
                   "workflow %d 份／执行脚本种类 %d／声称 %d 条／未接线红 %s／无路径可核的行 %s／后缀挡下的 token %s／落点 %s" % (
                       len(wf_texts), len(exec_paths), len(claims), claim_red, no_path,
                       [t for _l, t in non_script],
                       [(p.split("/")[-1], sorted({r.split("/")[-1] for r in exec_where.get(p, [])}))
                        for _l, p in claims])))
    planted_claim = "scripts/gate/planted_claim.py"
    # 形状必须与真 ci.yml 一致：`run: |` 自成一行为块首，续行比它更深。
    # 写成 step 内联的 `- run: |` 时复用的块判据（stripped.startswith('run:')）先拒掉整块，
    # 于是执行宇宙是空的——那时这枚变异体会在"没有任何东西被读到"上打出未接线，
    # 证明的是自己没电，不是通道有牙。
    echo_only = ("        run: |\n"
                 "          python scripts/gate/other.py --selftest\n"
                 '          echo "BLOCKED python %s --check：这条只是文案"\n' % planted_claim)
    ek, ed = (ci_exec_tokens(csp.ci_command_tokens(echo_only), echo_only.split("\n"))
              if csp else ([], []))
    planted_red = ci_claim_verdict([(1, planted_claim)], set(w for _l, w in ek), [], [])
    checks.append(("变异体23 声称只住在 echo 文案里时必须读成『未接线』（与 夹具M 同族的双向性——"
                   "一行 echo 不许把文档口径的『CI 门禁』骗过去），且被挡下的那条要按名印出来。"
                   "另钉『执行宇宙恰 1 条』：块没被读到时宇宙也是空的，『未接线』会在空集上恒真——"
                   "那证明的是仪器没电，不是通道有牙",
                   len(ek) == 1 and [w for _l, w in ek] == ["scripts/gate/other.py"]
                   and planted_claim not in set(w for _l, w in ek) and len(planted_red) == 1
                   and planted_red[0].startswith("CI 声称未接线") and ed != [],
                   "合成宇宙：执行 %s／打印里 %s／红 %s" % (
                       [w for _l, w in ek], [(l, w, h) for l, w, h in ed], planted_red)))
    m_claims, m_nopath, m_nonscript = doc_ci_claims(ag_lines, ag_tokens, sfx, mark="CI 门禁-不存在的标记")
    collapse_red = ci_claim_verdict(m_claims, exec_paths, m_nopath, m_nonscript)
    checks.append(("变异体24 把标记通道撤掉（换一个文档里不存在的词）⇒ 声称账必须当场塌成一条红，"
                   "而不是安静地印『0 条声称、全部核实』：这一格钉的是标记本身有没有咬住文档",
                   m_claims == [] and len(collapse_red) == 1
                   and collapse_red[0].startswith("CI 声称账塌缩"),
                   "撤标记后：声称 %d 条／红 %s／真标记下声称 %d 条" % (
                       len(m_claims), collapse_red, len(claims))))
    # 已知后缀边界的"必须响亮地红"对照：SCRIPT_SUFFIX 白名单不含 .mjs（复用尺的白名单，不改它），
    # 所以「node x.mjs # CI 门禁」这一行永远挂不上脚本路径 ⇒ 声称账塌缩成一条红，
    # 而不是安静地印「0 条声称、全部核实」。这条边界的方向是安全的（红，不是绿），钉住它是为了
    # 明天有人把白名单扩到 .mjs 时，这格会反过来要求他重钉判据。
    mjs_lines = ["        node scripts/gate/check-dead.mjs --selftest  # CI 门禁"]
    mjs_claims, mjs_nopath, mjs_ns = doc_ci_claims(
        mjs_lines, [(1, "scripts/gate/check-dead.mjs")], sfx)
    mjs_red = ci_claim_verdict(mjs_claims, set(["scripts/gate/check-dead.mjs"]),
                               mjs_nopath, mjs_ns)
    checks.append(("变异体25 后缀边界（`.mjs` 不在复用尺的 SCRIPT_SUFFIX 里）必须落在『响亮地红』那一侧："
                   "挂着标记却挂不上可核路径的行，只能进盲区账按行号印出来，绝不能被算成一条已核实的声称",
                   mjs_claims == [] and mjs_nopath == [1] and len(mjs_red) == 1
                   and mjs_red[0].startswith("CI 声称账塌缩")
                   and len(mjs_ns) == 1 and mjs_ns[0][1].endswith(".mjs"),
                   "合成行：声称 %d 条／无路径可核的行 %s／后缀挡下 %s／红 %s" % (
                       len(mjs_claims), mjs_nopath, [t for _l, t in mjs_ns], mjs_red)))
    rev_red, gate_scripts = gate_scripts_unlisted(exec_paths, ag_text)
    checks.append(("夹具Q 反向账：CI 每次真在跑、且住在 */gate/ 目录下的脚本必须在 AGENTS.md 点得名——"
                   "正向账（夹具O）问『声称的有没有跑』，本格问『跑的有没有写』，只钉一侧时另一侧漂移是静默的"
                   "（指南少一格，新人照少一格的口径跑，而那条门禁自己照样天天绿）；"
                   "目录名是口径，落在该目录外的执行对象只数不判，免得把 %s 读成执行宇宙的全部" % GATE_DIR_MARK,
                   bool(gate_scripts) and rev_red == [],
                   "gate 目录内执行脚本 %d 份／未登记 %s／执行宇宙合计 %d 份（目录外 %d 份不在本格射程）" % (
                       len(gate_scripts), rev_red, len(exec_paths), len(exec_paths) - len(gate_scripts))))
    rev_a, gate_a = gate_scripts_unlisted(
        set(["scripts/gate/a.py", "scripts/other/b.py"]), "# 只登记了别的\n")
    rev_b, gate_b = gate_scripts_unlisted(set(), "")
    checks.append(("变异体26 反向账的两条通道各钉一枚：① 门禁脚本在 CI 跑而指南没写时必须点名到那份路径，"
                   "而同宇宙里住在 gate 目录外的脚本**不许**被一起点名（口径过宽＝假阳，代价是人绕过指南）；"
                   "② 执行宇宙里没有 gate 脚本时必须读成『塌缩』而不是『零条未登记＝齐备』",
                   len(rev_a) == 1 and rev_a[0].endswith("scripts/gate/a.py：CI 天天执行它，AGENTS.md 里却点不到")
                   and gate_a == ["scripts/gate/a.py"]
                   and len(rev_b) == 1 and rev_b[0].startswith("反向账塌缩") and gate_b == [],
                   "合成宇宙①：gate 内 %s／红 %s ｜合成宇宙②：gate 内 %s／红 %s" % (
                       gate_a, rev_a, gate_b, rev_b)))
    oc_path = os.path.join(REPO, ONE_CLICK_DOC)
    oc_text = _read_code(oc_path) if os.path.isfile(oc_path) else ""
    oc_red, oc_invoked, oc_total, oc_claim = one_click_audit(gate_scripts, oc_text, ag_text)
    checks.append(("夹具R 第三本账：AGENTS.md 宣传的『一键质量检查』必须与 CI 真跑的 gate 集合同账，"
                   "且项数只许有一个算源——分母写成字面量 N 次就是 N 个第二源，漏改的那几处不报错、"
                   "只是安静地印错数（后果恰好是『本地一键全绿、CI 却红』）；一键里一条 gate 都没执行时"
                   "必须读成塌缩，而不是『零份未进＝齐备』",
                   bool(gate_scripts) and oc_text != "" and oc_red == [],
                   "gate 宇宙 %d 份／一键实际执行 %d 份／%s＝%s／文档项数 %s／红 %s" % (
                       len(gate_scripts), len(oc_invoked), ONE_CLICK_TOTAL, oc_total, oc_claim, oc_red)))
    oc_m1, oc_i1, oc_t1, oc_c1 = one_click_audit(
        ["scripts/gate/a.py"],
        '$Total = 1\nWrite-Host "[1/$Total] x"\n', "")
    oc_m2, oc_i2, oc_t2, oc_c2 = one_click_audit(
        ["scripts/gate/a.py"], 'Write-Host "[1/7] x"\n& python "scripts/gate/a.py"\n', "")
    oc_m3, oc_i3, oc_t3, oc_c3 = one_click_audit(
        ["scripts/gate/a.py"],
        "$Total = 2\nWrite-Host \"[1/$Total] x\"\n& python \"scripts/gate/a.py\"\n"
        "Write-Host \"[2/$Total] y\"\n",
        "python %s  # 一键质量检查（9 项）\n" % ONE_CLICK_DOC)
    oc_m4, oc_i4, oc_t4, oc_c4 = one_click_audit(
        ["scripts/gate/a.py"],
        "$Total = 1\nWrite-Host \"[1/$Total] x\"\n& python \"scripts/gate/a.py\"\n",
        "python %s  # 一键质量检查（1 项）\n" % ONE_CLICK_DOC)
    checks.append(("变异体27 一键账的四条通道各钉一枚：① 一条 gate 都没执行＝塌缩；"
                   "② 分母写成字面量＝不合一（哪怕它自洽、gate 也都已执行——那种『看起来全绿』也要红，"
                   "否则下一位加步骤的人没有仪器提醒）；③ 文档项数与 $Total 不符＝两数一起点名；"
                   "④ 反对照：单源且项数相符时必须零红（收紧判据必须留 clean 侧，不然『红一切』也能自证绿）",
                   len(oc_m1) == 2 and any(x.startswith("一键账塌缩") for x in oc_m1) and oc_i1 == []
                   and len(oc_m2) == 1 and oc_m2[0].startswith("一键分母不合一")
                   and oc_i2 == ["scripts/gate/a.py"]
                   and len(oc_m3) == 1 and oc_m3[0].startswith("一键项数与文档不符")
                   and oc_t3 == 2 and oc_c3 == 9
                   and oc_m4 == [] and oc_t4 == 1 and oc_i4 == ["scripts/gate/a.py"],
                   "①红 %s ｜②红 %s ｜③红 %s ｜④（必须 clean）红 %s" % (oc_m1, oc_m2, oc_m3, oc_m4)))
    ci_inv = []
    for rel_w, wtext_w in wf_texts:
        ci_inv += ci_gate_invocations(csp, rel_w, wtext_w)
    arg_red, arg_want = one_click_arg_parity(ci_inv, oc_text, gate_scripts, oc_invoked)
    checks.append(("夹具S 第四维：CI 与一键必须**以同一组参数**跑同一条 gate。只比路径时，一键里的 `--selftest`（只测仪器自己、一眼不看仓库）就能顶掉 CI 里的 `--check`（判仓库）——本地全绿照样藏着一条 CI 红，缺陷与 夹具R 同族，只是这次藏在参数里；未进一键的路径由 夹具R 出红，本格不重复出第二条（同一个缺陷两条红时，改其一会让人以为两条都好了）",
                   bool(ci_inv) and bool(oc_invoked) and arg_red == [],
                   "CI 侧 (路径,参数) 组 %d／落在双账里的 %d 组／涉及 %d 份路径／红 %s" % (
                       len(ci_inv), len(arg_want), len(set(p for p, _a in arg_want)), arg_red)))
    gp = "scripts/gate/a.py"
    ap_s1, _aw1 = one_click_arg_parity([(gp, ("--check",), "ci.yml", 3)],
                                      '$Total = 1\n& python "%s" --selftest\n' % gp, [gp], [gp])
    ap_s2, _aw2 = one_click_arg_parity([(gp, ("--check",), "ci.yml", 3)],
                                      '$Total = 1\n& python "%s" --check\n' % gp, [gp], [gp])
    ap_s3, _aw3 = one_click_arg_parity([("scripts/gate/b.py", (), "ci.yml", 1)],
                                      '& python "%s" --check\n' % gp, [gp, "scripts/gate/b.py"], [gp])
    ap_s4, _aw4 = one_click_arg_parity([(gp, (), "ci.yml", 4)],
                                      '& python "%s" --check\n' % gp, [gp], [gp])
    inv_s5 = ci_gate_invocations(csp, "ci.yml",
                                 "run: |\n  python %s --check && python %s\n"
                                 '  echo "python %s --census"\n' % (gp, gp, gp))
    checks.append(("变异体28 参数账的五条通道各钉一枚：① 同路径不同参数＝不合一；② 反对照：参数逐词相同必须零红（收紧不允变成『红一切』）；③ 双账交集为空＝塌缩，不许读成『0 组不合一＝参数同账』；④ **裸跑也是一个参数组**——写成 `if args:` 就把它跳过了，那正是本地全绿藏 CI 红的形状；⑤ 参数宇宙必须继承 echo 收窄与分隔词截断（echo 里的写法不算，同一行把同一条路径敲两遍要各成一组，且段落止于分隔词，不许把后半串的参数扒到前一段上）",
                   len(ap_s1) == 1 and ap_s1[0].startswith("一键参数不合一")
                   and "一键里同路径只有 --selftest" in ap_s1[0]
                   and ap_s2 == []
                   and len(ap_s3) == 1 and ap_s3[0].startswith("一键参数账塌缩")
                   and len(ap_s4) == 1 and 'CI 以「(裸跑，无参数)」执行它（ci.yml 行 4）' in ap_s4[0]
                   and inv_s5 == [(gp, ("--check",), "ci.yml", 2), (gp, (), "ci.yml", 2)],
                   "①红 %s ｜②（必须 clean）红 %s ｜③红 %s ｜④红 %s ｜⑤组 %s" % (
                       ap_s1, ap_s2, ap_s3, ap_s4, inv_s5)))
    tmp3 = tempfile.mkdtemp(prefix="mox-unclosed-")
    unc_p = os.path.join(tmp3, "unclosed.md")
    with io.open(unc_p, "w", encoding="utf-8", newline="\n") as fh:
        fh.write("在册装配节点 374 ＝ 带分支标签 359 ＋ 公共面 16\n")
    t_paths = walk_md(ROOTS)
    t_dirs_arg = ["docs/architecture"]
    t_dirs = sorted({os.path.relpath(os.path.dirname(p), REPO).replace("\\", "/") for p in t_paths})
    t_univ = outside_buckets()
    t_lines, t_bad = dircheck_emit(t_dirs, len(t_paths), account_only=True, universe=t_univ)
    checks.append(("夹具T 目录账的独立口径在真语料上必须自己站得住：只出目录账那半条（不碰集外账，"
                   "那半条吃字形、整块待裁决点 6）、分母现量且不塌、判决为空。"
                   "这一格是『可一键接』的证据：没有它，想接目录账的人只能整块等裁决，"
                   "或者更糟——把 --dircheck 当目录账接进去，让一条天天红的字形账替一条本来能绿的账背锅。"
                   "自 §1.8l 起这半条含第五本账（集外宇宙登记账），只走路径 ⇒ 仍然一字节不读字形",
                   t_bad == [] and bool(t_dirs) and bool(t_paths) and len(t_lines) == 1
                   and t_lines[0].startswith("扫描目录 ")
                   and not [l for l in t_lines if "集外主张" in l or "不闭合" in l]
                   and any("集外桶" in l for l in t_lines),
                   "--dir-account 真语料｜目录 %d／账 %d／文件 %d｜集外桶 %d／.md %d／出账 %d｜行数 %d／红 %s" % (
                       len(t_dirs), len(DIR_ACCOUNT), len(t_paths), len(t_univ),
                       sum(n for _, n in t_univ), len(OUTSIDE_ACCOUNT), len(t_lines), t_bad)))
    u_synth = [("planted", 3)]
    ua_synth = [("planted", "植入桶：只为把第五本账的通道在内存里逐格定价，一个字节不落盘")]
    g_synth = ["given/a.md"]
    ga_synth = [("given", "植入桶：让出面账的通道在内存里逐格定价，既不落盘也不碰 git")]
    out_synth = {"files": 1, "dirs": 1, "root_files": 0, "skip_dirs": 0,
                 "claims": 1, "violations": 1, "blind": 0, "quoted": 0,
                 "claim_files": [("docs/planted.md", 1)],
                 "details": [{"path": "docs/planted.md", "line": 1,
                              "span": "在册 5 ＝ 2 ＋ 2", "vals": [5, 4]}]}
    full_lines, full_bad = dircheck_emit(t_dirs_arg, 12, out_synth, False, u_synth, ua_synth,
                                         g_synth, ga_synth)
    acct_lines, acct_bad = dircheck_emit(t_dirs_arg, 12, out_synth, True, u_synth, ua_synth,
                                         g_synth, ga_synth)
    head_nums = re.findall(r"\d+", acct_lines[0])
    g_part = given_up_partition(g_synth, [n for n, _ in u_synth])
    u_synth_n = [str(len(t_dirs_arg)), str(len(DIR_ACCOUNT)), str(12), str(len(u_synth)),
                 str(sum(n for _, n in u_synth)), str(len(ua_synth)),
                 str(len(g_part)), str(sum(len(v) for v in g_part.values())), str(len(ga_synth))]
    col_bad, _col_head = dir_account_audit([], 0, None, u_synth, ua_synth, g_synth, ga_synth)
    acct_clean, _ac_head = dir_account_audit(t_dirs_arg, 12, None, u_synth, ua_synth, g_synth, ga_synth)
    pre_lines, pre_bad = dircheck_emit(["docs/_archive"], 12, out_synth, True, u_synth, ua_synth,
                                       g_synth, ga_synth)
    orig_fl = globals()["FL"]
    try:
        globals()["FL"] = _neutral_ledger()
        neu_lines, neu_bad = dircheck_emit(["docs/_archive"], 12, out_synth, True, u_synth, ua_synth,
                                           g_synth, ga_synth)
    finally:
        globals()["FL"] = orig_fl
    # 『集外』两个字自 §1.8l 起两条账共用（集外主张账／集外宇宙登记账），按子串判泄漏就会把
    # 目录账口径里合法的分母行判成缺陷——探针必须点名为"主张通道"的字样。
    def _claim_channel(ls):
        return [x for x in ls if "集外主张" in x or "不闭合" in x]
    m29_leak = (len(full_bad) == 1 and full_bad[0].startswith("集外主张不闭合")
                and acct_bad == [] and _claim_channel(acct_lines) == []
                and _claim_channel(acct_bad) == [] and "集外桶" in acct_lines[0])
    m29_head = (full_lines[0] == acct_lines[0] and head_nums == u_synth_n)
    m29_collapse = (len(col_bad) == 1 and col_bad[0].startswith("目录账分母塌缩")
                    and "扫描目录 0 个／文件 0 个" in col_bad[0])
    m29_clean = (acct_clean == [] and len(pre_bad) == 1
                 and pre_bad[0].startswith("出账目录出现在扫描集里"))
    m29_glyph = (neu_lines == pre_lines and neu_bad == pre_bad and bool(pre_bad))
    checks.append(("变异体29 独立口径的五条通道各钉一枚（自 §1.8m 起合成格一律带植入让出面，不许就地读 git）：① 同一份集外数据在两个口径里必须一个有一个没有"
                   "（若 account_only 只是纸面参数，两份输出同形，这条立刻红）；"
                   "② 分母行只许一处生成——两条口径的头一行逐字节相同，九个数依次为目录／账／文件／集外桶／集外 .md／集外出账／让出桶／让出份数／让出账，且必须等于 len(DIR_ACCOUNT)、len(OUTSIDE_ACCOUNT) 与 len(GIVEN_UP_ACCOUNT)（现推不是抄）；"
                   "③ 扫描集塌缩必须自己出一条红并把两个数点名（空集上的全称判据＝把零证据印成满把握）；"
                   "④ 反对照：合法目录＋非零文件数必须零红，而归档目录那一枚必须有红在场（收紧不允变成『红一切』）；"
                   "⑤ 这条口径不读词汇表——把 FL 换成永不开火的替身，含一条真红的目录账输出必须逐字节不变"
                   "（『不变』的证据是那红在场，不是空串对空串）。①的泄漏探针按通道点名（集外主张／不闭合），"
                   "并反向要求分母行里真的出现『集外桶』——第五本账若不在这条口径里，①也一样红；"
                   "⑤把行同形与红同形两半分开印，只印一半会把另一边的不同读成全同形",
                   m29_leak and m29_head and m29_collapse and m29_clean and m29_glyph,
                   "①全量红 %s／口径内红 %s ｜②头 %s ｜③%s ｜④clean %s／在场 %s ｜⑤行同形 %s／红同形 %s" % (
                       full_bad, acct_bad, head_nums, col_bad, acct_clean, pre_bad,
                       neu_lines == pre_lines, neu_bad == pre_bad)))
    u_real = outside_buckets()
    u_cross = outside_census()
    u_named = set(n for n, _ in OUTSIDE_ACCOUNT)
    u30_leak = universe_verdict(list(u_real) + [("planted_bucket", 7)])
    u30_stale = universe_verdict([b for b in u_real if b[0] != "tools"])
    # ④ 只改 tools 那一行的理由：整张出账表换成一条＝把另外十个桶一起注销，红的就不是理由通道了。
    u30_why = universe_verdict(u_real, [(n, "短") if n == "tools" else (n, w)
                                        for n, w in OUTSIDE_ACCOUNT])
    u30_collapse = dir_account_audit(["docs/architecture"], 5, None, [], [], g_synth, ga_synth)[0]
    u30_naive = universe_verdict([], [])
    pre_u = dir_account_audit(["docs/architecture"], 5, None,
                              list(u_real) + [("planted_bucket", 7)], None,
                              g_synth, ga_synth)[0]
    orig_fl_u = globals()["FL"]
    try:
        globals()["FL"] = _neutral_ledger()
        neu_u = dir_account_audit(["docs/architecture"], 5, None,
                                  list(u_real) + [("planted_bucket", 7)], None,
                                  g_synth, ga_synth)[0]
    finally:
        globals()["FL"] = orig_fl_u
    checks.append(("夹具U 第五本账在真语料上必须双向闭合：桶非空（空＝仪器自己瞎了，不是集外干净）、"
                   "未登记为空、失效为空、每条理由≥8 字，且桶数与出账条目数现量相等；"
                   "另加一枚交叉复算——同一批路径走 _scan_paths 得到的份数必须与这条只走路径的账相同，"
                   "否则『集外宇宙』在两条通道上是两个数（第二把尺的形状，不是第二把尺）",
                   bool(u_real) and universe_verdict(u_real) == []
                   and len(u_real) == len(OUTSIDE_ACCOUNT)
                   and set(n for n, _ in u_real) == u_named
                   and all(len(w) >= 8 for _, w in OUTSIDE_ACCOUNT)
                   and sum(n for _, n in u_real) == u_cross["files"],
                   "真宇宙｜桶 %d／.md %d／条目 %d｜交叉复算 %d｜红 %s" % (
                       len(u_real), sum(n for _, n in u_real), len(OUTSIDE_ACCOUNT),
                       u_cross["files"], universe_verdict(u_real))))
    checks.append(("变异体30 第五本账的五条通道各钉一枚：① 宇宙塌缩必须自己出一条红并把两个 0 点名，"
                   "而同一次调用里的双向判据本身确实恒真（u30_naive==[] ⇒ 塌缩红是唯一牙齿，第 35 型）；"
                   "② 在真语料上插一个没登记的桶只能多出这一条红，且它必须点名桶与份数"
                   "（红一切＝这条账没有登记这一说）；③ 反方向：把 tools 从宇宙里拿掉必须红"
                   "『失效条目』——登记了而桶空了要删条目，不是留个理由供着；"
                   "④ 理由短于 8 字必须红，且这条红来自理由通道而不是漏登记通道（桶仍在宇宙里）；"
                   "⑤ 这条通道不读词汇表——把 FL 换成永不开火的替身，含一条真红的宇宙账必须逐字节不变"
                   "（『不变』的证据是那红在场，不是空串对空串）",
                   len(u30_collapse) == 1 and u30_collapse[0].startswith("集外宇宙塌缩")
                   and "顶层桶 0 个／集外 .md 0 份" in u30_collapse[0] and u30_naive == [],
                   "①%s／naive %s ｜②%d 条 %s ｜③%d 条 %s ｜④%d 条 %s ｜⑤同形 %s／红在场 %s" % (
                       u30_collapse, u30_naive, len(u30_leak), u30_leak,
                       len(u30_stale), u30_stale, len(u30_why), u30_why,
                       pre_u == neu_u, pre_u),
                   ))
    checks.append(("变异体30·甲 ②③④ 三格合起来才叫双向：漏登记恰 1 条且点名 planted_bucket，"
                   "失效恰 1 条且点名 tools，理由不合格恰 1 条且点名 tools；"
                   "三条都必须在真语料（其余格全绿）之上多出这一条，否则任何一条红都可能来自别处",
                   len(u30_leak) == 1 and u30_leak[0].startswith("集外宇宙漏登记")
                   and "planted_bucket" in u30_leak[0] and "7 份" in u30_leak[0]
                   and len(u30_stale) == 1 and u30_stale[0].startswith("集外出账条目已失效")
                   and "tools" in u30_stale[0]
                   and len(u30_why) == 1 and u30_why[0].startswith("集外出账理由不合格")
                   and len(dircheck_verdict(["docs/architecture"], [])) == 0,
                   "②%s ｜③%s ｜④%s" % (u30_leak, u30_stale, u30_why)))
    g_real, g_err = tracked_md()
    g_seen_real = None if g_err else given_up_partition(g_real, [n for n, _ in u_real])
    g_named = set(n for n, _ in GIVEN_UP_ACCOUNT)
    g_real_bad = given_up_verdict(g_seen_real) if g_seen_real is not None else []
    checks.append(("夹具V 第六本账（让出面账）在真语料上必须双向闭合且确有事情可判：git 可用、"
                   "跟踪面非空、让出面非空（让出面为空时『齐备』是零证据，不是判决——第 35 型）、"
                   "未登记为空、失效为空、每条理由≥8 字、账名集合等于实测让出桶集合，"
                   "且让出的份数必须少于跟踪面总数（分区不许把整个仓吞成让出面）",
                   g_err is None and bool(g_real) and bool(g_seen_real)
                   and g_real_bad == [] and set(g_seen_real) == g_named
                   and all(not reason_bad(w) for _, w in GIVEN_UP_ACCOUNT)
                   and sum(len(v) for v in g_seen_real.values()) < len(g_real),
                   "git %s｜跟踪 .md %d 份｜让出桶 %d 个／%d 份／条目 %d｜红 %s" % (
                       g_err, len(g_real or []), len(g_seen_real or {}),
                       sum(len(v) for v in (g_seen_real or {}).values()),
                       len(GIVEN_UP_ACCOUNT), g_real_bad)))
    x_pr = sorted(prune_dirs())
    x_hits = {}
    for _n, _w in DIR_ACCOUNT:
        _d = entry_dirname(_n)
        x_hits[_n] = len([r for r in (g_real or []) if _d in r.split("/")])
    x_scan_last = set(dd.split("/")[-1] for dd in t_dirs)
    x_unexecuted = sorted(n for n in x_hits if entry_dirname(n) in x_scan_last)
    x_real_bad = exclusion_verdict(g_real, t_dirs, DIR_ACCOUNT)
    x_wired_bad = dir_account_audit(t_dirs, len(t_paths), universe=u_real)[0]
    _planted_acc = list(DIR_ACCOUNT) + [("docs/_ghost", "这一条一份也不排，只为把失效通道接到 git 真读上")]
    _orig_acc = globals()["DIR_ACCOUNT"]
    try:
        globals()["DIR_ACCOUNT"] = _planted_acc
        x_stale_wired = dir_account_audit(t_dirs, len(t_paths), universe=u_real)[0]
    finally:
        globals()["DIR_ACCOUNT"] = _orig_acc
    x_unpruned = sorted(set(os.path.dirname(_rel(p)).split("/")[-1] for p in
                            walk_md(ROOTS, prune=frozenset(["_never_exists"]))) & set(x_pr))
    x_skip_rows = {}
    for _n in sorted(OUTSIDE_SKIP):
        _base = os.path.join(REPO, _n)
        _t = len([r for r in (g_real or []) if top_bucket(r) == _n])
        x_skip_rows[_n] = (_t, len(outside_walk(REPO, [_n], [])[0]) if os.path.isdir(_base) else -1)
    x_fs_dropped = sum(v[1] for k, v in x_skip_rows.items() if v[1] > 0 and k not in ROOTS)
    x_dot_dirs = sorted(e for e in os.listdir(REPO)
                        if e.startswith(".") and os.path.isdir(os.path.join(REPO, e)))
    x_dot_fs = sum(len(outside_walk(REPO, [e], [])[0]) for e in x_dot_dirs)
    checks.append(("夹具W 第七本账（出账执行账）在真语料上必须三件都成立：① 每条出账条目在 git 跟踪面里"
                   "真的命中 >0 份（0 命中＝它今天什么都不排，是要删的条目不是要留的理由）；"
                   "② 派生剪枝确实执行了——扫描集的目录名里不许再出现任何一条目声称排除的目录；"
                   "③ 这一本挂在 git 真读那条通道上（用真语料跑整条 dir_account_audit 也必须零红），"
                   "而合成格不参与：合成喂的是让出面专用假名单，拿它判『每条都排掉了东西』会把假名单判成缺陷，"
                   "所以这里同时印『合成格未参与』那一格的实测（0 红不等于没判）。"
                   "分母本身也是断言：条目数、跟踪面、扫描集三者任一为空时上面两条都是恒真（第 35 型）",
                   bool(DIR_ACCOUNT) and bool(g_real) and bool(t_dirs) and bool(t_paths)
                   and x_real_bad == [] and x_wired_bad == [] and x_unexecuted == []
                   and all(v > 0 for v in x_hits.values()) and len(x_hits) == len(DIR_ACCOUNT)
                   and len(x_stale_wired) == 1 and x_stale_wired[0].startswith("出账条目已失效")
                   and bool(x_unpruned) and set(x_unpruned) <= set(x_pr),
                   "条目命中 %s｜派生剪枝 %s｜扫描集残留 %s｜真读整条审计红 %s｜"
                   "植入失效条目经审计印出 %s｜不剪枝时读路重新扫进的目录 %s｜"
                   "合成格未参与（出账执行账只在 git 真读上判）｜"
                   "SKIP 名单双口径（按顶层桶：跟踪／文件系统，-1＝目录不存在）%s｜"
                   "名单在文件系统口径下合计挡下 .md %d 份（ROOTS 已接管者不计＝未跟踪那一圈）｜"
                   "点前缀规则盖 %d 个目录／文件系统 .md %d 份" % (
                       x_hits, x_pr, x_unexecuted, x_wired_bad, x_stale_wired, x_unpruned,
                       x_skip_rows, x_fs_dropped, len(x_dot_dirs), x_dot_fs)))
    x32_unexec = dircheck_verdict(sorted(set(t_dirs) | set(["docs/x/_verification",
                                                             "docs/x/_data"])), DIR_ACCOUNT)
    x32_scope = exclusion_verdict(g_real, t_dirs, [("docs/**/_verification",
                                                   "把范围收窄到 docs 的旧写法，实测命中落在三家桶")]
                                  + [])
    x32_stale = exclusion_verdict(["other/a.md"], t_dirs, [("docs/_ghost", "本轮一份也不排的条目")])
    x32_collapse = exclusion_verdict([], t_dirs, DIR_ACCOUNT)
    checks.append(("变异体32 第七本账的四条通道各钉一枚且各只红自己那条：① 撤销派生剪枝（把扫描集还原成"
                   "从前那副把出账目录扫进来的样子）只能红『出账目录出现在扫描集里』；"
                   "② 把条目名从 `**/_verification` 收窄成 `docs/**/_verification` 只能红『出账范围不符』"
                   "（范围是一句判决，收窄了就必须与实际命中面相符）；③ 条目指向一个本轮跟踪面里不存在的"
                   "目录名只能红『出账条目已失效』；④ 跟踪面读到 0 份只能红『出账执行账塌缩』——"
                   "读到 0 时前三条全在空集上恒真，所以塌缩必须自己成一条红",
                   len(x32_unexec) == 2 and all(r.startswith("出账目录出现在扫描集里") for r in x32_unexec)
                   and len(x32_scope) == 1 and x32_scope[0].startswith("出账范围不符")
                   and len(x32_stale) == 1 and x32_stale[0].startswith("出账条目已失效")
                   and len(x32_collapse) == 1 and x32_collapse[0].startswith("出账执行账塌缩")
                   and len(set([r.split("：")[0] for r in x32_unexec + x32_scope
                                + x32_stale + x32_collapse])) == 4,
                   "①%s ｜②%s ｜③%s ｜④%s" % (x32_unexec, x32_scope, x32_stale, x32_collapse)))
    v31_err = "合成：git 不可用"
    v31_bad, v31_head = dir_account_audit(t_dirs_arg, 12, None, u_synth, ua_synth,
                                          None, ga_synth, v31_err)
    v31_base = dir_account_audit(t_dirs_arg, 12, None, u_synth, ua_synth, g_synth, ga_synth)[0]
    v31_leak = given_up_verdict({"given": ["given/a.md"], "planted": ["planted/a.md"]},
                                [("given", "合成桶已在册，只为让另一只桶单独成红")])
    v31_stale = given_up_verdict({}, [("given", "本轮让出面里没有它的跟踪 .md")])
    v31_why = given_up_verdict({"given": ["given/a.md"]}, [("given", "短")])
    v31_collapse = dir_account_audit(t_dirs_arg, 12, None, u_synth, ua_synth, [], [])[0]
    ua_ov = [("docs", "重叠夹具专用：docs 同时在 ROOTS 与宇宙名单＝同一份文档被两本账各算一次")]
    v31_overlap = dir_account_audit(t_dirs_arg, 12, None, [("docs", 3)],
                                    ua_ov, g_synth, ga_synth)[0]
    checks.append(("变异体31 第六本账的六条通道各钉一枚：① git 不可用必须整格 withheld、"
                   "把原因印进分母行且一条让出面红都不发（不许把没判读成没缺陷），"
                   "而同一次调用的其它两本账照旧要能出红（withheld 不等于闸门停了）；"
                   "② 在册桶之外多一只桶只能多出漏登记这一条并点名桶与份数；③ 反向：桶从让出面里消失"
                   "必须红失效；④ 理由短于阈值必须红且只红理由通道；⑤ 跟踪面 0 份必须自己成一条塌缩红；"
                   "⑥ 扫描集与集外宇宙共享桶名必须红重叠（这一格第一次把 OUTSIDE_SKIP 里 docs 那项的作用量出来）",
                   v31_bad == [] and "withheld" in v31_head and v31_err in v31_head
                   and len(dir_account_audit([], 0, None, u_synth, ua_synth,
                                             None, ga_synth, v31_err)[0]) == 1
                   and v31_base == [] and len(v31_leak) == 1 and len(v31_stale) == 1
                   and len(v31_why) == 1 and len(v31_collapse) == 1
                   and v31_collapse[0].startswith("让出面宇宙塌缩")
                   and len(v31_overlap) == 1 and v31_overlap[0].startswith("扫描集与集外宇宙桶名重叠")
                   and "docs" in v31_overlap[0],
                   "①withheld 红 %s／头 %s／塌缩同格 %s ｜②基线 %s ｜③%s ｜④%s ｜⑤%s ｜⑥%s" % (
                       v31_bad, v31_head,
                       dir_account_audit([], 0, None, u_synth, ua_synth,
                                         None, ga_synth, v31_err)[0],
                       v31_leak, v31_stale, v31_why, v31_collapse, v31_overlap)))
    checks.append(("变异体31·甲 ②③④⑤⑥ 五格必须两两红在不同通道上且各自恰一条："
                   "漏登记点名 planted 而不点名 given、失效点名 given、理由红只来自理由通道"
                   "（桶仍在让出面上）、塌缩红来自份数而非登记、重叠红点名 docs；"
                   "任何一格多出别家的红，就说明这两本账在同一条通道上互相顶（第 74 条⑵）",
                   len(v31_leak) == 1 and v31_leak[0].startswith("让出面漏登记")
                   and "planted" in v31_leak[0] and "1 份" in v31_leak[0]
                   and len(v31_stale) == 1 and v31_stale[0].startswith("让出面条目已失效")
                   and "given" in v31_stale[0]
                   and len(v31_why) == 1 and v31_why[0].startswith("让出面理由不合格")
                   and len(v31_collapse) == 1 and v31_collapse[0].startswith("让出面宇宙塌缩")
                   and len(v31_overlap) == 1 and v31_overlap[0].startswith("扫描集与集外宇宙桶名重叠")
                   and len(set([r.split("：")[0] for r in v31_leak + v31_stale + v31_why
                                + v31_collapse + v31_overlap])) == 5,
                   "②%s ｜③%s ｜④%s ｜⑤%s ｜⑥%s" % (
                       v31_leak, v31_stale, v31_why, v31_collapse, v31_overlap)))
    def _v33_dup():
        acc = list(DIR_ACCOUNT) + [("docs/**/node_modules",
                                    "只为把『名单不许重打』这条通道接到目录账上的合成条目")]
        orig = globals()["DIR_ACCOUNT"]
        try:
            globals()["DIR_ACCOUNT"] = acc
            return reco_exclusion_verdict([("node_modules", 0, 0, "", 0)], 5,
                                          ["node_modules"], ["node_modules"])
        finally:
            globals()["DIR_ACCOUNT"] = orig

    e_rows, e_base, e_skip = reco_exclusion_account()
    e_bad = reco_exclusion_verdict(e_rows, e_base, RECO_OUTSIDE_NESTED, RECO_GENERATED_NESTED)
    e_derived = sorted(set(prune_dirs()) & set(RECO_OUTSIDE_NESTED))
    e_only = set(reco_outside_paths(REPO, RECO_GENERATED_NESTED)[0])
    e_all = set(reco_outside_paths(REPO, RECO_OUTSIDE_NESTED)[0])
    e_hidden = sorted(e_all ^ e_only)
    # 合并的代价不写成"候选集相同"：那个基线要重打目录账的名字才是第二源。这里只判方向——
    # 新挡掉的份数照印，但挡掉的里面不许有一把尺。
    e_hidden_hits = (len(reco_audit([(p, _read_code(os.path.join(REPO, p))) for p in e_hidden])["foreign"])
                     if e_hidden else 0)
    e33_hidden = reco_exclusion_verdict([("a", 1, 2, "scripts/x/a.py:3", 1)], 5, ["a"], [])
    e33_short = reco_exclusion_verdict([("a", 0, 0, "", 0)], 5, ["a", "b"], [])
    e33_zero = reco_exclusion_verdict([("a", 0, 0, "", 0)], 0, ["a"], [])
    e33_empty = reco_exclusion_verdict([], 5, [], RECO_GENERATED_NESTED)
    e33_dup = _v33_dup()
    checks.append(("夹具X 第八本账（嵌套排除代价账）在真语料上必须四件都成立：① 名单每名都有一行反事实"
                   "读数（行数＝名单长度）；② 代价账整条零红；③ 目录账派生名确实进了代码名单（交集＝prune_dirs 全体）"
                   "而代码侧那份字面量与目录账不相交（重打＝第二源）；"
                   "④ 并入目录账派生名的代价必须可见且不含尺：只走构建派生字面量与走并好的名单，"
                   "两副名单集外候选的对称差＝合并新挡掉的那几份（份数由这一格现印，不进散文），"
                   "其中不许有任何自带算术记号的文件——合并只许挡掉派生副本，不许挡掉一把尺"
                   "（22:15 普查量的『并入前后候选集逐份相同』是对**旧的字面量名单**做的对照，"
                   "仪器内无法在不重打目录账名字的前提下复现那个基线，而重打它正是本轮要退役的第二源，"
                   "所以这里判的是方向，不是那次的差额）",
                   len(e_rows) == len(set(RECO_OUTSIDE_NESTED)) and e_base > 0
                   and e_bad == [] and set(e_derived) == set(prune_dirs())
                   and RECO_GENERATED_NESTED.isdisjoint(prune_dirs()) and e_hidden_hits == 0,
                   "名单 %d 名／集外基线 %d 份／少走目录 %d 个｜逐名（名单, 放出份数, 算术命中, 少走的目录）%s｜"
                   "目录账派生名 %s｜合并新挡 %d 份／其中算术命中 %d 处｜代价账红 %s" % (
                       len(set(RECO_OUTSIDE_NESTED)), e_base, e_skip,
                       [(r[0], r[1], r[2], r[4]) for r in e_rows],
                       e_derived, len(e_hidden), e_hidden_hits, e_bad)))
    checks.append(("变异体33 第八本账的五条通道各钉一枚且各只红自己那条：① 撤掉某个名字会放出带算术记号的"
                   "候选只红隐身并点名文件行号；② 名单少一行只红账塌缩；③ 集外候选读到 0 份只红分母塌缩"
                   "（第 35 型：0 份时逐名读数与隐身判据都在空集上恒真）；④ 名单整体为空只红名单塌缩；"
                   "⑤ 目录账的名字被重打在代码侧字面量里只红第二源——五枚红的前缀两两不同，"
                   "任何一枚多红别家就说明两条通道共用一条判据",
                   len(e33_hidden) == 1 and e33_hidden[0].startswith("嵌套排除挡住了自带算术记号的候选")
                   and len(e33_short) == 1 and e33_short[0].startswith("排除代价账塌缩")
                   and len(e33_zero) == 1 and e33_zero[0].startswith("排除代价账分母塌缩")
                   and len(e33_empty) == 1 and e33_empty[0].startswith("嵌套排除名单塌缩")
                   and len(e33_dup) == 1 and e33_dup[0].startswith("代码名单重打了目录账的名字")
                   and len(set([r.split("（")[0] for r in e33_hidden + e33_short + e33_zero
                                + e33_empty + e33_dup])) == 5,
                   "①%s ｜②%s ｜③%s ｜④%s ｜⑤%s" % (
                       e33_hidden, e33_short, e33_zero, e33_empty, e33_dup)))
    # 夹具Y／变异体34：第十本账（§1.8p）——集外侧 .md 那份嵌套名单自己的代价账。
    # 名单在 10-02 之前只在普查里量过「挡了多少目录」，从没进过判决：撤掉一名会放出几份 .md、
    # 少走几个目录，此前只能靠人肉走一遍。这里把它收进和 第八本账 同一形状的逐名反事实。
    o_rows, o_base, o_skip = outside_exclusion_account()
    o_bad = outside_exclusion_verdict(o_rows, o_base, OUTSIDE_NESTED, OUTSIDE_SKIP)
    o_census_files = outside_census()["files"]
    o_ignited = {r[0] for r in o_rows if r[1] or r[2]}
    o_must_fire = set(OUTSIDE_NESTED) - set(OUTSIDE_SKIP)
    o_bleed = sum(r[4] for r in o_rows)
    checks.append(("夹具Y 第十本账（集外 .md 嵌套排除代价账）在真语料上必须五件都成立："
                   "① 名单每名都有一行反事实读数（行数＝名单长度）且集外基线非空；② 代价账整条零红；"
                   "③ 这份名单不重打目录账的名字（与 prune_dirs 不相交——那份排除只能由 DIR_ACCOUNT 条目名派生）；"
                   "④ 顶层名单没覆盖的名字必须真点火（放出份数≠0 或少走目录≠0），只在不与 OUTSIDE_SKIP 重叠时要求："
                   "重叠的那几名是防御项，撤与不撤由顶层名单兜着，本账不判它们死；"
                   "⑤ 逐名反事实的基线与普查那把尺读到同一份集外分母（%d 份＝%d 份，不等说明代价账偷偷换了扫描口径），"
                   "且放出的路径里落在 docs/ 名下的必须 0 条（越界就不叫集外代价而叫集内缺口）"
                   % (o_base, o_census_files),
                   len(o_rows) == len(set(OUTSIDE_NESTED)) and o_base > 0 and o_bad == []
                   and set(OUTSIDE_NESTED).isdisjoint(prune_dirs())
                   and o_must_fire <= o_ignited and o_base == o_census_files and o_bleed == 0,
                   "名单 %d 名／集外基线 %d 份（普查那把尺 %d 份）／少走目录 %d 个｜"
                   "逐名（名单, 放出份数, 少走目录, 放出里集内条数）%s｜顶层名单没覆盖须点火的 %s／实际点火 %s｜"
                   "与目录账交集 %s｜代价账红 %s" % (
                       len(set(OUTSIDE_NESTED)), o_base, o_census_files, o_skip,
                       [(r[0], r[1], r[2], r[4]) for r in o_rows],
                       sorted(o_must_fire), sorted(o_ignited),
                       sorted(set(OUTSIDE_NESTED) & set(prune_dirs())), o_bad)))

    v34 = [outside_exclusion_verdict(*args) for _p, _anch, args in OUT_EX_CASES]
    checks.append(("变异体34 第十本账的六条通道各钉一枚且各只红自己那条：① 名单整体为空只红名单塌缩；"
                   "② 少一行反事实读数只红账塌缩（那一名没有代价记录）；③ 集外基线读到 0 份只红分母塌缩"
                   "（第 35 型：0 份时逐名读数与恒等式都在空集上恒真）；④ 一名既不放文件也不减目录、"
                   "顶层名单也不含它只红孤儿名；⑤ 名单里出现目录账的名字只红第二源；"
                   "⑥ 反事实放出的路径落在 docs/ 名下只红越界——六枚红各归自己的前缀且前缀两两不同，"
                   "任何一枚多红别家就说明两条通道共用一条判据",
                   all(len(r) == 1 and r[0].startswith(p)
                       for r, (p, _a, _n) in zip(v34, OUT_EX_CASES))
                   and len(set([r[0].split("（")[0] for r in v34 if r])) == len(OUT_EX_CASES),
                   " ／ ".join("%s" % (r,) for r in v34)))

    z_notes, z_detail = _ex_witness()
    checks.append(("夹具Z 第十本账的六枚针必须各自穿孔：把判决函数从本文件源码现抠（抠到的不是恰好一个函数就红、"
                   "抠到的红模板条数≠通道表条数也红、锚点在源码里不唯一或撤不动同样红——见证自己坏了不许静默跳过），"
                   "再在内存里逐条 `and False` 短路：撤第 k 条只许第 k 枚针哑、其余五枚照红，六次都得对上。"
                   "（承 第 34 型：撤了通道而对应那枚针不红＝针根本没穿这条通道，不是判决变得更稳；"
                   "_ex_witness 收 src 参数＝这台见证自己的三条守卫可以在不改仓内文件的前提下逐条撞红，"
                   "撞法见 §1.8p；全程 compile＋exec 在内存里跑，一个字节都不写盘）",
                   z_notes == [], z_detail))

    # 夹具AA：夹具Z 自己的牙。把它的判决函数源码现抠出来当注入面（表里那份锚点字符串不在这一段里，
    # 所以每处坏法在段内都只命中 1 次——不检查这一条就会造出「replace 打了个空炮而格子照样绿」的假见证）。
    # 「同一条分支复制两份」这一型在构造上够不着：锚点含 bad.append("集外 .md ，复制它就同时把红模板条数抄错，
    # 于是被前一条守卫先接住（顺序决定哪格被见证），所以这里只留命中 0 次那一侧（戊）。
    aa_src = _read_code(os.path.abspath(__file__))
    aa_fns = [n for n in ast.parse(aa_src).body if isinstance(n, ast.FunctionDef)
              and n.name == OUT_EX_HOME_FN]
    aa_seg = ast.get_source_segment(aa_src, aa_fns[0]) if len(aa_fns) == 1 else ""
    aa_out = [] if len(aa_fns) == 1 else [
        "判决函数在盘上源码里不是恰好 1 个（现量 %d）＝注入面抠不出来，四型坏法全都没跑" % len(aa_fns)]
    AA = [
        ("甲 有一条红的开头被改写（通道表还在、源码里的红模板条数对不上）",
         'bad.append("%s' % OUT_EX_CASES[5][0], 'bad.append("ZZ%s' % OUT_EX_CASES[5][0],
         "红模板现量 5 条"),
        ("乙 判决函数被改名（抠不到＝见证必须作废，不许静默通过）",
         "def %s(" % OUT_EX_HOME_FN, "def %s_x(" % OUT_EX_HOME_FN, "不是恰好 1 个"),
        ("丁 有一条分支永远不成立（对照打不红＝基线不对就不许往下撤）",
         OUT_EX_CASES[2][1].split("\n")[0], "    if False:", "未变异基线不是每枚各红 1 条"),
        ("戊 有一条分支换了写法（锚点命中 0 次＝撤错分支或一条没撤）",
         OUT_EX_CASES[3][1].split("\n")[0], OUT_EX_CASES[3][1].split("\n")[0] + "  # 写法变了",
         "命中 0 次"),
    ]
    for label, old, new, want in AA:
        if aa_seg.count(old) != 1:
            aa_out.append("%s：注入锚点在判决源码里命中 %d 次＝这一例改到的可能不是我要改的那处"
                          % (label[:2], aa_seg.count(old)))
            continue
        mut = aa_seg.replace(old, new, 1)
        if mut == aa_seg:
            aa_out.append("%s：注入没落地（replace 打了空炮）" % label[:2])
            continue
        nts, _det = _ex_witness(mut)
        if not nts:
            aa_out.append("%s 坏法注入后夹具Z 仍然 notes=[]＝这台见证没有牙" % label[:2])
        elif not any(want in x for x in nts):
            aa_out.append("%s 红了但不是因为预期的那条守卫（期望含「%s」，现量 %s）"
                          % (label[:2], want, nts))
    checks.append(("夹具AA 夹具Z 的四型坏法必须各撞红自己那条守卫——甲 红模板条数对不上／乙 抠不到函数／"
                   "丁 对照打不红（基线先作废）／戊 锚点命中 0 次；"
                   "注入前先断言锚点在段内恰好 1 次且 replace 真落地（否则＝静默 no-op 的假见证），"
                   "注入后还要求 notes 非空**且**红的原因是预期那条（红错原因也算没牙）。"
                   "全部只改内存里那份判决源码：夹具L 扫的是盘上文件，所以这一格不会顺手给自己配出六枚新针",
                   aa_out == [], "四型各撞红预期守卫 ／ 异常 %s" % aa_out))
    BATTERY = [
        ("第二把尺", lambda: reco_verdict(reco_audit(
            [HOME_STUB, ("scripts/gate/second_fw.py", SECOND_FW)]))),
        ("归宿文件不在扫描集", lambda: reco_verdict(
            reco_audit([("scripts/gate/second_fw.py", SECOND_FW)]))),
        ("判决集塌缩", lambda: reco_verdict(reco_audit([(RECO_HOME, "import re\n")]))),
        ("解析失败", lambda: reco_verdict(reco_audit(
            [HOME_STUB, ("scripts/gate/broken.py", "def (:\n")]))),
        ("集外候选读到 0 份代码文件", lambda: reco_outside_audit(items=[])["bad"]),
        ("第二把尺在扫描集外", lambda: reco_outside_audit(
            items=[("scripts/probes/planted2.py", SECOND_FW)])["bad"]),
        ("代码名单重打了目录账的名字", _v33_dup),
        ("嵌套排除名单塌缩", lambda: reco_exclusion_verdict([], 5, [], RECO_GENERATED_NESTED)),
        ("排除代价账塌缩", lambda: reco_exclusion_verdict(
            [("a", 0, 0, "", 0)], 5, ["a", "b"], [])),
        ("排除代价账分母塌缩", lambda: reco_exclusion_verdict(
            [("a", 0, 0, "", 0)], 0, ["a"], [])),
        ("嵌套排除挡住了自带算术记号的候选", lambda: reco_exclusion_verdict(
            [("a", 1, 2, "scripts/x/a.py:3", 1)], 5, ["a"], [])),

        ("集外主张不闭合", lambda: outside_verdict(_scan_paths([unc_p]))),
        ("集外探针读到 0 份 .md", _doc_outside_collapse),
        ("理由不合格", lambda: dircheck_verdict([], [("空理由", "")])),
        ("出账目录出现在扫描集里", lambda: dircheck_verdict(["docs/_archive"])),
        ("出账条目已失效", lambda: exclusion_verdict(["other/a.md"], [],
                                                    [("docs/_ghost", "理由够长够长够长")])),
        ("出账范围不符", lambda: exclusion_verdict(
            ["frontend-ui/x/_verification/y.md"], [],
            [("docs/**/_verification", "验证产物属派生副本，权威在源文档与代码本身")])),
        ("出账执行账塌缩", lambda: exclusion_verdict([], [], [])),
        ("CI 扫描集塌缩", lambda: ci_wiring_verdict(
            [], ["python %s %s" % (CI_WIRED[0], CI_SELFTEST_ARG)])),
        ("CI 未接线", lambda: ci_wiring_verdict(
            [(1, "scripts/gate/other_gate.py")],
            ["python scripts/gate/other_gate.py %s" % CI_SELFTEST_ARG])),
        ("CI 接线未带", lambda: ci_wiring_verdict(
            [(1, CI_WIRED[0]), (2, CI_WIRED[1])],
            ["python %s --census" % CI_WIRED[0], "python %s --recognizer" % CI_WIRED[1]])),
        ("CI 声称账塌缩", lambda: ci_claim_verdict(
            [], set(["scripts/gate/other.py"]), [3], [(4, "docs/x.md")])),
        ("CI 声称未接线", lambda: ci_claim_verdict(
            [(5, "scripts/gate/ghost.py")], set(["scripts/gate/other.py"]), [], [])),
        ("门禁脚本未登记在维护指南", lambda: gate_scripts_unlisted(
            set(["scripts/gate/a.py"]), "# 指南里没有它\n")[0]),
        ("反向账塌缩", lambda: gate_scripts_unlisted(set(), "# 什么都在跑\n")[0]),
        ("一键账塌缩", lambda: one_click_audit(["scripts/gate/a.py"], "# 一条也没跑\n", "")[0]),
        ("CI 门禁未进一键", lambda: one_click_audit(
            ["scripts/gate/a.py", "scripts/gate/b.py"], '& python "scripts/gate/a.py"\n', "")[0]),
        ("一键分母账塌缩", lambda: one_click_audit(
            ["scripts/gate/a.py"], '& python "scripts/gate/a.py"\n', "")[0]),
        ("一键分母不合一：段标签的分母", lambda: one_click_audit(
            ["scripts/gate/a.py"], '[1/7] x\n& python "scripts/gate/a.py"\n', "")[0]),
        ("一键分母不合一：变量", lambda: one_click_audit(
            ["scripts/gate/a.py"], '[1/$Total]\n[2/$Total]\n& python "scripts/gate/a.py"\n', "")[0]),
        ("一键分母不合一：", lambda: one_click_audit(
            ["scripts/gate/a.py"], '$Total = 3\n[1/$Total]\n[2/$Total]\n& python "scripts/gate/a.py"\n', "")[0]),
        ("一键项数与文档不符", lambda: one_click_audit(
            ["scripts/gate/a.py"],
            '$Total = 2\n[1/$Total]\n[2/$Total]\n& python "scripts/gate/a.py"\n',
            "python %s 一键（9 项）\n" % ONE_CLICK_DOC)[0]),
        ("一键参数账塌缩", lambda: one_click_arg_parity(
            [("scripts/gate/b.py", (), "ci.yml", 1)], '& python "scripts/gate/a.py"\n',
            ["scripts/gate/a.py", "scripts/gate/b.py"], ["scripts/gate/a.py"])[0]),
        ("一键参数不合一", lambda: one_click_arg_parity(
            [("scripts/gate/a.py", ("--check",), "ci.yml", 1)],
            '& python "scripts/gate/a.py" --selftest\n', ["scripts/gate/a.py"],
            ["scripts/gate/a.py"])[0]),
        ("目录账分母塌缩", lambda: dir_account_audit([], 0, None, [("planted", 1)],
                                                      [("planted", "合成桶，保证这一格只红塌缩一条")],
                                                      g_synth, ga_synth)[0]),
        ("集外宇宙塌缩", lambda: dir_account_audit(["docs/architecture"], 5, None, [], [],
                                                   g_synth, ga_synth)[0]),
        ("集外宇宙漏登记", lambda: universe_verdict([("planted", 4)], [])),
        ("集外出账条目已失效", lambda: universe_verdict([], [("ghost", "本轮宇宙里读不到这个桶")])),
        ("集外出账理由不合格", lambda: universe_verdict([("planted", 3)], [("planted", "短")])),
        ("让出面漏登记", lambda: given_up_verdict({"planted": ["planted/a.md"]}, [])),
        ("让出面条目已失效", lambda: given_up_verdict({}, [("ghost", "本轮让出面里没有它的跟踪 .md")])),
        ("让出面理由不合格", lambda: given_up_verdict({"given": ["given/a.md"]}, [("given", "短")])),
        ("让出面宇宙塌缩", lambda: dir_account_audit(["docs/architecture"], 5, None, u_synth,
                                                     ua_synth, [], [])[0]),
        ("扫描集与集外宇宙桶名重叠", lambda: dir_account_audit(["docs/architecture"], 5, None,
                                                               [("docs", 3)], ua_ov,
                                                               g_synth, ga_synth)[0]),
        ("盲区读账分母塌缩", lambda: blind_read_corpus_verdict(0, ["docs/a.md"])),
        ("盲区嫌疑未点名", lambda: blind_read_verdict(
            "docs/a.md", [dict(line=3, span=_NEEDLE_CLOSED)], {})),
        ("盲区嫌疑未闭合", lambda: blind_read_verdict(
            "docs/a.md", [dict(line=3, span=_NEEDLE_UNCLOSED)],
            {_suspect_key("docs/a.md", dict(line=3, span=_NEEDLE_UNCLOSED)): "真算术"})),
    ]
    # 第十本账的六枚针由通道表派生（同一条对照既打红变异体34，也当 夹具L 的针）——
    # 在 BATTERY 里再手抄一遍就会有两份会各自腐烂的对照。
    BATTERY += [(p, (lambda a=args: outside_exclusion_verdict(*a))) for p, _anch, args in OUT_EX_CASES]
    clean_dir = dircheck_verdict(["docs/architecture"], [("架构目录", "L2 架构文，一条主题一个目录")])
    covered, dead, errs = {}, [], []
    for key, f in BATTERY:
        reds, exc = _battery_run(f)
        errs += exc
        hit = [t for t in tmpls if t.startswith(key) and any(r.startswith(t) for r in reds)]
        if not hit:
            dead.append(key)
        for t in hit:
            covered.setdefault(t, []).append(key)
    shutil.rmtree(tmp3, ignore_errors=True)
    uncovered = [t for t in tmpls if t not in covered]
    checks.append(("夹具L 每条判决都要有针：红模板从本文件源码现取（不写清单），逐条要求一枚内存对照能打红它——"
                   "新增一条红而没配对照＝当场多出一格未覆盖，撤掉对照＝当场多出一格死键",
                   len(tmpls) >= 6 and not uncovered and not dead and not errs and clean_dir == [],
                   "红模板 %d 条 ／ 未覆盖 %s ／ 死键 %s ／ 异常 %s ／ 合法目录账不误报 %s" % (
                       len(tmpls), uncovered, dead, errs, clean_dir)))
    # 单源必须被"拧单源"打破：把 FL 的前缀门限抬高到没有任何截短头能过关，本文件的红模板宇宙
    # 必须跟着变（变成整串，里头带占位符）。拧不动＝这里还藏着第二把尺，明天的改动只落在那把没人看的尺上。
    orig_min = FL.OUTLET_MIN_PREFIX
    FL.OUTLET_MIN_PREFIX = 400
    try:
        knob = FL.red_outlet_universe(_read_code(os.path.abspath(__file__)), "append")
    finally:
        FL.OUTLET_MIN_PREFIX = orig_min
    checks.append(("变异体35 红模板抽取只有单源一个实现：抬高 FL 的前缀门限必须改动本文件的宇宙，"
                   "且还原后门限回到原值（宇宙按名点到几处不由这里判——上面那格夹具L 管）",
                   knob != tmpls and any("%" in t for t in knob)
                   and FL.OUTLET_MIN_PREFIX == orig_min,
                   "门限 %d→400→%d ／ 宇宙 %d 条→%d 条 ／ 拧后含占位符 %d 条" % (
                       orig_min, FL.OUTLET_MIN_PREFIX, len(tmpls), len(knob),
                       sum(1 for t in knob if "%" in t))))
    fails = 0
    for name, ok, detail in checks:
        print("%s %s  %s" % ("PASS" if ok else "FAIL", name, detail))
        if not ok:
            fails += 1
    print("SELFTEST 总 %d 例，FAIL %d 例" % (len(checks), fails))
    return 1 if fails else 0


def cmd_blindread(args):
    """第九本账的出版口：把盲区**读**出来——读＝逐条点名＋给判决，不是印一个分档数。
    分档数（glued=…／latin=…）回答的是「盲区有多少」，回答不了「盲区里有没有藏着错账」，
    所以嫌疑要解除遮蔽后交回唯一算源再判一次，判成真算术而不闭合的红在这里出。"""
    paths = walk_md(args.root)
    total, by_reason, suspects, bad = 0, {}, [], []
    for p in paths:
        rel, sp = _scan_file(p)
        total += len(sp["blind"])
        for b in sp["blind"]:
            by_reason[b["reason"]] = by_reason.get(b["reason"], 0) + 1
        adj = {}
        for b in sp["blind"]:
            span = b.get("span") or ""
            if not FL.EQ_FW_RE.search(span) or not (FL.ADD_RE.search(span) or FL.MUL_RE.search(span)):
                continue
            verdict = "非算术"
            for v in _unblind_variants(span):
                if FL.scan_text(v)["claims"]:
                    verdict = "真算术"
                    break
            adj[_suspect_key(rel, b)] = verdict
            suspects.append((rel, b.get("line"), span[:70], verdict))
        bad += blind_read_verdict(rel, sp["blind"], adj)
    bad += blind_read_corpus_verdict(total, paths)
    print("盲区读账 扫描 .md %d 份／盲区 %d 条／嫌疑 %d 条" % (len(paths), total, len(suspects)))
    print("盲区分档 " + "／".join("%s=%d" % (k, by_reason[k]) for k in sorted(by_reason)))
    for rel, line, span, verdict in suspects:
        print("嫌疑 %s:%d 「%s」判决 %s" % (rel, line, span, verdict))
    for b in bad:
        print("FAIL %s" % b)
    print("BLINDREAD 盲区 %d 条，嫌疑 %d 条，红 %d 条" % (total, len(suspects), len(bad)))
    return 1 if bad else 0


def main(argv=None):
    ap = argparse.ArgumentParser(description="全 docs/ 核心公式普查与复算")
    ap.add_argument("--census", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--dircheck", action="store_true")
    ap.add_argument("--dir-account", action="store_true", dest="dir_account")
    ap.add_argument("--recognizer", action="store_true")
    ap.add_argument("--blindread", action="store_true")
    ap.add_argument("--json", dest="json")
    ap.add_argument("--root", action="append", default=None)
    args = ap.parse_args(argv)
    if args.root is None:
        args.root = list(ROOTS)
    if args.selftest:
        return cmd_selftest(args)
    if args.dircheck or args.dir_account:
        return cmd_dircheck(args)
    if args.recognizer:
        return cmd_recognizer(args)
    if args.blindread:
        return cmd_blindread(args)
    return cmd_census(args)


if __name__ == "__main__":
    sys.exit(main())
