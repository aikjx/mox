# -*- coding: utf-8 -*-
"""verify-doc-ep038.py — DOC-EP-038 文档↔代码事实自动核对（企业级自动化）

对 docs/enterprise/38-企业级管理系统架构与业务处理流程文档-V2.1.md 中
登记的代码事实（业务域 / crate 数 / 中间件选型 / 网关 AI 端点 / 前端视图 / 映射表关键 crate）
与仓库真实代码逐项比对，输出 PASS/FAIL 报告，任一 FAIL 退出码非 0（可接入 CI 闸门）。

用法:
    python scripts/verify-doc-ep038.py            # 核对并输出报告（生成 docs/enterprise/38-VERIFY-REPORT.md）
    python scripts/verify-doc-ep038.py --quiet    # 仅输出结论行

事实分级：本文核对项均为【已确认 · 代码核实】事实，任何 FAIL 必须修复文档或代码后重跑。
"""
import argparse
import datetime
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOC = ROOT / "docs" / "enterprise" / "38-企业级管理系统架构与业务处理流程文档-V2.1.md"
REPORT = ROOT / "docs" / "enterprise" / "38-VERIFY-REPORT.md"

# ---------------------------------------------------------------- 代码事实（权威源）
def cargo_members():
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=str(ROOT), check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )
    meta = json.loads(out.stdout)
    pkgs = {p["name"]: p["manifest_path"] for p in meta["packages"] if p["id"] in meta["workspace_members"]}
    return pkgs

def domains_from_fs():
    d = ROOT / "platform" / "domains"
    return sorted(p.name for p in d.iterdir() if p.is_dir())

def dep_versions(root_toml_text, all_tomls_text):
    """从 workspace 根 Cargo.toml（及全仓兜底）提取关键依赖版本声明，兼容 'name = "v"' 与 'name = { version = "v" }' 两种形式。"""
    found = {}
    for name in ["axum", "sqlx", "rusqlite", "redis", "rocksdb", "tokio"]:
        pat = re.compile(
            r'^\s*' + re.escape(name) + r'\s*=\s*(?:"([^"]+)"|\{[^}]*?version\s*=\s*"([^"]+)"[^}]*\})',
            re.M,
        )
        m = pat.search(root_toml_text)
        if not m:
            # 不在 workspace 根时全仓兜底扫描（如 rocksdb 位于子 crate）
            m = pat.search(all_tomls_text)
        if m:
            found[name] = m.group(1) or m.group(2)
    return found

def gateway_ai_endpoints():
    gw = ROOT / "platform" / "gateway" / "mox-platform-gateway-svc" / "src"
    hits = set()
    if gw.exists():
        for f in gw.rglob("*.rs"):
            txt = f.read_text(encoding="utf-8", errors="ignore")
            for m in re.finditer(r'/ai/engine/(process|analyze|capabilities|metrics)', txt):
                hits.add(m.group(0))
    return sorted(hits)

def fe_views():
    v = ROOT / "frontend-ui" / "src" / "views"
    return len(list(v.rglob("*.vue"))) if v.exists() else 0

def fe_view_dirs():
    v = ROOT / "frontend-ui" / "src" / "views"
    return sorted(p.name for p in v.iterdir() if p.is_dir()) if v.exists() else []

# 业务视角 ↔ 平台域映射表中抽样的关键 crate（文档声明其存在）
KEY_CRATES = [
    "mox-platform-iam-core", "mox-iam-server", "mox-platform-meta-core",
    "mox-platform-datastore-core", "mox-platform-orchestrator-svc",
    "mox-platform-enterprise-svc", "mox-platform-integration-core",
    "mox-ai-agent-svc", "mox-ai-intent-svc", "mox-ai-expert-svc",
    "mox-flow-lowcode-core", "mox-flow-operator-core",
    "mox-flow-unified-process-core", "mox-flow-primiflow-svc", "mox-flow-fusion-svc",
    "mox-data-norm-core", "mox-data-standards-core", "mox-data-etl-svc", "mox-data-plane-svc",
    "mox-kg-algo-core", "mox-kg-hub-svc", "mox-kg-fusion-svc",
    "mox-kb-core", "mox-kb-server",
    "mox-voice-asr-svc", "mox-voice-intent-svc", "mox-voice-dsp-core",
    "mox-cloud-s3-svc", "mox-cloud-filer-svc",
    "mox-alliance-core", "mox-alliance-executor-svc", "mox-alliance-scheduler-svc",
    "mox-base-graph-core", "mox-base-perm-core", "mox-base-store-core",
    "mox-pipeline-framework", "mox-rbac-engine",
    "mox-market-template-svc", "mox-project-graph-core",
]

