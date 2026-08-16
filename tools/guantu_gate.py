#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
关图 CI 门禁（GR-STD-V1.0 · P5）

以信息关联关系图为唯一基准，在 CI 中强制「变更必须同步图」：
  1) 代码级图可重建（build）——捕获任何未同步的结构漂移
  2) REQ 根 + 六维绑定骨架可注入（skeleton）
  3) 采集硬违规（GR-E1 孤儿 / GR-E2 悬空 / GR-E3 缺证据）+ 需求对齐偏离（GR-E6）
  4) 首次运行把当前全部已知问题 + 当前覆盖率固化为基线（.guantu_baseline.json），
     放行以避免对已存在的孤儿/偏离做「打地鼠」式阻断；
  5) 后续运行按「相对基线」门禁：
       · 漂移——当前问题集相对基线不得出现「新增」项（新增孤儿/信息孤岛即阻断）；
       · 覆盖率回归——不得低于已固化基线覆盖率（允许 0.05 浮点容差）；
       · 绝对下限护栏（COVERAGE_FLOOR）。
     生成物/示例/构建产物目录自动豁免（派生代码不强制溯源需求根）。

退出码：0=门禁通过；1=门禁失败（应阻断合并）。
"""
import json
import os
import re
import subprocess
import sys

# ---------- 路径解析 ----------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)            # tools/.. = 仓库根
INFO_GRAPH_DIR = os.path.join(SCRIPT_DIR, "info-graph")
BASELINE_PATH = os.path.join(REPO_ROOT, ".guantu_baseline.json")

# 覆盖率绝对下限（安全护栏，通常远低于已固化的基线覆盖率；
# 真正的门禁依据是与基线覆盖率的「相对落差」，见 step5）
COVERAGE_FLOOR = 90.0
# 允许的诚实偏离（不属任何需求、有意保留的节点），其余偏离一律阻断
ALLOWED_DEVIATIONS = {"snake.py"}
# 生成物 / 构建产物 / 示例输出目录：派生代码不强制溯源需求根，门禁豁免
ALLOWED_PATH_PREFIXES = (
    "examples/out/",
    "target/",
    "node_modules/",
    "frontend/dist/",
    ".workbuddy/",
)

# 硬违规码：出现即门禁失败
HARD_ISSUES = ("GR-E1", "GR-E2", "GR-E3")
# 全部违规码（用于生成签名）
ALL_ISSUES = ("GR-E1", "GR-E2", "GR-E3", "GR-E6")


def log(msg: str) -> None:
    print(f"[guantu-gate] {msg}", flush=True)


def resolve_binary() -> str:
    for profile in ("release", "debug"):
        cand = os.path.join(INFO_GRAPH_DIR, "target", profile, "info-graph")
        if os.path.isfile(cand):
            return cand
    log("未找到预编译 info-graph，开始构建...")
    subprocess.run(["cargo", "build", "--release"], cwd=INFO_GRAPH_DIR, check=True)
    return os.path.join(INFO_GRAPH_DIR, "target", "release", "info-graph")


def run(bin_path: str, *args) -> str:
    proc = subprocess.run(
        [bin_path, *args],
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return proc.stdout


def node_path_of(line: str):
    """从违规行提取节点完整路径（Kind:path 形式），用于路径前缀豁免。"""
    m = re.search(r"(?:CodeFile|Script|Data|Interface|Function|Business|Runtime):(\S+)", line)
    return m.group(1) if m else None


def is_auto_allowed(line: str) -> bool:
    """生成产物路径 或 诚实偏离白名单 → 自动豁免。"""
    p = node_path_of(line)
    if p is None:
        return False
    base = p.rsplit("/", 1)[-1]
    if base in ALLOWED_DEVIATIONS:
        return True
    # 子串匹配：派生产物目录可能嵌套在 crates/*/ 等路径下，
    # 如 crates/primiflow/examples/out/ddl.sql 仍应豁免。
    return any(pre in p for pre in ALLOWED_PATH_PREFIXES)


def issue_signature(line: str) -> str:
    """把一条违规行规范为稳定签名（违规码 + 节点 id），用于漂移比对。"""
    code = None
    for c in ALL_ISSUES:
        if c in line:
            code = c
            break
    nid = None
    m = re.search(r"(?:CodeFile|Script|Data|Interface|Function|Business|Runtime):\S+", line)
    if m:
        nid = m.group(0)
    return f"{code}|{nid}"


def load_baseline():
    if not os.path.isfile(BASELINE_PATH):
        return None
    try:
        with open(BASELINE_PATH, "r", encoding="utf-8") as f:
            data = json.load(f)
        return {
            "signatures": set(data.get("signatures", [])),
            "coverage": float(data.get("coverage", 0.0)),
        }
    except Exception:
        return None


def save_baseline(sigs: set, coverage: float) -> None:
    with open(BASELINE_PATH, "w", encoding="utf-8") as f:
        json.dump(
            {"signatures": sorted(sigs), "coverage": coverage},
            f, ensure_ascii=False, indent=2,
        )


def main() -> int:
    bin_path = resolve_binary()
    log(f"使用关图工具: {bin_path}")

    # 1) 重建代码级图（捕获任何未同步的结构漂移）
    run(bin_path, "build", "--root", ".", "--out", "graph.json")
    log("step1 build 完成")

    # 2) 注入 REQ 根 + 六维绑定骨架
    run(bin_path, "skeleton", "--graph", "graph.json",
        "--spec", "guantu.req.json", "--out", "graph.enterprise.json")
    log("step2 skeleton 完成")

    # 3) 收集硬违规 + 偏离（生成产物路径自动豁免）
    vout = run(bin_path, "validate", "--graph", "graph.enterprise.json")
    dout = run(bin_path, "deviate", "--graph", "graph.enterprise.json")

    # 当前问题签名集（硬违规 + GR-E6，自动豁免后）；同时保留可读行
    cur_sigs: set = set()
    hard_lines: list = []
    for ln in vout.splitlines():
        if any(code in ln for code in HARD_ISSUES):
            if is_auto_allowed(ln):
                continue
            s = ln.strip()
            hard_lines.append(s)
            cur_sigs.add(issue_signature(s))
    for ln in dout.splitlines():
        if "[GR-E6]" in ln:
            if is_auto_allowed(ln):
                continue
            s = ln.strip()
            cur_sigs.add(issue_signature(s))

    # 覆盖率（保留用于阈值与报告）
    cov_m = re.search(r"需求对齐覆盖率:\s*([\d.eE+\-]+)%", dout)
    coverage = float(cov_m.group(1)) if cov_m else 0.0

    # 4) 基线处理
    baseline = load_baseline()
    if baseline is None:
        # 首次运行：把当前全部已知问题 + 当前覆盖率固化为基线并放行，
        # 避免对已存在的孤儿/偏离做「打地鼠」式阻断；后续任何新增即阻断合并。
        save_baseline(cur_sigs, coverage)
        log(f"✅ 首次运行：已将该批 {len(cur_sigs)} 个已知问题 + 覆盖率 {coverage:.1f}% 固化为基线（.guantu_baseline.json）")
        if hard_lines:
            log(f"⚠️ 基线内含 {len(hard_lines)} 个硬违规（首次运行不阻断），建议后续纳入六维绑定消项：")
            for ln in hard_lines[:20]:
                print(f"    ~ {ln}")
            if len(hard_lines) > 20:
                print(f"    ... 其余 {len(hard_lines) - 20} 项省略")
        return 0

    # 5) 后续运行：漂移 + 覆盖率回归 + 绝对下限 三项门禁
    #    —— 漂移检测天然覆盖「新增硬违规」：不在基线内的签名即为新增。
    new_sigs = cur_sigs - baseline["signatures"]
    if new_sigs:
        log(f"❌ 门禁失败：相对基线新增 {len(new_sigs)} 个未同步问题（隐性依赖/信息孤岛/新孤儿）")
        all_lines = [ln.strip() for ln in (vout + dout).splitlines()
                     if any(code in ln for code in ALL_ISSUES) and not is_auto_allowed(ln)]
        for sig in sorted(new_sigs):
            for ln in all_lines:
                if issue_signature(ln) == sig:
                    print(f"    + {ln}")
                    break
            else:
                print(f"    + {sig}")
        return 1

    # 覆盖率回归：不得低于已固化基线（允许 0.05 浮点容差）；同时设绝对下限护栏
    base_cov = baseline["coverage"]
    if coverage < base_cov - 0.05:
        log(f"❌ 门禁失败：需求对齐覆盖率 {coverage:.1f}% 较基线 {base_cov:.1f}% 回落，图结构出现新增未溯源节点")
        return 1
    if coverage < COVERAGE_FLOOR:
        log(f"❌ 门禁失败：需求对齐覆盖率 {coverage:.1f}% < 绝对下限 {COVERAGE_FLOOR:.1f}%")
        return 1

    log(f"✅ 门禁通过：无新增未同步问题（基线 {len(baseline['signatures'])} 项），"
        f"覆盖率 {coverage:.1f}%（基线 {base_cov:.1f}%）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
