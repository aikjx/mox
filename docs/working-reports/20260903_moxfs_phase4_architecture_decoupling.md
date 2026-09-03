# moxfs 阶段四架构解耦设计文档



| 字段       | 值                                                                                         |
| -------- | ----------------------------------------------------------------------------------------- |
| **文档标题** | moxfs 阶段四架构解耦设计文档                                                                         |
| **文档编号** | ADR-MOXFS-P4-20260903                                                                     |
| **版本**   | v1.0                                                                                      |
| **权威等级** | 🟢 ADR（Architecture Decision Record）                                                      |
| **日期**   | 2026-09-03                                                                                |
| **作者**   | 开发联盟 R（moxfs 全自研云盘知识库）                                                                    |
| **状态**   | ✅ 已落地（代码实测通过）                                                                             |
| **关联文档** | `docs/working-reports/20260903_moxfs_phase4_verification_report.md`（VR-MOXFS-P4-20260903） |
| **适用范围** | `platform/domains/cloud/` 全模块（7 个 crate：2 core + 5 svc）                                   |
| **项目定位** | moxfs 全自研云盘知识库；RustFS 仅为对标参考对象（Apache 2.0，源码 `ais/RustFS/`，只读）                            |



***

## 1. 文档元信息

### 1.1 编号与版本



| 项    | 值                     |
| ---- | --------------------- |
| 文档编号 | ADR-MOXFS-P4-20260903 |
| 版本   | v1.0                  |
| 发布日期 | 2026-09-03            |
| 作者   | 开发联盟 R                |
| 状态   | ✅ 已落地                 |
| 权威等级 | 🟢 ADR                |

### 1.2 关联文档



| 文档        | 编号                             | 路径                                                                                  |
| --------- | ------------------------------ | ----------------------------------------------------------------------------------- |
| 阶段四验证报告   | VR-MOXFS-P4-20260903           | `docs/working-reports/20260903_moxfs_phase4_verification_report.md`                 |
| 阶段三架构分析   | ADR-CLOUD-HYBRID-A-P3-20260903 | `docs/working-reports/20260903_hybrid_architecture_phase3_architecture_analysis.md` |
| 阶段三验证报告   | VR-CLOUD-HYBRID-A-P3-20260903  | `docs/working-reports/20260903_hybrid_architecture_phase3_verification_report.md`   |
| 路线 A 架构设计 | ADR-CLOUD-HYBRID-A-20260902    | `docs/working-reports/20260902_hybrid_architecture_route_a_design.md`               |

### 1.3 命名规范声明

本文档及阶段四全部交付物统一使用 **MOXFS** 前缀。项目主体为 **moxfs 全自研云盘知识库**，RustFS 仅作为对标参考对象（Apache 2.0 许可，源码位于 `ais/RustFS/`，只读，不引入依赖）。不再使用 "CLOUD-HYBRID" 或 "混合架构" 作为项目主体命名。



***

## 2. 背景与目标

### 2.1 项目定位

**moxfs** 是全自研云盘知识库系统，覆盖 S3 兼容对象存储、POSIX/FUSE 文件系统、纠删码数据面、元数据管理、生命周期管理、跨节点复制与再平衡等完整云存储能力。所有核心模块均为自研实现，零成品存储系统依赖。

**RustFS**（Apache 2.0 许可，源码 `ais/RustFS/`）仅作为对标学习的参考对象：



* 参考其 ecstore 纠删码存储架构设计理念

* 参考其 rio 异步 IO 模型

* 参考其 io-core 底层 IO 抽象

* **不引入 RustFS 作为直接依赖**，不复制 RustFS 代码

* moxfs 后端为独立适配层实现，通过 trait 抽象与可选后端对接

### 2.2 阶段三遗留问题

阶段三（架构融合）完成了 4 项 P0/P1 改造（PooledBuffer 四层分档缓冲池、CAS 背压接入写入主路径、ReaderCapability 组合式 reader 管线、三维扫描预算 + 全局配置），但识别出以下架构问题需在阶段四解决：



| # | 问题                                                         | 影响                            | 阶段四任务                                        |
| - | ---------------------------------------------------------- | ----------------------------- | -------------------------------------------- |
| 1 | L5 算法模块散落在 volume-svc 内，与业务逻辑耦合                            | 算法无法独立复用、独立测试、独立版本化           | P0-1: mox-cloud-kernel crate 抽离              |
| 2 | 核心 trait 分散在各 svc crate 内，无统一契约层                           | 跨 crate  trait 定义重复、版本不一致     | P0-2: mox-cloud-domain-traits crate          |
| 3 | s3-svc 声明 volume-svc 依赖但源码零引用（死依赖），对象数据内联在 ObjectMeta.data | 编译依赖膨胀、元数据与数据耦合、无法切换后端        | P0-3: s3→volume 解耦 + StorageBackend 依赖注入     |
| 4 | 无可选存储后端接入点                                                 | 无法对接 RustFS ecstore 等外部数据面    | P0-3: RustFsEcstoreBackend 骨架 + feature flag |
| 5 | 各 svc 错误类型独立，无统一顶层错误                                       | 跨 crate 错误传播需手动转换、上下文丢失       | P0-4: CloudError 统一错误类型                      |
| 6 | PooledBuffer 仅在 volume-svc 内可用                             | s3/filer 热点路径仍重复 Vec 分配       | P1-5: PooledBuffer 推广到 s3/filer              |
| 7 | ReaderPipeline 仅在 volume-svc 内定义                           | S3 GetObject 无法利用 hedged read | P1-6: ReaderPipeline 接入 S3 读路径               |

### 2.3 阶段四目标

**P0（必须完成，4 项）**：



1. 创建 `mox-cloud-kernel` crate：抽离 10 个 L5 算法模块，零业务依赖

2. 创建 `mox-cloud-domain-traits` crate：集中定义 5 大核心 trait + 30+ 关联类型

3. s3→volume 解耦 + RustFS ecstore 后端骨架：移除死依赖，StorageBackend 依赖注入，ObjectMeta 用 chunk\_id 替代内联 data

4. CloudError 统一错误类型：15 变体枚举 + 各 svc From 转换链

**P1（重要，2 项）**：

5\. PooledBuffer 推广到 s3-svc /filer-svc 热点路径

6\. ReaderPipeline 接入 S3 GetObject 读路径

**非目标**：



* RustFS ecstore 实际 FFI / 进程对接（留待阶段五）

* LocalFsStorageBackend 完整实现（留待阶段五）

* ReaderPipeline 默认启用（留待阶段五生产验证）



***

## 3. moxfs L1-L6 六层架构总览

### 3.1 层次定义



| 层级     | 名称      | 核心职责                          | 关键模块 /crate                                                                     |
| ------ | ------- | ----------------------------- | ------------------------------------------------------------------------------- |
| **L1** | API 网关层 | 对外协议入口、请求路由、认证鉴权              | s3-svc（S3 兼容 API）、filer-svc（POSIX/FUSE API）                                     |
| **L2** | 协议适配层   | 协议解析、语义映射、请求 / 响应序列化          | S3 协议解析器、POSIX 语义映射、SigV4 签名验证                                                  |
| **L3** | 元数据层    | bucket/object 元数据、目录树、生命周期、配额 | MetaStorage trait、bucket/object 元数据、lifecycle 评估器                               |
| **L4** | 数据路由层   | 后端路由、分片读写仲裁、读管线、背压            | StorageBackend trait、ShardReader/ShardWriter、ReaderPipeline、BackpressureMonitor |
| **L5** | 算法引擎层   | 纠删码编解码、写仲裁、读仲裁、缓冲池、扫描预算       | mox-cloud-kernel（10 个算法模块）                                                      |
| **L6** | 存储后端层   | 底层 chunk 级存取、多后端适配            | InMemoryStorageBackend、RustFsEcstoreBackend（骨架）、LocalFs（待实现）                    |

