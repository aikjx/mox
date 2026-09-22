# 混合架构（路线 A）整合方案：自研云盘控制面 × RustFS 数据面参考架构设计

| 字段 | 值 |
|---|---|
| **文档标题** | 混合架构（路线 A）整合方案：自研云盘控制面 × RustFS 数据面参考架构设计 |
| **版本** | v1.0 |
| **权威等级** | 开发专家联盟 · 架构决策级（Architecture Decision Record, ADR） |
| **日期** | 2026-09-02 |
| **文档编号** | ADR-CLOUD-HYBRID-A-20260902 |
| **适用范围** | `platform/domains/cloud/` 全模块；`ais/RustFS/` 仅作只读参考 |
| **密级** | 内部 · 研发可见 |

---

## 2. 执行摘要（混合架构核心决策）

### 2.1 核心立场

本方案确立**路线 A（混合架构）**为璇玑云盘的唯一演进路径：

> **保留自研云盘知识库控制面（`platform/domains/cloud/`）的全部架构主权，将 RustFS（`ais/RustFS/`）定位为数据面/底层算法的参考源，通过"算法吸收 + 架构模式借鉴 + 可选 L7 trait 对接"三分类策略实现融合，绝不整体替换或直接 copy 代码。**

### 2.2 三大核心发现

1. **自研纠删码实现已超越 RustFS 对应能力**：自研 `gf256_simd.rs` 采用自研 16 子表 LUT 级联 AVX2/NEON 双架构实现，配合 `reed_solomon.rs` 的 Vandermonde+Gauss-Jordan 全自研引擎与 `PathChoice::Auto` 运行时微基准决策机制，在架构自洽性与跨架构可移植性上优于 RustFS 对外部 `reed-solomon-erasure` crate 的直接依赖。**结论：纠删码内核【自研保留】，仅吸收 RustFS 的写仲裁/读仲裁/一致性校验等外围算法模式。**

2. **RustFS 在 I/O 管线、缓冲背压、扫描预算、自愈调度四个维度有成熟的工程化模式可直接借鉴**：`rio` 的组合式 reader 管线（能力探测 trait + 宏透传）、`io-core` 的 CAS 信号量背压 + 四层分档缓冲池 RAII、`scanner` 的三维预算（时长/对象数/目录数）+ CancellationToken 子令牌取消、`heal` 的 owned 初始化（取消安全）+ MRF 队列消费 + 幸存者盘替换恢复记录。这些模式与自研 AIS 7 层模型的 L5/L6/L7 高度契合，应【借鉴吸收】。

3. **RustFS 的 `storage-api` trait 分层设计是 L5 Domain Traits 的优秀参照**：其将对象存储抽象拆分为 `ObjectIO` / `ObjectOperations` / `ListOperations` / `MultipartOperations` / `HealOperations` / `NamespaceLocking` 六个正交 trait，配合泛型关联类型实现后端解耦。自研当前 L5 仅有 `ObjectStorageProvider` 等粗粒度 trait，应【借鉴吸收】其正交拆分方法论。

### 2.3 决策矩阵总览

| 分类 | 数量 | 代表项 |
|---|---|---|
| **【自研保留】** | 12 | 纠删码 GF(2^8) SIMD 内核、ReedSolomonEngine、Raft Master、五 svc 拓扑、AIS 7 层模型、审计 hash_chain |
| **【借鉴吸收】** | 18 | 写仲裁 WriteProgressPolicy、读仲裁 hedge+locality、CAS 背压信号量、四层缓冲池、三维扫描预算、组合式 reader 管线、正交 trait 拆分、owned 初始化模式、dirty-scope generation 快路径 |
| **【对接集成】** | 5 | RustFS ecstore 作为可选 L7 EC 后端、rio 作为可选 I/O 管线后端、heal 作为可选自愈引擎、storage-api 作为 trait 对齐参考、lifecycle 作为策略引擎参考 |

---

## 3. RustFS 架构算法深度分析

### 3.1 ecstore — 纠删码存储引擎

**源码位置**：`ais/RustFS/crates/ecstore/src/erasure/coding/{erasure.rs, encode.rs, decode.rs}`

#### 3.1.1 双后端编码器架构

RustFS ecstore 采用**现代/遗留双后端**设计：

- **现代后端**：`reed_solomon_erasure::galois_8::ReedSolomon`（GF(2^8)，最大总 shard 数 = Field::ORDER = 256）
- **遗留后端**：`reed_solomon_simd`（仅当 `uses_legacy=true` 时启用，用于兼容读/修复旧版文件；legacy shard_size 公式为 `(div_ceil + 1) & !1`，对齐 MinIO 兼容语义）

**编码器进程级缓存**：`MODERN_REED_SOLOMON_CACHE`（64 entries）/ `LEGACY_REED_SOLOMON_CACHE`（16 entries），键为 `(data_shards, parity_shards)`，采用 `OnceLock + RwLock<HashMap<(u16,u16), Arc<ReedSolomon>>>` 实现零开销复用。

**shard_size 公式**：
- 现代：`block_size.div_ceil(data_shards)`
- 遗留：`(block_size.div_ceil(data_shards) + 1) & !1`

#### 3.1.2 EncodedBlock 零拷贝设计

`EncodedBlock` 采用**单块连续 backing buffer + shard_size 字段**设计，测试断言 `size_of::<EncodedBlock>() == size_of::<Bytes>() + size_of::<usize>()`。每个队列条目仅持有一个 backing buffer 句柄，`shards()` 方法通过 `chunks_exact` 切片视图暴露各 shard，避免 per-shard 独立分配。

**三条零拷贝编码路径**：
1. `encode_data`：借切片拷贝（最通用）
2. `encode_data_owned`：`Vec<u8> → Bytes → try_into_mut`，当 refcount==1 时零拷贝原地编码
3. `encode_data_bytes_mut`：调用方预预留 `shard_size * total_shards` 容量后 `resize` 不重分配；容量不变式依赖 shard_size 公式对 `data_len` 的单调性

#### 3.1.3 写仲裁（MultiWriter）

流式编码管线采用 **producer ∥ consumer** 模型：producer 按 `block_size` 切块 → 编码 → mpsc 发送；consumer 通过 `MultiWriter` 并发写全部 shard。

- **通道容量**：`max_inflight_bytes`（默认 32MB）/ `expanded_block_bytes`，clamp 到 `[1, 32]` 块
- **批处理**：`batch=4 blocks`
- **取消安全**：`AbortOnDropTask` 保证取消/写失败时 abort producer 防泄漏
- **写仲裁规则**：每块并发写所有存活 shard writer（`FuturesUnordered`），成功数 ≥ `write_quorum` 即通过；失败 writer 立即置 `None` 在 commit 前剔除
- **WriteProgressPolicy**（防 black-hole peer）：
  - `stall_timeout`：默认开启，按块 re-arm，只约束 stall 不约束总时长
  - `absolute_cap`：默认关闭，防 slow-drip peer
  - 来源：backlog#1319
- **运行时策略**：`MultiThread` runtime 内联执行 EC 编码（~110µs/1MiB block，p99 ~542µs，`block_in_place` 反而更贵，backlog#932）；`CurrentThread` 用 `spawn_blocking`
- **inflight 计量**：`QueuedInflightBytes` RAII gauge 精确计量 inflight 字节，所有退出路径 settle

#### 3.1.4 读仲裁（ParallelReader）

- **读仲裁规则**：每 stripe 只发 `data_shards` 个读（locality 偏好排序），成功满 `data_shards` 即停；失败时从备用 reader（parity）补位
- **Hedge**：`shard_read_hedge_delay = min(read_timeout, 100ms)`，straggler 超时后若仲裁可满足则放弃等待（backlog#1156 首字节延迟优化）
- **locality 感知调度**：`RUSTFS_SHARD_LOCALITY_SCHEDULING` 三态（off/observe/on），`ShardReadCost` 按 `Local/SameNode/Remote/Unknown` 排序
- **lockstep 模式**（`verify_reconstruction=true`）：所有 engaged reader 每 stripe 严格读一块保持互相对齐，修复 backlog#832 desync：parity 中途接入必须靠 `DeferredReaderStripeHandle::advance_stripes` 对齐到当前 stripe
- **data-shards-only 门控**（backlog#923）：healthy 对象每 stripe 只读 data shards（省一半读 IOPS/bitrot 哈希），parity 保持未打开 deferred reader，数据 shard 中途死亡才按 stripe 对齐接入
- **depth-1 stripe 预取**（backlog#930 HP-9，默认关闭）：`prefetch_count>1` 或 bitrot-decode overlap 开启时，下一 stripe 读与当前 stripe 重建/发射重叠；cancel-safety 用 `biased select`，emit Stop 时立即 drop 读 future（整个管线结构化 async 无 spawn，drop 即真正取消）

#### 3.1.5 一致性校验

- `verify_data_and_parity`：重算 parity 比对
- `decode_data_with_reconstruction_verification`：当可用 shard > data_shards 时，用冗余 parity 重建后逐块比对，不一致 **fail-closed** 返回 `InvalidData "inconsistent read source shards"`
- `has_valid_dimensions()`：防御损坏元数据导致的除零

#### 3.1.6 分层存储与条带布局

ecstore 内部按 **pool → set → disk** 三级拓扑组织（`ais/RustFS/crates/ecstore/src/layout/{disks_layout.rs, set_layout.rs, pool_space.rs}`），EC 条带跨 set 内 disk 分布。分层索引机制通过 `set_disk/ops/` 下的 `object.rs` / `multipart.rs` / `heal.rs` / `bitrot_self_verify.rs` 实现。冷热分层通过 `services/tier/` 模块（含 10+ 云后端适配器）实现，与 lifecycle transition 联动。

