# KG 领域 Crate 代码质量与功能完备度评测报告

> 评测范围：`platform/domains/kg/` 目录下所有 Rust crate
> 评测日期：2026-08-30
> 评测维度：文档注释、核心结构、代码量、测试完备度、README、实现状态

---

## 一、Crate 总览（共 5 个）

| 子目录 | Crate 名称 | 路径 | 分类 |
|--------|-----------|------|------|
| api/ | mox-kg-api | `kg/api/` | API 接口层 |
| core/ | mox-kg-algo-core | `kg/core/mox-kg-algo-core/` | 核心算法层 |
| core/ | mox-kg-meta-core | `kg/core/mox-kg-meta-core/` | 核心元数据层 |
| sdk/ | mox-kg-sdk | `kg/sdk/mox-kg-sdk/` | SDK 客户端层 |
| svc/ | mox-kg-fusion-svc | `kg/svc/mox-kg-fusion-svc/` | 融合服务层 |

---

## 二、详细评测表

| 评测项 | mox-kg-api | mox-kg-algo-core | mox-kg-meta-core | mox-kg-sdk | mox-kg-fusion-svc |
|--------|-----------|------------------|------------------|------------|-------------------|
| **src/ 文件数** | 1 | 4 (+ 2 bin) | 7 | 1 | 6 |
| **总代码行数** | 86 | 3,115 | 2,002 | 659 | 1,848 |
| **估算有效代码行** | ~78 | ~2,790 | ~1,881 | ~611 | ~1,607 |
| **lib.rs 文档注释** | 单行 `//!` | 完整多行 `//!`（含模块说明、算法介绍） | 完整多行 `//!`（含架构分层、模块列表） | 3 行 `//!`（说明用途） | 多行 `//!`（算法说明） |
| **核心数据结构** | 4 个 DTO + 错误枚举 | 9 个核心结构（KnowledgeGraph 等） | 20+ 个（Schema/Auth/Partition/Raft） | 11 个 DTO + GraphClient | 8 个核心结构（RrfFusion 等） |
| **核心 Trait 数** | 4（GraphStore/Analytics/Fusion/Stream） | 1（GraphAlgorithm） | 多个（RaftStorage 等） | 0（纯 struct impl） | 0（纯 struct impl） |
| **实现状态** | **API-only** | **完整实现** | **完整实现** | **完整实现** | **完整实现** |
| **tests/ 目录** | 无 | 无（但有内联单元测试） | 有（1 个测试文件） | 有（1 个测试文件） | 有（1 个测试文件） |
| **测试文件行数** | 0 | 内联 ~20 个 `#[test]` | 664 行 | 493 行 | 715 行 |
| **examples/ 数量** | 0 | 0 | 0 | 30 个 | 0 |
| **README.md** | 无 | 有（56 行，内容充实） | 有（68 行，内容充实） | 无 | 无 |
| **bin/ 工具** | 无 | 2 个（对账+公式导出） | 无 | 无 | 无 |

---

## 三、各 Crate 详细评测

### 1. mox-kg-api — API 接口定义层

**路径**：`platform/domains/kg/api/`

**代码结构**：
- 单文件 `src/lib.rs`，共 86 行
- 定义 4 个核心 trait：
  - `GraphStore` — 图存储（增删改查 + Cypher 查询）
  - `GraphAnalytics` — 图分析（PageRank、介数中心性、连通分量、最短路径、社区发现）
  - `GraphFusion` — 结果融合（RRF + 实体对齐）
  - `GraphStream` — 事件流（发布/订阅）
- 4 个 DTO 结构：`GraphNode`、`GraphEdge`、`FusionResult`、`GraphEvent`
- 错误类型 `KgApiError`（5 种变体）

**文档质量**：
- 模块级单行文档注释，说明是 trait contracts
- 部分 struct 有中文文档注释（如"知识图谱节点"）
- trait 方法缺少独立文档注释

**测试**：无任何测试

**README**：不存在

