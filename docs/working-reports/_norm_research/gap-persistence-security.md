# 企业级就绪度差距核查 —— 持久化 × 安全

- 范围：`platform/gateway/mox-platform-gateway-svc`(:3080 唯一入口) + `alliance` scheduler/executor(:3100/:3200) + `platform` orchestrator(:3001) + 网关内嵌域(kg/kb/cloud/iam/experts)
- 方法：只读静态核查，每条结论落到 `crate/文件:行号`
- 分级：**P0**=上生产必须 / **P1**=加固 / **P2**=锦上添花
- 日期：2026-09-16

> 总体结论：网关边缘的"认证(Authentication)"做得扎实（HS256 真验签、release 强制 JWT_SECRET、CORS 收紧、限流）；
> 但**授权(RBAC)在网关 handler 层基本缺位**，且**用户身份在网关→编排器边界被降级为单一 Admin 服务令牌**；
> 持久化呈"两极分化"——IAM/Experts/KV 已落 SQLite/磁盘，但**联盟任务与 DAG 运行态在网关内嵌路径上纯内存**。

---

## 一、紧凑差距矩阵（速览）

| # | 维度 | 差距 | 位置(文件:行) | 级别 |
|---|------|------|----------------|------|
| S1 | 安全 | **任意已登录用户可自助签发 admin 级 API Key（权限提升）** | `system/security.rs:38-59` | **P0** |
| S2 | 安全 | 敏感管理端点(api-keys CRUD/审计日志/权限查询)无任何角色检查，仅"有 JWT 即放行" | `system/security.rs:26,63,97`; `rbac.rs:73-89` | **P0** |
| S3 | 安全 | 网关剥离用户 JWT、向下游注入单一 Admin 服务令牌，端到端 RBAC 退化为单 Admin | `proxy.rs:149-158`; `orchestrator/main.rs:716-720`; `rbac_middleware.rs:269-272` | **P0** |
| P1 | 持久 | **网关内嵌联盟任务/DAG 执行态/日志纯内存，重启全丢** | `http-sdk/alliance.rs:293-300`; `executor-core/dag_engine.rs:58` | **P0** |
| P2 | 持久 | API Key 只写 SQLite、鉴权中间件只读内存表，重启后已发 Key 全部失效 | `auth.rs:39,136-139`; `system/security.rs:48` | P1 |
| S4 | 安全 | 编排器审计事件仅内存 Vec，重启即失；审计签名密钥默认硬编码 | `rbac_middleware.rs:415-458`; `main.rs:332-334` | P1 |
| S5 | 安全 | L0 `/metrics`、`/api/v1/status`、`/api/v1/domains` 完全匿名，泄露指标/路由表/版本 | `lib.rs:281-285,329-332` | P1 |
| P3 | 持久 | 编排器知识图谱运行态、KG 图运行态纯内存，种子加载后变更不回写 | `orchestrator/main.rs:346`; `kg-service/graph_index.rs:760-766` | P1 |
| P4 | 持久 | 网关内监控业务时序、通知/工作区/杂项等子态为进程内 Vec/HashMap | `modules.rs:79-90`; `routes.rs:78` | P2 |
| S6 | 安全 | 编排器 `OUS_RBAC_TOKENS` 未配置时为"兼容模式"（虽默认关，需显式 OUS_AUTH_COMPAT=1 才放宽） | `main.rs:319-323,727-737` | P2 |

---

## 二、A. 持久化核查

### A.1 联盟 scheduler / executor 任务仓储

任务仓储抽象 `TaskRepository`（trait），三种后端实现于
`platform/domains/alliance/core/mox-alliance-scheduler-core/src/storage.rs:25-34`：

| 实现 | 落盘介质 | 重启恢复 | 证据 |
|------|----------|----------|------|
| `InMemoryTaskRepository` | 进程内 `RwLock<HashMap<Uuid,Task>>` | **不恢复，全丢** | `storage.rs:37-72` |
| `FileTaskRepository` | 全量 JSON 快照 `./data/alliance_tasks.json`，每次写原子 rename | **可恢复任务元数据/状态** | `storage.rs:80-174`; 装配 `scheduler-svc/server.rs:125,136-137` |
| `BatchedFileTaskRepository` | 同上但延迟 flush，崩溃丢失未 flush 写 | 半恢复 | `storage.rs:186-336` |