---

### 3.2 rio — I/O 管线

**源码位置**：`ais/RustFS/crates/rio/src/lib.rs` 及同目录 `{encrypt_reader, compress_reader, hash_reader, etag_reader, checksum, limit_reader, hardlimit_reader, http_reader, reader, writer, errors}.rs`

#### 3.2.1 组合式 Reader 管线

`rio` 的核心抽象是 **Reader trait = ReadStream（AsyncRead + Unpin + Send + Sync）+ ReaderCapabilities**，其中 `ReaderCapabilities` 是三个能力探测 trait 的组合：

- `EtagResolvable`：可解析 ETag
- `HashReaderDetector`：可检测哈希 reader
- `TryGetIndex`：可尝试获取索引

`DynReader = Box<dyn Reader>` 作为类型擦除后的句柄。`WarpReader` 将裸 `ReadStream` 包装为 `DynReader`。

#### 3.2.2 能力透传宏

`delegate_reader_capabilities!` 宏为包装 reader 递归透传能力，确保管线中任意一层包装后，底层能力（如 ETag 可解析性）不丢失。这是组合式管线的关键工程保障。

#### 3.2.3 管线阶段

PUT 数据流的 reader 管线阶段为：`encrypt_reader → compress_reader → hash_reader → etag_reader → checksum → limit_reader → hardlimit_reader → http_reader`。每阶段是一个独立的 reader 包装器，通过组合而非继承实现功能叠加。

#### 3.2.4 架构价值

这种**能力探测 + 宏透传 + 组合式包装**的模式，使得 I/O 管线可以在运行时根据底层 reader 的能力动态优化（如已知 ETag 时跳过哈希计算），同时保持类型安全。

---

### 3.3 io-core — 缓冲池与背压

**源码位置**：`ais/RustFS/crates/io-core/src/{backpressure.rs, pool.rs}`

#### 3.3.1 BackpressureMonitor — CAS 信号量式准入控制

- **默认参数**：`max_concurrent=32`、`high_water=0.8`（阈值 25）、`low_water=0.5`（阈值 16）、`cooldown=100ms`
- **状态机**：`Normal / Warning / Critical` 三态
- **try_acquire**：用 CAS 循环保证不超 `max_concurrent`，满则拒绝并计数（`rejection_rate` 指标）
- **release**：用 `fetch_update` + `checked_sub` 防下溢——未配对 release 不得把 `current` 卷回 `usize::MAX` 导致永久拒绝
- **设计要点**：纯原子操作无锁，高并发下零竞争；cooldown 防止抖动导致的频繁状态切换

#### 3.3.2 四层分档缓冲池

| 档位 | 大小 | 最大缓存数 |
|---|---|---|
| Small | 4 KB | 1000 |
| Medium | 64 KB | 500 |
| Large | 512 KB | 100 |
| XLarge | 4 MB | 25 |

- **并发控制**：`Semaphore` 控制每档并发上限
- **复用队列**：`Mutex<Vec<BytesMut>>` 存储可复用缓冲
- **RAII 归还**：`PooledBuffer` 持 `OwnedSemaphorePermit + ManuallyDrop<BytesMut>`，`Drop` 自动归还（tier 为空则直接释放）
- **选档**：`select_tier` 按请求 size 选档
- **增长**：容量不足的复用缓冲 `reserve` 增长并记账
- **指标**：`available_buffers` gauge（backlog#806 修正：复用取出时 `fetch_sub`）、`hit_rate/tier` 命中率、`allocated_bytes`

#### 3.3.3 架构价值

四层分档设计覆盖了从元数据操作（4KB）到大块 EC 编码（4MB）的全场景；Semaphore 防止单档耗尽导致全局饥饿；RAII 自动归还消除泄漏风险。

---

### 3.4 lifecycle — 对象生命周期管理

**源码位置**：`ais/RustFS/crates/lifecycle/src/{core.rs, evaluator.rs, rule.rs, object_lock.rs, tagging.rs}`

#### 3.4.1 Evaluator 策略评估器

`Evaluator` 持有 `policy: Arc<BucketLifecycleConfiguration>` 和可选 `lock_retention: Option<Arc<ObjectLockConfiguration>>`，对对象版本列表执行生命周期策略评估。

**核心评估逻辑**（`eval_inner`）：
1. 遍历对象版本列表，对每个版本调用 `policy.eval_inner(obj, now, newer_noncurrent_versions)`
2. **复制等待门控**：若 `lifecycle_action_waits_for_replication(event.action)` 且对象处于 `Pending/Failed` 复制状态，则事件降级为 `NoneAction`
3. **DeleteAllVersions 短路**：若任一版本命中 `DeleteAllVersionsAction` 且桶未启用 object lock、无版本被锁、无版本复制 pending，则执行全版本删除并 `break 'top_loop` 跳过剩余版本评估
4. **Object Lock 防护**：对 `DeleteAction/DeleteVersionAction` 等，检查 `is_object_locked(obj)`，锁定则降级
5. **noncurrent 版本计数**：`newer_noncurrent_versions` 计数器用于 `NoncurrentVersionExpiration` 的 `NewerNoncurrentVersions` 规则

#### 3.4.2 动作枚举

`IlmAction` 涵盖：`NoneAction / DeleteAction / DeleteVersionAction / DeleteAllVersionsAction / DelMarkerDeleteAllVersionsAction / DeleteRestoredAction / DeleteRestoredVersionAction / TransitionAction / TransitionVersionAction` 等。

#### 3.4.3 分层迁移触发

lifecycle 的 `TransitionAction` 与 ecstore 的 `services/tier/` 模块联动，通过 `tier_sweeper.rs` / `tier_delete_journal.rs` / `tier_free_version_recovery.rs` 等实现冷热数据迁移的事务性保障。

---

### 3.5 scanner — 并发扫描器

**源码位置**：`ais/RustFS/crates/scanner/src/{scanner.rs, scanner_budget.rs, scanner_folder.rs, scanner_io.rs, sleeper.rs, runtime_config.rs, remote_scanner.rs}`

#### 3.5.1 ScannerCycleBudget — 三维预算控制

`ScannerCycleBudget` 实现**三维度扫描预算**，任一维度耗尽即通过 `CancellationToken` 子令牌取消当前扫描周期：

| 维度 | 配置字段 | 取消原因 |
|---|---|---|
| 运行时长 | `max_duration: Option<Duration>` | `Runtime` |
| 对象数 | `max_objects: Option<u64>` | `Objects` |
| 目录数 | `max_directories: Option<u64>` | `Directories` |

**实现机制**：
- 每个 budget 持有 `parent.child_token()`，超时通过 `tokio::spawn` + `tokio::select!` 监听
- 对象数/目录数通过 `AtomicU64` + `saturating_fetch_add`（CAS 循环）计数，达限后 `cancel_for_reason`（CAS 保证首次取消原因可观测）
- `reason: Arc<AtomicU8>` 记录首次取消原因，后续取消不覆盖
- `remaining_config()` 计算剩余预算，支持**断点续扫**：将剩余配置传递给下一周期
- `record_remote_progress()` 聚合远端扫描进度，支持分布式扫描
- `requires_serial_progress_accounting()`：当配置了对象数/目录数限制时，要求串行进度记账以保证计数准确
- `Drop` 时自动 `token.cancel()` 防泄漏

#### 3.5.2 限速与并发

`runtime_config.rs`（52KB）定义扫描器的运行时配置，包括并发度、限速参数。`sleeper.rs` 实现自适应休眠机制，根据系统负载动态调整扫描速率。

#### 3.5.3 架构价值

三维预算 + CancellationToken 子令牌 + 断点续扫的组合，使得全量扫描可以在不影响在线业务的前提下增量完成，且每个周期的预算消耗可观测、可恢复。

---

### 3.6 object-capacity — 容量管理

**源码位置**：`ais/RustFS/crates/object-capacity/src/{capacity_manager.rs, capacity_scope.rs, scan.rs, types.rs}`

#### 3.6.1 CapacityScope 注册表

`CapacityScope` 描述一次容量计算涉及的磁盘集合（`Vec<CapacityScopeDisk>`，每个 disk 含 `endpoint + drive_path`）。

**注册表机制**：
- `capacity_scope_registry()`：`OnceLock<Mutex<HashMap<Uuid, CapacityScopeEntry>>>`，按 token 存储 scope
- **软限制**：`CAPACITY_SCOPE_REGISTRY_SOFT_LIMIT = 2048`
- **硬限制**：`CAPACITY_SCOPE_REGISTRY_HARD_LIMIT = 4096`
- **TTL**：`CAPACITY_SCOPE_TTL = 300s`
- **淘汰策略**：达到软限时 `prune_expired_entries` + `enforce_hard_limit`（按 `recorded_at` LRU 淘汰最旧条目）
- **合并语义**：同 token 的非过期 scope 合并去重；过期 scope 直接替换（防止复活 stale disk，backlog#1022 #35）
- **Poison 恢复**：`lock().unwrap_or_else(|p| p.into_inner())` 处理 mutex poison

#### 3.6.2 Global Dirty Scope — Generation 快路径（backlog#1315）

这是 object-capacity 最精巧的性能优化：

