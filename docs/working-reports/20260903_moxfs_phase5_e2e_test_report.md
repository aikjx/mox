# MOXFS 阶段五 · 全链路端到端测试报告

| 项目 | 内容 |
|------|------|
| 文档编号 | E2E-MOXFS-P5-20260903 |
| 版本 | V1.0 |
| 日期 | 2026-09-03 |
| 项目 | moxfs 全自研云盘知识库 |
| 阶段 | 阶段五 · 企业级质量加固与全链路验证 |
| 测试文件 | `platform/domains/cloud/svc/mox-cloud-s3-svc/tests/t_e2e_phase5.rs` |
| 对标参考对象 | RustFS（Apache 2.0，源码位于 `ais/RustFS/`） |

---

## 1. 测试架构说明

### 1.1 架构概述

阶段五端到端测试采用 **InMemoryStorageBackend + Mock 故障注入 + SHA-256 数据完整性校验** 的三层架构，在不依赖真实 S3 存储的前提下，完整验证从 S3 API 入口到存储后端落盘的全链路行为。

```
┌─────────────────────────────────────────────────────────┐
│                    S3 API 入口层                          │
│  (PutObject / GetObject / DeleteObject / DeleteObjects)  │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                  业务逻辑层                                │
│  (MultiWriter / HedgedReader / ReaderPipeline /           │
│   ReedSolomon / LifecycleEvaluator / Backpressure)        │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│              InMemoryStorageBackend（Mock 层）             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  Backend A  │  │  Backend B  │  │  Backend C  │ ... │
│  │ (正常/故障)  │  │ (正常/慢)    │  │ (正常/故障)  │     │
│  └─────────────┘  └─────────────┘  └─────────────┘     │
│  故障注入：put 失败 / get 慢 / get 失败 / 延迟注入        │
└─────────────────────────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│              数据完整性校验层                               │
│  (SHA-256 hash 校验 / chunk_id 格式验证 / 元数据一致性)   │
└─────────────────────────────────────────────────────────┘
```

### 1.2 核心组件

| 组件 | 作用 |
|------|------|
| `InMemoryStorageBackend` | 内存存储后端，模拟真实 S3 存储的 put/get/delete 行为 |
| `FaultInjectingBackend` | 故障注入包装器，可配置 put 失败率、get 延迟、get 失败率 |
| `Sha256Verifier` | SHA-256 数据完整性校验，写入时计算 hash，读取时验证 |
| `MetadataChecker` | 元数据一致性校验，验证 chunk_id、size、content_type 等字段 |

### 1.3 测试运行命令

```bash
cd platform/domains/cloud/svc/mox-cloud-s3-svc
cargo test --test t_e2e_phase5
```

---

## 2. 五条全链路场景详细描述

### 2.1 场景一：对象写入全链路

#### 流程图

```
客户端 PutObject 请求
    │
    ▼
S3 Service 接收请求 → 校验参数（bucket/key/size）
    │
    ▼
MultiWriter 分发到 N 个存储后端（并发写入）
    │
    ▼
ReedSolomon 编码（k+m shard）
    │
    ▼
各后端 put shard 数据
    │
    ▼
等待 Quorum 确认（至少 k 个成功）
    │
    ▼
元数据持久化（chunk_id / size / content_type / sha256）
    │
    ▼
返回 PutObject 响应（ETag = chunk_id）
```

#### 测试用例列表（5 个）

| 测试名 | 验证点 |
|--------|--------|
| `test_e2e_write_1kb_object` | 1KB 小对象写入全链路，SHA-256 校验通过，chunk_id 格式正确 |
| `test_e2e_write_256kb_object` | 256KB 中等对象写入，验证多分片编码正确性 |
| `test_e2e_write_4mb_object` | 4MB 大对象写入，验证多分片 + 多后端并发写入 |
| `test_e2e_write_sha256_integrity` | 写入后独立计算 SHA-256，与元数据中存储的 hash 一致 |
| `test_e2e_write_metadata_consistency` | chunk_id 格式（`{k}-{m}-{hash}`）、size、content_type 元数据一致性 |

#### 故障注入方式

本场景为正常路径，不注入故障。

---

### 2.2 场景二：对象读取全链路

#### 流程图

```
客户端 GetObject 请求（含可选 Range）
    │
    ▼
S3 Service 接收请求 → 查找元数据
    │
    ▼
HedgedReader 发起并发读取（主请求 + 对冲请求）
    │
    ▼
ReaderPipeline 多后端透明切换（后端失败自动降级）
    │
    ▼
ReedSolomon 解码（从 k 个 shard 重建数据）
    │
    ▼
数据校验（SHA-256 完整性校验）
    │
    ▼
返回对象数据（或 Range 切片）
```

