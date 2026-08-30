# MOX Platform Domain Crates 代码质量与功能完备度评测报告

> 评测范围：`platform/domains/platform/` 目录下所有 Rust crate
> 评测日期：2026-08-30
> 总 crate 数：**18** 个（api/1 + core/12 + sdk/2 + svc/3）

---

## 一、总体概览

| 分类 | 数量 | 占比 |
|------|------|------|
| 完整实现 | 2 | 11.1% |
| 实质实现 | 9 | 50.0% |
| 骨架/部分实现 | 5 | 27.8% |
| API-only (Trait 契约) | 1 | 5.6% |
| 测试基础设施 | 1 | 5.6% |

**总代码量（src/.rs）**：约 **44,363 行**
**总测试代码量（tests/.rs）**：约 **7,477 行**
**总测试用例数**：约 **431 个**（单元测试 + 集成测试）

---

## 二、详细评测表

### 2.1 api/ 目录

| # | Crate 名称 | 分类 | src文件数 | src行数 | 文档注释 | 核心结构 | tests/目录 | 测试用例数 | README | 备注 |
|---|-----------|------|-----------|---------|----------|----------|-----------|-----------|--------|------|
| 1 | **api** | API-only | 1 | 124 | 有（单行模块级） | IAM/Meta/Datastore/Orchestrator/Enterprise 五大 trait 契约 + 数据结构 + 错误类型 | 无 | 0 | 无 | 纯 trait 定义层，无业务实现。为各 core crate 提供接口契约。 |

**评估说明**：
- `lib.rs` 仅 124 行，定义了 `PlatformApiError`、`IdentityProvider`、`UserManager`、`TenantInfo` 等类型
- 典型的 API 边界 crate，职责单一（接口定义），不需要测试
- 缺少完整的模块级文档注释（仅一行 `//!`）

---

### 2.2 core/ 目录（12 个 crate）

| # | Crate 名称 | 分类 | src文件数 | src行数 | 文档注释 | 核心结构 | tests/目录 | 测试用例数 | README | 备注 |
|---|-----------|------|-----------|---------|----------|----------|-----------|-----------|--------|------|
| 2 | **mox-connector-core** | 骨架/部分实现 | 6 | 589 | 完整（含快速开始示例） | traits / registry / connectors(webhook) / protocol | 无 | 0 | 无 | 连接器框架骨架，仅实现 webhook 一种连接器 |
| 3 | **mox-dsql-core** | 实质实现 | 6 | 1,424 | 有（文件头注释） | cache / engine / error / model / storage + DsqlManager | 有 (1文件/708行) | 28 | 无 | 动态SQL引擎，含完整 CRUD + 缓存 + 迁移 |
| 4 | **mox-enterprise-core** | 实质实现 | 18 | 2,222 | 完整（三大能力说明） | sso(4种) / compliance(3模块) / customization(3模块) / traits | 无 | 9 (单元) | 无 | 政企适配层，SSO/合规/定制三大能力齐全 |
| 5 | **mox-kg-core** | 实质实现 | 6 | 2,120 | 有（文件头注释） | dsl / engine / error / model / storage + KgManager | 无 | 18 (单元) | 无 | 自研知识图谱核心引擎，RocksDB 存储 |
| 6 | **mox-platform-datastore-core** | 实质实现 | 11 | 2,163 | 完整（多行模块级） | dao / field / hash / memory_repos / query / tx + 多后端 | 有 (1文件/217行) | 19 | 无 | 多后端数据存储抽象（SQLite/PG/MySQL） |
| 7 | **mox-platform-iam-core** | 实质实现 | 3 | 1,275 | 无（仅版权头） | ddl.sql / model / repo + IamRepository | 无 | 1 (单元) | 无 | IAM 领域，租户/用户/角色/权限/菜单完整模型 |
| 8 | **mox-platform-integration-core** | 实质实现 | 23 | 3,376 | 完整（含架构图+快速开始） | bootstrap / builtin / config / coordinator / extension / factory / flow / health / protocol | 无 | 6 (单元) | 无 | 统一集成层，L5 架构枢纽，模块最多的 core crate |
| 9 | **mox-platform-meta-core** | 实质实现 | 3 | 1,372 | 无（仅版权头） | ddl.sql / model / repo + MetaRepository | 无 | 2 (单元) | 无 | 元数据领域，实体/字段/视图/工作流完整模型 |
| 10 | **mox-platform-operator-core** | 骨架/迁移中 | 4 | 1,874 | 完整（含迁移状态清单） | kernel / kernel_ext / monad | 无 | 44 (单元) | 无 | 算子核心抽象，kernel 层已完成，高层抽象待迁移 |
| 11 | **mox-platform-orchestrator-core** | 实质实现 | 6 | 2,058 | 完整（多行英文） | event / metrics / module / orchestrator / pipeline | 有 (1文件/242行) | 7 | 无 | DAG 工作流编排引擎，含拓扑调度/资源约束/事件驱动 |
| 12 | **mox-platform-system-core** | **完整实现** | 22 | 5,384 | 完整 | config / crypto / domain_traits / error / event / metrics / model / orchestrator / persistence_provider / ratelimit / rbac / repo(3后端) / server / services / store | 有 (4文件/2379行) | 56 | **有 (62行)** | L7 基础设施层，业务真源，全仓唯一 L7 crate |
| 13 | **mox-plugin-core** | 实质实现 | 11 | 2,194 | 完整（含目录结构+快速开始） | host_api / lifecycle / loader / manifest / market / registry | 无 | 16 (单元) | 无 | WASM 插件框架，含生命周期/加载器/注册表/宿主API |

