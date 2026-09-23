# MOX 部署进程拓扑与跨域请求闭环（源码实证）

> 编号：DOC-GOV L7 · working-reports/_norm_research
> 生成日期：2026-09-16
> 方法：以 `D:\a10\aikjx\gitcode\infotopograph` 真实源码为准逐跳核实；端口唯一权威 `docs/api/PORT-REGISTRY.md`（PORT-REGISTRY-001 V1.2，§6.4 已归一化为四进程）。
> 范围：仅企业默认四进程（3080 / 3001 / 3100 / 3200）+ 可选独立扩展（3411–3414）。不涉及 8080、五进程等历史形态。

---

## 一、结论速览（先看这里）

1. **唯一 HTTP 入口 = 网关 `mox-server`（crate `mox-platform-gateway-svc`），监听 3080**。它把 KG / KB / Cloud / IAM / RBAC / 专家联盟任务域**全部进程内内嵌**为子路由（`modules.rs::build_module_routers`），并对未命中的 `/api/*` 做反向代理。
2. **operator-server 二进制 = crate `mox-platform-orchestrator-svc`**（`Cargo.toml [[bin]] name = "operator-server"`），监听 **3001**，是「算子统一系统运行时 / 业务域宿主」，承载 `/api/graph/*`、`/api/ai/*`、`/api/market/*`、`/api/agent/*`、`/api/governance/*` 等业务域；它是网关反向代理的后端，**不是**专家联盟任务调度器。
3. **专家联盟任务域闭环不经过 operator-server:3001**：网关进程内由 `mox-alliance-http-sdk` 直接用 reqwest **HTTP** 后连 scheduler:3100 / executor:3200（环境变量 `MOX_ALLIANCE_SCHEDULER_URL` / `MOX_ALLIANCE_EXECUTOR_URL` 开关）。scheduler:3100 再桥接分发到 executor:3200。
4. 域扩展（KG/KB/Cloud/IAM）默认内嵌；仅当容器需要水平拆分时用**同一个 `mox-server` 二进制 + `MOX_HOST_ROLE=kg|cloud|kb|iam`** 起独立宿主，占用 **3411/3412/3414/3413**，由 nginx 统一入口。

---

## 二、四进程 + 独立扩展拓扑表

### 2.1 企业默认四进程

| 进程名（脚本/日志） | 二进制 | crate（仓库路径） | 端口 | 职责 | 关键源码证据 |
|---|---|---|---|---|---|
| **mox-server** | `mox-server.exe` | `platform/gateway/mox-platform-gateway-svc` | **3080** | 唯一 HTTP 入口。中间件分层 可观测→CORS→限流→JWT 鉴权；内嵌 KG/KB/Cloud/IAM/RBAC/专家联盟任务域；反向代理业务域到 3001、项目域到 8000；HTTP 后连联盟 3100/3200 | `src/main.rs:27`(默认 3080)、`src/lib.rs:266 build_gateway_router`、`src/modules.rs:108 build_module_routers`、`src/proxy.rs` |
| **operator-server** | `operator-server.exe` | `platform/domains/platform/svc/mox-platform-orchestrator-svc` | **3001** | 算子运行时 / 业务域宿主：聚合 primiflow / fusion / ai-agent / data-catalog / kg-algo 等子服务为库挂载；承载 `/api/graph/*` `/api/ai/*` `/api/market/*` `/api/agent/*` `/api/governance/*` 等 | `Cargo.toml [[bin]] name="operator-server"`、`src/main.rs:641-645`（默认 3001，`MOX_ORCHESTRATOR_PORT`） |
| **mox-alliance-scheduler** | `mox-alliance-scheduler.exe` | `platform/domains/alliance/svc/mox-alliance-scheduler-svc` | **3100** | 专家联盟调度器：任务创建/列表/详情/操作、专家匹配（RuleBasedExpertMatcher）、计划生成；任务仓库（内存/文件快照）；桥接执行器 | `Cargo.toml [[bin]] name="mox-alliance-scheduler"`、`src/bin/main.rs:69-71 with_executor_url`、`src/routes.rs:19` |
| **mox-alliance-executor** | `mox-alliance-executor.exe` | `platform/domains/alliance/svc/mox-alliance-executor-svc` | **3200** | 专家联盟执行器：DAG 执行 + 节点调度 + 状态管理；`expert` 模式内嵌 `mox-ai-expert-svc` 跑真实 AI 专家节点 | `Cargo.toml [[bin]] name="mox-alliance-executor"`、`src/bin/main.rs:61-63 ExecutorMode::Expert` |