- `global_dirty_scope_registry()`：`OnceLock<Mutex<HashSet<CapacityScopeDisk>>>`，存储需要刷新容量的脏磁盘
- `DIRTY_GENERATION: AtomicU64`：单调递增的脏注册表 generation
- **record_global_dirty_scope(scope) → u64**：记录脏磁盘并返回当前 generation。调用方缓存该 generation
- **快路径跳过**：后续写入时，若 `current_dirty_generation() == cached_generation`，说明自上次记录后无 drain 操作，磁盘仍在脏注册表中，**可完全跳过 registry mutex**
- **drain_global_dirty_scopes()**：排空脏注册表并 `DIRTY_GENERATION.fetch_add(1)`。空注册表不推进 generation（避免强制冗余 re-mark）
- **不变式**：generation 的加载和推进都在 registry mutex 保护下，保证观察到 `generation == g` 的 set 其磁盘在下次 drain 前一定存在
- **可观测性**：`GLOBAL_DIRTY_UPGRADE_COUNT` 记录 mutex 升级次数，稳态写入应保持不变

#### 3.6.3 架构价值

- CapacityScope 注册表实现了**分布式容量计算的 token 化**：写操作记录涉及的磁盘集合，后台扫描器按 token 取回 scope 执行容量刷新
- Dirty scope generation 快路径将稳态写入的 registry mutex 竞争降为零，仅在 drain 后首次写入时升级
- 软/硬限制 + TTL + LRU 淘汰防止注册表无限增长

---

### 3.7 heal — 自愈/修复

**源码位置**：`ais/RustFS/crates/heal/src/lib.rs` 及 `heal/` 子模块

#### 3.7.1 全局 HealRuntime

`heal` crate 采用**进程级单例 HealRuntime**设计：

- `GLOBAL_HEAL_RUNTIME: OnceCell<HealRuntime>`：持有 `manager: Arc<HealManager>` + `channel_processor: Arc<Mutex<HealChannelProcessor>>`
- `GLOBAL_HEAL_RUNTIME_INIT: Mutex<()>`：初始化互斥锁，防止并发初始化
- `GLOBAL_AHM_SERVICES_CANCEL_TOKEN: OnceLock<CancellationToken>`：全局取消令牌

#### 3.7.2 Owned 初始化模式（取消安全）

`init_heal_manager_with_workload_provider` 通过 `run_owned_initialization` 执行初始化：

```rust
async fn run_owned_initialization<T, F>(initialization: F) -> Result<T>
where T: Send + 'static, F: Future<Output = Result<T>> + Send + 'static
{
    tokio::spawn(initialization).await
        .map_err(|err| Error::Other(...))
}
```

**关键设计**：初始化在 `tokio::spawn` 的 owned task 中执行，调用方（HTTP/startup）的取消不会 abandon 已 spawn scheduler 的 manager。初始化失败时 `stop_initializing_manager` 保证资源回滚。

#### 3.7.3 MRF 队列消费

初始化完成后立即 `heal::mrf_queue::spawn_mrf_consumer(heal_manager.clone())`，启动 MRF（Most Recently Failed）意图消费者，处理错误路径修复意图 + 持久化 journal replay。

#### 3.7.4 HealChannelProcessor

`HealChannelProcessor` 从 `rustfs_common::heal_channel::init_heal_channels()` 获取接收端，处理 heal 通道消息。通道初始化失败时整个 runtime 回滚（manager.stop + 返回 Config error）。

#### 3.7.5 替换恢复记录（ReplacementRecovery）

- `ReplacementRecoveryRecord`：持久化在幸存者盘上的替换恢复记录，含 `task_id / state / generation / set_disk_id / target_slots`
- `current_replacement_recovery_snapshot()`：读取所有本地幸存者盘的记录，通过 `BTreeMap<String, ReplacementRecoveryRecord>` 按 task_id 聚合
- **冲突检测**：同一 task_id 的不同记录若状态不一致（非 Completed/CleanupPending 兼容对），标记为 `Unknown` 并记录原因
- `definitive` 标志：仅当所有记录一致且无读取错误时为 true

#### 3.7.6 HealStorageAPI 抽象

`heal::storage::HealStorageAPI` trait 定义自愈所需的存储操作：`get_object_meta / ec_decode_rebuild / get_bucket_info / list_buckets / object_exists / heal_object / heal_bucket / heal_format / list_objects_for_heal_page / get_disk_for_resume`。这是典型的 L5 trait 设计，将自愈逻辑与存储后端解耦。

---

### 3.8 replication — 复制管理

**源码位置**：`ais/RustFS/crates/replication/src/{config.rs, object.rs, operation.rs, queue.rs, resync.rs, runtime.rs, stats.rs, mrf.rs, filemeta.rs, delete.rs, multipart.rs, http.rs, rule.rs, tagging.rs, storage_api.rs}`

replication crate 是 RustFS 中规模最大的子系统之一（17 个源文件，`config.rs` 63KB / `filemeta.rs` 49KB / `mrf.rs` 38KB / `resync.rs` 47KB / `stats.rs` 30KB）。

**核心组件**：
- `ReplicationConfig`：桶级复制配置，含目标 ARN、规则、优先级
- `ReplicationState` / `ReplicationStatusType`：对象级复制状态（Pending/Completed/Failed/Empty）
- `VersionPurgeStatusType`：版本清除状态
- `replication_queue`：异步复制队列，支持批量提交
- `resync`：全量/增量重新同步引擎
- `mrf`：失败重试队列（Most Recently Failed）
- `stats`：复制统计与指标

**与 lifecycle 的联动**：`replication_status_blocks_lifecycle` 函数定义哪些复制状态会阻塞生命周期操作（Pending/Failed 阻塞 Delete/Transition 类动作）。

---

### 3.9 storage-api — 存储 API 抽象层

**源码位置**：`ais/RustFS/crates/storage-api/src/{object.rs, bucket.rs, admin.rs, capability.rs, error.rs, multipart.rs, observability.rs, replication.rs, topology.rs, snapshots/}`

#### 3.9.1 正交 Trait 分层

storage-api 将对象存储操作拆分为**六个正交 trait**：

| Trait | 职责 | 关键方法 |
|---|---|---|
| `ObjectIO` | 底层 I/O | `get_object_reader / put_object` |
| `ObjectOperations` | 对象操作 | `get_object_info / verify_object_integrity / copy_object / delete_object / delete_objects / put_object_metadata / get/put/delete_object_tags / transition_object / restore_transitioned_object` |
| `ListOperations` | 列表/遍历 | `list_objects_v2 / list_object_versions / walk` |
| `MultipartOperations` | 分段上传 | `list_multipart_uploads / new_multipart_upload / copy_object_part / put_object_part / get_multipart_info / list_object_parts / abort_multipart_upload / complete_multipart_upload` |
| `HealOperations` | 自愈 | `heal_format / heal_bucket / heal_object / get_pool_and_set / check_abandoned_parts` |
| `NamespaceLocking` | 命名空间锁 | `new_ns_lock` |

每个 trait 使用**泛型关联类型**（`type Error / type ObjectInfo / type ObjectOptions / ...`）实现后端解耦，`async_trait` 宏提供 async 方法支持。

#### 3.9.2 HTTP 前置条件

`ObjectPreconditionState` + `HTTPPreconditions` 实现 S3 兼容的条件请求：`If-Match / If-None-Match / If-Modified-Since / If-Unmodified-Since`，含 `etag_matches`（trim quotes + wildcard `*`）和 `is_modified_since`（unix_timestamp 比较）。

#### 3.9.3 WalkOptions

`WalkOptions<Filter>` 支持遍历配置：`filter / marker / latest_only / ask_disks / versions_sort / limit / include_free_versions / walkdir_timeout / walkdir_stall_timeout`。`walkdir_timeout` 和 `walkdir_stall_timeout` 是遍历操作的超时防护，防止慢盘钉死遍历。

#### 3.9.4 架构价值

正交 trait 分层使得：
1. 后端可以按需实现部分 trait（如只读后端只需 `ObjectIO + ListOperations`）
2. 测试可以 mock 单个 trait
3. 新增操作类型（如 HealOperations）不影响已有 trait
4. 泛型关联类型避免了 `Box<dyn Error>` 的类型擦除损失

---

### 3.10 common / concurrency / config — 基础设施

#### 3.10.1 common

`ais/RustFS/crates/common/` 是搁置的领域代码集合，含 `heal_channel`（~776 行）、`scanner metrics`（~4810 行）等。ARCHITECTURE.md 明确指出 common crate 是结构问题之一，目标态应将领域代码迁回对应 crate。

#### 3.10.2 concurrency

`ais/RustFS/crates/concurrency/` 提供并发原语，含 `WorkloadAdmissionSnapshotProvider`（被 heal crate 引用，用于工作负载准入控制快照）。

#### 3.10.3 config

`ais/RustFS/crates/config/` 提供配置管理。ecstore 内部的 `config/{scanner.rs, heal.rs, storageclass.rs}` 定义各子系统的运行时配置。

---

## 4. AIS 7 层映射表

### 4.1 自研 AIS 7 层模型回顾

根据 `docs/working-reports/20260823_cloud_drive_and_relgraph_selfdev_plan.md` 第 4.1 节，自研云盘 AIS 7 层架构为：