**默认后端结论：**
- 独立 scheduler-svc(:3100)：默认 **File 快照**（`server.rs:107,125` 注释明确"未设置时默认 file"），环境变量 `MOX_ALLIANCE_STORAGE_MODE=memory` 才切纯内存。任务本身**可跨重启恢复**。
  - 但注意是**全量 JSON 快照**（每次 `save` 序列化整个 `Vec<Task>`，`storage.rs:118-150`），非事务型 DB，高并发/大规模下有放大与锁风险（P2 工程性）。
- **网关内嵌联盟路由（`/alliance/v1/*`、`/api/alliance/*`）= 纯内存**：
  网关 `alliance.rs:2` 仅是 `mox_alliance_http_sdk` 的再导出；其 `AllianceGatewayState::new()`
  `tasks: Arc::new(InMemoryTaskRepository::new())`（`http-sdk/alliance.rs:295`），
  另加 `execution: Arc<RwLock<HashMap<Uuid,ExecutionState>>>`（`alliance.rs:297`）与 SSE 日志广播 `broadcast`（`alliance.rs:298`）。
  → **经网关创建的任务、DAG 节点执行态、专家结果、实时日志，重启即全部丢失。**

**DAG / 计划 / 专家结果：**
- executor 的 DAG 运行态 `states: Arc<RwLock<HashMap<Uuid, TaskExecutionState>>>`
  （`mox-alliance-executor-core/src/dag_engine.rs:58,123,229`），**全仓无任何 sqlite/sled/rocksdb/落盘调用**，纯内存。
- 专家注册表：scheduler 侧 `RuleBasedExpertMatcher` 进程内（`http-sdk/alliance.rs:195-291` 每次启动重种 6 个内置专家）。

### A.2 网关内嵌域持久化介质

| 域 | 路由前缀 | 持久化介质 | 重启恢复 | 证据 |
|----|----------|------------|----------|------|
| IAM | `/api/system/*` `/api/security/*` `/rbac/v1/*` | **SQLite** 文件 `data/mox.db`，22 张表 + seed | ✅ 真实落盘 | `lib.rs:226-236` |
| Experts(智能体集群) | `/api/experts/*` | **SQLite** `data/experts.db`（experts/sessions/messages/graph_nodes/edges/bookings），含 JSON→SQLite 一次性迁移 | ✅ | `experts_db.rs:7,34,75,87-158,604` |
| KG | `/kg/v1/*` `/graph/v1/*` | **内存**：图从种子文件 `read_to_string` 加载；索引为 `Mutex<HashMap>` | ❌ 运行期变更丢失 | `kg_graph.rs:92`; `graph_index.rs:760-766` |
| KB | `/api/kb/*` | **本地磁盘**对象存储 `./data/store`(chunks/kv/objects/refs) | ✅ 文件 | `mox-kb-svc/src/lib.rs:71,74`; `document.rs:208` |
| Cloud | `/cloud/v1/*` `/s3/*` | **本地磁盘**对象存储 | ✅ 文件 | `cloud.rs`（域描述 `routes.rs:74`） |
| Monitor | `/api/monitor/*` | 运行时指标内存；`business_timeseries` 描述自承"待接入历史存储" | ❌ 历史指标不落盘 | `routes.rs:78` |
| Enterprise 子态 | `/api/enterprise/*` 通知/工作区/杂项/kb_ext | 进程内 `Vec`/`HashMap`（`ModuleStates` 统一 new） | ❌ | `modules.rs:79-90` |

### A.3 operator-server(:3001, mox-platform-orchestrator-svc)业务数据落点

