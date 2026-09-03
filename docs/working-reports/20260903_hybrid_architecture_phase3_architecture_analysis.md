# 混合架构（路线 A）阶段三 · 架构分析与融合设计文档

| 字段 | 值 |
|---|---|
| **文档标题** | 混合架构（路线 A）阶段三 · 架构分析与融合设计 |
| **版本** | v1.0 |
| **权威等级** | 🟢 ADR（架构决策记录） |
| **文档编号** | ADR-CLOUD-HYBRID-A-P3-20260903 |
| **日期** | 2026-09-03 |
| **关联文档** | `docs/working-reports/20260902_hybrid_architecture_route_a_design.md`（ADR-CLOUD-HYBRID-A-20260902） |
| **适用范围** | `platform/domains/cloud/` 全模块（5 个 svc crate + api + sdk） |
| **分析方法** | 源码级模块梳理 + 依赖图分析 + 耦合点识别 + RustFS 架构对标 + 端到端业务流程建模 |

---

## 1. 执行摘要

本阶段三在阶段一（绿色基线 1042 测试）和阶段二（核心算法吸收 1080 测试）基础上，执行**架构融合**：对自研云盘知识库做系统性架构分析，识别耦合点与边界不清处，设计更清晰的分层架构、模块边界、接口契约，并完成 4 项关键架构改造。

**当前架构概览**：5 个 svc crate（master/volume/s3/filer/rebalance）+ api + sdk，Master-Volume 控制面/数据面分离，S3 兼容门面，Filer POSIX 抽象，Rebalance 容量均衡。阶段二新增 multi_writer / hedged_reader / backpressure 三个算法模块。

**识别的 6 大主要问题**：
1. **缓冲管理分散**：volume/s3/filer 各自实现内存缓冲，无统一池化，存在重复分配和内存泄漏风险
2. **读路径不统一**：普通读与 hedged 读是两套独立实现，无法组合嵌套，能力探测缺失
3. **背压未接入主路径**：阶段二建好的 BackpressureMonitor 是独立模块，未接入 volume_server 写入入口
4. **扫描调度无预算**：lifecycle 扫描无时间/IO/容量三维预算，高负载下可能抢占前台 IO
5. **配置硬编码**：阈值、超时、并发数散落在各模块，无统一配置结构体，不可运行时调整
6. **可选后端无接入点**：RustFS ecstore/rio 作为可选 L7 后端仅有设计，无 feature flag 接入点

**阶段三完成的 4 项架构改造**：
- PooledBuffer 四层分档缓冲池（统一缓冲管理）
- CAS 背压信号量接入 volume_server 写入主路径
- ReaderCapability 组合式 reader 管线（统一读路径，trait 抽象可插拔）
- 三维扫描预算 + 全局配置结构体 + Feature Flags（灵活可扩展）

**新增测试 52 个**，全量回归保持绿色（volume-svc 125 lib + s3-svc 98 lib，集成测试无回归）。

---

## 2. 当前架构系统性梳理

### 2.1 crate 级模块清单

| Crate | 类型 | 核心职责 |
|---|---|---|
| mox-cloud-api | api | 公共数据结构、错误类型、trait 定义 |
| mox-cloud-sdk | sdk | 客户端 SDK，封装 gRPC/HTTP 调用 |
| mox-cloud-master-svc | svc | 控制面：卷管理、Raft 高可用、拓扑、快照、恢复计划 |
| mox-cloud-volume-svc | svc | 数据面：chunk 读写、纠删码、存储分层、缓存、仲裁 |
| mox-cloud-s3-svc | svc | S3 兼容门面：38+ S3 API、生命周期、复制、清单、ACL |
| mox-cloud-filer-svc | svc | POSIX 抽象：目录/文件、元数据三后端、配额、文件锁、快照 |
| mox-cloud-rebalance-svc | svc | 容量均衡：迁移计划、放置策略、执行调度 |

### 2.2 volume-svc 模块级清单（阶段三改造重点）