| 层级 | 职责 | 自研实现位置 |
|---|---|---|
| **L2 Gateway** | HTTP 路由 / S3 兼容 API / Sidecar 降级 | `platform/gateway/runtime/src/routes/cloud_drive.rs`（规划中） |
| **L3 Orchestration** | Node workflow 编排 / 冷热迁移工作流 | `platform/backend-node/src/workflow-engine.js`（wf-file-upload-v1 / wf-cloud-tier-migration） |
| **L4 Services** | 业务服务 crate（Master/Volume/Filer/S3/Tiering/Quota/IamSts） | `platform/domains/cloud/svc/{mox-cloud-master-svc, mox-cloud-volume-svc, mox-cloud-s3-svc, mox-cloud-filer-svc, mox-cloud-rebalance-svc}` |
| **L5 Domain Traits** | 领域抽象 trait（DIP 依赖倒置） | `platform/domains/cloud/core/`（规划中：ObjectStorageProvider / MetaStorageProvider / ChunkManagerProvider / QuotaProvider / IamPolicyProvider） |
| **L6 Kernel** | 纯运算（零外部 crate） | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/{gf256_simd.rs, reed_solomon.rs}`（已实现）；规划中：lru_std_impl / sha256_chunk_id / raft_log_binary_encode / crc32c_watermark |
| **L7 Infra** | 基础设施适配器 | 规划中：FSChunkBackend / S3ChunkBackend / RocksDBVolumeStore / PostgresMetaStore / RedisClusterMetaStore |

### 4.2 RustFS → 自研 AIS 7 层映射

| RustFS 组件 | 可借鉴模式 | 映射到自研层级 | 具体映射位置 |
|---|---|---|---|
| **ecstore erasure/coding** | 写仲裁 WriteProgressPolicy、读仲裁 hedge+locality、lockstep 对齐、deferred parity、reconstruction verification fail-closed、EncodedBlock 单 backing buffer | **L6 Kernel**（外围算法）+ **L4 Services**（volume-svc 读写路径） | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` 扩展；`volume_server.rs` 读写仲裁 |
| **ecstore layout (pool/set/disk)** | 三级拓扑组织、盘池选择、数据分布 | **L4 Services**（master-svc 调度） | `mox-cloud-master-svc/src/{scheduler.rs, volume_allocator.rs}` |
| **rio reader 管线** | 组合式 Reader trait、能力探测（EtagResolvable/HashReaderDetector/TryGetIndex）、delegate_reader_capabilities 宏、WarpReader 包装 | **L5 Domain Traits**（I/O 管线抽象）+ **L6 Kernel**（管线阶段实现） | `platform/domains/cloud/core/` 新增 `io_pipeline.rs` trait；`mox-cloud-volume-svc/src/` 实现 encrypt/compress/hash 阶段 |
| **io-core backpressure** | CAS 信号量准入控制、三态状态机（Normal/Warning/Critical）、cooldown 防抖、release 防下溢 | **L6 Kernel**（背压算法）+ **L4 Services**（volume-svc I/O 准入） | `platform/domains/cloud/core/` 新增 `backpressure.rs`；`volume_server.rs` 集成 |
| **io-core pool** | 四层分档缓冲池（4KB/64KB/512KB/4MB）、Semaphore 并发控制、Mutex 复用队列、PooledBuffer RAII 自动归还 | **L6 Kernel**（缓冲池算法） | `platform/domains/cloud/core/` 新增 `buffer_pool.rs` |
| **lifecycle evaluator** | 策略评估器、复制等待门控、Object Lock 防护、DeleteAllVersions 短路、noncurrent 版本计数 | **L4 Services**（s3-svc lifecycle） | `mox-cloud-s3-svc/src/lifecycle/` 扩展 |
| **scanner budget** | 三维预算（时长/对象数/目录数）、CancellationToken 子令牌取消、saturating_fetch_add CAS、remaining_config 断点续扫、remote_progress 分布式聚合 | **L6 Kernel**（扫描预算算法）+ **L4 Services**（s3-svc inventory / rebalance-svc 扫描） | `platform/domains/cloud/core/` 新增 `scan_budget.rs`；`mox-cloud-rebalance-svc/src/` 集成 |
| **object-capacity scope** | CapacityScope 注册表（软/硬限制 + TTL + LRU）、dirty-scope generation 快路径（backlog#1315）、token 化容量计算 | **L6 Kernel**（容量算法）+ **L4 Services**（master-svc 配额） | `platform/domains/cloud/core/` 新增 `capacity_scope.rs`；`mox-cloud-master-svc/src/scheduler.rs` 集成 |
| **heal** | owned 初始化（取消安全）、MRF 队列消费、HealChannelProcessor、ReplacementRecovery 幸存者盘记录、HealStorageAPI trait 抽象 | **L5 Domain Traits**（HealStorageAPI）+ **L4 Services**（volume-svc rebuild + 新增 heal-svc） | `platform/domains/cloud/core/` 新增 `heal_storage.rs` trait；`mox-cloud-volume-svc/src/rebuild.rs` 扩展 |
| **replication** | ReplicationConfig / ReplicationState / VersionPurgeStatus、复制队列、resync 引擎、MRF 失败重试、与 lifecycle 联动门控 | **L4 Services**（s3-svc replication） | `mox-cloud-s3-svc/src/replication/` 扩展 |
| **storage-api** | 正交 trait 分层（ObjectIO/ObjectOperations/ListOperations/MultipartOperations/HealOperations/NamespaceLocking）、泛型关联类型、HTTP 前置条件、WalkOptions 超时防护 | **L5 Domain Traits**（核心参考） | `platform/domains/cloud/core/` 按正交拆分重构现有 trait |
| **ecstore services/tier** | 冷热分层、10+ 云后端适配器、tier_sweeper / tier_delete_journal / tier_free_version_recovery | **L4 Services**（volume-svc storage_tier） | `mox-cloud-volume-svc/src/storage_tier.rs` 扩展 |
| **ecstore bucket/quota** | checker / reservation 配额检查与预留 | **L4 Services**（master-svc 配额） | `mox-cloud-master-svc/src/scheduler.rs` 扩展 |
| **ecstore services/rebalance** | control/entry/meta/migration/runtime/types/worker 再均衡子系统 | **L4 Services**（rebalance-svc） | `mox-cloud-rebalance-svc/src/{placement_strategy.rs, rebalance_controller.rs}` 参考 |

---

## 5. 三分类决策矩阵

### 5.1 【自研保留】— 自研已有且架构合理，不改动

| 编号 | 功能点 | 自研实现位置 | 保留理由 |
|---|---|---|---|
| R-01 | **GF(2^8) SIMD 向量乘法内核** | `mox-cloud-volume-svc/src/gf256_simd.rs` | 自研 16 子表 LUT 级联 AVX2/NEON 双架构实现，含运行时特征检测 + scalar tail 处理 + fused mul-xor 内核，架构自洽性优于 RustFS 对外部 crate 的依赖 |
| R-02 | **ReedSolomonEngine 全自研引擎** | `mox-cloud-volume-svc/src/reed_solomon.rs` | Vandermonde 编码矩阵 + Gauss-Jordan 求逆 + Matrix 缓存 + PathChoice（Auto/Simd/Scalar）+ `auto_prefers_simd` 运行时微基准决策 + 遗留 2+1 XOR 兼容，T22 验收测试全覆盖 |
| R-03 | **Raft Master 控制面** | `mox-cloud-master-svc/src/{raft_master.rs, master_server.rs}` | Raft 共识 + 卷分配 + 心跳 + 调度，自研控制面主权不可让渡 |
| R-04 | **五 svc 拓扑架构** | `platform/domains/cloud/svc/` 下五个 crate | Master/Volume/S3/Filer/Rebalance 职责分离清晰，对齐 SeaweedFS 最佳实践 |
| R-05 | **AIS 7 层模型** | `docs/working-reports/20260823_...md` | 璇玑全域架构规范，L2-L7 DIP 倒置不可变更 |
| R-06 | **审计 hash_chain** | `platform/backend-node/src/`（已有） | 璇玑企业级治理核心能力，SHA-256 HMAC + TTI 180 天，不可替换 |
| R-07 | **EC rebuild 作业** | `mox-cloud-volume-svc/src/rebuild.rs` | manifest CRC64 校验 + replica/EC 双路径 + aggregate CRC64 更新，逻辑完整 |
| R-08 | **EcProfile 配置** | `mox-cloud-volume-svc/src/profile.rs` | data_shards/parity_shards/min_obj_size 配置 + is_replica 判定，自研语义 |
| R-09 | **EcManifest 元数据** | `mox-cloud-volume-svc/src/manifest.rs` | crc64_ecma + shard_count + tier + original_size，自研布局 |
| R-10 | **fs_layout 磁盘布局** | `mox-cloud-volume-svc/src/fs_layout.rs` | mountpath/bucket_prefix/oid/shard_id 路径布局，自研规范 |
| R-11 | **volume_server 服务框架** | `mox-cloud-volume-svc/src/volume_server.rs` | 卷服务入口 + metrics 集成，自研服务框架 |
| R-12 | **metrics 指标体系** | `mox-cloud-volume-svc/src/metrics.rs` | encode_us / shards_lost / rebuild_count 等自研指标，对齐 SLO 4 窗口 |

### 5.2 【借鉴吸收】— RustFS 有更优实现，将算法/架构模式吸收进自研代码（重写，不直接 copy）