#### 测试用例列表（5 个）

| 测试名 | 验证点 |
|--------|--------|
| `test_e2e_read_normal` | 正常读取完整对象，SHA-256 校验通过 |
| `test_e2e_read_range` | Range 请求（bytes=0-1023），返回正确切片 |
| `test_e2e_read_404` | 读取不存在的对象，返回 404 NoSuchKey |
| `test_e2e_read_reader_pipeline_multi_backend` | ReaderPipeline 多后端场景，主后端失败后自动切换备用后端 |
| `test_e2e_read_5mb_large_object` | 5MB 大对象读取，验证多分片解码 + 数据完整性 |

#### 故障注入方式

- `test_e2e_read_reader_pipeline_multi_backend`：主后端 get 注入失败，备用后端正常

---

### 2.3 场景三：对象删除全链路

#### 流程图

```
客户端 DeleteObject / DeleteObjects 请求
    │
    ▼
S3 Service 接收请求 → 查找元数据
    │
    ▼
MultiWriter 并发删除各后端 shard
    │
    ▼
元数据标记删除（软删除 / 硬删除）
    │
    ▼
返回删除响应
```

#### 测试用例列表（3 个）

| 测试名 | 验证点 |
|--------|--------|
| `test_e2e_delete_normal` | 正常删除对象，删除后读取返回 404 |
| `test_e2e_delete_batch` | 批量删除（DeleteObjects），多个对象同时删除 |
| `test_e2e_delete_idempotent` | 幂等删除：删除不存在的对象不报错，重复删除结果一致 |

#### 故障注入方式

本场景不注入故障，重点验证幂等性。

---

### 2.4 场景四：故障恢复全链路

#### 流程图

```
正常操作中注入故障
    │
    ├── MultiWriter 部分后端失败 → Quorum 降级 → 仍可写入
    ├── 部分 shard 丢失 → ReedSolomon 重建 → 数据完整
    ├── 全部后端失败 → 返回错误 → 不丢数据
    ├── HedgedReader 慢后端 → 对冲请求接管 → 延迟可控
    ├── 存储后端 put 失败 → 写入降级 → 元数据一致
    └── 过多 shard 丢失（>m）→ 重建失败 → 返回明确错误
```

#### 测试用例列表（6 个）

| 测试名 | 故障注入 | 验证点 |
|--------|---------|--------|
| `test_e2e_fault_multi_writer_partial_failure` | N 个后端中 1 个 put 失败 | MultiWriter 仍可写入（Quorum 满足），数据完整 |
| `test_e2e_fault_erasure_coding_reconstruction` | 模拟 m 个 shard 丢失 | ReedSolomon 从剩余 k 个 shard 重建，SHA-256 校验通过 |
| `test_e2e_fault_all_backends_fail` | 全部后端 put 失败 | 返回明确错误，不产生脏数据，元数据一致 |
| `test_e2e_fault_hedged_reader_slow_backend` | 主后端 get 注入 500ms 延迟 | HedgedReader 对冲请求在超时后接管，总延迟 < 主后端延迟 |
| `test_e2e_fault_storage_backend_put_failure` | 特定后端 put 注入失败 | 写入降级到其他后端，元数据记录实际写入位置 |
| `test_e2e_fault_too_many_shards_lost` | 模拟 >m 个 shard 丢失 | ReedSolomon 重建失败，返回明确错误（不返回损坏数据） |

#### 故障注入方式

- 使用 `FaultInjectingBackend` 包装 `InMemoryStorageBackend`
- 支持配置：`put_fail_count`（前 N 次 put 失败）、`get_delay_ms`（get 延迟）、`get_fail_count`（前 N 次 get 失败）

---

### 2.5 场景五：生命周期迁移全链路

#### 流程图

```
LifecycleEvaluator 定期扫描
    │
    ▼
transition_scan：筛选符合迁移条件的对象（年龄 > 阈值）
    │
    ▼
迁移执行：从热存储层迁移到冷存储层
    │
    ▼
元数据更新：storage_class 标记为冷存储
    │
    ▼
迁移后透明读取：客户端无感知，ReaderPipeline 自动从冷层读取
    │
    ▼
过期对象清理：expiration_scan 删除过期对象
    │
    ▼
扫描预算控制：单次扫描不超过预算上限，防止影响在线业务
```

#### 测试用例列表（5 个）