| 数据 | 介质 | 证据 |
|------|------|------|
| 算子包市场 | **文件 JSON** `$OUS_HOME/market/packages/<id>.json`，启动 seed + 迁移旧 `./data/market` | `market.rs:11,15,332,858-866` |
| 知识图谱(`build_knowledge_graph`) | **内存** | `main.rs:346` |
| 审计事件 | **内存** `MemoryAuditSink`（`Mutex<Vec<AuditEvent>>`） | `rbac_middleware.rs:415-458`; `main.rs:330-331` |
| RBAC 令牌 | 环境变量(`OUS_API_TOKEN`/`OUS_RBAC_TOKENS`)，进程内 HashMap | `rbac_middleware.rs:265-305` |
| WASM 插件 | 目录 `./plugins` 加载 | `main.rs:342` |

### A.4 持久化结论：哪些重启会丢（P0 候选）

**P0 — 重启即丢的关键运行态：**
1. **网关内嵌联盟**：经 `/alliance/v1/*`、`/api/alliance/*` 创建的任务、DAG 节点状态、专家结果、执行日志 —— 纯内存（`alliance.rs:293-300` + `dag_engine.rs:58`）。生产四进程编排下，调度走独立 scheduler(文件快照)尚可，但**经 :3080 直挂的联盟请求走的是网关内 InMemory 后端**，与 scheduler 进程不是同一份状态。
2. **执行器 DAG 运行态**：executor 侧无任何持久化，重启后在跑的 DAG 节点上下文丢失。

**P1 — 重启即丢但非"在途业务"：**
3. 编排器审计轨迹（内存 Vec）——合规留痕易失。
4. KG 图运行态 / 编排器知识图谱（种子外的新增边、索引）。

**已落盘、重启可恢复：** IAM(SQLite)、Experts(SQLite)、KB(磁盘对象)、Cloud(磁盘)、市场包(文件 JSON)、独立 scheduler 任务(文件快照)。

---

## 三、B. 安全核查

### B.1 网关路由装配与鉴权挂载

装配入口 `build_host_router`（`lib.rs:271-343`）。外层 `.layer` 链为：可观测 → CORS → 限流（`lib.rs:333-341`），**不含鉴权**；鉴权按路由组 `route_layer(auth_middleware)` 逐组挂。

| 路由组 | 挂载方式 | 证据 |
|--------|----------|------|
| `actuator` 管理面 | 整组 `route_layer(auth_middleware)` | `lib.rs:277-280` |
| `l0`（`/health` `/api/v1/status` `/api/v1/domains` `/metrics`） | **无鉴权层，直接 merge** | `lib.rs:281-285,331` |
| `protected`（全部业务域） | 合并后**一次性** `route_layer(auth_middleware)` | `modules.rs:183-188` |

`modules.rs:114-181` 把 KG/AI、KB(`/api/kb`)、alliance、system、security、rbac、voice、melody、cloud、**兜底反代 proxy**、monitor、workspace、projects、experts×7、misc、kb_ext、notification、enterprise 全部 merge 进同一 router，再统一挂鉴权（`modules.rs:185-188`）。**因 merge 全部发生在 route_layer 之前，这些业务组均被 JWT/API-Key 覆盖。**

### B.2 鉴权覆盖矩阵

| 路由前缀 | 是否公开 | 公开/受保护理由 | 过鉴权 | 过 RBAC(角色) | 风险 |
|----------|----------|-----------------|--------|----------------|------|
| `/health` `/actuator/health` `/actuator/info` | 是 | 探针/构建信息 | 否 | — | 低（设计如此） |
| `/metrics` | **是** | l0 未挂鉴权 | **否** | — | **中**（泄露指标+限流统计） |
| `/api/v1/status` `/api/v1/domains` | **是** | l0 未挂鉴权 | **否** | — | **中**（泄露版本/全量路由表，侦察面） |
| `/actuator/env` `/actuator/logs` `/actuator/api/:id/enable|disable` | 否 | 管理面 | ✅ JWT | 仅"登录即可"，无 admin 角色校验 | 中 |
| `/api/system/*` `/api/security/*` | 否 | 业务组 | ✅ JWT/APIKey | **❌ handler 无角色检查** | **高** |
| `/rbac/v1/*` | 否 | 业务组 | ✅ | 仅 `/current` 用 ApiAuth；`/permissions` 可查任意 user_id | 中 |
| `/api/experts/*` `/alliance/v1/*` `/api/alliance/*` `/api/kb/*` `/kg/v1/*` `/cloud/v1/*` 等 | 否 | 业务组 | ✅ | 无细粒度角色 | 中 |
| `/api/*`（兜底反代→:3001）/ `/api/projects/*`（→:8000） | 否 | 业务组 | ✅（网关侧） | 网关侧鉴通过后**剥离用户身份**，下游只见单一 Admin 令牌 | **高** |
| 编排器直连 `/api/mox/**` `/api/ai/chat` `GET /api/market/**` | 是 | 编排器 `is_public_route` 白名单 | **否**（编排器侧） | — | 仅当 :3001 直接暴露时为裸奔 |

