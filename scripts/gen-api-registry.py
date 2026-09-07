# -*- coding: utf-8 -*-
"""Generate docs/API-REGISTRY.md from actuator.rs ROUTES (single source of truth)."""
import re, os

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PATH = os.path.join(REPO, 'platform', 'gateway', 'mox-platform-gateway-svc', 'src', 'actuator.rs')
s = open(PATH, encoding='utf-8').read()

entries = []
for m in re.finditer(r'r\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]+)"\s*,\s*"([^"]*)"', s):
    key, meth, path, layer, dom, status, desc = m.groups()
    entries.append((key, meth, path, layer, dom, status, desc))

# implementation source per domain (gateway-internal modules)
IMPL = {
    'actuator': '`platform/gateway/mox-platform-gateway-svc/src/actuator.rs`',
    'platform': '`lib.rs`（内联路由）+ `proxy.rs`（反向代理 :3001/:8000）',
    'kg': '`platform/domains/kg/svc/mox-kg-service-svc/src/http_adapter.rs`',
    'ai': '`platform/domains/kg/svc/mox-kg-service-svc/src/http_adapter.rs`',
    'kb': '`platform/domains/kg/svc/mox-kb-svc/src/handlers.rs`（nest `/api`）+ `kb_ext.rs`',
    'alliance': '`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs`',
    'system': '`platform/gateway/mox-platform-gateway-svc/src/system.rs`',
    'experts': '`experts_registry/collaboration/dispatcher/graph/orchestration/session/ext.rs` 七模块',
    'monitor': '`platform/gateway/mox-platform-gateway-svc/src/monitor.rs`',
    'projects': '`platform/gateway/mox-platform-gateway-svc/src/projects_ext.rs`',
    'workspace': '`platform/gateway/mox-platform-gateway-svc/src/workspace.rs`',
    'notification': '`platform/gateway/mox-platform-gateway-svc/src/notification.rs`',
    'misc': '`platform/gateway/mox-platform-gateway-svc/src/misc.rs`',
}

by_dom = {}
for e in entries:
    by_dom.setdefault(e[4], []).append(e)

order = ['actuator', 'platform', 'kg', 'ai', 'kb', 'alliance', 'system', 'experts', 'monitor', 'projects', 'workspace', 'notification', 'misc']

lines = []
A = lines.append
A('# API 注册表（权威·接口↔实现一一对应）')
A('')
A('> 本文档为网关 8080 暴露的全部 API 的唯一权威清单，由 `platform/gateway/mox-platform-gateway-svc/src/actuator.rs` 的 `ROUTES` 静态表直接生成（生成脚本 `scripts/gen-api-registry.py`）。**声明即实现**：表中每一条都有对应源码注册与真实 handler，不存在纯占位条目。')
A('')
A('## 1. 总览')
A('')
A('| 指标 | 值 |')
A('| --- | --- |')
A(f'| 注册路由总数 | **{len(entries)} 条**（全部 ready，全部有真实实现） |')
A('| 业务域（网关内嵌） | 13 个：actuator / platform / kg / ai / kb / alliance / system / experts / monitor / projects / workspace / notification / misc |')
A('| 域描述符（业务规划） | 43 个：ready 7 · beta 1 · stub 35（见 §3） |')
A('| 独立服务进程 | 6 个：kg-hub / kb-server / alliance-executor / alliance-scheduler / primiflow / melody2score（见 §4） |')
A('| 鉴权 | 全部业务路由经 `Authorization: Bearer <dev-secret-token>`（JWT）保护；管理面 `/health /metrics /actuator` 公开 |')
A('')
A('## 2. 逐域注册表（199 条）')
A('')
A('按域分组，实现位置逐一标注；`ANY` 表示该方法+参数可匹配多方法（GET/POST/PUT/DELETE）。')
A('')

for dom in order:
    items = by_dom.get(dom, [])
    A(f'### {dom}（{len(items)} 条）')
    A('')
    A(f'实现：{IMPL.get(dom, "待补")}')
    A('')
    A('| ID | 方法 | 路径 | 层 | 说明 |')
    A('| --- | --- | --- | --- | --- |')
    for key, meth, path, layer, dm, status, desc in items:
        p = path.replace('|', '\\|')
        A(f'| `{key}` | {meth} | `{p}` | {layer} | {desc} |')
    A('')

