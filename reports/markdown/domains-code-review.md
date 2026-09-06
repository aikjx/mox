# 三大域 Rust 模块代码质量与功能完备度评测报告

> 评测范围：`platform/domains/cloud/`、`platform/domains/data/`、`platform/domains/flow/`
> 评测日期：2026-08-30

---

## 一、总体概览

| 域 | Crate 数 | src 文件数 | 代码总行数 | 测试用例数 | README 数 | 完整实现 | 实质实现 | 骨架/API-only |
|----|---------|-----------|-----------|-----------|----------|---------|---------|--------------|
| cloud | 6 | 44 | 11,160 | 50 | 2 | 2 | 3 | 1 |
| data | 10 | 39 | 10,373 | 162 | 2 | 2 | 6 | 2 |
| flow | 7 | 58 | 14,160 | 78 | 6 | 3 | 3 | 1 |
| **合计** | **23** | **141** | **35,693** | **290** | **10** | **7** | **12** | **4** |

---

## 二、状态分级定义

| 等级 | 定义 |
|------|------|
| **完整实现** | 功能模块完整、代码量充足（>2000 行或多模块）、有集成测试、有 README 文档 |
| **实质实现** | 核心功能已实现、有一定代码量（500~2000 行）、有或无测试、结构清晰 |
| **骨架占位** | 有基本模块结构和类型定义，但核心逻辑较薄，测试缺失 |
| **API-only** | 仅定义 trait 契约和数据结构，无具体业务实现（作为跨域接口层是合理的） |

---

## 三、Cloud 域详细评测

### Crate 列表

| # | Crate 名称 | 子目录 | src 文件 | 代码行数 | 测试数 | 测试文件 | 示例数 | README | 状态评级 |
|---|-----------|--------|---------|---------|-------|---------|-------|--------|---------|
| 1 | mox-cloud-api | api | 1 | 100 | 0 | 0 | 0 | 否 | **API-only** |
| 2 | mox-cloud-sdk | sdk | 1 | 724 | 0 | 1 | 38 | 否 | **实质实现** |
| 3 | mox-cloud-filer-svc | svc | 9 | 1,352 | 10 | 1 | 0 | 否 | **实质实现** |
| 4 | mox-cloud-master-svc | svc | 6 | 933 | 23 | 1 | 0 | 是 | **实质实现** |
| 5 | mox-cloud-s3-svc | svc | 16 | 5,223 | 0 | 1 | 1 | 否 | **完整实现** |
| 6 | mox-cloud-volume-svc | svc | 11 | 2,828 | 17 | 1 | 0 | 是 | **完整实现** |

### 各 Crate 简评

**1. mox-cloud-api** — API-only
- 仅定义 `CloudApiError`、`CloudVolume`、`S3Bucket`、`ResourceStatus` 等数据结构和基础类型
- 作为跨域接口契约层，定位合理

**2. mox-cloud-sdk** — 实质实现
- 内存模拟的 S3/STS/IAM/Quota/WORM/Lifecycle/HashChain 统一门面
- 单文件 724 行，功能覆盖面广但全部集中在一个文件中
- 38 个 examples 覆盖 S3 操作、STS、IAM、配额、WORM、生命周期等场景
- 无单元测试（测试通过 examples 体现）
- **问题**：单文件过大，建议拆分模块；无 README

**3. mox-cloud-filer-svc** — 实质实现
- POSIX Filer 服务，支持 SQLite / Postgres+Citus / Redis 三种元数据后端
- 自研 FUSE 客户端（模拟 mount/ls/write，避免跨平台依赖）
- 9 个模块，结构清晰
- 10 个测试用例
- **问题**：无 README；`#![allow(dead_code)]` 标记存在未使用代码

**4. mox-cloud-master-svc** — 实质实现
- 云盘控制面：卷注册/心跳、卷分配、副本 quorum、快照管理、集群状态
- 6 个模块，结构清晰
- 23 个测试，测试覆盖较好
- 有 README

**5. mox-cloud-s3-svc** — 完整实现
- S3 兼容服务，100% 自研，34 个 API 全量实现
- 覆盖：桶管理、对象操作、列表、MPU（分片上传）、Versioning、Policy、Lifecycle、CORS 等
- 16 个模块，架构完整
- 签名复用 mox-standards sigv4，ETag 复用 etag_crc32c
- **问题**：无 README；测试文件存在但 test_count=0（可能用的是集成测试方式）

**6. mox-cloud-volume-svc** — 完整实现
- 云盘数据面：chunk 读写、容量控制、自研 RS(2+1 XOR) 纠删码、chunk 重建
- 额外提供完整 Reed-Solomon(n+k) over GF(2^8) EC 引擎（含 SIMD 优化）
- 11 个模块，含 profile/manifest/fs_layout/rebuild/metrics 等完整组件
- 17 个测试用例
- 有 README

