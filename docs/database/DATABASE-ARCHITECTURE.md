# 全局数据库架构设计（DATABASE-ARCHITECTURE）

> **定位**：本文件是 MOX/infotopograph **以代码为准的实际持久化架构权威源（as-built）**。
> 数据日期：2026-09-17。证据逐域来自真实 DDL 与建表调用点，完整证据见
> `docs/working-reports/_norm_research/db-inventory.md`（含全部 文件:行号）。
>
> **与既有母版的分工**：本目录 `mox_sys/` 与 `mox 模块化系统架构企业级数据库模板.md` 是
> **MySQL 8.3 目标模型（target）**；本文件是 **Rust 运行时实际落地（as-built）**。
> 运行时当前以 SQLite 为主，二者不是安装关系，目标态见 §3 迁移路线。

---

## 1. 存储分层总览

系统不是单一数据库，而是**按域选择引擎**的四层混合持久化：

| 层 | 引擎 | 承担职责 | 物理形态 |
|---|---|---|---|
| 关系层 | SQLite（WAL）/ 预留 Postgres | 身份、权限、元数据、业务万能表、对话、流程、专家记录 | 各域 `.db` 文件 |
| 图邻接层 | SQLite 邻接表 + JSON 快照 | 知识图谱顶点/边 | `kg_vertex/kg_edge`、`kg_nodes/kg_edges` + `mox-kg-graph.json` |
| 对象层 | 对象存储（master/volume/s3/filer/rebalance + 纠删码） | 文件、卷、块，无业务关系表 | 文件/卷，无关系表 |
| 文档/内存层 | 进程内 HashMap / 文件 JSON 快照 | KB 知识库文档、联盟任务/DAG 运行态 | 无表或 `alliance_tasks.json` |

### 1.1 域 → 引擎 → 物理库/文件 → 建表入口 总表

| 域 | 引擎 | 物理库/文件 | 建表入口 | 表数 |
|---|---|---|---|---|
| IAM/系统 | SQLite WAL | **data/mox.db**（与 meta/biz_data 同连接） | enterprise-svc `app_state.rs:54` → `iam-core/repo.rs:51` | 22 |
| meta | SQLite WAL | data/mox.db（同上共享） | `app_state.rs:56` → `meta-core/repo.rs:141` | 16 |
| datastore/biz_data | SQLite WAL | data/mox.db（同上共享） | `app_state.rs:60` → `datastore-core/dao.rs:54` | 3 + 动态 kv_store |
| DSQL | SQLite（运行）+ PG（参考 schema） | 自身 SQLite 文件 | `dsql-core/storage.rs:36`（001/002/003） | 8 |
| KG core | SQLite | open(path)，无固定名 | `kg-core/storage.rs:37` | 2 |
| KG storage-svc | SQLite + JSON 快照 | data/mox-kg-graph.json | `kg-storage-svc/lib.rs:342` | 2 |
| expert-svc | SQLite WAL | data/mox-expert-svc.db | `ai-expert-svc/persistence.rs:31` | 4 |
| agent-svc | SQLite | operator_dialogue.db（cwd） | conversation/dialogue_graph/req_compiler | 5 |
| 联盟 alliance-engine | 默认内存；PG 可选 | data/alliance_tasks.json（文件快照） | InMemory 默认；PG `migrations/001_init.sql` | 5（PG schema） |
| primiflow 运行时 | SQLite WAL | data/store.db | `primiflow-svc/persistence.rs:140` | 4 |
| primiflow gen | PG 风格（仅生成产物） | 无落库 | `src/gen/ddl.sql` | 6（规格） |
| market | 无自身库，发 PG DDL 模板 | — | `template_market/seed.rs:91-153` | 6（模板） |
| cloud | 对象存储 | 文件/卷，无关系表 | filer 仅内存 inodes | 0 |
| kb | 内存 HashMap | 无 | `kb-server/main.rs:24` InMemoryKbStore | 0 |

> IAM 实际 **22 表**（非预期 24）：iam_tenant/iam_department/iam_user/iam_user_dept/iam_role/
> iam_permission/iam_user_role/iam_role_permission/iam_role_inherit/iam_menu/iam_user_menu/
> iam_role_menu/iam_data_permission/iam_resource/iam_tenant_setting/audit_log/sys_post/
> sys_dict_type/sys_dict_data/sys_config/sys_oper_log/sys_logininfor/sys_api_key。

---

## 2. 统一库 vs 分库现状与建议

**现状：一主多从的碎片化持久化。** 磁盘实测 `data/` 下 4 个 SQLite 文件：

| 文件 | 大小 | 内容 |
|---|---|---|
| `data/mox.db` | 4.1 MB | 唯一聚合库：IAM(22)+meta(16)+biz_data(3)，**同一 `Arc<Mutex<Connection>>`** |
| `data/mox-expert-svc.db` | 36 KB | expert-svc 专家库 |
| `data/experts.db` | 172 KB | expert-svc 另一历史路径 |
| `data/store.db` | 12 KB | primiflow 运行时 |

另有文件快照 `alliance_tasks.json`、`mox-kg-graph.json`，以及 cwd 的 `operator_dialogue.db`。