| 模块 | 职责 | 阶段 |
|---|---|---|
| lib.rs | 模块导出 | 基线 |
| volume_server.rs | VolumeServer 主服务，chunk 读写入口 | 基线 + 阶段三背压接入 |
| reed_solomon.rs | GF(2^8) 纠删码引擎，矩阵缓存，重建验证 | 基线 + 阶段二优化 |
| storage_tier.rs | 热/温/冷三层存储抽象，迁移策略 | 基线 |
| chunk_manager.rs | chunk 生命周期管理 | 基线 |
| metrics.rs | Prometheus 指标，含 P4 性能基准测试 | 基线 |
| error.rs | VolumeError 错误枚举 | 基线 + 阶段三 BackpressureRejected |
| config.rs | VolumeServiceConfig 全局配置 | **阶段三新增** |
| buffer_pool.rs | PooledBuffer 四层分档缓冲池 | **阶段三新增** |
| reader_capability.rs | ReaderCapability trait + ReaderPipeline | **阶段三新增** |
| multi_writer.rs | MultiWriter + WriteProgressPolicy 写仲裁 | 阶段二新增 |
| hedged_reader.rs | HedgedReader + locality 读仲裁 | 阶段二新增 |
| backpressure.rs | CAS 无锁背压信号量 + 三态状态机 | 阶段二新增 |

### 2.3 crate 级依赖图

```
mox-cloud-api (公共类型/trait)
    ↑
mox-cloud-sdk (客户端)
    ↑
┌─────────────────────────────────────────┐
│              mox-cloud-s3-svc            │
│  (S3 门面，依赖 volume + filer + master) │
└──────────────┬──────────────────────────┘
               │ 调用
┌──────────────▼──────────────────────────┐
│           mox-cloud-volume-svc            │
│  (数据面核心：纠删码/读写/分层/缓存/仲裁)  │
└──────────────┬──────────────────────────┘
               │ 元数据
┌──────────────▼──────────────────────────┐
│            mox-cloud-filer-svc            │
│  (POSIX 抽象：元数据三后端/配额/文件锁)    │
└─────────────────────────────────────────┘

mox-cloud-master-svc (控制面，独立 Raft 集群)
mox-cloud-rebalance-svc (容量均衡，独立调度)
```

### 2.4 耦合点识别（5 类）

| # | 耦合类型 | 位置 | 影响 | 改进方向 |
|---|---|---|---|---|
| C1 | **s3→volume 直接依赖** | s3-svc 直接调用 volume_server 方法 | S3 门面与数据面紧耦合，无法替换后端 | 引入 StorageBackend trait，s3 依赖 trait 而非具体类型 |
| C2 | **缓冲管理分散** | volume/s3/filer 各自 `Vec::with_capacity` | 重复分配、无上限、内存碎片 | 统一 PooledBuffer 池化（阶段三已完成） |
| C3 | **读路径双轨** | 普通 read_chunk 与 HedgedReader 独立 | 无法组合、能力无法探测、代码重复 | ReaderCapability trait 统一（阶段三已完成） |
| C4 | **配置散落** | 各模块硬编码阈值/超时 | 不可调、不可测试、运维困难 | 全局 Config 结构体 + from_env（阶段三已完成） |
| C5 | **算法与服务耦合** | reed_solomon/multi_writer 等在 volume-svc 内 | 算法无法独立测试/复用/替换 | 阶段四抽离到 mox-cloud-kernel crate |

### 2.5 冗余与边界不清（4 类）

| # | 问题 | 说明 |
|---|---|---|
| R1 | **元数据抽象重复** | filer 的 MetaStorage trait 与 volume 的 chunk 元数据管理有重叠 |
| R2 | **错误类型不统一** | 各 crate 独立定义 Error，转换链长且丢失上下文 |
| R3 | **并发控制模式不一** | 有的用 Mutex，有的用 RwLock，有的用原子变量，无统一规范 |
| R4 | **测试基础设施重复** | 各 crate 独立实现 mock 存储、测试辅助函数 |

---

## 3. 目标分层架构设计（L1-L6）

### 3.1 六层定义

