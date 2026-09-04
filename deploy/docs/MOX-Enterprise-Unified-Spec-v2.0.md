# MOX 企业级归一化架构与交付总纲 v2.0

> **本文档定位：MOX 全生命周期唯一的总纲规范。**
> 一个概念只在此定义一次。操作级子手册（切换步骤 / Runbook / 容量）仅在文末附录列路径和摘要，正文不重复展开，无多余设计。
>
> **生效范围：** T0（单机 SQLite+FS）→ T3（3 Region × 千亿亿级 / ZB 容量），所有档位共享统一抽象。
> **必须配套代码锚点：** 所有规范对应到 `platform/backend-node/src/` 的真实文件与行号，无虚设条目。

---

## 1. 文档归一化规则（先读，避免重复）

1. **一次定义原则：** 三大铁律 / Key 同构公式 / 28 项验收 / 6 域图谱 Schema / 档位 T0–T3 / 12 阶段 / SLO 阈值 / 红线 —— 全文各只出现 **一次**。后续章节只引用「§x.y」编号，不重写。
2. **抽象统一：** 所有存储（SQLite / PG / 云）走同一个 `StorageProvider`；所有云盘后端（FS / MinIO / S3 / OSS）走同一个 `ChunkBackend`；所有业务产物走同一个「知识图谱中枢 Node + Edge」。
3. **子规范不展开：** 运维级操作步骤、告警 YAML、巡检脚本见附录 B「子规范索引」，正文只说「做什么 / 验收什么」，不说「具体点哪几个按钮」。
4. **企业级红线前置：** 所有不可违反的约束（RPO / 审计 / 不可删除 / 0 差异）直接写在条目末尾红色块 `🚨 MUST`，不用另外翻文件。

---

## 2. 总架构（6 层中枢化）

### 2.1 6 层横向分层（自顶向下）

| 层 | 名称 | 职责 | 关键抽象 |
|---|---|---|---|
| L5 | 交互 & 接入层 | AI 工作台 / 多租户 / GeoDNS / API GW / WAF | `routes/` 路由集 · RBAC/ABAC |
| L4 | 应用 & 集成层 | 专家联盟 AI Engine / WORM 对象锁 / 事件总线 / LLM 市场 | `expert-alliance-engine.js` · 原子写 JSON |
| L3 | 核心服务层 | 云盘 FS/S3 同构 / 版本 / 引用计数 GC / 存储 Provider 抽象 | `file-store.js` · `storage/index.js` |
| **L3.5** | **知识图谱中枢（唯一）** | **统一 Node + Edge 存储；所有业务产物横向关联；跨阶段追溯** | `graph_edges` 表 + 6 方法接口 |
| L2 | 数据 & 存储层 | 元数据分片 / 对象 3 层生命周期 / TiKV / Iceberg 湖仓 / 向量索引 | better-sqlite3 → PG → TiDB 平滑切换 |
| L1 | 基建 & SRE 层 | 3 Region × 3 AZ / 37 项 SLO / F1-F14 Runbook / FinOps / 审计 hash_chain | Helm 伞图 · Prometheus · ClickHouse · OTel |

### 2.2 唯一主路径（垂直贯通，不重复）
所有请求与写入必须走：`L5 → L4 → L3 → L3.5 落中枢 → L2 → L1`。**L3.5 落图不是可选，是 MUST。**

```
L5 入口 ──→ L4 业务 ──→ L3 核心 ──→ ★ L3.5 图谱中枢 ★ ──→ L2 存储 ──→ L1 基建
                │            │            │  (Node+Edge)  │
                └────────────┴────────────┴───── 横向跨域关联（所有实体互跳）
```

### 2.3 变更治理 · 三大铁律（仅此一处定义 · MUST）
🚨 **任何 FS↔S3 / SQLite↔PG / 单 Region↔多 Region 的切换都必须遵守，零例外：**
1. **真相源唯一：** 切换前必须明确磁盘/对象端哪一侧是 Source of Truth，绝不双真相。
2. **对账窗口必开：** 切换后至少 7×24h 深度对账，差异必须 = 0；非 0 绝不关双写。
3. **回滚链先演练：** 正式切换前必须完整走一遍回滚成功，再推切换；绝不允许「到时候再说怎么回滚」。

---

## 3. 存储统一规范（档位 + 同构 + 切换）

