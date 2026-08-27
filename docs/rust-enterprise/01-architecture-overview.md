# 01 · 6层企业级模块化架构总览

> **版本**: v1.0 · **日期**: 2026-08-27

## 一、架构设计原则

1. **分层明确**：L0-L5 六层，依赖方向严格自上而下，禁止反向依赖
2. **域隔离**：8 个业务域（AI/KG/Flow/Cloud/Data/Voice/Market/Streams）独立 crate，域间通过 trait 接口通信
3. **模块化单体**：单二进制部署，可按域拆分为微服务（ADR-16）
4. **企业级红线**：
   - 社区检测仅允许 CNM（模块度贪心凝聚），禁用 LPA
   - 图算法必须 CSR 优化，Pearson 相关系数 ≥ 0.9999
   - 所有中心性指标必须附带人读公式
   - 密度指标必须附带解读文案（高/中/稀疏）

---

## 二、六层架构详解

### L0 · 接入层（Access Layer）

**职责**：多端接入、协议转换、SDK 封装

| 组件 | 技术 | 说明 |
|---|---|---|
| Web UI | frontend-ui/ | React/Vue 前端 |
| MCP Client | MCP 协议 | Model Context Protocol 客户端 |
| CLI | clap | 命令行工具 |
| SDK | Rust/TypeScript | 多语言 SDK |
| OpenAPI | OpenAPI 3.0 | API 文档自动生成 |

**依赖方向**：仅依赖 L1 网关层，不直接访问 L2+

---

### L1 · 网关层（Gateway Layer）

**核心 crate**: `mox-platform-gateway-svc`

**职责**：
- HTTP 路由分发（31 域模块化注册中心）
- 认证鉴权（JWT 验证 + RBAC）
- 限流熔断（resilience 模块）
- 可观测性（metrics + tracing + logging）
- 健康检查（/health, /ready, /domains）

**技术栈**:
- axum 0.7（HTTP 框架）
- tokio（异步运行时）
- tower-http（CORS / Trace / Compression 中间件）
- mox-framework（统一错误 / 认证 / 配置）

**路由注册机制**:
```rust
// routes.rs 核心设计
const DOMAINS: &[Domain] = &[
    Domain { prefix: "/kg/v1",     name: "kg/知识图谱",   status: "ready", owner: "L2" },
    Domain { prefix: "/ai/engine", name: "ai/AI引擎",     status: "ready", owner: "L3" },
    Domain { prefix: "/cloud/v1",  name: "cloud/云存储",  status: "stub",  owner: "L4" },
    // ... 共 31 个域声明
];

pub fn build_gateway_router() -> Router {
    // 自动 merge ready 域 + stub 域注入统一 404 响应
}
```

**Feature 开关**:
- `default`: 纯手写 TCP HTTP 解析器（单节点专用，零依赖）
- `axum-gateway`: 基于 axum 的企业网关路由（31域模块化）

---

### L2 · 应用编排层（Orchestration Layer）

| crate | 职责 |
|---|---|
| `mox-enterprise-svc` | 企业级业务编排、P0-P12 流程引擎 |
| `mox-orchestrator-svc` | 通用编排器、任务调度、工作流 |
| `mox-platform-test-harness` | 测试编排、冒烟测试、集成测试 |

**核心能力**:
- P0-P12 十三阶段业务流程引擎（详见 [02-business-flow.md](./02-business-flow.md)）
- 持久化工作流（ADR-14）
- 跨域事务协调（Saga 模式）

---

### L3 · 业务服务层（Service Layer · 8域）

#### 域 1: AI（人工智能）
| crate | 职责 |
|---|---|
| `mox-ai-orchestrator-svc` | AI 编排、意图路由、能力调度 |
| `mox-ai-intent-core` | 意图识别核心（classify_intent / score_alliance_candidates） |
| `mox-data-norm-intent-native` | 原生意图归一化 SDK |

#### 域 2: KG（知识图谱）
| crate | 职责 |
|---|---|
| `mox-kg-service-svc` | KG 服务层、6接口 HTTP 适配、Cypher/nGQL 解析 |
| `mox-kg-algo-core` | 图算法核心（PageRank/PPR/Brandes/harmonic/CNM/Dijkstra） |
| `mox-kg-meta-core` | 图谱元数据核心 |
| `mox-kg-fusion-svc` | 图谱融合服务 |
| `mox-kg-spark-svc` | 图谱 Spark 分布式计算 |

#### 域 3: Flow（流程引擎）
| crate | 职责 |
|---|---|
| `mox-flow-engine-svc` | 流程引擎服务 |
| `mox-flow-op-core` | 流程优化核心 |

#### 域 4: Cloud（云存储）
| crate | 职责 |
|---|---|
| `mox-cloud-master-svc` | 云存储主服务（元数据管理、Chunk 调度） |
| `mox-cloud-s3-svc` | S3 兼容对象存储 |
| `mox-cloud-foundation` | 云存储基础库（GraphQueryProvider / ChunkManagerProvider） |
| `mox-cloud-volume-svc` | 卷管理服务 |