| 编号 | 功能点 | RustFS 参考位置 | 吸收到自研位置 | 吸收内容 |
|---|---|---|---|---|
| A-01 | **写仲裁 WriteProgressPolicy** | `ecstore/src/erasure/coding/encode.rs` | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` | stall_timeout 按块 re-arm + absolute_cap 防 slow-drip peer + 失败 writer commit 前剔除 + write_quorum 仲裁 |
| A-02 | **读仲裁 hedge + locality** | `ecstore/src/erasure/coding/decode.rs` | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` | hedge_delay=min(timeout,100ms) + ShardReadCost 排序 + data-shards-only 门控 + deferred parity 按 stripe 对齐接入 |
| A-03 | **lockstep 条带对齐** | `ecstore/src/erasure/coding/decode.rs` | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` | DeferredReaderStripeHandle::advance_stripes 防 parity 中途接入 desync |
| A-04 | **reconstruction verification fail-closed** | `ecstore/src/erasure/coding/erasure.rs` | `mox-cloud-volume-svc/src/reed_solomon.rs` | 冗余 parity 重建后逐块比对，不一致返回 InvalidData |
| A-05 | **EncodedBlock 单 backing buffer** | `ecstore/src/erasure/coding/erasure.rs` | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` | 单连续 Bytes + shard_size 字段，chunks_exact 切片视图，避免 per-shard 分配 |
| A-06 | **零拷贝 encode_data_owned / bytes_mut** | `ecstore/src/erasure/coding/erasure.rs` | `mox-cloud-volume-svc/src/reed_solomon.rs` | Vec→Bytes→try_into_mut（refcount==1 零拷贝）+ 预预留容量后 resize 不重分配 |
| A-07 | **CAS 背压信号量** | `io-core/src/backpressure.rs` | `platform/domains/cloud/core/backpressure.rs`（新建） | max_concurrent + high/low water + cooldown + CAS try_acquire + fetch_update checked_sub release 防下溢 + 三态状态机 |
| A-08 | **四层分档缓冲池** | `io-core/src/pool.rs` | `platform/domains/cloud/core/buffer_pool.rs`（新建） | 4KB/64KB/512KB/4MB 四档 + Semaphore 并发 + Mutex 复用队列 + PooledBuffer RAII 归还 + select_tier + hit_rate 指标 |
| A-09 | **组合式 reader 管线** | `rio/src/lib.rs` | `platform/domains/cloud/core/io_pipeline.rs`（新建） | Reader = ReadStream + ReaderCapabilities（EtagResolvable/HashReaderDetector/TryGetIndex）+ delegate_reader_capabilities 宏 + WarpReader |
| A-10 | **三维扫描预算** | `scanner/src/scanner_budget.rs` | `platform/domains/cloud/core/scan_budget.rs`（新建） | max_duration/max_objects/max_directories 三维 + CancellationToken 子令牌 + saturating_fetch_add CAS + remaining_config 断点续扫 + remote_progress 聚合 |
| A-11 | **dirty-scope generation 快路径** | `object-capacity/src/capacity_scope.rs` | `platform/domains/cloud/core/capacity_scope.rs`（新建） | DIRTY_GENERATION 单调 + record 返回 generation + 快路径跳过 mutex + drain 推进 generation + 空 drain 不推进 |
| A-12 | **CapacityScope 注册表** | `object-capacity/src/capacity_scope.rs` | `platform/domains/cloud/core/capacity_scope.rs`（新建） | 软=2048/硬=4096 + TTL=300s + LRU 淘汰 + 合并去重 + 过期替换防复活 + poison 恢复 |
| A-13 | **owned 初始化模式** | `heal/src/lib.rs` | `platform/domains/cloud/core/` 通用模式 | run_owned_initialization（tokio::spawn 包裹）+ 初始化互斥锁 + 失败回滚 + 取消安全 |
| A-14 | **正交 trait 分层方法论** | `storage-api/src/object.rs` | `platform/domains/cloud/core/` 重构 | ObjectIO/ObjectOperations/ListOperations/MultipartOperations/HealOperations/NamespaceLocking 六 trait 正交拆分 + 泛型关联类型 |
| A-15 | **HTTP 前置条件** | `storage-api/src/object.rs` | `mox-cloud-s3-svc/src/` | ObjectPreconditionState + HTTPPreconditions + etag_matches（trim+wildcard）+ is_modified_since |
| A-16 | **WalkOptions 超时防护** | `storage-api/src/object.rs` | `mox-cloud-s3-svc/src/` + `mox-cloud-rebalance-svc/src/` | walkdir_timeout + walkdir_stall_timeout 防慢盘钉死遍历 |
| A-17 | **lifecycle 复制等待门控** | `lifecycle/src/evaluator.rs` | `mox-cloud-s3-svc/src/lifecycle/` | replication_status_blocks_lifecycle（Pending/Failed 阻塞 Delete/Transition）+ DeleteAllVersions 短路 + Object Lock 防护 |
| A-18 | **AbortOnDropTask 取消安全** | `ecstore/src/erasure/coding/encode.rs` | `platform/domains/cloud/core/` 通用模式 | producer 任务 AbortOnDrop 保证取消时 abort 防泄漏 |

### 5.3 【对接集成】— 作为可选 L7 数据面接入点，通过 trait 抽象与 RustFS 对接（不强制依赖）

| 编号 | 功能点 | RustFS 组件 | 对接方式 | 自研 trait 抽象 |
|---|---|---|---|---|
| I-01 | **可选 EC 存储后端** | `ais/RustFS/crates/ecstore/` | 通过 L5 `ChunkManagerProvider` trait 的可选实现，将 RustFS ecstore 作为高性能 EC 后端接入；自研 volume-svc 为默认后端 | `platform/domains/cloud/core/chunk_manager.rs` |
| I-02 | **可选 I/O 管线后端** | `ais/RustFS/crates/rio/` | 通过 L5 `IoPipelineProvider` trait 的可选实现，将 RustFS rio 作为 I/O 管线后端；自研 io_pipeline 为默认实现 | `platform/domains/cloud/core/io_pipeline.rs` |
| I-03 | **可选自愈引擎** | `ais/RustFS/crates/heal/` | 通过 L5 `HealStorageAPI` trait 的可选实现，将 RustFS heal 作为自愈引擎；自研 rebuild + 调度为默认实现 | `platform/domains/cloud/core/heal_storage.rs` |
| I-04 | **trait 对齐参考** | `ais/RustFS/crates/storage-api/` | 不作为运行时依赖，仅作为自研 L5 trait 设计的参考标准；自研 trait 签名可与之对齐以便未来对接 | `platform/domains/cloud/core/` 全部 trait |
| I-05 | **可选生命周期策略引擎** | `ais/RustFS/crates/lifecycle/` | 通过 L5 `LifecyclePolicyProvider` trait 的可选实现，将 RustFS lifecycle 作为策略引擎；自研 s3-svc lifecycle 为默认实现 | `platform/domains/cloud/core/lifecycle_policy.rs` |

---

## 6. 关键算法吸收方案

### 6.1 纠删码吸收方案

#### 6.1.1 现状对比

| 维度 | 自研（gf256_simd + reed_solomon） | RustFS（ecstore erasure/coding） |
|---|---|---|
| **GF(2^8) 内核** | 自研 16 子表 LUT 级联 AVX2/NEON，`gf_vec_mul_auto` + `gf_vec_mul_xor_auto` | 外部 `reed-solomon-erasure` crate（现代）+ `reed_solomon_simd`（遗留） |
| **RS 引擎** | 自研 Vandermonde + Gauss-Jordan，`ReedSolomonEngine` | 外部 crate 封装 |
| **路径选择** | `PathChoice::{Auto, Simd, Scalar}`，`auto_prefers_simd` 运行时微基准（7 次中位数比较） | 编译期选择现代/遗留后端 |
| **矩阵缓存** | `Mutex<Vec<CachedMatrix>>`，线性查找 | `OnceLock + RwLock<HashMap<(d,m), Arc<..>>>`，O(1) 查找 |
| **写仲裁** | 无（同步写所有 shard） | MultiWriter + WriteProgressPolicy（stall_timeout + absolute_cap）+ write_quorum |
| **读仲裁** | 无（读所有可用 shard） | ParallelReader + hedge + locality + data-shards-only + deferred parity + lockstep |
| **一致性校验** | rebuild 时 CRC64 比对 manifest | reconstruction verification（冗余 parity 逐块比对，fail-closed）+ has_valid_dimensions |
| **零拷贝** | 无（encode 返回 `Vec<Vec<u8>>`） | encode_data_owned（Vec→Bytes→try_into_mut）+ encode_data_bytes_mut（预预留不重分配）+ EncodedBlock 单 backing buffer |

#### 6.1.2 吸收策略

**内核层【自研保留】**：`gf256_simd.rs` 和 `reed_solomon.rs` 的数学内核不改动，这是自研的核心优势。

**外围算法【借鉴吸收】**：

1. **矩阵缓存优化**（A-类，优先级高）：将 `Mutex<Vec<CachedMatrix>>` 改为 `OnceLock + RwLock<HashMap<(u16,u16), Arc<Matrix>>>`，O(1) 查找替代线性扫描。

2. **写仲裁 WriteProgressPolicy**（A-01，优先级高）：在 `erasure_coding_ext.rs` 中实现 `MultiWriter` 模式：
   - 每块并发写所有存活 shard，成功数 ≥ write_quorum 通过
   - `stall_timeout` 按块 re-arm（默认 30s，可配置）
   - `absolute_cap` 可选（防 slow-drip peer）
   - 失败 writer commit 前剔除
   - `AbortOnDropTask` 保证 producer 取消安全