| 层 | 名称 | 职责 | 对应模块 |
|---|---|---|---|
| **L1** | 协议门面层 | S3 API 解析、请求路由、ACL/鉴权 | s3-svc（handlers/） |
| **L2** | 业务编排层 | 生命周期、复制、清单、快照编排 | s3-svc（lifecycle/replication/）、master-svc |
| **L3** | 控制面层 | 卷管理、拓扑、Raft 一致性、恢复计划 | master-svc |
| **L4** | 数据面服务层 | VolumeServer、chunk 读写入口、背压接入 | volume-svc（volume_server.rs） |
| **L5** | 算法内核层 | 纠删码、读写仲裁、缓冲池、背压、扫描预算 | volume-svc（reed_solomon/multi_writer/hedged_reader/buffer_pool/backpressure/scanner） |
| **L6** | 存储后端层 | 本地盘、对象存储、可选 RustFS ecstore/rio | volume-svc（storage_tier/）+ feature flag 接入点 |

### 3.2 模块归属表

| 模块 | 目标层 | 当前位置 | 状态 |
|---|---|---|---|
| S3 handlers | L1 | s3-svc | 已就位 |
| Lifecycle/Replication | L2 | s3-svc | 已就位 + 阶段三扫描预算 |
| Master/Raft | L3 | master-svc | 已就位 |
| VolumeServer | L4 | volume-svc | 已就位 + 阶段三背压接入 |
| ReedSolomon | L5 | volume-svc | 已就位 + 阶段二优化 |
| MultiWriter | L5 | volume-svc | 阶段二新增 |
| HedgedReader | L5 | volume-svc | 阶段二新增 |
| Backpressure | L5 | volume-svc | 阶段二新增 + 阶段三接入 L4 |
| BufferPool | L5 | volume-svc | **阶段三新增** |
| ReaderCapability | L5 | volume-svc | **阶段三新增** |
| Scanner/ScanBudget | L5 | s3-svc | **阶段三新增** |
| Config | L5/L4 | s3-svc + volume-svc | **阶段三新增** |
| StorageBackend trait | L6 | 待定义 | 阶段四 |
| RustFS ecstore 接入点 | L6 | feature flag | 阶段三定义接入点（默认 false） |

### 3.3 核心接口契约（4 个）

#### 3.3.1 StorageBackend trait（L6，阶段四定义）

```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn put_chunk(&self, chunk_id: &ChunkId, data: &[u8]) -> Result<()>;
    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>>;
    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<()>;
    async fn list_chunks(&self, prefix: &str) -> Result<Vec<ChunkMeta>>;
    fn backend_type(&self) -> BackendType; // Local / S3 / RustFSEcstore
}
```

#### 3.3.2 ReaderCapability trait（L5，阶段三已实现）

```rust
#[async_trait]
pub trait ReaderCapability: Send + Sync {
    async fn read_shard(&self, shard_id: &ShardId, offset: u64, len: usize) -> Result<Vec<u8>>;
    fn read_cost(&self) -> ShardReadCost; // Local < SameNode < Remote < Unknown
    fn endpoint(&self) -> &str;
    fn supports_hedged_read(&self) -> bool { false }
    fn supports_zero_copy(&self) -> bool { false }
    fn read_timeout(&self) -> Duration { Duration::from_secs(30) }
}
```

#### 3.3.3 MetaStorage trait（L4，filer 已有，阶段四统一）

```rust
#[async_trait]
pub trait MetaStorage: Send + Sync {
    async fn get_attr(&self, path: &Path) -> Result<FileAttr>;
    async fn set_attr(&self, path: &Path, attr: FileAttr) -> Result<()>;
    async fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>>;
    async fn create(&self, path: &Path, kind: NodeKind) -> Result<()>;
    async fn remove(&self, path: &Path) -> Result<()>;
    async fn rename(&self, from: &Path, to: &Path) -> Result<()>;
}
```

#### 3.3.4 LifecycleEvaluator trait（L2，阶段三扫描预算集成）

