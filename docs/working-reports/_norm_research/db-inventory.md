# 持久化现状核查清单（db-inventory）

> 核查口径：以**仓库代码为准**，只读打开各域真实 DDL 与建表调用点。
> 核查日期：2026-09-17。磁盘实况取自 `data/` 目录实际文件。
> 未改动任何代码。行号均为 `platform/domains/` 下相对路径内的行号。

---

## 0. 总表：域 → 存储引擎 → 物理库/文件 → 建表入口 → 表数量

| 域 | 存储引擎 | 物理库 / 文件 | 建表入口（文件:行） | 表数量 |
|---|---|---|---|---|
| IAM/系统管理 | SQLite (WAL) | **data/mox.db**（与 meta/biz_data 同库共享连接） | `mox-platform-enterprise-svc/src/app_state.rs:54` → `mox-platform-iam-core/src/repo.rs:51` | **22** |
| meta 元数据 | SQLite (WAL) | data/mox.db（同上共享连接） | `app_state.rs:56` → `mox-platform-meta-core/src/repo.rs:141` | **16** |
| datastore/biz_data | SQLite (WAL) | data/mox.db（同上共享连接） | `app_state.rs:60` → `mox-platform-datastore-core/src/dao.rs:54` | **3**（biz_data / biz_data_version / biz_data_relation）+ 动态 `kv_store` |
| DSQL | SQLite（运行）+ Postgres（参考 schema） | 自身 SQLite 文件（`DsqlStorage::open(path)`，测试用 meta.db/exec.db） | `mox-dsql-core/src/storage.rs:36`（include 001/002/003） | **8** |
| KG（core） | SQLite | GraphStorage::open(path)，无固定文件名 | `mox-kg-core/src/storage.rs:37` | **2**（kg_vertex / kg_edge） |
| KG（storage-svc） | SQLite + 文件快照 | `with_persistence(db_path)`；磁盘另有 `data/mox-kg-graph.json` | `mox-kg-storage-svc/src/lib.rs:342`（init_db） | **2**（kg_nodes / kg_edges，整表快照读写） |
| AI/专家 (expert-svc) | SQLite (WAL) | **data/mox-expert-svc.db**（默认）；磁盘另有 `data/experts.db` | `mox-ai-expert-svc/src/persistence.rs:31`（open） | **4**（experts / sessions / plans / kv） |
| AI/对话 (agent-svc) | SQLite | `operator_dialogue.db`（cwd，`lib.rs:115`） | `conversation.rs:50` / `dialogue_graph.rs:53` / `requirement_compiler.rs:234` | **5**（sessions / messages / dialogue_sessions / dialogue_messages / blueprints） |
| 联盟 (alliance-engine) | **默认内存**；Postgres 为可选生产实现 | 无 SQLite 落库；磁盘 `data/alliance_tasks.json` 为文件快照 | 默认 `InMemoryTaskRepository`（`persistence.rs:231`）；PG 见 `migrations/001_init.sql` | **5（PG schema）**：alliance_tasks/events/experts/phase_stats/schema_migrations |
| primiflow（运行时） | SQLite (WAL) | **data/store.db** | `mox-flow-primiflow-svc/src/persistence.rs:140`（SCHEMA_SQL @ :116） | **4**（kb_assets / kb_graph / trace / projects） |
| primiflow（gen 演示 schema） | Postgres 风格（仅生成产物） | 无落库，模板字符串 | `src/gen/ddl.sql`（pgvector 注释，UUID/TIMESTAMPTZ） | 6 表（projects/conversations/topologys/assets/artifacts/trace_links） |
| market（模板市场） | 无自身库；生成 DDL 字符串 | — | `mox-market-template-svc/src/template_market/seed.rs:91-153` | 6 表（product/cart/orders/payment/member/point_log，PG 风格模板） |
| cloud | 对象存储（master/volume/s3/filer/rebalance） | 文件/卷对象，无业务关系表 | filer 仅内存 SQLite inodes（`meta_sqlite.rs:9` open_in_memory） | 0 持久化业务表 |
| kb | 进程内存 HashMap | 无文件/库 | `mox-kb-server/src/main.rs:24` InMemoryKbStore | 0 表 |

---

## 1. IAM / 系统管理

- **DDL 文件**：`mox-platform-iam-core/src/ddl.sql`（注释自称 SQLite，列对齐 Node 版 schema.js）。
- **全部表名（22，编号见 DDL 注释 1–22）**：
  `iam_tenant`、`iam_department`、`iam_user`、`iam_user_dept`（3b）、`iam_role`、`iam_permission`、`iam_user_role`、`iam_role_permission`、`iam_role_inherit`、`iam_menu`、`iam_user_menu`、`iam_role_menu`、`iam_data_permission`、`iam_resource`、`iam_tenant_setting`、`audit_log`（链式哈希真源）、`sys_post`、`sys_dict_type`、`sys_dict_data`、`sys_config`、`sys_oper_log`、`sys_logininfor`、`sys_api_key`。
  （预期 24，实际 **22**。）
