#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-locale-format-outlets.py —— 前端时间/日历显示口径门禁（FE-LOCALE-GOV-V1.0 执行门禁）

权威规范：docs/architecture/frontend/FRONTEND-MODULE-GOVERNANCE-v1.0.md §5.10／§5.27／§5.30／§5.31／§5.32
本门禁的由来：那三轮普查的 needle 只写了 `toLocaleString`，把 `toLocaleDateString` /
`toLocaleTimeString` 两族整整 14 处带参调用漏在尺子外面（§5.31）。本门禁把 needle 换成
**整个 `toLocale*` 家族**，并把"登记口只许一处"从人工复算变成机器判据，防止盲区再次打开。

检查项（frontend-ui/src 下的 .vue/.js/.ts，排除 *.test.js 与 *.stories.js）：
  L1. UNPINNED_DATE  时间接收者上的无参 locale 格式化 = 0（输出随访问者浏览器区域漂移）。
     接收者判定：同一行 `new Date(...)`，或调用点左侧的变量在本文件里由 `X = new Date(` 赋值而来。
     存量余账见 EXPECTED_UNPINNED（棘轮：新增站点或计数增长都是 ERROR；清零要删条目而不是写 0）。
  L2. OUTLET_LITERAL  出口写法只许活在登记口 utils/time.js：
     `{ hour: '2-digit', minute: '2-digit' }`、`toLocaleString('zh-CN', { hour12: false })`、
     `{ hour: '2-digit', minute: '2-digit', second: '2-digit' }` 三个串各只许出现在该文件。
     注意 L2 钉的是**登记口的完整写法**，不是任意选项片段——
     `{ hour12: false }` 单独出现是合法的别的档（首版把整族 hour12 都判成违规，
     在 AdminMonitor 的带秒档上假红，见 §5.32）。
  L3. LOCALE_PIN      带参调用必须 pin 到 'zh-CN'；出现其它 locale = ERROR（期望 0）。
  L4. SCAN_SET        被扫文件数必须 ≥ MIN_FILES；判集塌缩时"0 命中"与"没有缺陷"同形，
     所以 selftest 里有一枚"不存在的仓库根必须被拒绝"的正对照。
  I1. 数字千分位的无参 `toLocaleString()`（与时间无关）仅 INFO 点名，不判定——豁免必须按名打印。
  I2. 登记口外的带参 `toLocale*` 调用点按文件 INFO 列出（＝绕开出口的直写余账，task #25）。

用法：
  python scripts/gate/check-locale-format-outlets.py            # 扫描 + 判定
  python scripts/gate/check-locale-format-outlets.py --selftest # 合成语料：缺陷必须红 + 合法必须不红 + 空判集必须被拒