```rust
pub trait LifecycleEvaluator: Send + Sync {
    fn should_transition(&self, obj: &ObjectMeta, budget: &ScanBudget) -> TransitionDecision;
    fn should_expire(&self, obj: &ObjectMeta) -> bool;
    fn next_scan_time(&self, last_scan: Instant, budget: &ScanBudget) -> Instant;
}
```

---

## 4. 算法与架构解耦分析

### 4.1 24 项算法清单

| # | 算法 | 所在模块 | 层级 | 解耦状态 |
|---|---|---|---|---|
| 1 | GF(2^8) 矩阵乘法 | reed_solomon | L5 | 已解耦（纯函数） |
| 2 | 矩阵缓存 LUT | reed_solomon | L5 | 阶段二优化为 RwLock<HashMap> |
| 3 | 纠删码编码 | reed_solomon | L5 | 已解耦 |
| 4 | 纠删码重建 | reed_solomon | L5 | 已解耦 |
| 5 | reconstruction verification | reed_solomon | L5 | 阶段二新增 fail-closed |
| 6 | SIMD 加速编码 | reed_solomon | L5 | 已解耦（feature flag） |
| 7 | MultiWriter 写仲裁 | multi_writer | L5 | 阶段二新增，已解耦 |
| 8 | WriteProgressPolicy | multi_writer | L5 | 阶段二新增，已解耦 |
| 9 | HedgedReader 读仲裁 | hedged_reader | L5 | 阶段二新增，已解耦 |
| 10 | ShardReadCost 排序 | hedged_reader | L5 | 阶段二新增，已解耦 |
| 11 | CAS 背压信号量 | backpressure | L5 | 阶段二新增，已解耦 |
| 12 | 三态状态机 | backpressure | L5 | 阶段二新增，已解耦 |
| 13 | BackpressurePermit RAII | backpressure | L5 | 阶段二新增，已解耦 |
| 14 | PooledBuffer 池化 | buffer_pool | L5 | **阶段三新增，已解耦** |
| 15 | 四层分档分配 | buffer_pool | L5 | **阶段三新增，已解耦** |
| 16 | ReaderCapability trait | reader_capability | L5 | **阶段三新增，已解耦** |
| 17 | ReaderPipeline 组合 | reader_capability | L5 | **阶段三新增，已解耦** |
| 18 | ScanBudget 三维预算 | scanner | L5 | **阶段三新增，已解耦** |
| 19 | 双令牌桶限流 | scanner | L5 | **阶段三新增，已解耦** |
| 20 | 存储分层迁移 | storage_tier | L4/L5 | 部分耦合（与 volume_server 耦合） |
| 21 | Raft 共识 | master-svc | L3 | 已解耦（独立 crate） |
| 22 | Rebalance 放置策略 | rebalance-svc | L3 | 已解耦（独立 crate） |
| 23 | 生命周期规则评估 | s3-svc | L2 | 部分耦合（与 handlers 耦合） |
| 24 | 配额管理 | filer-svc | L4 | 部分耦合（与 meta 耦合） |

### 4.2 解耦建议

| 优先级 | 建议 | 说明 |
|---|---|---|
| P0 | 阶段四创建 `mox-cloud-kernel` crate | 将 L5 算法（reed_solomon/multi_writer/hedged_reader/backpressure/buffer_pool/reader_capability/scanner）抽离到独立 crate，可独立测试/复用 |
| P0 | 阶段四创建 `mox-cloud-domain-traits` crate | 将 L4/L6 trait（StorageBackend/MetaStorage/LifecycleEvaluator）集中定义，解除 s3→volume 直接依赖 |
| P1 | 统一错误类型 | 定义 `CloudError` 顶层枚举，各 crate Error 转换为 CloudError，保留上下文 |
| P1 | 统一并发控制规范 | 制定 Mutex/RwLock/Atomic 使用规范，文档化每个锁的粒度和争用预期 |
| P2 | 统一测试基础设施 | 创建 `mox-cloud-test-utils` crate，集中 mock 存储/测试辅助 |

---

## 5. 端到端业务处理流程图（10 个核心流程）

> 每个流程含：步骤编号 / 模块职责 / 输入输出 / 错误处理 / 并发控制点

