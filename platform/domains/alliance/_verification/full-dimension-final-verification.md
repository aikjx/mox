# 专家联盟 · 全维终验报告（后端部分）

- **执行时间**：2026-10-03（周六）
- **执行方**：专家联盟后端全维终验（验证 → 全量测试 → 分析 → 优化修复 → 记录）
- **工程根**：`D:\a10\aikjx\gitcode\infotopograph`
- **基线参照**：08 号架构 §十三 全维度功能总表（`docs/expert-alliance/08-normalized-architecture.md` 尾部）+ `backend-fix-report.md` 各轮节（N4/T2/T3/A1/A2/D4/D8/T4/SSE+webhook/配额）
- **本轮实跑为准**：所有测试数字为本轮 2026-10-03 实跑值，非照抄基线
- **不触碰他人并发未提交**：`experts_ext.rs`、`alliance/mod.rs`、`tests/tenant_expert_isolation.rs`、`favorite_repository.rs`、`favorite_transactions.rs`（开工前 `git status` 已确认归属，全程未改）

---

## 一、全维验证清单（后端部分）

每项给出「功能 | 验证方式（代码位置 / 测试名 / 运行证据）| 结果」。
结果口径：✅ 有代码 + 有测试/运行证据；🟡 部分落地/留增强；🔴 缺陷。

### 1. 专家注册画像

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| 注册专家（POST /api/experts） | 代码 `experts_registry.rs create_expert`；单测 `test_create_expert`；E2E `a1_quota_tenant.rs tenant_expert_quota_enforced_per_tenant`（实跑 200） | ✅ |
| 列表专家（GET，分页/领域过滤） | 代码 `list_experts`；单测 `test_list_experts_pagination`、`test_list_by_domain`、`test_tenant_isolation_list_experts_cross_tenant_invisible` | ✅ |
| 更新专家（PUT，merge 语义） | 单测 `test_update_expert_merge` | ✅ |
| 停用专家（软删除 enabled=false） | 单测 `test_soft_delete_expert` | ✅ |
| 收藏专家（按租户，落盘） | 代码 `experts_common.rs favorites HashMap` + `experts_db.rs upsert_favorite/load_all_favorites`；E2E `favorite_transactions.rs`（他人并发，实跑 6 passed 0 failed） | ✅ |

### 2. 专家图谱

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| 图谱查询（neighbors/BFS/stats/community） | 单测 `test_neighbors`、`test_bfs_shortest_path`、`test_bfs_shortest_path_unreachable`、`test_compute_graph_stats`、`test_detect_communities`、`test_graph_structure` | ✅ |
| rebuild（增量建图 + 版本号递增） | 单测 `experts_db::incremental_tests::bump_meta_upsert`、`upsert_node_insert_then_update`、`edge_upsert_then_renumber_keeps_seq_order`、`delete_node_cascades_edges`；`test_rebuild_version_increment` | ✅ |
| N4 CRUD（node/edge upsert/delete，RBAC 保护） | 单测 `test_crud_requires_auth_401`、`test_mutate_graph_rbac_rejects` | ✅ |
| rag-expand（环保护/无自环） | 单测 `test_rag_cycle_guard_no_self_loop`、`experts_graph.rs rag_*` 单测 | ✅ |
| optimal-team（最小代价团队） | 单测 `test_find_optimal_team` | ✅ |

### 3. 编排

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| plans 生成（POST /plan/generate，不执行） | 代码 `experts_orchestration.rs generate_plan_handler`；单测 `test_orchestrate_flow`；E2E `sse_event_stream_e2e`（实跑 200） | ✅ |
| orchestration_history（执行历史，落盘） | 代码 `insert_history_record`/`load_all_history`；单测 `test_get_history_filter_by_tenant`；E2E `d4_crash_recovery` | ✅ |
| execute（POST /plan/execute，Kahn DAG 拓扑） | 代码 `execute_plan_handler` + `topological_sort`；单测 `test_execute_plan`、`test_topological_sort`、`test_topological_sort_cycle`、`test_dag_execution_order` | ✅ |
| orchestrate 一键编排（生成+执行） | 代码 `orchestrate`；单测 `test_orchestrate_flow` | ✅ |

