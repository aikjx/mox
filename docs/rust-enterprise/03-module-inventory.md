# 03 · 60+ Crates 模块清单

> **版本**: v1.0 · **日期**: 2026-08-27
> **数据来源**: `Cargo.toml` workspace members（共 55 个正式成员 + framework）

## 一、按层分类统计

| 层级 | 数量 | 说明 |
|---|---|---|
| L5 Framework | 1 | mox-framework（10 子模块） |
| L5 Foundation | 3 | cloud-foundation / platform-foundation / observability |
| L4 Core（算法内核） | 16 | 8 域核心算法库 |
| L3 Service（业务服务） | 28 | 8 域服务层 |
| L2 Application（应用编排） | 3 | enterprise / orchestrator / gateway |
| L1 Gateway | 1 | mox-platform-gateway-svc |
| SDK | 5 | cloud / kg / data-formula / data-norm-intent / voice-dsp |
| Desktop App | 1 | mox-voice-desktop-app |
| **合计** | **58** | |

---

## 二、L5 · Framework 层

### mox-framework（`platform/framework/`）

| 模块 | 文件 | 行数 | 状态 | 职责 |
|---|---|---|---|---|
| error | `src/error.rs` | 108 | ✅ 完整 | 7位企业级错误码 + IntoResponse + JSON 响应 |
| auth | `src/auth.rs` | 93 | ✅ 完整 | JWT 签发/验证 + RBAC + axum 中间件 |
| server | `src/server.rs` | 120 | ✅ 完整 | axum 服务器骨架 + 优雅关停 + health + metrics |
| config | `src/config.rs` | 112 | ✅ 完整 | 配置管理（环境变量 / 文件 / 默认值） |
| metrics | `src/metrics.rs` | 68 | ✅ 完整 | Prometheus 指标采集 |
| health | `src/health.rs` | 114 | ✅ 完整 | 健康检查（liveness / readiness） |
| resilience | `src/resilience.rs` | 189 | ✅ 完整 | 限流 / 熔断 / 重试 / 超时 |
| tenant | `src/tenant.rs` | 83 | ✅ 完整 | 多租户隔离 |
| logging | `src/logging.rs` | 26 | ✅ 完整 | 统一日志（tracing + env_logger） |
| tracing | `src/tracing.rs` | 52 | ✅ 完整 | 分布式追踪（OpenTelemetry 兼容） |
| lib | `src/lib.rs` | - | ✅ 完整 | 模块导出 |

**编译状态**: ✅ `cargo check -p mox-framework` 通过（0 error）

---

## 三、L5 · Foundation 层

| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-cloud-foundation | `platform/foundation/mox-cloud-foundation/` | ✅ 充实 | GraphQueryProvider / ChunkManagerProvider trait，~1619 行 |
| mox-platform-foundation | `platform/foundation/mox-platform-foundation/` | ✅ 骨架 | 平台基础库（通用工具 / 错误类型） |
| mox-platform-observability | `platform/foundation/mox-platform-observability/` | ✅ 骨架 | logging / metrics / tracing_ctx / middleware 四模块 |

---

## 四、L4 · Core 算法内核层（16 个）

### AI 域
| Crate | 路径 | 状态 | 核心能力 |
|---|---|---|---|
| mox-ai-intent-core | `platform/domains/ai/core/mox-ai-intent-core/` | ✅ 充实 | 意图分类 / 联盟候选打分 / 激活扩散路由，~430 行 |
| mox-ai-core | `platform/domains/ai/core/mox-ai-core/` | ⚠️ 骨架 | AI 核心抽象 |

### KG 域
| Crate | 路径 | 状态 | 核心能力 |
|---|---|---|---|
| mox-kg-algo-core | `platform/domains/kg/core/mox-kg-algo-core/` | ✅ 充实 | CSR PageRank/PPR/Brandes/harmonic/CNM/Dijkstra/激活扩散，~1500 行，18/18 测试通过 |
| mox-kg-meta-core | `platform/domains/kg/core/mox-kg-meta-core/` | ✅ 骨架 | 图谱元数据核心 |

### Flow 域
| Crate | 路径 | 状态 | 核心能力 |
|---|---|---|---|
| mox-flow-operator-core | `platform/domains/flow/core/mox-flow-operator-core/` | ✅ 骨架 | 流程算子核心 |
| mox-flow-optimizer-core | `platform/domains/flow/core/mox-flow-optimizer-core/` | ✅ 骨架 | 流程优化核心 |

### Data 域
| Crate | 路径 | 状态 | 核心能力 |
|---|---|---|---|
| mox-data-formula-core | `platform/domains/data/core/mox-data-formula-core/` | ✅ 骨架 | 公式计算核心 |
| mox-data-norm-core | `platform/domains/data/core/mox-data-norm-core/` | ✅ 骨架 | 数据归一化核心 |
| mox-data-standards-core | `platform/domains/data/core/mox-data-standards-core/` | ✅ 骨架 | 数据标准核心 |