### 5.1 对象写入流程（PutObject）

```
Client → S3 Handler(L1) → Lifecycle Check(L2) → VolumeServer(L4)
  → Backpressure.acquire(L5) → BufferPool.acquire(L5)
  → ReedSolomon.encode(L5) → MultiWriter.write_quorum(L5)
  → StorageBackend.put(L6) → MetaStorage.set_attr(L4)
  → BufferPool.release(L5) → Backpressure.release(L5) → Response(L1)
```

| 步骤 | 模块 | 输入 | 输出 | 错误处理 | 并发控制 |
|---|---|---|---|---|---|
| 1 | S3 Handler | HTTP 请求 | PutObjectRequest | 400/403/404 | 无状态 |
| 2 | Lifecycle | ObjectMeta | 是否拦截 | 409（Object Lock） | — |
| 3 | VolumeServer | ChunkData | WriteResult | VolumeError 传播 | **Backpressure.try_acquire**（阶段三） |
| 4 | BufferPool | size | PooledBuffer | OOM 时分配失败 | 池化互斥 |
| 5 | ReedSolomon | data | data+parity shards | 编码失败 | 纯函数无状态 |
| 6 | MultiWriter | shards | write_quorum_result | 少于 quorum 失败 | FuturesUnordered 并发 |
| 7 | StorageBackend | shard_id+data | 落盘确认 | IO 错误重试 | 各后端独立 |
| 8 | MetaStorage | path+attr | 元数据持久化 | 冲突错误 | 元数据后端锁 |
| 9 | BufferPool | — | 归还缓冲 | — | RAII 自动 |
| 10 | Backpressure | — | 释放 permit | — | RAII 自动 |
| 11 | S3 Handler | WriteResult | HTTP 200/ETag | 错误映射 | — |

### 5.2 对象读取流程（GetObject）

```
Client → S3 Handler(L1) → VolumeServer(L4)
  → ReaderPipeline.read_first_success(L5) [HedgedReader + SimpleReader 组合]
  → ReedSolomon.reconstruct(L5) [若有 shard 缺失]
  → reconstruction verification(L5) [fail-closed]
  → BufferPool.wrap(L5) → Response Stream(L1)
```

| 步骤 | 模块 | 关键控制点 |
|---|---|---|
| 1 | S3 Handler | Range 解析、ACL 检查 |
| 2 | VolumeServer | 路由到 ReaderPipeline |
| 3 | ReaderPipeline | **read_first_success**：FuturesUnordered 并发，取最快返回（阶段三 ReaderCapability） |
| 4 | HedgedReader | locality 排序（Local<SameNode<Remote），hedge_delay 后追加备用 reader |
| 5 | ReedSolomon | 若可用 shard < data_shards，触发重建 |
| 6 | reconstruction verification | 冗余 parity 可用时重算比对，不一致 **fail-closed**（阶段二） |
| 7 | BufferPool | 包装返回数据，RAII 自动归还 |
| 8 | S3 Handler | 流式返回，Content-MD5 校验 |

### 5.3 对象删除流程（DeleteObject）

```
Client → S3 Handler → Object Lock Check → Lifecycle Pending Replication Check
  → DeleteAllVersions 短路（阶段三） → MetaStorage.remove → StorageBackend.delete
  → 标记 tombstone → 异步 GC
```

**DeleteAllVersions 三重守卫**（阶段三）：
1. 无 Object Lock 保留
2. 无版本锁定（Legal Hold）
3. 无 Pending 状态的复制任务
满足后走短路路径，直接删除所有版本，跳过逐版本循环。

### 5.4 纠删码编码流程

```
input_data → 分片(data_shards) → 补零对齐 → GF(2^8) 矩阵乘法
  → parity_shards → [可选 SIMD 加速] → 输出(data+parity)
  → 矩阵缓存查找(RwLock<HashMap> O(1)) → 未命中则计算并缓存
```

