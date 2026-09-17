# 企业级最优存储算法与处理流程设计（STORAGE-OPTIMAL-DESIGN）

> **定位**：本文件是 MOX/infotopograph **企业级目标态（target）存储设计稿**，与 as-built 权威
> `DATABASE-ARCHITECTURE.md` 配套。as-built 回答"现在是什么"，本文件回答"应该演进到什么、为什么"。
> 设计日期：2026-09-17。只做设计，不改业务代码。

---

## 0. 设计原则

1. **按数据特征选引擎，不一律 SQLite**：读写比、生命周期、一致性/性能要求决定选型。
2. **先救"重启即丢"，再谈高并发**：RPO 从∞（纯内存）收敛到秒级，再优化吞吐。
3. **单库 WAL 是当前最优，PG 是上限而非起点**：企业内部百级并发下 SQLite WAL 足够；拆库/上 PG 是容量与并发到顶后的动作，不是默认动作。
4. **增量优于全量**：全量快照只用于冷备，运行态一律增量/append。
5. **统一迁移治理**：把已定义但闲置的 `_mox_migrations` 定为唯一版本记账框架。

---

## 1. 按数据特征选最优存储算法

### 1.1 KB 知识库文档 / 向量 / 分块（当前全内存，最优先）

**现状**：`InMemoryKbStore { HashMap<String, Document> }`（kb-server/main.rs:24），重启全丢。

**最优选型：单库 SQLite + FTS5，向量侧挂轻量方案。**
- **为什么 FTS5 而不是外部 ES**：单节点、知识库规模在十万级文档以内时，FTS5 与 SQLite 同文件、零额外进程、随 WAL 事务一致，运维成本远低于独立 ES 集群；引入 ES 需独立部署、索引重建、集群一致性，属 P2 规模扩张后才需要。
- **落法**：`kb_doc(id, title, source, mime, created_at, deleted_at)` 存元数据；`kb_chunk(doc_id, seq, text, embedding_ref)` 存分块；`CREATE VIRTUAL TABLE kb_fts USING fts5(text, content='kb_chunk')` 做 BM25 全文检索。
- **向量存哪**：embedding 维度通常 768/1024，**不入关系索引**——存 `kb_chunk.embedding BLOB`（float32），检索用应用层**暴力余弦 + 预过滤**（按 doc_id/租户先筛到数百条再算相似度）。只有当分块 >50 万且 QPS 高时才上专用向量索引（sqlite-vec / lantern），当前不值得引入。
- **分块与 embedding 组织**：分块表带 `doc_id + seq` 顺序锚，重 embed 时按 `doc_id` 级联删除重写，不做全局向量版本耦合。

### 1.2 联盟任务 / DAG 运行态（当前内存 + JSON 全量快照）

**现状**：`InMemoryTaskRepository` + 每次 save 全量 `alliance_tasks.json`。

**最优选型：关系表 + 增量 checkpoint，精确恢复到节点。**
- **任务表**：`alliance_task(id, goal, state, fusion_strategy, expert_weights_json, created, updated)`。
- **DAG 节点表**：`alliance_task_node(task_id, node_id, expert, deps_json, state, started_at, finished_at, result_ref)`。
- **增量 checkpoint**：节点状态转移（pending→running→done/failed）各写一行 `UPDATE` 或 append `event`，**不再整任务全量序列化**。崩溃重启后按 `alliance_task` + `alliance_task_node` 重建 DAG：running 态节点标为"未完成"重跑或按幂等键跳过。
- **恢复到哪个节点**：每个节点结果存 `result_ref`（指针而非大对象），重启只需重放未完成节点，已完成节点直接读结果——**精确到节点级 RPO**。
- 已备的 PG schema（`migrations/001_init.sql`）可直接作目标表结构参考，先在 SQLite 落地，PG 切换时同构迁移。

### 1.3 KG 图（当前邻接表 + JSON 快照，双表名并存）

**现状**：`kg_vertex/kg_edge` 与 `kg_nodes/kg_edges` 两套，storage-svc 整表 DELETE 重插。

**最优选型：收敛单套邻接表 + 增量边，不上图数据库。**
- **为什么邻接表而不是图库**：当前查询以点查邻居、单跳/两跳遍历为主，邻接表（`vertex(id,label,props_json)` + `edge(src,dst,label,weight)`）+ `edge(src,label)`、`edge(dst,label)` 两个复合索引足够；引入 Neo4j/Nebula 是多跳递归 + 图算法规模化后的事，当前无此负载。
- **收敛**：择 `kg_nodes/kg_edges` 一套为准（或 kg_vertex/kg_edge，二者取一），废弃另一套。
- **遍历算法**：邻居查询走 `edge(src=?,label=?)` 索引；两跳在应用层做 BFS 并限制深度（≤2）与命中上限（≤200），避免递归 CTE 全图扫描。
- **全量重插改增量**：`persist()` 不再 `DELETE FROM edges` 全量重写，改为按变更集 `INSERT OR REPLACE`/`DELETE`，边表 `src` 索引保证增量 O(Δ)。

