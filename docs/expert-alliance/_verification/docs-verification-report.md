# 专家联盟文档体系三方一致性核验报告（docs ↔ code ↔ architecture）

> 核验员：专家联盟文档体系核验员（「专家联盟」文档体系核验）
> 核验日期：2026-09-27
> 核验范围：docs/expert-alliance（01-07 + CURRENT-ARCHITECTURE V1.1 + index.html + _archive 目录结构）、docs/api（PORT-REGISTRY / API-CRYPTO-TRANSPORT / API-SPECIFICATION / TCP-SPECIFICATION）、docs/database（DATABASE-ARCHITECTURE / STORAGE-OPTIMAL-DESIGN / mox-v3.0-baseline.sql）、docs/enterprise（39 号）、docs/normalization、docs/architecture/meta
> 权威基线：`docs/expert-alliance/CURRENT-ARCHITECTURE.md` V1.1（2026-09-24，自标 🟢权威，source_of_truth=代码事实逐条重核）
> 核验性质：**只读**。未修改任何业务文档；本报告为唯一新增产物。
> 行号口径：文档行号 = 该文件物理行号；代码行号 = 本次实读 `experts_db.rs` 的物理行号。未翻代码处一律标 ❓未核 / 未对代码。

---

## 0. 核验结论速览（TL;DR）

1. **01-07 全部为 V1.0（2026-09-25），均未在文首标注「已被 CURRENT-ARCHITECTURE V1.1 取代/部分冲突」**。其中 01/02/03/04/05/06 自标 🟢权威，07 自标 🟡参考。与 V1.1 权威基线之间存在 **3 类硬冲突**：
   - **WebSocket 幻影**：06 §4、02 §1.2 L6、01 F-04、03 §4 都写了 `/ws/v1/*` 进度推送；V1.1 §6.1 补记实测网关全 crate `WebSocketUpgrade` 零命中。
   - **plans/tasks 表幻影**：05 §1/§9 称 Task/Node/CollaborationPlan 落 SQLite `experts_db.rs`；本次实读 `experts_db.rs:91-158` 只有 7 张表（experts/sessions/session_messages/graph_nodes/graph_edges/graph_meta/bookings），**无 plans/tasks/nodes 表**。
   - **专家状态枚举三套不一致**：01/03/04 用 `active/inactive`，05 用 `active/busy/offline/inactive`，V1.1 §1.2 实测网关 `availability.status` 取值为 `online/busy/offline/away`。
2. **端口**：3080/3100/3200 与 PORT-REGISTRY 完全一致；但 **CURRENT §2.1 端口表漏登 3400/3300/3210**，而 PORT-REGISTRY §3.2 三行均在册。
3. **数据库**：网关 SQLite（experts_db.rs）表名与 baseline.sql 的 `ea_*` 前缀表**完全不对应**；baseline.sql 是 MySQL 8.3 目标模板（非 PG），且 DATABASE-ARCHITECTURE as-built 登记的联盟 PG schema 是「5 表 via migrations/001_init.sql」，与 baseline.sql 的 11 张 ea_* 表又是两套。
4. **enterprise 39 号**：已带「历史快照说明（2026-09-13）」，引用已删除的 backend-node / mox-expert crate 与已迁移的 :8080，定性为**历史诊断文档**，非现行架构。
5. **最大体系缺口**：01-07 与 CURRENT-ARCHITECTURE V1.1 之间**缺一张导航/状态图**，读者无法判断哪份是现状、哪份是目标态；MOX_* 环境变量散落在 5+ 份文档里无单一权威页。

---

## 1. 01-07 文档一致性总表

口径：「章节数」= 该文件 `##` 二级标题数（含子节不计入）；行号为物理行号。