### 3.2 ASCII 架构图



```
┌─────────────────────────────────────────────────────────────────────┐

│                        L1  API 网关层                                │

│   ┌──────────────────────┐      ┌──────────────────────┐           │

│   │     s3-svc           │      │    filer-svc         │           │

│   │  S3 兼容 API (:9000) │      │  POSIX/FUSE API      │           │

│   └──────────┬───────────┘      └──────────┬───────────┘           │

└──────────────┼───────────────────────────────┼───────────────────────┘

&#x20;              │                               │

┌──────────────▼───────────────────────────────▼───────────────────────┐

│                        L2  协议适配层                                  │

│   S3 协议解析 │ POSIX 语义映射 │ SigV4 验证 │ ETag 计算              │

└──────────────────────────────┬────────────────────────────────────────┘

&#x20;                              │

┌──────────────────────────────▼────────────────────────────────────────┐

│                        L3  元数据层                                    │

│   ┌──────────────────────────────────────────────────────────┐        │

│   │  MetaStorage trait (L4)                                   │        │

│   │  bucket/object 元数据 │ 目录树 │ lifecycle │ 配额 │ 版本  │        │

│   └──────────────────────────────────────────────────────────┘        │

└──────────────────────────────┬────────────────────────────────────────┘

&#x20;                              │

┌──────────────────────────────▼────────────────────────────────────────┐

│                        L4  数据路由层                                  │

│   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  │

│   │ StorageBackend   │  │ ShardReader /    │  │ ReaderPipeline   │  │

│   │ trait (L6 抽象)  │  │ ShardWriter      │  │ (hedged read)    │  │

│   └────────┬─────────┘  └──────────────────┘  └──────────────────┘  │

│            │                                                            │

│   ┌────────▼─────────┐  ┌──────────────────┐                          │

│   │ Backpressure     │  │ BufferPool       │                          │

│   │ Monitor (CAS)    │  │ (四层分档)       │                          │

│   └──────────────────┘  └──────────────────┘                          │

└──────────────────────────────┬────────────────────────────────────────┘

&#x20;                              │

┌──────────────────────────────▼────────────────────────────────────────┐

│                   L5  算法引擎层 (mox-cloud-kernel)                   │

│  ┌──────┐ ┌──────────┐ ┌───────────┐ ┌────────────┐ ┌───────────┐  │

│  │reed\_ │ │gf256\_    │ │multi\_     │ │hedged\_     │ │backpressure│ │

│  │solomon│ │simd      │ │writer     │ │reader      │ │            │  │

│  └──────┘ └──────────┘ └───────────┘ └────────────┘ └───────────┘  │

│  ┌──────────┐ ┌────────────────┐ ┌───────┐ ┌────────┐ ┌──────────┐ │

│  │buffer\_   │ │reader\_capability│ │scanner│ │profile│ │metrics   │ │

│  │pool      │ │                │ │       │ │        │ │          │ │

│  └──────────┘ └────────────────┘ └───────┘ └────────┘ └──────────┘ │

│  零业务依赖：仅 tokio / async-trait / futures / thiserror /           │

│  parking\_lot / serde / bytes / rand / tracing                          │

└──────────────────────────────┬────────────────────────────────────────┘

&#x20;                              │

┌──────────────────────────────▼────────────────────────────────────────┐

│                        L6  存储后端层                                  │

│   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  │

│   │ InMemory         │  │ RustFsEcstore    │  │ LocalFs          │  │

│   │ StorageBackend   │  │ Backend (骨架)   │  │ (待实现)         │  │

│   │ ✅ 完整实现       │  │ ⚠️ Unsupported   │  │ ⏳ 阶段五        │  │

│   └──────────────────┘  └──────────────────┘  └──────────────────┘  │

│   feature flag: rustfs\_ecstore\_backend (默认 false)                   │

└─────────────────────────────────────────────────────────────────────────┘
```

### 3.3 依赖方向



```
svc crates (s3/volume/filer/master/rebalance)

&#x20;   │

&#x20;   ├──► mox-cloud-kernel (L5 算法引擎，零业务依赖)

&#x20;   │

&#x20;   └──► mox-cloud-domain-traits (L4/L6 trait 契约，零 svc 依赖)

&#x20;             │

&#x20;             └──► 无（纯 trait 定义层）

mox-cloud-kernel ──X──► mox-cloud-domain-traits  (无依赖，算法层不依赖契约层)

mox-cloud-domain-traits ──X──► mox-cloud-kernel  (无依赖，契约层不依赖算法层)
```

**关键设计决策**：kernel 与 domain-traits 之间**无相互依赖**。kernel 是纯算法层，不感知 StorageBackend/MetaStorage 等业务 trait；domain-traits 是纯契约层，不感知纠删码 / 背压等算法实现。svc crate 同时依赖两者，在业务逻辑中组合使用。



***

## 4. mox-cloud-kernel crate 设计

### 4.1 定位

`mox-cloud-kernel` 是 moxfs 的 **L5 纯算法内核 crate**，从 volume-svc /s3-svc 抽离 10 个算法模块，实现：



* **零业务依赖**：不依赖任何 svc crate、不依赖 domain-traits

* **独立编译 / 测试 / 版本化**：算法变更不触发业务 crate 重编译

* **跨 crate 复用**：volume-svc、s3-svc、filer-svc 共享同一套算法实现

* **API 零变更**：通过内联 re-export 模块保持 volume-svc/s3-svc 原有模块路径不变

**crate 路径**：`platform/domains/cloud/core/mox-cloud-kernel/`

### 4.2 10 个模块职责



| #  | 模块                  | 文件                              | 职责                                           | 关键技术                                                                   |
| -- | ------------------- | ------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------- |
| 1  | `reed_solomon`      | `reed_solomon.rs` (\~37KB)      | Vandermonde 矩阵 + GF (2^8) Gauss-Jordan 纠删码引擎 | 矩阵缓存、reconstruction verification、2+1/12+4 等 profile                    |
| 2  | `gf256_simd`        | `gf256_simd.rs` (\~30KB)        | GF (2^8) SIMD 加速内核                           | AVX2/NEON 自动检测、`gf_vec_mul_auto`、`is_avx2_supported`                   |
| 3  | `multi_writer`      | `multi_writer.rs` (\~15KB)      | 多副本写仲裁                                       | 法定人数确认、WriteProgressPolicy、ShardWriter trait                           |
| 4  | `hedged_reader`     | `hedged_reader.rs` (\~21KB)     | Hedged 读仲裁                                   | 并发取最快、locality 感知、ShardReadCost 分级                                     |
| 5  | `backpressure`      | `backpressure.rs` (\~18KB)      | CAS 背压信号量                                    | 三态状态机（Acquired/Rejected/Waiting）、AtomicUsize CAS、permit RAII           |
| 6  | `buffer_pool`       | `buffer_pool.rs` (\~32KB)       | 四层分档缓冲池                                      | 64B-4KB / 4KB-64KB / 64KB-1MB / 1MB-16MB、Weak 避免循环引用、PooledBuffer RAII |
| 7  | `reader_capability` | `reader_capability.rs` (\~26KB) | ReaderCapability trait + ReaderPipeline      | 6 方法 trait（2 默认实现）、组合式管线、`read_first_success`、`probe_capabilities`     |
| 8  | `scanner`           | `scanner.rs` (\~19KB)           | 三维扫描预算                                       | TimeBudget + IoBudget（双令牌桶）+ CapacityBudget、ScanBudgetTracker          |
| 9  | `profile`           | `profile.rs` (\~3KB)            | EcProfile 纠删码配置                              | data\_shards/parity\_shards、DEFAULT\_MIN\_OBJ\_SIZE                    |
| 10 | `metrics`           | `metrics.rs` (\~16KB)           | 数据面指标                                        | encode\_us 直方图、REBUILD\_COUNT、SHARDS\_LOST\_TOTAL、全局原子指标               |