#### 域 5: Data（数据治理）
| crate | 职责 |
|---|---|
| `mox-data-norm-*` | 数据归一化系列（intent / entity / relation） |
| `mox-platform-datastore-core` | 数据存储核心（SQLite/PostgreSQL 抽象） |
| `mox-platform-foundation` | 平台基础库 |

#### 域 6: Voice（语音）
| crate | 职责 |
|---|---|
| `mox-voice-*` | 语音识别 / 合成 / 声纹（ADR-15 域独立化） |

#### 域 7: Market（插件市场）
| crate | 职责 |
|---|---|
| `mox-market-*` | 插件注册 / 分发 / 计费 |

#### 域 8: Streams（数据流）
| crate | 职责 |
|---|---|
| `mox-streams-*` | 实时数据流 / 事件总线 / CDC |

---

### L4 · 算法内核层（Kernel Layer）

| crate | 核心算法 | 代码量 |
|---|---|---|
| `mox-kg-algo-core` | CSR PageRank / PPR / Brandes 介数 / harmonic 紧密 / CNM 社区 / Dijkstra / 激活扩散 / 余弦相似度 | ~1500 行 |
| `mox-ai-intent-core` | 意图分类 / 联盟候选打分 / 激活扩散路由 | ~430 行 |
| `mox-flow-op-core` | 流程优化算法 | - |
| `mox-data-norm-core` | 数据归一化算法 | - |
| `mox-dsp-core` | 数字信号处理 | - |

**算法红线合规**:
- ✅ CSR vs Dense Pearson ≥ 0.9999（PageRank + PPR 双验证）
- ✅ 规范化拉普拉斯 CSR 与 Dense 等价
- ✅ CNM 社区发现（非 LPA）
- ✅ Brandes 介数中心性
- ✅ harmonic 紧密中心性
- ✅ 所有中心性指标附带人读公式
- ✅ 密度指标附带三档解读文案

---

### L5 · 基础层（Foundation Layer）

#### Framework 子层（`mox-framework`）
| 模块 | 行数 | 职责 |
|---|---|---|
| `error.rs` | 108 | 7位企业级错误码 + IntoResponse + JSON 错误响应 |
| `auth.rs` | 93 | JWT 签发/验证 + RBAC + axum 中间件 |
| `server.rs` | 120 | axum 服务器骨架 + 优雅关停 + health + metrics |
| `config.rs` | 112 | 配置管理（环境变量 / 文件 / 默认值） |
| `metrics.rs` | 68 | Prometheus 指标采集 |
| `health.rs` | 114 | 健康检查（liveness / readiness） |
| `resilience.rs` | 189 | 限流 / 熔断 / 重试 / 超时 |
| `tenant.rs` | 83 | 多租户隔离 |
| `logging.rs` | 26 | 统一日志（tracing + env_logger） |
| `tracing.rs` | 52 | 分布式追踪（OpenTelemetry 兼容） |

#### Foundation 子层
| crate | 职责 |
|---|---|
| `mox-cloud-foundation` | 云存储基础 trait（GraphQueryProvider / ChunkManagerProvider） |
| `mox-platform-foundation` | 平台基础库（通用工具 / 错误类型） |
| `mox-platform-observability` | 可观测性（logging / metrics / tracing_ctx / middleware） |

---

## 三、依赖方向图

```
L0 接入层
    ↓ (HTTP / gRPC / SDK)
L1 网关层 ──── mox-framework (L5)
    ↓ (trait 接口)
L2 应用编排层 ──── mox-framework (L5)
    ↓ (域间 trait)
L3 业务服务层 (8域独立 crate)
    ↓ (算法 trait)
L4 算法内核层
    ↓ (基础库)
L5 基础层 (Framework + Foundation)
```

**严格规则**:
- 上层可依赖下层，下层不可依赖上层
- 同层域间通过 trait 接口通信，禁止直接 impl 依赖
- L4 算法内核仅依赖 L5 基础层，不依赖任何业务服务

---

## 四、Workspace 结构

```
infotopograph/
├── Cargo.toml              # workspace 根（60+ members）
├── platform/
│   ├── framework/          # L5: mox-framework
│   ├── foundation/         # L5: cloud/platform/observability
│   ├── gateway/            # L1: mox-platform-gateway-svc
│   ├── orchestrator/       # L2: enterprise/orchestrator/test-harness
│   └── domains/
│       ├── ai/             # L3+L4: AI 域
│       ├── kg/             # L3+L4: KG 域
│       ├── flow/           # L3+L4: Flow 域
│       ├── cloud/          # L3+L4: Cloud 域
│       ├── data/           # L3+L4: Data 域
│       ├── voice/          # L3: Voice 域
│       ├── market/         # L3: Market 域
│       └── streams/        # L3: Streams 域
└── docs/rust-enterprise/   # 本文档集
```

---

*详见 [03-module-inventory.md](./03-module-inventory.md) 获取完整 60+ crates 清单。*