| 文档 | 二级章节数 | 与 V1.1 冲突点（文档路径:行号） | 过期未标注章节 | 增量新内容（V1.1 未覆盖） |
|---|---|---|---|---|
| **01-prd.md**（V1.0 🟢权威，173 行） | 5 | ① F-04 进度推送写 `WebSocket/SSE`（01-prd.md:75）vs V1.1 §6.1 `/ws/v1/*=0`；② F-02「对应代码 scheduler-core/matcher.rs (RuleBasedExpertMatcher)」（01-prd.md:58）vs V1.1 §3.1 `modular_matcher` 才是生产主路径、matcher 是 fallback；③ F-02 过滤「状态=Active」（01-prd.md:56）vs V1.1 §1.2 实测 `online/busy/offline/away` | F-09 协作记忆/案例库三层记忆（01-prd.md:114-120）描述「工作→会话→案例库」持久化，但 V1.1 §5 明确 favorites/plans/orchestration_history 均进程内重启即失，无案例库落表 | §3 角色权限五角色（ADMIN/OPERATOR/EXPERT_OWNER/AUDITOR/GUEST，01-prd.md:132-140）；§4 关键业务参数阈值表（01-prd.md:144-157） |
| **02-architecture.md**（V1.0 🟢权威，214 行） | 6 | ① L6 接入层写 `REST/WebSocket/SSE`（02-architecture.md:29）vs V1.1 §6.1 无 WS；② §2.2 把「网关→HTTP→scheduler→executor」画成默认调用链（02-architecture.md:103-117）vs V1.1 §4.1 默认形态是全进程内（`MOX_ALLIANCE_REMOTE_MODE=off`），跨进程仅在 `=auto`；③ §3.2 把 RuleBasedExpertMatcher 列为主匹配器（02-architecture.md:144）vs V1.1 §3.1 modular_matcher 主路径 | §1.2 L5 标注「11 个 .rs, 8680 行」（02-architecture.md:31）已过期——V1.1 §1.2 实测 11 文件合计 9924 行；同文件 §1.3 逐文件行数求和恰为 9924，**文内自相矛盾**；§1.2 L3「config-core 10 大专家」（:46）vs V1.1 §3.6 补记 vec! 实为 11 条 | §4 架构归一化（分层依赖规则 02-architecture.md:154-161 + Crate 依赖矩阵 :163-186）；§5 技术选型版本表（:190-203） |
| **03-business-flow.md**（V1.0 🟢权威，261 行） | 7 | ① §1 六步主流程按跨服务链路画（03-business-flow.md:11-44）vs V1.1 §4.1 默认进程内；② §2「候选过滤：状态仅 Active」（:56）vs V1.1 §4.2 健康是加权项非过滤；③ §4「WebSocket/SSE 推送节点状态」（:170）vs V1.1 无 WS；④ §7 注册成功「状态=Active」、心跳超时→Offline（:251,258）vs V1.1 §1.2 网关侧无探活、availability.status 是登记值 | §7 专家注册流程（03-business-flow.md:236-261）描述「注册中心验证→心跳上报→分级聚合→超时离线」的完整探活闭环，V1.1 §1.2 明确网关侧 health 探活为零、健康只在契约层/scheduler 侧 | §6 异常处理流程（重试 3 次/指数退避 1s→2s→4s/关键节点定义，:208-232）；§3.3 Dynamic 决策流程图（:109-133） |
| **04-state-machine.md**（V1.0 🟢权威，167 行） | 8 | ① §1.1 任务状态 `pending→planning→executing→fusing`（04-state-machine.md:16-29）vs 05 §2 TaskStatus 枚举为 `pending/planning/running/paused/completed/failed/cancelled`（无 executing/fusing，多 paused）；② §1.2 节点状态缺 ready/cancelled（:33-45）vs 05 §3 NodeStatus 含 ready/cancelled；③ §2 专家状态机 `inactive/active/busy/offline`（:51-62）vs V1.1 §1.2 `online/busy/offline/away`；④ §4.2「状态过滤 status=Active」（:103）vs V1.1 §4.2 健康加权非过滤 | （状态枚举本身即过期，未标注） | §3 审批状态机 pending/approved/rejected（04-state-machine.md:68-82）——V1.1 全文未提审批流，❓未核代码是否存在；§8 数据一致性规则（:160-167） |
| **05-data-model.md**（V1.0 🟢权威，212 行） | 9 | ① §1 称 CollaborationPlan 落 SQLite `plans` 表（05-data-model.md:18）vs V1.1 §5「experts_db.rs 无 plans 表，plans 进程内重启即失」；② §9 称 Task/Node/Plan 持久化于 experts_db.rs（:206）vs 实读 experts_db.rs:91-158 **无 tasks/nodes/plans 表**（任务在 scheduler-core/storage 与 executor state_sink）；③ §9 称 Expert 注册表「内存 HashMap，scheduler 侧 Phase1」（:207）vs V1.1 §5 专家注册表在网关 SQLite 表 experts；④ §5 ExpertStatus `active/busy/offline/inactive`（:141-146）vs V1.1 §1.2 `online/busy/offline/away`；⑤ §9 称审计日志 SQLite experts_db.rs（:210）vs 实读该文件无 audit_logs 表 | §9 整节存储架构（05-data-model.md:202-212）与 V1.1 §5 存储表三处对不上 | §8 PlanDynamicRoute 字段表（decision_node/field/operator/value/true_branch/false_branch，:189-198）；§7 FusionStrategy 列 9 值（:173-185）——注意与 01/03/04 自称的「6 种融合策略」互相矛盾 |
| **06-api-spec.md**（V1.0 🟢权威，193 行） | 5 | ① §2 标题「50 个」全部列在 `/api/experts/*`+`/api/expert-graph/*`+`/api/ai/*`（06-api-spec.md:25-110），**完全未列 `/api/alliance/*` 一族**——V1.1 §6.1 实测 `/api/experts/*`=43 + `/api/alliance/*`=20（去重路径）；V1.1 §4.1 明确任务创建真端点是 `POST /api/alliance/tasks`（actuator.rs:519），06 文档无此端点；② §4 列 `/ws/v1/experts/tasks/:task_id/progress`（:180）vs V1.1 §6.1 补记 `/ws/v1/*=0`、WebSocketUpgrade 零命中；③ §1 路由前缀写 `/api/experts/* + /api/expert-graph/*`（:21）vs V1.1 第二前缀是 `/api/alliance/*` | （整份 API 清单基于 V1.0 路由表，未随 2026-09 边界归一化更新） | §3 请求/响应示例（生成计划 :116-148、多轮辩论 :150-172）；§5 错误码表（:184-193） |
| **07-deployment.md**（V1.0 🟡参考，133 行） | 6 | ① §6 配置参考只列 gateway/scheduler/executor/registry.port + db.path + nacos.addr（07-deployment.md:126-133），**漏列 V1.1 反复强调的 4 个 MOX_* 运行开关**（CRYPTO/REMOTE_MODE/STORAGE_MODE/HA_MODE）；② §5.1 称 `/metrics` 在「gateway 侧」（:109）vs V1.1 §3.1 `/metrics` 挂在 scheduler-svc:3100 | §4 演进路线 Phase1 仍写「单机演示 SQLite+内嵌Nacos+进程内调用」（07-deployment.md:96），未反映 V1.1 §8 已落地项：传输加密 P0 完成、registry-svc 独立完成、HA 部分落地 | §2 本地启动五步骤（:35-53）；§3 docker-compose.yml 片段（:57-88） |