启动接线证据：`scripts/startup/start-mox-enterprise.ps1`
- `Start-One mox-alliance-scheduler --port 3100`
- `Start-One mox-alliance-executor --port 3200`
- `Start-One operator-server --port 3001`（env `OUS_API_TOKEN`）
- `Start-One mox-server --port 3080`（env `MOX_ALLIANCE_SCHEDULER_URL=http://127.0.0.1:3100`、`MOX_ALLIANCE_EXECUTOR_URL=http://127.0.0.1:3200`）
- 脚本头注释明确：「KG/KB/Cloud/IAM 已内嵌网关，默认不重复启动独立域进程」。

### 2.2 可选独立扩展（默认不开）

| 角色 | 二进制 | 触发方式 | 容器端口 | 何时启用 / 为何默认不开 |
|---|---|---|---|---|
| KG 独立宿主 | 同 `mox-server` | `MOX_HOST_ROLE=kg` | **3411** | 仅容器水平拆分 / 单域隔离时启用 |
| Cloud 独立宿主 | 同 `mox-server` | `MOX_HOST_ROLE=cloud` | **3412** | 同上 |
| IAM 独立宿主 | 同 `mox-server` | `MOX_HOST_ROLE=iam` | **3413** | 同上 |
| KB 独立宿主 | 同 `mox-server` | `MOX_HOST_ROLE=kb` | **3414** | 同上 |

源码证据：
- `platform/gateway/mox-platform-gateway-svc/src/deployment.rs:6` `enum HostRole { All, Kg, Cloud, Kb, Iam }`；`domain_router()` 对非 `All` 角色只装配该域路由，其余域不挂。
- `src/lib.rs:415` `serve_forever` 读 `MOX_HOST_ROLE`（默认 `all`）→ `build_host_router`；`All` 走 `modules::build_module_routers`（全内嵌），否则走 `deployment::domain_router`。
- `docker-compose.domains.yml`：`fused` profile = `MOX_HOST_ROLE=all` 单容器 3080（默认）；`split` profile = 4 个同镜像容器各设 `MOX_HOST_ROLE=kg/cloud/kb/iam` + `MOX_GATEWAY_PORT=3411/3412/3414/3413`，由 nginx `entry` 统一 3080 入口。
- 为何默认不开：`docs/api/PORT-REGISTRY.md §6.4`——默认企业部署由五进程收敛为四进程，KG/KB/Cloud/IAM 继续由网关内嵌；独立二进制 3411–3414 不进默认启动链路（单二进制融合部署最简、前端单入口 3080、SQLite 单副本 PVC）。旧 `mox-kg-server/mox-cloud-server/mox-iam-server/mox-kb-server` 独立进程端口已由 8101–8104 迁至 3411–3414 且仅作可选扩展。

---

## 三、调用方向与协议（谁调谁）

| 调用方 | 被调方 | 协议 | 触发 / 环境变量 | 证据 |
|---|---|---|---|---|
| 外部客户端 | 网关 :3080 | HTTP | 前端 vite `/api` 代理 → 3080 | `lib.rs:464 axum::serve` |
| 网关 :3080 | operator-server :3001 | **HTTP 反向代理**（reqwest） | `ORCHESTRATOR_URL`（默认 `http://127.0.0.1:3001`），未命中的 `/api/*` catch-all | `proxy.rs:43-44`、`proxy.rs:98-101` |
| 网关 :3080 | PrimiFlow :8000 | HTTP 反向代理 | `PRIMIFLOW_URL`（默认 :8000），仅 `/api/projects/*` | `proxy.rs:87-96` |
| 网关 :3080 | alliance scheduler :3100 | **HTTP**（reqwest，10s 超时） | `MOX_ALLIANCE_SCHEDULER_URL` | `alliance/sdk/.../alliance_remote.rs:72,128` |
| 网关 :3080 | alliance executor :3200 | **HTTP**（reqwest） | `MOX_ALLIANCE_EXECUTOR_URL` | `alliance_remote.rs:73,139` |
| scheduler :3100 | executor :3200 | **HTTP bridge**（reqwest） | `executor_bridge.base_url`（`config/alliance-scheduler.yml`） | `scheduler main.rs:70 with_executor_url`、`scheduler routes.rs:279 proxy_to_executor` |
| executor :3200 | AI 专家（`mox-ai-expert-svc`） | **进程内库调用**（非网络） | `ExecutorMode::Expert` | `executor Cargo.toml` 依赖 `mox-ai-expert-svc` |