3. **读仲裁 hedge + locality**（A-02/A-03，优先级中）：
   - 每 stripe 只发 `data_shards` 个读，成功即停
   - `hedge_delay = min(read_timeout, 100ms)`
   - `ShardReadCost` 按 Local/SameNode/Remote 排序
   - `data-shards-only` 门控：healthy 对象只读 data shards，parity deferred
   - `lockstep` 对齐：parity 中途接入时 `advance_stripes` 对齐当前 stripe

4. **reconstruction verification**（A-04，优先级中）：在 `reed_solomon.rs` 中新增 `decode_with_verification` 方法，当可用 shard > data_shards 时，用冗余 parity 重建后逐块比对，不一致 fail-closed。

5. **零拷贝编码路径**（A-05/A-06，优先级低）：
   - 新增 `EncodedBlock` 类型：单连续 `Bytes` + `shard_size`，`shards()` 用 `chunks_exact` 切片
   - 新增 `encode_owned(Vec<u8>)`：refcount==1 时 `try_into_mut` 零拷贝
   - 新增 `encode_bytes_mut(BytesMut)`：预预留容量后不重分配

#### 6.1.3 不吸收项

- RustFS 的双后端（现代/遗留）设计：自研无历史兼容包袱，不需要 legacy 后端
- RustFS 的 `reed-solomon-erasure` 外部依赖：自研内核更优，不引入外部依赖

---

### 6.2 生命周期吸收方案

#### 6.2.1 现状

自研 `mox-cloud-s3-svc` 已有 lifecycle 模块（用户任务描述确认），但未深入源码。RustFS lifecycle 的 `Evaluator` 模式有以下可借鉴点：

#### 6.2.2 吸收内容（A-17）

1. **复制等待门控**：`replication_status_blocks_lifecycle` 函数——对象处于 `Pending/Failed` 复制状态时，`DeleteAction/DeleteVersionAction/DeleteAllVersionsAction/TransitionAction` 等动作降级为 `NoneAction`。这保证复制未完成的对象不被生命周期删除/迁移。

2. **DeleteAllVersions 短路**：当某版本命中 `DeleteAllVersionsAction` 时，检查桶级 object lock + 版本级锁 + 复制 pending，若全部通过则执行全版本删除并 `break` 跳过剩余版本评估。这避免了对已决定全删的对象做不必要的逐版本评估。

3. **Object Lock 防护**：`is_object_locked` 检查用户自定义元数据中的 `X-Amz-Object-Lock-*` 头 + 桶级默认保留期，锁定对象的删除动作降级。

4. **noncurrent 版本计数**：`newer_noncurrent_versions` 计数器支持 `NoncurrentVersionExpiration` 的 `NewerNoncurrentVersions` 规则（保留最近 N 个非当前版本）。

---

### 6.3 容量管理吸收方案

#### 6.3.1 现状

自研 master-svc 的 `scheduler.rs`（50KB）和 `volume_allocator.rs` 已有卷分配与调度逻辑，但未见独立的容量 scope 管理与 dirty-scope 快路径机制。

#### 6.3.2 吸收内容（A-11/A-12）

1. **CapacityScope 注册表**（A-12）：
   - 在 `platform/domains/cloud/core/capacity_scope.rs` 新建
   - `CapacityScopeDisk { endpoint, drive_path }` + `CapacityScope { disks: Vec<CapacityScopeDisk> }`
   - 注册表：`OnceLock<Mutex<HashMap<Uuid, CapacityScopeEntry>>>`，软=2048/硬=4096/TTL=300s
   - 淘汰：达软限时 prune expired + enforce hard limit（LRU by recorded_at）
   - 合并：同 token 非过期 scope 合并去重；过期直接替换防复活
   - Poison 恢复：`lock().unwrap_or_else(|p| p.into_inner())`

2. **dirty-scope generation 快路径**（A-11，最精巧）：
   - `DIRTY_GENERATION: AtomicU64` 单调递增
   - `record_global_dirty_scope(scope) -> u64`：记录脏磁盘并返回当前 generation
   - 调用方缓存 generation，后续写入若 `current_dirty_generation() == cached_generation` 则跳过 mutex
   - `drain_global_dirty_scopes()`：排空并 `fetch_add(1)`；空 drain 不推进
   - 不变式：generation 加载/推进都在 mutex 保护下
   - 可观测性：`GLOBAL_DIRTY_UPGRADE_COUNT` 记录升级次数

3. **与自研调度的集成**：
   - 写操作时 `record_capacity_scope(token, scope)` 记录涉及的磁盘
   - 后台容量扫描器按 token `take_capacity_scope` 取回 scope 执行刷新
   - 刷新完成后 `drain_global_dirty_scopes` 推进 generation

---

### 6.4 I/O 管线吸收方案

#### 6.4.1 现状

自研 volume-svc 目前是同步 I/O 模型（`fs::read` / `fs::write`），无异步 I/O 管线、无缓冲池、无背压控制。

#### 6.4.2 吸收内容

1. **组合式 reader 管线**（A-09）：
   - 在 `platform/domains/cloud/core/io_pipeline.rs` 新建
   - `Reader trait = ReadStream + ReaderCapabilities`
   - `ReaderCapabilities = EtagResolvable + HashReaderDetector + TryGetIndex`
   - `DynReader = Box<dyn Reader>`
   - `delegate_reader_capabilities!` 宏：为包装 reader 递归透传能力
   - `WarpReader`：将裸 ReadStream 包装为 DynReader
   - 管线阶段：`encrypt → compress → hash → etag → checksum → limit → hardlimit`

2. **CAS 背压信号量**（A-07）：
   - 在 `platform/domains/cloud/core/backpressure.rs` 新建
   - `BackpressureMonitor { max_concurrent: AtomicUsize, current: AtomicUsize, high_water: f32, low_water: f32, cooldown: Duration }`
   - `try_acquire() -> bool`：CAS 循环，满则拒绝 + rejection_rate++
   - `release()`：`fetch_update` + `checked_sub` 防下溢
   - 三态状态机：`Normal / Warning / Critical`
   - cooldown 防抖

3. **四层分档缓冲池**（A-08）：
   - 在 `platform/domains/cloud/core/buffer_pool.rs` 新建
   - 四档：Small 4KB(max1000) / Medium 64KB(max500) / Large 512KB(max100) / XLarge 4MB(max25)
   - `Semaphore` 控制每档并发
   - `Mutex<Vec<BytesMut>>` 复用队列
   - `PooledBuffer { permit: OwnedSemaphorePermit, buf: ManuallyDrop<BytesMut> }`，Drop 自动归还
   - `select_tier(size)` 选档
   - 指标：`available_buffers / hit_rate / allocated_bytes`

4. **与 volume-svc 的集成路径**：
   - 阶段一：先引入缓冲池（A-08），替换 `fs::read` 中的 `Vec::with_capacity` 分配
   - 阶段二：引入背压（A-07），在 volume_server 的并发写路径加准入控制
   - 阶段三：引入 I/O 管线（A-09），将 encrypt/hash/checksum 重构为组合式 reader

---

## 7. 分阶段实施路线图

### 7.1 阶段一：修复编译 + 测试绿色基线

**目标**：确保自研云盘代码在当前状态下可编译、测试全绿，为后续吸收工作建立可靠基线。

**周期**：2 周

**工作项**：

| 编号 | 工作项 | 负责模块 | 验收标准 |
|---|---|---|---|
| P1-01 | `cargo build --workspace` 全量编译通过 | `platform/domains/cloud/` | exit 0，零 ERROR |
| P1-02 | `cargo clippy --workspace -- -D warnings` 通过 | 全模块 | 零 clippy ERROR |
| P1-03 | `cargo test --workspace` 全量测试通过 | 全模块 | 0 fail |
| P1-04 | T22 纠删码验收测试回归 | `mox-cloud-volume-svc` | t22_avx2_rand_1m / t22_encode_bit_identical_3x4x2_grid / t22_decode_lost_4_reconstruct_identical_1000 全绿 |
| P1-05 | rebuild 测试回归 | `mox-cloud-volume-svc` | encode_write_and_rebuild_small GREEN |
| P1-06 | 建立 CI 基线快照 | CI 配置 | 记录当前测试数量/覆盖率/编译时间 |

**风险**：若当前代码存在编译错误，需先修复。本阶段不做任何功能变更，仅修复编译/测试问题。

**注意**：本阶段是文档路线图内容，实际执行需用户授权。

---

### 7.2 阶段二：吸收核心算法

**目标**：吸收纠删码外围算法、生命周期策略、容量管理三个核心算法模块，不改变现有服务拓扑。

**周期**：6 周

**工作项**：