**建议（企业部署）：聚合为单库多 schema，而非继续分库。**
- mox.db 已证明"多域共享一个连接 + WAL"可行且稳定，应以此为收敛方向。
- 把 expert-svc / agent-svc / primiflow / DSQL 的 SQLite 逐步收敛进同一企业库（同实例不同 schema/表前缀），消除"4 个 .db + 2 个 JSON 快照"的碎片。
- KG 双套表名（`kg_vertex/kg_edge` 与 `kg_nodes/kg_edges`）应择一收敛。
- **迁移路径**：先收敛 SQLite 内部分库→单文件；再在 §3 切 PG 时天然变为单实例多 schema。分库仅在未来按域水平拆分容量瓶颈时再考虑，当前无此必要。

---

## 3. SQLite → Postgres 生产化路线

逐域标注（已核实）：

| 域 | 当前 | PG schema | 切 PG 工作量 | 主要风险 |
|---|---|---|---|---|
| DSQL | SQLite | ✅ 已备 `dsql_schema_postgres.sql`（同 8 表） | 低（schema 已双套） | JSONB 类型映射、分区建议 |
| 联盟 alliance-engine | 内存/JSON | ✅ 已备 `001_init.sql`（UUID/JSONB/plpgsql） | 中（仓储已写 PgPool，需显式开启 + 接入任务） | 默认 InMemory，需改默认开关 |
| IAM / meta / datastore | SQLite | ❌ 无 | 中高（22+16+3 表手写 DDL） | 自增主键→BIGSERIAL/UUID、JSON 列、外键 |
| KG / expert-svc / agent-svc / primiflow | SQLite | ❌ 无 | 中 | 邻接表全量快照写法需改为增量 |
| cloud | 对象存储 | — | 不适用 | — |
| kb | 内存 HashMap | ❌ 无 | 中 | 需先从内存落表，再谈 PG |
| market | 发 PG 模板 | — | 不适用（无自身库） | — |

**结论**：只有 **DSQL、联盟** 两个域"备好未接"PG；其余域全部"仅 SQLite"。
路线建议分三步：
1. **先收内存/文件态**：联盟 DAG 运行态、KB 文档先落 SQLite（否则无库可迁）。
2. **再切已备的两个域**：DSQL 与联盟先接 PG，作为 PoC 验证连接层与类型映射。
3. **最后迁核心关系域**：IAM/meta/datastore 手写 PG DDL，统一主键策略（应用层 UUID v7，对齐 `mox_sys` 母版结论 3），替换 SQLite 自增。

类型差异要点：SQLite 动态类型 → PG 需显式 `JSONB/TIMESTAMPTZ/BIGSERIAL`；外键 SQLite 已 `foreign_keys=ON`，PG 侧保持服务内逻辑外键（对齐母版结论 6：默认不建跨服务物理 FK）。**未发现 KingbaseES 专属脚本**，PG 方言文件可作适配基础但需单独验证。

---

## 4. 迁移治理

- `_mox_migrations` 唯一定义/写入在 `datastore-core/src/lib.rs:176-180`（`migrate(name,sql)`：查表→执行→记账）。
- **事实：框架挂着、未统一。** IAM/meta/DSQL/KG/primiflow 都不用它，各自 `execute_batch` + `CREATE TABLE IF NOT EXISTS` + "duplicate column" 容错做幂等。真正生产调用 `migrate()` 的路径未见（仅自测）。
- 现状结论：**没有跨域统一迁移表，也没有版本回滚机制**；靠幂等建表保证重复启动不出错，但 DDL 演进只能靠"加列容错"式补丁（如 IAM 末尾的幂等 ALTER）。
- **建议**：以 `_mox_migrations` 为统一记账表，把各域 `init_schema()` 改走 `migrate(name, sql)`；新域必须用迁移表；回滚策略初期可不做（幂等前向迁移），但版本号必须单调、可审计。

---

## 5. 已知技术债（单列）

| # | 技术债 | 证据 | 影响 |
|---|---|---|---|
| T1 | **联盟 DAG 运行态内存未落盘** | `alliance-engine/persistence.rs:231` InMemory 默认；磁盘 `alliance_tasks.json` 为全量快照 | 重启在途任务/DAG 节点状态丢失；全量快照有放大 |
| T2 | **KB 纯内存 HashMap** | `kb-server/main.rs:24` InMemoryKbStore | 重启知识库全丢，且无 SQLite/PG 实现可迁 |
| T3 | **API-Key 重启回灌缺失** | 鉴权中间件只持内存表，启动未从 `mox.db` 回灌 `sys_api_key` | 重启后已发 API Key 失效 |
| T4 | KG 双套表名并存 | `kg_vertex/kg_edge` vs `kg_nodes/kg_edges`，整表 DELETE 重插 | 两套未统一，全量快照并发有放大 |
| T5 | `_mox_migrations` 未被采用 | 见 §4 | 无统一版本治理 |
| T6 | cloud filer inode 仅内存 SQLite | `meta_sqlite.rs:9` open_in_memory | inode 元数据重启丢失（对象本体不丢） |

> 与更早的安全/持久化差距报告一致：T1/T3 属 P1 待办；auth_session 明文密码迁移 TODO 仍挂起，未见迁移脚本。

---

## 6. 权威关系

- 运行态事实（本文件）：`docs/database/DATABASE-ARCHITECTURE.md`（本文件）。
- 目标母版（MySQL 8.3）：`docs/database/mox_sys/mox_sys-universal-template.sql` + `docs/database/README.md`。
- 证据：`docs/working-reports/_norm_research/db-inventory.md`。
- 端口/进程事实：`docs/api/PORT-REGISTRY.md`；接口事实：`docs/API-REGISTRY.md`。