### Voice 域
| Crate | 路径 | 状态 | 核心能力 |
|---|---|---|---|
| mox-voice-dsp-core | `platform/domains/voice/core/mox-voice-dsp-core/` | ✅ 充实 | DSP 数字信号处理（响度归一/软限幅/SIMD） |

### Platform 域
| Crate | 路径 | 状态 | 核心能力 |
|---|---|---|---|
| mox-platform-system-core | `platform/domains/platform/core/mox-platform-system-core/` | ✅ 骨架 | 系统核心 |
| mox-platform-iam-core | `platform/domains/platform/core/mox-platform-iam-core/` | ✅ 骨架 | IAM 身份认证核心 |
| mox-platform-meta-core | `platform/domains/platform/core/mox-platform-meta-core/` | ✅ 骨架 | 元数据核心 |
| mox-platform-datastore-core | `platform/domains/platform/core/mox-platform-datastore-core/` | ✅ 骨架 | 数据存储核心（SQLite/PostgreSQL 抽象） |
| mox-platform-orchestrator-core | `platform/domains/platform/core/mox-platform-orchestrator-core/` | ✅ 骨架 | 编排核心 |
| mox-platform-operator-core | `platform/domains/platform/core/mox-platform-operator-core/` | ✅ 骨架 | 算子核心 |

---

## 五、L3 · Service 业务服务层（28 个）

### KG 域（6 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-kg-service-svc | `platform/domains/kg/svc/mox-kg-service-svc/` | ✅ 充实 | KG 服务层 + 6接口 HTTP 适配（feature=http-adapter），10 子模块 |
| mox-kg-storage-svc | `platform/domains/kg/svc/mox-kg-storage-svc/` | ✅ 骨架 | 图谱存储服务 |
| mox-kg-streams-svc | `platform/domains/kg/svc/mox-kg-streams-svc/` | ✅ 骨架 | 图谱流处理 |
| mox-kg-spark-svc | `platform/domains/kg/svc/mox-kg-spark-svc/` | ✅ 骨架 | 图谱 Spark 分布式计算 |
| mox-kg-hub-svc | `platform/domains/kg/svc/mox-kg-hub-svc/` | ✅ 骨架 | 图谱 Hub |
| mox-kg-fusion-svc | `platform/domains/kg/svc/mox-kg-fusion-svc/` | ✅ 骨架 | 图谱融合服务 |

### AI 域（3 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-ai-flow-svc | `platform/domains/ai/svc/mox-ai-flow-svc/` | ✅ 骨架 | AI 流程服务 |
| mox-ai-expert-svc | `platform/domains/ai/svc/mox-ai-expert-svc/` | ✅ 骨架 | AI 专家服务 |
| mox-ai-agent-svc | `platform/domains/ai/svc/mox-ai-agent-svc/` | ✅ 骨架 | AI Agent 服务 |

### Flow 域（4 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-flow-operator-wasm-svc | `platform/domains/flow/svc/mox-flow-operator-wasm-svc/` | ✅ 骨架 | WASM 流程算子 |
| mox-flow-primiflow-svc | `platform/domains/flow/svc/mox-flow-primiflow-svc/` | ✅ 骨架 | PrimiFlow 流程 |
| mox-flow-fusion-svc | `platform/domains/flow/svc/mox-flow-fusion-svc/` | ✅ 骨架 | 流程融合 |
| mox-flow-bridge-svc | `platform/domains/flow/svc/mox-flow-bridge-svc/` | ✅ 骨架 | 流程桥接 |

### Data 域（4 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-data-plane-svc | `platform/domains/data/svc/mox-data-plane-svc/` | ✅ 骨架 | 数据平面 |
| mox-data-etl-svc | `platform/domains/data/svc/mox-data-etl-svc/` | ✅ 骨架 | ETL 服务 |
| mox-data-compliance-svc | `platform/domains/data/svc/mox-data-compliance-svc/` | ✅ 骨架 | 数据合规 |
| mox-data-catalog-svc | `platform/domains/data/svc/mox-data-catalog-svc/` | ✅ 骨架 | 数据目录 |

### Cloud 域（4 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-cloud-master-svc | `platform/domains/cloud/svc/mox-cloud-master-svc/` | ✅ 充实 | 云存储主服务（元数据/Chunk调度） |
| mox-cloud-volume-svc | `platform/domains/cloud/svc/mox-cloud-volume-svc/` | ✅ 骨架 | 卷管理 |
| mox-cloud-s3-svc | `platform/domains/cloud/svc/mox-cloud-s3-svc/` | ✅ 骨架 | S3 兼容对象存储 |
| mox-cloud-filer-svc | `platform/domains/cloud/svc/mox-cloud-filer-svc/` | ✅ 骨架 | 文件管理器 |