# ---------------------------------------------------------------- 核心功能完成度核对（E 系列）
# 对照 38 号文档 6.2「业务系统视角」7 项核心能力；证据 = 后端 API 路由 + 引擎 crate + 前端视图，三者齐备才算完成。
CORE_CAPABILITIES = [
    {
        "id": "E1", "name": "组织权限（SSO / RBAC）",
        "api": ["/api/system/user", "/api/system/role", "/api/system/menu", "/api/auth/me", "/api/system/permissions"],
        "crates": ["mox-rbac-engine", "mox-platform-iam-core", "mox-iam-server", "mox-base-perm-core"],
        "views": ["auth/Login.vue", "admin/panels/AdminUser.vue", "admin/panels/AdminRole.vue", "admin/panels/AdminMenu.vue"],
        "mapping": "platform:mox-platform-iam-core/mox-iam-server; base:mox-base-perm-core; foundation:mox-rbac-engine",
    },
    {
        "id": "E2", "name": "主数据 / 编码 / 字典",
        "api": ["/api/system/dict/type", "/api/system/dict/data"],
        "crates": ["mox-platform-meta-core", "mox-platform-datastore-core"],
        "views": ["admin/panels/AdminDict.vue"],
        "mapping": "platform:mox-platform-meta-core / mox-platform-datastore-core",
    },
    {
        "id": "E3", "name": "业务办理（单据生命周期 / 编排执行）",
        "api": ["/api/tasks", "/api/projects", "/api/alliance/tasks"],
        "crates": ["mox-platform-orchestrator-svc", "mox-platform-enterprise-svc", "mox-flow-operator-core"],
        "views": ["misc/BusinessHall.vue", "project/TaskView.vue", "project/Workbench.vue"],
        "mapping": "platform:mox-platform-orchestrator-svc / mox-platform-enterprise-svc; flow:mox-flow-operator-core",
    },
    {
        "id": "E4", "name": "审批 / 流程（低代码 + 统一流程）",
        "api": ["/api/ai/flows", "/api/alliance/tasks"],
        "crates": ["mox-flow-unified-process-core", "mox-flow-lowcode-core"],
        "views": ["workflow/WorkflowView.vue", "workflow/panels/WorkflowFlowsPanel.vue", "workflow/panels/AutomationPanel.vue"],
        "mapping": "flow:mox-flow-unified-process-core / mox-flow-lowcode-core",
    },
    {
        "id": "E5", "name": "报表统计（KPI / 数据标准）",
        "api": ["/api/workspace/kpi", "/api/system/operlog/export"],
        "crates": ["mox-data-standards-core", "mox-data-formula-core", "mox-data-catalog-svc"],
        "views": ["workspace/panels/KpiPanel.vue", "admin/panels/AdminOverview.vue"],
        "mapping": "data:mox-data-formula-core / mox-data-standards-core / mox-data-catalog-svc",
    },
    {
        "id": "E6", "name": "系统管理 / 审计 / 通知",
        "api": ["/api/system/config", "/api/system/operlog", "/api/system/logininfor", "/api/security/audit-log", "/api/notifications"],
        "crates": ["mox-platform-system-core", "mox-alliance-config-core"],
        "views": ["admin/panels/AdminConfig.vue", "admin/panels/AdminAudit.vue", "admin/panels/AdminLogs.vue", "admin/panels/AdminMonitor.vue"],
        "mapping": "platform:mox-platform-system-core; alliance:mox-alliance-config-core",
    },
    {
        "id": "E7", "name": "AI 服务（Agent / RAG / Function Calling）",
        "api": ["/ai/engine/process", "/ai/engine/analyze", "/api/experts", "/api/ai/flows"],
        "crates": ["mox-ai-agent-svc", "mox-ai-intent-svc", "mox-ai-expert-svc", "mox-kb-core", "mox-kg-hub-svc"],
        "views": ["ai/ChatView.vue", "ai/BotCenterView.vue", "workspace/panels/AIAssistantPanel.vue"],
        "mapping": "ai:mox-ai-agent-svc / mox-ai-intent-svc / mox-ai-expert-svc; kb/kg 知识底座",
    },
]

# ---------------------------------------------------------------- 文档声明（被核对方）
def doc_text():
    return DOC.read_text(encoding="utf-8")

def doc_domains(txt):
    """文档 6.2 表中代码域列（反引号包裹的域代码行）。"""
    rows = re.findall(r'^\| `(platform|ai|flow|data|kg|kb|voice|cloud|alliance|base|foundation|market|project)` \|', txt, re.M)
    return sorted(set(rows))