| 测试名 | 验证点 |
|--------|--------|
| `test_e2e_lifecycle_evaluator_trait` | LifecycleEvaluator trait 接口正确性，evaluate 返回正确的迁移/过期决策 |
| `test_e2e_lifecycle_transition_scan` | transition_scan 正确筛选符合条件的对象，迁移后 storage_class 更新 |
| `test_e2e_lifecycle_transparent_read_after_migration` | 迁移后读取对象，数据完整（SHA-256 校验），客户端无感知 |
| `test_e2e_lifecycle_expiration` | 过期对象被 expiration_scan 清理，清理后读取返回 404 |
| `test_e2e_lifecycle_scan_budget` | 扫描预算控制：对象数超过预算时，单次扫描只处理预算内数量 |

#### 故障注入方式

本场景不注入故障，重点验证生命周期管理逻辑正确性。

---

## 3. 测试结果汇总

### 3.1 总体结果

| 指标 | 数值 |
|------|------|
| 测试总数 | **24** |
| 通过 | **24** |
| 失败 | **0** |
| 跳过 | **0** |
| 通过率 | **100%** |

### 3.2 分场景结果

| 场景 | 测试数 | 通过 | 失败 | 通过率 |
|------|--------|------|------|--------|
| 场景一：对象写入全链路 | 5 | 5 | 0 | 100% |
| 场景二：对象读取全链路 | 5 | 5 | 0 | 100% |
| 场景三：对象删除全链路 | 3 | 3 | 0 | 100% |
| 场景四：故障恢复全链路 | 6 | 6 | 0 | 100% |
| 场景五：生命周期迁移全链路 | 5 | 5 | 0 | 100% |
| **合计** | **24** | **24** | **0** | **100%** |

### 3.3 验证命令与输出

```bash
$ cargo test --test t_e2e_phase5
running 24 tests
test test_e2e_write_1kb_object ... ok
test test_e2e_write_256kb_object ... ok
test test_e2e_write_4mb_object ... ok
test test_e2e_write_sha256_integrity ... ok
test test_e2e_write_metadata_consistency ... ok
test test_e2e_read_normal ... ok
test test_e2e_read_range ... ok
test test_e2e_read_404 ... ok
test test_e2e_read_reader_pipeline_multi_backend ... ok
test test_e2e_read_5mb_large_object ... ok
test test_e2e_delete_normal ... ok
test test_e2e_delete_batch ... ok
test test_e2e_delete_idempotent ... ok
test test_e2e_fault_multi_writer_partial_failure ... ok
test test_e2e_fault_erasure_coding_reconstruction ... ok
test test_e2e_fault_all_backends_fail ... ok
test test_e2e_fault_hedged_reader_slow_backend ... ok
test test_e2e_fault_storage_backend_put_failure ... ok
test test_e2e_fault_too_many_shards_lost ... ok
test test_e2e_lifecycle_evaluator_trait ... ok
test test_e2e_lifecycle_transition_scan ... ok
test test_e2e_lifecycle_transparent_read_after_migration ... ok
test test_e2e_lifecycle_expiration ... ok
test test_e2e_lifecycle_scan_budget ... ok
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## 4. 数据完整性验证方法

### 4.1 SHA-256 全链路校验

所有写入和读取测试均执行 SHA-256 数据完整性校验：

1. **写入时**：客户端构造随机数据 → 计算 SHA-256 hash（记为 `expected_hash`）→ 调用 PutObject 写入
2. **读取时**：调用 GetObject 读取数据 → 计算返回数据的 SHA-256 hash（记为 `actual_hash`）→ 断言 `actual_hash == expected_hash`
3. **元数据校验**：读取元数据中存储的 `sha256` 字段 → 断言与 `expected_hash` 一致

### 4.2 校验覆盖范围

| 场景 | SHA-256 校验 |
|------|-------------|
| 对象写入（1KB/256KB/4MB） | ✅ 写入后读取校验 |
| 对象读取（正常/Range/5MB） | ✅ 读取数据校验 |
| 故障恢复（纠删码重建/部分失败） | ✅ 重建后数据校验 |
| 生命周期迁移（迁移后透明读取） | ✅ 迁移后读取校验 |

### 4.3 chunk_id 格式验证

chunk_id 采用 `{k}-{m}-{data_hash}` 格式，测试中验证：
- k 和 m 为正整数，且 k + m ≤ 最大 shard 数
- data_hash 为 64 字符十六进制字符串（SHA-256）
- chunk_id 全局唯一

---

## 5. 故障注入验证结果

### 5.1 MultiWriter 部分失败

- **注入方式**：N 个后端中 1 个 `put_fail_count = 1`
- **验证结果**：✅ MultiWriter 仍可成功写入（Quorum 满足），写入后读取数据完整，SHA-256 校验通过
- **关键指标**：成功写入的后端数 ≥ k（Quorum 阈值）

### 5.2 HedgedReader 慢后端

- **注入方式**：主后端 `get_delay_ms = 500`
- **验证结果**：✅ HedgedReader 在主后端超时后发起对冲请求，对冲请求从备用后端成功读取，总延迟显著低于 500ms
- **关键指标**：对冲请求触发条件 = 主后端延迟 > hedged_timeout

### 5.3 存储后端 put 失败

- **注入方式**：特定后端 `put_fail_count = 1`
- **验证结果**：✅ 写入降级到其他后端，元数据记录实际写入的后端列表，后续读取从正确后端获取数据
- **关键指标**：元数据中 `stored_backends` 与实际成功写入的后端一致

### 5.4 纠删码重建

- **注入方式**：模拟 m 个 shard 丢失（删除对应后端数据）
- **验证结果**：✅ ReedSolomon 从剩余 k 个 shard 成功重建原始数据，SHA-256 校验通过
- **关键指标**：丢失 shard 数 ≤ m（容错上限）

### 5.5 过多 shard 丢失

- **注入方式**：模拟 >m 个 shard 丢失
- **验证结果**：✅ ReedSolomon 重建失败，返回明确错误（`ErasureCodingError::TooManyErasures`），不返回损坏数据
- **关键指标**：错误类型正确，不产生静默数据损坏

---

## 6. 元数据一致性验证

### 6.1 验证项

| 验证项 | 说明 | 状态 |
|--------|------|------|
| chunk_id 一致性 | 写入返回的 chunk_id 与元数据中存储的一致 | ✅ |
| size 一致性 | 元数据中 size 与实际数据大小一致 | ✅ |
| content_type 一致性 | 元数据中 content_type 与写入时指定的一致 | ✅ |
| sha256 一致性 | 元数据中 sha256 与实际数据 SHA-256 一致 | ✅ |
| storage_class 一致性 | 生命周期迁移后 storage_class 正确更新 | ✅ |
| stored_backends 一致性 | 元数据中 stored_backends 与实际写入后端一致 | ✅ |

### 6.2 验证方法

每次写入后，通过元数据查询接口获取元数据，与写入时的期望值逐字段比对。

---

## 7. 现有测试无回归确认

### 7.1 回归测试范围

| 测试套件 | 测试数 | 结果 |
|----------|--------|------|
| t6_m2_s3_service（S3 服务集成测试） | 333 | ✅ 全绿 |
| t_integration_s3（S3 集成测试） | 50 | ✅ 全绿 |
| lib 单元测试（mox-cloud-kernel） | 113 | ✅ 全绿 |
| **合计** | **496** | **✅ 全绿** |

### 7.2 回归验证命令

```bash
# S3 服务集成测试
cargo test --test t6_m2_s3_service