---

### 2.3 sdk/ 目录（2 个 crate）

| # | Crate 名称 | 分类 | src文件数 | src行数 | 文档注释 | 核心结构 | tests/目录 | 测试用例数 | README | 备注 |
|---|-----------|------|-----------|---------|----------|----------|-----------|-----------|--------|------|
| 14 | **mox-platform-test-harness** | 测试基础设施 | 2 | 181 | 有（说明用途） | rubric + 多 crate 重导出 | 有 (7文件/2207行) | 205 | 无 | T21 E2E 测试框架 + Task12 评分器，200+ 矩阵测试 |
| 15 | **mox-plugin-sdk** | 骨架/轻量实现 | 5 | 514 | 完整（含快速开始+目录结构） | error / host_api / macros / manifest | 无 | 3 (单元) | 无 | 第三方插件开发 SDK，体积符合 SDK 预期 |

---

### 2.4 svc/ 目录（3 个 crate）

| # | Crate 名称 | 分类 | src文件数 | src行数 | 文档注释 | 核心结构 | tests/目录 | 测试用例数 | README | 备注 |
|---|-----------|------|-----------|---------|----------|----------|-----------|-----------|--------|------|
| 16 | **mox-content-publisher** | 骨架/部分实现 | 4 | 849 | 完整（含快速开始示例） | api / model / publisher | 无 | 0 | 无 | 内容多平台发布服务，基于 Connector Framework |
| 17 | **mox-platform-enterprise-svc** | 骨架/部分实现 | 5 | 969 | 有（多行英文） | app_state / auth / routes / main + TenantManager | 有 (1文件/443行) | 8 (单元) | 无 | 企业级服务（多租户/特性开关/配置/健康检查） |
| 18 | **mox-platform-orchestrator-svc** | **完整实现** | 33 | 14,533 | 有 | cordis(6) / handlers(4) / routes(4) / sidecar(2) / ai_router / api_standard / automation / market(4) / openapi / rbac_middleware / subservers | 有 (7文件/1281行) | 45 | **有 (61行)** | L3 编排层网关，全仓最大 crate，16 crate 聚合入口 |

---

## 三、分类深度分析

### 3.1 完整实现（2 个）