---

## 四、Data 域详细评测

### Crate 列表

| # | Crate 名称 | 子目录 | src 文件 | 代码行数 | 测试数 | 测试文件 | 示例数 | README | 状态评级 |
|---|-----------|--------|---------|---------|-------|---------|-------|--------|---------|
| 1 | mox-data-api | api | 1 | 152 | 0 | 0 | 0 | 否 | **API-only** |
| 2 | mox-data-formula-core | core | 6 | 1,423 | 0 | 0 | 0 | 否 | **完整实现** |
| 3 | mox-data-norm-core | core | 4 | 644 | 0 | 0 | 0 | 否 | **实质实现** |
| 4 | mox-data-standards-core | core | 10 | 3,556 | 141 | 1 | 1 | 是 | **完整实现** |
| 5 | mox-data-formula-native | sdk | 1 | 306 | 0 | 0 | 0 | 否 | **实质实现** |
| 6 | mox-data-norm-intent-native | sdk | 1 | 218 | 0 | 0 | 0 | 否 | **实质实现** |
| 7 | mox-data-catalog-svc | svc | 3 | 1,436 | 3 | 1 | 0 | 是 | **实质实现** |
| 8 | mox-data-compliance-svc | svc | 4 | 777 | 18 | 1 | 0 | 否 | **实质实现** |
| 9 | mox-data-etl-svc | svc | 4 | 696 | 0 | 0 | 0 | 否 | **骨架占位** |
| 10 | mox-data-plane-svc | svc | 5 | 1,165 | 0 | 0 | 0 | 否 | **实质实现** |

### 各 Crate 简评

**1. mox-data-api** — API-only
- 定义 `DataRecord`、`RecordMetadata`、`DataApiError` 等跨域数据类型
- 作为接口层合理

**2. mox-data-formula-core** — 完整实现
- 12 项图公式的 Rust 最高性能实现：密度、度中心性、介数中心性（Brandes 并行）、紧密中心性、PageRank（Gauss-Seidel）、PPR、CNM 社区检测、模块度、K-Core、特征向量中心性、三角计数、同配系数
- 有严格的精度护栏常量（PPR_D=0.85、PPR_MAX_ITER=30 等）
- **问题**：无单元测试文件（测试可能集成在其他地方）；无 README

**3. mox-data-norm-core** — 实质实现
- 归一化流水线：去重（Ahash 指纹）、规则求解器、冲突融合、增量字段合并
- 4 个模块，功能完整
- **问题**：无独立测试文件（仅内联 smoke 测试）；无 README

**4. mox-data-standards-core** — 完整实现
- 10 项标准矩阵：SM3、SM2、SM4、HMAC-SHA256（FIPS）、SigV4、RFC5424、ETag CRC32C、登堡哈希链、STS-SM2
- 141 个测试用例，测试覆盖非常充分
- 有 README 和 tasks.md
- 国密算法通过 feature flag (`gm-sm`) 控制

**5. mox-data-formula-native** — 实质实现
- napi-rs 绑定：将 formula-core 暴露为 Node.js 原生模块
- 单文件 306 行，封装完整
- **问题**：无 README；无测试

**6. mox-data-norm-intent-native** — 实质实现
- napi-rs 绑定：norm-core + intent-core 的 Node.js 原生模块
- 单文件 218 行
- **问题**：无 README；无测试

**7. mox-data-catalog-svc** — 实质实现
- 业务全景目录：流程图 + 六维关系网建模
- 遵循 DIP（依赖反转原则），仅依赖 expert_traits 和投影类型
- 包含空间光速螺旋模型分析算子（spiral 模块）
- **问题**：3 个测试较少

**8. mox-data-compliance-svc** — 实质实现
- PII 检测、分类、脱敏，支持 GDPR/CCPA/PCI-DSS/HIPAA
- 13 种 PII 类型（邮箱、电话、身份证、IP 等）
- 18 个测试
- 模块：audit_record、legal_hold、miji
- **问题**：无 README

**9. mox-data-etl-svc** — 骨架占位
- ETL 管线引擎定义：Source/Transform/Sink trait
- 4 个模块：abi、context、registry
- **问题**：无测试；无 README；具体 source/sink 实现似乎较少

**10. mox-data-plane-svc** — 实质实现
- 统一数据面：摄入、转换、路由，支持流/批处理
- 模块：fshc、listeners、mountpath、multipart
- **问题**：无测试；无 README

---

## 五、Flow 域详细评测

### Crate 列表