### 4. 会话

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| 会话生命周期（create/get/list/close） | 代码 `experts_session.rs`；lib 单测覆盖（189→190 全绿） | ✅ |
| 会话带租户隔离 | `TenantId` extractor 下推；`tenant_expert_isolation.rs`（他人并发，实跑 1 passed） | ✅ |

### 5. RBAC

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| RbacAction 8 动作枚举 | 代码 `experts_rbac.rs`（GraphRead/Write/EdgeMutate/NodeMutate/DatasetMutate/ExpertRegister/Execute/Admin）；单测 `test_rbacaction_count` | ✅ |
| enforce 决策（角色→动作→放行/403） | 单测 `test_enforce_admin_allows_all`、`test_enforce_viewer_denied`、`test_enforce_operator_only_data_ops`、`test_enforce_unknown_role_default_deny` | ✅ |
| 审计（enforce 失败落审计日志） | 单测 `test_rbac_audit_log_records_decision`、`test_rbac_audit_records_per_tenant`、`test_rbac_enforce_idempotent_under_concurrency` | ✅ |
| 处理端真挂 enforce（非仅模块自测） | E2E `test_mutate_graph_rbac_rejects`（viewer 写边真实 403）、`test_crud_requires_auth_401` | ✅ |

### 6. 多租户

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| 数据隔离（注册表/图谱/计划按租户下推） | 单测 `test_tenant_isolation_list_experts_cross_tenant_invisible`；E2E `a2_multi_instance_consistency`、`tenant_expert_isolation` | ✅ |
| 配额（专家数 per tenant） | 代码 `experts_common.rs check_expert_quota`；E2E `a1_quota_tenant.rs`（实跑 tenant-a 第3个真实409） | ✅ |
| 配额（**计划数 per tenant，本轮新增**） | 代码 `check_plan_quota` + `count_plans_by_tenant`；纯函数单测 `test_check_plan_quota_with_threshold`；E2E `a3_plan_quota_tenant.rs`（实跑第2个真实409） | ✅ |
| 审计带租户（emit_audit 带 tenant_id） | 代码 `experts_common.rs emit_audit`（`audit: {"tenant": tenant,...}`）；单测 `test_audit_cross_tenant_isolation`、`test_audit_append_is_best_effort` | ✅ |

### 7. 落盘（D4 三表 + 恢复）

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| plans/history/favorites 三表写穿 | 代码 `upsert_plan`/`insert_history_record`/`upsert_favorite`；schema v1-v4 | ✅ |
| 崩溃恢复（重启后状态不丢） | E2E `d4_crash_recovery.rs`（真实 SQLite，实跑 1 passed）；`experts_db_persistence.rs`（8 passed） | ✅ |
| 写后即落 + best-effort 不阻断 | `retry_write` 忙等待重试；单测 `test_retry_write_retries_on_locked_then_succeeds` | ✅ |

### 8. 无状态化（A2 阶段一）

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| 冷数据读路径实时查 SQLite（plans/history/favorites） | 代码 `load_plans_by_tenant`/`get_plan`/`load_favorites_by_tenant`/`load_history_by_tenant`；E2E `a2_multi_instance_consistency.rs`（实跑 1 passed） | ✅ |
| 跨实例即一致（B 不重启读 A 写穿） | 同上 E2E 用两份 state 交叉验证 | ✅ |
| registry/graph 高频态外移 | 仍进程内（高频态，阶段二方案稿见 §五） | 🟡 |

### 9. 事件（T4 总线）

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| 进程内事件总线（订阅/发布/派发） | 代码 `experts_events.rs EventBus`；单测 `test_bus_subscribe_then_publish_delivers`、`test_bus_event_types_filter` | ✅ |
| event_log 落表（消费者写盘，带租户） | 代码 `spawn_event_log_consumer` + `insert_event_log`/`load_event_log_by_tenant`；E2E `t4_event_bus_e2e.rs`（实跑 2 passed） | ✅ |
| 计划生命周期事件链（created→status_changed） | E2E `t4_event_bus_e2e`（PlanCreated/PlanStatusChanged 真实派发） | ✅ |