要点：
- **网关 → operator-server:3001** 是「用户鉴权后、业务域透明转发」，网关注入服务令牌（`ORCHESTRATOR_SERVICE_TOKEN` / `OUS_API_TOKEN`）替换用户 JWT——编排器只认 `OUS_API_TOKEN`/`OUS_RBAC_TOKENS`，不认网关签发的用户 JWT（`proxy.rs:35-38,149-158`）。
- **网关 ↔ alliance** 全 HTTP，无 gRPC 业务数据面。网关对 scheduler 管任务 CRUD/专家匹配；对 executor 管执行状态/节点/融合结果；scheduler 内部再桥接 executor 分发 DAG。三者是「网关双侧直连 + scheduler→executor 桥接」的混合拓扑。
- 鉴权本身在**网关进程内**完成（见第四节步骤 2），后端服务不重复验用户 JWT。

---

## 四、跨域请求闭环（逐跳，每跳落 crate / 文件 / 路由前缀）

### 4.1 主链路：专家联盟问答 / 任务编排（`POST /api/alliance/tasks`）

| 步 | 动作 | crate / 源码 | 路由前缀 / 协议 |
|---|---|---|---|
| 1 | 外部请求打到网关 | `mox-platform-gateway-svc`（`lib.rs serve_forever` 绑定 0.0.0.0:3080） | `POST /api/alliance/tasks`（HTTP） |
| 2 | 中间件链：可观测 → CORS → 限流 → **JWT 鉴权** | `mox-platform-gateway-svc`：`auth.rs::auth_middleware` + `validate_token`（真 HMAC-SHA256/HS256 验签、校验 iss/exp，依赖契约 `mox_platform_api::UserInfo`）；IAM 数据 `mox_platform_iam_core::IamRepository`（SQLite `data/mox.db`）；公开路径白名单见 `config.rs public_paths` | 除 `/health`、`/api/auth/login|register|refresh`、`/actuator/health|info` 外全部强制鉴权 |
| 3 | 命中联盟任务域路由 | `mox-alliance-http-sdk`：`alliance.rs::build_alliance_router_with:1749`（`/api/alliance/tasks`） | 前缀 `/api/alliance/*` |
| 4 | 网关 handler 远程优先转发到调度器 | `mox-alliance-http-sdk`：`alliance_remote.rs::remote_create_task:354` → `RemoteAllianceClient.scheduler_post("/tasks")`（reqwest） | 网关 → **scheduler:3100** `POST /tasks`（HTTP） |
| 5 | 调度器落库 + 专家匹配 + 计划生成 | `mox-alliance-scheduler-svc`：`routes.rs:88 create_task` → `mox-alliance-scheduler-core`（`TaskScheduler.submit_task` + `RuleBasedExpertMatcher`，任务仓库 内存/文件快照）；随后 `with_executor_url` 桥接 | scheduler:3100 进程内 + → **executor:3200**（HTTP bridge）分发 DAG |
| 6 | 执行器按 DAG 逐节点跑 AI 专家 | `mox-alliance-executor-svc`：`ExecutorMode::Expert` → `mox-alliance-executor-core` DAG 调度；每专家节点调用进程内 `mox-ai-expert-svc` | executor:3200 进程内库调用 |
| 7 | 融合策略汇总 | executor 汇总各节点输出 → `/tasks/:id/result`（FusionOutput，含 `node_contributions`）；网关侧 `alliance_remote.rs::remote_fusion_result:798` 取结果并 `norm_fusion` 归一化（`weighted`→`weighted_voting` 等） | executor:3200 内部融合 |
| 8 | 状态轮询 / 取结果原路返回 | 网关 `GET /api/alliance/tasks/:id/status` → `remote_status_poll:860`（同时 `scheduler_get(/tasks/:id)` + `executor_get(/tasks/:id/status)` 合并）；`GET /api/alliance/tasks/:id/fusion-result` → `remote_fusion_result`；统一 `api_ok` 信封 | 网关 → 外部（HTTP 响应原路） |

> 关键纠偏：**operator-server:3001 不在这条联盟任务域闭环里**。联盟任务域在网关进程内由 `mox-alliance-http-sdk` 直接后连 3100/3200；3001 仅在请求落入「业务域 catch-all `/api/*`」时才被网关反代命中。这与「网关→3001→3100」的线性假设不符，以上述源码为准。
>
> 另：网关内 `experts_*` 七模块（`experts_registry/collaboration/session/dispatcher/graph/orchestration/ext`，共享 `ExpertsSharedState`）是另一组「专家广场/会话/图谱」本地能力面，不参与远程 3100/3200 任务执行链路。

### 4.2 对照链路：普通业务请求（KG 图谱 `/api/graph/*`）

