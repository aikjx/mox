# 开发专家联盟 · 云盘模块 × 关系图模块 自研 全维分析 + 架构/设计文档计划

> 文档路径：`.trae/documents/20260823_cloud_drive_and_mox_selfdev_plan.md`
> 方法论：璇玑 AIS 七层层抽象 + 最优开源参照（SeaweedFS/JuiceFS/NebulaGraph）× 璇玑现有代码基线（SPEC-1~SPEC-v4 S-review）
> 产出形式：专家联盟分析报告 + 里程碑 M0~M6 + 模块矩阵 + 核心接口 + 风险规避 + 验收/验证清单

---

## 一、Repository Research（璇玑当前「云盘 × 关系图」基线 —— 必须作为「自研地基」而非从零重写）

### 1.1 云盘模块 现状（SPEC-1 / SPEC-2 双 GREEN 底座）

| 层级 | 代码位置（绝对路径） | 已实现能力 | SPEC 基线通过 |
|---|---|---|---|
| L2 Gateway | [gateway/runtime/src/routes/ai_engine.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/gateway/runtime/src/routes/ai_engine.rs) | Sidecar 文件 proxy | ✅ SPEC-6 基线 |
| L3 Orchestration | [backend-node/src/routes/atlas.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/atlas.js) | 路由 shell + 审计 hash_chain | ✅ SPEC-v4 GREEN |
| L4 Services (新) | **待新建** `mox-cloud-drive` Rust crate | — | ❌ 本次自研重点 |
| L5 Domain | **抽自现有** `IChunkBackend` trait | 9 方法抽象（write/read/has/delete/list + MPU 4） | ✅ SPEC-2 GREEN |
| L6 Kernel | **待新建** 纯运算（纠删码 Reed-Solomon / SHA-256 + CRC 校验矩阵 / 拓扑秩 / LRU 纯算法） | — | ❌ 本次自研重点 |
| L7 Infra | [chunk-backend.js FSChunk / S3Chunk](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/storage/chunk-backend.js) | 本地 FS + S3 兼容（MinIO/SeaweedFS/OSS）双实现 | ✅ SPEC-2 T2 4 GREEN |
| 上层（Node 业务层） | [file-store.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/file-store.js) | 1 MiB Chunk / 100MB MPU / 30 天 LRU GC / 软删 / 版本 manifest / SHA-256 去重 / Quota | ✅ SPEC-2 T2 16 GREEN |

**已掌握的自研技术栈清单（不用重造）**：
- ① 分块存储通用接口（IChunkBackend 9 方法抽象）；
- ② 纠删码后端（S3/FS 双实现 + MPU 7 方法全通 Node 级 mock GREEN）；
- ③ 版本 manifest（本地 vN.json 主版本 + 可选远端写回）；
- ④ 引用计数 GC（file_chunk_refs entity + 30 天 grace）；
- ⑤ 企业级软删 + Quota 字节控制；
- ⑥ 审计 hash_chain 不可篡改链（sha256 HMAC + TTI 180 天）。

### 1.2 关系图模块 现状（SPEC-3 / 4 / 5 / 7 GREEN 底座）