| # | Crate 名称 | 子目录 | src 文件 | 代码行数 | 测试数 | 测试文件 | 示例数 | README | 状态评级 |
|---|-----------|--------|---------|---------|-------|---------|-------|--------|---------|
| 1 | mox-flow-api | api | 1 | 116 | 0 | 0 | 0 | 否 | **API-only** |
| 2 | mox-flow-operator-core | core | 12 | 2,365 | 52 | 3 | 0 | 是 | **完整实现** |
| 3 | mox-flow-optimizer-core | core | 1 | 320 | 0 | 0 | 0 | 是 | **骨架占位** |
| 4 | mox-flow-bridge-svc | svc | 13 | 1,488 | 11 | 2 | 0 | 是 | **实质实现** |
| 5 | mox-flow-fusion-svc | svc | 11 | 3,950 | 0 | 1 | 2 | 是 | **完整实现** |
| 6 | mox-flow-operator-wasm-svc | svc | 1 | 564 | 0 | 0 | 0 | 是 | **实质实现** |
| 7 | mox-flow-primiflow-svc | svc | 19 | 5,357 | 15 | 4 | 19 | 是 | **完整实现** |

### 各 Crate 简评

**1. mox-flow-api** — API-only
- 定义 `FlowDefinition`、`FlowStatus`、`NodeStatus`、`FlowApiError` 等类型
- 接口层定位合理

**2. mox-flow-operator-core** — 完整实现
- 算子统一系统核心库，实现六条数学公理
- 12 个模块：kernel（纯内核零外部依赖）、kernel_ext、category、conservation、engine、monad、operator、registry、resource、state、types
- L6 纯内核层与 L5 扩展层分层清晰（DIP）
- 52 个测试，3 个测试文件，测试覆盖较好
- 有 README，有 benches

**3. mox-flow-optimizer-core** — 骨架占位
- 基于 DAG 的算子调度优化器
- 单文件 320 行，仅实现基本的 DAG 构建和拓扑排序
- 依赖 petgraph
- **问题**：实现较薄；无测试；优化逻辑（资源约束、调度算法）似乎尚未充分展开

**4. mox-flow-bridge-svc** — 实质实现
- Hermes Flow Bridge：零侵入插件，将 flow-ai + mox-expert 注入 Hermes Agent Ultra
- 13 个模块：bridge、hooks、mini_hermes、normalize、plugin、recorder、router、state、integration
- 11 个测试，2 个测试文件
- 有 README 和 DESIGN.md
- 通过 feature flag 控制 hermes/live 集成

**5. mox-flow-fusion-svc** — 完整实现
- PrimiFlow 多维度融合归一化一体化架构层
- 11 个模块：unified（统一图模型）、envelope（跨层信封）、registry（能力融合）、platform（一体化编排）、config、observability、server、sixdim、ptdoc
- 企业级 REST 服务层（Bearer 鉴权 / CORS / 六维溯源查询）
- 有 Dockerfile、.dockerignore
- 有 README，有 PT-DOC 文档集（10 篇）
- **问题**：测试文件存在但 test_count=0（可能是集成测试方式）

**6. mox-flow-operator-wasm-svc** — 实质实现
- WASM 算子插件系统：热加载、类型检查、资源隔离
- O3 补丁：fuel 指令预算、内存页数硬上限、执行遥测
- 基于 wasmer 实现
- **问题**：单文件 564 行，建议拆分；无测试

**7. mox-flow-primiflow-svc** — 完整实现
- PrimiFlow 全域原语智能平台：关联图谱驱动的需求→代码/数据骨架生成
- 19 个源文件，7 大模块：assoc、executor、generate、parse、persistence、runner、server、gen
- 15 个测试，4 个测试文件，19 个示例
- 企业级端到端场景验证
- 有 README
- 复用 flow-ai 的 κ-τ 拓扑原语引擎

---

## 六、汇总对比表

### 按域汇总

| 维度 | cloud | data | flow | 评价 |
|------|-------|------|------|------|
| Crate 数量 | 6 | 10 | 7 | data 域 crate 最多，分层最细 |
| 总代码量 | 11,160 行 | 10,373 行 | 14,160 行 | flow 域代码量最大 |
| 平均每 crate | 1,860 行 | 1,037 行 | 2,023 行 | flow 单 crate 平均规模最大 |
| 测试总数 | 50 | 162 | 78 | data 域测试最多（standards-core 贡献 141） |
| 测试密度（行/测试） | 223 | 64 | 182 | data 域测试密度最高 |
| README 覆盖率 | 33% (2/6) | 20% (2/10) | 86% (6/7) | flow 域文档最完善 |
| 完整实现占比 | 33% | 20% | 43% | flow 域成熟度最高 |

### 按分层汇总