- **物理文件**：不是独立库。`app_state.rs:29-46` 用一个 `rusqlite::Connection::open(path)` 打开 `data/mox.db`，同一个 `Arc<Mutex<Connection>>` 同时喂给 `IamRepository` / `MetaRepository` / `UniversalBizDAO`。
- **建表调用点**：
  - `repo.rs:17` `DDL_SQL = include_str!("ddl.sql")`；
  - `repo.rs:51` `init_schema()`：按 `;` 切分逐条 `execute_batch`（repo.rs:59），对 "already exists/duplicate column" 容错；
  - 末尾 `repo.rs:69-72` 幂等 `ALTER TABLE iam_data_permission ADD COLUMN field_permissions_json`。
- **WAL/迁移框架**：WAL 由 `app_state.rs:37` `pragma_update journal_mode=WAL` 开启（+ `foreign_keys=ON` :38）。**未使用** `_mox_migrations`，靠 `CREATE TABLE IF NOT EXISTS` + 列已存在容错做幂等。

## 2. meta 元数据

- **DDL 文件**：`mox-platform-meta-core/src/ddl.sql`。
- **全部表名（16，与预期一致）**：
  `meta_industry_package`、`meta_tenant_industry`、`meta_entity`、`meta_field`、`meta_view`、`meta_view_column`、`meta_workflow`、`meta_workflow_node`、`meta_workflow_transition`、`meta_workflow_instance`、`meta_workflow_instance_state`、`meta_rule`、`meta_page`、`meta_component`、`meta_field_option_dict`、`meta_field_option_dict_item`。
- **建表入口**：`repo.rs:16` `DDL_SQL = include_str!("ddl.sql")`；`repo.rs:141` `init_schema()` → `execute_batch`（:149）。与 IAM 同库同连接。

## 3. DSQL

- **SQLite 迁移（运行时实际执行）**：
  - `migrations/001_init.sql`：`dsql_datasource`、`dsql_definition`、`dsql_version_history`、`dsql_audit_log`；
  - `migrations/002_process.sql`：`dsql_process_definition`、`dsql_process_audit`；
  - `migrations/003_logic_and_enhancement.sql`：`dsql_logic`、`dsql_logic_version`（纯索引增强，无新表）。
  - **合计 8 张表**。
- **Postgres 版**：`migrations/dsql_schema_postgres.sql`（BIGSERIAL / JSONB / TIMESTAMPTZ，`psql -d mox` 执行），同名 8 表，含分区建议。
- **结论：双套确认**——SQLite 迁移由 Rust 运行时 `include_str!` + `execute_batch` 自动跑（`mox-dsql-core/src/storage.rs:36-48`）；Postgres 文件为生产部署参考脚本，**代码不会自动执行**。WAL 见 `storage.rs:21`。

## 4. datastore / biz_data

- **DDL 文件**：`mox-platform-datastore-core/src/ddl.sql`：`biz_data`（万能表，~40 个 ext_* 槽位列 + 软删除 + 版本链）、`biz_data_version`（版本链）、`biz_data_relation`（关联关系）。
- **注意双处定义**：`dao.rs:54 init_schema()` 是**运行时真正执行**的建表，内联 `CREATE TABLE biz_data`（`id` 主键，:76）+ `biz_data_version`（:112）；`ddl.sql` 为配套/参考（字段命名 `biz_id` 与运行时 `id` 略有差异）。
- **通用 KV**：`lib.rs:226-231` `KvStore::new(conn, table)` 动态建 `kv_store(key,value,updated_at)`（表名可传入），不是 ddl.sql 里的固定表。
- **WAL/迁移**：`lib.rs:138` `PRAGMA journal_mode=WAL; foreign_keys=ON; synchronous=NORMAL;`。

## 5. KG

- **core**（`mox-kg-core/src/storage.rs`）：`kg_vertex`、`kg_edge`，SQLite 内联 DDL `execute_batch`（:39-64），`GraphStorage::open(path)`（:17）。**是 SQLite**。
- **storage-svc**（`mox-kg-storage-svc/src/lib.rs`）：`kg_nodes`、`kg_edges`，SQLite 内联 DDL（:345-360）。注意它是**整表快照**：`persist()` 先 `DELETE FROM kg_edges/kg_nodes`（:368-369）再全量重插，`load()` 全量读入内存 `GraphStore`。
- 两套表名并存（`kg_vertex/kg_edge` 与 `kg_nodes/kg_edges`），是两条并行实现，未统一。磁盘 `data/mox-kg-graph.json` 为 JSON 文件快照。