| 层级 | 代码位置 | 已实现能力 | 基线 |
|---|---|---|---|
| L4 Services (Rust) | [graph-algorithms/lib.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/graph-algorithms/src/lib.rs) | 7 算法 Rust 单源：CNM / PPR（d=0.85 · iter=30）/ Brandes / Harmonic / Degree / Density / RAW 双向展开 | ✅ SPEC-v4 T3/T12 70/70 GREEN Δ≤1e-6；加速 2879× |
| L6 Kernel（内嵌于 operator-core） | [kernel.rs](file:///d:/a10/aikjx/gitcode/infotopograph/platform/services/operator-core/src/kernel.rs) + kernel_ext.rs | 纯 std 类型/运算；LPA 禁用出口公域 | ✅ SPEC-v4 T7 19 GREEN |
| L7 Infra (Adapter) | [nebulagraph-adapter.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/nebulagraph-adapter.js) | Nebula + Mock 双驱动 / CDC 事件总线 / L1 LRU-ttl 10k~20k entries | ✅ SPEC-3 12 GREEN |
| L3 EAF 编排 | [ai-flow-graph.js / workflow-engine.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/workflow-engine.js) | 3 内置 workflow 510 step 节点写回 / runs_on 边 / slo_snapshot | ✅ SPEC-v4 T13 96 GREEN |
| L3 端点 | [routes/graph.js](file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/routes/graph.js) | search rerank + activation spread RRF 融合 rerank | ✅ SPEC-7 7 GREEN |

**已掌握的关系图技术栈**：
- ① 7 核心算法正确性（Node↔Rust 10 fixture × 7 算法 = 70 对账 ≤ 1e-6）；
- ② 图分片路由：project_domain hash（SPEC-1 双写回源基线）；
- ③ CDC + L1 缓存：node 更新 → bus 发事件 → 命中项失效；
- ④ 图谱反向同步铁律：每一次代码提交/业务变更写回六层；
- ⑤ 16 Rust crate 在图谱注册（三注册表）。

---

## 二、最佳开源项目的 代码架构 × 开发规模 基准参考（来源 web 实证）

### 2.1 云盘（分布式对象 + POSIX 文件）三大参照系 横向对比

| 项目 | 开源协议 | 开发语言 | 启动年 → 至今（历时） | 核心代码量（主干）| 核心模块（可直接借鉴到璇玑分层）| 自研规模对标（专家判断）|
|---|---|---|---|---|---|---|
| **SeaweedFS** | Apache 2.0（商业友好） | Go | 2015 → 2026（约 10+ 年）| ~70k LOC Go（weed 主包 46k / pkg 24k）| **Master（卷管理 + 心跳 + 副本分配）** · **Volume（数据实体磁盘 + 校验 + 快照 + 恢复）** · **Filer（POSIX 元数据 / DB 多后端适配）** · **S3 Gateway（104 个 S3 API 实现覆盖）** · **Transparent Tier（热/冷分层）** · **Erasure Coding（28 种 EC 条带配置）** | 最大量级，完整自研 **48~60 人·月** 10 年沉淀 ≈ 璇玑要达到生产 GA 级 S3 兼容 + POSIX 需要 24~36 人·月（在璇玑已有基线之上，**不是重新写 SeaweedFS**，而是 SPEC-2 之上扩 4 大新组件）|
| **JuiceFS** | Apache 2.0 | Go | 2020 → 2026（约 6 年）| ~80k LOC Go（pkg 61k + cmd 10k + sdk 4k）| **Meta 抽象层（Meta Interface + Redis / TiKV / PG / MySQL / SQLite 10 后端）** · **Object 抽象层（ObjectStorage + 30+ 云厂商适配）** · **Chunk / Slice 分片管理 + 加密 + 压缩** · **Multi-Cache 内存/磁盘/分布式 LRU + 预取** · **FUSE / POSIX 接口层** · **Quota / 目录配额 / 快照 / WORM** | 体量比 SeaweedFS 稍大，璇玑如做「云盘 POSIX 语义 + 多 Meta 后端」规模对标 JuiceFS，在已有基线之上 **36~48 人·月** |
| **MinIO（历史 AGPL 前版本参考）** | AGPL v3 3.0+（⚠️ 不建议商业自研参照实现）| Go | 2014 → 2026（11 年）| ~120k LOC | **Erasure Coding 层 + bitrot protection** · **IAM STS 认证** · **桶管理/版本/生命周期** · **分布式纠删码 4~16 节点** | ❌ **禁用作技术底座**：AGPL v3 network copyleft 会将璇玑整套应用感染开源；因此仅借鉴「SPEC-2 已实现的 7 MPU 接口」已足够，**不再深入 MinIO 实现细节以免污染协议** |

#### 「璇玑云盘 · 自研分层参考最佳组合」（不是抄某一个）= **JuiceFS 的 Meta/Object 双抽象（L5）+ SeaweedFS 的 Master/Volume 拓扑控制（L4）+ 璇玑已有 L6/7 纠删码 + 缓存 + 审计链**

### 2.2 关系图 分布式图数据库 两大参照系 横向对比

| 项目 | 协议 | 语言 | 内核开始时间 | 核心代码量 | 核心模块（可直接借鉴分层）| 璇玑自研对标规模 |
|---|---|---|---|---|---|---|
| **NebulaGraph**（v3+v5 白皮书论文）| Apache 2.0 | C++ 内核（Storage 层 RocksDB）+ Java/Go 客户端 | 2018 Nov → 2026（8 年）+ 2021.11~至今 v5.0 GQL 原生支持 3 年专项 | ~180k LOC C++（core）| **Meta Service（Raft 3 副本 · schema / 权限 / 心跳 / 主选举）** · **Storage Service（3 层：Storage API / Raft / RocksDB KV）** · **Graph Service（无状态 · nGQL / openCypher / GQL 语言层 + Optimizer）** · **图分区（VID hash / 分片路由）** · **Raft 协议强一致 / WAL / 快照备份 / Full+Inc BR** · **Flink CDC + Spark Connector（大数据生态）** · **Algorithm（社区发现 / 中心性 / 路径 20+ 算法库 Plato）** | 璇玑现有 7 算法 = Nebula Algorithm 的核心子集；要达到 Nebula 级分布式存储/计算分离，**自研规模 48~72 人·月**（L7 RocksDB/分片/Raft 3 大件最耗时）|
| **Neo4j Community v4.x（GPLv3 仅单机）** | GPLv3 / 企业闭源 | Java + 原生图引擎 | 2007 → 2026（20 年）| 无法估算（社区版无分布式）| Cypher 语言 / ACID 事务 / PageRank / GDS 算法库 500+ 函数 | ❌ **信创不兼容 + 社区版无分布式**；仅借鉴 **Cypher → nGQL/GQL 的语法桥接层**（璇玑 workflow 可做翻译器薄壳）|

#### 「璇玑关系图 · 自研分层参考最佳组合」 = **NebulaGraph Storage/Graph/Meta 三组件分层（L7）+ 璇玑现有 Rust 7 算法（L4 singleSource）+ Nebula Adapter CDC/L1（L7）**

---

## 三、专家联盟 自研规模估算（基于「璇玑已有基线」× 参考项目规模 × AIS 7 层开发规范）

> 注：下列估算基于 **璇玑现有 SPEC-1~SPEC-v4 GREEN 基础（≈ 已完成 45% 基础能力）**；从零重写约 3× 时间；团队组成建议：6 人开发（Rust 3 名 · Node 2 名 · 架构/DBA/SRE 各 1 名轮转）+ 1 名独立 review

### 3.1 云盘模块：**全自研达到企业生产可用（GA）= 24 人·月（日历 6 个月）**

| 里程碑 | 日历周期 | 人·月 | 交付核心（对齐 AIS 7 层 + 参照最佳项目组件）| 验收清单 |
|---|---|---|---|---|
| **M0 基础骨架** | 第 1~3 周 | 2.5 | ① L5 新建 `mox-domain-abstractions`：`ObjectStorageProvider` / `MetaStorageProvider` / `ChunkManagerProvider` 三大 trait（JuiceFS Meta/Object 双抽象架构）；② 新建 Rust crate `mox-cloud-drive` L4；③ L6 kernel.rs 新建纠删码 Reed-Solomon 纯运算 + CRC32C/Adler32 纯校验 | SPEC TDD：三大 trait + 3 个 mock provider 各 5 GREEN；纠删码 RS(4,2) 恢复测试 5 GREEN |
| **M1 Master-Volume 拓扑层**（SeaweedFS 核心分层）| 第 4~9 周 | 5.0 | ① Master（卷分配 · 心跳 · 副本因子 N=2/3）；② Volume Server（条带写入/读取 · 256MiB 卷文件 · 快照/恢复 · 心跳上报）；③ 磁盘 O(1) 寻址：FileID → VolumeID + Offset（SeaweedFS 机制，避免小文件 inode 爆炸）| Master 单节点 1k QPS；Volume 3 副本写后读一致性；10k 小文件写后读取 100% 正确 |
| **M2 S3 兼容层 + MPU 强化** | 第 10~15 周 | 4.5 | ① Rust S3 Service 104 API（80% 最常⽤ 30 个：Put/Get/Delete/MultipartUpload/ListBuckets/Versions/Tagging/Policy）；② 多版本 + 桶生命周期（30 天过期）+ WORM；③ 服务端加密（SSE-C，企业增强） | s5cmd 或 mc 工具冒烟；5 GB 大文件 MPU ≥1.2GB/s（4 并发）；4 节点 kill-2 后读 100%（EC RS(6,3)）|
| **M3 POSIX Filer 层**（JuiceFS/Filer 组件） | 第 16~21 周 | 5.0 | ① Filer POSIX：mkdir / symlink / rename / chmod / xattr；② Meta 后端 3 种适配器：SQLite（dev）/ Postgres+Citus（prod）/ Redis（cluster）；③ 客户端 libfuse（Linux）+ Dokan（Windows） | POSIX 测试套件 fio 100%（随机读写 4k / 顺序读 1M）；目录项 1M 不崩（两级哈希前缀）；元数据操作 P99 ≤ 20 ms |
| **M4 分层 + 安全 + 管控** | 第 22~25 周 | 3.5 | ① 热/温/冷分层（MinIO/S3 存储 classes 映射，JuiceFS 1.4 tiering 思路）；② HMAC 签名审计（不可篡改链，璇玑已有）+ Bucket IAM + STS AssumeRole；③ 目录 Quota + 用户级 Quota + 容量配额 | 冷迁移 1TB 不中断服务；IAM 策略 100 条规则引擎；Quota 超限写请求被拒；审计 hash 链验证通过 |
| **M5 GA 验收 & 灾备** | 第 26 周 + 额外 2 周 buffer | 3.5 | ① 3 AZ 部署演练 + DR（RPO=0，RTO<60s，SPEC-13 基线）；② 全量回归：10 亿对象稳定性；③ 企业文档：运维手册 / Helm 一键安装（oss / enterprise 两 tier）/ SLO 仪表板 | 10 亿对象 7 天压测零丢数据；Helm 一键部署 ≤ 25 分钟；SLO p99 ≥ 99.9% |

**云盘 合计**：**24 人·月**（日历 26~28 周 ≈ **6~7 个月**）；团队规模建议：6 人（Rust 3 / Node 2 / SRE+运维 1）+ 独立 review 1 名（轮转）

### 3.2 关系图模块：**全自研达到企业生产可用（GA）= 36 人·月（日历 9 个月）**

| 里程碑 | 日历 | 人·月 | 交付核心（NebulaGraph 三组件架构 × 璇玑现有 Rust 算法） | 验收清单 |
|---|---|---|---|---|
| **R0 基础骨架 + 数据模型统一** | 1~4 周 | 3.0 | ① L5 `GraphProviderTrait` + `MetaProviderTrait` + `AlgorithmProviderTrait`；② 节点/边统一 Schema：VID 64 位（Nebula 做法：UUID hash → 64bit）/ Tag 类型 / EdgeType；③ L6 纯算法：Raft log 纯结构 + WAL 编码（与 serde 分开） | 三大 trait mock 各 5 GREEN；Schema + 类型系统 8 GREEN（CRUD 每种 edge type / tag） |
| **R1 Meta Service（Raft 副本）** | 5~12 周 | 6.0 | ① Meta 三节点 Raft（Leader/Follower/Listener 角色，Nebula Meta）；② Schema 管理（Tag/EdgeType/索引）；③ 权限 + 分区路由；④ 心跳；⑤ 快照备份/恢复 | 三节点 Raft kill-1 自恢复 ≤ 5s；Schema 变更不中断服务；100 Schema CRUD 全 pass |
| **R2 Storage Service（RocksDB + 分片）** 最耗时部分 | 13~26 周（**3 个月**）| 12.0 | ① KV 存储引擎：先 Rust RocksDB binding（rocksdb crate），后续可扩展 Citus/Postgres 作为 KV；② 分片路由：VID hash（Nebula 做法）；③ Raft 一致性（每个分片独立 Raft）；④ Storage API：getNeighbors / lookup / insert / delete / update；⑤ CDC 到上层 | 10 亿边写入 P95 写 ≤ 2ms（单机）；分片自动平衡 16→32 节点；CDC 事件 exactly-once；读端一致性 |
| **R3 Graph Service（无状态 Query Engine + Optimizer）** | 27~33 周 | 6.0 | ① nGQL 最小子集（60 条常用语句）+ openCypher 兼容；② Query Optimizer：索引命中 / 剪枝 / 并行扫描；③ 现有 Rust 7 算法接入 Graph Service；④ 工作流 step 写回（SPEC-v4 T13） | 2 跳查询 100k 节点 子图 ≤ 200ms；3 跳 ≤ 5s（Nebula 基线）；Cypher 用户上手 0 门槛（转换器）|
| **R4 生态集成 + 图投影（v5 企业级特性）** | 34~37 周 | 4.5 | ① Flink CDC 连接器（数据导入流式同步）；② Spark Connector（离线批量）；③ 图投影（Graph Projection 子图隔离分析，Nebula v5.0）；④ AI RAG 接入（璇玑 SPEC-v4 wf-ai-rag-v1） | Flink 10k TPS 无丢数据；Spark 10 亿边批量导入 ≤ 30 min；子图分析 100 条查询内存安全 |
| **R5 GA 验收 + 信创 + HA + 文档** | 38~39 周 + 3 周 buffer | 4.5 | ① 信创兼容：鲲鹏 + 统信 OS（源码编译 + 回归）；② 3 AZ HA + 备份/恢复；③ 运维手册 + 中文 800 页文档（NebulaGraph 手册对标）；④ SPEC-4 全量 70/70 对账不变 + 性能 ≥ Rust 单源基线 | 信创物理机回归 100%；灾难演练 RPO=0 RTO<60s；Helm 一键安装；中文手册齐备 |

**关系图 合计**：**36 人·月**（日历 39~42 周 ≈ **9~10 个月**）；团队规模建议：8 人（Rust 4 · KV/RocksDB+Raft 专家 1 · C++ 绑定 1 · Node/EAF 1 · DBA+SRE 1）

### 3.3 两模块 总估算（串行 vs 并行）& 推荐团队配置

| 交付形态 | 人·月（合计）| 日历（并行：6~8 人 × 同时攻坚两条线 M/R 交错） | 日历（串行） |
|---|---|---|---|
| **云盘 GA** + **关系图 GA**（双模块全量自研） | **60 人·月** | **10~12 个月（推荐，资源充分）** | 15 个月（不推荐） |
| 云盘 GA + 关系图 MVP（R3 前交付，R4/R5 后续迭代） | 24 + 21 = **45 人·月** | **8 个月** | 12 个月 |
| **璇玑现有基线复用 45%**，只扩「企业级缺失核心组件」（推荐路径：不用重写 M0/M1/M2 部分，SPEC-2 已有 MPU/版本；只需扩 M3 POSIX / R1-R3 分布式图三核心）| 14 + 22 = **36 人·月**（最省钱方案） | **7 个月（高性价比路线）** | 9 个月 |

---

## 四、设计文档（AIS 7 层 × 璇玑实现 模块矩阵 + 核心接口）

### 4.1 云盘模块 AIS 7 层架构

```
┌────────────────────────────────────────────────────────────────────┐
│ L2 GATEWAY RUST    /cloud_drive/*   (GET/PUT/DELETE/MPU + S3-compat) │
│                     AC-10 路由语义 · Sidecar 降级（3s）               │
├────────────────────────────────────────────────────────────────────┤
│ L3 L3 ORCH Node    workflow wf-file-upload-v1 (5 steps)              │
│                     wf-cloud-tier-migration（冷热迁移 + 审计）        │
├────────────────────────────────────────────────────────────────────┤
│ L4 RUST CRATE mox-cloud-drive  ───────────────────────────────────┤
│   MasterService / VolumeService / FilerService / S3Service /         │
│   TieringService / QuotaService / IamStsService                     │
├────────────────────────────────────────────────────────────────────┤
│ L5 DOMAIN TRAITS（L5 DIP）  ── 与实现完全解耦 ───────────────────────┤
│   ObjectStorageProvider · MetaStorageProvider · ChunkManagerProvider │
│   QuotaProvider            · IamPolicyProvider                        │
├────────────────────────────────────────────────────────────────────┤
│ L6 KERNEL（纯 std）       ── 零外部 crate ───────────────────────────┤
│   reed_solomon_encode/decode  ·  lru_std_impl ·  sha256_chunk_id     │
│   raft_log_binary_encode      ·  crc32c_watermark                   │
├────────────────────────────────────────────────────────────────────┤
│ L7 INFRA（mox-system 单 crate 独有 rusqlite / remote）           │
│   FSChunkBackend · S3ChunkBackend（SPEC-2 已有） · RocksDBVolumeStore │
│   PostgresMetaStore · RedisClusterMetaStore                         │
└────────────────────────────────────────────────────────────────────┘
```

### 4.2 关系图模块 AIS 7 层架构

```
┌────────────────────────────────────────────────────────────────────┐
│ L2 GATEWAY RUST   /graph/*  ·  /atlas/verify  ·  /workflow/execute  │
│                   AC-10 语义 · Sidecar 3s 降级                       │
├────────────────────────────────────────────────────────────────────┤
│ L3 Node EAF       wf-graph-bulk-v1 · wf-ai-rag-v1 · rerank + CEM   │
│                   每 step → workflow_step node + runs_on edge         │
├────────────────────────────────────────────────────────────────────┤
│ L4 RUST CRATES  (已有 graph-algorithms 7 algo 单源 + 新增三 crate)  │
│   mox-graph-meta（R1） ·  mox-graph-storage（R2 RocksDB+Raft）│
│   mox-graph-engine（R3 nGQL + Optimizer）                        │
├────────────────────────────────────────────────────────────────────┤
│ L5 DOMAIN 抽象 ── 永远先写 trait 再 impl ────────────────────────────┤
│   GraphQueryProvider  ·  MetaKvProvider  ·  AlgorithmSingleProvider │
│   PartitionRouterProvider  ·  CdcPublisherProvider                   │
├────────────────────────────────────────────────────────────────────┤
│ L6 KERNEL 纯 std      现有 operator-core/kernel.rs 扩展             │
│   7 算法纯结构 impl（operator-core） ·  raft_entry_encode / decode   │
│   vid_hash_std      ·  wal_checksum                                  │
├────────────────────────────────────────────────────────────────────┤
│ L7 INFRA  Mox-system ONLY：                                      │
│   RemoteGraphDriver(Gremlin/Nebula) ·  NebulaAdapter CDC L1 Cache    │
│   RocksDbStore(Nebula R2 参照) ·  PostgresCitusMeta + RedisCache    │
└────────────────────────────────────────────────────────────────────┘
```

### 4.3 核心接口定义（L5 traits 必须先实现，TDD RED→GREEN 强制）

#### 云盘 L5 `ObjectStorageProvider`（JuiceFS pkg/object/interface.go 思想 + 璇玑 IChunkBackend 扩展）

```rust
// mox-domain-abstractions/src/object_storage.rs
#[async_trait]
pub trait ObjectStorageProvider: Send + Sync {
    async fn put(&self, key: &str, data: Bytes) -> Result<()>;
    async fn get(&self, key: &str, range: Option<(u64, u64)>) -> Result<Bytes>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: &str, continuation: Option<&str>, limit: u32) -> Result<ListResult>;
    // MPU
    async fn create_multipart_upload(&self, key: &str) -> Result<String>; // upload_id
    async fn upload_part(&self, key: &str, uid: &str, pn: u16, part: Bytes) -> Result<PartETag>;
    async fn complete_multipart_upload(&self, key: &str, uid: &str, parts: Vec<PartETag>) -> Result<()>;
    async fn abort_multipart_upload(&self, key: &str, uid: &str) -> Result<()>;
    // 企业
    async fn put_object_tagging(&self, key: &str, tags: BTreeMap<String,String>) -> Result<()>;
    async fn head(&self, key: &str) -> Result<ObjectHead>; // last-modified + ETag + size
}
```

#### 关系图 L5 `GraphQueryProvider`（Nebula GraphService 对外接口 最小子集）

```rust
// mox-domain-abstractions/src/graph_query.rs
#[async_trait]
pub trait GraphQueryProvider: Send + Sync {
    async fn insert_vertex(&self, space: &str, vid: i64, tags: Vec<TaggedProps>) -> Result<()>;
    async fn insert_edge(&self, space: &str, src: i64, dst: i64, etype: &str, rank: i64, props: Props) -> Result<()>;
    async fn lookup(&self, space: &str, vids: &[i64]) -> Result<Vec<Vertex>>;
    async fn get_neighbors(&self, space: &str, vids: &[i64], edge_types: &[&str], direction: Direction) -> Result<Vec<Subgraph>>;
    async fn go(&self, space: &str, steps: u8, src: i64, edge_type: &str, direction: Direction) -> Result<Vec<Path>>;
    async fn query_ngql(&self, space: &str, q: &str) -> Result<ResultSet>;
    // CDC 事件订阅
    async fn subscribe_cdc(&self, topics: &[&str]) -> Result<tokio::sync::broadcast::Receiver<CdcEvent>>;
}
```

---

## 五、Files and Modules to Create/Modify（实施文件清单）

### 5.1 云盘模块（M0~M5 路径列表）

| 类别 | 路径 | 动作 |
|---|---|---|
| Workspace Cargo 成员 | `Cargo.toml` | 追加 `platform/services/mox-cloud-drive` / `mox-domain-abstractions` |
| L5 Trait 定义（关键 DIP）| `platform/services/mox-domain-abstractions/src/{object_storage,meta_storage,chunk_manager,quota,iam}.rs` | **新建**（JuiceFS 两大接口 + SeaweedFS Quota/IAM）|
| L6 Kernel（纯 std）| `platform/services/operator-core/src/kernel.rs` 追加模块 ReedSolomon / Crc / Lru / RaftEntry | **修改**（不得 use serde 等外部 crate）|
| L6 Wrapper | `operator-core/src/kernel_ext.rs` | **修改**（serde/nalgebra wrap，原实现已有）|
| L4 业务 crate | `platform/services/mox-cloud-drive/src/{master,volume,filer,s3,tier,quota,iam}.rs` | **新建**（6 个子模块对齐 SeaweedFS/JuiceFS）|
| L2 Gateway Rust 路由 | `gateway/runtime/src/routes/cloud_drive.rs` + handlers 模块 | **新建**（S3 兼容 30 API 最常用）|
| Node 编排 | `backend-node/src/workflow-engine.js` 追加 wf-cloud-tier-migration | **修改** |
| 测试 | `services/mox-cloud-drive/tests/{m0_traits_red_green, m1_master_volume, m2_s3_mpu, m3_posix_filer, m4_tier_iam, m5_ha_recovery}.rs` | **新建 6 个套件**（TDD 每 milestone 一套 ≥ 10 GREEN）|

### 5.2 关系图模块（R0~R5 路径列表）

| 类别 | 路径 | 动作 |
|---|---|---|
| Workspace Cargo | `Cargo.toml` | 追加 `mox-graph-meta` / `mox-graph-storage` / `mox-graph-engine`（3 新 crate） + `rocksdb` workspace dep（Apache 2.0）|
| L5 图抽象 | `mox-domain-abstractions/src/{graph_query, graph_meta, graph_algo, partition, cdc}.rs` | **新建**（Nebula 三大服务分层接口）|
| L4 Crate-Meta | 3 新 crate src/lib.rs 三常量（CRATE_ID/ENGINE/META） | **新建**（T2 规范）|
| L6 Kernel | operator-core/kernel.rs 新增：raft_encode / vid_hash_std / wal_checksum | **修改**（纯 std 无 extern）|
| L7 RocksDB impl | mox-graph-storage/src/kv_rocksdb.rs | **新建**（C binding rocksdb crate 安全封装）|
| Node 编排 | routes/workflow.js 图分片路由；routes/graph.js 追加 /graph/partition/balance | **修改**（S4 写后 Raft 校验自动幂等重试，SPEC-2 基线）|
| 测试 | 6 套集成测试（r0_schema / r1_meta_raft / r2_storage_billion / r3_engine_ngql / r4_cdc_flink / r5_xinchuang_ha） | **新建** 每 ≥ 10 GREEN |

---

## 六、Implementation Steps（依赖顺序，保证不会写坏现有 SPEC GREEN）

### 阶段 A：云盘（M0→M5）

1. **M0**：先新建 `mox-domain-abstractions` crate → 5 大 trait + mock 实现 → 写测试 30 GREEN → 更新 SPEC 三注册表（T1 方法）→ README 8 节齐（T11 规范）
2. **M1**：实现 `mox-cloud-drive/src/{master,volume}.rs`（Master/Volume 拓扑）→ 依赖注入 L5 trait；RocksDB（或本地 LMDB）存储卷元数据 → 10 GREEN
3. **M2**：S3 Service 30 API 最常用（80% 覆盖）→ Gateway 路由注册 → mc/s5cmd 冒烟测试 → ≥ 20 GREEN
4. **M3**：Filer POSIX + 3 Meta 后端（SQLite/Postgres/Redis）→ fio 套件冒烟；≥ 25 GREEN
5. **M4**：Quota/IAM/STS + 分层冷热迁移；审计 hash_chain 已存在，此处加调用
6. **M5**：Helm chart + HA 演练；文档齐备；**每一步严格 TDD：先写失败测试**

### 阶段 B：关系图（R0→R5）

1. **R0**：L5 5 大 trait（graph_query / graph_meta / graph_algo / partition / cdc）→ mock 25 GREEN；三常量 + README；Schema 类型系统 8 GREEN
2. **R1**：Meta Service 三节点 Raft（`raft` crate async-raft 最成熟）；Schema 管理；≥ 20 GREEN
3. **R2**（**最长 3 个月，最关键**）：Storage Service（RocksDB KV + 分片路由 + Raft + 5 Storage API）；最需要 RocksDB+Raft 资深工程师；≥ 40 GREEN
4. **R3**：Graph Service（nGQL parser subset + Optimizer + 7 Rust 算法接入）；≥ 30 GREEN
5. **R4**：Flink CDC 连接器 + Spark Connector（Java/Scala 组件，可外包/社区共建）；图投影实现；≥ 15 GREEN
6. **R5**：信创物理机回归 + 中文 800 页文档 + 灾难演练；Helm 齐备

### 并发规则（避免共享状态破坏）

- M0/R0 同时起 + M1/R1 同时起：独立 crate 无文件冲突
- M2/R2：R2 时间最长（3 月），M2 在 R2 第 2 月并行启动
- M3/R3：第 7~10 月双轨并发

---

## 七、Dependencies & Considerations

1. **Rust 生态依赖约束（版本统一 workspace 继承，T4 基线已 GREEN）**：
   - `rocksdb` 0.22.x（Apache 2.0）仅在 mox-graph-storage（L7）引用；禁止进入 L4/L5/L6
   - `async-raft` / `raft` 仅 Meta/Storage 服务端
   - `libfuse` / `fuser` 仅云盘 Filer 客户端；不得进 Kernel
   - **不得新增 AGPL 依赖**：避免 MinIO/Neo4j Community 协议感染
2. **精度护栏 / 路由护栏 红线锁死**：
   - PPR d=0.85 / maxIter=30；CNM 模块化度 Newman 公式；RAW 双向展开；Density 无 toFixed；LPA 公开出口禁用
   - Router AC-10 路由语义：S3 静态路由优先、参数少优先、同参数长路径优先
3. **璇玑已有基线 100% 兼容**：
   - `NebulaAdapter`、`IChunkBackend`、`workflow-engine.js`、`audit hash_chain` 全部向后兼容；老 SPEC-1~SPEC-v4 GREEN 不能出现任何回归
4. **商业建议**：云盘/关系图自研的 **正确姿势是 45% 基线复用 + 55% 核心缺失组件补齐**，而不是从零重写 SeaweedFS/Nebula
5. **合规**：自研全部代码 → 自有版权 → 避免 AGPL/GPL 感染；审计 hash_chain 符合等保三级；信创物理机测试

---

## 八、Validation（每个里程碑 ≥ 10 GREEN，验收清单）

### 8.1 云盘里程碑验收

| 里程碑 | 单元测试数量 | 集成测试 | 性能验收 | 企业验收 |
|---|---|---|---|---|
| M0 | 30/30 | 3 crate 100% workspace build/clippy | trait mock 1 ms 以内返回 | 三注册表 + README 8 节齐 |
| M1 | 30/30 + | 3 节点 Master 主从选举 | 1k ops/s | 心跳 1 秒超时自动切换 |
| M2 | 20/20 + | mc s3 冒烟 100 命令 | 5 GB MPU ≥1.2 GB/s | 4 node kill-2 100% 可读 |
| M3 | 25/25 + | fio 4 场景 100% | 元数据 P99 ≤ 20 ms | POSIX 语义 pjd-fstest 通过率 ≥ 95% |
| M4 | 20/20 + | IAM 策略引擎 100 条 + Quota 拒绝 | 冷迁移 ≥ 1 TB/h | 审计 hash_chain 验证 |
| M5 | 30/30 + | 3 AZ + DR 演练 | SLO p99 ≥ 99.9% | Helm ≤ 25 分钟部署；10 亿对象稳定 |

### 8.2 关系图里程碑验收

| 里程碑 | 单测 | 集成 | 性能 | 企业 |
|---|---|---|---|---|
| R0 | 33/33 | Schema 100 CRUD | Schema ops ≤ 1 ms | 三注册 + README 8 节 |
| R1 | 30/30 + | 3 Meta 节点 Raft kill-1 恢复 ≤ 5 s | ops ≥ 3 k/s | 权限模型 10 级 |
| R2 | 40/40 + | 分片平衡 16→32 | 写 P95 ≤ 2 ms · 10 亿边 | CDC exactly-once |
| R3 | 30/30 + | 2 跳查询 100k 节点图 ≤ 200 ms · 3 跳 ≤5 s | 对比 Rust 7 算法性能 ≥ 1× | openCypher 用户 100% 兼容 |
| R4 | 15/15 + | Flink 10k TPS 无丢 · Spark 10 亿边导入 ≤ 30 min | 子图投影 ≥ 10x 内存节省 | AI RAG 准确率 ≥ 基线 |
| R5 | 45/45 + | 信创物理机回归 100% + 灾难 RPO=0 RTO<60s | SLO p99 ≥ 99.9% | Helm ≤ 30 分钟；中文手册 800 页齐备 |

---

## 九、Risks & Handling

| 风险等级 | 风险 | 规避策略 |
|---|---|---|
| 🔴 HIGH | R2 Storage Service（RocksDB + Raft + 分片）开发周期可能超 3 月（最常见延期原因）| ① 预研期 2 周 PoC 先做 RocksDB KV + Raft 最小闭环；② 招 RocksDB+Raft 资深工程师（1 名全职）；③ 无法按期则 「RocksDB 单机 + Postgres sharding 过渡方案」上线，后续补齐 Raft |
| 🔴 HIGH | 协议污染：错误引入 AGPL 组件（MinIO/Neo4j Community 直接依赖）| ① 建立 License 白名单 CI（cargo deny + license-scanner）；② 所有新 crate CI 自动跑 license 扫描；③ 违规=FAIL 阻断 |
| 🟡 MEDIUM | 云盘 POSIX 语义（M3）性能瓶颈：小文件 inode 爆炸 | ① 借鉴 SeaweedFS「两级目录散列」+「O(1) FileID → Offset」；② 小文件 < 64 KiB 直接 inline 到 meta 后端（JuiceFS 思路）；③ 合并写（compaction）定时任务 |
| 🟡 MEDIUM | 关系图超热点节点（度分布倾斜热点邻居查询 O(N)）| ① SPEC 问题基线已有；② 实现热点切分：超级节点分桶 + 缓存邻居表；③ 度中心性结果 TTL 缓存（NebulaGraph 最佳实践）|
| 🟢 LOW | 小团队并行写同文件冲突 | ① 按 crate 拆分工单：每人只改自己负责 crate；② L5 trait 评审后冻结；③ 仅 1 人可改 operator-core/kernel.rs |
| 🟢 LOW | 文档与实现脱节（SPEC-v4 T10 四方对账经验）| 每次 PR 必须：① 测试 GREEN；② 写回 6 层图谱；③ /atlas/verify 报告对比 diff |

---

## 十、里程碑 视觉甘特图（推荐 10~12 个月 GA）

```
Month  1        2        3        4        5        6        7        8        9       10       11       12
云盘   [ M0  ][    M1   ][    M2   ][  M3   ][ M4][M5   ]              buffer
关系图 [ R0 ][      R1     ][              R2  3 个月最耗时              ][    R3   ][ R4 ][    R5   + buffer ]
```

**推荐团队配置（总 8 人，避免过大沟通成本）**：
- Rust 工程师 4（1 名 资深 RocksDB+Raft 领导 R2；1 名 网络+安全领导 M2/M4；2 名 实现 L4/L5）
- Node/EAF 2 人（领导 L3 编排 + 审计 + workflow 扩展）
- 架构 + SRE + 独立 review 1 人（轮转 + 每周独立 Review）
- 项目管理 / 文档 1 人（中文手册齐备 + 信创对接）

---

## 十一、后续 Approve Gate（Plan Mode 规范要求）

本计划为 **开发专家联盟级分析 + 实施计划**，覆盖用户请求的三部分：
1. ✅ 「要多久？」：云盘 24 人·月（6 月）· 关系图 36 人·月（9 月）· 复用基线后 36 人·月 / 7 月；全并行 60 人·月 / 10~12 月；
2. ✅ 「参考 AIS 架构 + 最好开发项目代码架构」：JuiceFS Meta/Object 双抽象（L5）+ SeaweedFS Master/Volume（L4）+ NebulaGraph 三服务（Meta/Storage/Graph）+ 璇玑 7 层 AIS 完整对齐；
3. ✅ 「设计文档」：模块矩阵 / 接口定义 / 架构图 / 验证 / 风险规避 / 甘特图 / 团队配置 齐备；

下一步：请用户审阅本计划 → **Approve** 后，按 Plan Mode §六 实施步骤逐个里程碑 TDD RED→GREEN 开工（无需再创建独立 Spec，除非用户要求独立 Review gate 才升级 Spec Mode）。