| 步 | 动作 | crate / 源码 | 路由前缀 / 协议 |
|---|---|---|---|
| 1 | 外部请求到网关 | `mox-platform-gateway-svc` :3080 | `GET/POST /api/graph/...`（HTTP） |
| 2 | 同一套鉴权中间件 | `auth.rs::auth_middleware`（JWT HS256） | 同上 |
| 3 | 未命中网关原生路由（`/api/system`、`/api/security`、`/kg/v1`、`/api/alliance`、`/rbac/v1` 等均更具体优先） | 落入兜底反代 | `proxy.rs::build_proxy_router:98` `/api` fallback |
| 4 | 网关透明反代到业务宿主并注入服务令牌 | `proxy.rs::proxy_handler:109`（替换 Authorization 为 `OUS_API_TOKEN`） | 网关 → **operator-server:3001**（HTTP） |
| 5 | 编排器计算图谱 | `mox-platform-orchestrator-svc` → `mox-kg-algo-core`（知识图谱算法） | 3001 进程内 |
| 6 | 响应透传回外部 | `proxy_handler` 拷贝状态码/响应体 | 3001 → 网关 → 外部 |

> 补充：网关自身还**进程内**内嵌一套 KG HTTP 适配 `/kg/v1/*`（`lib.rs:72 use mox_kg_service_svc::http_adapter`；`modules.rs:116 build_kg_ai_router`），该路径**不离开 3080**、不反代 3001。即「KG 有两个入口形态」：`/kg/v1/*` 走网关内嵌 adapter，`/api/graph/*` 走反代到 3001 的 kg-algo-core。

---

## 五、最终明确结论

1. **网关内嵌哪些域（默认 `MOX_HOST_ROLE=all`，单进程 3080 内）**：KG + AI 引擎（`mox-kg-service-svc` http_adapter，`/kg/v1/*`）、KB（`mox-kb-svc`，对外 `/api/kb/*`）、Cloud（`/cloud/v1/*`）、IAM（`/api/system/*` `/api/security/*`）、RBAC（`/rbac/v1/*`）、Voice/Melody、专家联盟任务域（`/api/alliance/*`）及专家广场 7 模块——全部由 `modules.rs::build_module_routers` 在进程内 merge，并统一挂 `auth_middleware`。
2. **网关后连哪些外部进程**：
   - operator-server **:3001**（HTTP 反向代理，`ORCHESTRATOR_URL`，业务域 `/api/*` 兜底 + 服务令牌注入）；
   - PrimiFlow **:8000**（HTTP 反向代理，仅 `/api/projects/*`）；
   - alliance scheduler **:3100**（HTTP，`MOX_ALLIANCE_SCHEDULER_URL`，任务/专家）；
   - alliance executor **:3200**（HTTP，`MOX_ALLIANCE_EXECUTOR_URL`，执行状态/节点/融合）。
3. **鉴权落点**：网关 `auth.rs` 自做 HS256 验签（`JWT_SECRET`，`mox_platform_api::UserInfo` 契约）；IAM 账户/角色/权限数据由 `mox-platform-iam-core`（SQLite）提供；网关 crate 未直接依赖 `mox-auth-core` / `mox-rbac-engine`（grep 0 命中），RBAC 展示层复用同一 IAM 仓储（`rbac.rs`）。
4. **端口纪律**：本拓扑仅引用 3080 / 3001 / 3100 / 3200（独立扩展 3411/3412/3413/3414），与 `docs/api/PORT-REGISTRY.md §3.1/§3.2/§3.3/§6.4` 一致；不出现 8080、五进程等历史形态。

---

## 附：核实文件清单

- 网关：`platform/gateway/mox-platform-gateway-svc/src/{main.rs,lib.rs,config.rs,auth.rs,modules.rs,deployment.rs,proxy.rs,rbac.rs,alliance.rs,alliance_remote.rs(壳)}`
- 联盟 HTTP SDK：`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/{alliance.rs,alliance_remote.rs}`
- 编排器：`platform/domains/platform/svc/mox-platform-orchestrator-svc/{Cargo.toml,src/main.rs,src/subservers.rs,src/routes/ai_engine.rs}`
- 联盟服务：`platform/domains/alliance/svc/mox-alliance-scheduler-svc/{Cargo.toml,src/bin/main.rs,src/routes.rs}`、`.../mox-alliance-executor-svc/{Cargo.toml,src/bin/main.rs}`
- 部署/权威：`scripts/startup/start-mox-enterprise.ps1`、`docker-compose.domains.yml`、`docs/api/PORT-REGISTRY.md`