## 6. AI / 联盟

- **expert-svc**：`mox-ai-expert-svc/src/persistence.rs:40-59` 内联建 `experts(id,meta_json)`、`sessions(id,data_json)`、`plans(id,data_json)`、`kv(key,value_json)`，WAL（:39）。**db 路径**：`server.rs:516` 默认 `data/mox-expert-svc.db`（env 可覆盖；测试 `:memory:` :522）。
- **agent-svc**：
  - `conversation.rs:50-51`：`sessions`、`messages`；
  - `dialogue_graph.rs:53,59`：`dialogue_sessions`、`dialogue_messages`；
  - `requirement_compiler.rs:234`：`blueprints`；
  - db 文件名：`lib.rs:115` `"operator_dialogue.db"`（相对 cwd）。
- **联盟 alliance_tasks/events/phase_stats**：
  - PostgreSQL schema 在 `mox-ai-alliance-engine/migrations/001_init.sql`（UUID/JSONB/TIMESTAMPTZ/plpgsql 触发器）：`alliance_tasks`、`alliance_events`、`alliance_experts`、`alliance_phase_stats`、`schema_migrations`。
  - **但运行时默认是内存**：`persistence.rs:231` `impl Default for InMemoryTaskRepository`；`DatabaseTaskRepository`（PgPool，:406-409）需显式传 `database_url` 才启用。测试全部用 InMemory（:895-993）。
  - 磁盘 `data/alliance_tasks.json` 证实当前以**文件快照**落盘，未落 SQLite/PG。

## 7. primiflow

- **运行时（真实落盘）**：`mox-flow-primiflow-svc/src/persistence.rs:116-126` `SCHEMA_SQL` 建 `kb_assets`、`kb_graph`、`trace`、`projects` 4 表；`Persistence::sqlite(path)`（:140）经 `SqlitePersistence::file` 落盘，磁盘 `data/store.db`。
- **gen/ddl.sql**：`src/gen/ddl.sql` 是带 pgvector 注释的 Postgres 风格**生成产物/规格**（projects/conversations/topologys/assets/artifacts/trace_links 6 表），**不是运行时建表脚本**；运行时并无 conversations/topologys/artifacts/trace_links 这几张表（运行时仅 4 表，且 kb 图以 `kb_graph.graph_json` 单行 JSON 存）。

## 8. market

- `mox-market-template-svc/src/template_market/seed.rs:91-153`：`product`、`cart`、`orders`、`payment`（mall 版）+ `member`、`point_log`（member 版）。
- **关键定性**：这是**模板市场向外发放的 PostgreSQL 风格 DDL 模板字符串**（`mall_schema_sql()` / `member_schema_sql()`），不是 market 服务自身的持久化。market 服务本身无独立业务库。测试 `tests.rs:130-131` 仅断言字符串包含这些表名。

## 9. cloud

- svc 组件齐全：`mox-cloud-master-svc` / `-volume-svc` / `-s3-svc` / `-filer-svc` / `-rebalance-svc` / `-server`。
- **对象存储定位确认**：filer 仅在内存用 SQLite 存 inode 元数据（`meta_sqlite.rs:9` `Connection::open_in_memory()` 建 `inodes(...)` 表），**无持久化关系业务表**，无纠删码/业务关系表落盘文件。

## 10. kb

- `mox-kb-server/src/main.rs:24`：`InMemoryKbStore { docs: RwLock<HashMap<String, Document>> }`，`main.rs:69-70` 注入 `KbManager`。
- `mox-kb-core/src/lib.rs:32` 定义抽象 `KbStore` trait（save/get/search/delete/list_versions），注释写明"可接入 SQLite/PostgreSQL/Cloud"，但**当前唯一实现是内存 HashMap**。
- document/model/search/analyze/link/version 全部为进程内存态，**无表、无文件**。

---

## 横切问题（文件:行号证据）

### Q1. 现在到底几个 SQLite 文件？各在哪？谁连谁？

磁盘 `data/` 实测（2026-09-17）：

| 文件 | 大小 | 谁打开 | 打开点 |
|---|---|---|---|
| `data/mox.db` | 4.1 MB | enterprise-svc（:3002 企业真源），IAM+meta+biz_data **共用一个连接** | `enterprise-svc/src/main.rs:63` → `app_state.rs:29-46` `Connection::open(path)` |
| `data/mox-expert-svc.db` | 36 KB | expert-svc | `expert-svc/src/server.rs:516`（默认值） |
| `data/experts.db` | 172 KB | expert-svc（另一路径/历史） | 同 PersistenceDb::open |
| `data/store.db` | 12 KB | primiflow | `primiflow-svc/src/persistence.rs:140` sqlite(path) |