### 4.3 依赖关系

**Cargo.toml 依赖清单**（实测，零业务依赖）：



| 依赖            | 用途                     |
| ------------- | ---------------------- |
| `tokio`       | 异步运行时、async trait 支持   |
| `async-trait` | async fn in trait      |
| `futures`     | Future 组合子             |
| `thiserror`   | 错误类型派生                 |
| `parking_lot` | 高效 Mutex/RwLock        |
| `serde`       | 序列化 / 反序列化（配置持久化）      |
| `bytes`       | Bytes/BytesMut 零拷贝缓冲区  |
| `rand`        | 随机数（hedged read 抖动、测试） |
| `tracing`     | 结构化日志                  |

**Feature flags**：



* `simd`：启用 SIMD 加速路径（默认不启用，运行时自动检测）

**依赖关系图**：



```
mox-cloud-kernel

&#x20;   ├── tokio

&#x20;   ├── async-trait

&#x20;   ├── futures

&#x20;   ├── thiserror

&#x20;   ├── parking\_lot

&#x20;   ├── serde

&#x20;   ├── bytes

&#x20;   ├── rand

&#x20;   └── tracing

（无任何 svc crate 依赖、无 domain-traits 依赖、无项目内部 crate 依赖）
```

### 4.4 API 兼容策略：内联 re-export 模块模式

volume-svc 和 s3-svc 原有大量代码通过 `crate::reed_solomon::ReedSolomonEngine`、`crate::buffer_pool::BufferPool` 等路径引用算法模块。抽离到 kernel crate 后，为保持 **API 零变更**，采用**内联 re-export 模块**模式：

**volume-svc&#x20;**`lib.rs`**&#x20;实测模式**（10 个内联模块）：



```
pub mod backpressure {

&#x20;   pub use mox\_cloud\_kernel::backpressure::\*;

}

pub mod buffer\_pool {

&#x20;   pub use mox\_cloud\_kernel::buffer\_pool::\*;

}

pub mod gf256\_simd {

&#x20;   pub use mox\_cloud\_kernel::gf256\_simd::\*;

}

pub mod hedged\_reader {

&#x20;   pub use mox\_cloud\_kernel::hedged\_reader::\*;

}

pub mod metrics {

&#x20;   pub use mox\_cloud\_kernel::metrics::\*;

}

pub mod multi\_writer {

&#x20;   pub use mox\_cloud\_kernel::multi\_writer::\*;

}

pub mod profile {

&#x20;   pub use mox\_cloud\_kernel::profile::\*;

}

pub mod reader\_capability {

&#x20;   pub use mox\_cloud\_kernel::reader\_capability::\*;

}

pub mod reed\_solomon {

&#x20;   pub use mox\_cloud\_kernel::reed\_solomon::\*;

}

// scanner 在 s3-svc 中使用相同模式
```

**s3-svc&#x20;**`lib.rs`**&#x20;实测模式**：



```
pub mod scanner {

&#x20;   pub use mox\_cloud\_kernel::scanner::\*;

}
```

**效果**：



* 所有原有 `crate::reed_solomon::*`、`crate::buffer_pool::*` 等引用路径**零修改**

* 28 处 volume-svc 调用方、15 处 s3-svc 调用方无需任何改动

* 模块文档注释、trait 方法签名、类型定义全部保持不变

* 6 个原 `pub(crate)` 项提升为 `pub`（kernel crate 边界需要）



***

## 5. mox-cloud-domain-traits crate 设计

### 5.1 定位

`mox-cloud-domain-traits` 是 moxfs 的 **L4/L6 领域契约层 crate**，集中定义所有跨 crate 共享的核心 trait，消除各 svc crate 内部 trait 定义分散、重复、版本不一致的问题。

**设计约束**：



* **零 svc 依赖**：不依赖任何 volume/s3/filer/master/rebalance svc crate

* **零 kernel 依赖**：不依赖 mox-cloud-kernel（契约层不感知算法实现）

* **Object-safe**：所有 trait 均可通过 `dyn Trait` 动态分发

* **serde 派生**：所有数据结构体派生 `Serialize/Deserialize`

**crate 路径**：`platform/domains/cloud/core/mox-cloud-domain-traits/`

### 5.2 5 大核心 trait

#### 5.2.1 StorageBackend（L6 存储后端抽象）

**文件**：`storage_backend.rs`

**层级**：L6



```
\#\[async\_trait]

pub trait StorageBackend: Send + Sync {

&#x20;   async fn put\_chunk(\&self, chunk\_id: \&ChunkId, data: &\[u8]) -> Result\<ChunkInfo, StorageError>;

&#x20;   async fn get\_chunk(\&self, chunk\_id: \&ChunkId) -> Result\<Vec\<u8>, StorageError>;

&#x20;   async fn delete\_chunk(\&self, chunk\_id: \&ChunkId) -> Result\<bool, StorageError>;

&#x20;   async fn chunk\_exists(\&self, chunk\_id: \&ChunkId) -> Result\<bool, StorageError>;

&#x20;   async fn list\_chunks(\&self, prefix: \&str, marker: Option<\&str>, limit: u32) -> Result\<ChunkListPage, StorageError>;

&#x20;   fn backend\_type(\&self) -> BackendType;

&#x20;   fn capabilities(\&self) -> BackendCapabilities;

&#x20;   fn name(\&self) -> &'static str;

}
```

**关联类型**：`ChunkId`（String 新类型）、`ChunkInfo`、`ChunkListPage`、`BackendType`（5 变体：LocalFs/S3Compatible/RustFsEcstore/InMemory/Other）、`BackendCapabilities`、`ConsistencyModel`（3 变体）、`StorageError`（6 变体）

**职责**：统一底层 chunk 级存取契约，所有存储后端（内存 / 本地 FS/RustFS ecstore/S3 兼容）均需实现。

#### 5.2.2 MetaStorage（L4 元数据存储抽象）

**文件**：`meta_storage.rs`

**层级**：L4

**职责**：统一文件 / 对象元数据存取契约，支持目录树遍历、并发控制、事务语义。

**关联类型**：`MetaKey`、`MetaValue`、`DirEntry`、`DirListPage`、`EntryType`、`ConcurrencyModel`、`MetaError`

#### 5.2.3 LifecycleEvaluator（L4 生命周期评估抽象）

**文件**：`lifecycle.rs`

**层级**：L4

**职责**：统一存储分级与过期策略评估，支持 hot/warm/cold 三级存储、自动转储、过期删除。

**关联类型**：`StorageClass`、`StorageClassTransition`、`LifecycleAction`、`LifecycleThresholds`、`ObjectLifecycleMeta`、`ReplicationStatus`

#### 5.2.4 ShardReader（L4 分片读取抽象）

**文件**：`shard_reader.rs`

