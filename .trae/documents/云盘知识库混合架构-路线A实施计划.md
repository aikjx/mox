# 云盘知识库混合架构（路线A）实施计划

## Context（背景与目标）

当前云盘知识库存在三处核心问题：
1. **存储抽象只有内存实现**：`mox-base-store-core` 的 `ObjectStore`/`KvStore` 契约已定义，但唯一实现是 `InMemoryObjectStore`（`simple_hash` 占位，非真 SHA-256），无法持久化。
2. **S3 协议层悬空**：`mox-cloud-s3-svc` 的 `AppState.storage` 是 `InMemoryStorage`，MPU 分片缓存在内存 `BTreeMap`，生产不可用。
3. **知识库 API 是占位**：legacy `backend-rust` 的 `/kb/*` handler（`kb_document_analyze` 空、`kb_search` 空、`kb_stats` 返回 0）无真实功能。

用户决策（路线A 混合架构）：**保留自研为主体，参考 `ais/RustFS` 的优秀架构算法**（Reed-Solomon 纠删码、Bitrot 检测+自愈、对象版本/快照、对象缓存），整合云盘知识库所有功能；以**专家联盟方法论**（多领域专家协同评审/设计/开发/验证）保证架构质量；**模块化**；**测试分析验证**。

**关键修正（已核实）**：
- 自研已有完整 RS 纠删码引擎：`platform/domains/cloud/svc/mox-cloud-volume-svc/src/`（`reed_solomon.rs` Vandermonde+GF(2^8)、`erasure_coding_ext.rs` Cauchy/渐进重建、`manifest.rs` EcManifest、`fs_layout.rs` 两级散列布局、`rebuild.rs`）。**阶段3 复用该引擎，不重写**；RustFS `bridge.rs` 仅作"降级读一致性"语义参考。
- 真实构建路径是根 workspace `platform/`；`mox-workspace/` 是骨架 scaffold，不作为落点。
- 已存在 4 套重叠存储抽象，本方案**以 `mox-base-store-core` 为唯一物理口契约**收敛，不新建第 5 套。
- 已有自研 S3 客户端可复用：`platform/foundation/mox-audit/src/s3.rs`（手写 SigV4，与 MinIO/COS/OBS/OSS 互通）。**不引入 `aws-sdk-s3`**。

## 目标架构

```
L1 基础层   platform/foundation + domains/base
            mox-base-store-core (ObjectStore/KvStore/ObjectStreamWriter 唯一契约)
            mox-cloud-foundation (ObjectStorageProvider/QuotaProvider/PartETag)
L2 算法内核 platform/domains/cloud/core
            mox-cloud-store-core【新增】FS/S3后端+去重+GC+版本+bitrot+heal+缓存
            mox-cloud-volume-svc  (复用 EC 引擎)
L3 业务服务 platform/domains/cloud/svc + kg/svc
            mox-cloud-store-svc【新增】装配 FILE_BACKEND + 管理面 API
            mox-cloud-s3-svc / mox-cloud-filer-svc (改造：真实后端)
            mox-kb-svc【新增】知识库文档 + 图谱挂图 + 检索
L4 应用编排 platform/domains/cloud/api + kg/api
            mox-cloud-api (VolumeManager/BucketManager/QuotaManager + StorageAdmin【新增】)
L5 网关层   platform/gateway/mox-platform-gateway-svc/routes.rs
L6 接入层   legacy backend-rust (/kb/* 转发) + frontend-ui
```

**依赖方向**（无环）：`mox-cloud-store-core → base-store-core + cloud-foundation + volume-svc + mox-audit`；协议层（s3-svc/filer-svc）→ store-core 门面类型；`mox-kb-svc → store-core + kg-storage-svc + ai-expert-svc`。

## 分阶段实施（每阶段可编译可测试）

### 阶段1：存储抽象真实化
**新建** `platform/domains/cloud/core/mox-cloud-store-core/`：
- `fs_backend.rs`：`FsObjectStore`（实现 `ObjectStore`+`KvStore`+`ObjectStreamWriter`）、`AtomicFile`（tmp+rename 原子写）
- `kv_backend.rs`：`FsKvStore`（原子 JSON 落盘）
- `stream_writer.rs`：`FsStreamWriter`（分块滚动哈希，供 MPU 落盘复用）
- `dedup.rs`：`ChunkRefManager`（内容寻址去重，`chunks/<xx>/<sha256>` 两级散列）
- `gc.rs`：`GarbageCollector`（引用计数 GC，支持 `dry_run`，30 天 grace）
- `versioning.rs`：`VersionManager`（`versions/<fileId>/vN.json` 零拷贝恢复）
- `backend.rs`：`create_backend(&StoreConfig)`，先只支持 `Fs`