**评价**：**API-only（纯接口定义）**。作为领域接口层定位清晰，仅包含 trait 和 DTO，无任何实现逻辑。代码精简，但文档注释不够详尽，缺少示例和测试。适合作为上层服务和下层实现之间的契约层。

---

### 2. mox-kg-algo-core — 图算法核心库

**路径**：`platform/domains/kg/core/mox-kg-algo-core/`

**代码结构**：
- `src/lib.rs` — 1,623 行，主算法实现
- `src/flow_graph.rs` — 658 行，AI 流程图谱引擎
- `src/bin/compare_with_node.rs` — 416 行，与 Node.js 侧对账工具
- `src/bin/export_formula.rs` — 418 行，算法公式导出 CLI
- 总计 3,115 行，约 2,790 行有效代码

**核心功能**：
- 8 大图算法 Rust 原生零第三方重实现：
  1. PageRank（个性化 PR / 激活扩散）
  2. CNM 社区检测
  3. Brandes 介数中心性
  4. Harmonic 紧密中心性
  5. 激活扩散（Activation Spread）
  6. 模块度（Modularity）
  7. 密度（Density）
  8. RRF 融合排序
- 统一 `GraphAlgorithm<Input, Output>` trait
- `KnowledgeGraph` 主容器 + `KnowledgeGraphBuilder` 链式构建器
- AI 流程图谱引擎（意图规则、能力元、激活扩散）

**文档质量**：
- lib.rs 顶部有完整中文模块文档（算法介绍、公理说明）
- 核心结构和常量有详细文档注释
- README.md 内容充实（56 行），包含：
  - 概述与 AIS 层级说明
  - 模块结构表格
  - 关键 Trait & Impl 说明
  - 单测指引与精度护栏
  - 二次开发指引
  - TDD RED→GREEN 工作流

**测试**：
- 无独立 `tests/` 目录
- 内联单元测试：lib.rs 有 11 个 `#[test]`，flow_graph.rs 有 9 个
- README 声称断言覆盖：PageRank 转置图、CNM 社区检测模块度、Harmonic 距离、激活扩散收敛

**README**：存在，内容充实，质量高

**评价**：**完整实现（高质量核心算法库）**。代码量大、功能完整、文档详尽、有 TDD 工作流和精度护栏。两个 bin 工具提供了跨语言对账和公式导出能力。唯一不足是测试散落在源码内联中，缺少独立集成测试文件。

---

### 3. mox-kg-meta-core — 图元数据 Raft 服务

**路径**：`platform/domains/kg/core/mox-kg-meta-core/`

**代码结构**：
- `src/lib.rs` — 40 行（模块导出）
- `src/meta_server.rs` — 861 行（对外 API + Raft 集群编排）
- `src/raft_state_machine.rs` — 291 行（RaftStorage 实现 + 状态机）
- `src/schema_store.rs` — 269 行（Space/Tag/EdgeType 管理）
- `src/auth_store.rs` — 308 行（用户/角色/Policy/鉴权）
- `src/partition_store.rs` — 189 行（VID 哈希分片 + 路由）
- `src/error.rs` — 44 行（统一错误类型）
- 总计 2,002 行，约 1,881 行有效代码

**核心功能**：
- 3 节点 Raft 共识集群（基于 `async-raft 0.6`）
- Schema 管理：Space、Tag、EdgeType 的增删改查
- 权限鉴权：用户、角色、Policy、加盐 SHA-256 密码
- 分区路由：VID 哈希分片 + shard ↔ storage host 映射
- 可选 RocksDB 持久化快照（feature `persist-rocksdb`）
- 兼容 L5 `GraphMetaProvider` trait

**文档质量**：
- lib.rs 顶部有完整架构文档（架构分层图、模块列表）
- README.md 内容充实（68 行），包含：
  - 架构 ASCII 图
  - 依赖许可白名单表格
  - TDD RED/GREEN 阶段说明
  - 25 个测试用例覆盖矩阵
  - 独立运行命令
  - 与 L5 对齐说明