**层级**：L4

**职责**：统一跨节点分片读取与对冲读契约，支持 locality 感知、超时控制、读修复。

**关联类型**：`ShardLocation`、`ShardReadCost`、`HedgeConfig`、`StorageTier`、`ReadError`

#### 5.2.5 ShardWriter（L4 分片写入抽象）

**文件**：`shard_writer.rs`

**层级**：L4

**职责**：统一多副本分片写入与法定人数确认契约，支持并发提示、写入进度策略。

**关联类型**：`WriteQuorum`、`WriteResult`、`ConcurrencyHint`、`WriteError`

### 5.3 关联类型统计



| 模块                | 结构体 / 枚举数量                                                                                                               |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `storage_backend` | 8（ChunkId/ChunkInfo/ChunkListPage/BackendType/BackendCapabilities/ConsistencyModel/StorageError + trait）                 |
| `meta_storage`    | 8（MetaKey/MetaValue/DirEntry/DirListPage/EntryType/ConcurrencyModel/MetaError + trait）                                   |
| `lifecycle`       | 7（StorageClass/StorageClassTransition/LifecycleAction/LifecycleThresholds/ObjectLifecycleMeta/ReplicationStatus + trait） |
| `shard_reader`    | 6（ShardLocation/ShardReadCost/HedgeConfig/StorageTier/ReadError + trait）                                                 |
| `shard_writer`    | 5（WriteQuorum/WriteResult/ConcurrencyHint/WriteError + trait）                                                            |
| `error`           | 2（CloudError 15 变体 + CloudResult 别名）                                                                                     |
| **合计**            | **36**                                                                                                                   |

### 5.4 Object-safe 保证

所有 5 大 trait 均满足 object-safe 约束：



* 所有方法均为 `&self`（无 `self` by value）

* 无泛型方法（使用 `async_trait` 宏消除 async fn 的 impl Trait 问题）

* 无关联常量（trait 级）

* 返回类型均为具体类型或 `Result<T, E>`（无 `impl Trait`）

* 已通过 `Box<dyn StorageBackend>` / `Arc<dyn StorageBackend>` 动态分发实测验证

### 5.5 CloudError 统一错误类型

**文件**：`error.rs`

`CloudError` 是 moxfs 跨 crate 统一顶层错误枚举，覆盖存储、元数据、读、写、各 svc 服务层及通用错误场景。

**15 变体清单**（实测）：



| #  | 变体                              | 用途               | 转换来源                              |
| -- | ------------------------------- | ---------------- | --------------------------------- |
| 1  | `Storage(#[from] StorageError)` | 存储后端错误           | StorageError 自动转换                 |
| 2  | `Meta(#[from] MetaError)`       | 元数据存储错误          | MetaError 自动转换                    |
| 3  | `Read(#[from] ReadError)`       | 分片读取错误           | ReadError 自动转换                    |
| 4  | `Write(#[from] WriteError)`     | 分片写入错误           | WriteError 自动转换                   |
| 5  | `Volume(String)`                | volume-svc 错误    | VolumeError → CloudError          |
| 6  | `S3(String)`                    | s3-svc 错误        | S3Error → CloudError              |
| 7  | `Filer(String)`                 | filer-svc 错误     | FilerError → CloudError           |
| 8  | `Master(String)`                | master-svc 错误    | 预留                                |
| 9  | `Rebalance(String)`             | rebalance-svc 错误 | 预留                                |
| 10 | `BackpressureRejected(String)`  | 背压机制拒绝请求         | VolumeError::BackpressureRejected |
| 11 | `NotFound(String)`              | 资源未找到            | 各 svc NotFound 变体                 |
| 12 | `AlreadyExists(String)`         | 资源已存在            | S3Error BucketAlreadyExists 等     |
| 13 | `InvalidInput(String)`          | 无效输入             | 各 svc InvalidInput 变体             |
| 14 | `Unsupported(String)`           | 不支持的操作           | RustFsEcstoreBackend 骨架           |
| 15 | `Internal(String)`              | 内部错误             | 兜底                                |

**CloudResult 别名**：



```
pub type CloudResult\<T> = Result\<T, CloudError>;
```

**From 转换链**（实测）：



* `StorageError → CloudError`：`#[from]` 自动

* `MetaError → CloudError`：`#[from]` 自动

* `ReadError → CloudError`：`#[from]` 自动

* `WriteError → CloudError`：`#[from]` 自动

* `VolumeError → CloudError`：`volume-svc/src/error.rs:37` 手动 impl，BackpressureRejected/ChunkNotFound 语义映射

* `S3Error → CloudError`：`s3-svc/src/error.rs:121` 手动 impl，NotFound/AlreadyExists/InvalidInput/Unsupported 语义映射

* `FilerError → CloudError`：`filer-svc/src/error.rs:46` 手动 impl，NotFound/InvalidInput/Unsupported 语义映射



***

## 6. s3→volume 解耦方案

### 6.1 解耦前状态

**死依赖问题**：



* s3-svc `Cargo.toml` 声明了 `mox-cloud-volume-svc` 依赖

* 但 s3-svc 源码中**零引用** volume-svc 的任何类型 / 函数

* 这导致：编译 volume-svc 变更后 s3-svc 被迫重编译、依赖图膨胀、架构上暗示 s3 依赖 volume 数据面

**数据内联问题**：



* `ObjectMeta` 结构体包含 `data: Vec<u8>` 字段，对象数据直接内联在元数据中

* 这导致：元数据与数据耦合、无法切换存储后端、大对象内存占用高、元数据操作携带数据开销

### 6.2 解耦后状态

**依赖移除**（实测 `s3-svc/Cargo.toml`）：



* 移除 `mox-cloud-volume-svc` 依赖

* 新增 `mox-cloud-domain-traits` 依赖（StorageBackend trait）

* 新增 `mox-cloud-kernel` 依赖（scanner/buffer\_pool 等算法）

* s3-svc 当前依赖：`mox-cloud-foundation`、`mox-data-standards-core`、`mox-cloud-master-svc`、`mox-cloud-domain-traits`、`mox-cloud-store-core`、`mox-cloud-kernel`（无 volume-svc）

**数据与元数据分离**（实测 `s3_server.rs:70-72`）：



```
/// 对象元数据（数据与元数据分离）

pub struct ObjectMeta {

&#x20;   // ... 其他元数据字段 ...

&#x20;   /// 数据块 ID，指向 storage\_backend 中存储的实际数据。

&#x20;   chunk\_id: String,

&#x20;   // 不再有 data: Vec\<u8> 字段

}
```

### 6.3 S3Server 重构：依赖注入 + Builder 模式

**核心字段**（实测 `s3_server.rs:51-55`）：



```
pub struct S3Server {

&#x20;   // ... 其他字段 ...

&#x20;   /// 存储后端（依赖注入，默认 InMemoryStorageBackend）

&#x20;   storage\_backend: Arc\<dyn StorageBackend>,

&#x20;   /// 读管线（可选，默认 None，op\_get\_object 走单后端）

&#x20;   reader\_pipeline: Option\<Arc\<crate::storage::reader\_pipeline::S3ReaderPipeline>>,

&#x20;   /// 缓冲池（PooledBuffer 推广）

&#x20;   buffer\_pool: Arc\<BufferPool>,

}
```

**Builder 方法**（实测）：