真 SHA-256（`sha2`）替代 `simple_hash` 占位。**测试**：put/get/range/delete/head/exists 往返、原子写崩溃安全、去重 refcount、GC dry-run vs 实跑、KvStore 重启持久化；集成 `tests/t1_fs_backend_lifecycle.rs`（tempfile，Windows 兼容，无 mmap/POSIX-only API）。

### 阶段2：可插拔后端 + S3 协议层
- store-core 新增 `s3_backend.rs`（`S3ObjectStore`+`S3Client`，复用 `mox-audit::s3::S3Sink` 签名扩展完整 SigV4 客户端）、`fallback.rs`（`FallbackObjectStore`：Head 404→读 FS→回填，实现"三大铁律"目标空读自动回源）、`backend.rs` 支持 `FILE_BACKEND ∈ {fs|s3|minio|oss}`。
- 改造 `mox-cloud-s3-svc`：`AppState.storage` 换 `Arc<dyn ObjectStore>`；新增 `BucketLayer`（桶元数据走 KvStore）；`MultipartManager` 分片改磁盘句柄（复用 `FsStreamWriter`），保留 CRC32C+`etag_multipart`；**`InMemoryStorage` 保留为测试双**（现有测试套件保持绿）。
- 改造 `mox-cloud-filer-svc`：新增 `StoreCoreObjectStorage` 桥接。
- 新增 `mox-cloud-api::StorageAdmin` trait（status/switch/migrate/verify/gc/stats）。
**测试**：`S3ObjectStore` 对测试内联 MockS3Server（canned XML，不依赖外部 MinIO）；key 同构断言（FS 路径 = S3 key 逐字一致）；HTTP 集成 Put/Get/List/MPU；`MOX_S3_INTEGRATION=1` 时连真实 MinIO（docker-compose 已有）。

### 阶段3：企业级算法（复用 volume-svc 引擎）
- store-core 新增 `erasure.rs`（`ErasureStore` 装饰器，复用 `ReedSolomonEngine`/`EcManifest`/`fs_layout`）、`bitrot.rs`（`BitrotDetector`，分片 crc32c + 限速后台扫描）、`heal.rs`（`HealCoordinator`，复用 `ProgressiveRebuilder`）、`snapshot.rs`、`cache.rs`（加权 LRU + singleflight，自研不引 moka）。
- **范围控制**：默认持久化 = **N 副本 + 分片 checksum**；完整 RS-EC 由 `ERASURE_CODING=rs` + `DATA_SHARDS/PARITY_SHARDS` 配置开启。EC 作为 `ObjectStore` 装饰器，后端不可知，可随时叠加。
**测试**：EC 编解码往返（4+2）、丢片恢复矩阵（丢 1..=parity 片重建逐字节一致）、腐坏片检测、缓存命中/驱逐、快照创建/恢复；`tests/t2_erasure_engine_matrix.rs`；属性测试（0/1/部分/整块/大块载荷）。

### 阶段4：知识库业务整合
**新建** `platform/domains/kg/svc/mox-kb-svc/`：`document.rs`（`KbDocumentService` CRUD+版本）、`analyze.rs`（`KbAnalyzer` 调专家联盟，替换占位 `kb_document_analyze`）、`link.rs`（`GraphLinker` 建 Document/Chunk/Entity/Relation 节点边 → `mox-kg-storage-svc`）、`search.rs`、`version.rs`、`handlers.rs`（axum，对齐 legacy `/kb/*` API 面）。
- 改造 legacy `backend-rust/src/api/handlers.rs` L1491-1529 为**转发**到 `mox-kb-svc`（路由前缀不变，前端零改动）。
- 网关 `routes.rs` 按 `platform_config.json` 注册新服务路由；根 `Cargo.toml` members 追加 3 个新 crate。
**测试**：文档 CRUD+版本、挂图节点/边正确性、检索；`tests/t3_kb_graph_link.rs`；网关 HTTP 全链路 E2E + 前端冒烟。

## 专家联盟方法论落地