### 1.4 cloud 对象 inode 元数据（当前内存 SQLite inode）

**现状**：filer `open_in_memory()` 存 inode，重启丢；对象本体已在卷。

**最优选型：本地嵌入式 KV（boltDB）或 SQLite 元数据表，对象本体不动。**
- **为什么不是纯内存**：inode 是命名空间一致性关键，重启丢会导致卷挂载后"文件看不见"。
- **为什么不是 PG**：inode 是单节点 filer 的本地热点小表，强一致在单机内即可，无需跨网络。
- **选型**：优先 **SQLite `inode` 表**（与现有栈一致、WAL 已验证），key=inode path/id、value=元数据 JSON；若后续需要高频随机小对象元数据，再评估 boltDB/LSM。当前不引新引擎。

### 1.5 IAM/meta 业务表（当前 SQLite WAL 共享 mox.db）

**现状**：IAM 22 + meta 16 + biz_data 3 共用一个 `Arc<Mutex<Connection>>`。

**最优：单库 WAL 是当前最优；明确何时到上限。**
- **WAL 参数**：`journal_mode=WAL` + `synchronous=NORMAL` + `wal_autocheckpoint=1000`（页）。WAL 下单写多读，写吞吐在百级 TPS 内无瓶颈。
- **何时到上限**：`Arc<Mutex<Connection>>` 是**单写者串行**，真正的瓶颈不是 WAL 而是这把全局 Mutex。当单进程写并发 >50 TPS 或出现写锁等待，才：①拆读连接池（写 1 个、读 N 个只读连接，WAL 天然支持多读）；②再不行才把 IAM/meta 拆到 PG 独立实例。**当前不需要。**
- **结论**：先把全局 Mutex 改为"单写 + 多读"连接池即可显著提升，不必急着拆库。

### 1.6 日志 / 审计 / oper_log（追加写、极少更新）

**现状**：`audit_log`（链式哈希）、`sys_oper_log`、`sys_logininfor` 混在 mox.db。

**最优选型：追加式分区表 + 冷热分层。**
- **为什么时序库/独立列存**：审计日志是 append-only、按时间查询、几乎不更新，混在主库会随时间膨胀主库 WAL。
- **落法**：`audit_log` 按月分区（SQLite 内用 `audit_log_2026_09` 表轮换），热数据近 3 个月在线，冷数据导出压缩归档。链式哈希审计（已做）保持不变，保证可追溯不可篡改。
- **GC**：超过保留期（如 12 个月）的分区 detach 导出，不做行级 DELETE。

---

## 2. 处理流程（写路径 / 读路径 / 恢复 / GC）

### 2.1 写路径
- **事务边界**：一个业务操作 = 一个 `BEGIN...COMMIT`；跨表一致性（如建用户+授权）同事务。
- **WAL flush**：`synchronous=NORMAL` 时 commit 落 WAL 即返回，崩溃最多丢最后一个未 fsync 的事务（RPO=秒级）。
- **checkpoint**：`wal_autocheckpoint=1000` 页自动合并，避免 WAL 无限增长。
- **写放大控制**：联盟/KG 改增量而非全量重写（§1.2/1.3）；biz_data 版本链只在真正修订时追加新版本，不原地覆盖历史。

### 2.2 读路径
- **分层读**：内存热层（LruCache，热点 KB 文档/会话）+ SQLite 持久层。读 miss 才落盘。
- 索引命中优先：FTS5 查分块、`edge(src,label)` 查邻居、主键查任务。

### 2.3 崩溃恢复
- **RPO/RTO 目标**：见 §3。
- **重启回放**：SQLite WAL 重放未 checkpoint 事务自动完成（RDB 级）；联盟 DAG 重建节点态；KG 从邻接表加载（不再依赖全量 JSON）。
- **半写记录清理**：`state=running` 的任务/节点重启后标记为 `interrupted`，按幂等键重跑；append 的 event 用唯一 `(task_id, seq)` 去重。

### 2.4 快照与增量
- **全量快照**：仅冷备用，每日一次 `sqlite .backup`，不入运行路径。
- **增量**：运行态靠 WAL + 增量 checkpoint；备份恢复 = 最近全量 + 重放其后 WAL。

### 2.5 GC / 清理
- 软删除 `deleted_at` 标记，定期（如每日）物理清理超期软删。
- biz_data 版本链：保留 N 个历史版本，收敛中间版本。
- KV 表 `kv_store`：定期清理过期 key。
- 审计分区按月 detach 归档。

---

## 3. 一致性与可靠性指标