def doc_crate_count(txt):
    m = re.search(r'共 \*\*(\d+) 个业务域、(\d+) 个 Crate\*\*', txt)
    return (int(m.group(1)), int(m.group(2))) if m else None

def doc_dep(txt, name):
    """抓取文档中 `name X.Y` 反引号声明片段（版本号）。"""
    m = re.search(r'`' + re.escape(name) + r'\s+([^`]+)`', txt)
    return m.group(1).strip() if m else None

def doc_endpoints(txt):
    return sorted(set(re.findall(r'/ai/engine/(process|analyze|capabilities|metrics)', txt)))

def doc_fe_views(txt):
    m = re.search(r'含 (\d+) 个视图', txt)
    return int(m.group(1)) if m else None

# ---------------------------------------------------------------- 核对
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--quiet", action="store_true")
    args = ap.parse_args()

    members = cargo_members()
    code_domains = domains_from_fs()
    root_toml = (ROOT / "Cargo.toml").read_text(encoding="utf-8", errors="ignore")
    all_tomls = "\n".join(
        f.read_text(encoding="utf-8", errors="ignore")
        for f in (ROOT / "platform").rglob("Cargo.toml")
    )
    deps = dep_versions(root_toml, all_tomls)
    endpoints = gateway_ai_endpoints()
    views = fe_views()
    view_dirs = fe_view_dirs()
    txt = doc_text()

    checks = []  # (id, 名称, 代码事实, 文档声明, PASS/FAIL, 说明)
    def add(cid, name, code_val, doc_val, ok, note=""):
        checks.append((cid, name, code_val, doc_val, ok, note))

    # D1 业务域数量
    add("D1", "业务域数量", f"{len(code_domains)} 个", f"{doc_crate_count(txt)[0] if doc_crate_count(txt) else '?'} 个",
        len(code_domains) == doc_crate_count(txt)[0] if doc_crate_count(txt) else False,
        "代码: " + ", ".join(code_domains))

    # D2 域列表覆盖（文档表包含全部代码域）
    missing = [d for d in code_domains if d not in doc_domains(txt)]
    add("D2", "域列表覆盖", ", ".join(code_domains), "文档 6.2 域表",
        not missing, "缺失: " + (", ".join(missing) if missing else "无"))

    # D3 crate 数
    code_n = len(members)
    doc_pair = doc_crate_count(txt)
    add("D3", "crate 总数", f"{code_n}", f"{doc_pair[1] if doc_pair else '?'}",
        doc_pair is not None and doc_pair[1] == code_n, "cargo metadata workspace_members")

    # D4 中间件选型版本
    for name, doc_ok in [("axum", "0.7"), ("sqlx", "0.8"), ("rusqlite", "0.31"), ("redis", "0.26"), ("rocksdb", "0.25"), ("tokio", "1")]:
        cv = deps.get(name)
        dv = doc_dep(txt, name)
        ok = cv is not None and cv.startswith(doc_ok) and (dv is not None)
        add(f"D4-{name}", f"依赖 {name}", f"{cv}（代码）", f"{dv}（文档）", ok)

    # D5 网关 AI 四端点
    missing_ep = [e for e in ["/ai/engine/process", "/ai/engine/analyze", "/ai/engine/capabilities", "/ai/engine/metrics"] if e not in endpoints]
    add("D5", "网关 AI 四端点", ", ".join(endpoints), "process/analyze/capabilities/metrics",
        not missing_ep, "缺失: " + (", ".join(missing_ep) if missing_ep else "无"))

    # D6 前端视图数
    add("D6", "前端视图数", f"{views}", f"{doc_fe_views(txt) if doc_fe_views(txt) is not None else '?'}",
        doc_fe_views(txt) == views, "视图区: " + ", ".join(view_dirs))

    # D7 映射表关键 crate 存在性
    absent = [c for c in KEY_CRATES if c not in members]
    add("D7", "业务↔平台映射关键 crate", f"{len(KEY_CRATES)} 个抽样", "文档 6.2 映射表",
        not absent, "缺失: " + (", ".join(absent) if absent else "无"))

    # D8 API-REGISTRY 新鲜度（可再生成性核对）
    #   gen-api-registry.py 以 actuator.rs ROUTES 为单一权威源生成注册表；
    #   E 系列 API 证据取自注册表，注册表一旦过期核对地基即失效。
    #   流程：备份 → 重新生成 → 对比（规范化换行）→ 恢复，全程只读。
    reg_path = ROOT / "docs" / "API-REGISTRY.md"
    reg_backup = reg_path.read_bytes()
    reg_fresh = None
    reg_route_count = None
    try:
        gen = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "gen-api-registry.py")],
            cwd=str(ROOT), check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        reg_new = reg_path.read_text(encoding="utf-8", errors="ignore")
        reg_old = reg_backup.decode("utf-8", errors="ignore")
        reg_fresh = reg_new.replace("\r\n", "\n") == reg_old.replace("\r\n", "\n")
        m = re.search(rb"generated, (\d+) routes", gen.stdout)
        reg_route_count = m.group(1).decode() if m else "?"
    except Exception as exc:
        reg_fresh = False
        reg_route_count = f"异常: {exc}"
    finally:
        reg_path.write_bytes(reg_backup)  # 恢复，核对保持只读
    add("D8", "API-REGISTRY 新鲜度", f"{reg_route_count} 条（重新生成）", "docs/API-REGISTRY.md",
        bool(reg_fresh), "不一致: 运行 python scripts/gen-api-registry.py 同步" if not reg_fresh else "一致")

    # ------------------------------------------------- E 系列：核心功能完成度（API + crate + 前端视图 三证据）
    api_reg = (ROOT / "docs" / "API-REGISTRY.md").read_text(encoding="utf-8", errors="ignore")
    views_root = ROOT / "frontend-ui" / "src" / "views"
    for cap in CORE_CAPABILITIES:
        api_missing = [p for p in cap["api"] if p not in api_reg]
        crate_missing = [c for c in cap["crates"] if c not in members]
        view_missing = [v for v in cap["views"] if not (views_root / v).exists()]
        ok = not (api_missing or crate_missing or view_missing)
        ev = []
        if api_missing:
            ev.append("API缺:" + ",".join(api_missing))
        if crate_missing:
            ev.append("crate缺:" + ",".join(crate_missing))
        if view_missing:
            ev.append("视图缺:" + ",".join(view_missing))
        note = ("证据全齐 ✅ " if ok else " | ".join(ev)) + " | 映射: " + cap["mapping"]
        add(cap["id"], "核心功能·" + cap["name"], "API " + str(len(cap["api"])) + " + crate " + str(len(cap["crates"])) + " + 视图 " + str(len(cap["views"])),
            "38 号 6.2 业务视角映射表", ok, note)

    fails = [c for c in checks if not c[4]]
    ts = datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    lines = []
    A = lines.append
    A("# DOC-EP-038 文档↔代码事实自动核对报告")
    A("")
    A(f"> 生成时间：{ts} · 生成脚本：`scripts/verify-doc-ep038.py` · 核对对象：`38-企业级管理系统架构与业务处理流程文档-V2.1.md`")
    A("> 结论：**" + ("全部 PASS ✅" if not fails else f"{len(fails)} 项 FAIL ❌") + "**（D1~D8 + E1~E7 共 " + str(len(checks)) + " 项核对）")
    A("")
    A("| 编号 | 核对项 | 代码事实 | 文档声明 | 结果 | 说明 |")
    A("|------|--------|----------|----------|------|------|")
    for cid, name, cv, dv, ok, note in checks:
        A(f"| {cid} | {name} | {cv} | {dv} | {'✅ PASS' if ok else '❌ FAIL'} | {note} |")
    A("")
    if fails:
        A("## 失败项处置")
        A("")
        A("1. **文档漂移**（代码已变而文档未更新）→ 更新 38 号文档对应章节并在版本记录 + 00-INDEX §5 留痕；")
        A("2. **代码缺失**（文档声明而代码不存在）→ 属名实分裂，按 `22-全文档归一化总控卡` 裁决，补代码或删声明；")
        A("3. 修复后重跑本脚本直至全 PASS。")
    else:
        A("## 结论")
        A("")
        A("文档与代码一致，DOC-EP-038 登记的全部代码事实（域 / crate / 选型 / 端点 / 视图 / 映射）均可在仓库中核实。")
    A("")
    A("*本报告由脚本确定性生成，禁止手改；核对项新增/变更须同步修改脚本与文档。*")

    REPORT.write_text("\n".join(lines), encoding="utf-8")

    if not args.quiet:
        for cid, name, cv, dv, ok, note in checks:
            print(f"[{'PASS' if ok else 'FAIL'}] {cid} {name}: 代码={cv} | 文档={dv} | {note}")
        print(f"\n结论: {len(checks)} 项核对, {len(fails)} 项 FAIL. 报告: {REPORT.relative_to(ROOT)}")
    else:
        print(f"DOC-EP-038 verify: {len(checks)} checks, {len(fails)} FAIL -> {'PASS' if not fails else 'FAIL'}")

    sys.exit(1 if fails else 0)

if __name__ == "__main__":
    main()