| 方法                                            | 签名                                                                                   | 用途                                                   |
| --------------------------------------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------------- |
| `new(port, master)`                           | `pub fn new(port: u16, master: Option<Arc<MasterServer>>) -> Self`                   | 默认构造，使用 InMemoryStorageBackend，reader\_pipeline=None |
| `with_storage_backend(port, master, backend)` | `pub fn with_storage_backend(..., storage_backend: Arc<dyn StorageBackend>) -> Self` | 注入自定义存储后端                                            |
| `with_reader_pipeline(pipeline)`              | `pub fn with_reader_pipeline(mut self, pipeline: Arc<S3ReaderPipeline>) -> Self`     | 注入读管线（链式调用）                                          |

**构造流程**：



1. `S3Server::new()` → 内部调用 `with_storage_backend(port, master, Arc::new(InMemoryStorageBackend::new()))`

2. `with_storage_backend()` 设置 storage\_backend，reader\_pipeline 默认为 None

3. 可选链式 `.with_reader_pipeline(pipeline)` 启用 hedged read

### 6.4 数据路由：8 个数据操作函数改为 async trait 路由

**chunk\_id 生成规则**（实测 `s3_server.rs:166-170`）：



```
fn object\_chunk\_id(bucket: \&str, key: \&str, version\_id: \&str) -> String {

&#x20;   // 格式：obj:{bucket}:{key}:{version\_id}

&#x20;   // 覆盖写入时 chunk\_id 不变，put\_chunk 自然覆盖

&#x20;   // 删除标记不创建 chunk（chunk\_id 为空字符串）

}
```

**3 个底层路由函数**（实测 `s3_server.rs:190-232`）：



| 函数                                                                  | 路由方法                                                        | 说明                            |
| ------------------------------------------------------------------- | ----------------------------------------------------------- | ----------------------------- |
| `read_object_data(storage_backend, chunk_id)`                       | `storage_backend.get_chunk(&ChunkId::new(chunk_id))`        | chunk\_id 为空（删除标记）时返回空 Vec    |
| `write_object_data(storage_backend, bucket, key, version_id, data)` | `storage_backend.put_chunk(&ChunkId::new(&chunk_id), data)` | 生成 chunk\_id 后写入，返回 chunk\_id |
| `delete_object_data(storage_backend, chunk_id)`                     | `storage_backend.delete_chunk(&ChunkId::new(chunk_id))`     | chunk\_id 为空时跳过               |

**8 个数据操作函数**（全部改为 async 通过 trait 路由）：



| # | 操作函数                         | 路由调用                                                                     | 说明          |
| - | ---------------------------- | ------------------------------------------------------------------------ | ----------- |
| 1 | `op_put_object`              | `write_object_data()` → `put_chunk`                                      | 写入对象数据到后端   |
| 2 | `op_get_object`              | 优先 `reader_pipeline.read_object()`，回退 `read_object_data()` → `get_chunk` | 读取对象数据      |
| 3 | `op_delete_object`           | `delete_object_data()` → `delete_chunk`                                  | 删除对象数据      |
| 4 | `op_head_object`             | 元数据操作，不路由数据（仅检查 chunk\_id 非空）                                            | HEAD 不读数据   |
| 5 | `op_copy_object`             | `read_object_data()` + `write_object_data()` → `get_chunk` + `put_chunk` | 复制：读源 + 写目标 |
| 6 | `op_upload_part_copy`        | `read_object_data()` + MPU part 存储 → `get_chunk`                         | 分片复制        |
| 7 | `op_complete_or_abort_mpu`   | 完成时合并 parts 数据 → `put_chunk`；中止时删除临时 chunks → `delete_chunk`             | MPU 完成 / 中止 |
| 8 | `op_delete_multiple_objects` | 循环 `delete_object_data()` → `delete_chunk`                               | 批量删除        |

### 6.5 Send 问题修复：parking\_lot MutexGuard 作用域

**问题**：`parking_lot::MutexGuard` 实现 `!Send`，在 async 函数中持有 guard 跨越 `.await` 点会导致 Future 不实现 Send，无法在多线程运行时中 spawn。

**修复方案**：统一采用**代码块作用域**模式，确保 guard 在 `.await` 之前释放：



```
// 错误模式（guard 跨越 .await）

let guard = self.chunks.write();

let data = guard.get(\&id).cloned();

let result = backend.put\_chunk(\&id, \&data).await;  // guard 仍持有，!Send

// 正确模式（代码块作用域隔离）

let data = {

&#x20;   let guard = self.chunks.read();

&#x20;   guard.get(\&id).cloned()

};  // guard 在此释放

let result = backend.put\_chunk(\&id, \&data.as\_ref().unwrap()).await;  // 安全
```

**实测修复点**：s3-svc `in_memory.rs` 中所有 `get_chunk`/`list_chunks` 操作均采用代码块作用域，guard 在异步调用前释放。



***

## 7. RustFS ecstore 可选后端设计

### 7.1 定位

`RustFsEcstoreBackend` 是 moxfs 的 **可选 L6 存储后端**，对标参考 RustFS ecstore 的纠删码存储架构设计。

**关键声明**：



* RustFS 仅为**对标参考对象**（Apache 2.0，源码 `ais/RustFS/`，只读）

* moxfs 后端为**独立适配层实现**，不直接引入 RustFS 依赖

* 当前阶段为**骨架实现**，所有数据面方法返回 `StorageError::Unsupported`

* 实际 FFI / 进程对接留待阶段五

### 7.2 RustFsEcstoreBackend 骨架

**文件**：`s3-svc/src/storage/rustfs_ecstore.rs`（\~7.9KB）

**结构体**（实测）：



```
pub struct RustFsEcstoreBackend {

&#x20;   endpoint: String,   // RustFS ecstore 进程监听地址

&#x20;   pool\_name: String,  // EC 存储池名称

}
```

**构造函数**：



```
pub fn new(endpoint: String, pool\_name: String) -> Self
```

**诊断方法**：



* `is_available() -> bool`：当前始终返回 `false`（骨架阶段）

* `endpoint() -> &str`：获取配置的 endpoint

* `pool_name() -> &str`：获取配置的 pool\_name

**StorageBackend trait 实现**（全部返回 Unsupported）：



| 方法             | 返回                                                                         | 说明                            |
| -------------- | -------------------------------------------------------------------------- | ----------------------------- |
| `put_chunk`    | `Err(StorageError::Unsupported)`                                           | EC 编码分片→写入多节点（待实现）            |
| `get_chunk`    | `Err(StorageError::Unsupported)`                                           | 多节点读 data+parity→EC 解码重建（待实现） |
| `delete_chunk` | `Err(StorageError::Unsupported)`                                           | 通知 ecstore 回收所有 shards（待实现）   |
| `chunk_exists` | `Err(StorageError::Unsupported)`                                           | 待实现                           |
| `list_chunks`  | `Err(StorageError::Unsupported)`                                           | 待实现                           |
| `backend_type` | `BackendType::RustFsEcstore`                                               | 静态标识                          |
| `capabilities` | range\_read=true, atomic\_write=true, consistency=Strong, max\_chunk=128MB | 目标能力声明                        |
| `name`         | `"rustfs-ecstore-backend"`                                                 | 静态名称                          |

**错误信息常量**：



```
const UNSUPPORTED\_MSG: \&str =

&#x20;   "RustFS ecstore backend: 接入点已定义，实际 RustFS 进程/FFI 对接待后续阶段";
```

### 7.3 Feature Flag

**定义**（实测 `s3-svc/Cargo.toml:39-46`）：



```
\[features]

default = \[]

rustfs\_ecstore\_backend = \[]
```

**默认值**：`false`（不启用）