#### mox-platform-system-core
- **代码规模**：5,384 行 src + 2,379 行测试 = 7,763 行
- **模块数量**：22 个源文件，覆盖业务全栈
- **测试完备度**：56 个测试用例（30 单元 + 26 集成），测试/代码比约 0.44:1
- **文档**：有 README.md（62 行），含 7 个章节（概述/层级/模块结构/关键Trait/测试指引/二次开发/TDD流程）
- **多后端支持**：SQLite / PostgreSQL / MySQL 三种后端实现 Repository trait
- **业务覆盖**：成员/任务/权限/通信四大核心域 + RBAC + 限流 + 事件编排 + 加密 + 指标

#### mox-platform-orchestrator-svc
- **代码规模**：14,533 行 src + 1,281 行测试 = 15,814 行（全仓最大）
- **模块数量**：33 个源文件
- **测试完备度**：45 个测试用例（29 单元 + 16 集成）
- **文档**：有 README.md（61 行），含 7 个章节
- **核心能力**：Cordis-5 插件化运行时 + 16 crate 聚合网关 + RBAC 中间件 + 算子市场 DSL + 治理台 HITL + Node.js sidecar + OpenAPI spec
- **路由覆盖**：agent / ai_engine / governance / market / voice_proxy

### 3.2 实质实现（9 个）

这 9 个 crate 均有完整的核心实现，但在测试覆盖或周边配套上有提升空间：

| Crate | 亮点 | 短板 |
|-------|------|------|
| mox-dsql-core | 完整动态SQL引擎，缓存+模板+多数据源 | 测试仅 28 个，文档仅文件头注释 |
| mox-enterprise-core | 4 种 SSO + 3 合规模块 + 3 定制模块 | 仅 9 个单元测试，无集成测试 |
| mox-kg-core | 自研 KG 引擎，DSL + 引擎 + 存储 | 仅 18 个单元测试，无集成测试 |
| mox-platform-datastore-core | 3 后端抽象 + DAO + 字段槽分配 | 测试 19 个，偏少 |
| mox-platform-iam-core | 完整 IAM 模型（10+ 实体） | 仅 1 个测试用例 |
| mox-platform-integration-core | 23 个模块，L5 集成枢纽 | 仅 6 个单元测试 |
| mox-platform-meta-core | 完整元数据模型（实体/字段/工作流） | 仅 2 个测试用例 |
| mox-platform-orchestrator-core | DAG 编排 + 资源约束 + 检查点 | 测试 7 个，偏少 |
| mox-plugin-core | WASM 插件全生命周期管理 | 仅 16 个单元测试，无集成测试 |

### 3.3 骨架/部分实现（5 个）

| Crate | 当前状态 | 缺失项 |
|-------|---------|--------|
| mox-connector-core | 框架完整，仅 webhook 一个实现 | 更多连接器实现（REST/gRPC/WS/SOAP/文件）、0 测试 |
| mox-platform-operator-core | kernel/kernel_ext/monad 完成 | StateVector/Operator trait/ExecutionContext 等待迁移 |
| mox-plugin-sdk | 基础结构齐全 | 宏功能完善、更多宿主API绑定、测试极少（仅 3 个） |
| mox-content-publisher | 发布器结构完整 | 0 测试、平台实现依赖 Connector Framework |
| mox-platform-enterprise-svc | HTTP 服务骨架 + 租户管理 | 特性开关/配置管理/健康检查等功能深度不足 |

### 3.4 API-only（1 个）

**api** crate 是纯接口定义层，符合 DIP（依赖倒置原则）架构模式：
- 仅定义 trait 和数据结构，无业务逻辑
- 作为各 core crate 之间的契约边界
- 不需要测试和 README

### 3.5 测试基础设施（1 个）

**mox-platform-test-harness** 是特殊的 SDK 类 crate：
- src 仅 181 行（Rubric 评分器 + 重导出）
- tests/ 目录有 7 个文件、2,207 行、202 个集成测试
- 承担 T21 全链路 E2E 测试矩阵职责
- 测试用例覆盖：EC 矩阵、秘籍矩阵、融合矩阵、可观测性矩阵、CRC64 往返、ETL 矩阵、评分计算