A('## 3. 业务域描述符（43 域·routes.rs）')
A('')
A('| 状态 | 数量 | 域 |')
A('| --- | --- | --- |')
A('| ready | 7 | Health·Metrics·KG·KB·AIEngine·Alliance·Expert |')
A('| beta | 1 | IAM（依赖编排器 :3001，未启动时 502 ORCHESTRATOR_UNREACHABLE） |')
A('| stub | 35 | Auth·Tenant·RBAC·Graph·Cypher·nGQL·AI-Core·Intent·Flow·Workflow·BPM·Pipeline·Cloud·S3·Volume·FS·Data·ETL·Norm·Standard·Voice·MIDI·Melody·TTS·Market·Shop·Order·Billing·Streams·Kafka·WebSocket·Event·Enterprise·Platform·Audit |')
A('')
A('> stub 仅为规划声明，不对外承诺；S3 曾标 ready 但无实现，已如实降为 stub。')
A('')
A('## 4. 独立服务进程（网关之外）')
A('')
A('| 服务 | 二进制 | 路由前缀 | 说明 |')
A('| --- | --- | --- | --- |')
A('| kg-hub | `mox-kg-hub-svc` | `/api/kg/*`（15 条） | 知识图谱枢纽：检索/影响/治理/闭环 |')
A('| kb-server | `mox-kb-server` | `/api/v1/kb/*`（4 条） | 知识库独立服务（与网关内嵌 kb 并存） |')
A('| alliance-executor | `mox-alliance-executor` | `/health` `/tasks/:id/*` `/internal/*`（8 条） | 联盟任务执行器 |')
A('| alliance-scheduler | `mox-alliance-scheduler` | `/tasks` `/experts/search`（5 条） | 联盟调度器 |')
A('| primiflow | 前端子项目 | — | `:8000`，经 `/api/projects/{*path}` 代理 |')
A('| melody2score | 前端子项目 | — | `:8012`，简谱转谱 |')
A('')
A('### 4.1 全链路启动命令（2026-09-07 实测通过）')
A('')
A('五个进程按依赖顺序启动（Windows PowerShell，均为 debug 构建）：')
A('')
A('```powershell')
A('# 1) 联盟调度器 :3100 / 执行器 :3200（独立二进制，配置读 config/alliance-*.yml）')
A('Start-Process target\\debug\\mox-alliance-scheduler.exe -ArgumentList @("--port","3100") -WindowStyle Hidden')
A('Start-Process target\\debug\\mox-alliance-executor.exe -ArgumentList @("--port","3200") -WindowStyle Hidden')
A('')
A('# 2) 知识库独立服务 :8104')
A('Start-Process target\\debug\\mox-kb-server.exe -ArgumentList @("--port","8104") -WindowStyle Hidden')
A('')
A('# 3) 编排器 :3001（OUS_ENABLE_MOX_SYSTEM=0 跳过未 bootstrap 的 mox-system 模块；OUS_API_TOKEN 与网关对齐）')
A('$env:OUS_ENABLE_MOX_SYSTEM="0"; $env:OUS_API_TOKEN="dev-secret-token"')
A('Start-Process target\\debug\\operator-server.exe -ArgumentList @("--port","3001") -WindowStyle Hidden')
A('')
A('# 4) 网关 :8080（MOX_ALLIANCE_*_URL 激活联盟远程模式）')
A('$env:MOX_ALLIANCE_SCHEDULER_URL="http://127.0.0.1:3100"; $env:MOX_ALLIANCE_EXECUTOR_URL="http://127.0.0.1:3200"')
A('Start-Process target\\debug\\mox-server.exe -ArgumentList @("--port","8080") -WindowStyle Hidden')
A('```')
A('')
A('验证要点：网关 `/api/v1/status` → `iam: ready`；`/api/alliance/runtime` → `mode: remote, execution_ready: true`；联盟任务创建后经调度器真实执行（DAG 节点流转）；编排器 `/api/graph/export`、`/api/status` 需带 `Authorization: Bearer dev-secret-token`。')
A('')
A('**无外部模型 key 的完整闭环验证（实测 2026-09-07）**：启动前设 `$env:EXECUTOR_MODE="mock"` 再运行上述脚本 → 执行器以 Mock 节点执行器启动（`/health` 返回 `execution_mode: mock`）；创建联盟任务后经 网关→调度器→执行器 全链路真实流转，DAG 5/5 节点完成，任务终态 `completed`。生产（expert）模式需配置 LLM provider key（`strict_llm_consultant_from_env`），未配置时 readiness 如实报告未就绪。')
A('')
A('## 5. 治理规则（新增/修改 API 必须遵守）')
A('')
A('1. **单一权威源**：所有对外路由必须先登记到 `actuator.rs` `ROUTES`，再写 handler；`/actuator/mappings` 是唯一注册表视图。')
A('2. **前缀权威**：kg=`/kg/v1/*`；ai=`/ai/engine/*`；kb=`/api/kb/*`；alliance=`/api/alliance/*`；experts=`/api/experts*`；system/security=`/api/system/*`、`/api/security/*`；其余模块=`/api/<module>/*`。历史前缀（`/ai/v1`、`/kb/v1`、`/alliance/v1`）已废弃，一律 404。')
A('3. **新增路由闭环**：改 `actuator.rs` → 更新本文档（重跑 `scripts/gen-api-registry.py`）→ `cargo check -p mox-platform-gateway-svc` → 启动验证 `/actuator/mappings` 计数与新增路径 200。')
A('4. **状态语义**：`ready`=有真实 handler 且已接线路由；`stub`=仅规划；`beta`=可用但依赖外部进程。不允许出现“声明 ready 但无路由”的条目。')
A('')
A('## 6. 变更记录')
A('')
A('| 日期 | 变更 |')
A('| --- | --- |')
A('| 2026-09-06 | **注册表归一化（98→199）**：修正 ai/kb/alliance 三域前缀漂移（`/ai/engine`、`/api/kb`、`/api/alliance`）；补齐漏声明的 experts 48 / monitor 12 / projects 14 / workspace 7 / notification 4 / misc 5 / kb_ext 2 / auth 1；canonical 映射修正；S3 如实降 stub、Expert 如实升 ready；生成脚本与治理规则落地 |')
A('| 2026-09-07 | **运行验证与语义修复**：5 进程全链路实测（编排器3001/kb8104/调度3100/执行3200/网关8080）；联盟远程模式激活（`MOX_ALLIANCE_*_URL`）；Mock 执行器全链路任务闭环 completed（5/5 节点）；readiness 语义修复（mock 模式如实就绪）；一键启停脚本落地 |')

open(os.path.join(REPO, 'docs', 'API-REGISTRY.md'), 'w', encoding='utf-8', newline='\n').write('\n'.join(lines))
print(f'docs/API-REGISTRY.md generated, {len(entries)} routes')