| 编号 | 工作项 | 吸收编号 | 负责模块 | 验收标准 |
|---|---|---|---|---|
| P2-01 | 矩阵缓存优化（Mutex\<Vec\> → RwLock\<HashMap\>） | 纠删码-1 | `mox-cloud-volume-svc/src/reed_solomon.rs` | O(1) 查找；benchmark 显示缓存命中延迟降低 ≥50% |
| P2-02 | reconstruction verification fail-closed | A-04 | `mox-cloud-volume-svc/src/reed_solomon.rs` | 新增 `decode_with_verification`；冗余 shard 不一致时返回 InvalidData；单元测试覆盖 |
| P2-03 | 写仲裁 MultiWriter + WriteProgressPolicy | A-01 | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` | stall_timeout 按块 re-arm；失败 writer commit 前剔除；write_quorum 可配置；集成测试覆盖 black-hole peer 场景 |
| P2-04 | 读仲裁 hedge + locality + data-shards-only | A-02/A-03 | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` | hedge_delay=min(timeout,100ms)；ShardReadCost 排序；healthy 对象只读 data shards；lockstep 对齐 |
| P2-05 | lifecycle 复制等待门控 + DeleteAllVersions 短路 | A-17 | `mox-cloud-s3-svc/src/lifecycle/` | Pending/Failed 复制状态阻塞 Delete/Transition；DeleteAllVersions 短路跳过剩余评估；Object Lock 防护 |
| P2-06 | CapacityScope 注册表 | A-12 | `platform/domains/cloud/core/capacity_scope.rs`（新建） | 软=2048/硬=4096/TTL=300s；LRU 淘汰；合并去重；过期替换防复活；poison 恢复；单元测试全覆盖 |
| P2-07 | dirty-scope generation 快路径 | A-11 | `platform/domains/cloud/core/capacity_scope.rs` | DIRTY_GENERATION 单调；快路径跳过 mutex；drain 推进 generation；空 drain 不推进；GLOBAL_DIRTY_UPGRADE_COUNT 可观测 |
| P2-08 | EncodedBlock 单 backing buffer + 零拷贝路径 | A-05/A-06 | `mox-cloud-volume-svc/src/erasure_coding_ext.rs` | EncodedBlock size_of == Bytes + usize；encode_owned refcount==1 零拷贝；encode_bytes_mut 预预留不重分配；benchmark 显示大对象编码内存分配减少 ≥60% |

**里程碑交付**：
- 核心算法吸收完成，测试全绿
- 性能基准报告：纠删码编码/解码延迟、内存分配对比
- 容量管理快路径验证：稳态写入 mutex 升级次数为 0

---

### 7.3 阶段三：架构融合

**目标**：吸收 I/O 管线、缓冲池、背压、扫描预算、自愈调度等架构模式，完成 L5/L6 层的架构融合。

**周期**：8 周

**工作项**：

| 编号 | 工作项 | 吸收编号 | 负责模块 | 验收标准 |
|---|---|---|---|---|
| P3-01 | 四层分档缓冲池 | A-08 | `platform/domains/cloud/core/buffer_pool.rs`（新建） | 4KB/64KB/512KB/4MB 四档；Semaphore 并发；PooledBuffer RAII 归还；hit_rate 指标；volume-svc 集成替换 Vec 分配 |
| P3-02 | CAS 背压信号量 | A-07 | `platform/domains/cloud/core/backpressure.rs`（新建） | max_concurrent=32；high=0.8/low=0.5；cooldown=100ms；release 防下溢；三态状态机；volume_server 写路径集成 |
| P3-03 | 组合式 reader 管线 | A-09 | `platform/domains/cloud/core/io_pipeline.rs`（新建） | Reader = ReadStream + ReaderCapabilities；delegate_reader_capabilities 宏；WarpReader；encrypt/compress/hash/etag/checksum/limit/hardlimit 阶段；volume-svc 读路径集成 |
| P3-04 | 三维扫描预算 | A-10 | `platform/domains/cloud/core/scan_budget.rs`（新建） | max_duration/max_objects/max_directories；CancellationToken 子令牌；saturating_fetch_add CAS；remaining_config 断点续扫；remote_progress 聚合；rebalance-svc 集成 |
| P3-05 | L5 trait 正交拆分重构 | A-14 | `platform/domains/cloud/core/` | ObjectIO/ObjectOperations/ListOperations/MultipartOperations/HealOperations/NamespaceLocking 六 trait；泛型关联类型；现有 trait 兼容迁移 |
| P3-06 | HTTP 前置条件 + WalkOptions 超时 | A-15/A-16 | `mox-cloud-s3-svc/src/` | If-Match/If-None-Match/If-Modified-Since/If-Unmodified-Since；etag_matches trim+wildcard；walkdir_timeout/stall_timeout |
| P3-07 | owned 初始化模式 + AbortOnDropTask | A-13/A-18 | `platform/domains/cloud/core/` 通用 | run_owned_initialization 取消安全；初始化互斥锁；失败回滚；AbortOnDropTask producer 防泄漏 |
| P3-08 | 自愈调度增强 | heal 参考 | `mox-cloud-volume-svc/src/rebuild.rs` 扩展 | MRF 队列消费模式；ReplacementRecovery 幸存者盘记录；HealStorageAPI trait 抽象；rebuild 作业调度优先级 |

**里程碑交付**：
- L5/L6 架构融合完成，`platform/domains/cloud/core/` 新增 6 个模块
- volume-svc / s3-svc / rebalance-svc 完成集成
- 性能基准报告：I/O 吞吐、p99 延迟、内存占用、并发能力
- 背压验证：高并发下 rejection_rate 可控，无内存泄漏

---

### 7.4 阶段四：可选 RustFS L7 数据面对接

**目标**：通过 L5 trait 抽象，实现 RustFS ecstore / rio / heal / lifecycle 作为可选 L7 数据面后端的对接能力，不强制依赖。

**周期**：4 周（可选，按需启动）

**工作项**：

| 编号 | 工作项 | 对接编号 | 负责模块 | 验收标准 |
|---|---|---|---|---|
| P4-01 | RustFS ecstore 作为可选 EC 后端 | I-01 | `platform/domains/cloud/core/chunk_manager.rs` | ChunkManagerProvider trait 定义；RustFS ecstore adapter 实现；feature flag `rustfs-ecstore` 控制；自研 volume-svc 为默认后端 |
| P4-02 | RustFS rio 作为可选 I/O 管线后端 | I-02 | `platform/domains/cloud/core/io_pipeline.rs` | IoPipelineProvider trait 定义；RustFS rio adapter 实现；feature flag 控制 |
| P4-03 | RustFS heal 作为可选自愈引擎 | I-03 | `platform/domains/cloud/core/heal_storage.rs` | HealStorageAPI trait 定义；RustFS heal adapter 实现；feature flag 控制 |
| P4-04 | RustFS lifecycle 作为可选策略引擎 | I-05 | `platform/domains/cloud/core/lifecycle_policy.rs` | LifecyclePolicyProvider trait 定义；RustFS lifecycle adapter 实现；feature flag 控制 |
| P4-05 | 对接集成测试 | 全部 | 集成测试 | feature flag 开启时 RustFS 后端可用；关闭时自研后端默认；双后端数据格式兼容验证 |

**里程碑交付**：
- 4 个可选 L7 后端对接完成
- feature flag 机制验证
- 双后端兼容性测试报告
- 对接性能基准：RustFS 后端 vs 自研后端的吞吐/延迟对比

**注意**：本阶段为可选阶段，仅在需要 RustFS 数据面能力时启动。RustFS 源码保持只读，不修改、不移动、不删除。

---

### 7.5 路线图总览

```
阶段一（2周）  阶段二（6周）           阶段三（8周）                阶段四（4周，可选）
┌──────────┐  ┌──────────────────────┐ ┌──────────────────────────┐ ┌──────────────────────┐
│ 编译+测试 │  │ 核心算法吸收           │ │ 架构融合                   │ │ 可选 L7 对接          │
│ 绿色基线  │→ │ 纠删码外围/生命周期/  │→ │ I/O管线/缓冲池/背压/     │→ │ RustFS ecstore/rio/  │
│          │  │ 容量管理               │ │ 扫描预算/trait重构/自愈   │ │ heal/lifecycle        │
└──────────┘  └──────────────────────┘ └──────────────────────────┘ └──────────────────────┘
  0~2 周         2~8 周                   8~16 周                       16~20 周（按需）
```

**总周期**：阶段一至三共 16 周（约 4 个月），阶段四可选 4 周。

**团队配置建议**：
- Rust 工程师 3 名（1 名资深负责 L6 内核算法，2 名负责 L4/L5 集成）
- 架构师 1 名（负责 trait 设计评审 + 跨模块协调）
- SRE/测试 1 名（负责性能基准 + 集成测试 + CI）

---

## 8. 风险与规避

### 8.1 技术风险

| 编号 | 风险 | 等级 | 规避策略 |
|---|---|---|---|
| T-01 | **纠删码外围算法吸收引入数据一致性风险**：写仲裁/读仲裁逻辑复杂，若实现有误可能导致数据丢失或静默损坏 | 🔴 高 | ① 严格 TDD：先写失败测试再实现；② 保留现有同步写路径作为 fallback，通过 feature flag 渐进切换；③ 集成测试覆盖 black-hole peer / straggler / 部分失败等边界场景；④ reconstruction verification fail-closed 作为最后防线 |
| T-02 | **I/O 管线重构引入回归**：组合式 reader 管线改变了读路径，可能影响已有功能 | 🟡 中 | ① 分阶段引入：先缓冲池→再背压→最后 I/O 管线，每阶段独立验证；② 保留旧读路径，通过 trait 抽象双实现并行；③ 全量回归测试 + fio 性能基准对比 |
| T-03 | **L5 trait 正交拆分破坏现有实现**：将粗粒度 trait 拆分为六个正交 trait，可能导致现有实现不兼容 | 🟡 中 | ① 新增 trait 不删除旧 trait，旧 trait 作为组合 trait 存在；② 提供 trait 自动实现（blanket impl）桥接旧接口；③ 分模块迁移，每模块独立测试 |
| T-04 | **dirty-scope generation 快路径的并发正确性**：generation 快路径依赖微妙的不变式，若实现有误可能导致脏磁盘遗漏 | 🟡 中 | ① 严格遵循 RustFS 的不变式：generation 加载/推进都在 mutex 保护下；② 白盒测试验证 GLOBAL_DIRTY_UPGRADE_COUNT 稳态为 0；③ 压力测试验证高并发下无遗漏 |
| T-05 | **RustFS 版本漂移**：RustFS 持续演进，参考的算法模式可能变化 | 🟢 低 | ① 吸收时重写而非 copy，锁定当前版本的算法语义；② 文档记录吸收时的 RustFS commit hash；③ 定期 review RustFS changelog，按需更新 |

