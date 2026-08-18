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
  6) P9「先判重后立项」：对 docs/graph/requests/*.json 中的新需求逐条在关图判重，
     判定 reuse/incremental 放行，new（无任何对应能力）即阻断，强制人工确认
     确有必要才新立项 —— 从机制上杜绝重复造系统。

设计铁律（历次真实缺陷的固化教训）：
  · 子命令非零退出必须响亮失败，禁止吞掉 panic 后以空图判"通过"（虚假合规）；
  · REQ 规格路径集中定义并启动断言，避免规格迁移后门禁静默空跑；
  · Windows 下可执行文件需带 .exe 后缀，否则永远误判"未编译"。

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
# REQ 规格（相对仓库根）。历史缺陷：规格由仓库根移入 docs/graph/ 后门禁未跟随，
# skeleton 读不到文件 → 静默空跑。此处集中定义并在启动时断言存在。
REQ_SPEC_REL = os.path.join("docs", "graph", "guantu.req.json")
# 新需求判重（P9 先判重后立项）：该目录下每个 *.json 都会跑 dedup，未命中即阻断
DEDUP_REQUEST_DIR_REL = os.path.join("docs", "graph", "requests")

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


def _binary_names():
    """Windows 需带 .exe，否则 os.path.isfile 永远为 False → 误判未编译。"""
    return ("info-graph.exe", "info-graph") if os.name == "nt" else ("info-graph", "info-graph.exe")


def resolve_binary() -> str:
    for profile in ("release", "debug"):
        for name in _binary_names():
            cand = os.path.join(INFO_GRAPH_DIR, "target", profile, name)
            if os.path.isfile(cand):
                return cand
    log("未找到预编译 info-graph，开始构建...")
    subprocess.run(["cargo", "build", "--release"], cwd=INFO_GRAPH_DIR, check=True)
    for name in _binary_names():
        cand = os.path.join(INFO_GRAPH_DIR, "target", "release", name)
        if os.path.isfile(cand):
            return cand
    raise RuntimeError("构建后仍未找到 info-graph 可执行文件")


def run(bin_path: str, *args, check: bool = True) -> str:
    """执行子命令。

    铁律：默认 check=True —— 子命令非零退出即立刻失败并打印输出。
    历史缺陷：旧版忽略 returncode，spec 路径错误导致 skeleton panic 被静默吞掉，
    门禁却继续以空图判定"通过"，形成虚假合规。
    """
    proc = subprocess.run(
        [bin_path, *args],
        cwd=REPO_ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if check and proc.returncode != 0:
        log(f"❌ 子命令失败(exit={proc.returncode}): {' '.join(args)}")
        print(proc.stdout)
        raise SystemExit(1)
    return proc.stdout


def node_path_of(line: str):
    """从违规行提取节点完整路径（Kind:path 形式），用于路径前缀豁免。"""
    m = re.search(r"(?:CodeFile|Script|Data|Interface|Function|Business|Runtime):(\S+)", line)
    return m.group(1) if m else None


# 派生豁免集：节点自身无路径（如 Data:DATA_R1S 这类由 DDL 解析出的表），
# 但其全部关联边都来自已豁免的生成产物 → 同样豁免（由 compute_derived_exempt 填充）
DERIVED_EXEMPT: set = set()


def _path_exempt(p: str) -> bool:
    base = p.rsplit("/", 1)[-1]
    if base in ALLOWED_DEVIATIONS:
        return True
    # 子串匹配：派生产物目录可能嵌套在 crates/*/ 等路径下，
    # 如 crates/primiflow/examples/out/ddl.sql 仍应豁免。
    return any(pre in p for pre in ALLOWED_PATH_PREFIXES)


def compute_derived_exempt(graph_path: str) -> set:
    """计算「派生自豁免产物」的节点集合。

    原则：豁免应沿数据血缘传递。若某节点的**全部**关联边对端都位于已豁免路径
    （如 examples/out/ 下的生成 DDL），则该节点本身也是派生产物，不应要求溯源需求根。
    典型场景：Data:DATA_R1S 仅由 crates/primiflow/examples/out/ddl.sql 的
    CREATE TABLE 解析产生，节点自身没有文件路径，因而逃过路径豁免。
    """
    try:
        with open(graph_path, "r", encoding="utf-8") as f:
            g = json.load(f)
    except Exception as exc:
        log(f"⚠️ 派生豁免计算跳过（读取 {graph_path} 失败: {exc}）")
        return set()
    path_of = {n["id"]: n.get("path", "") for n in g.get("nodes", [])}
    incident: dict = {}
    for e in g.get("edges", []):
        incident.setdefault(e["from"], []).append(e["to"])
        incident.setdefault(e["to"], []).append(e["from"])
    derived = set()
    for nid, p in path_of.items():
        # 自身路径已可豁免的无需再算；完全孤立的节点不予豁免（应由 GR-E1 治理）
        if _path_exempt(p) or nid not in incident:
            continue
        peers = incident[nid]
        if peers and all(_path_exempt(path_of.get(peer, "")) for peer in peers):
            derived.add(nid)
    return derived