**"裸奔"业务端点判定：**
- 经 :3080 唯一入口，**未发现挂在鉴权之前的业务端点**——业务组全部在 `modules.rs` 统一鉴权层之后；`/metrics`、`/api/v1/*` 是信息泄露类（P1），非业务数据裸奔。
- **真正的 P0 不是"无鉴权裸奔"，而是"鉴权形同虚设"型裸奔**：见 S1/S2——这些端点**过了 auth_middleware**（要一个有效 JWT），但**任何有效 JWT（含低权用户、甚至自签 admin API Key）都能调管理面写操作**。从授权视角等同裸奔。
- 若编排器 :3001 / PrimiFlow :8000 不做网络隔离而直接对外，则 `/api/mox/**`、`POST /api/ai/chat` 为编排器侧公开业务端点（`rbac_middleware.rs:479-493,554-561`）。

### B.3 JWT 验签 / API-Key / OUS_API_TOKEN

- **JWT 验签真实有效**：HS256 HMAC-SHA256，强制 `alg==HS256`（拒 alg=none/RS*），`verify_slice` 恒定时间比较，校验 `iss`/`exp`，`sub/tenant_id/roles` 入 claims（`auth.rs:73-133`）。`JWT_SECRET` 缺失时 release 直接 panic（`config.rs:66-94`）。✅
- **dev 后门**：`dev_mode && token=="dev-secret-token"` 直接授 admin（`auth.rs:158-167`）；`dev_mode` 默认 `cfg!(debug_assertions)`（`config.rs:30-32,55`），release 需显式 `MOX_DEV_MODE=1` 才开。release 默认安全，debug 构建需留意。
- **API-Key**：中间件内存表 `HashMap<hash,user>`（`auth.rs:39,136-139`）。运行期创建 key 时 `register_api_key`（`security.rs:48`），吊销时移除（`security.rs:69`）；但**启动时没有从 `data/mox.db` 回灌已存 key**（全仓 `register_api_key` 生产调用点仅此一处 + 测试）。→ **DB 里的 key 重启后鉴权中间件认不出**，API-Key 持久化只做了一半。
- **OUS_API_TOKEN**：不是网关入站校验项，而是**网关→编排器的出站服务令牌**，取 `ORCHESTRATOR_SERVICE_TOKEN` 回退 `OUS_API_TOKEN`（`proxy.rs:50-52`），转发时**剥离客户端 `Authorization` 改注服务令牌**（`proxy.rs:129-154`）。未配置时仅 warn 后裸发（`proxy.rs:156-158`），由编排器自己 401 兜底。

### B.4 RBAC 是否真做了授权

- 网关 `auth_middleware` 只做"认证 + 注入 UserInfo"，不做角色判定（`auth.rs:143-201`）。
- `ApiAuth` extractor 全仓仅 3 处使用：定义 `auth.rs:228`、`/rbac/v1/current`(`rbac.rs:93`)、当前用户(`system/permission.rs:12`)。
- **敏感管理 handler 一律不接收 ApiAuth、不查 roles**：
  - `POST /api/security/api-keys`：任意认证用户可**新签绑定 `DEFAULT_USER="admin-user"` 的 key**（`security.rs:38-49`，`DEFAULT_USER` 见 `rbac.rs:28`），validate 还硬编码 `["read","write"]`（`security.rs:89`）→ **横向/垂直越权到 admin**。
  - `DELETE /api/security/api-keys/:id`、`GET /api/security/api-keys`、`GET /api/security/audit-log`：仅 `State`，无角色门槛（`security.rs:26,63,97`）。
  - `GET /rbac/v1/permissions?user_id=`、`get_permissions`：可查任意 user_id，无"只能查自己"约束（`rbac.rs:81-88`; `permission.rs:25-37`）。