### 8.2 工程风险

| 编号 | 风险 | 等级 | 规避策略 |
|---|---|---|---|
| E-01 | **阶段一编译修复范围不可控**：当前代码可能存在大量编译错误，修复周期超出预期 | 🟡 中 | ① 先执行 `cargo build` 评估错误数量；② 若错误 >50 处，拆分为多个子任务并行修复；③ 设定硬截止：超过 3 周未完成则降级为"仅核心 crate 编译通过" |
| E-02 | **性能回退**：新引入的缓冲池/背压可能在特定场景下性能不如原实现 | 🟡 中 | ① 每阶段建立性能基准，新旧路径并行对比；② 缓冲池预热机制：启动时预分配一定数量缓冲；③ 背压参数可配置，根据实际负载调优 |
| E-03 | **测试覆盖不足**：新增算法模块测试不完整，导致潜在 bug 逃逸 | 🟡 中 | ① 每个新模块要求 ≥80% 行覆盖率；② 关键算法（纠删码/背压/缓冲池）要求 property-based testing；③ 集成测试覆盖真实 I/O 场景 |
| E-04 | **RustFS 源码只读约束违反**：开发过程中意外修改 RustFS 源码 | 🟢 低 | ① RustFS 目录加入 `.gitignore` 或只读挂载；② CI 检查 RustFS 目录文件 hash 不变；③ 代码评审时检查 diff 范围 |

### 8.3 架构风险

| 编号 | 风险 | 等级 | 规避策略 |
|---|---|---|---|
| A-01 | **过度吸收导致自研架构失焦**：盲目照搬 RustFS 模式，破坏自研 AIS 7 层模型的简洁性 | 🟡 中 | ① 三分类决策矩阵严格执行：每项吸收必须明确分类和理由；② 架构评审 gate：每个吸收项需架构师评审通过；③ 定期回顾：若某吸收项未带来可测量收益则回退 |
| A-02 | **可选 L7 对接增加维护负担**：RustFS 后端对接后需要维护双实现，增加测试和维护成本 | 🟢 低 | ① 阶段四为可选，仅在明确需求时启动；② feature flag 默认关闭；③ 对接代码隔离在独立 adapter crate，不影响核心代码 |
| A-03 | **控制面与数据面边界模糊**：吸收过程中可能将 RustFS 的控制面逻辑（如 ecstore 的 bucket/quota）误引入自研控制面 | 🟡 中 | ① 明确边界：RustFS 仅作数据面/算法参考，控制面（Master 调度/配额/心跳）完全自研；② 吸收项评审时检查是否越界；③ 文档记录控制面主权清单 |

---

## 9. 引用文档清单

### 9.1 RustFS 参考源码（仓根相对路径）

| 类别 | 路径 | 说明 |
|---|---|---|
| 入口文档 | `ais/RustFS/ARCHITECTURE.md` | RustFS 架构总览、crate 分类、6 条架构不变量、启动序列 |
| 入口文档 | `ais/RustFS/AGENTS.md` | 开发规范、Serde 安全、内部元数据双键、usage cache 序列化 |
| 纠删码 | `ais/RustFS/crates/ecstore/src/erasure/coding/erasure.rs` | 双后端编码器、EncodedBlock、零拷贝路径、一致性校验 |
| 纠删码 | `ais/RustFS/crates/ecstore/src/erasure/coding/encode.rs` | MultiWriter 写仲裁、WriteProgressPolicy、流式编码管线 |
| 纠删码 | `ais/RustFS/crates/ecstore/src/erasure/coding/decode.rs` | ParallelReader 读仲裁、hedge、locality、lockstep、deferred parity |
| I/O 管线 | `ais/RustFS/crates/rio/src/lib.rs` | 组合式 Reader 管线、ReaderCapabilities、delegate_reader_capabilities 宏 |
| 缓冲背压 | `ais/RustFS/crates/io-core/src/backpressure.rs` | CAS 信号量准入控制、三态状态机、release 防下溢 |
| 缓冲池 | `ais/RustFS/crates/io-core/src/pool.rs` | 四层分档缓冲池、Semaphore 并发、PooledBuffer RAII |
| 生命周期 | `ais/RustFS/crates/lifecycle/src/evaluator.rs` | 策略评估器、复制等待门控、DeleteAllVersions 短路、Object Lock 防护 |
| 扫描器 | `ais/RustFS/crates/scanner/src/scanner_budget.rs` | 三维扫描预算、CancellationToken 子令牌、断点续扫、分布式聚合 |
| 容量管理 | `ais/RustFS/crates/object-capacity/src/capacity_scope.rs` | CapacityScope 注册表、dirty-scope generation 快路径（backlog#1315） |
| 自愈 | `ais/RustFS/crates/heal/src/lib.rs` | 全局 HealRuntime、owned 初始化、MRF 队列、ReplacementRecovery |
| 存储 API | `ais/RustFS/crates/storage-api/src/object.rs` | 正交 trait 分层、HTTP 前置条件、WalkOptions 超时防护 |
| 复制 | `ais/RustFS/crates/replication/src/` | ReplicationConfig/State、复制队列、resync、MRF、stats |
| 拓扑布局 | `ais/RustFS/crates/ecstore/src/layout/` | pool/set/disk 三级拓扑、盘池选择、数据分布 |
| 冷热分层 | `ais/RustFS/crates/ecstore/src/services/tier/` | tier 服务、10+ 云后端适配器、tier_sweeper |
| 配额 | `ais/RustFS/crates/ecstore/src/bucket/quota/` | checker、reservation 配额检查与预留 |
| 再均衡 | `ais/RustFS/crates/ecstore/src/services/rebalance/` | control/entry/meta/migration/runtime/worker |

### 9.2 自研云盘源码（仓根相对路径）

| 类别 | 路径 | 说明 |
|---|---|---|
| 纠删码 SIMD | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/gf256_simd.rs` | 自研 GF(2^8) AVX2/NEON 双架构实现，16 子表 LUT 级联 |
| RS 引擎 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/reed_solomon.rs` | 自研 Vandermonde+Gauss-Jordan RS 引擎，PathChoice 运行时决策 |
| EC 扩展 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/erasure_coding_ext.rs` | EC 编码扩展（45KB） |
| 重建 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/rebuild.rs` | EC rebuild 作业，manifest CRC64 校验 |
| 冷热分层 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/storage_tier.rs` | 存储分层（50KB） |
| 卷服务 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/volume_server.rs` | Volume 服务入口 |
| 元数据 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/manifest.rs` | EcManifest，crc64_ecma |
| 配置 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/profile.rs` | EcProfile，data/parity shards 配置 |
| 布局 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/fs_layout.rs` | 磁盘路径布局 |
| 指标 | `platform/domains/cloud/svc/mox-cloud-volume-svc/src/metrics.rs` | 性能指标 |
| Master | `platform/domains/cloud/svc/mox-cloud-master-svc/src/raft_master.rs` | Raft 共识（50KB） |
| Master 调度 | `platform/domains/cloud/svc/mox-cloud-master-svc/src/scheduler.rs` | 卷调度（50KB） |
| Master 分配 | `platform/domains/cloud/svc/mox-cloud-master-svc/src/volume_allocator.rs` | 卷分配 |
| Master 副本 | `platform/domains/cloud/svc/mox-cloud-master-svc/src/volume_replica.rs` | 卷副本管理 |
| Master 快照 | `platform/domains/cloud/svc/mox-cloud-master-svc/src/snapshot.rs` | 快照 |
| S3 服务 | `platform/domains/cloud/svc/mox-cloud-s3-svc/` | S3 兼容服务（lifecycle/inventory/batch_ops/replication/MPU） |
| Filer 服务 | `platform/domains/cloud/svc/mox-cloud-filer-svc/` | POSIX 服务（meta_trait 多后端/posix_api/dir_entry_cache） |
| 再均衡 | `platform/domains/cloud/svc/mox-cloud-rebalance-svc/` | 再均衡（placement_strategy/rebalance_controller） |
| 核心抽象 | `platform/domains/cloud/core/` | L5 Domain Traits（规划中） |
| API 定义 | `platform/domains/cloud/api/` | API 定义 |
| SDK | `platform/domains/cloud/sdk/` | SDK |
| 服务 API | `platform/domains/cloud/svcapi/` | 服务间 API |

### 9.3 自研计划与对比文档（仓根相对路径）

| 路径 | 说明 |
|---|---|
| `docs/working-reports/20260823_cloud_drive_and_relgraph_selfdev_plan.md` | AIS 7 层架构定义、M0~M5 里程碑规划、云盘×关系图自研计划 |
| `docs/working-reports/mox-vs-opensource-comparison-report.md` | 璇玑自研 vs 开源竞品全维对比分析报告 |

---

> **文档结束** — 本文档为开发专家联盟架构决策级（ADR）文档，所有结论基于 RustFS 与自研云盘的源码实测分析，不含编造内容。RustFS 源码保持只读，不修改、不移动、不删除。