关键算法点：
- 矩阵缓存：阶段二从 `Mutex<Vec>` 线性查找优化为 `OnceLock<RwLock<HashMap<(u16,u16), Arc<Matrix>>>>`，double-checked locking
- SIMD：16 子表 LUT 级联 AVX2/NEON，feature flag `simd_force_on/off` 控制
- 编码参数：data_shards + parity_shards，默认 4+2，支持 12+4 等配置

### 5.5 纠删码重建流程

```
available_shards → 缺失检测 → 重建矩阵推导 → GF(2^8) 逆矩阵乘法
  → 重建缺失 shards → [冗余 parity 可用时] reconstruction verification
  → 重算 parity 逐字节比对 → 一致则返回 / 不一致则 ReconstructionVerificationFailed (fail-closed)
```

**fail-closed 原则**（阶段二）：当冗余 parity 可用时，重建后必须重算 parity 并逐字节比对，不一致直接返回错误，绝不返回可能损坏的数据。

### 5.6 生命周期迁移流程

```
Lifecycle Scanner → ScanBudget 检查(时间/IO/容量三维) → 遍历对象
  → LifecycleEvaluator.should_transition → 符合规则 → 标记 Pending
  → 复制到目标层 → 复制等待门控(阶段三) → 切换元数据指针
  → 源层对象标记为可 GC
```

**三维扫描预算**（阶段三）：
- TimeBudget：单次扫描最大时长，超时暂停
- IoBudget：单次扫描最大 IO 操作数，令牌桶限流
- CapacityBudget：目标层剩余容量阈值，低于阈值暂停迁移

### 5.7 Rebalance 流程

```
Rebalance Scheduler → 采集各 volume 容量/负载 → compute_cluster_balance
  → 生成迁移计划(plan_id) → 选择源/目标 chunk → 执行迁移
  → 进度跟踪 → 完成后更新拓扑 → 下一轮调度
```

关键修复（阶段一）：
- 1MB 最小迁移阈值改为仅跳过 `bytes_to_move == 0`
- 空节点 balance 返回 0 而非 100
- `estimated_improvement` 从硬编码 60 改为基于标准差线性映射

### 5.8 快照流程

```
Client/API → SnapshotManager.create → 冻结元数据视图
  → 记录 chunk 引用计数 → 写入快照元数据 → 返回 snapshot_id
  → [读取时] 按快照时间点视图读取 → COW 保护
```

### 5.9 配额流程

```
FileOperation → QuotaManager.check → 当前用量 + 本次操作 ≤ 配额?
  → 是：放行，更新用量
  → 否：返回 QuotaExceeded 错误
```

### 5.10 文件锁流程

```
FileOperation → FileLock.acquire → 检查冲突锁(读-写/写-写)
  → 无冲突：加锁，返回 LockGuard
  → 冲突：等待或返回 LockConflict
  → [操作完成] LockGuard drop 自动释放
```

---

## 6. 灵活可扩展性设计

### 6.1 Feature Flags（10 个）

| Flag | 默认值 | 控制 |
|---|---|---|
| `rustfs_ecstore_backend` | false | 启用 RustFS ecstore 作为可选 L6 后端（接入点已定义） |
| `rustfs_rio_backend` | false | 启用 RustFS rio 作为可选 I/O 管线（接入点已定义） |
| `simd_force_on` | false | 强制启用 SIMD 纠删码加速 |
| `simd_force_off` | false | 强制禁用 SIMD（回退标量） |
| `backpressure_enabled` | true | 启用 CAS 背压信号量（阶段三接入写入主路径） |
| `hedged_read_enabled` | true | 启用 hedged 读仲裁 |
| `multi_writer_enabled` | true | 启用 MultiWriter 写仲裁 |
| `buffer_pool_enabled` | true | 启用 PooledBuffer 池化（阶段三） |
| `scan_budget_enabled` | true | 启用生命周期三维扫描预算（阶段三） |
| `reconstruction_verification` | true | 启用纠删码重建 fail-closed 验证（阶段二） |

### 6.2 可配置参数清单（7 类）

