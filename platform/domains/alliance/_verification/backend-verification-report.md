# 专家联盟后端代码事实核验报告

> 核验日期：2026-09-27
> 核验范围：`platform/domains/alliance/`（16 crates）+ `gateway/mox-platform-gateway-svc/src/alliance/`（11 文件）+ 网关路由注册处
> 权威文档：`docs/expert-alliance/CURRENT-ARCHITECTURE.md` V1.1（2026-09-24）
> 核验方式：全量 grep + 逐文件行号精读；所有结论附 `文件:行号`；零命中亦显式标注。

---

## 一、核验摘要表

| # | 核验项 | 权威文档 V1.1 声称 | 代码实测 | 判定 | 代码位置 |
|---|--------|-------------------|---------|------|---------|
| 1 | 16 crates 拓扑 | §1.1 列出 api/core(6)/proto(4)/sdk(2)/svc(3)=16 | 实测 16 个 crate 目录均含 Cargo.toml，api/ 根目录即 mox-alliance-api | ✅一致 | 见 §四 清单 |
| 2 | 网关 alliance/ 11 文件 | §1.2 列出 9 experts_*.rs + mod.rs + registry_client.rs = 11 文件，行数 9924 | 实测 11 个 .rs，行数逐文件与文档完全一致（collaboration 2227 / common 1014 / db 721 / dispatcher 1173 / ext 366 / graph 1099 / orchestration 1236 / registry 1009 / session 959 / mod 28 / registry_client 92） | ✅一致 | `gateway/src/alliance/` 目录列表 |
| 3 | 端口 gateway:3080 | §2.1 gateway=3080 | `main.rs:27` `.unwrap_or(3080)`；`config.rs:181` `port: 3080` | ✅一致 | gateway/src/main.rs:27, config.rs:181 |
| 4 | 端口 scheduler:3100 | §2.1 scheduler=3100 | `boot-config/src/lib.rs:65,184` `port: 3100`；bin/main.rs:45 注释 PORT-NORM-001 | ✅一致 | boot-config/src/lib.rs:65,184 |
| 5 | 端口 executor:3200 | §2.1 executor=3200 | `boot-config/src/lib.rs:247` `port: 3200`；executor app_state 默认 3200 | ✅一致 | boot-config/src/lib.rs:247 |
| 6 | 端口 registry:3400 | §2.1 registry=3400 | `registry-svc/app_state.rs:50` `bind_addr: "0.0.0.0:3400"` | ✅一致 | registry-svc/src/app_state.rs:50 |
| 7 | scheduler 6 条路由 | §3.1/§6.2 routes.rs:23-32 注册 6 路径 | 实测 6 路径：/health, /metrics, /leadership, /tasks, /tasks/:task_id, /experts/search（行 23-32 一致） | ✅一致 | scheduler-svc/src/routes.rs:23-32 |
| 8 | scheduler 动词面数 | §6.2 声称"9 个动词面" | 实测 8 个动词面（health/metrics/leadership 各 1 + tasks 2 + tasks/:id 2 + experts/search 1 = 8） | ❌不一致（文档多算 1） | scheduler-svc/src/routes.rs:23-32 |
| 9 | 7 种协作模式 | §3.7 planner.rs:81-92 七条 match 分支 | 实测 match 块在 :81-91，7 臂：Parallel/Sequential/Voting/Hierarchical/Debate/Iterative/Dynamic（:88 Dynamic 调 generate_dynamic_plan） | ✅一致（行号实际 81-91，文档写 81-92 差 1 行） | scheduler-core/src/planner.rs:81-91 |
| 10 | /api/experts 43 路径 | §6.1 去重 43 | 实测 actuator.rs ROUTES 中 /api/experts/* 去重路径 = **43** | ✅一致 | gateway/src/actuator.rs:592-645 |
| 11 | /api/alliance 20 路径 | §6.1 去重 20 | 实测 /api/alliance/* 去重路径 = **20** | ✅一致 | gateway/src/actuator.rs:518-537 |
| 12 | /ws/v1 零 WebSocket | §6.1 补记 WebSocketUpgrade 零命中 | 全 crate grep `WebSocketUpgrade` = **0 hits** | ✅一致 | grep 全 domains/alliance + gateway/src |
| 13 | favorites 进程内即失 | §5 favorites=进程内 HashSet | `experts_common.rs:472` `Arc<Mutex<HashSet<String>>>`；全 gateway alliance grep favorites 持久化 = **0 hits** | ✅一致 | experts_common.rs:472,510 |
| 14 | plans 进程内即失 | §5 plans=HashMap 进程内 | `experts_common.rs:468` `Arc<Mutex<HashMap<String, CollaborationPlan>>>`；experts_db.rs 无 plans 表 | ✅一致 | experts_common.rs:468,508 |
| 15 | orchestration_history 进程内 | §5 history=Vec 进程内 | `experts_common.rs:470` `Arc<Mutex<Vec<OrchestrationRecord>>>`；experts_db.rs 无对应表 | ✅一致 | experts_common.rs:470,509 |
| 16 | SQLite WAL + busy_timeout | §8 存储行 | `PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=...` 在 storage.rs:489,913；gateway experts_db.rs:78 同设 | ✅一致 | scheduler-core/src/storage.rs:489,913；gateway experts_db.rs:78 |
| 17 | busy_timeout 可配 | §8 MOX_ALLIANCE_SQLITE_BUSY_MS | `storage.rs:412` `std::env::var("MOX_ALLIANCE_SQLITE_BUSY_MS")`，默认 5000 | ✅一致 | scheduler-core/src/storage.rs:408-412 |
| 18 | HA 拒绝 file/memory 启动 | §8 "JSON 文件仓库单写者，开 HA 直接拒绝启动" | `server.rs:272-276` `anyhow::ensure!` 当 ha_on && mode!="sqlite" 时 panic 启动 | ✅一致 | scheduler-svc/src/server.rs:272-276 |
| 19 | registry-svc:3400 独立可启动 | §8 registry 已落地 | registry-svc 有独立 main.rs/bin、server.rs:83 TcpListener::bind、app_state.rs:50 默认 3400 | ✅一致 | registry-svc/src/server.rs:83, app_state.rs:50 |
| 20 | 10:1:1 分级心跳聚合 | §8 aggregation.rs + POST /api/registry/aggregated-heartbeat | `registry-core/src/aggregation.rs:11,489` 注释 10:1:1；`registry-svc/routes.rs:55` 注册该 POST | ✅一致 | registry-core/src/aggregation.rs:11；registry-svc/src/routes.rs:55 |
| 21 | LeaseStore/LeaderElector/SqliteLeaseStore | §8 leadership.rs 三 trait/结构 | `leadership.rs:110` trait LeaseStore；`:194` struct LeaderElector；`storage.rs:890` struct SqliteLeaseStore | ✅一致 | scheduler-core/src/leadership.rs:110,194；storage.rs:890 |
| 22 | fencing 任期 epoch | §8 每次抢占 epoch+1 | `leadership.rs:57-58` `pub epoch: u64`；`:100` 抢占 `epoch: cur.epoch+1`；`:186` `accepts(epoch)` 校验 | ✅一致 | scheduler-core/src/leadership.rs:57,100,186 |
| 23 | leader 专属对账 + 孤儿接管 | §8 reconcile_active_tasks + stall | `ha.rs:200` 仅 leader 调 `reconcile_active_tasks(stall, now)`；`:205` fencing `accepts(view.epoch)`；stall 默认 300000ms（ha.rs:83） | ✅一致 | scheduler-svc/src/ha.rs:200,205,83 |
| 24 | MOX_API_CRYPTO=sm4 6 处 | §8 网关/调度/执行/注册/桥/SDK 6/6 | 实测 6 处：gateway lib.rs:332 / scheduler routes.rs:36 / executor routes.rs:93 / registry routes.rs:60 / executor_bridge.rs:112-139 / alliance_remote.rs:155-171 | ✅一致 | 见 §二 详表 |
| 25 | MOX_ALLIANCE_STORAGE_MODE 三态 | §5 memory/file/sqlite | `server.rs:113-124` 读 env，默认 "file"；`:142-171` match memory/sqlite/_(file)；executor state_sink.rs:175 同 | ✅一致 | scheduler-svc/src/server.rs:113-171；executor-svc/src/state_sink.rs:175-200 |
| 26 | MOX_ALLIANCE_REMOTE_MODE off/auto | §4.1 | `alliance_remote.rs:101` 读 env，默认 "auto"；:103 `if mode=="off" return None` | ✅一致 | http-sdk/src/alliance_remote.rs:101-105 |
| 27 | availability.status 登记值非探活 | §1.2 补记 | `experts_registry.rs:127-128` 从请求 body 写入 `exp.availability.status = s`；registry-svc health_probe 默认关闭 | ✅一致 | gateway experts_registry.rs:127-128；registry-svc app_state.rs:56 |
| 28 | experts_db.rs 零 fs::write | §1.2 补记 | fs_write grep 在 gateway alliance/ 下 experts_db.rs 零命中（仅 cloud.rs/misc.rs/file_storage 有） | ✅一致 | grep fs_write 结果 |
| 29 | engine_status 恒 "running" | §1.2 补记 | `experts_dispatcher.rs:536` 硬编码 `"engine_status": "running"` | ✅一致 | gateway experts_dispatcher.rs:536 |
| 30 | circuit_breakers 恒 [] | §1.2 补记 | dispatcher.rs:494 `let circuit_breakers: Vec<Value> = if let Some(map) = fc_guard`；fc_guard 来自 `get_failure_counts()`，初始为 None → 空 Vec；失败经 ensure_failure_map 写入后非空 | ⚠️部分一致（初始空，但失败后会填充，非"恒 []"） | gateway experts_dispatcher.rs:493-542,794,828 |

---

## 二、§8/§9 增量核验详情

> V1.1 明确标注 §8/§9 "本轮未重核，沿用 V1.0"。本节逐行给真实代码坐标。

### 2.1 §8 差距表逐行核验

**(1) 存储：SQLite WAL + busy_timeout**
- WAL PRAGRA 执行点：`scheduler-core/src/storage.rs:489` 与 `:913`（两处 open 连接均设）：
  ```
  "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout={busy_ms};"
  ```
- busy_timeout 可配：`storage.rs:408-412` 读 `MOX_ALLIANCE_SQLITE_BUSY_MS`，默认 5000ms。
- 网关侧 experts.db 同样 WAL：`gateway/src/alliance/experts_db.rs:78` `conn.pragma_update(None, "journal_mode", "WAL")`，busy_timeout=5s（:76）。
- **判定：✅ 属实。**

**(2) JSON 文件仓库"单写者，开 HA 拒绝启动"**
- 拒绝逻辑：`scheduler-svc/src/server.rs:272-276`：
  ```rust
  anyhow::ensure!(
      Self::ha_storage_ok(ha_cfg.is_some(), &storage_mode, self.task_repository.is_some()),
      "MOX_ALLIANCE_HA_MODE 要求 MOX_ALLIANCE_STORAGE_MODE=sqlite（当前 {storage_mode}）：..."
  );
  ```
- `ha_storage_ok` 在 `:108-110`：`!ha_on || injected_repo || mode == "sqlite"`。
- FileTaskRepository 单写者注释：`server.rs:127` "file：文件快照持久化到 ./data/alliance_tasks.json"；storage.rs:168,288 用 `std::fs::write(&tmp, &raw)` 全量重写（非增量 upsert）。
- **判定：✅ 属实。**

**(3) registry-svc:3400 独立可启动 + registry-proto 契约层**
- 独立 bin：`registry-svc/src/bin/main.rs:9`；server.rs:19 `state.config.bind_addr.parse()`；:83 `TcpListener::bind(addr)`。
- 默认地址：`app_state.rs:50` `bind_addr: "0.0.0.0:3400"`，可被 `MOX_ALLIANCE_REGISTRY_ADDR` 覆盖（:75）。
- registry-proto 契约层：`proto/mox-alliance-registry-proto/src/` 含 `errors.rs / traits.rs / types.rs`（lib.rs 声明 3 模块）。
- **判定：✅ 属实。**

**(4) 10:1:1 分级心跳聚合**
- 纯计算 crate：`core/mox-alliance-registry-core/src/aggregation.rs:11` "node→rack→cell 分级心跳聚合（10:1:1）"；:489 "10 rack × fan_in 10 = 100 条叶心跳 → 1 条 cell 续约"。
- HTTP 端点：`registry-svc/src/routes.rs:55` `"/api/registry/aggregated-heartbeat"`；storage.rs:276 注释说明降流。
- **判定：✅ 属实。**

**(5) 调度器多活 leadership.rs**
- 三 trait/结构：
  - `trait LeaseStore`：`leadership.rs:110`
  - `struct LeaderElector`：`leadership.rs:194`
  - `struct SqliteLeaseStore`：`storage.rs:890`（impl LeaseStore 在 :951）
- fencing：`leadership.rs:57-58` `pub epoch: u64`；:100 抢占 `epoch: cur.epoch + 1`；:186 `pub fn accepts(&self, epoch: u64) -> bool { self.leader && self.epoch == epoch }`。
- 租约表：`storage.rs:939` `CREATE TABLE IF NOT EXISTS alliance_leader_lease (scope PK, holder, epoch, expires_at_ms)`。
- leader 专属对账：`scheduler-svc/src/ha.rs:200` `scheduler.reconcile_active_tasks(stall, chrono::Utc::now())`；:196 `if !view.leader { ... continue }`。
- 孤儿接管：stall 窗口 `ha.rs:60` 注释"执行器明确不认识且超过此时长无进展的任务才判为孤儿"；默认 `MOX_ALLIANCE_HA_STALL_MS=300000`（ha.rs:83）。
- fencing 拒绝旧写：`ha.rs:205` `let fenced_out = !lock(&elector).leadership().accepts(view.epoch);`
- 配置项：`ha.rs:12-18` 注释列出 6 个 MOX_ALLIANCE_HA_* 变量。
- **判定：✅ 属实。**

**(6) MOX_API_CRYPTO=sm4 一键开关 6 处**
| # | 层 | 代码位置 | 作用 |
|---|----|---------|------|
| 1 | 网关 | `gateway/src/lib.rs:332` 注释；crypto_middleware 挂载 | 入站解密 + 出站加密信封 |
| 2 | 调度器 | `scheduler-svc/src/routes.rs:36` `.layer(mox_api_crypto::middleware::crypto_middleware)` | 入站解密 |
| 3 | 执行器 | `executor-svc/src/routes.rs:93` 同上 crypto_middleware | 入站解密 |
| 4 | 注册中心 | `registry-svc/src/routes.rs:60` 同上 crypto_middleware | 入站解密 |
| 5 | 桥（ExecutorBridge） | `scheduler-core/src/executor_bridge.rs:112` crypto_post；:120 outbound_headers；:123 seal_request；:139 open_response | 出站加密 + 入站解密 |
| 6 | SDK | `http-sdk/src/alliance_remote.rs:156` outbound_headers；:160 seal_request；:171 open_response | 网关→调度/执行远程调用加密 |
- gzip+SM4-GCM 实际算法实现不在本仓库 alliance/ 内，而在外部 crate `mox_api_crypto`（作为依赖引入，`use mox_api_crypto::...`）。alliance 侧只调用其 `middleware::crypto_middleware` / `client::outbound_headers` / `seal_request` / `open_response`。
- **判定：✅ 6/6 挂载点属实；算法本体在外部 mox-api-crypto crate，本仓库内零 SM4/gzip 实现代码（grep `sm4|SM4|gzip|GzDecoder` 在 alliance/ 仅命中注释 4 处，无实际加密代码）。**

**(7) MOX_ALLIANCE_STORAGE_MODE 三态**
- scheduler 侧：`server.rs:113-124` 读 env；:142 match "memory" → InMemoryTaskRepository；"sqlite" → SqliteTaskRepository；_（默认 file）→ FileTaskRepository。
- executor 侧：`state_sink.rs:175` 读同一 env；:176 match "sqlite" → SqliteExecutionStateSink；"memory" → None（纯内存）；_ → FileExecutionStateSink。
- 默认值：scheduler 默认 "file"（server.rs:121）；executor 默认 file（state_sink.rs:196 `_ =>` 分支）。
- 旧变量兼容：`server.rs:116-119` 兼容旧 `ALLIANCE_TASK_STORE`，deprecated 告警。
- **判定：✅ 属实。**

**(8) MOX_ALLIANCE_REMOTE_MODE off/auto**
- `alliance_remote.rs:101` `let mode = std::env::var("MOX_ALLIANCE_REMOTE_MODE").unwrap_or_else(|_| "auto".to_string());`
- :103-105 `if mode.eq_ignore_ascii_case("off") { return None; }`
- :106-107 读 SCHEDULER_URL / EXECUTOR_URL。
- **判定：✅ 属实。**

### 2.2 §9 关键设计决策 6 条逐条核验

1. **务实优先（先 2 svc + 网关内联）**：✅ 属实。网关内联 alliance/ 模块直接处理 /api/experts/*；scheduler/executor 为可选远程后端（MOX_ALLIANCE_REMOTE_MODE=off 时全本地）。
2. **纯算法与 IO 分离**：✅ 属实。mox-alliance-core 仅 dag/fusion/utils 三模块，无 IO；IO 在 svc 层（state_sink.rs / storage.rs）。
3. **共享状态单点（ExpertsSharedState）**：✅ 属实。`experts_common.rs:456` 定义，集中持有 registry/sessions/graph/plans/history/favorites/audit。
4. **桥接模式（ExecutorBridge trait 可替换）**：✅ 属实。`executor_bridge.rs:55` `async fn health_check(&self)` trait；实现含 HttpExecutorBridge（:366）/ InProcessExecutorBridge（:461）/ Mock（:535,729）。
5. **模块化专家配置（10 大专家独立 LLM 配置，未配置回退全局默认）**：✅ 属实。config-core/examples/domain_experts.rs:108 build_domain_experts() 返回 11 条；server.rs:283-293 `set_global_llm_config` 作为回退。
6. **周期职责单点、请求路径多活（对账用租约，写幂等）**：✅ 属实。ha.rs:200 仅 leader 跑 reconcile；:203-205 注释"租约选主只保证互斥、不保证旧 leader 那一轮不跑完，故这里只观测不追回（对账写本身幂等，双跑不产生错误状态）"；:205 用 accepts(epoch) fencing。租约表与任务表同库（storage.rs:880 注释"同文件另有 SqliteLeaseStore"）。

---

## 三、新发现缺口清单（按严重程度排序）

> 已知缺口（favorites/plans/orchestration_history 进程内、无 WS 只有 SSE、availability.status 登记值、engine_status 恒 running、experts_db 零 fs::write）不重复列出。

### 🔴 高严重度

**N1. 下游三个 svc（scheduler:3100 / executor:3200 / registry:3400）无任何鉴权中间件**
- 现象：scheduler/executor/registry 三个服务的路由层只挂了 `request_tracing_layer` + `crypto_middleware`，没有 JWT/Bearer 校验。
- 证据：
  - `scheduler-svc/src/routes.rs:34-36` 仅 `.layer(request_tracing_layer)` + `.layer(crypto_middleware)`；无 auth。
  - `executor-svc/src/routes.rs:91-93` 同上。
  - `registry-svc/src/routes.rs:59-60` 仅 crypto_middleware。
  - 对比网关：`gateway/src/auth.rs:143` `auth_middleware` 校验 JWT，public_paths 仅 /api/auth/login|register|refresh（config.rs:45-49）。
- 影响：任何能直连 :3100/:3200/:3400 的客户端可绕过网关鉴权，直接 POST /tasks 创建任务、GET /api/registry/experts 拉取专家清单、POST /api/registry/aggregated-heartbeat 续约。crypto_middleware 只做加密不做身份认证。
- 建议：下游 svc 增加 JWT 校验中间件，或至少校验 X-Tenant-Id/X-User-Id 签名。

**N2. 审计事件 Actor 硬编码为 system，无真实用户身份**
- 现象：所有审计事件的 actor 都是 `AuditActor::system()`，不记录是谁执行的操作。
- 证据：`gateway/src/alliance/experts_common.rs:593` `AuditActor::system()`；dispatcher.rs:580/651/749、registry.rs:375/397/419、session.rs:183/409/449/613 调用 emit_audit 时均不传用户 ID。
- 影响：审计链虽有 SHA-256 哈希链防篡改（experts_common.rs:521-574，写入 data/audit/experts-audit.ndjson），但无法追溯"谁干的"——合规审计价值大打折扣。
- 建议：从 ApiAuth extractor 取 UserInfo 注入 AuditActor。

**N3. SQLite 无 schema 版本表，无增量迁移机制**
- 现象：仅有一次性 JSON→SQLite 导入，无 PRAGMA user_version、无 schema_version 表。
- 证据：grep `user_version|schema_version|PRAGMA user_version` 在 alliance/ 与 gateway/src/alliance/ 零命中。experts_db.rs:604 `migrate_json_to_sqlite()` 只做旧 JSON 文件导入后 rename 归档；scheduler-core/storage.rs 用 `CREATE TABLE IF NOT EXISTS` 建表，无版本演进逻辑。
- 影响：未来给 experts/sessions/graph_nodes 加列时，老库不会自动 ALTER TABLE，新代码读旧库会报列不存在。
- 建议：引入 `PRAGMA user_version` + 版本号迁移脚本。

### 🟡 中严重度

**N4. 图谱 graph_nodes/graph_edges 无节点级 CRUD API，只能全量 rebuild**
- 现象：graph 表存在（experts_db.rs:136-152），但对外端点全是只读查询，唯一写操作是 POST /api/expert-graph/rebuild（全量重建）。
- 证据：
  - `gateway/src/alliance/experts_graph.rs:909-916` 8 条路由：get_graph / stats / neighbors / collaborators / path / communities / optimal-team(POST) / rebuild(POST)。
  - grep `INSERT INTO graph_nodes|UPDATE graph|DELETE graph|add_node|create_node` 在 experts_graph.rs 零命中。
  - 写路径只有 `save_graph`（experts_common.rs:637-638）全量 DELETE+INSERT（experts_db.rs:372-408）。
- 影响：业务侧无法通过 API 新增/编辑一条专家关系，只能全量重建；图谱与专家注册脱钩（专家 CRUD 后图谱不自动更新，需手动触发 rebuild）。
- 建议：补 POST /api/expert-graph/nodes、POST /edges 端点，或在专家 CRUD 时自动增量维护图谱。

**N5. registry-svc 主动健康探测默认关闭**
- 现象：health_probe 代码存在且真实发 HTTP GET，但默认 `health_probe_enabled: false`。
- 证据：`registry-svc/src/app_state.rs:56` `health_probe_enabled: false`；server.rs:39 `if state.config.health_probe_enabled { ... spawn probe_task }`；health_probe.rs:56 `impl HealthProbe for HttpHealthProbe` 真实 GET health_check_url。
- 影响：默认部署下，registry 仍只靠心跳超时摘节点，不会主动访问 health_check_url 探活；"进程活着但服务坏"的情况要等心跳 lease 超时（默认 15s，aggregation.rs:139）才暴露。
- 建议：生产默认开启，或在文档明确标注需显式设 MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=true。

**N6. registry_client.rs:26 存在生产路径 .expect() 恐慌点**
- 现象：网关 RegistryClient::new 里 `.expect("failed to build http client")`。
- 证据：`gateway/src/alliance/registry_client.rs:26` `.build().expect("failed to build http client")`。
- 影响：reqwest::Client::build() 失败概率极低（仅在 TLS 初始化失败时），但一旦发生会 panic 整个网关进程。
- 建议：改为返回 Result 并降级为 None。

**N7. 调度器/执行器 /metrics 返回自定义 JSON 快照，非 Prometheus 文本格式**
- 现象：gateway /metrics 是 Prometheus 文本（o11y.rs:21 `use prometheus::{...}`），但 scheduler/executor /metrics 是 `Json(state.metrics.snapshot())`。
- 证据：
  - `scheduler-svc/src/routes.rs:112` `Json(state.metrics.snapshot())`。
  - `executor-svc/src/routes.rs:127` 注释"运行指标快照（纯原子计数 JSON...）"。
  - `scheduler-core/src/metrics.rs:9` 基于 `std::sync::atomic::AtomicU64`，无 prometheus crate 依赖。
- 影响：Prometheus 抓取方需额外写 JSON→Prometheus 适配器；alert 规则无法直接复用 gateway 的指标族。
- 建议：scheduler/executor 也暴露 Prometheus 文本格式端点。

**N8. DAG 执行器并行度无显式上限配置**
- 现象：dag_engine 有并行执行，但未发现 max_parallel_concurrency 配置项。
- 证据：grep `max_parallel|concurrency|parallelism|worker_pool` 在 executor-core/dag_engine.rs 零命中；expert_executor.rs 有 timeout（默认 300s，:52）+ retry（默认 3，:98 读 MOX_EXECUTOR_MAX_RETRIES）+ 指数退避（:78），但并行度靠 tokio 任务自然调度，无信号量限流。
- 影响：超大 DAG（几十节点并行）可能瞬间打爆下游 LLM provider 配额。
- 建议：在 dag_engine 加 Semaphore 限并发。

### 🟢 低严重度 / 观察项

**N9. 调度器动词面数文档口径错误（8 而非 9）**
- 现象：§6.2 写"6 条路径、9 个动词面"，实测 8 个。
- 证据：routes.rs:23-32 逐行计数：health(1)+metrics(1)+leadership(1)+tasks(2)+tasks/:id(2)+experts/search(1)=8。
- 建议：把"9"改为"8"。

**N10. LLM 路由熔断器仅在 scheduler-core，网关 dispatcher 侧熔断器是独立内存 map**
- 现象：scheduler-core/llm_router.rs:55 `circuit_break_until`、:116 threshold=5、:118 duration=60s，真实熔断；但网关 experts_dispatcher.rs:494 的 circuit_breakers 是另一个独立内存 map（get_failure_counts/ensure_failure_map），与 scheduler 侧不共享状态。
- 证据：`scheduler-core/src/llm_router.rs:477` `if state.consecutive_failures >= self.circuit_break_threshold { state.circuit_break_until = Some(...) }`；gateway dispatcher.rs:794/828 `ensure_failure_map()` 独立计数。
- 影响：网关本地预览模式与远程 scheduler 模式的熔断状态不互通。
- 建议：文档说明两套熔断的边界。

**N11. 会话跨进程恢复：单进程内可恢复，多副本不共享**
- 现象：sessions 表落 SQLite（experts_db.rs:108），重启后 load_sessions（experts_common.rs:483）；但网关单副本部署，无多副本会话同步。
- 证据：sessions 存在网关本地 data/experts.db；网关本身未做多活。
- 影响：网关扩容到多副本时会话不共享（sticky session 才行）。
- 建议：已知架构边界，文档应标注。

---

## 四、Crate 真实清单（16 个）

| # | crate 名 | 目录路径 | Cargo.toml | lib.rs 模块 |
|---|---------|---------|-----------|------------|
| 1 | mox-alliance-api | `domains/alliance/api/` | ✅（api/ 根目录即 crate，无同名子目录） | src/dto.rs 等（HTTP DTO） |
| 2 | mox-alliance-core | `core/mox-alliance-core/` | ✅ | dag / fusion / utils |
| 3 | mox-alliance-scheduler-core | `core/mox-alliance-scheduler-core/` | ✅ | matcher / matching / modular_matcher / planner / scheduler / llm_router / executor_bridge / registry / synchronizer / config_sync / storage / leadership / metrics |
| 4 | mox-alliance-executor-core | `core/mox-alliance-executor-core/` | ✅ | condition / dag_engine / expert_executor / fusion / mock_executor / state_sink |
| 5 | mox-alliance-config-core | `core/mox-alliance-config-core/` | ✅ | engine / error / events / store / validator / examples（含 domain_experts.rs 11 专家种子） |
| 6 | mox-alliance-boot-config | `core/mox-alliance-boot-config/` | ✅ | config_store / experts / nacos_config / naming |
| 7 | mox-alliance-registry-core | `core/mox-alliance-registry-core/` | ✅ | aggregation（仅 1 模块，纯计算零 IO） |
| 8 | mox-alliance-common-proto | `proto/mox-alliance-common-proto/` | ✅ | constants / error / events / naming / traits / types |
| 9 | mox-alliance-scheduler-proto | `proto/mox-alliance-scheduler-proto/` | ✅ | matcher / scheduler / types |
| 10 | mox-alliance-executor-proto | `proto/mox-alliance-executor-proto/` | ✅ | dag_engine / node_executor / types |
| 11 | mox-alliance-registry-proto | `proto/mox-alliance-registry-proto/` | ✅ | errors / traits / types |
| 12 | mox-alliance-sdk | `sdk/mox-alliance-sdk/` | ✅ | client |
| 13 | mox-alliance-http-sdk | `sdk/mox-alliance-http-sdk/` | ✅ | alliance / alliance_remote |
| 14 | mox-alliance-scheduler-svc | `svc/mox-alliance-scheduler-svc/` | ✅ | app_state / ha / routes / server（+ bin/main.rs） |
| 15 | mox-alliance-registry-svc | `svc/mox-alliance-registry-svc/` | ✅ | app_state / contract / health_probe / models / routes / server / storage（+ bin/main.rs） |
| 16 | mox-alliance-executor-svc | `svc/mox-alliance-executor-svc/` | ✅ | app_state / routes / server / state_sink（+ bin/main.rs） |

> 补充：`domains/alliance/tools/rnacos/` 不是 crate（无 Cargo.toml），是 Nacos 测试用二进制，不计入 16。

---

## 五、网关路由真实计数（去重路径逐条列出）

### 5.1 `/api/experts/*` 共 43 条（actuator.rs:592-645）

```
/api/experts
/api/experts/:id
/api/experts/:id/consult
/api/experts/:id/consult-now
/api/experts/:id/favorite
/api/experts/:id/metrics
/api/experts/algorithm-analysis
/api/experts/bookings
/api/experts/bookings/:id/cancel
/api/experts/bookings/:id/consult-room
/api/experts/bookings/mine
/api/experts/capabilities
/api/experts/debate
/api/experts/dispatcher/config
/api/experts/dispatcher/consult
/api/experts/dispatcher/dispatch
/api/experts/dispatcher/multi-consult
/api/experts/dispatcher/reset-all
/api/experts/dispatcher/reset/:id
/api/experts/dispatcher/status
/api/experts/enterprise/analyze
/api/experts/enterprise/consult
/api/experts/intelligent-consult
/api/experts/metrics
/api/experts/multi-consult
/api/experts/orchestrate
/api/experts/orchestration/history
/api/experts/orchestration/plugins
/api/experts/orchestration/stats
/api/experts/overview
/api/experts/plan/execute
/api/experts/plan/generate
/api/experts/route
/api/experts/semantic-search
/api/experts/sessions
/api/experts/sessions/:id
/api/experts/sessions/:id/archive
/api/experts/sessions/:id/export
/api/experts/sessions/:id/messages
/api/experts/sessions/:id/similar-search
/api/experts/sessions/stats
/api/experts/stats
/api/experts/team
```

方法级行数 46（含 PUT /dispatcher/config、PUT /bookings/:id/cancel 等）。

### 5.2 `/api/alliance/*` 共 20 条（actuator.rs:518-537）

```
/api/alliance/experts/search
/api/alliance/runtime
/api/alliance/stats
/api/alliance/tasks
/api/alliance/tasks/:id
/api/alliance/tasks/:id/cancel
/api/alliance/tasks/:id/dag
/api/alliance/tasks/:id/execution-status
/api/alliance/tasks/:id/fusion
/api/alliance/tasks/:id/fusion-result
/api/alliance/tasks/:id/logs
/api/alliance/tasks/:id/logs/stream
/api/alliance/tasks/:id/nodes
/api/alliance/tasks/:id/nodes/:node_id
/api/alliance/tasks/:id/pause
/api/alliance/tasks/:id/plan
/api/alliance/tasks/:id/resume
/api/alliance/tasks/:id/retry
/api/alliance/tasks/:id/status
/api/alliance/tasks/:id/toggle-done
```

### 5.3 /ws/v1 零 WebSocket

- `grep WebSocketUpgrade` 全 domains/alliance + gateway/src = **0 hits**。
- 联盟侧实时性仅 SSE：`GET /api/alliance/tasks/:id/logs/stream`（actuator.rs:530）。

---

## 六、端口与环境变量权威清单

### 6.1 监听端口

| 进程 | 地址 | 默认值 | 生效代码位置 |
|------|------|--------|------------|
| platform-gateway-svc | 0.0.0.0:3080 | 3080 | gateway/src/main.rs:27（`.unwrap_or(3080)`）、config.rs:181 |
| alliance-scheduler-svc | 由 yml / env 覆盖 | 3100 | boot-config/src/lib.rs:65,184（`port: 3100`）；bin/main.rs:46 parse host:port |
| alliance-executor-svc | 由 yml / env 覆盖 | 3200 | boot-config/src/lib.rs:247（`port: 3200`）；bin/main.rs:42 |
| alliance-registry-svc | 0.0.0.0:3400 | 3400 | registry-svc/src/app_state.rs:50；MOX_ALLIANCE_REGISTRY_ADDR 覆盖 |

### 6.2 MOX_* 环境变量清单

| 变量 | 默认值 | 作用 | 生效代码位置 |
|------|--------|------|------------|
| `MOX_ALLIANCE_STORAGE_MODE` | `file` | memory/file/sqlite 三态切任务存储 | scheduler-svc/src/server.rs:114；executor-svc/src/state_sink.rs:175 |
| `ALLIANCE_TASK_STORE`（deprecated） | — | 旧存储模式变量，兼容告警 | server.rs:116-119 |
| `MOX_ALLIANCE_REMOTE_MODE` | `auto` | off=强制本地；auto=配了 URL 就走远程 | http-sdk/src/alliance_remote.rs:101 |
| `MOX_ALLIANCE_SCHEDULER_URL` | 未设置 | 远程调度器基址 | alliance_remote.rs:106 |
| `MOX_ALLIANCE_EXECUTOR_URL` | 未设置 | 远程执行器基址 | alliance_remote.rs:107 |
| `MOX_ALLIANCE_HA_MODE` | off | on=开启租约选主 + leader 对账 | scheduler-svc/src/ha.rs:74 |
| `MOX_ALLIANCE_HA_LEASE_MS` | 10000 | 租约时长 | ha.rs:78 |
| `MOX_ALLIANCE_HA_TICK_MS` | lease/3 | 竞选/续约/对账节奏 | ha.rs:81 |
| `MOX_ALLIANCE_HA_STALL_MS` | 300000 | 孤儿判定静默窗口 | ha.rs:83 |
| `MOX_ALLIANCE_HA_DB` | data/alliance_tasks.db | 租约表所在库 | ha.rs:91 |
| `MOX_ALLIANCE_HA_HOLDER` | scheduler-<pid> | 副本标识 | ha.rs:85 |
| `MOX_ALLIANCE_SQLITE_BUSY_MS` | 5000 | SQLite busy_timeout | scheduler-core/src/storage.rs:412 |
| `MOX_API_CRYPTO` | 未设置 | =sm4 开启 gzip+SM4-GCM 全链路加密 | 6 处挂载点见 §2.1(6) |
| `MOX_ALLIANCE_REGISTRY_ADDR` | 0.0.0.0:3400 | registry-svc 监听地址 | registry-svc/src/app_state.rs:75 |
| `MOX_ALLIANCE_REGISTRY_DB` | — | registry SQLite 路径 | app_state.rs:80 |
| `MOX_ALLIANCE_REGISTRY_SNAPSHOT` | — | 实例快照路径；空串=纯内存 | app_state.rs:85 |
| `MOX_ALLIANCE_REGISTRY_REAP_MS` | — | 回收任务间隔 | app_state.rs:88 |
| `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED` | false | 主动健康探测开关 | app_state.rs:95 |
| `MOX_ALLIANCE_REGISTRY_PROBE_INTERVAL_MS` | 30000 | 探测周期 | app_state.rs:99 |
| `MOX_ALLIANCE_REGISTRY_PROBE_TIMEOUT_MS` | 5000 | 单次探测超时 | app_state.rs:106 |
| `MOX_ALLIANCE_CONFIG_FILE` | config/alliance-*.yml | 引导配置文件路径 | scheduler/executor bin/main.rs:30,27 |
| `MOX_ALLIANCE_EXECUTOR_MODE` | — | expert/mock 执行器模式（旧 EXECUTOR_MODE 兼容） | boot-config/src/lib.rs:541-548 |
| `MOX_ALLIANCE_EXPERTS_FILE` | config/alliance-experts.yml | 专家配置 yml 路径 | boot-config/src/experts.rs:16 |
| `MOX_EXECUTOR_NODE_TIMEOUT_MS` | 60000 | 单节点超时 | executor-core/src/expert_executor.rs:93 |
| `MOX_EXECUTOR_MAX_RETRIES` | 3 | 节点最大重试 | expert_executor.rs:98 |
| `MOX_EXECUTOR_INITIAL_RETRY_DELAY_MS` | 1000 | 初始重试延迟 | expert_executor.rs:103 |
| `MOX_EXECUTOR_MAX_RETRY_DELAY_MS` | 30000 | 最大重试延迟 | expert_executor.rs:108 |
| `MOX_EXECUTOR_BACKOFF_FACTOR` | 2.0 | 退避系数 | expert_executor.rs:113 |
| `MOX_AUDIT_LOG_PATH` | data/audit/experts-audit.ndjson | 审计 NDJSON 路径 | experts_common.rs:563 |
| `MOX_AUDIT_HMAC_SECRET` | mox-experts-alliance-audit | 审计哈希链 HMAC 密钥 | experts_common.rs:571 |

---

## 七、核验纪律说明

- 所有"已实现"结论均附 `文件:行号`；行号基于 2026-09-27 工作区快照。
- `WebSocketUpgrade`、`PRAGMA user_version`、图谱节点级 CRUD 等零命中项已在正文显式标注。
- SM4/gzip 算法本体在外部 `mox-api-crypto` crate（不在本仓库 alliance/ 目录内），本仓库仅挂载中间件与调用客户端函数——这一事实已在 §2.1(6) 说明，不编造算法实现位置。
- 未对任何业务代码做修改，仅在 `_verification/` 下产出本报告与辅助脚本。