**条件编译**（实测 `storage/mod.rs:20-21, 26-27`）：



```
\#\[cfg(feature = "rustfs\_ecstore\_backend")]

pub mod rustfs\_ecstore;

\#\[cfg(feature = "rustfs\_ecstore\_backend")]

pub use rustfs\_ecstore::RustFsEcstoreBackend;
```

**lib.rs 导出**（实测 `lib.rs:84-85`）：



```
\#\[cfg(feature = "rustfs\_ecstore\_backend")]

pub use storage::RustFsEcstoreBackend;
```

**启用方式**：



```
cargo build -p mox-cloud-s3-svc --features rustfs\_ecstore\_backend

cargo test -p mox-cloud-s3-svc --features rustfs\_ecstore\_backend
```

### 7.4 接入点说明



| 接入点                             | 状态     | 说明                                     |
| ------------------------------- | ------ | -------------------------------------- |
| trait 实现签名                      | ✅ 已定义  | 8 个方法签名完整                              |
| 构造函数 `new(endpoint, pool_name)` | ✅ 已定义  | 参数类型明确                                 |
| `is_available()` 探测             | ✅ 已定义  | 骨架返回 false                             |
| feature flag 编译路径               | ✅ 已验证  | 默认 false / 启用后均编译通过                    |
| 单元测试                            | ✅ 已验证  | 4 个测试（构造 / Unsupported / 元数据 / Debug）  |
| 实际数据面操作                         | ⚠️ 待实现 | 全部返回 Unsupported                       |
| FFI / 进程通信                      | ⏳ 阶段五  | Unix Socket /gRPC 对接 RustFS ecstore 进程 |

### 7.5 后续对接计划（阶段五）



1. **通信层**：通过 Unix Socket 或 gRPC 与独立部署的 RustFS ecstore 进程通信

2. **写入路径**：EC 编码分片 → 写入多节点 data+parity shards → 返回 chunk 元信息（含 EC profile）

3. **读取路径**：从多节点读取 data+parity shards → EC 解码重建 → 返回完整数据

4. **删除路径**：通知 ecstore 回收所有关联 shards

5. **元数据管理**：EC profile、replica 位置、chunk 健康状态



***

## 8. P1 增强设计

### 8.1 PooledBuffer 推广到 s3-svc /filer-svc

#### 8.1.1 背景

阶段三在 volume-svc 内实现了 PooledBuffer 四层分档缓冲池，但 s3-svc 和 filer-svc 的热点路径仍使用 `Vec::with_capacity()` 重复分配内存，导致：



* 频繁的堆分配 / 释放开销

* 内存碎片

* 大对象（MB 级）分配延迟高

#### 8.1.2 s3-svc 接入

**S3Server 字段**（实测 `s3_server.rs`）：



```
pub struct S3Server {

&#x20;   // ...

&#x20;   buffer\_pool: Arc\<BufferPool>,

}
```

**热点路径使用**：



* `op_put_object`：使用 `buffer_pool.acquire(size)` 获取 PooledBuffer 接收上传数据

* `op_get_object`：使用 PooledBuffer 暂存读取的数据

* `op_copy_object`：使用 PooledBuffer 中转复制数据

* `op_upload_part_copy`：使用 PooledBuffer 暂存分片数据

#### 8.1.3 filer-svc 接入

**FilerServer 字段**（实测 `filer_server.rs:39-40`）：



```
pub struct FilerServer {

&#x20;   // ...

&#x20;   /// 四层分档缓冲池（PooledBuffer 推广，热点路径复用分配）

&#x20;   pub buffer\_pool: Arc\<BufferPool>,

}
```

**InMemoryObjectStorage 字段**（实测 `filer_server.rs:151-152`）：



```
pub struct InMemoryObjectStorage {

&#x20;   // ...

&#x20;   buffer\_pool: Arc\<BufferPool>,

}
```

**S3ObjectStorage 字段**（实测 `filer_server.rs:278-279`）：



```
pub struct S3ObjectStorage {

&#x20;   // ...

&#x20;   buffer\_pool: Arc\<BufferPool>,

}
```

**热点路径使用**（实测 `filer_server.rs:206-207, 307-308`）：



```
// 写入路径

let mut pooled = self.buffer\_pool.acquire(data.len());

pooled.as\_mut().copy\_from\_slice(data);

// ... 使用 pooled ...

// RAII：pooled drop 时自动归还缓冲池

// 读取路径

let mut pooled = self.buffer\_pool.acquire(size);

// ... 读入 pooled ...
```

**Builder 方法**（实测 `filer_server.rs:291`）：



```
pub fn with\_buffer\_pool(buffer\_pool: Arc\<BufferPool>) -> Self
```

#### 8.1.4 四层分档规格



| 档位     | 大小范围       | 典型用途         |
| ------ | ---------- | ------------ |
| Tier 0 | 64B - 4KB  | 小对象、元数据、ETag |
| Tier 1 | 4KB - 64KB | 中等对象、MPU 分片  |
| Tier 2 | 64KB - 1MB | 大对象、批量操作     |
| Tier 3 | 1MB - 16MB | 超大对象、EC 编码块  |

**RAII 自动归还**：`PooledBuffer` 实现 `Drop`，离开作用域时自动将底层 `Vec<u8>` 归还对应档位的缓冲池，无需手动调用 release。

### 8.2 ReaderPipeline 接入 S3 读路径

#### 8.2.1 背景

阶段三在 volume-svc 内定义了 `ReaderCapability` trait 和 `ReaderPipeline` 组合式读管线，但 S3 GetObject 路径无法利用 hedged read（多后端并发取最快）。阶段四将 ReaderPipeline 接入 S3 读路径。

#### 8.2.2 StorageBackendReader：trait 适配层

**文件**：`s3-svc/src/storage/reader_pipeline.rs`

**定位**：包装单个 `StorageBackend`，实现 `ReaderCapability` trait，将 `get_chunk(chunk_id)` 适配为 `read_shard(shard_index)`。

**结构体**（实测）：



```
pub struct StorageBackendReader {

&#x20;   backend: Arc\<dyn StorageBackend>,

&#x20;   chunk\_id: String,

&#x20;   endpoint\_label: String,

}
```

**ReaderCapability trait 实现**（实测）：



| 方法                        | 实现                                                                                                             |
| ------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `read_shard(shard_index)` | 忽略 shard\_index（S3 对象读无分片概念），调用 `backend.get_chunk(&ChunkId::new(&self.chunk_id))`                             |
| `read_cost()`             | 根据 `backend.backend_type()` 映射：LocalFs/InMemory→Local，S3Compatible→Remote，RustFsEcstore→SameNode，Other→Unknown |
| `endpoint()`              | 返回 `"{backend_type}-{chunk_id}"` 标签                                                                            |
| `supports_hedged_read()`  | 返回 `true`（StorageBackendReader 支持 hedged read）                                                                 |

#### 8.2.3 S3ReaderPipeline：组合式管线封装

**结构体**（实测）：



```
pub struct S3ReaderPipeline {

&#x20;   backends: Vec\<Arc\<dyn StorageBackend>>,

}
```

**构造方法**：



* `empty() -> Self`：创建空管线

* `new(backends: Vec<Arc<dyn StorageBackend>>) -> Self`：创建包含指定后端的管线

* `with_backend(backend: Arc<dyn StorageBackend>) -> Self`：Builder 链式添加后端

**核心方法**：