### 3.1 统一 Provider / Backend 抽象（仅此一处）
- **图谱存储 Provider 抽象接口：** `StorageProvider` 声明 18 个方法 + 4 个默认实现（SQLite / Memory / Postgres / MySQL stub）。
  代码锚点：[storage/index.js L5-L29](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js#L5-L29)、[storage/index.js L733-L746](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js#L733-L746)
- **云盘 Backend 路由：** `FILE_BACKEND ∈ { fs | s3 | minio | oss }`，运行时通过 env 切换。
- **双写机制：** `DualWriteStorage(primary, secondary)` 实现三大铁律的第 2 条对账窗口。
  代码锚点：[storage/index.js L623-L731](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js#L623-L731)

### 3.2 档位 T0–T3（仅此一张矩阵 · 唯一）
升级阈值写死，不允许靠人拍脑袋：
| 档 | 对象数 / 容量 | 图谱 | 云盘 | SLA 红限 |
|---|---|---|---|---|
| T0 默认 | < 10⁷ / < 10TB | SQLite WAL | FS 两级散列 | ≥ 99.5% |
| T1 中小 | 10⁷–10¹⁰ / 10TB–1PB | PG 1 主 2 从 | S3/MinIO EC 4+2 | ≥ 99.9% · RPO<5m |
| T2 亿级 | 10¹⁰–10¹⁴ / 1PB–100PB | TiDB/TiKV 256 shards | S3 双 Region CRR | ≥ 99.95% · RPO=0 |
| T3 千亿亿级 | 10¹⁴–10²⁰ / 100PB–10ZB | 1024 vnode 全球 | EC 12+4 · 3 Region×3AZ | ≥ 99.99% · RTO<30s |

🚨 **MUST：** 任何一档的单次 GC dry-run 超过 24h / 单 PG > 2TB / 跨域延迟 > 50ms 不可接受 → 立即升档。

### 3.3 环境变量总表（仅此一张 · 唯一）
**图谱存储 9 变量：** `DB_PROVIDER`、`DB_PATH`、`PG_HOST/PORT/USER/PASS/DB`、`STORAGE_DUAL_WRITE`、`STORAGE_READ_PREF`
**云盘 10 变量：** `FILE_BACKEND`、`DATA_DIR`、`S3_ENDPOINT / BUCKET / ACCESS_KEY / SECRET / REGION / SSE / FORCE_PATH_STYLE`、`MINIO_URL`、`OSS_ENDPOINT`
🚨 **MUST：** ACCESS_KEY / SECRET **只允许从 Secret envFrom 注入**，禁止写进 custom-values.yaml；违反直接打回 CR。

### 3.4 云盘 Key 同构公式（仅此一处定义 · 唯一）
**FS 与所有对象后端（S3 / MinIO / OSS）共用同一个 Key 公式，零改代码切换：**
```
key = sha256(content).slice(0, 2) + '/' + sha256(content)
```
- FS 路径 = `DATA_DIR/file-store/chunks/<xx>/<sha256>`
- S3 对象键 = `<xx>/<sha256>`
**代码同构证明（唯一锚点）：**
  FS 端 [chunk-backend.js L62](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L62)
  S3 端 [chunk-backend.js L164](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js#L164)
  两行表达式完全相同，即「同构」的数学证明。

### 3.5 引用计数 GC（仅此一处）
Chunk 不随文件删除而物理删；先减引用计数，计数归零且 30 天 dry-run 通过才进入真实回收。
代码锚点：[file-store.js L126-L136](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L126-L136)
版本秒级回滚（零拷贝）：[file-store.js L371-L396](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js#L371-L396)

### 3.6 立即修补 3 件事（仅此一处 · 本周落地）
> 3 个低成本、高收益的硬修补，总工作量约 2 人天，不含任何新设计：
1. **`lib/json-store.js` 的 writeJSON 改成 tmp+rename 原子写**，直接抄已正确实现的
   [expert-alliance-engine.js L40-L48](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/expert-alliance-engine.js#L40-L48)
   当前 json-store 直接原地 `writeFileSync`，崩溃会半写截断 → 修好之后崩溃只会保留旧文件。
2. **CI 加一条 300MB 混合载荷烟测：** 10% 空 / 30% <4KB / 50% 2–50MB / 10% 100MB+，重复内容 3 份。验收硬指标 `hashMismatch = 0` 且去重率与实际重复率吻合。
3. **运维脚本落地成真实文件：** 把三本手册里的「4 档 Prometheus 告警 YAML」「FS/Bucket 对账 Cron」「verify-key-isomorphism.sh」从 MD 里搬出到 `deploy/scripts/` 和 `deploy/prometheus/`，禁止脚本只存在于文档里。

---

## 4. 端到端业务流程（12 阶段图谱增强 · 唯一）

### 4.1 编号规范 · P0–P12（仅此一组）
- **立项 P0** → 知识抽取 P1 → 架构&UI 设计 **P2**
- **构建 P3** AI 代码 → **P4 云盘&图谱基线（红限重点）** → 测试修复 **P5**
- **切换 P6 FS↔S3/PG（红限重点）** → 4 档灰度 **P7** → UAT 验收 **P8**
- 运行 **P9** → FinOps 容量 **P10** → 审计归档 + AAR **P11/P12**

### 4.2 跨阶段关联 3 条金链（MUST · 唯一）
所有阶段的 Node/Edge 写入完成后，这三条链必须 `findPath()` 非空，否则阶段不推进：
1. **需求根因链（6 跳）：** `Requirement → Design → API_Contract → CodeFile → TestCase → Bug`
2. **切换审计链：** `CR-001 → SwitchPlan → MigrateJob → VerifyReport → Snapshot → hash_chain_block`
3. **组织自进化闭环（绿色回流）：** `AAR（事后复盘）—[improves_next]→ 下次 P0 Project 基线`

### 4.3 9 日组合上云节奏（图谱 + 云盘同时切换 · 唯一）
D-1 回滚演练成功 → D0 预检查 + env 热切 → D1-D3 图谱双写 + 存量迁移 → D4-D6 云盘热切 + 冷热迁移 → D7 空读回填 100% → D8 7×24 对账差异 = 0 → D9 关双写 + 归档入链。具体步骤、API 入口、回滚预案见 **附录 B-01**。

---

## 5. 知识图谱中枢规范（万物关联 · L3.5 唯一实现）

### 5.1 架构定位（MUST · 唯一）
L3.5 是「唯一横向关联中枢」，不做任何业务计算；**只做两件事：存 Node + 存 Edge。** 上层任何业务模块（项目 / 云盘 / 故障 / 归档）通过同一套接口写入和检索，不允许各模块自造关联表。

### 5.2 存储扩展（唯一 DDL）
在现有 `entities / kv_store / logs` 三张表**之后**，追加一张 `graph_edges`。向后兼容，0 破坏：
```sql
CREATE TABLE IF NOT EXISTS graph_edges (
  id INTEGER PRIMARY KEY,
  src   TEXT NOT NULL,
  rel   TEXT NOT NULL,
  dst   TEXT NOT NULL,
  props TEXT,
  tombstone INTEGER DEFAULT 0,
  reason TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(src, rel, dst)
);
CREATE INDEX IF NOT EXISTS idx_edges_src ON graph_edges(src);
CREATE INDEX IF NOT EXISTS idx_edges_dst ON graph_edges(dst);
CREATE INDEX IF NOT EXISTS idx_edges_rel ON graph_edges(rel);
```
注入点（仅此两处）：[db.js L15-L43](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/db.js#L15-L43) 的 `exec` 块、以及 [storage/index.js SQLiteProvider.connect()](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/index.js#L40-L100) 的同构 DDL。

### 5.3 6 域 25 实体类型 40 关系类型 Schema（仅此一张 · 唯一）
一个 `entity_type` / 一个 `rel` 只在这里出现一次，绝不允许业务代码私造字符串：

| 域 | Nodes entity_type | Edges rel |
|---|---|---|
| ① 租户&项目 | `projects`、`tenants`、`users`、`roles`、`org_departments`、`sign_records` | `member_of`、`owner_of`、`signs`、`approves` |
| ② 需求&设计 | `requirements`、`design_docs`、`ui_pages`、`api_contracts`、`documents`、`tags` / `concepts` | `tracks_back_to`、`realized_by`、`implements`、`has_ui`、`mentions` |
| ③ 代码&测试 | `code_files`、`code_functions`、`libs_pkgs`、`test_cases`、`test_runs`、`bugs`、`patches` | `implements`、`depends_on`、`tests`、`covers`、`found_in`、`fixes` |
| ④ 云盘&存储 | `files`、`file_versions`、`file_chunks`、`file_chunk_refs`、`buckets`、`endpoints`、`lock_records`、`file_store_index` | `contains_chunk`、`same_hash_as`、`refed_by_file`、`locks_immutable`、`attached_as`、`migrated_to` |
| ⑤ 变更&交付 | `switch_plans`、`migrate_jobs`、`verify_reports`、`canary_stages`、`snapshots`、`uat_cases`、`cr_documents` | `targets`、`released_via`、`rollback_to`、`validates_end_to_end` |
| ⑥ 运维&归档 | `incidents`、`runbooks`、`alerts`、`metric_series`、`shards`、`tier_rules`、`archive_records`、`aar_reviews`、`hash_chain_blocks` | `caused_by`、`resolved_by`、`moved_to_tier`、`balanced_to`、`contains`、`improves_next` |

### 5.4 接口（6 个方法 · 唯一声明）
统一挂在 `StorageProvider` 抽象层，SQLite/Postgres/TiKV 各实现一次，上层不改：
```
addEdge(src, rel, dst, props?)          // 原子写，UNIQUE 冲突即已存在
removeEdge(src, rel, dst, reason)       // MUST 走 tombstone 标记，不物理删
neighbors(nodeId, dir='both'|in|out)    // 一阶邻居列表
neighborhoodSubgraph(seedIds, hops=3, maxNodes=5000)  // 返回 Cytoscape {nodes,edges}
findPath(fromId, toId, maxHops=6)       // 返回最短证据链
pageRank(relFilter?)                     // 复用 graphEngine.computePersonalizedPageRank
```
`pageRank` 直接复用已实现 [ai-engine.js L239-L252](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/ai-engine.js#L239-L252)，不重复造。

### 5.5 图谱治理三条红线（MUST · 唯一）
🚨 写入 CR-003《知识图谱治理规范》，零例外：
1. **孤立节点率 ≤ 0.5%。** 新实体 0 条边不允许入生产图，自动进「待补全」队列。
2. **§4.2 的三条金链覆盖率 100%。** 任一 Project 节点 `findPath` 返回空 → 阶段不推进。
3. **Edge 不允许物理删除。** 删除只改 `tombstone=1` 并写 `reason`，同时写入 hash_chain_block；7 年内任何时间必须能回放「谁、为何、何时删的」。

### 5.6 立即落地 4 步（唯一排期）
对应 CR-003 的里程碑，本周完成骨架：
1. Step 1（1h）：完成 §5.2 DDL 注入；备份原 ous.db 再执行。
2. Step 2（1d）：在 StorageProvider 声明 6 方法 + SQLite/Postgres 实现；DualWriteStorage 自动双写 edge。
3. Step 3（2d）：P0–P12 每个阶段调用点各加 1 行 `addEdge`；不影响现有 entities 写链。
4. Step 4（1d）：开放 2 个 API `GET /kg/neighborhood` + `GET /kg/path` 给前端 Cytoscape 邻域子图。

---

## 6. SRE 与 SLO（37 项的唯一阈值基线）

本节给**每一层统一的 SLO 红限**；具体 37 项 Prometheus 指标、告警 YAML、Runbook F1-F14 内容见附录 B-02 / B-03。

| 层 | 核心 SLO 红限（MUST） | 违反后果 |
|---|---|---|
| L5 Web 接入 | p95 < 300ms；鉴权 p99 < 80ms；跨租户 0 泄露 | >5min SRE 立刻介入 |
| L4 AI 引擎 | 单次处理 p95 < 45s；工作流可靠性 ≥ 99.9% | 自动降级已绑定备用模型 |
| L3 核心服务 | 对象上传/下载 p99 < 500ms；hashMismatch = 0；GC 回收准确度 ≥ 99.99% | 非 0 立刻关双写 |
| L3.5 图谱中枢 | 邻域 3 跳查询 p99 < 800ms；孤立节点 ≤ 0.5% | 暂停落新 Node 先补边 |
| L2 分片/存储 | 分片不均衡 ≤ 15%（>25% 强制扩容）；加权成本 ≤ ¥0.035/GB·月（T3） | 扩容专项立即启动 |
| L1 基建 | SLA ≥ 99.99%；告警 MTTR < 5min；混沌演练 100% 通过；审计 100% | 月度红线不达标升级 CR |

---

## 7. 开发完成验收（CR-002 唯一 28 项清单）

28 项只在本节出现一次。A-E 五大类、任何红色警戒项未通过禁止推进。签字页为 CR-002 正文的第 3 页（业务 / 架构 / 安全合规 / SRE 四方）。清单原文在部署验收时使用的唯一电子表单编号 CR-002-2026，条目对应表：

| 类 | 项数 | 红线警戒项 |
|---|---|---|
| A 需求·设计门禁 | 6 项 | A6 零越权；A1 CR-001 签字 |
| B 构建·质量门禁 | 8 项 | B3 SAST 0 Critical；B4 hashMismatch=0；B7 WORM 0 篡改通过 |
| C 切换·灰度门禁 | 7 项 | C2 回滚演练先通过；C3 7×24 对账差异 = 0；C4 GC ≥ 99.99% |
| D 业务·SRE 门禁 | 5 项 | D1 UAT 100%；D3 F1-F14 MTTR < 5min |
| E 合规·归档门禁 | 2 项 | E1 7/7 齐全入 hash_chain；E2 审计零红项 |

> 📌 具体 28 项的每一格「要求 · 证据 · 签字责任人」完整展开版存放在：`deploy/docs/CR-002-acceptance-checklist-v2.0.pdf`（电子签名系统托管），本总纲不重复粘贴 28 行避免冗余。

---

## 8. mox 模块化系统架构开发完成里程碑（唯一排期 · 4 阶段）
从当前代码基线到 T3 企业级稳定运行的唯一节奏，不再另立计划表：

| 阶段 | 周期 | 交付对照章节 | 负责人（建议） |
|---|---|---|---|
| W1–W2 立即修补 | 2 周 | §3.6 三件事 + §5.6 图谱骨架 Step1-2 | 后端 Lead |
| W3–W8 T0→T1 上云 | 6 周 | §3.2 档位 · §4.3 9 日节奏 | SRE + 后端 |
| W9–W20 T1→T2 分库分存 | 12 周 | §2.2 6 层 · §5 图谱中枢 Step3-4 | 架构 + DBA |
| W21–W40 T2→T3 多 Region | 20 周 | §3.2 T3 阈值 · §6 SLO 红限 | 基建 + SRE + 合规 |
| W41+ 永续运营 | 常驻 | §6 / §7 / §5.5 红线 | 平台委员会 |

---

## 附录 A · 代码锚点唯一索引（按文件路径排序，正文已只标一次，此处备查）

> **2026-08-27 更新**：原 `backend-node/src/*.js` 7 个锚点（A1-A7）对应的 Node.js 目录已退役清空，现归档为**历史参考**。所有新开发必须使用下方 Rust 代码锚点（A8-A19）。

### A1-A7（历史参考 · backend-node 已清空，Git 历史可追溯）

1. `platform/backend-node/src/config.js` 存储/云盘默认值 —— §3.1 / §3.3（历史）
2. `platform/backend-node/src/db.js` L11-L43 ous.db WAL schema —— §3.1 / §5.2（历史）
3. `platform/backend-node/src/storage/index.js` L5-L29 Provider 抽象 —— §3.1 / §5.2 / §5.4（历史）
4. `platform/backend-node/src/storage/chunk-backend.js` L62 Key 同构 FS↔S3 —— §3.4（历史）
5. `platform/backend-node/src/file-store.js` L111 索引+引用计数 GC —— §3.5（历史）
6. `platform/backend-node/src/expert-alliance-engine.js` L40-L48 原子写 —— §3.6（历史）
7. `platform/backend-node/src/ai-engine.js` L189-L215 图谱+pagerank —— §5.4（历史）

### A8-A19（当前生产真相源 · 纯 Rust 6 层架构）

8. [Gateway 主入口 build_gateway_router](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/mox-platform-gateway-svc/src/lib.rs#L23-L51) —— **L1 网关层唯一路由入口**，12 端点（/health · 6 KG · 4 AI · /api/v1/status）
9. [Gateway CLI 入口 + 端口绑定](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/mox-platform-gateway-svc/src/main.rs#L16-L66) —— 默认 `0.0.0.0:8080`，替换 3000/3001/3002
10. [31 域模块化路由注册表 routes.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/mox-platform-gateway-svc/src/routes.rs) —— 每域 `(prefix, name, status, owner)` 四要素
11. [KG/AI HTTP 适配层 10 接口](file:///d:/a10/aikjx/gitcode/infotopograph/platform/domains/kg/svc/mox-kg-service-svc/src/http_adapter.rs) —— 6 KG + 4 AI 业务响应
12. [KG 算法核心 11 项数学红线实现](file:///d:/a10/aikjx/gitcode/infotopograph/platform/domains/kg/core/mox-kg-algo-core/src/lib.rs) —— Brandes/harmonic/CNM/PageRank 18/18 tests
13. [框架统一错误 + axum IntoResponse](file:///d:/a10/aikjx/gitcode/infotopograph/platform/framework/src/error.rs) —— 任何层抛出 FrameworkError → 统一 JSON 响应
14. [架构守护测试 7 条不变量](file:///d:/a10/aikjx/gitcode/infotopograph/platform/arch-test/src/lib.rs#L157-L427) —— 分层依赖/跨域 API/循环依赖/API 纯净/架构-数据分离/硬编码路径/插件三方分离
15. [图谱实体表 graph_edges DDL](file:///d:/a10/aikjx/gitcode/infotopograph/deploy/sql/mox-step1-graph-edges.sql) —— SQLite/PG 双方言幂等，4 索引+tombstone
16. [云盘 FS Layout + Reed-Solomon Volume](file:///d:/a10/aikjx/gitcode/infotopograph/platform/domains/cloud/svc/mox-cloud-volume-svc/src/fs_layout.rs) —— ADR-001 §3.4 两级散列 SHA-256 实现
17. [Cloud S3 Server S3 兼容协议层](file:///d:/a10/aikjx/gitcode/infotopograph/platform/domains/cloud/svc/mox-cloud-s3-svc/src/s3_server.rs) —— MinIO/AWS SDK 零改直接访问
18. [Enterprise JWT 签发 + 动态实体 CRUD](file:///d:/a10/aikjx/gitcode/infotopograph/platform/domains/platform/svc/mox-platform-enterprise-svc/) —— 3002 端口 JWT 登录+审计链+hash 链完整性
19. [AI 意图识别 classify_intent + 专家打分](file:///d:/a10/aikjx/gitcode/infotopograph/platform/domains/ai/core/mox-ai-intent-core/src/lib.rs#L305-L410) —— 激活扩散 A5 算法 + IntentPattern/ExpertCandidate 类型

---

## 附录 B · 子规范索引（正文不展开，once-defined 原则）

> **2026-08-27 更新**：B-02/B-03 两份子手册内容已完整合并入 B-01（v2.0.0），并执行物理删除避免概念漂移。任何引用使用「详见 B-01 §x」格式。

| 编号 | 子规范路径 | 摘要 | 正文引用章节 |
|---|---|---|---|
| B-01 | `deploy/docs/FS-S3-full-lifecycle-ops-guide.md` | **唯一的**图谱切换 + 云盘切换 SOP（5+4 步·9 日节奏·回滚·F1-F14 Runbook）【合并原 B-02/B-03】 | §2.3 / §3.1 / §3.4 / §4.3 |
| B-04 | `deploy/docs/ha-capacity-tco.md` | 多活 HA 拓扑 · 容量规划公式 · FinOps ¥0.035/GB·月 基线计算过程 | §3.2 / §6 |
| B-05 | `deploy/docs/xinchuang-matrix.md` | 信创适配矩阵（国密 SM2/SM3/SM4 · 国产库 OS/DB/CPU 兼容列表） | §2.1 L5 |
| B-06 | `deploy/docs/ops-manual.md` | 日常运维操作手册（巡检脚本 / 备份 / 恢复 / 变更窗口） | §6 L1 |
| B-07 | `deploy/docs/trace-8stages-dashboard.json` | OTel 链路看板 Grafana Dashboard JSON 模板（8 阶段端到端） | §6 L1 |
| B-08 | `deploy/docs/MOX-Fullstack-Auto-Delivery-Plan-v2.0.md` | **唯一的** 12 周全自动开发交付计划（W1-W12 Gate + 回滚机制） | §8 |
| B-09 | `deploy/docs/MOX-Architecture-Decision-Records-v1.0.md` | **唯一的**架构决策记录（ADR-001 到 ADR-006 + 待办列表） | §2.1 / §2.2 |
| B-10 | `deploy/docs/MOX-NodeToRust-Migration-Handover-v1.0.md` | **唯一的** Node→Rust 迁移覆盖矩阵 + P0-P3 缺口 20 项 + 证据链 | 本附录 C/D/E |
| B-11 | `deploy/docs/DOCUMENT-INDEX.md` | 文档索引总图（deploy/docs 11 份文档 + 45 份 crate README 索引） | 阅读入口 |

---

## 附录 C · Rust Gateway 8080 全面接管说明（2026-08-27 生效）

### C.1 端口对照表（单二进制收敛）

| 端口 | 原承载技术 | 原服务 | 新地址（统一入口） | 状态 |
|---|---|---|---|---|
| 3000 | Node.js HTTP | backend-node 主 HTTP（32 路由 + 静态） | `http://0.0.0.0:8080` Gateway | **RETIRED 已停用** |
| 3001 | Rust (旧) | 原 operator HTTP | `http://0.0.0.0:8080` Gateway | **RETIRED 已停用** |
| 3002 | Rust (过渡) | mox-platform-enterprise-svc（IAM/动态实体 CRUD/JWT） | 保留，Week 2 前合并入 8080（ADR-007） | **TEMP 过渡端口** |
| **8080** | **Rust axum（新）** | **mox-server 单二进制** · 6 层架构唯一对外入口 | **8080** | **ACTIVE 生产入口** |

### C.2 当前 12 个已就绪接口（12/12 冒烟测试 2026-08-27 通过）

```http
### L0 通用
GET  /health                                    → ok=true, gateway=rust-axum, bind=0.0.0.0:8080
GET  /api/v1/status                             → domains_ready, stub_count=28, endpoints_ready=12

### L2 KG（6）
GET  /kg/v1/stats                               → 图谱统计（nodes/edges/density+文案）
GET  /kg/v1/neighborhood?center=&depth=&limit=  → 邻域子图 Cytoscape 兼容
GET  /kg/v1/path?src=&dst=&k=                   → K 路径查找
GET  /kg/v1/shortest-path?src=&dst=             → 最短路径（Dijkstra）
GET  /kg/v1/centrality?method=&top=             → betweenness Brandes / harmonic closeness / pagerank
GET  /kg/v1/communities?method=cnm              → CNM 模块度贪心凝聚社区

### L3 AI Engine（4）
POST /ai/engine/process      + JSON body        → 自动意图识别→能力路由
POST /ai/engine/analyze      + JSON body        → 显式能力执行
GET  /ai/engine/capabilities                    → 能力矩阵自描述
GET  /ai/engine/metrics                         → 成功率/降级率/延迟 P50/P99
```

### C.3 启动命令（唯一入口）

```bash
cargo run -p mox-platform-gateway-svc
# 或自定义：
cargo run -p mox-platform-gateway-svc -- --bind 127.0.0.1 --port 9000
```

### C.4 31 域状态分布（routes.rs 注册表）

| 状态 | 域数 | 代表 |
|---|---|---|
| READY | 2 域 | kg/v1, ai/engine |
| STUB（占位，返回含 `note` 的结构化 JSON） | 28 域 | chat, kb, mcp, cloud, atlas, auto-dev, tasks, optimizer, security, ... |
| RETIRING | 1 域 | backend-node（已停用） |

---

## 附录 D · Node.js → Rust 迁移覆盖矩阵（2026-08-27 快照）

详细完整 32 模块逐行矩阵见 **B-10** `MOX-NodeToRust-Migration-Handover-v1.0.md` §2。此处给出 once-defined 唯一的顶层汇总（不复制 32 行避免重复）：

```
总加权覆盖度：约 23%
  就绪（80-100%）：   3 / 32 （KG · AI · Gateway）
  部分（30-79%）：  13 / 32 （Cloud 55% / Enterprise 35% / Flow 50% / Atlas-AutoDev 32% / Expert 38% / Data 48%）
  待迁移（0-29%）：  16 / 32 （Chat/KB/MCP/WebSearch/Artifacts/RBAC/Optimizer/Modules/...）
```

按 8 大域加权汇总表：

| 域 | 覆盖度 | 核心 ready 子项 | 最大缺口 |
|---|---|---|---|
| KG 知识图谱 | 85% | algo-core 18/18 tests · HTTP 6/6 | demo→真实数据桥接（P0-2） |
| AI 智能引擎 | 60% | 4 路由 HTTP OK · IntentPattern | LLM provider 路由上线 |
| Cloud 云存储 | 55% | S3/FS/Volume 4 crate 就绪（含 tests） | HTTP 路由挂 Gateway |
| Flow/Workflow | 50% | 6 个 core-svc crate 建立 | 全 HTTP 路由 |
| Expert Alliance | 38% | mox-ai-expert-svc 大模块 70% 实现 | http_adapter 20+ 路由 |
| Atlas + AutoDev | 32% | orchestrator + operator core | P0-P12 管道 HTTP |
| Enterprise 底座 | 35% | 3002 动态实体+JWT 通过 | RBAC + 多租户/FinOps |
| 其他 17 路由 | 15% | crate 部分空壳 | 新建对应 Rust 模块 |

---

## 附录 E · 功能缺口 P0-P3 待补清单（20 项唯一真源）

详细每项的「影响/建议目标/验收标准」见 **B-10** `MOX-NodeToRust-Migration-Handover-v1.0.md` §3-§4。此处只列出排期（不重复细节）：

| 优先级 | 项数 | 代表条目 | 完成窗口 |
|---|---|---|---|
| **P0**（阻断生产） | 3 项 | P0-1 RBAC AuthLayer · P0-2 KG 实桥接 · P0-3 3002→8080 合并 | **Week 1 - Week 2** |
| **P1**（关键业务） | 5 项 | P1-1 Cloud HTTP · P1-2 Chat · P1-3 KB · P1-4 Expert HTTP · P1-5 Orchestrator P0-P12 | **Week 3 - Week 4** |
| **P2**（重要可降级） | 7 项 | Flow WASM · CEM 算法 · MCP Bridge · Audit Log · Marketplace · Data ETL HTTP · backend-rust 迁入（ADR-002 3 月窗口） | **Month 2 - Month 3** |
| **P3**（按需/半年内） | 4 项 | Web Search · Artifacts · Plugin Admin · Engine Kernel/Universe 迁移 | **Q4 2026** |
| **合计** | **19 项**（不含 P3-4 子项） | | |

---

## 附录 F · 架构去重决策与演进路线（2026-08-27 生效）

### F.1 三套历史后端 → 唯一 6 层架构的去重结论

| 项目 | 当前状态 | 去重决策（ADR-001 + ADR-002） | 完成日期 |
|---|---|---|---|
| `platform/backend-node/` | 已清空（0 files / 0 dirs），残留空壳（句柄锁） | **立即退役** · Git 历史保留 30 天审计 · 数据迁到 `projects/` · 空壳重启后删除 | 2026-08-27 内容已删 |
| `platform/backend-rust/` | 独立 workspace · Q/R/S/T 4 模块成熟 + istio 配置 | **不硬删 · 逐模块迁入 6 层架构**（详见 ADR-002 迁入映射表），3 个月窗口，2026-11-27 前未迁出即判定不需要后整删 | 2026-11-27（3 个月） |
| `platform/{gateway,domains,foundation,framework,shared,scripts,arch-test}` | ACTIVE · 60+ crates · 12 接口冒烟通过 | **生产唯一真路径** · 所有新开发必须落在 6 层定位中 · `arch-test` 守护测试 CI 必须过 | **ACTIVE**（立即） |

### F.2 文档去重结论（ADR-006 once-defined）

| 文档 | 状态 | 决策 | 生效日期 |
|---|---|---|---|
| `filesystem-backend-structure-sop.md` | 存在·重复 | **已物理删除**。内容完整合并到 `FS-S3-full-lifecycle-ops-guide.md` v2.0.0 §1-§2 | 2026-08-27 |
| `storage-cloud-switch-sop.md` | 存在·重复 | **已物理删除**。内容完整合并到 `FS-S3-full-lifecycle-ops-guide.md` v2.0.0 §3-§10 | 2026-08-27 |
| backend-rust/*.txt（21 个构建日志） | 存在·垃圾 | **已清理**（159,969 bytes 临时输出） | 2026-08-27 |
| 45 份 crate 内 README/DESIGN/tasks.md | 存在·crate 自描述 | **保留**。Rust crate README 不重复定义全局概念，仅描述 crate API；全局规则引用本总纲/ADR 节号；索引入口见 **B-11 DOCUMENT-INDEX.md** | 保留 |

### F.3 架构演进路线图（唯一排期）

```
2026 Q3 (现在)  ████████  单二进制 Gateway 8080 + Node 退役 + 文档归一化 (今日完成)
2026 Q3 W3-W4  ████████  P0 三项 + P1 五项 = 企业可用生产版本
2026 Q4        ████████  P2 全部 + backend-rust 4 模块迁入 + ADR-007~010 决策落地
2027 Q1        ████████  P3 全部 + AIS 6 层 31 域 100% Ready (Stubs=0)
2027 Q2+       ████████  T2 分库分存上线 → T3 多 Region 双活 (总纲 §8)
```

---

> **文档生效声明（2026-08-27 更新）：** 本总纲为 MOX v2.0 唯一企业级规范。任何与子规范（附录 B 11 份）冲突的地方，以本总纲为准；任何概念重复定义的地方，以「本总纲中对应编号章节」的定义为唯一真源。严禁新增不在 §5.3 Schema 里的 entity_type / rel，违者 CR 直接打回。严禁绕过 6 层架构新增独立监听端口的后端服务（ADR-004 单二进制原则），违者 CR 直接打回。