### 10. SSE 事件帧 / Webhook

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| SSE 事件流（text/event-stream，真实流式） | E2E `sse_event_stream_e2e.rs`（起真实 axum + reqwest stream，实跑 4 passed）；`alliance_stream_contract.rs`（1 passed） | ✅ |
| Webhook 注册/列表/删除（租户隔离） | 代码 `EventBus register/list/delete_webhook`；E2E `sse_event_stream_e2e`（POST /webhooks 实跑 200） | ✅ |
| Webhook 真实 HTTP 派发（异步派发器 + 重试） | 代码 `spawn_webhook_dispatcher`（真实 reqwest POST，无 mock）；E2E `sse_event_stream_e2e` 起 loopback receiver 收真实帧 | ✅ |
| **Webhook 订阅持久化（本轮新增，v5）** | 代码 `alliance_webhooks` 表 + 写穿 + 启动读回；E2E `webhook_persistence.rs`（登记→模拟崩溃重启→逐字段恢复+跨租户隔离+删除持久化+幂等，实跑 1 passed） | ✅（本轮从 🟡 升 ✅） |

### 11. D8 探活

| 功能 | 验证方式（证据） | 结果 |
|---|---|---|
| registry-svc 主动健康探测默认开启 | 代码 `app_state.rs:63 health_probe_enabled: true`、`server.rs:39` 仅在 true 时 spawn；单测 `health_probe.rs:227 assert!(cfg.health_probe_enabled)` | ✅ |
| env 可关 / 30s 周期 | `app_state.rs:107` env 解析；registry-svc 实跑 28 passed | ✅ |

### 12. 内部鉴权（MOX_INTERNAL_TOKEN）

| 功能 | 验证方式（证据） |
|---|---|
| 网关→registry-svc 出站注入 Bearer | 代码 `registry_client.rs:27-36`（配置后自动带 `Authorization: Bearer`） | ✅ |
| scheduler-svc internal_auth_layer 校验 | 代码 `scheduler-svc/routes.rs:49`（MOX_INTERNAL_TOKEN/ALT 双值、公共路径白名单、dev 旁路） | ✅ |
| N6 降级（不 panic） | `registry_client.rs:37-41` build 失败降级默认 client | ✅ |

### 13. MCP 三工具

| 功能 | 验证方式（证据） |
|---|---|
| expert_search / optimal_team / graph_expand 三工具 | 代码 `mcp/mox-alliance-mcp-server/src/main.rs`（3 个 tool 分支） | ✅ |
| stdio JSON-RPC 2.0 + Content-Length 帧 | 代码 `read_message`/`write_message`；单测 5 个（encode/decode/frame 往返） | ✅ |

### 14. 执行器（信号量 / dag_engine）

| 功能 | 验证方式（证据） |
|---|---|
| DAG 最大并行度信号量 | 代码 `executor-core/dag_engine.rs Semaphore`/`acquire_owned`/`resolve_dag_max_parallel`（ENV_DAG_MAX_PARALLEL）；executor-core 实跑 50 passed（含 e2e/bench） | ✅ |

**清单小结**：后端部分 50+ 子项，✅ 覆盖功能闭环；唯一历史 🟡「webhook 重启即失」本轮已补 ✅；遗留 🟡 仅「registry/graph 高频态外移（阶段二）」与「SSO/SAML 真实 handler」（方案稿，见 §五）。**无 🔴 缺陷项。**

---

## 二、全量测试真实数字（2026-10-03 实跑）

| crate | 套件 | passed | failed | 说明 |
|---|---|---:|---:|---|
| mox-platform-gateway-svc | lib | **190** | 0 | 基线 189 + 本轮新增 `test_check_plan_quota_with_threshold` 1 |
| mox-platform-gateway-svc | 集成（tests/） | **73** | 0 | 基线 68 + 本轮新增 `webhook_persistence` 1 + `a3_plan_quota_tenant` 1（注：`favorite_transactions` 他人并发从 3→6，未触碰） |
| scheduler-core | lib | 115 | 0 | 与基线一致 |
| executor-core | lib | 43 | 0 | |
| executor-core | bench | 1 | 0 | |
| executor-core | e2e | 6 | 0 | |
| registry-core | lib | 16 | 0 | |
| scheduler-svc | lib+集成 | 23 | 0 | lib12 + 集成11 |
| executor-svc | lib+集成 | 15 | 0 | lib9 + 集成6 |
| registry-svc | lib+集成 | 28 | 0 | lib17 + 集成11 |
| mox-alliance-mcp-server | lib | 5 | 0 | |
| mox-alliance-http-sdk | lib | 15 | 0 | |
| **合计** | | **530** | **0** | gateway lib+集成 263 + 三 svc 66 + core 181(scheduler115+executor50+registry16) + mcp5 + sdk15 |

