# 纯 Rust 企业级模块化架构 · 文档总览

> **版本**: v1.0 · **日期**: 2026-08-27 · **状态**: 已交付（R1-R6 全链路通过）

## 一、项目定位

本项目将原 Node.js 后端全面迁移至 **纯 Rust 企业级模块化架构**，采用 **6 层归一化 + 8 业务域** 设计，覆盖 60+ Rust crates，实现从接入层到算法内核的全栈 Rust 化。

**核心目标**：
- 零 Node.js 依赖，单二进制部署
- 模块化单体 → 可演进微服务
- 全维度业务流程贯通（P0-P12）
- 企业级可观测性、安全性、合规性

---

## 二、文档索引

| 编号 | 文档 | 内容摘要 |
|---|---|---|
| 01 | [6层架构总览](./01-architecture-overview.md) | L0-L5 分层设计、8域划分、依赖方向、技术栈 |
| 02 | [P0-P12 业务流程图](./02-business-flow.md) | 13阶段端到端业务流程、关键产物、API入口 |
| 03 | [60+ Crates 模块清单](./03-module-inventory.md) | 按层/域分类的完整 crate 清单、状态、代码量 |
| 04 | [31域路由 API 规范](./04-api-gateway-routes.md) | Gateway 模块化路由注册中心、各域端点定义 |
| 05 | [KG 算法核心接口](./05-kg-algorithm-core.md) | 6大算法、CSR优化、公式文档、密度解读、路径查找 |
| 06 | [AI 引擎接口规范](./06-ai-engine-api.md) | 意图识别、能力路由、能力矩阵、CEM指标 |
| 07 | [编译与测试指南](./07-build-and-test.md) | cargo check/test 命令、feature 开关、冒烟验证 |
| 08 | [业务功能关联关系图](./08-business-function-relation.md) | 8域关联总图、ER实体图、跨域调用链路、数据流向、依赖矩阵 |

---

## 三、架构速览

```
┌─────────────────────────────────────────────────────────────┐
│  L0 接入层    Web UI / MCP Client / CLI / SDK / OpenAPI     │
├─────────────────────────────────────────────────────────────┤
│  L1 网关层    mox-platform-gateway-svc (axum 单二进制)       │
│              routes.rs: 31域注册中心                           │
├─────────────────────────────────────────────────────────────┤
│  L2 应用编排  Orchestrator / Enterprise Scheduler / Test      │
├─────────────────────────────────────────────────────────────┤
│  L3 业务服务  8域 Service: AI · KG · Flow · Cloud · Data     │
│              · Voice · Market · Streams                       │
├─────────────────────────────────────────────────────────────┤
│  L4 算法内核  kg-algo-core · ai-intent-core · flow-op-core   │
│              · data-norm-core · dsp-core                      │
├─────────────────────────────────────────────────────────────┤
│  L5 基础层    Framework (auth/error/server/metrics/...)       │
│              Foundation (cloud/platform/observability)         │
└─────────────────────────────────────────────────────────────┘
```

---

## 四、验收里程碑

| 阶段 | 任务 | 状态 | 验证命令 |
|---|---|---|---|
| R1 | 盘点 60+ crates 现状 | ✅ | `cargo check --workspace` |
| R2 | 设计 6层架构 + P0-P12 流程图 | ✅ | 本文档集 |
| R3 | 补齐 Framework 层（error→HTTP） | ✅ | `cargo check -p mox-framework` |
| R4 | Gateway 31域模块化路由桩 | ✅ | `cargo check -p mox-platform-gateway-svc --features axum-gateway` |
| R5 | KG/AI 核心算法 + HTTP 适配层 | ✅ | `cargo test -p mox-kg-algo-core` (18/18) |
| R6 | 冒烟验证 | ✅ | 见 [07-编译与测试指南](./07-build-and-test.md) |

---

## 五、关键技术决策

| 决策项 | 选择 | 理由 |
|---|---|---|
| Web 框架 | axum 0.7 + tokio | 生态成熟、Tower 中间件兼容、异步高性能 |
| 架构模式 | 模块化单体（Modular Monolith） | 单二进制部署 + 可按域拆分为微服务 |
| 图算法存储 | CSR（Compressed Sparse Row） | 内存高效、PageRank/PPR Pearson ≥ 0.9999 |
| 社区检测 | CNM 模块度贪心凝聚 | 项目红线：禁用 LPA，CNM 为唯一合规算法 |
| 错误体系 | 7位企业级错误码 + IntoResponse | 4xxx客户端 / 5xxx服务端 / 6xxx网关 / 7xxx存储 |
| 认证授权 | JWT + RBAC + axum 中间件 | 无状态、可横向扩展、细粒度权限 |

---

## 六、下一步路线

- [ ] **R7**: 修复 Gateway 历史代码 API 漂移（cli.rs / http_server.rs 50+ error）
- [ ] **R8**: 补齐 Cloud / Data / Voice / Market / Streams 5域 HTTP 适配层
- [ ] **R9**: 删除 `backend-node/` 目录，Rust Gateway 端口 8080 全面接管
- [ ] **R10**: 集成 OpenTelemetry 全链路追踪 + Prometheus 指标导出
- [ ] **R11**: 模块化单体 → 微服务拆分预案落地（见 ADR-16）

---

*本文档集为纯 Rust 企业级架构迁移的权威单源（Single Source of Truth），所有架构决策以本文档为准。*