- 编排器侧**有完整 RBAC**（6 角色、路由→权限矩阵、跨租户隔离、HMAC 签名审计，`rbac_middleware.rs:33-185,545-632`），但因 S3 网关把所有人替换成同一个 Admin 服务令牌（`main.rs:716-720`），**该 RBAC 对"来自网关的所有用户"恒等于 Admin**，细粒度授权在端到端口径下不生效。

---

## 四、P0 / P1 / P2 分级清单

### P0（上生产必须修）
- **S1 任意登录用户可签发 admin 级 API Key**：`system/security.rs:38-49` → 管理端点加 admin 角色校验。
- **S2 管理面/审计/权限查询无授权**：api-keys CRUD、audit-log、`/rbac/v1/permissions` 全靠"有 JWT"。
- **S3 用户身份在网关→下游被降级为单一 Admin 服务令牌**：后端无法做真正的用户级授权（`proxy.rs:149-158`）。
- **P1 网关内嵌联盟任务/DAG/专家结果/执行日志纯内存**（`http-sdk/alliance.rs:293-300`、`dag_engine.rs:58`），重启在途业务状态全丢；需统一落到与独立 scheduler 相同的 File/SQLite 后端。

### P1（生产前应加固）
- **P2 API-Key 持久化断链**：启动期从 `data/mox.db` 回灌 key 到 `AuthMiddleware.api_keys`。
- **S4 审计轨迹易失**：编排器 `MemoryAuditSink` 落盘/送日志；`OUS_AUDIT_KEY` 缺失时应拒绝启动而非用硬编码默认值（`main.rs:332-334`）。
- **S5 匿名信息泄露**：`/metrics`、`/api/v1/status`、`/api/v1/domains` 收进鉴权或至少裁剪版本/路由表。
- **P3 KG/编排器图运行态内存**：决定是否需要持久化增量。

### P2（锦上添花）
- **P4** 监控业务时序、通知/工作区/杂项等子态落盘。
- **S6** 编排器"兼容模式"（前缀推角色）保持默认关；独立 scheduler 全量 JSON 快照改为增量/DB。
- debug 构建的 `dev-secret-token` 后门在 CI/预发镜像需确认不被带入 release。

---

## 五、关键证据索引（文件:行号）

- 路由装配：`platform/gateway/mox-platform-gateway-svc/src/lib.rs:266-343`（l0 公开 281-285；protected 鉴权 290-295；actuator 鉴权 277-280）
- 业务组统一鉴权：`src/modules.rs:108-189`（merge 顺序 114-181；route_layer 185-188）
- JWT HS256：`src/auth.rs:73-133`；public_path 前缀匹配 64-66；dev 后门 158-167；API-Key 内存表 39,136-139
- 配置：`src/config.rs:45-58`(public_paths)、`66-95`(JWT_SECRET/dev_mode)
- 管理端点无角色校验：`src/system/security.rs:26,38-49,63,79,97`；`src/rbac.rs:73-89`；`src/system/permission.rs:12,17-37`
- 反代降级身份：`src/proxy.rs:50-52,129-158`
- 编排器 RBAC：`platform/domains/platform/svc/mox-platform-orchestrator-svc/src/rbac_middleware.rs:89-185,479-493,545-632`；`main.rs:300-339,703-737,793+`
- 联盟任务仓储：`.../alliance/core/mox-alliance-scheduler-core/src/storage.rs:25-174`；装配 `.../svc/mox-alliance-scheduler-svc/src/server.rs:107-142`
- 网关内嵌联盟纯内存：`.../alliance/sdk/mox-alliance-http-sdk/src/alliance.rs:293-300`；DAG 内存 `.../executor-core/src/dag_engine.rs:58`
- 持久化：IAM SQLite `lib.rs:226-236`；Experts SQLite `experts_db.rs:34,75,87-158`；KB 磁盘 `mox-kb-svc/src/lib.rs:71`；市场文件 `market.rs:11,858`