- **cargo check**：`cargo check -p mox-platform-gateway-svc` 退出 0，**无 error**；17 个 warning 全部为既有（`experts_session.rs` 未用 tenant extractor、`experts_graph.rs`/`experts_dispatcher.rs` 测试 drop_on_ref），本轮新代码零新增 warning。
- 三 svc 合计 66（scheduler23+executor15+registry28）与早前基线一致。

### 失败逐条分析

本轮全量实跑中，gateway lib 首次出现 **1 个失败**：

- `alliance::experts_dispatcher::tests::test_reset_expert`（`experts_dispatcher.rs:1101`，`previous_failures` 期望 3 实得 0）。
  - **定位**：该测试依赖进程级全局 `static FAILURE_COUNTS: parking_lot::Mutex<Option<HashMap>>`（`experts_dispatcher.rs:43`）。同模块 `reset_all` handler（:868）会清空整张表；并行测试调度下，`test_reset_all` 可能在本用例 `insert(exp-ai-001,3)` 与 `reset_expert` 断言之间清掉全局表。
  - **判定**：**既有测试隔离 flaky（环境/并行调度），非本轮改动引入，非真业务缺陷**。
  - **验证**：单独跑 `cargo test --lib test_reset_expert` → **1 passed**；全量重跑 → **190 passed 0 failed**（EXIT=0），复跑两次均绿。
  - **处置**：不弱化断言、不改业务逻辑。留作 §五 建议项（把 `FAILURE_COUNTS` 改为按测试唯一 key 或加 `#[serial]` 串行化），避免误改他人/既有测试语义。

---

## 三、优化推进项（本轮真实落地）

### (a) Webhook 订阅表落盘 —— 最高价值低成本，已完成

**背景**：T4 轮诚实标注 webhook 注册为进程内内存 HashMap、重启即失。本轮把订阅持久化。

**改动（schema v4 → v5）**：

1. `experts_db.rs`
   - `SCHEMA_VERSION` 4→5；`init_schema` 新增表 `alliance_webhooks(id PK, tenant_id, url, event_types JSON, created_at)` + `idx_webhooks_tenant`。
   - 新增 `WebhookRow`、`upsert_webhook_conn`/`upsert_webhook`（ON CONFLICT upsert）、`delete_webhook_row_conn`/`delete_webhook_row`（按 id+租户，防跨租户删）、`load_all_webhooks`（启动读回）。复用既有 `retry_write`/best-effort 约定。
2. `experts_events.rs`
   - `register_webhook`：入内存热投影后**写穿 SQLite**（event_types 序列化为 JSON 列）；`delete_webhook`：命中即删内存 + 写穿删除。
   - 新增 `restore_webhooks_from_db()`：启动从 SQLite 读回重建内存注册表（event_types 解析失败降级空数组=全收，不丢订阅）。
3. `experts_common.rs` `new()`：总线创建后立即 `events.restore_webhooks_from_db()`。

**测试**：新增 `tests/webhook_persistence.rs`——真实临时 SQLite + 真实 `ExpertsSharedState::new()`，走「登记 → drop state 模拟崩溃 → 重启读回 → 逐字段一致 → 跨租户不可见 → 派发匹配语义恢复 → 删除持久化 → 二次重启幂等」闭环。**实跑 1 passed**。

**效果**：webhook 订阅重启后自动恢复，从 08 §十三 🟡 升 ✅。

### (b) 配额补 DAG（计划）维度 —— 已完成

**背景**：A1 阶段二时 plans 还是全局扁平内存 HashMap、按租户计数需全表扫描，故如实未做。A2 后 plans 立为 SQLite 唯一真相（`collaboration_plans(tenant_id, plan_id)` + `idx_plans_tenant`），计数变 O(index) 真实廉价——本轮补齐。

**改动**：