def is_auto_allowed(line: str) -> bool:
    """生成产物路径 / 诚实偏离白名单 / 派生自豁免产物 → 自动豁免。"""
    m = re.search(r"(?:CodeFile|Script|Data|Interface|Function|Business|Runtime):\S+", line)
    if m and m.group(0) in DERIVED_EXEMPT:
        return True
    p = node_path_of(line)
    if p is None:
        return False
    return _path_exempt(p)


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


def run_dedup_gate(bin_path: str) -> int:
    """P9 先判重后立项：对 docs/graph/requests/*.json 逐条判重，未命中即阻断。

    目录不存在或为空 → 跳过（不阻断），使该能力可渐进启用。
    """
    req_dir = os.path.join(REPO_ROOT, DEDUP_REQUEST_DIR_REL)
    if not os.path.isdir(req_dir):
        log("step6 dedup 跳过：无 docs/graph/requests/（可放入新需求规格以启用先判重后立项）")
        return 0
    specs = sorted(f for f in os.listdir(req_dir) if f.endswith(".json"))
    if not specs:
        log("step6 dedup 跳过：docs/graph/requests/ 为空")
        return 0
    blocked = []
    for s in specs:
        rel = os.path.join(DEDUP_REQUEST_DIR_REL, s).replace("\\", "/")
        out = run(bin_path, "dedup", "--graph", "graph.enterprise.json",
                  "--spec", rel, "--fail-on-new", check=False)
        m = re.search(r'"verdict":\s*"(\w+)"', out)
        verdict = m.group(1) if m else "unknown"
        sim = re.search(r'"similarity":\s*([\d.]+)', out)
        simv = sim.group(1) if sim else "?"
        log(f"  · {s}: {verdict} (similarity={simv})")
        if verdict == "new":
            blocked.append(s)
        elif verdict == "unknown":
            log(f"    ⚠️ 无法解析判重结果，请检查规格格式：{rel}")
            blocked.append(s)
    if blocked:
        log(f"❌ 门禁失败：{len(blocked)} 条需求未命中现有能力，需人工确认确有必要新立项："
            f"{', '.join(blocked)}")
        log("   （确认必要后，将其能力节点纳入 guantu.req.json 绑定，或移出 requests/ 目录）")
        return 1
    log(f"✅ step6 dedup 通过：{len(specs)} 条需求均可复用/增量实现，未出现重复造系统")
    return 0


def main() -> int:
    bin_path = resolve_binary()
    log(f"使用关图工具: {bin_path}")

    spec_abs = os.path.join(REPO_ROOT, REQ_SPEC_REL)
    if not os.path.isfile(spec_abs):
        log(f"❌ 门禁失败：REQ 规格不存在: {REQ_SPEC_REL}")
        return 1

    # 1) 重建代码级图（捕获任何未同步的结构漂移）
    run(bin_path, "build", "--root", ".", "--out", "graph.json")
    log("step1 build 完成")

    # 2) 注入 REQ 根 + 六维绑定骨架（0 个 REQ 根时工具会响亮失败，不再静默空跑）
    sk_out = run(bin_path, "skeleton", "--graph", "graph.json",
                 "--spec", REQ_SPEC_REL.replace("\\", "/"), "--out", "graph.enterprise.json")
    m = re.search(r"需求根\s*(\d+)\s*个，绑定边\s*(\d+)\s*条", sk_out)
    if m:
        log(f"step2 skeleton 完成：REQ 根 {m.group(1)} 个，绑定边 {m.group(2)} 条")
    else:
        log("step2 skeleton 完成")

    # 2.5) 计算派生豁免集（豁免沿血缘传递，见 compute_derived_exempt）
    global DERIVED_EXEMPT
    DERIVED_EXEMPT = compute_derived_exempt(os.path.join(REPO_ROOT, "graph.enterprise.json"))
    if DERIVED_EXEMPT:
        log(f"step2.5 派生豁免 {len(DERIVED_EXEMPT)} 个节点（其全部关联边均来自生成产物）")

    # 3) 收集硬违规 + 偏离（生成产物路径自动豁免）
    #    validate/deviate 发现问题时按约定以 exit 1 返回，属预期输出而非执行失败，故 check=False
    vout = run(bin_path, "validate", "--graph", "graph.enterprise.json", check=False)
    dout = run(bin_path, "deviate", "--graph", "graph.enterprise.json", check=False)

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
        return run_dedup_gate(bin_path)

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

    log(f"✅ 关图门禁通过：无新增未同步问题（基线 {len(baseline['signatures'])} 项），"
        f"覆盖率 {coverage:.1f}%（基线 {base_cov:.1f}%）")

    # 5.5) 棘轮：治理只允许单向收紧。一旦问题消项或覆盖率提升，立刻固化为新基线，
    #      使已取得的治理成果无法被后续提交悄悄回退。
    fixed = baseline["signatures"] - cur_sigs
    if fixed or coverage > base_cov + 0.05:
        save_baseline(cur_sigs, coverage)
        parts = []
        if fixed:
            parts.append(f"消项 {len(fixed)} 个")
        if coverage > base_cov + 0.05:
            parts.append(f"覆盖率 {base_cov:.1f}% → {coverage:.1f}%")
        log(f"🔒 棘轮已收紧基线（{'，'.join(parts)}），该成果今后不可回退")

    # 6) P9 先判重后立项（新需求必须先在关图判重，杜绝重复造系统）
    return run_dedup_gate(bin_path)


if __name__ == "__main__":
    sys.exit(main())