**测试**：
- `tests/t5_r1_meta_raft.rs` — 664 行
- 25 个测试用例，覆盖：
  - TR5.2 Raft 3 节点选举
  - TR5.3 Schema 管理（Space/Tag/EdgeType）
  - TR5.4 Auth 用户/授权/撤销
  - TR5.5 分区路由 + VID hash
  - TR5.6 白名单依赖检查
  - TR5.7 禁用品牌检查

**README**：存在，内容充实，质量高

**评价**：**完整实现（生产级 Raft 元数据服务）**。架构清晰、模块划分合理、测试覆盖全面（25 个用例覆盖 7 个需求点）。有完善的鉴权和分片路由机制，支持 RocksDB 持久化。文档和测试质量均为最高水准。

---

### 4. mox-kg-sdk — 图服务 SDK（内存模拟）

**路径**：`platform/domains/kg/sdk/mox-kg-sdk/`

**代码结构**：
- 单文件 `src/lib.rs`，共 659 行
- 约 611 行有效代码
- 无额外模块文件

**核心功能**：
- `GraphClient` — 内存模拟的图服务客户端 facade
- **CDC 功能**：消费者管理、offset 管理、去重统计、延迟监控、消费者 ID 轮转
- **Spark 连接器**：分页读节点/边、批量写、幂等 upsert、往返统计
- **图投影**：按类型/社区/标签/属性/度数进行投影过滤
- **AC-15 故障注入矩阵**：8 种故障场景（双重幂等、丢零、部分、磁盘满、审计回调、超时去重、延迟尖峰、审计回调+）
- 无网络 I/O，所有状态在内存中

**文档质量**：
- 顶部 3 行英文文档注释，说明是 in-memory fake facade
- 结构和方法有基本文档
- 缺少 README.md
- 但有 30 个 examples 文件作为使用示例

**测试**：
- `tests/test_sdk_graph.rs` — 493 行
- 覆盖：30-example 清单、CDC 生命周期、Spark 读写、投影操作、AC-15 故障矩阵、状态共享
- 30 个 examples 文件作为集成示例

**examples**：30 个，编号 graph-001 到 graph-030，覆盖：
- CDC 场景（7 个）：new、next_blocking、resume_offset、100k_writer、dedup_stats、lag_monitor、consumer_id_rotate
- Spark 场景（7 个）：paged_nodes、paged_edges、bulk、idempotent_upsert、roundtrip_2k_3k、roundtrip_5k_8k、stats_accumulate
- Projection 场景（8 个）：type_out、type_in、community_in、attr_out、attr_in、degree_out、label_in
- AC-15 故障场景（8 个）：F1/F3/F6/F7/F8/F12/F13/F14

**README**：不存在

**评价**：**完整实现（SDK/Facade 层）**。虽然是单文件，但功能密度极高，实现了 CDC、Spark、Projection、AC-15 故障注入四大类功能。30 个 examples 是最大亮点，作为使用文档和测试用例双重作用。缺少 README 是明显缺憾。

---

### 5. mox-kg-fusion-svc — 图融合服务

**路径**：`platform/domains/kg/svc/mox-kg-fusion-svc/`

**代码结构**：
- `src/lib.rs` — 301 行（RRF 融合 + 实体对齐 + 融合流水线）
- `src/graph_writer.rs` — 586 行（图写入器）
- `src/graph_projection_bridge.rs` — 369 行（图投影桥接）
- `src/tag_parser.rs` — 304 行（标签解析器）
- `src/audit_sync.rs` — 209 行（审计链同步）
- `src/cdc_stage.rs` — 79 行（CDC 阶段）
- 总计 1,848 行，约 1,607 行有效代码