| 类别 | 参数 | 默认值 | 位置 |
|---|---|---|---|
| 纠删码 | data_shards / parity_shards | 4 / 2 | ErasureCodingConfig |
| 纠删码 | matrix_cache_size | 256 | ErasureCodingConfig |
| 写仲裁 | write_quorum / stall_timeout | data+1 / 5s | WriteArbitrationConfig |
| 读仲裁 | hedge_delay / read_timeout | 200ms / 30s | ReadArbitrationConfig |
| 背压 | max_concurrent_writes / cooldown | 1024 / 1s | BackpressureConfig |
| 缓冲池 | 四层分档上限 / 全局上限 | 各 64MB / 256MB | BufferPoolConfig |
| 扫描预算 | time_budget / io_budget / capacity_threshold | 30s / 1000 / 20% | ScanBudget |

所有参数支持 `from_env()` 环境变量覆盖（30+ 环境变量），无需重新编译。

### 6.3 可插拔后端设计

```
VolumeServer
  ├── StorageBackend (trait, L6)
  │     ├── LocalBackend (默认，本地盘)
  │     ├── S3Backend (远程对象存储)
  │     └── RustFSEcstoreBackend (可选, feature flag, 阶段四)
  └── MetaStorage (trait, L4)
        ├── SQLiteMeta (默认，单机)
        ├── PgCitusMeta (分布式)
        └── RedisMeta (缓存层)
```

---

## 7. 阶段三已完成改造详情

### 7.1 PooledBuffer 四层分档缓冲池

- **文件**：`volume-svc/src/buffer_pool.rs`（~900 行）
- **设计**：四层分档（64B-4KB / 4KB-64KB / 64KB-1MB / 1MB-16MB），`Weak<BufferPoolInner>` 避免循环引用，`PooledBuffer` RAII 自动归还，`Deref/DerefMut` 到 `[u8]`
- **全局上限**：256MB，`BufferPoolConfig` 完全可配置
- **与 RustFS 差异**：独立重写，用 `Vec<u8>` + `Mutex<Vec<Vec<u8>>>` 而非 RustFS 的 `BytesMut` + `tokio::Semaphore`
- **测试**：14 个新增测试

### 7.2 CAS 背压接入写入主路径

- **修改**：`volume_server.rs` — `VolumeServer` 新增 `backpressure: Arc<BackpressureMonitor>` 字段，`write_chunk()` 入口调用 `try_acquire()`，被拒绝返回 `VolumeError::BackpressureRejected`
- **修改**：`error.rs` — 新增 `BackpressureRejected` 变体
- **兼容性**：`new()` 签名不变（28 处调用方零修改），新增 `with_backpressure_config()` Builder
- **测试**：5 个背压接入测试

### 7.3 ReaderCapability 组合式 reader 管线

- **文件**：`volume-svc/src/reader_capability.rs`
- **设计**：`ReaderCapability` trait（6 个方法，2 个默认实现）+ `SimpleReader` + `ReaderPipeline`（组合式，支持嵌套）+ `probe_capabilities()`
- **修改**：`hedged_reader.rs` — 为 `HedgedReader` 实现 `ReaderCapability` trait
- **测试**：11 个 reader_capability 测试 + 2 个 hedged_reader 集成测试

### 7.4 三维扫描预算 + 灵活可配置

- **文件**：`s3-svc/src/scanner.rs`（ScanBudget 三维 + ScanBudgetTracker + ScanStats）
- **文件**：`s3-svc/src/config.rs`（S3ServiceConfig + LifecycleConfig + ReplicationConfig + InventoryConfig + FeatureFlags + from_env）
- **文件**：`volume-svc/src/config.rs`（VolumeServiceConfig + ErasureCodingConfig + WriteArbitrationConfig + ReadArbitrationConfig + VolumeFeatureFlags + from_env）
- **修改**：`s3-svc/lifecycle.rs` — 集成 scan_budget + transition_scan 预算检查
- **测试**：21 个新增测试

---

## 8. 改进路线图

### 8.1 阶段三已完成（10 项）