另有**文件快照**（非 SQLite）：`data/alliance_tasks.json`（联盟任务）、`data/mox-kg-graph.json`（KG 图）。
运行时另有相对 cwd 的 `operator_dialogue.db`（agent-svc，`lib.rs:115`）。

**谁连谁**：`mox.db` 是唯一"多域共享"的库——IAM、meta、biz_data 三个 repo 持有同一个 `Arc<Mutex<Connection>>`（app_state.rs:44-46）。其余域各开各的文件，互不连接。

### Q2. `_mox_migrations` 表的定义与写入点；共用还是各自一套？

- **唯一定义点**：`mox-platform-datastore-core/src/lib.rs:176`
  `CREATE TABLE IF NOT EXISTS _mox_migrations (name TEXT PRIMARY KEY, applied_at TEXT ...)`。
- **写入点**：`lib.rs:177-180`，在 `DatastoreConnection::migrate(name, sql)` 内——先查 `_mox_migrations` 是否已有 name，没有则执行 sql 并 `INSERT INTO _mox_migrations`。
- **共用还是各自一套**：它属于 `DatastoreConnection` 这一通用连接抽象。**实际没有被 IAM/meta/DSQL/KG/primiflow 使用**——这些域全部直接 `execute_batch` 跑 `CREATE TABLE IF NOT EXISTS`，靠幂等容错。真正调用 `migrate()` 的生产路径未见（仅 `lib.rs:275` 自测）。因此它是"挂在 datastore 框架上、各连接独立记账"的机制，**不是跨域统一迁移表**；当前事实上各域各搞各的幂等建表。

### Q3. 哪些域除 SQLite 外还有 Postgres/KingbaseES schema 文件？

| 域 | PG schema | 状态 |
|---|---|---|
| DSQL | `migrations/dsql_schema_postgres.sql` | **已备 PG schema**（同 8 表，生产参考） |
| 联盟 alliance-engine | `migrations/001_init.sql`（PG 方言） | **已备 PG schema**（运行时默认仍 InMemory，DB 仓储需显式开启） |
| market 模板 | `seed.rs:91-153`（PG 风格字符串） | 对外模板，非自身库 |
| primiflow gen | `src/gen/ddl.sql`（PG 风格） | 生成规格产物，非运行时 |
| IAM / meta / datastore / KG / expert-svc / agent-svc / primiflow 运行时 / kb / cloud | — | **仅 SQLite**（或内存/文件） |

未发现 KingbaseES 专属 schema 文件（PG 方言文件理论上可作为 KingbaseES 基础适配，但无专门 KBS 脚本）。

### Q4. 启动建表入口：gateway 启动时跑了哪些 init_schema？

- **企业真源入口**：`mox-platform-enterprise-svc/src/main.rs:63` `AppState::open_memory_or_file(&db_path, ...)`（DB_PATH 默认 `:memory:`，生产指向 `data/mox.db`）。
- 其内 `app_state.rs:52-61` 在一个 `spawn_blocking` 内依次跑：
  1. `iam.init_schema()`（:54）+ `iam.seed()`（:55）
  2. `meta.init_schema()`（:56）+ `meta.seed_industry()`（:58）
  3. `dao.init_schema()`（:60）—— biz_data 系。
  三者共用同一 `mox.db` 连接（app_state.rs:44-46），WAL 在 :37 开启。
- **编排器自身表**：`mox-platform-system-core/src/store.rs:89` `repo.migrate()` 负责旧编排器的 璇玑/成员/任务/审计 表（与 enterprise-svc 的 mox.db 是另一套 Store）。
- 其余域（DSQL、KG、expert-svc、primiflow）的建表都在**各自进程打开自己的 SQLite 文件时**由各自 `open()` 内联 `execute_batch` 完成，不经过 gateway 统一入口。

---

## 一句话结论

**当前是"一主多从"的碎片化持久化**：`data/mox.db`（4.1MB）是唯一聚合库，容纳 IAM(22)+meta(16)+biz_data(3)；DSQL、KG(core/storage-svc 两套表名)、expert-svc、agent-svc、primiflow 各自独立开 SQLite 文件；联盟/KB/cloud 实为内存或 JSON 文件快照。Postgres schema 仅 DSQL 与联盟"备好未接"，运行时仍是 SQLite/内存主导；`_mox_migrations` 框架存在但事实上各域各自幂等建表，未形成统一迁移治理。