### 1.1 06-api-spec 端点数与 V1.1 §6.1 的对账

- V1.1 §6.1（CURRENT-ARCHITECTURE.md:227-235）：`/api/experts/*` 43 + `/api/alliance/*` 20 = **63 条去重路径**（按 actuator.rs ROUTES 去重、忽略动词）；前端台账的 74 是 (path, method) 行数，两者口径不同不可互换。
- 06-api-spec.md §2 自列 50 条（2.1=10 + 2.2=9 + 2.3=6 + 2.4=6 + 2.5=8 + 2.6=6 + 2.7=5 = 50），全部落在 `/api/experts/*` / `/api/expert-graph/*` / `/api/ai/*`。
- **缺口**：06 未收录任何 `/api/alliance/*` 端点（V1.1 称有 20 条），包括 V1.1 §4.1 点名的 `POST /api/alliance/tasks`、`GET /api/alliance/tasks/:id/logs/stream`（SSE）等。
- **幻觉**：06 §4 的 `/ws/v1/...` 在 V1.1 实测中不存在。

---

## 2. 端口权威核对表

权威来源：`docs/api/PORT-REGISTRY.md` V1.2（2026-09-14，自标 🟢权威，全仓库端口唯一权威）。

| 端口 | PORT-REGISTRY 记载（位置） | CURRENT-ARCHITECTURE §2 记载 | 一致性 |
|---|---|---|---|
| 3080 | RUNTIME `api`（Rust 网关 mox-platform-gateway-svc），🟢运行中，/health 探活（PORT-REGISTRY.md:31,50） | §2.1 platform-gateway-svc:3080（CURRENT:77） | ✅ 一致 |
| 3100 | ALLIANCE scheduler-svc，🟢已启用（PORT-REGISTRY.md:37,64） | §2.1 scheduler-svc:3100（CURRENT:78） | ✅ 一致 |
| 3200 | ALLIANCE executor-svc，🟢已启用（PORT-REGISTRY.md:38,65） | §2.1 executor-svc:3200（CURRENT:79） | ✅ 一致 |
| 3400 | ALLIANCE registry-svc，🟢运行中（PORT-REGISTRY.md:67）；env `MOX_ALLIANCE_REGISTRY_*` 覆盖 | §2.1 进程表**未列 3400**；仅 §1.1 crate 拓扑列出 mox-alliance-registry-svc:3400（CURRENT:41） | ⚠️ CURRENT §2.1 运行时进程表漏登 3400（02-architecture.md:101 与 07-deployment.md:20 已列） |
| 3300 | ALLIANCE AI 专家服务桥接基址（scheduler→3300，`expert_service.base_url`），🟢已启用（PORT-REGISTRY.md:39,66） | V1.1 全文未提 3300 | ❌ CURRENT 端口体系未覆盖 3300 桥接端口 |
| 3210 | ALLIANCE codeengine-svc（自研 AI 代码引擎），`MOX_CODEENGINE_PORT` 覆盖，🟢已启用（PORT-REGISTRY.md:40,68） | §7.1 提到 expert-code-engine 模块（CURRENT:277），但 §2 端口表未列 3210 | ⚠️ 模块在、端口未进 §2 表 |
| 50051 | ANCILLARY gRPC（`mox-dualrpc`/framework默认/**专家联盟内部 gRPC**）（PORT-REGISTRY.md:79） | §2.2 注释「gRPC :50051 在 framework 层保留但专家联盟未使用」（CURRENT:96） | ⚠️ 措辞差异：PORT-REGISTRY 写「专家联盟内部 gRPC」，CURRENT 写「专家联盟未使用」，需统一 |
| 33080/33100/33200 | 联盟本地开发端口（start-alliance-local.ps1 默认），🟡本地开发（PORT-REGISTRY.md:69-71） | 未提 | ℹ️ 本地开发段，不影响生产一致性 |
| 33020 | 专家联盟前端 dev（npm run dev --port 33020）（PORT-REGISTRY.md:91） | 未提 | ℹ️ 前端 dev 端口，CURRENT 未覆盖前端 |

### 2.1 MOX_* 环境变量权威定义位置

| 环境变量 | 取值/含义 | 权威定义位置 |
|---|---|---|
| `MOX_API_CRYPTO` | off/未设置=关闭（默认）；`sm4`=data gzip+SM4-GCM 全链路加密 | `docs/api/API-CRYPTO-TRANSPORT.md:29`（开关表）；密钥 `MOX_API_CRYPTO_KEY` 见 :31；CURRENT §8:310 引用 |
| `MOX_ALLIANCE_REMOTE_MODE` | `off`/未配置=全进程内默认；`auto`+远程 URL=跨服务链路 | **仅 CURRENT-ARCHITECTURE.md:172,183** 定义，无独立专页；代码引用 alliance_remote.rs:35/:101 |
| `MOX_ALLIANCE_STORAGE_MODE` | `memory`/`file`(默认快照)/`sqlite`(完整持久化+重启恢复) | `docs/standards/expert-alliance-port-norm.md:171,231`（env↔yml 映射）；CURRENT §5:219 生产建议；另散见 docs/API-REGISTRY.md:104、docs/architecture/MODULARITY.md:52 |
| `MOX_ALLIANCE_HA_MODE` | `on`=开启租约选主+fencing（默认 off=单副本） | `docs/architecture/microservices/07-massive-scale-100k-nodes.md:86,124`（运行面 ha.rs）；CURRENT §3.1:128、§6.2:243、§8:311 |
| `MOX_ALLIANCE_REGISTRY_*` | registry-svc 配置覆盖 | `docs/api/PORT-REGISTRY.md:67` |
| `MOX_ALLIANCE_SERVER_PORT` | 覆盖默认端口（如 3100） | `docs/api/PORT-REGISTRY.md:73`；`docs/standards/expert-alliance-port-norm.md:156` |

> 注：`docs/standards/expert-alliance-port-norm.md`（PORT-NORM-001）是 3000-3999 段的细粒度权威，本核验未深读其正文，仅从引用处取证；该文件存在性已确认。

---

## 3. 数据库表权威核对表

### 3.1 实读结果

**A. 网关嵌入式 SQLite（`platform/gateway/mox-platform-gateway-svc/src/alliance/experts_db.rs`，721 行）**
本次实读 `CREATE TABLE` 语句，共 7 张表：

| 表名 | 建表语句行号 |
|---|---|
| `experts` | experts_db.rs:91 |
| `sessions` | experts_db.rs:108 |
| `session_messages` | experts_db.rs:124 |
| `graph_nodes` | experts_db.rs:136 |
| `graph_edges` | experts_db.rs:143 |
| `graph_meta` | experts_db.rs:152 |
| `bookings` | experts_db.rs:158 |

**无** `plans` / `tasks` / `nodes` / `favorites` / `orchestration_history` / `audit_logs` 表。这与 CURRENT §5（CURRENT:211-213）一致：favorites/plans/orchestration_history 均进程内 HashMap/Vec，重启即失。

**B. PostgreSQL/MySQL 基线 `docs/database/mox-v3.0-baseline.sql`（245 行）**
专家联盟相关表全部以 `ea_` 前缀命名，共 11 张：

| 表名 | 建表语句行号 | 业务注释 |
|---|---|---|
| `ea_alliance` | baseline.sql:222 | 专家联盟（collaboration_mode/fusion_strategy） |
| `ea_expert` | baseline.sql:223 | 专家注册 |
| `ea_capability` | baseline.sql:224 | 专家能力 |
| `ea_expert_capability` | baseline.sql:225 | 专家-能力关系 |
| `ea_domain` | baseline.sql:226 | 专家领域 |
| `ea_expert_domain` | baseline.sql:227 | 专家-领域关系 |
| `ea_case` | baseline.sql:228 | 专家协作案例 |
| `ea_task` | baseline.sql:229 | 联盟协作任务 |
| `ea_task_node` | baseline.sql:230 | 任务节点（node_type 含 CONDITION/FUSION/HUMAN） |
| `ea_task_edge` | baseline.sql:231 | 任务 DAG 边（含 condition_expr） |
| `ea_task_result` | baseline.sql:232 | 任务结果 |

### 3.2 三方对账

| 表/对象 | baseline.sql（PG/MySQL 目标） | 网关 experts_db.rs（SQLite 现状） | CURRENT §5 存储表 | 字段/命名差异 |
|---|---|---|---|---|
| 专家注册 | `ea_expert`（:223，列 expert_code/expert_name/provider_code/profile/quality_score） | `experts`（:91） | 网关 SQLite 表 experts（CURRENT:206） | **表名不同**：ea_expert vs experts；字段未对代码细核 ❓未核 |
| 会话 | （baseline 无 ea_session） | `sessions`（:108）+ `session_messages`（:124） | 网关表 sessions/session_messages（CURRENT:209） | baseline.sql **未覆盖会话表** |
| 知识图谱 | （baseline 用 kg_entity/kg_relation:235-236，非联盟专用） | `graph_nodes`/`graph_edges`/`graph_meta`（:136/143/152） | 网关表 graph_nodes/graph_edges/graph_meta（CURRENT:210） | baseline 的 KG 表是通用图谱，与联盟 graph_* 不对应 |
| 预约 booking | （baseline 无） | `bookings`（:158） | CURRENT §1.2 提及 experts_ext.rs 预约（:57） | baseline 未覆盖 |
| 协作任务 | `ea_task`（:229）+ `ea_task_node`（:230）+ `ea_task_edge`（:231） | **无**（任务在 scheduler-core/storage 与 executor state_sink，落 data/alliance_tasks.db/.json） | scheduler-core/storage SQLite+JSON；executor state_sink WAL SQLite（CURRENT:207-208） | 任务表不在网关 experts_db.rs，而 05-data-model.md:206 误称在 experts_db.rs |
| 协作计划 | （baseline 无独立 plan 表，并入 ea_task_node） | **无 plans 表**（进程内 HashMap） | 进程内，experts_common.rs:468（CURRENT:212） | 05-data-model.md:18 误称 SQLite plans 表 |
| 案例库 | `ea_case`（:228） | 无 | V1.1 §5 未提案例库落表 | baseline 有 ea_case，现状未实现 |
| 融合结果 | `ea_task_result`（:232） | 无（执行器侧 `__fusion_output__` 保留行） | executor state_sink，__fusion_output__ 行（CURRENT:208） | 命名/存储位置均不同 |

### 3.3 关键结论

1. **baseline.sql 是 MySQL 8.3 目标模板，不是 PostgreSQL 现状**：全文使用 `ENGINE=InnoDB`、`BINARY(16)`、`DATETIME(3)`、`CHARACTER SET utf8mb4`——MySQL 方言。`docs/database/DATABASE-ARCHITECTURE.md:7-9` 自述 mox_sys/ 与本模板是「MySQL 8.3 目标模型（target）」，as-built 是 SQLite。任务书称其为「PostgreSQL 基线」实为 MySQL 目标模板。
2. **DATABASE-ARCHITECTURE as-built 与 CURRENT §5 也有口径差**：DATABASE-ARCHITECTURE.md:36 把联盟记为「默认内存；PG 可选；data/alliance_tasks.json；PG migrations/001_init.sql 共 5 表」，**未单列网关 experts_db.rs 的 experts/sessions/graph_* 7 张表**（它把 172KB 的 data/experts.db 归到 :58「expert-svc 另一历史路径」）。即 DB as-built 总表与 CURRENT §5 对「网关联盟 SQLite」的登记粒度不一致。
3. **baseline.sql 的 ea_task_node.node_type 含 CONDITION/HUMAN**（:230），但 V1.1 §3.7 补记明确「代码里不存在条件分支执行」——baseline 目标态领先于现状。

---

## 4. 文档地图（docs/expert-alliance + 关联文件）

```
docs/expert-alliance/
├── CURRENT-ARCHITECTURE.md        [权威现状] V1.1 2026-09-24 🟢 唯一权威（本核验基线）
├── 01-prd.md                     [PRD] V1.0 2026-09-25 🟢  ← 未标注被 V1.1 取代
├── 02-architecture.md            [总体架构] V1.0 🟢       ← 同上
├── 03-business-flow.md           [业务流程] V1.0 🟢       ← 同上
├── 04-state-machine.md           [状态机/业务规则] V1.0 🟢 ← 同上
├── 05-data-model.md               [数据模型/字典] V1.0 🟢   ← 同上（含 plans 表幻觉）
├── 06-api-spec.md                 [API 规范] V1.0 🟢       ← 同上（含 WS 幻觉、漏 /api/alliance/*）
├── 07-deployment.md               [部署运维] V1.0 🟡参考    ← 漏 MOX_* env 表
├── index.html                     [可视化导航页] （未深读正文）
├── _verification/                 [本报告新增目录]
│   └── docs-verification-report.md
├── _archive/
│   ├── v1/  (00-INTEGRATED-INDEX / 01-ENTERPRISE-OPTIMIZATION / 02-DUAL-PLATFORM-RELATIONSHIP /
│   │         03-GLOSSARY / architecture-flow-source / code-engine-alliance-mode / DAG-VISUALIZATION-DESIGN /
│   │         DEPLOYMENT-DESIGN / EA-DIAG-001.html / EA-SCEN-001 / enterprise-scenario-alliance-matrix /
│   │         expert-registry-and-protocol / FRONTEND-MODULE / knowledge-graph-schema / README)
│   ├── v2/  (00-requirements / 01-architecture / 02-domain-model / 03-business-flow / 04-api-design /
│   │         05-data-architecture / 06-security-observability / 07-roadmap / README)
│   ├── v3/  (01-architecture-optimization / 02-requirements-matrix / 03-business-flow-diagrams / README)
│   ├── old-architecture-html/ (deployment-guide.html / ops-manual.html / system-architecture-design.html)
├── _shots/  (architecture-diagram / business-flow 桌面+移动截图)

docs/api/（关联权威）
├── PORT-REGISTRY.md               [端口唯一权威] V1.2 2026-09-14 🟢
├── API-CRYPTO-TRANSPORT.md        [MOX_API_CRYPTO 传输加密权威]
├── API-SPECIFICATION.md           （未逐行深读）
├── TCP-SPECIFICATION.md           （未逐行深读）
└── README.md / mox-module-manifest.schema.json

docs/database/（关联权威）
├── DATABASE-ARCHITECTURE.md       [as-built 实际持久化权威] 2026-09-17
├── STORAGE-OPTIMAL-DESIGN.md      [target 存储演进设计] 2026-09-17
├── mox-v3.0-baseline.sql          [MySQL 8.3 目标 DDL，含 ea_* 11 表]
└── mox_sys/（内核 module-registry.yml 等，通用平台非联盟专用）

docs/architecture/meta/（视角参考，非现状）
├── 02-EXPERT-ALLIANCE-ARCHITECTURE.md   V2.0 🟡参考 2026-08-31（宇宙架构视角，7 服务目标态）
├── 04-EXPERT-ALLIANCE-v3-MODULAR.md     V3.0 🟡参考 2026-08-31（God Module→7 微服务目标态）
└── 00-COSMIC-META-ARCHITECTURE.md / 01-DATABASE-DDL.sql / 03-DATABASE-DESIGN-SPEC.md

docs/enterprise/
└── 39-开发专家联盟-架构诊断与SaaS化最优方案-V1.1.md   [历史诊断，见 §7]

docs/normalization/
└── ARC-INDEX.md / API-INDEX.md / BP-INDEX.md / TPL-INDEX.md / VAL-INDEX.md（通用归一化索引，非联盟专用）

docs/standards/（被引用但未在核验清单内）
└── expert-alliance-port-norm.md   [PORT-NORM-001，3000-3999 段细粒度权威]
```

---

## 5. 文档体系缺口清单

| # | 缺口 | 现状证据 | 建议 |
|---|---|---|---|
| G1 | **01-07 与 CURRENT-ARCHITECTURE 的导航/状态图缺失** | 01-07 均无「本文为 V1.0 目标态，现状以 CURRENT-ARCHITECTURE V1.1 为准」标注；读者无法区分现状/目标 | 在每份 01-07 文首加权威声明，并新增一张导航图（见 §6 建议的 08 文档） |
| G2 | **环境变量手册缺失** | MOX_API_CRYPTO/REMOTE_MODE/STORAGE_MODE/HA_MODE 散落在 API-CRYPTO-TRANSPORT、port-norm、07-massive-scale、CURRENT 五处；07-deployment §6 完全未列 | 新增 env 参考页或并入 08 |
| G3 | **前端模块说明缺失** | PORT-REGISTRY:91 登记前端 dev 端口 33020；联盟前端台账称 74 个 (path,method) ready；但 docs/expert-alliance 下无前端文档（v1/FRONTEND-MODULE.md 已归档） | 新增「前端模块与路由台账」文档 |
| G4 | **部署运维手册过薄** | 07-deployment 仅 133 行、🟡参考；无故障排查、无日志位置、无数据恢复步骤、无 MOX_ALLIANCE_STORAGE_MODE=sqlite 生产开关 | 扩容 07 或新增 ops-runbook |
| G5 | **API 变更日志缺失** | 2026-09 边界归一化删了 /tasks/:id/nodes、/tasks/:id/result（CURRENT:250），06-api-spec 未记录此次变更 | 新增 CHANGELOG 或在 06 加变更记录 |
| G6 | **故障排查指南缺失** | V1.1 提示了大量坑（重启任务丢失/空 history 不代表无记录/WS 不存在），但无排障文档 | 新增 troubleshooting 页 |
| G7 | **DB schema 三方对照缺失** | 网关 SQLite 7 表 / baseline.sql ea_* 11 表 / DATABASE-ARCHITECTURE as-built 5 表 PG 三套未对齐 | 在 08 或独立 data-mapping 文档对齐 |
| G8 | **05-data-model 与 01/03/04 内部枚举不一致** | FusionStrategy 6 vs 9；TaskStatus executing/fusing vs running/paused；ExpertStatus active/inactive vs online/away | 以代码/common-proto 为准统一枚举字典 |

---

## 6. 新文档命名与大纲建议（企业级模块化归一化架构文档）

### 6.1 编号建议

现有编号已用到 **07-deployment.md**。建议新文档编号为 **`08`**，命名：

- 文件名：**`08-normalized-architecture.md`**（与 01-07 数字序一致、kebab-case）
- doc_id：`EA-ARCH-NORM-001`
- title：《专家联盟企业级模块化归一化架构（导航+现状+目标合一）》
- authority：🟢权威（与 CURRENT-ARCHITECTURE 并列，互为索引；建议在文首声明「现状事实以 CURRENT-ARCHITECTURE V1.1 为最终裁决，本文负责归一化导航与目标态路线」）

> 不建议另起 `09` 或字母前缀（如 `NA-01`）：01-07 已是用户熟悉的数字序，插入 08 可被 index.html 自然收录；若未来要拆「现状/目标」双轨，再把 CURRENT 重命名为 `00-current.md` 即可。

### 6.2 章节大纲建议（10 章）

1. **文档导航与权威链**——一张图说清 01-07 / CURRENT / _archive v1-v3 / enterprise 39 / architecture/meta 02-04 各自定位，标注「现状事实→CURRENT」「目标态→本文 §9」「历史→_archive」
2. **模块清单与物理代码分布**——对齐 CURRENT §1（16 crates + 网关 11 模块 9924 行），含行数漂移警示
3. **服务拓扑与端口单表**——合并 CURRENT §2 与 PORT-REGISTRY §3.2：3080/3100/3200/3300/3400/3210，标注 LOCAL 33080/33100/33200
4. **运行开关（MOX_* 环境变量）单一参考**——CRYPTO/REMOTE_MODE/STORAGE_MODE/HA_MODE/REGISTRY_*/SERVER_PORT 六变量，含默认值、取值、生产建议、权威出处链接
5. **存储与持久化对账**——三栏表：网关 SQLite 7 表（experts_db.rs:91-158）/ scheduler+executor 任务库（alliance_tasks.db）/ target ea_* 11 表（baseline.sql:222-232），标注「重启即失」清单（favorites/plans/orchestration_history）
6. **API 清单与路由前缀**——按 V1.1 §6.1 口径：/api/experts/* 43 + /api/alliance/* 20，明确「无 /ws/v1/*、实时性仅 SSE logs/stream」
7. **状态机与枚举字典（统一版）**——以 common-proto/types.rs 为源，统一 TaskStatus/NodeStatus/ExpertStatus/AllianceMode/FusionStrategy 五套枚举，消除 01/03/04/05 间的 6vs9、activevs online 矛盾
8. **匹配评分与 Dynamic 决策规则**——对齐 V1.1 §4.2（健康加权 0.15 非过滤）与 planner.rs decide_dynamic_mode
9. **目标态演进路线（归一化）**——对齐 V1.1 §8：已完成项（registry 独立、传输加密 P0、HA 部分落地）标 ✅，P1 项（PostgreSQL+Redis+pgvector、跨机 LeaseStore、ShardFanout），P2/P3 项（fusion/memory 独立、gRPC、JSON-RPC+MCP）
10. **故障排查与常见误读**——收录 V1.1 已踩坑：空 history≠无记录、重启任务丢失根因=REMOTE_MODE、/metrics 在 scheduler 不在 gateway、状态字段≠探活

---

## 7. enterprise 39 号文档定位结论

- 文件：`docs/enterprise/39-开发专家联盟-架构诊断与SaaS化最优方案-V1.1.md`（doc_id EA-DOC-061，🟢权威，最后更新 2026-08-31）
- **定性：带退役快照说明的「历史诊断 + SaaS 化方案」文档，非现行实现架构。**
- 证据：
  1. 文首 line 9 自带「⚠️ 历史快照说明（2026-09-13）」：自述为「诊断时点的事实记录」，并明确 `platform/backend-node/`（Node.js 平台层）与 `platform/domains/mox-expert/` crate 已在 Rust 统一后**退役删除**。
  2. 正文引用 `platform/backend-node/src/ai-engine-core.js`（:42-49）、`platform/domains/mox-expert/src/rbac/policy.rs`（:20-28）——这些路径在当前仓库已不存在。
  3. 引用网关端口 :8080（:9），而 PORT-REGISTRY §6.4（2026-09-14）已将 Rust 网关从 8080 迁至 **3080**。
  4. 其结论「SaaS MVP 9-10 周、工作量砍 40%」属规划判断，非代码现状。
- **与 CURRENT-ARCHITECTURE.md 的关系**：**被取代关系**，非引用/冲突/补充。CURRENT-ARCHITECTURE V1.1（2026-09-24）是 Rust 统一后的现状权威；39 号是 Rust 统一前（Node 时代）的诊断快照。二者时间相差近 1 个月、技术栈不同（Node vs Rust），不应并列引用。建议在 39 号文首已有快照说明的基础上，加一行「现行架构见 `docs/expert-alliance/CURRENT-ARCHITECTURE.md` V1.1」。

---

## 8. 未核实项与边界声明

- ❓未核：04-state-machine §3 审批状态机（pending/approved/rejected）在代码中是否有实现——本次未翻 experts_*.rs 全文，仅据 CURRENT 未提及推断为缺口。
- ❓未核：API-SPECIFICATION.md / TCP-SPECIFICATION.md 正文（任务清单要求读，但本次未逐行比对端点；从 PORT-REGISTRY 与 06-api-spec 的对账已能定位主要冲突）。
- ❓未核：index.html 正文内容（仅确认存在，未解析其内链）。
- ❓未核：experts_db.rs 7 张表的具体字段与索引（本次仅取 CREATE TABLE 行号与表名；字段级对账需后续逐列读 :91-165）。
- ❓未核：docs/standards/expert-alliance-port-norm.md 正文（仅从 env grep 取证其存在与 env↔yml 映射行号）。
- 本报告所有「代码事实」仅来自本次实读的 `experts_db.rs`（721 行）行号；其余代码引用（如 matcher.rs:166、planner.rs:81-92、alliance.rs:565）均转引自 CURRENT-ARCHITECTURE.md V1.1 自身的行号标注，未二次翻代码复核。
- 未对任何业务文档做修改。



---

## 9. P0-D WS 文档幻影修订（2026-09-29 第三轮）

12-innovation-roadmap.md §1.3 表第 3 行后半（#25 / 10-D3）。本轮对 01/02/03/06 四个 V1.0 目标态文档
**只插入补记、不改原文**（保持历史原貌 + 显式标注被取代），与 08/INDEX/CURRENT 已有的幻影标注闭环。

### 修订动作（每份文档：标题下横幅 + 章节末尾补记）

| 文档 | 横幅（标题下） | ⚠️ 补记插入位置 | 原文 /ws/v1 表述 |
|---|---|---|---|
| 01-prd.md | :11 | F-04 DAG 执行引擎表格后（:81） | :75 `进度推送 WebSocket/SSE 实时推送节点级进度` |
| 02-architecture.md | :12 | §1.2 六层架构代码块后（:64） | :29 L6 接入层 `REST / WebSocket / SSE` |
| 02-architecture.md | （同上横幅） | §2.2 进程间调用链代码块后（:123） | :111 `↓ REST/WebSocket` |
| 03-business-flow.md | :11 | §4 节点执行代码块后（:178） | :170 `[5]进度推送 WebSocket/SSE 推送节点状态` |
| 06-api-spec.md | :12 | §4 WebSocket 接口表格后（:184） | :182 `/ws/v1/experts/tasks/:task_id/progress` |

### 补记统一文案

> ⚠️ **V1.1 核对补记（2026-09-29）**：本文此处所述的 `/ws/v1/*` WebSocket 推送在实现中不存在
> （全 crate `WebSocketUpgrade` 零命中）；实时性由 SSE `GET /api/alliance/tasks/:id/logs/stream` 承担。
> 以 CURRENT-ARCHITECTURE.md V1.1 为准。

横幅统一文案：
> 状态：V1.0 目标态，部分结论已被 CURRENT-ARCHITECTURE.md V1.1 取代（见文中 ⚠️ 补记）。

### 插入方式与校验

- Python 脚本 `p0d_patch_docs.py`（UTF-8 读写，保留原换行），按锚点字符串定位插入，幂等。
- 首跑后人工读回：01:81 / 02:64,123 / 03:178 / 06:184 共 **5 处补记 + 4 处横幅**，无乱码、无重复插入。
- 02 文档因含两处 WS 表述，首跑脚本幂等逻辑（全文含标记即跳过）误吞第二处，已用 Edit 在 §2.2 代码块后手工补插（:123），并复核。
- 原文一字未改（01:75、02:29,111、03:170、06:182 的 WebSocket 字样原样保留，仅在其所在表格/代码块后追加补记）。

*报告完。*