**核心功能**：
- **RRF 融合引擎**：Reciprocal Rank Fusion（k=60 默认），支持加权融合
- **实体对齐器**：按规范 ID 去重 + 置信度评分
- **融合流水线**：多源结果统一排序
- **标签解析**：Tag/TagSet 解析，对象标签提取
- **图写入器**：GraphWriter，对象→标签→边的自动写入
- **审计链**：AuditChain，完整操作审计追踪
- **CDC 阶段**：tag_cdc_graph_stage，标签变更 CDC 流转
- **图投影桥接**：GraphProjectionBridge，投影操作适配

**文档质量**：
- lib.rs 顶部有英文文档注释（算法说明）
- 各子模块有基本文档
- 缺少 README.md

**测试**：
- `tests/t4_fusion_matrix.rs` — 715 行
- 14 个集成测试用例（tr1..tr14）
- 端到端测试：PutObject → Tag → CDC → GraphWriter → Audit 全链路

**README**：不存在

**评价**：**完整实现（融合服务层）**。6 个模块分工明确，覆盖了从对象标签解析到图写入、审计追踪的完整链路。RRF 融合和实体对齐是核心算法。测试覆盖全面（14 个用例、715 行），但缺少 README 文档。

---

## 四、汇总评级

### 实现状态分类

| 状态 | Crate | 数量 |
|------|-------|------|
| **完整实现** | mox-kg-algo-core、mox-kg-meta-core、mox-kg-sdk、mox-kg-fusion-svc | 4 |
| **API-only** | mox-kg-api | 1 |
| **骨架/占位** | （无） | 0 |

### 综合质量排名

| 排名 | Crate | 代码量 | 文档 | 测试 | 综合评价 |
|------|-------|--------|------|------|----------|
| 1 | mox-kg-meta-core | 2,002 行 | ★★★★★ | ★★★★★ | 架构清晰、测试最全、文档最完善 |
| 2 | mox-kg-algo-core | 3,115 行 | ★★★★★ | ★★★★☆ | 代码量最大、算法最丰富，测试内联为主 |
| 3 | mox-kg-fusion-svc | 1,848 行 | ★★★☆☆ | ★★★★☆ | 功能完整，测试充分，缺 README |
| 4 | mox-kg-sdk | 659 行 | ★★☆☆☆ | ★★★★☆ | 单文件但功能密度高，30 个 examples 亮眼，缺 README |
| 5 | mox-kg-api | 86 行 | ★★☆☆☆ | ☆☆☆☆☆ | 纯接口定义，定位清晰但文档和测试不足 |

---

## 五、问题与改进建议

### 共性问题

1. **README 覆盖率不足**：5 个 crate 中仅 2 个有 README（40%）
   - 建议：mox-kg-api、mox-kg-sdk、mox-kg-fusion-svc 补充 README.md

2. **mox-kg-api 完全无测试**
   - 建议：至少添加 trait 边界测试和 DTO 序列化测试

3. **mox-kg-algo-core 测试散落在内联中**
   - 建议：将集成测试移至 `tests/` 目录，提升可维护性

4. **文档语言不统一**
   - 部分 crate 用中文文档（algo-core、meta-core），部分用英文（api、sdk、fusion-svc）
   - 建议：统一文档语言策略

### 各 Crate 具体建议

| Crate | 改进建议 |
|-------|----------|
| mox-kg-api | 补充 README、增加 trait 文档注释、添加基础单元测试 |
| mox-kg-algo-core | 将内联测试迁移到 `tests/` 目录、增加集成测试 |
| mox-kg-meta-core | （已较完善，无重大改进点） |
| mox-kg-sdk | 补充 README（可基于 30 个 examples 组织）、将大文件拆分为多模块 |
| mox-kg-fusion-svc | 补充 README、完善模块级文档注释 |

---

## 六、统计数据汇总

- **Crate 总数**：5 个
- **总代码行数**：~7,710 行
- **总测试行数**：~1,872 行（不含 algo-core 内联测试）
- **README 覆盖率**：2/5 = 40%
- **测试覆盖率**（有 tests/ 目录）：3/5 = 60%
- **完整实现率**：4/5 = 80%