| 分层 | cloud | data | flow | 说明 |
|------|-------|------|------|------|
| api | 1 (API-only) | 1 (API-only) | 1 (API-only) | 三个域均有独立 api crate，架构一致 |
| core | 无 | 3 (formula/norm/standards) | 2 (operator/optimizer) | data 域 core 层最丰富；cloud 无 core 层 |
| svc | 4 | 4 | 4 | svc 层数量相当 |
| sdk | 1 | 2 | 无 | cloud/data 有 Node.js 原生绑定 SDK；flow 无 |

---

## 七、总体问题清单

### P0 — 关键问题

1. **测试覆盖不均衡**
   - data 域 162 个测试中，`mox-data-standards-core` 独占 141 个（87%），其余 9 个 crate 合计仅 21 个
   - `mox-cloud-s3-svc`（5223 行，S3 全量实现）零单元测试
   - `mox-flow-fusion-svc`（3950 行）零单元测试
   - `mox-data-etl-svc`、`mox-data-plane-svc` 零测试
   - 所有 api / sdk crate 均零测试

2. **部分 crate 实现深度不足**
   - `mox-flow-optimizer-core`：单文件 320 行，仅基础 DAG 拓扑排序，与"优化器"定位有差距
   - `mox-data-etl-svc`：仅 trait 定义和基础结构，具体 source/sink 实现较少

### P1 — 重要问题

3. **README 覆盖率低**
   - 整体 README 覆盖率仅 43%（10/23）
   - data 域最低：20%（2/10）
   - cloud 域：33%（2/6）
   - 核心 crate 如 `mox-cloud-s3-svc`、`mox-data-formula-core` 均无 README

4. **单文件 crate 过多**
   - 共 8 个 crate 仅有 1 个 src 文件（api 层 3 个 + sdk 层 3 个 + optimizer-core + wasm-svc）
   - `mox-cloud-sdk` 单文件 724 行实现全部功能，建议拆分
   - `mox-flow-operator-wasm-svc` 单文件 564 行，建议拆分模块

5. **cloud 域缺少 core 层**
   - cloud 域仅有 api/sdk/svc 三层，缺少独立的 core 层
   - S3 核心逻辑、volume 核心算法目前都在 svc 层，不利于复用和测试

### P2 — 改进建议

6. **命名一致性**
   - data 域 svc 命名：`mox-data-catalog-svc`、`mox-data-compliance-svc`、`mox-data-etl-svc`、`mox-data-plane-svc`
   - flow 域 svc 命名：`mox-flow-bridge-svc`、`mox-flow-fusion-svc`、`mox-flow-operator-wasm-svc`、`mox-flow-primiflow-svc`
   - cloud 域 svc 命名：`mox-cloud-filer-svc`、`mox-cloud-master-svc`、`mox-cloud-s3-svc`、`mox-cloud-volume-svc`
   - 各有风格，建议统一命名规范

7. **SDK 层测试缺失**
   - `mox-data-formula-native` 和 `mox-data-norm-intent-native` 均为 napi-rs 绑定，无测试
   - 建议增加 Node.js 侧集成测试或 Rust 侧单元测试

8. **examples 数量不均**
   - `mox-cloud-sdk` 有 38 个 examples，远超其他 crate
   - `mox-flow-primiflow-svc` 有 19 个
   - 多数 crate 无 examples

9. **未使用代码标记**
   - `mox-cloud-filer-svc` 使用了 `#![allow(dead_code)]`，存在未使用代码
   - 建议清理 dead code 或明确标记原因

---

## 八、总结

**整体成熟度排序：flow > cloud > data**

- **flow 域**：代码量最大（14k 行）、README 覆盖率最高（86%）、完整实现比例最高（43%），架构最完善。算子核心、融合层、PrimiFlow 平台均达到完整实现水平。
- **cloud 域**：6 个 crate 分工清晰，S3 和 volume 服务完整度高，SDK 示例丰富。但缺少 core 层抽象，README 不足。
- **data 域**：crate 数量最多（10个），分层最细，standards-core 测试质量极高。但整体测试分布极不均衡，多个 svc crate 实现偏薄，README 严重缺失。

**三大域共同优点：**
- 架构分层清晰（api/core/svc/sdk）
- 版权和文档注释规范（每个 lib.rs 都有详细头部注释）
- 错误类型统一（thiserror）
- 序列化统一（serde）
- DIP 依赖反转原则应用到位（如 catalog-svc 仅依赖 trait）

**三大域共同短板：**
- 测试覆盖不均衡，核心模块测试充分但边缘模块零测试
- README 文档覆盖率不足（尤其是 data 和 cloud 域）
- API crate 均为纯类型定义，trait 契约较少（更多依赖在 svc 层内部定义）