1. `experts_db.rs`：新增 `count_plans_by_tenant(tenant) -> i64`（`SELECT COUNT(*) ... WHERE tenant_id=?1`，DB 不可用按 0 放行，不阻断写路径）。
2. `experts_common.rs`：新增 `DEFAULT_QUOTA_PLANS_PER_TENANT=1000`、`quota_plans_per_tenant()`（env `MOX_ALLIANCE_QUOTA_PLANS_PER_TENANT`）、`check_plan_quota_with()`（纯函数，超限返回 409 + 结构化 `resource=plan/quota/used`）、`check_plan_quota()`；纯函数单测 `test_check_plan_quota_with_threshold`。
3. `experts_orchestration.rs`：在 `orchestrate` 与 `generate_plan_handler` 入口（task 非空校验后、生成计划前）挂 `count_plans_by_tenant` + `check_plan_quota`，超限真实 409、不落库。
   - **语义说明**：`execute_plan_handler` 执行既有计划（不新增行），故不挂配额——挂了反而错。

**测试**：新增 `tests/a3_plan_quota_tenant.rs`——真实路由器 + JWT，env `MOX_ALLIANCE_QUOTA_PLANS_PER_TENANT=1`，tenant-a 第1次 200/第2次真实 409（body 含 `resource=plan/quota=1/used=1`），tenant-b 配额独立同验证。**实跑 1 passed**。

### (c) 其他低成本优化

经分析，本轮 (a)(b) 已是两项明确高价值项，均带真实 E2E 闭环；**不再追加第三项**，避免过度工程与回归面。唯一顺手清理：新测试 `a3_plan_quota_tenant.rs` 闭包多余 `mut`（消除新 warning）。

---

## 四、修复清单

| 项 | 类型 | 处置 |
|---|---|---|
| gateway lib `test_reset_expert` 首跑失败 | 既有测试隔离 flaky（全局 `FAILURE_COUNTS` 并行被 `reset_all` 清空），非真缺陷、非本轮引入 | 单独跑/全量复跑均绿；不弱化断言。留 §五 建议 |
| webhook 重启即失 | 历史 🟡（真功能缺口） | (a) 落盘修复，新增 E2E 闭环 |
| 配额缺 DAG 维度 | 历史 🟡（真功能缺口） | (b) 接入 generate/orchestrate，新增 E2E 闭环 |

**无真业务缺陷修复**（本轮实跑未发现真缺陷）；**无锚点漂移**（未改任何既有断言，仅新增测试与功能代码）。

---

## 五、留待项及原因（不硬做，产方案稿）

1. **registry/graph 高频态外移（A2 阶段二）**：当前 registry/graph 仍进程内（高频读，亚毫秒要求）。外移需引入外部存储（Redis/共享 SQLite WAL 多连接）+ 失效广播，体量大、跨进程一致性边界多，本轮不动。方案：registry 节点数据迁 Redis（hash per tenant），graph 边邻接表迁 Redis，网关读面改 Redis 优先 + SQLite 兜底；跨实例一致性靠 Redis pub/sub 失效通知。
2. **跨进程事件广播（SSE 多副本 fan-out）**：当前事件总线进程内，多副本下 A 实例发的事件 B 实例的 SSE 连接收不到。方案：接入 Redis pub/sub（或现有的 mox 消息总线），各实例订阅后转发本地 SSE/webhook。
3. **SSO SAML/CAS/LDAP 真实 handler**：现有 `experts_sso.rs` 仅 stub/501。需对接真实 IdP，缺测试 IdP 环境，产方案稿不硬做。
4. **dispatcher 全局 `FAILURE_COUNTS` 测试隔离**：建议改为按测试唯一专家 key 或加串行化，消除偶发 flaky；本轮不改既有测试语义，如实标注。

---

## 六、改动文件清单（本轮，可回退）

| 文件 | 改动 |
|---|---|
| `experts_db.rs` | schema v5 + `alliance_webhooks` 表 + webhook CRUD + `count_plans_by_tenant` |
| `experts_events.rs` | webhook 写穿 + `restore_webhooks_from_db` + 注释 |
| `experts_common.rs` | `new()` 调 restore；计划配额守卫（env/纯函数/单测） |
| `experts_orchestration.rs` | orchestrate + generate_plan_handler 挂计划配额 |
| `tests/webhook_persistence.rs` | 新增（webhook 落盘 E2E） |
| `tests/a3_plan_quota_tenant.rs` | 新增（计划配额 E2E） |