| 域 | RPO | RTO | 跨域事务 | 一致性模型 |
|---|---|---|---|---|
| IAM/权限 | 秒级（WAL） | <5s | 否（单库本地事务） | 强一致 |
| meta/biz_data | 秒级 | <5s | 否 | 强一致 |
| KB | 秒级（落盘后） | <10s | 否 | 强一致 |
| 联盟 DAG | 秒级（节点 checkpoint） | <15s | 否（任务内事务） | 任务内强、任务间最终一致 |
| KG | 秒级 | <10s | 否 | 最终一致（应用层索引更新） |
| cloud 对象 | 0（对象已在卷） | <5s | 否 | inode 与对象最终一致 |
| 审计日志 | 秒级 | <5s | 否 | 追加、可追溯 |

> **不做跨域分布式事务**：四进程之间用事件/最终一致（任务状态异步同步），不引入 2PC。

---

## 4. 迁移治理：统一 `_mox_migrations`

- **定 `_mox_migrations(name TEXT PK, applied_at)` 为唯一版本记账框架**（已定义于 datastore-core/lib.rs:176）。
- **各域关系**：现状各域 `execute_batch` + `CREATE IF NOT EXISTS` 幂等建表——保留幂等建表作为"基线 schema"，**新增/变更一律走 `migrate(version, sql)`**，不再裸加列。
- **up/down**：初期只做 up（前向幂等）；down 用"新列可空/可忽略"实现，不做破坏性回滚。
- **版本号**：`域_序号`（如 `iam_002`、`kb_001`）单调递增，启动时 `_mox_migrations` 记账，已跑过的跳过。
- **关系**：基线 DDL（include_str! ddl.sql）保证全新部署可用；迁移表保证已部署实例平滑升级。

---

## 5. 落地路线（P0/P1/P2）

### P0（重启不丢，最高优先）
| 项 | 内容 | 验证标准 |
|---|---|---|
| KB 落盘 | kb_store 从内存改 SQLite + FTS5 | 写入文档→重启→重新查询/全文检索仍命中 |
| 联盟 DAG 落盘 | 任务表 + 节点表 + 增量 checkpoint | 提交任务跑到第 N 节点→杀进程重启→任务恢复到断点节点继续 |

### P1（收敛与统一）
| 项 | 内容 | 验证标准 |
|---|---|---|
| KG 双表名收敛 | 择一为准，全量重插改增量 | 新增/删除边后重启，图与持久层一致；持久化耗时 O(Δ) |
| cloud inode 落盘 | filer inode 从 open_in_memory 改持久 SQLite | 重启后已建文件可见 |
| `_mox_migrations` 统一 | 新变更走 migrate()，版本记账 | 全新实例与旧实例升级路径都建出一致 schema |
| IAM 读连接池 | 单写多读 | 并发读压测下写锁等待下降 |

### P2（高并发/规模化）
| 项 | 内容 | 触发条件 |
|---|---|---|
| 拆库/上 PG | IAM/meta 迁 PG；DSQL/联盟先接已备 PG | 写并发 >50 TPS 或单库 >10GB |
| 向量索引 | sqlite-vec/外部向量库 | 分块 >50 万 |
| KB 上 ES | 外部全文检索集群 | 文档 >百万级 |
| 审计独立列存/时序 | 审计迁独立存储 | 日志增速 >GB/月 |

---

## 7. P1 落地补充说明（2026-09-17 执行记录）

- **P1-3 `_mox_migrations` 统一——本期只补指引、不改代码**：评估后，KB 与联盟的新 SQLite 存储已各自带幂等建表（`CREATE IF NOT EXISTS`），强行回灌 `_mox_migrations` 需改动刚验收的两个 crate 并引入记账耦合，风险大于收益。落地指引：①新域/新表仍可直接幂等建表；②**凡涉及"列变更/表结构演进"一律走 `migrate(域_版本号, sql)`**，不再裸加列；③`_mox_migrations` 由 datastore-core/lib.rs:176 的 `migrate()` 统一记账。待下一轮低风险窗口再把 KB/联盟的 init 迁移到该框架。
- **P1-4 IAM 单写多读连接池——本期不动代码，仅记录触发条件**：全局 `Arc<Mutex<Connection>>` 改单写多读会触及 IAM/meta 核心 repo，风险高。触发拆分的条件：单进程写锁等待明显、写吞吐 >50 TPS。届时先把"写 1 连接 + 读 N 只读连接"分离（WAL 原生支持多读），再评估 r2d2 池。当前保持现状。

---

## 6. 权威关系

- 本设计（target）：`docs/database/STORAGE-OPTIMAL-DESIGN.md`。
- 运行态事实（as-built）：`docs/database/DATABASE-ARCHITECTURE.md`。
- 目标母版（MySQL 8.3）：`docs/database/mox_sys/`。
- 证据：`docs/working-reports/_norm_research/db-inventory.md`。