- 每阶段设**评审门**：`mox-cloud-store-svc/src/expert_gate.rs` 的 `ExpertGate::run_stage_gate(stage)`，调用 `mox_ai_expert_svc::expert_traits::llm_consultant()`（有 `MOX_LLM_API_KEY` 走真实 LLM，否则本地引擎），输出结构化评审到 `.runtime/`（JSON）。
- **评审矩阵**：
  - 设计评审（编码前）：architecture（crate 边界/依赖方向/抽象收敛）+ algorithm（EC/去重/GC 正确性）
  - 代码评审：code_quality（clippy 门禁）+ security_code（SigV4、凭据注入、**路径穿越防护**——FS key 拼接防 `../`）+ maintainability
  - 验证评审：testing（测试矩阵）+ performance（criterion bench）+ observability（`/storage/status` 指标）
  - 业务评审：business（KB 语义）+ permission（文档 ACL/多租户）+ data（跨后端一致性/三大铁律）
- **争议决策走辩论**：分歧点调用 `alliance/debate.rs` 并行咨询+归一合成，落 `gate.rs` 质量门禁后推进。

## 验证方案

- **编译**：`cargo build -p mox-cloud-store-core -p mox-cloud-store-svc -p mox-cloud-s3-svc -p mox-cloud-filer-svc -p mox-kb-svc`；`cargo clippy --workspace --all-targets -- -D warnings`（对齐门禁）
- **测试**：`cargo test -p <上述 crate>`；回归 `cargo test --workspace`（保持 s3-svc 现有套件绿）
- **基准**：store-core `benches/`（FS 吞吐、EC 编解码、去重命中率、GC 扫描）
- **S3 互操作**：docker-compose 起 MinIO → `FILE_BACKEND=s3` → key 同构断言
- **混合载荷烟测**（规范 §3.6）：10% 空 / 30% <4KB / 50% 2–50MB / 10% 100MB+，重复 3 份，并发 harness，断言 `hashMismatch=0` 且去重率吻合
- **KB E2E**：`POST /kb/documents` → analyze → `GET /kb/documents/:id/graph-link` → 图谱验证 + 前端冒烟

## 风险与边界

1. **EC 复杂度控制**：默认 checksum+N 副本，RS-EC 配置化开启；复用 volume-svc 引擎不重写。
2. **bitrot 边界**：首版只做分片 crc32c + 后台限速扫描 + 渐进式重建，不做跨节点/跨 region（T2/T3 只留配置钩子）。
3. **legacy 边界**：`mox-workspace` scaffold 与 Node legacy **不改动、不作为落点**；legacy handlers 只转发。
4. **抽象收敛**：`mox-base-store-core` 为唯一物理口契约；不动 `mox-flow-unified-storage-core`。
5. **避免过度工程**：v1 不做分布式/raft/多盘池；单机持久 + 可插拔远端 S3 为 T0/T1 范围；kg 的 rocksdb/raft 属图存储域，不并入云盘存储。
6. **依赖**：不引 `aws-sdk-s3`（重）、不引 `moka`，S3 客户端与缓存均自研复用。
7. **并发与原子性**：原子写 tmp+rename；GC 与写竞争用逐 chunk 锁（参考 `mox-cloud-filer-svc::file_lock::FileLockManager`）；Windows 环境测试全走 tempfile，避免 POSIX-only API。
8. **向后兼容**：s3-svc `InMemoryStorage` 保留为测试双；`api/mod.rs` 路由前缀不变，前端零改动。

## 关键文件

- 契约基准：`platform/domains/base/mox-base-store-core/src/lib.rs`
- S3 层改造主战场：`platform/domains/cloud/svc/mox-cloud-s3-svc/src/s3_server.rs`、`src/mpu.rs`
- EC 引擎复用入口：`platform/domains/cloud/svc/mox-cloud-volume-svc/src/lib.rs`（`reed_solomon.rs`/`manifest.rs`/`fs_layout.rs`/`rebuild.rs`）
- 图谱挂图目标：`platform/domains/kg/svc/mox-kg-storage-svc/src/lib.rs`
- KB 转发改造：`platform/legacy/backend-rust/src/api/handlers.rs` L1491-1529、`src/api/mod.rs` L795-814
- S3 客户端复用：`platform/foundation/mox-audit/src/s3.rs`
- 布局/GC/切换规格：`deploy/docs/FS-S3-full-lifecycle-ops-guide.md`、`deploy/docs/MOX-Enterprise-Unified-Spec-v2.0.md`
- RustFS 参考：`ais/RustFS/crates/ecstore/src/erasure/codec/bridge.rs`（降级读一致性）、`crates/object-data-cache/src/cache.rs`（缓存语义）