---

## 四、代码质量指标汇总

### 4.1 文档质量

| 指标 | 数值 |
|------|------|
| 有完整模块级文档（//!）的 crate | 13/18 (72.2%) |
| 有 README.md 的 crate | 2/18 (11.1%) |
| 含代码示例的文档 | 8/18 (44.4%) |
| 仅版权头无文档 | 2/18 (11.1%) |

### 4.2 测试覆盖

| 指标 | 数值 |
|------|------|
| 有 tests/ 目录的 crate | 7/18 (38.9%) |
| 有单元测试（src 内）的 crate | 14/18 (77.8%) |
| 无任何测试的 crate | 3/18 (16.7%) |
| 测试用例 > 20 的 crate | 6/18 (33.3%) |

**零测试 crate**：
1. api（合理，纯 trait 定义）
2. mox-connector-core
3. mox-content-publisher

### 4.3 代码量分布

| 层级 | Crate 数 | 平均 src 行数 | 总 src 行数 |
|------|---------|-------------|-----------|
| api | 1 | 124 | 124 |
| core | 12 | 2,117 | 25,407 |
| sdk | 2 | 348 | 695 |
| svc | 3 | 5,450 | 16,351 |
| **总计** | **18** | **2,465** | **42,577** |

---

## 五、改进建议（按优先级）

### 高优先级

1. **为零测试的核心 crate 补充基础测试**
   - `mox-connector-core`：补充 Connector trait 的单元测试和 webhook 连接器集成测试
   - `mox-content-publisher`：补充发布流程的单元测试

2. **提升 IAM 和 Meta 核心域的测试覆盖**
   - `mox-platform-iam-core`：仅 1 个测试，远低于同规模 crate 平均水平
   - `mox-platform-meta-core`：仅 2 个测试，实体/字段/工作流均需覆盖

3. **完成 mox-platform-operator-core 的迁移**
   - lib.rs 文档明确列出了 5 项待办（StateVector、Operator trait、ExecutionContext 等）
   - 作为跨域共享的核心抽象，延迟迁移会阻塞下游 crate 升级

### 中优先级

4. **为更多 crate 添加 README.md**
   - 目前仅 2/18 crate 有 README
   - 建议核心 crate（dsql-core、kg-core、plugin-core、integration-core）优先补充

5. **为 enterprise-core 和 integration-core 补充集成测试**
   - 这两个 crate 代码量较大（2,222 / 3,376 行）但测试极少（9 / 6 个）
   - SSO 多 provider 集成、扩展点注册流程等关键路径需测试保障

6. **丰富 connector-core 的连接器实现**
   - 目前仅 webhook 一种连接器
   - 协议层（protocol/mod.rs 125 行）已有框架，可加速 REST/gRPC 等连接器实现

### 低优先级

7. **统一文档风格**
   - 部分 crate 使用中文文档，部分使用英文，建议统一或双语
   - 部分 crate 仅有文件头注释（如 dsql-core、kg-core），建议补充模块级 `//!` 文档

8. **plugin-sdk 功能完善**
   - 宏（macros.rs 仅 62 行）和宿主 API 绑定可进一步丰富
   - 作为第三方开发生态入口，SDK 完备度直接影响生态体验

---

## 六、总结

`platform/domains/platform/` 目录下的 18 个 Rust crate 整体呈现出**"核心扎实、外围拓展中"**的态势：

- **两个旗舰 crate**（system-core、orchestrator-svc）实现完备、测试充分、文档齐全，达到生产级质量
- **九个核心 crate** 功能完整但测试覆盖不均，属于"功能可用、质量待加强"阶段
- **五个骨架级 crate** 结构清晰但实现深度不足，主要集中在连接器、算子、SDK 和新服务领域
- **架构设计** 体现了清晰的分层（API → Core → SDK → Svc）和 DIP 原则，模块边界合理

整体代码质量处于 **中等偏上** 水平，核心基础设施扎实，外围能力正在逐步填充中。