| 方法                                                              | 用途                                                                                  |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `read_object(chunk_id) -> Result<Vec<u8>, ReadCapabilityError>` | 为每个后端创建 StorageBackendReader，构建 ReaderPipeline，调用 `read_first_success(0)` 并发取最快成功结果 |
| `read_object_sequential(chunk_id)`                              | 顺序读取（按 locality 成本排序后逐个尝试，用于调试对比）                                                   |
| `backend_count() -> usize`                                      | 获取后端数量                                                                              |

**并发取最快逻辑**（实测 `reader_pipeline.rs:139-159`）：



1. 检查 backends 非空，空则返回 `AllFailed(0)`

2. 为每个后端创建 `StorageBackendReader::new(backend.clone(), chunk_id)`

3. 构建 `ReaderPipeline`，逐个添加 reader

4. 调用 `pipeline.read_first_success(0)` 并发发起读请求

5. 第一个 `Ok` 立即返回（hedged read）

6. 全部失败返回 `ReadCapabilityError::AllFailed`

#### 8.2.4 S3Server 集成

**字段**（实测 `s3_server.rs:55`）：



```
reader\_pipeline: Option\<Arc\<crate::storage::reader\_pipeline::S3ReaderPipeline>>,
```

**Builder**（实测 `s3_server.rs:306-309`）：



```
/// 默认 None，op\_get\_object 走单后端 storage\_backend.get\_chunk()。

pub fn with\_reader\_pipeline(mut self, pipeline: Arc\<S3ReaderPipeline>) -> Self {

&#x20;   self.reader\_pipeline = Some(pipeline);

&#x20;   self

}
```

**op\_get\_object 路由逻辑**（实测）：



```
// 优先通过 reader\_pipeline 读取（hedged read，多后端取最快）

if let Some(pipeline) = \&state.reader\_pipeline {

&#x20;   if pipeline.backend\_count() > 0 {

&#x20;       data = pipeline.read\_object(\&chunk\_id).await?;

&#x20;   }

}

// 回退：单后端 storage\_backend.get\_chunk()

if data.is\_empty() && !chunk\_id.is\_empty() {

&#x20;   data = read\_object\_data(\&state.storage\_backend, \&chunk\_id).await?;

}
```

**可选启用**：`reader_pipeline` 默认为 `None`，不启用 hedged read；通过 `with_reader_pipeline()` Builder 显式启用。阶段五生产验证后可考虑默认启用。



***

## 9. 依赖关系与循环依赖保证

### 9.1 依赖方向图



```
┌─────────────────────────────────────────────────────────────────┐

│                        svc crates                                │

│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌──────────┐ │

│  │  s3-svc     │ │ volume-svc  │ │ filer-svc   │ │master/   │ │

│  │             │ │             │ │             │ │rebalance │ │

│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘ └────┬─────┘ │

│         │               │               │              │        │

│         └───────────────┴───────┬───────┴──────────────┘        │

│                                   │                                 │

│                    ┌──────────────┴──────────────┐                │

│                    │                             │                │

│              ┌─────▼─────┐               ┌─────▼─────┐          │

│              │  mox-     │               │  mox-     │          │

│              │  cloud-   │               │  cloud-   │          │

│              │  kernel   │               │  domain-  │          │

│              │  (L5)     │               │  traits   │          │

│              └─────┬─────┘               │  (L4/L6)  │          │

│                    │                       └─────┬─────┘          │

│                    │                             │                │

│              零业务依赖                     零 svc 依赖            │

│              (仅外部crates)                (仅外部crates)          │

└────────────────────┼─────────────────────────────┼────────────────┘

&#x20;                    │                             │

&#x20;                    ▼                             ▼

&#x20;             外部 crates                    外部 crates

&#x20;             tokio/async-trait/            async-trait/thiserror/

&#x20;             futures/thiserror/            serde/bytes/tokio/

&#x20;             parking\_lot/serde/            parking\_lot

&#x20;             bytes/rand/tracing
```

### 9.2 依赖矩阵



| 依赖方 →             | kernel | domain-traits | s3-svc | volume-svc | filer-svc | master-svc | rebalance-svc |
| ----------------- | ------ | ------------- | ------ | ---------- | --------- | ---------- | ------------- |
| **kernel**        | —      | ❌ 无           | ❌ 无    | ❌ 无        | ❌ 无       | ❌ 无        | ❌ 无           |
| **domain-traits** | ❌ 无    | —             | ❌ 无    | ❌ 无        | ❌ 无       | ❌ 无        | ❌ 无           |
| **s3-svc**        | ✅ 依赖   | ✅ 依赖          | —      | ❌ 已移除      | ❌ 无       | ✅ 依赖       | ❌ 无           |
| **volume-svc**    | ✅ 依赖   | ✅ 依赖          | ❌ 无    | —          | ❌ 无       | ❌ 无        | ❌ 无           |
| **filer-svc**     | ✅ 依赖   | ✅ 依赖          | ❌ 无    | ❌ 无        | —         | ❌ 无        | ❌ 无           |
| **master-svc**    | ❌ 无    | ❌ 无           | ❌ 无    | ❌ 无        | ❌ 无       | —          | ❌ 无           |
| **rebalance-svc** | ❌ 无    | ❌ 无           | ❌ 无    | ❌ 无        | ❌ 无       | ❌ 无        | —             |

### 9.3 循环依赖检测结果



| 检测项                    | 结果    | 说明                          |
| ---------------------- | ----- | --------------------------- |
| kernel → domain-traits | ✅ 无依赖 | 算法层不感知业务 trait              |
| domain-traits → kernel | ✅ 无依赖 | 契约层不感知算法实现                  |
| s3-svc → volume-svc    | ✅ 已移除 | 死依赖已清除                      |
| volume-svc → s3-svc    | ✅ 无依赖 | volume 不依赖 s3               |
| svc → svc 循环           | ✅ 无循环 | 各 svc 之间无相互依赖               |
| 全 workspace 循环依赖       | ✅ 无循环 | `cargo tree --workspace` 无环 |

**关键保证**：kernel 与 domain-traits 是两个**独立的叶子 crate**，无相互依赖，所有 svc crate 同时依赖两者但不在两者之间建立依赖。这确保了：



* 算法变更（kernel）不触发 trait 契约层（domain-traits）重编译

* trait 变更（domain-traits）不触发算法层（kernel）重编译

* 两者可独立版本化、独立发布



***

## 10. API 兼容性验证

### 10.1 公共 API 签名不变



| API 类别                               | 解耦前  | 解耦后            | 变更    |
| ------------------------------------ | ---- | -------------- | ----- |
| `S3Server::new(port, master)`        | 2 参数 | 2 参数           | ❌ 无变更 |
| `VolumeServer::new(...)`             | 原签名  | 原签名            | ❌ 无变更 |
| `FilerServer::new(...)`              | 原签名  | 原签名            | ❌ 无变更 |
| `ReedSolomonEngine::encode(...)`     | 原签名  | 原签名（re-export） | ❌ 无变更 |
| `BufferPool::acquire(size)`          | 原签名  | 原签名（re-export） | ❌ 无变更 |
| `BackpressureMonitor::try_acquire()` | 原签名  | 原签名（re-export） | ❌ 无变更 |
| `HedgedReader::read(...)`            | 原签名  | 原签名（re-export） | ❌ 无变更 |
| `MultiWriter::write(...)`            | 原签名  | 原签名（re-export） | ❌ 无变更 |
| `ScanBudget::new(...)`               | 原签名  | 原签名（re-export） | ❌ 无变更 |

### 10.2 新功能通过 Builder 方法和 Option 字段可选启用