1. ✅ PooledBuffer 四层分档缓冲池
2. ✅ CAS 背压接入 volume_server 写入主路径
3. ✅ ReaderCapability 组合式 reader 管线
4. ✅ HedgedReader 实现 ReaderCapability trait
5. ✅ 三维扫描预算（Time/IO/Capacity）
6. ✅ S3ServiceConfig 全局配置 + from_env
7. ✅ VolumeServiceConfig 全局配置 + from_env
8. ✅ 10 个 Feature Flags（含 RustFS 后端接入点）
9. ✅ DeleteAllVersions 三重守卫短路
10. ✅ 新增 52 个测试，全量回归绿色

### 8.2 阶段四建议（13 项，按优先级排序）

**P0（必须）**：
1. 创建 `mox-cloud-kernel` crate：抽离 L5 算法（reed_solomon/multi_writer/hedged_reader/backpressure/buffer_pool/reader_capability/scanner）
2. 创建 `mox-cloud-domain-traits` crate：集中定义 StorageBackend/MetaStorage/LifecycleEvaluator trait
3. 解除 s3→volume 直接依赖：s3 依赖 StorageBackend trait 而非 VolumeServer 具体类型
4. RustFS ecstore 后端实现：实现 StorageBackend trait，feature flag 控制

**P1（重要）**：
5. 统一错误类型：CloudError 顶层枚举，保留上下文链
6. 统一 MetaStorage 抽象：filer 与 volume 的元数据管理统一
7. PooledBuffer 接入 s3/filer：将缓冲池推广到所有 crate
8. ReaderPipeline 接入 s3 读路径：S3 GetObject 使用 ReaderPipeline

**P2（改进）**：
9. 统一测试基础设施：mox-cloud-test-utils crate
10. 并发控制规范文档化
11. 配置热更新：支持运行时修改配置（无需重启）
12. 可观测性增强：统一 metrics/tracing/logging

**P3（长期）**：
13. RustFS rio I/O 管线对接：作为可选 I/O 层

---

## 9. 引用文档清单（29 项，仓根相对路径）

1. `docs/working-reports/20260902_hybrid_architecture_route_a_design.md`
2. `docs/working-reports/20260902_hybrid_architecture_phase1_verification_report.md`
3. `docs/working-reports/20260903_hybrid_architecture_phase2_verification_report.md`
4. `docs/working-reports/20260823_cloud_drive_and_relgraph_selfdev_plan.md`
5. `docs/expert-alliance/00-INTEGRATED-INDEX.md`
6. `docs/standards/expert-alliance-normalization-mode.md`
7. `docs/standards/expert-alliance-flow-standard.md`
8. `docs/working-reports/mox-expert-alliance-processing-mode.md`
9. `docs/domains-code-review.md`
10. `distributed-architecture-report/distributed-architecture-report.html`
11. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/lib.rs`
12. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/volume_server.rs`
13. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/reed_solomon.rs`
14. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/multi_writer.rs`
15. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/hedged_reader.rs`
16. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/backpressure.rs`
17. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/buffer_pool.rs`
18. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/reader_capability.rs`
19. `platform/domains/cloud/svc/mox-cloud-volume-svc/src/config.rs`
20. `platform/domains/cloud/svc/mox-cloud-s3-svc/src/lib.rs`
21. `platform/domains/cloud/svc/mox-cloud-s3-svc/src/lifecycle.rs`
22. `platform/domains/cloud/svc/mox-cloud-s3-svc/src/scanner.rs`
23. `platform/domains/cloud/svc/mox-cloud-s3-svc/src/config.rs`
24. `platform/domains/cloud/svc/mox-cloud-filer-svc/src/lib.rs`
25. `platform/domains/cloud/svc/mox-cloud-master-svc/src/lib.rs`
26. `platform/domains/cloud/svc/mox-cloud-rebalance-svc/src/lib.rs`
27. `ais/RustFS/ARCHITECTURE.md`
28. `ais/RustFS/docs/architecture/s3-compatibility-matrix.md`
29. `ais/RustFS/README_ZH.md`

---

**文档版本**：v1.0 ｜ **发布日期**：2026-09-03 ｜ **权威等级**：🟢 ADR
**维护规则**：架构变更后必须同步更新本文档；每次更新递增版本号并更新"最后验证日期"。