**未触碰（他人并发）**：`experts_ext.rs`、`alliance/mod.rs`、`tests/tenant_expert_isolation.rs`、`favorite_repository.rs`、`favorite_transactions.rs`。

---

## 七、前端部分 · 全维终验（2026-10-03）

> 本节为单一权威整合：前端终验节合并自 `frontend-ui/src/modules/expert-alliance/_verification/frontend-fix-report.md` 末尾「全维终验（2026-10-03）」节，数字均读自该报告实跑值，不照抄、不臆造。
> 结构：7.1 验证清单（前端逐项证据）→ 7.2 vitest 全量数字 → 7.3 本轮优化推进 → 7.4 修复闭环 → 7.5 留待；前后端合并视图见 §八。

### 7.1 前端全维验证清单（逐项证据，全部 ✅）

| 功能面 | 验证方式（证据） | 结果 |
|---|---|---|
| U1 画布·拖拽移动节点 | GraphCanvas.vue:136-164 pointer 事件 emit drag；store setNodePosition→dragPositions；证据 graph-canvas.test.js / alliance-graph.store.test.js | ✅ |
| U1 画布·连线 | GraphCanvas.vue:12 连线起点高亮 + :358 canvasClickNode 态机 + createGraphEdge:262 | ✅ |
| U1 画布·邻域展开 | GraphNodeInspector.vue:32「展开邻域」→expandSelectedNeighborhood:429（seed/ maxDepth=2），mergeRagResults:393 幂等并入 | ✅ |
| U1 画布·节点增删 | Inspector:36-37→updateGraphNode/deleteGraphNode；视图新增节点对话框 createGraphNode；store CRUD:229-271 | ✅ |
| U2 透明面板·权重 0.05 口径 | MatchExplainPanel.vue:18「健康度按 0.05 加权」；五维 :42-46；证据本轮新增 match-explain-panel.test.js | ✅ |
| U2 透明面板·维度条 | MatchExplainPanel.vue:8-14 每维 mxe-row；挂载点 AllianceExpertsView.vue:129；v-if="scores" 缺省不渲染 | ✅ |
| SSE·composable | useAllianceEventStream.js:7-21（Bearer/auth watch/onScopeDispose）；contract/event-stream.js:33 AbortController 契约 | ✅ |
| SSE·store | alliance-orch.store.js:40 liveEvents / :225 applyAllianceEvent / :243-250 带 plan_id 防抖 800ms 真拉 / 30 条上限 | ✅ |
| SSE·视图挂载（编排台） | AllianceOrchestrationView.vue:284 import / :314 use / :322 onMounted start / :325 onUnmounted stop | ✅ |
| SSE·防泄漏 | onUnmounted stop + onScopeDispose + event-stream.js finally reader.cancel/releaseLock + watch auth→stop | ✅ |
| 按钮权限·v-role-any | 活代码 10 处（Inspector/Console/Graph/Experts 视图），统一 ['super_admin','tenant_admin']，与路由 requiresRole、后端 ADMIN_ROLES 三端同源 | ✅ |
| 契约·单向依赖 | contract 生产文件仅内部互引；api/alliance.api.js 仅 import contract+http 工厂+kernel envelope | ✅ |
| 契约·双向守卫 | contract.test.js 101（端点清单↔API-REGISTRY 双向、禁用端点不复活）；graph.test.js 32（GRAPH_NODE_TYPE↔Rust builder 双向钉死） | ✅ |
| 契约·样式/词汇/棘轮门禁 | style.test.js 21；registry-name-outlets.test.js 15；vocabulary-ownership.test.js 19 | ✅ |
| 视图·编排台/图谱/专家/控制台 | 四视图活代码接线（SSE 实时面板/重建/编辑模式/CRUD/调度单专家与全量重置） | ✅ |
| store/api 方法面 | 6 store 各有 *.store.test.js；alliance.api.test.js 58；endpoints.js 端点 key 由 contract.test 101 项对齐 API-REGISTRY | ✅ |

> 清单小结：前端全维逐项证据全部 ✅。6 个终端 view（编排台/图谱/专家/控制台/协作/会话）仍无挂载测试——重度依赖 Element Plus + 多 store + auth，硬挂脆弱；本轮以纯呈现组件 MatchExplainPanel 真实 mount 测试补空（见 7.3b），完整 view 挂载如实列入 7.5 留待。