退出码：0 = 无 ERROR；1 = 存在 ERROR 或 selftest 有未落地判据。
"""
from __future__ import annotations

import argparse
import os
import re
import sys

MIN_FILES = 100
REGISTRY = "utils/time.js"
OUTLET_LITERALS = ["{ hour: '2-digit', minute: '2-digit' }", "toLocaleString('zh-CN', { hour12: false })",
                   "{ hour: '2-digit', minute: '2-digit', second: '2-digit' }"]
# 存量余账（2026-09-29 实测，坐标见 reports/data/locale-clock-manifest.txt 与 locale-family-census.txt）
EXPECTED_UNPINNED = {
    "modules/admin-lowcode/engine/widgetRegistry.js": 1,
    "views/admin/panels/AdminHitl.vue": 1,
}

CALL = re.compile(r"\.(toLocale(?:String|DateString|TimeString))\s*\(([^()]*)\)", re.S)
DATE_VAR = re.compile(r"\b(\w+)\s*=\s*new Date\(")


def classify(rel, text):
    """返回 (errors, unpinned 计数, 数字千分位行, 绕开出口的带参调用行)。纯函数，selftest 直接喂合成语料。"""
    errs = []
    unpinned = 0
    numgroup = []
    direct = []
    date_vars = set(DATE_VAR.findall(text))
    for m in CALL.finditer(text):
        method, args = m.group(1), m.group(2).strip()
        line = text.count("\n", 0, m.start()) + 1
        head = text[:m.start()].rsplit("\n", 1)[-1]
        if not args:
            prev = re.search(r"([A-Za-z_$][\w$]*)\s*$", head)
            is_time = ("new Date" in head) or (prev is not None and prev.group(1) in date_vars)
            if is_time:
                unpinned += 1
            else:
                numgroup.append("%s:%d %s" % (rel, line, (head + "." + method + "()").strip()[:88]))
            continue
        mm = re.match(r"^['\"]([\w-]+)['\"]", args)
        if mm and mm.group(1) != "zh-CN":
            errs.append("L3 LOCALE_PIN %s:%d pin 到了 `%s` 而非 zh-CN" % (rel, line, mm.group(1)))
        for lit in OUTLET_LITERALS:
            if lit in args:
                errs.append("L2 OUTLET_LITERAL %s:%d 在登记口之外重复了出口写法片段 `%s`" % (rel, line, lit))
        if rel != REGISTRY:
            direct.append("%s:%d" % (rel, line))
    if rel != REGISTRY:
        for lit in OUTLET_LITERALS:
            if lit in text:
                errs.append("L2 OUTLET_LITERAL %s 出现出口写法串 `%s`" % (rel, lit))
    return errs, unpinned, numgroup, direct


def collect(repo_root):
    src = os.path.join(repo_root, "frontend-ui", "src")
    files = []
    for root, dirs, names in os.walk(src):
        dirs[:] = [d for d in dirs if d not in ("node_modules", ".vite")]
        for fn in names:
            if not fn.endswith((".vue", ".js", ".ts")):
                continue
            if fn.endswith((".test.js", ".stories.js")):
                continue
            files.append(os.path.join(root, fn))
    return src, sorted(files)


def check(repo_root, quiet=False):
    src, files = collect(repo_root)
    def emit(s):
        if not quiet:
            print(s)
    if len(files) < MIN_FILES:
        emit("[ERROR] L4 SCAN_SET 被扫文件 %d < 下限 %d（判集塌缩，0 命中不算证据）:: %s"
             % (len(files), MIN_FILES, src))
        return 1
    errs = []
    unp_total, unp_by, num_lines, direct_lines = 0, {}, [], []
    for p in files:
        rel = os.path.relpath(p, src).replace("\\", "/")
        text = open(p, encoding="utf-8", errors="replace").read()
        e, u, nl, dl = classify(rel, text)
        errs += e
        if u:
            unp_total += u
            unp_by[rel] = unp_by.get(rel, 0) + u
        num_lines += nl
        direct_lines += dl
    extra = {k: v for k, v in unp_by.items() if k not in EXPECTED_UNPINNED}
    grown = {k: (v, EXPECTED_UNPINNED[k]) for k, v in unp_by.items()
             if k in EXPECTED_UNPINNED and v > EXPECTED_UNPINNED[k]}
    gone = {k: v for k, v in EXPECTED_UNPINNED.items() if k not in unp_by}
    for k, v in sorted(extra.items()):
        errs.append("L1 UNPINNED_DATE 新增站点 %s（%d 处）：时间接收者上的无参 locale 格式化，输出随浏览器区域漂移" % (k, v))
    for k, (now, exp) in sorted(grown.items()):
        errs.append("L1 UNPINNED_DATE 棘轮回涨 %s：%d > 登记 %d" % (k, now, exp))
    if errs:
        for e in errs:
            emit("[ERROR] " + e)
    else:
        emit("[OK] L1/L2/L3 无违规：扫描 %d 个文件；无参时间格式化余账 %d 处＝登记表；出口写法只在 %s"
             % (len(files), unp_total, REGISTRY))
    emit("[INFO] L1 余账明细（清零后要删条目而不是写 0）：%s"
         % (", ".join("%s x%d" % kv for kv in sorted(unp_by.items())) or "空"))
    if gone:
        emit("[SHRANK] 以下站点已不再命中（请把条目从 EXPECTED_UNPINNED 删掉）：%s"
             % ", ".join("%s x%d" % kv for kv in sorted(gone.items())))
    emit("[INFO] I1 数字千分位（与时间无关，不判定，按名点名 %d 处）：%s"
         % (len(num_lines), "; ".join(num_lines)))
    emit("[INFO] I2 登记口外的带参 toLocale* 直写 %d 处（task #25 余账）：%s"
         % (len(direct_lines), "; ".join(sorted(direct_lines))))
    return 1 if errs else 0


def selftest():
    """合成语料：每枚缺陷样例必须被自己的规则点名；每枚合法样例必须不红；空判集必须被拒绝。"""
    cases = [
        ("L1 无参 + 同行 new Date", "a/views/X.vue", "const t = new Date(ts).toLocaleString()", True),
        ("L1 无参 + 别名 Date 变量", "a/views/Y.vue", "const d = new Date(t)\nreturn d.toLocaleString()", True),
        ("L2 登记口外重复时钟档选项", "a/views/Z.vue",
         "new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })", True),
        ("L2 登记口外重复完整 F2 写法", "a/views/W.vue",
         "new Date().toLocaleString('zh-CN', { hour12: false })", True),
        ("L3 pin 到别的 locale", "a/views/V.vue", "new Date().toLocaleDateString('en-US')", True),
        ("合法：登记口自身", REGISTRY, "return new Date(t).toLocaleString('zh-CN', { hour12: false })", False),
        ("合法：数字千分位无参", "a/views/N.vue", "{{ stats.total_tokens.toLocaleString() }}", False),
        ("合法：pin 到 zh-CN 的日期档", "a/views/M.vue", "new Date(s.created_at).toLocaleDateString('zh-CN')", False),
        # 反例：别的档合法带 hour12（首版在这里假红过）
        ("合法：带秒档自己写 hour12（不是登记口写法）", "a/views/S.vue",
         "new Date().toLocaleTimeString('zh-CN', { hour12: false })", False),
    ]
    bad = 0
    for name, rel, text, expect_err in cases:
        errs, unp, _, _ = classify(rel, text)
        fired = bool(errs) or unp > 0
        tag = "PASS" if fired == expect_err else "FAIL"
        bad += 0 if fired == expect_err else 1
        emit_err = (":: " + errs[0]) if errs else (":: unpinned=%d" % unp if unp else "")
        print("[%s] %s：fired=%s want=%s %s" % (tag, name, fired, expect_err, emit_err))
    # L4 正对照：不存在的仓库根必须拒绝，而不是印"0 命中＝干净"
    bogus = os.path.join(os.path.sep, "definitely-not-a-repo-root-9f2c")
    rc = check(bogus, quiet=True)
    print("[%s] L4 空判集必须被拒（不存在的仓库根）：rc=%d want=1" % ("PASS" if rc == 1 else "FAIL", rc))
    bad += 0 if rc == 1 else 1
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    n = len(collect(root)[1])
    print("[%s] L4 真仓库扫描集 = %d 个文件（下限 %d）" % ("PASS" if n >= MIN_FILES else "FAIL", n, MIN_FILES))
    bad += 0 if n >= MIN_FILES else 1
    print("SELFTEST %s（%d 例未落地）" % ("FAIL" if bad else "PASS", bad))
    return 1 if bad else 0


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", default=None)
    ap.add_argument("--selftest", action="store_true")
    a = ap.parse_args()
    if a.selftest:
        return selftest()
    root = a.repo or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    return check(root)


if __name__ == "__main__":
    sys.exit(main())