| 新功能                        | 启用方式                                            | 默认状态     | 不启用时行为                          |
| -------------------------- | ----------------------------------------------- | -------- | ------------------------------- |
| 自定义存储后端                    | `S3Server::with_storage_backend(backend)`       | InMemory | 使用默认内存后端                        |
| ReaderPipeline hedged read | `S3Server::with_reader_pipeline(pipeline)`      | None     | 走单后端 `get_chunk()`              |
| RustFS ecstore 后端          | `--features rustfs_ecstore_backend`             | false    | 模块不编译                           |
| filer 自定义缓冲池               | `InMemoryObjectStorage::with_buffer_pool(pool)` | 默认池      | 使用 `BufferPool::with_default()` |

**设计原则**：所有新功能均为**可选启用**，默认行为与阶段三完全一致，确保零回归。

### 10.3 re-export 模块路径验证



| 原有路径                                                      | 解耦后路径                                                                       | 验证结果   |
| --------------------------------------------------------- | --------------------------------------------------------------------------- | ------ |
| `mox_cloud_volume_svc::reed_solomon::ReedSolomonEngine`   | `mox_cloud_volume_svc::reed_solomon::ReedSolomonEngine`（re-export 自 kernel） | ✅ 路径不变 |
| `mox_cloud_volume_svc::buffer_pool::BufferPool`           | 同上模式                                                                        | ✅ 路径不变 |
| `mox_cloud_volume_svc::backpressure::BackpressureMonitor` | 同上模式                                                                        | ✅ 路径不变 |
| `mox_cloud_volume_svc::hedged_reader::HedgedReader`       | 同上模式                                                                        | ✅ 路径不变 |
| `mox_cloud_volume_svc::multi_writer::MultiWriter`         | 同上模式                                                                        | ✅ 路径不变 |
| `mox_cloud_volume_svc::reader_capability::ReaderPipeline` | 同上模式                                                                        | ✅ 路径不变 |
| `mox_cloud_volume_svc::scanner::ScanBudget`               | 同上模式                                                                        | ✅ 路径不变 |
| `mox_cloud_s3_svc::scanner::ScanBudget`                   | `mox_cloud_s3_svc::scanner::ScanBudget`（re-export 自 kernel）                 | ✅ 路径不变 |
| `mox_cloud_s3_svc::InMemoryStorageBackend`                | `mox_cloud_s3_svc::InMemoryStorageBackend`（新增）                              | ✅ 新增导出 |

**新增可用路径**（kernel 直接引用）：



```
use mox\_cloud\_kernel::reed\_solomon::ReedSolomonEngine;

use mox\_cloud\_kernel::buffer\_pool::{BufferPool, PooledBuffer};

use mox\_cloud\_kernel::backpressure::BackpressureMonitor;

// ... 等 10 个模块
```

### 10.4 可见性提升说明

抽离到 kernel crate 后，原 volume-svc 内 6 个 `pub(crate)` 项需提升为 `pub`（因为 kernel crate 边界需要跨 crate 可见）：



| 原项                       | 原可见性       | 新可见性 | 用途                 |
| ------------------------ | ---------- | ---- | ------------------ |
| `BackpressureConfig` 字段  | pub(crate) | pub  | 跨 crate 配置构造       |
| `BufferTierConfig` 字段    | pub(crate) | pub  | 跨 crate 分档配置       |
| `ShardReadCost` 变体       | pub(crate) | pub  | 跨 crate 成本匹配       |
| `WriteProgressPolicy` 变体 | pub(crate) | pub  | 跨 crate 策略选择       |
| `EcProfile` 字段           | pub(crate) | pub  | 跨 crate profile 构造 |
| `ScanBudgetTracker` 方法   | pub(crate) | pub  | 跨 crate 预算追踪       |

**影响**：这些项原本仅在 volume-svc 内可见，提升为 pub 后对整个 workspace 可见。由于它们是纯数据结构 / 配置类型，无安全敏感内容，提升可见性无风险。



***

## 11. 阶段五路线图建议

基于阶段四架构解耦成果，建议阶段五（数据面对接与性能优化）按以下优先级推进：

### 11.1 P0（必须）



| # | 任务                               | 说明                                                                                                                     | 依赖            |
| - | -------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------------- |
| 1 | **RustFS ecstore 实际 FFI / 进程对接** | 实现 `RustFsEcstoreBackend` 的 put\_chunk/get\_chunk/delete\_chunk 数据面操作，通过 Unix Socket 或 gRPC 与独立部署的 RustFS ecstore 进程通信 | 阶段四骨架已就绪      |
| 2 | **LocalFsStorageBackend 完整实现**   | 基于本地文件系统实现 StorageBackend trait，支持 chunk 级文件存取、目录分片、fsync 持久化                                                          | 阶段四 trait 已定义 |
| 3 | **MetaStorage trait 落地实现**       | 将 s3-svc/filer-svc 现有元数据逻辑适配为 MetaStorage trait 实现，统一元数据存取路径                                                           | 阶段四 trait 已定义 |

### 11.2 P1（重要）



| # | 任务                                         | 说明                                                                                             |
| - | ------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| 4 | **ReaderPipeline 默认启用 + hedged read 生产验证** | 在多后端部署场景下默认启用 ReaderPipeline，验证 hedged read 的尾延迟优化效果                                           |
| 5 | **PooledBuffer 全路径推广**                     | 将 PooledBuffer 推广到 master-svc（快照 / 日志）、rebalance-svc（数据迁移）等剩余热点路径                              |
| 6 | **CAS 背压扩展到 read\_chunk 和批量操作**            | 阶段三仅接入 write\_chunk，阶段五扩展到读路径和批量删除 / 复制                                                        |
| 7 | **ShardReader/ShardWriter trait 落地实现**     | 将 volume-svc 现有 hedged\_reader/multi\_writer 适配为 domain-traits 的 ShardReader/ShardWriter trait |

### 11.3 P2（优化）



| #  | 任务           | 说明                                                                         |
| -- | ------------ | -------------------------------------------------------------------------- |
| 8  | **性能基准测试**   | 建立标准化性能基准套件：吞吐 / 延迟 / CPU / 内存，对比 InMemory vs LocalFs vs RustFs ecstore 后端 |
| 9  | **零拷贝读路径优化** | 利用 StorageBackend capabilities 的 `supports_range_read`，实现范围读零拷贝            |
| 10 | **异步生态升级**   | 评估从 `async-trait` 迁移到原生 async fn in trait（Rust 1.75+），消除 Box 开销            |

### 11.4 阶段四→阶段五衔接

阶段四完成了**架构解耦**（算法独立化、trait 集中化、s3 解耦、可选后端接入点），为阶段五的**数据面对接**奠定了基础：



* `RustFsEcstoreBackend` 骨架已定义 trait 实现签名，阶段五只需填充数据面逻辑

* `StorageBackend` trait 已验证 object-safe，可动态分发任意后端

* `S3Server` 已支持依赖注入，切换后端无需修改 S3 协议层

* `mox-cloud-kernel` 已独立，算法优化不影响业务 crate



***

**文档版本**：v1.0 ｜ **发布日期**：2026-09-03 ｜ **权威等级**：🟢 ADR

**文档编号**：ADR-MOXFS-P4-20260903

**项目**：moxfs 全自研云盘知识库 ｜ RustFS 仅为对标参考对象（Apache 2.0，只读）

**代码实测基准**：`platform/domains/cloud/` 7 crate 全量编译 + 测试通过（1139 测试，0 失败）