### 7.2 vitest 全量真实数字（2026-10-03 实跑）

- 命令：`npx vitest run src/modules/expert-alliance`
- **终验结果：Test Files 29 passed (29) / Tests 675 passed (675)，0 失败，exit 0。**
- 对比基线 28 文件 / 667 用例：+1 测试文件（match-explain-panel.test.js）+8 用例（experts store 5 + mount 3）= 675。
- 失败分析：**0 失败**，无需逐条归因。
- 门禁复跑：style(21)、registry-name-outlets(15)、vocabulary-ownership(19)、contract(101) 随全量复跑仍绿，未新增任何失败。

### 7.3 本轮优化推进（前端，真实落地 + 测试闭环）

- **(a) 专家注册表补挂 SSE（消费 ExpertRegistered/ExpertDisabled）**：store/alliance-experts.store.js 新增 `applyRegistryEvent(kind, envelope)` / `clearRegistryEventTimer()`（按信封形状判定：带 `expert_id`＝注册表身份变更，不硬编码事件名字面量；800ms 防抖合并突发帧后真拉 `loadExperts()`+`loadStats()`，仅当 capabilities 已加载才补拉目录）；AllianceExpertsView.vue 建 eventStream（onEvent→store.applyRegistryEvent，onError 静默），onMounted start / onUnmounted stop。路由 `/alliance/experts` 已限管理员，挂流不面向全体用户。测试：alliance-experts.store.test.js 新增 5 用例（命中真拉/突发两帧合并/不带 expert_id 的 Plan 帧忽略/目录已加载才补拉/clear 挂起未触发重拉）。
- **(b) MatchExplainPanel 真实 mount 测试（U2 组件此前零测试）**：新建 components/match-explain-panel.test.js（@vue/test-utils 真 mount，+3 用例）：scores=null 整个 `.mxe-root` 不渲染（降级不破坏卡片）；带 scores→5 个 `.mxe-row`+总分演算行+健康度 0.05 口径文案+五维标签；`setProps({matchScore:0.923})` 驱动 DOM 跟随重渲染。
- **(c) 刻意只做 a/b 两项**：未给注册表页新增可见事件流面板（避免新模板+CSS+词表账的过度设计），保持最小可回退。

### 7.4 前端修复闭环

- 本轮无真缺陷需修复：进场基线 28/667 全绿，a/b 改动后 29/675 仍全绿，未引入新缺陷。
- 另用临时 SFC 编译探针（@vitejs/plugin-vue 真编译两个改动视图，跑完即删）确认 AllianceExpertsView.vue / AllianceOrchestrationView.vue 编译无错，补齐「6 view 无挂载测试、视图 script 改动不被测试覆盖」的缺口。

### 7.5 前端留待项

- 控制台 / 图谱视图未挂流（Expert* 帧本轮已被注册表消费；控制台任务列表挂流价值中等留待下轮，图谱视图重建/邻域手动触发、挂流收益低）。
- 完整 view 的 store→DOM mount 测试仍缺（留待引入全站测试基座后再补）。
- 画布 DAG 导出/预演、minimap、虚拟滚动、G9 三栈归一（方案稿，本轮不硬做）。

---

## 八、全维终验总表（前后端合并视图，2026-10-03）

> 本表把 §一–§六（后端）与 §七（前端）合并为单一权威视图。所有数字为本轮 2026-10-03 实跑，与两路报告逐字一致。

### 8.1 全量测试总表

| 侧 | 套件 | passed | failed | 说明 |
|---|---|---:|---:|---|
| 后端 | gateway-svc lib | **190** | 0 | 基线 189 + test_check_plan_quota_with_threshold 1 |
| 后端 | gateway-svc 集成（tests/） | **73** | 0 | 基线 68 + webhook_persistence 1 + a3_plan_quota_tenant 1（favorite_transactions 他人并发 3→6，未触碰） |
| 后端 | scheduler-core lib | 115 | 0 | 与基线一致 |
| 后端 | executor-core | 50 | 0 | lib43 + bench1 + e2e6 |
| 后端 | registry-core lib | 16 | 0 | |
| 后端 | 三 svc（scheduler/executor/registry） | 66 | 0 | lib+集成合计 23+15+28 |
| 后端 | mox-alliance-mcp-server | 5 | 0 | stdio JSON-RPC 帧往返 |
| 后端 | mox-alliance-http-sdk | 15 | 0 | |
| **后端小计** | | **530** | **0** | cargo check 无 error（17 warning 全既有，本轮新代码零新增 warning） |
| 前端 | expert-alliance vitest | **675** | **0** | 29 文件全绿；门禁 style21/name-outlets15/vocab19/contract101 随全量复跑仍绿 |
| **合计** | | **1205** | **0** | 后端 530 + 前端 675 |