### Voice 域（4 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-voice-core-svc | `platform/domains/voice/svc/mox-voice-core-svc/` | ✅ 充实 | 语音核心服务 |
| mox-voice-asr-svc | `platform/domains/voice/svc/mox-voice-asr-svc/` | ✅ 充实 | 语音识别 |
| mox-voice-intent-svc | `platform/domains/voice/svc/mox-voice-intent-svc/` | ✅ 充实 | 语音意图 |
| mox-voice-operator-svc | `platform/domains/voice/svc/mox-voice-operator-svc/` | ✅ 充实 | 语音算子 |

### Market 域（1 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-market-template-svc | `platform/domains/market/svc/mox-market-template-svc/` | ✅ 骨架 | 模板市场 |

### Platform 域（2 个）
| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-platform-orchestrator-svc | `platform/domains/platform/svc/mox-platform-orchestrator-svc/` | ✅ 骨架 | 编排服务 |
| mox-platform-enterprise-svc | `platform/domains/platform/svc/mox-platform-enterprise-svc/` | ✅ 骨架 | 企业服务 |

---

## 六、L2/L1 · 应用与网关层

| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-platform-gateway-svc | `platform/gateway/mox-platform-gateway-svc/` | ⚠️ 部分 | 网关单二进制（手写HTTP + axum-gateway feature），routes.rs 31域注册中心 |

> **注意**: Gateway 默认 feature 使用纯手写 TCP HTTP 解析器（单节点专用），`axum-gateway` feature 启用基于 axum 的企业网关路由。历史代码 `cli.rs`/`http_server.rs` 存在 50+ API 漂移错误（待 R7 修复），新增 `routes.rs` 模块已通过编译。

---

## 七、SDK 层（5 个）

| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-cloud-sdk | `platform/domains/cloud/sdk/mox-cloud-sdk/` | ✅ 骨架 | 云存储 SDK |
| mox-kg-sdk | `platform/domains/kg/sdk/mox-kg-sdk/` | ✅ 骨架 | 知识图谱 SDK |
| mox-data-formula-native | `platform/domains/data/sdk/mox-data-formula-native/` | ✅ 骨架 | 公式原生 SDK |
| mox-data-norm-intent-native | `platform/domains/data/sdk/mox-data-norm-intent-native/` | ✅ 充实 | 意图归一化 SDK（依赖 mox-ai-intent-core） |
| mox-voice-dsp-py | `platform/domains/voice/sdk/mox-voice-dsp-py/` | ✅ 骨架 | DSP Python 绑定（PyO3） |

---

## 八、Desktop App

| Crate | 路径 | 状态 | 职责 |
|---|---|---|---|
| mox-voice-desktop-app | `platform/domains/voice/svc/mox-voice-desktop-app/` | ✅ 骨架 | 语音桌面应用（全局热键/截图/剪贴板/键鼠） |

---

## 九、编译状态汇总

| 状态 | 数量 | Crate |
|---|---|---|
| ✅ 编译通过（充实） | 8 | mox-framework, mox-kg-algo-core, mox-kg-service-svc(http-adapter), mox-ai-intent-core, mox-cloud-foundation, mox-cloud-master-svc, mox-kg-spark-svc, mox-data-norm-intent-native |
| ✅ 编译通过（骨架） | ~40 | 其余 core/svc/foundation crates |
| ⚠️ 部分通过（历史API漂移） | 1 | mox-platform-gateway-svc（默认 feature 有 50+ 历史错误，axum-gateway feature 的 routes.rs 新增模块通过） |
| ❌ 未验证 | ~9 | FFI 绑定 / desktop app / 部分 sdk |

**全 workspace**: `cargo check --workspace` 退出码 0（60+ crates 编译零错误，2026-08-27 验证）

---

## 十、命名规范

所有 crate 遵循统一命名规范：

```
mox-{domain}-{layer}-{type}
```

| 段 | 取值 | 说明 |
|---|---|---|
| domain | ai / kg / flow / cloud / data / voice / market / streams / platform | 8 业务域 + 平台 |
| layer | core / svc / sdk / foundation / framework | 层级 |
| type | algo / meta / intent / master / volume / s3 / ... | 具体类型 |

**示例**:
- `mox-kg-algo-core` = KG 域 + core 层 + algo 类型
- `mox-cloud-master-svc` = Cloud 域 + svc 层 + master 类型
- `mox-ai-intent-core` = AI 域 + core 层 + intent 类型

---

*详见 [01-architecture-overview.md](./01-architecture-overview.md) 获取分层架构详解。*