# S3 集成测试
cargo test --test t_integration_s3

# kernel lib 单元测试
cargo test -p mox-cloud-kernel --lib
```

### 7.3 回归结论

阶段五新增的 24 个 e2e 测试未对现有测试造成任何回归，所有现有测试保持全绿。阶段五期间修复的 `src/replication.rs` 测试模块预存在的 S3Error 导入缺失问题，也未引入新的回归。

---

## 8. 结论与后续建议

### 8.1 结论

1. **24 个 e2e 测试全部通过**，覆盖 5 条全链路场景（写入/读取/删除/故障恢复/生命周期迁移）
2. **数据完整性**：所有读写路径均通过 SHA-256 全链路校验，无静默数据损坏
3. **故障恢复**：6 个故障恢复场景全部验证通过，包括部分失败、纠删码重建、慢后端对冲、过多 shard 丢失等边界情况
4. **元数据一致性**：所有元数据字段（chunk_id/size/content_type/sha256/storage_class/stored_backends）一致性验证通过
5. **无回归**：现有 496 个测试（333+50+113）全部保持全绿

### 8.2 后续建议

| 优先级 | 建议 | 说明 |
|--------|------|------|
| P1 | 增加真实 S3 后端 e2e 测试 | 当前使用 InMemoryStorageBackend，后续可增加真实 MinIO/S3 后端的 e2e 测试 |
| P1 | 增加混沌工程测试 | 引入随机故障注入（网络分区、磁盘满、超时等），验证系统在混沌环境下的稳定性 |
| P2 | 增加性能 e2e 测试 | 在 e2e 测试中增加延迟和吞吐量指标采集，建立性能基线 |
| P2 | 增加多 bucket 并发测试 | 验证多 bucket 并发读写场景下的隔离性和性能 |
| P3 | e2e 测试纳入 CI 门禁 | 将 24 个 e2e 测试纳入 CI 流水线，每次 PR 自动运行 |

---

*报告基于代码实测数据生成，所有测试结果来自 `cargo test --test t_e2e_phase5` 实际运行结果。RustFS 为对标参考对象（Apache 2.0），moxfs 为全自研实现。*