### 8.2 本轮优化推进合并清单（均真实落地 + E2E/测试闭环）

| # | 侧 | 项 | 落地证据 |
|---|---|---|---|
| 1 | 后端 | webhook 订阅表落盘（schema v4→v5 + alliance_webhooks 表，写穿 + 启动读回） | tests/webhook_persistence.rs（登记→模拟崩溃重启→逐字段恢复+跨租户隔离+删除持久化+二次重启幂等，1 passed） |
| 2 | 后端 | 配额补 DAG（计划）维度（count_plans_by_tenant + check_plan_quota 挂 orchestrate/generate） | tests/a3_plan_quota_tenant.rs（env 配额=1，tenant-a 第2个计划真实 409，tenant-b 配额独立，1 passed） |
| 3 | 前端 | 专家注册表补挂 SSE（applyRegistryEvent + AllianceExpertsView 挂流） | alliance-experts.store.test.js 新增 5 用例 |
| 4 | 前端 | MatchExplainPanel 真实 mount 测试 | match-explain-panel.test.js +3 用例 |

### 8.3 修复闭环（历史 🟡 → ✅）

| 历史缺口 | 原口径 | 本轮处置 |
|---|---|---|
| webhook 重启即失（T4） | 进程内内存 HashMap，重启即失（08 §十三 行内注记） | 落盘修复（§三a），新增 E2E 闭环，升 ✅ |
| 配额缺 DAG（计划）维度（A1） | plans 全局扁平内存，按租户计数需全表扫描，如实未做 | 接入 generate/orchestrate（§三b），新增 E2E 闭环，升 ✅ |
| gateway lib test_reset_expert 首跑失败 | 既有 FAILURE_COUNTS 全局态并行被 reset_all 清空 | 单独跑/全量复跑均绿（190 passed 0 failed），非本轮引入、非真缺陷；不弱化断言，留 §五 建议 |

### 8.4 留待合并清单（方案稿，本轮不硬做）

- **A2 阶段二**：registry/graph 高频态外移、执行器 task 状态、跨进程事件广播（SSE 多副本 fan-out）、多副本写冲突策略。
- **SSO**：SAML/CAS/LDAP 真实 handler（待真实 IdP；OAuth2/OIDC 授权码交换已实现但缺 IdP 端到端验证）。
- **MCP Client**：Server 已闭环，Client 端与凭证托管留待（M2）。
- **前端挂流与测试**：控制台 / 图谱视图 SSE 挂流；完整 view 的 store→DOM mount 测试。
- **前端画布**：DAG 导出/预演、minimap、虚拟滚动、G9 三栈归一。
- **测试隔离**：dispatcher 全局 FAILURE_COUNTS 串行化/按测试唯一 key（消除偶发 flaky）。

### 8.5 终验结论

- **后端**：14 大类 50+ 子项逐项核到代码 `文件:行号` 与测试证据，**无 🔴 缺陷项**；唯一历史 🟡「webhook 重启即失」本轮补 ✅；后端全量 **530 passed / 0 failed**，cargo check 无 error。
- **前端**：全维验证清单逐项证据**全部 ✅**；全量 **29 文件 / 675 用例，100% 全绿 0 失败**，门禁全绿。
- **合并口径**：本轮终验合计 **1205 passed / 0 failed**（后端 530 + 前端 675）；两项历史 🟡（webhook 落盘、计划配额）均升 ✅ 且带真实 E2E 闭环；唯一测试失败 test_reset_expert 已定位为既有并行 flaky（非本轮引入、非真缺陷）。
- **权威指向**：本报告为全维终验单一权威；前端终验原始证据见 frontend-fix-report.md 末尾节；架构总表口径见 08 §十三；状态账见 16 号总账 §五/§十一。
