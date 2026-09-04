# 璇玑 RelGraph · 算子统一系统（OUS）企业级架构文档

> **文档版本**：v1.0.0
> **创建日期**：2026-09-04
> **文档状态**：正式发布
> **架构师**：璇玑架构团队
> **适用范围**：全栈三端（docs / frontend-ui / platform）

---

## 文档变更记录

| 版本 | 日期 | 变更内容 | 作者 |
|------|------|----------|------|
| v1.0.0 | 2026-09-04 | 初始正式版本，确立三端融合企业级架构 | 璇玑架构团队 |

---

## 目录

1. [项目概述](#1-项目概述)
2. [架构设计原则](#2-架构设计原则)
3. [系统总体架构](#3-系统总体架构)
4. [后端平台架构（platform）](#4-后端平台架构platform)
5. [前端架构（frontend-ui）](#5-前端架构frontend-ui)
6. [文档体系架构（docs）](#6-文档体系架构docs)
7. [核心模块详细设计](#7-核心模块详细设计)
8. [数据模型设计](#8-数据模型设计)
9. [API 设计规范](#9-api-设计规范)
10. [安全架构](#10-安全架构)
11. [可观测性架构](#11-可观测性架构)
12. [部署架构](#12-部署架构)
13. [开发规范](#13-开发规范)
14. [测试策略](#14-测试策略)
15. [性能与扩展性](#15-性能与扩展性)
16. [开发路线图](#16-开发路线图)

---

## 1. 项目概述

### 1.1 项目定位

**璇玑 RelGraph · 算子统一系统（OUS）** 是一个企业级多专家智能编排平台，通过"开发专家联盟"模式，将多个领域专家（安全、权限、算法、架构、性能、测试等）组织为协作联盟，对用户需求进行mox 模块化系统架构维度分析、辩论、合成与质量门禁，最终输出企业级可交付成果。

### 1.2 核心价值

- **mox 模块化系统架构分析**：14维归一化评估，覆盖安全、权限、算法、架构、性能、可维护性等
- **多专家协作**：6阶段管线（意图→组队→辩论→合成→门禁→学习），模拟专家团队协作
- **质量门禁**：ABCD四级质量分级，D级自动阻断，确保交付质量
- **企业级弹性**：熔断、限流、舱壁隔离、指数退避重试，保障高可用
- **全链路可追溯**：trace_id 全链路透传，7审计事件完整记录

### 1.3 三端协同

| 端 | 技术栈 | 职责 |
|----|--------|------|
| **docs** | Markdown + HTML | 文档驱动开发、架构决策记录、需求-架构映射、验证报告归档 |
| **frontend-ui** | Vue 3 + Vite + Element Plus + Pinia | 用户交互、SSE流式展示、可视化编排、组件库 |
| **platform** | Rust + Cargo Workspace + Tokio + Axum | 核心编排、专家联盟引擎、任务调度、弹性治理、API网关 |

---

## 2. 架构设计原则

### 2.1 核心原则

| 原则 | 说明 | 落地方式 |
|------|------|----------|
| **领域驱动设计（DDD）** | 按业务域划分限界上下文，每个域独立演进 | 11个业务域，每域6层（api/core/proto/sdk/svc/svcapi） |
| **依赖注入** | 依赖通过 trait 抽象注入，不直接依赖具体实现 | ExpertConsultant、ExecutorBridge、ExpertMatcher 等 trait |
| **单一职责** | 每个模块只做一件事，可独立测试和替换 | 25个前端API模块、30+组件、11个后端域 |
| **归一化设计** | 跨端统一错误码、质量分、事件格式、追踪ID | 14维归一化（Rust唯一实现）、错误归一化（前端）、AllianceEvent统一格式 |
| **弹性优先** | 所有跨进程调用必须有熔断、限流、超时、重试 | framework/resilience.rs 统一弹性层 |
| **可观测性** | 全链路 trace、结构化日志、指标采集 | tracing + OpenTelemetry + Prometheus |
| **文档驱动** | 架构决策必须有ADR记录，需求必须有架构映射 | docs/enterprise/ ADR编号文档 |

### 2.2 模块化归一化三要素

每个模块必须明确定义三个契约：

```
┌─────────────────────────────────────────────────┐
│                   Module X                        │
│  ┌─────────────┐  ┌──────────┐  ┌────────────┐ │
│  │ Input Contract│→│  Core Logic│→│Output Contract││
│  │  shape/dtype │  │          │  │  Message   │ │
│  │  range/必填  │  │          │  │  +metadata │ │
│  └─────────────┘  └────┬─────┘  └────────────┘ │
│                         │                          │
│                  ┌──────▼──────┐                  │
│                  │State Contract│                  │
│                  │ version·快照  │                  │
│                  │ 可回滚·种子   │                  │
│                  └─────────────┘                  │
└─────────────────────────────────────────────────┘
```

- **输入契约**：明确输入的类型、范围、必填字段，运行时校验
- **输出契约**：统一 MOXMessage 格式，携带元数据（latency、degraded、stats）
- **状态契约**：状态版本化、可快照、可回滚、可校验

---

## 3. 系统总体架构

### 3.1 分层架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        客户端层（Client）                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  Web 浏览器   │  │  移动端 H5   │  │  第三方 API 客户端    │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
└─────────┼───────────────────┼───────────────────────┼──────────────┘
          │                   │                       │
┌─────────▼───────────────────▼───────────────────────▼──────────────┐
│                      前端层（frontend-ui）                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │ 视图层    │  │ 组件层    │  │ 状态层    │  │ API 层（含SSE）  │  │
│  │ (40+视图) │  │ (30+组件) │  │ (Pinia)  │  │ (25个API模块)    │  │
│  └──────────┘  └──────────┘  └──────────┘  └────────┬─────────┘  │
└──────────────────────────────────────────────────────────┼────────────┘
                                                             │ REST/SSE
┌──────────────────────────────────────────────────────────▼────────────┐
│                      网关层（Gateway）                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │
│  │  认证鉴权     │  │  路由分发     │  │  限流/熔断/日志/追踪          │ │
│  └──────────────┘  └──────────────┘  └──────────────────────────────┘ │
└──────────────────────────────────────────────────────────┬────────────┘
                                                             │
┌──────────────────────────────────────────────────────────▼────────────┐
│                    业务编排层（Orchestration）                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │
│  │ 联盟引擎      │  │ 任务调度器    │  │ 工作流引擎                    │ │
│  │ (6阶段管线)   │  │ (生命周期)    │  │ (DAG执行)                    │ │
│  └──────────────┘  └──────────────┘  └──────────────────────────────┘ │
└──────────────────────────────────────────────────────────┬────────────┘
                                                             │
┌──────────────────────────────────────────────────────────▼────────────┐
│                     领域服务层（Domain Services）                        │
│  ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐  │
│  │AI域│ │联盟│ │KG域│ │云盘│ │数据│ │流程│ │市场│ │项目│ │语音│  │
│  └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘ └────┘  │
└──────────────────────────────────────────────────────────┬────────────┘
                                                             │ gRPC
┌──────────────────────────────────────────────────────────▼────────────┐
│                     智能层（Intelligence - Python）                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────────────┐ │
│  │ LLM 推理服务  │  │ Embedding服务 │  │ 向量库/RAG/训练服务          │ │
│  └──────────────┘  └──────────────┘  └──────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 模块成熟度分级

| 级别 | 域 | 状态 | 策略 |
|------|-----|------|------|
| **P0 生产可用** | ai, alliance, platform, foundation, framework | 核心充实，测试覆盖 | 持续优化，接入LLM，补持久化 |
| **P1 开发中** | kg, cloud, flow, data | 脚手架+部分核心 | 按优先级填充核心逻辑 |
| **P2 规划中** | market, project, voice, base | 脚手架 | 保留接口，文档先行 |

---

## 4. 后端平台架构（platform）

### 4.1 Cargo Workspace 结构

```
platform/
├── Cargo.toml                    # Workspace 根配置
├── foundation/                    # L0 基础库层
│   ├── mox-error/                # 统一错误码体系
│   ├── mox-tracing/              # 结构化日志与追踪
│   ├── mox-config/               # 配置管理
│   ├── mox-metrics/              # 指标采集
│   └── mox-foundation/           # 基础类型与 CrateMeta
├── framework/                     # L1 应用框架层
│   ├── src/
│   │   ├── lib.rs
│   │   ├── resilience.rs         # 熔断/限流/舱壁/重试
│   │   ├── pipeline.rs           # 管线抽象（PhaseId + PhaseHandler）
│   │   └── config.rs
│   └── Cargo.toml
├── domains/                       # L2 业务域层（11个域）
│   ├── ai/                        # AI 域
│   │   ├── api/                   # API 定义
│   │   ├── core/                  # 核心逻辑
│   │   │   ├── mox-ai-alliance-engine/   # 联盟引擎（6阶段管线）
│   │   │   ├── mox-ai-expert-core/       # 专家核心（14维归一化）
│   │   │   └── mox-ai-flow-core/         # 流程核心
│   │   ├── proto/                 # 协议类型
│   │   ├── sdk/                   # 客户端 SDK
│   │   ├── svc/                   # 服务实现
│   │   └── svcapi/                # 服务 API
│   ├── alliance/                  # 联盟域
│   │   ├── core/
│   │   │   ├── mox-alliance-scheduler-core/  # 任务调度器
│   │   │   ├── mox-alliance-executor-core/   # 执行器核心
│   │   │   └── mox-alliance-config-core/     # 配置中心
│   │   └── ...
│   ├── kg/                        # 知识图谱域
│   ├── cloud/                     # 云盘域
│   ├── data/                      # 数据域
│   ├── flow/                      # 流程域
│   ├── market/                    # 市场域
│   ├── platform/                  # 平台域
│   ├── project/                   # 项目域
│   └── voice/                     # 语音域
├── gateway/                       # L3 网关层
│   └── mox-platform-gateway-svc/ # API 网关服务
├── shared/                        # 共享库
│   ├── mox-unified-algo-core/    # 统一算法核心（相似度/排序/图算法）
│   └── mox-unified-embed-core/   # 统一嵌入核心
├── crates/                        # 语言绑定
│   ├── python/                    # Python 绑定
│   └── nodejs/                    # Node.js 绑定
└── legacy/                        # 遗留代码（防腐层）
```

### 4.2 每域六层结构

每个业务域严格遵循六层结构：

| 层 | 目录 | 职责 | 依赖方向 |
|----|------|------|----------|
| **API** | `api/` | 对外 API 定义、请求/响应类型 | 依赖 proto |
| **Core** | `core/` | 核心业务逻辑、领域模型、算法 | 依赖 foundation, shared |
| **Proto** | `proto/` | 协议类型、枚举、常量（SSOT） | 无业务依赖 |
| **SDK** | `sdk/` | 客户端 SDK，封装 API 调用 | 依赖 api, proto |
| **Svc** | `svc/` | 服务实现，HTTP/gRPC 服务端 | 依赖 core, api |
| **SvcAPI** | `svcapi/` | 服务 API 路由、中间件组装 | 依赖 svc |

**依赖规则**：内层不依赖外层，同层不互相依赖（通过 proto 解耦）。

---

## 5. 前端架构（frontend-ui）

### 5.1 技术栈

| 类别 | 技术 | 版本 | 用途 |
|------|------|------|------|
| 框架 | Vue | ^3.4 | 响应式 UI 框架 |
| 构建 | Vite | ^5.0 | 极速构建与 HMR |
| UI 库 | Element Plus | ^2.4 | 企业级组件库 |
| 状态管理 | Pinia | ^2.2 | 组合式状态管理 |
| 路由 | Vue Router | ^4.3 | 前端路由 |
| HTTP | Axios | ^1.6 | HTTP 客户端（含拦截器） |
| 图表 | ECharts | ^5.4 | 数据可视化 |
| 图谱 | 3d-force-graph | ^1.0 | 3D 力导向图 |
| 3D | Three.js | ^0.185 | 3D 渲染 |
| Markdown | markdown-it | ^15.0 | Markdown 渲染 |
| 流程图 | Mermaid | ^11.17 | 流程图渲染 |
| 乐谱 | VexFlow | ^5.0 | 乐谱渲染 |
| 工具 | @vueuse/core | ^11.0 | Vue 组合式工具集 |
| 校验 | Zod | ^3.23 | 运行时类型校验 |

### 5.2 目录结构

```
frontend-ui/
├── src/
│   ├── api/                      # API 接口层（25个模块，按业务域拆分）
│   │   ├── http.js               # HTTP 核心实例（拦截器/重试/归一化）
│   │   ├── index.js              # 统一导出
│   │   ├── alliance.js           # 专家联盟 SSE
│   │   ├── experts.api.js        # 专家管理
│   │   ├── ai.api.js             # AI 对话
│   │   ├── llm.api.js            # LLM 模型管理
│   │   ├── graph.api.js          # 知识图谱
│   │   ├── kb.api.js             # 知识库
│   │   ├── workflow.api.js       # 工作流
│   │   ├── system.api.js         # 系统管理
│   │   ├── monitor.api.js        # 监控
│   │   └── ...
│   ├── components/               # 全局可复用组件
│   │   ├── common/               # 通用基础组件（无业务逻辑）
│   │   │   ├── DataTable.vue
│   │   │   ├── SearchForm.vue
│   │   │   ├── Pagination.vue
│   │   │   ├── StatusTag.vue
│   │   │   ├── EmptyState.vue
│   │   │   ├── LoadingState.vue
│   │   │   ├── ConfirmDialog.vue
│   │   │   └── PageHeader.vue
│   │   ├── layout/               # 布局组件
│   │   ├── expert/               # 专家相关组件
│   │   ├── ai/                   # AI 相关组件
│   │   ├── PhasePipeline.vue     # 阶段管线组件
│   │   ├── MessageBubble.vue     # 消息气泡
│   │   └── ...
│   ├── composables/              # 组合式函数（逻辑复用）
│   │   ├── useTheme.js
│   │   ├── useKnowledgeBase.js
│   │   ├── useMessageActions.js
│   │   ├── projectContext.js
│   │   └── workspace/            # 工作台专用
│   │       ├── useAlliance.js
│   │       ├── useGraphCanvas.js
│   │       ├── useTaskOrchestration.js
│   │       └── useWhiteboard.js
│   ├── stores/                   # Pinia 状态管理
│   │   ├── app.store.js
│   │   ├── auth.store.js
│   │   ├── user.store.js
│   │   ├── ai.store.js
│   │   ├── project.store.js
│   │   ├── permission.store.js
│   │   └── ui.store.js
│   ├── views/                    # 页面视图（按业务域组织）
│   │   ├── ai/                   # AI 域
│   │   ├── expert/               # 专家域
│   │   ├── graph/                # 图谱域
│   │   ├── project/              # 项目域
│   │   ├── workspace/            # 工作台
│   │   ├── admin/                # 管理域
│   │   └── misc/                 # 通用页面
│   ├── router/                   # 路由配置
│   ├── constants/                # 常量定义
│   ├── styles/                   # 全局样式与设计 token
│   │   ├── global.css
│   │   └── themes/               # 4套主题
│   ├── utils/                    # 工具函数
│   ├── types.js                  # 类型定义
│   ├── App.vue                   # 根组件
│   └── main.js                   # 应用入口
├── public/                       # 静态资源
├── tests/                        # 测试
├── stories/                      # Storybook
├── package.json
├── vite.config.js
└── MODULE-MANIFEST.md           # 模块清单（自动生成）
```

### 5.3 HTTP 拦截器规范

`http.js` 是前端核心基础设施，必须实现：

1. **请求拦截器**：
   - 自动注入 Authorization Bearer Token
   - 自动注入 X-Request-Id（crypto.randomUUID）
   - 自动注入当前 project_id（GET 走 params，POST 走 body）
   - 生产环境禁用默认令牌

2. **响应拦截器**：
   - 统一信封解包：`{code, msg, data}`（新格式）和 `{success, data}`（旧格式兼容）
   - code=0 自动返回 data 本体
   - code≠0 统一带 code 前缀拒绝

3. **错误拦截器**：
   - 指数退避重试（默认2次，502/503/504/网络错误）
   - 幂等请求（GET/HEAD/OPTIONS）自动重试，非幂等需显式 `_retryOnPost`
   - 错误消息归一化：优先响应体字段，缺失时按状态码中文兜底
   - 401 始终提示并广播 `mox:auth-failed` 事件
   - 503/超时/网络错误弹全局提示，`silent` 请求静默

4. **多实例工厂**：
   - `createHttpInstance(baseURL)` 创建独立实例，共享拦截器
   - 业务面 `/api` 与管理面 `/actuator` 分离

---

## 6. 文档体系架构（docs）

### 6.1 文档分类

| 类别 | 目录 | 用途 | 规范 |
|------|------|------|------|
| **企业级文档** | `enterprise/` | 需求、架构、设计、验收报告（编号00-37） | 每篇必须有编号、版本、状态 |
| **架构文档** | `architecture/` | 架构规范、扩展指南、错误码参考 | 与代码同步更新 |
| **专家联盟** | `expert-alliance/` | 联盟架构 v2/v3、领域模型、API设计 | 版本化管理 |
| **标准规范** | `standards/`, `specs/` | 架构标准、端口规范、开发规范 | SSOT，跨端引用 |
| **微服务** | `microservices/` | 服务边界、通信、数据、部署 | 6篇系列文档 |
| **数据库** | `database/` | DDL、迁移计划、标准评审 | 版本化迁移 |
| **验证报告** | `enterprise-verification/`, `working-reports/` | 测试输出、性能基准、质量审计 | 每次验证归档 |
| **API 规范** | `api/` | REST/TCP API 规范 | 与后端路由对齐 |
| **模块文档** | `modules/` | 各模块详细设计 | 按模块组织 |

### 6.2 文档驱动开发流程

```
需求提出 → 07-mox 模块化系统架构需求明确书.md
    ↓
架构设计 → 02-architecture.md + ADR-XX 决策记录
    ↓
详细设计 → 03-design.md + 模块设计文档
    ↓
开发实现 → 代码 + 单元测试
    ↓
验证测试 → working-reports/XX-验证报告.md
    ↓
验收归档 → enterprise/XX-验收报告.md
```

### 6.3 ADR（架构决策记录）规范

每篇 ADR 必须包含：

- **标题**：ADR-XX + 决策主题
- **状态**：Proposed / Accepted / Deprecated / Superseded
- **背景**：问题描述和上下文
- **决策**：做出的选择
- **后果**：正面影响、负面影响、需后续跟进
- **替代方案**：考虑过的其他方案及不选原因

---

## 7. 核心模块详细设计

### 7.1 联盟引擎（AllianceEngine）

**位置**：`domains/ai/core/mox-ai-alliance-engine/`

**6阶段管线**：

```
Intent → Team → Debate → Synthesize → Gate → Learn → Done
  ①       ②       ③          ④          ⑤       ⑥      ⑦
```

| 阶段 | 处理器 | 输入 | 输出 | 关键逻辑 |
|------|--------|------|------|----------|
| **① Intent** | IntentClassifier | query: String | IntentResult | 关键词分类 + 置信度 + 降级标记 |
| **② Team** | TeamAssembler | IntentResult + team_size | TeamResult | 按意图匹配专家 + 敏感域自动加入安全/权限专家 |
| **③ Debate** | DebateEngine | query + TeamResult | DebateResult | 并行咨询专家 + 共识度计算 + 加权合成 |
| **④ Synthesize** | - | DebateResult | Markdown | 按权重排序专家观点 + Top-3 合成结论 |
| **⑤ Gate** | QualityGate | Intent+Team+Debate | GateResult | 14维归一化 + ABCD分级 + D级阻断 |
| **⑥ Learn** | KnowledgeLearner | GateScore+Intent+Debate | LearnResult | EWMA 更新专家画像权重 |
| **⑦ Done** | - | 全部 | AllianceEvent×7 | 7事件 + 7审计 + trace_id |

**共识度公式**：
```
σ = sqrt(Σ(score_i - mean)² / n)
σ_norm = min(σ / (max - min + 1e-9), 1.0)
consensus = (1 - σ_norm) × 0.70 + avg_confidence × 0.30
```

**合成权重公式**：
```
w = 0.50 × score + 0.30 × confidence + 0.20 × (priority/100)
超时专家 w = 0
归一化：w_i / Σw
```

### 7.2 十四维归一化（Normalizer）

**位置**：`domains/ai/core/mox-ai-expert-core/src/normalize.rs`

**归一化规则**：
```
基础分 = clamp(expert.score, 0, 1)

if has_veto(expert):
    score = 0
else:
    score = max(0, score - count_blocking(expert) × 0.3)
    score = max(0, score - count_warning(expert) × 0.1)

normalized_score = score × dimension_weight(dimension)
overall_score = Σ(normalized_score_i) / Σ(weight_i)
```

**质量分级**：
| 等级 | 分数范围 | 处理 |
|------|----------|------|
| **A** | ≥ 0.85 | 通过，优质交付 |
| **B** | ≥ 0.70 | 通过，标准交付 |
| **C** | ≥ 0.50 | 有条件通过，可重试优化 |
| **D** | < 0.50 | 阻断，必须修复后重新提交 |

**维度权重（SSOT）**：Permission/Security 权重最高，Business/Documentation 权重适中。

### 7.3 任务调度器（TaskScheduler）

**位置**：`domains/alliance/core/mox-alliance-scheduler-core/`

**任务状态机**：
```
Pending → Planning → Running → Completed
                          ↓         ↓
                        Paused    Failed
                          ↓
                       Cancelled
```

**核心能力**：
- 队列容量控制（`queue_capacity`）
- 并发上限控制（`max_concurrent_tasks`）
- 租户隔离（`tenant_id` 校验，跨租户返回 TenantMismatch）
- 专家匹配（`ExpertMatcher` trait，规则匹配/向量匹配可插拔）
- 计划生成（`SimplePlanGenerator`，DAG 计划）
- 执行器桥接（`ExecutorBridge` trait，进程内/HTTP远程可插拔）
- 状态同步（从执行器同步进度到本地任务）
- 暂停/恢复/取消

**持久化**：`TaskRepository` trait，当前 `InMemoryTaskRepository`，生产环境需实现 `DatabaseTaskRepository`（PostgreSQL）。

### 7.4 弹性框架（Resilience）

**位置**：`framework/src/resilience.rs`

**四种弹性模式**：

| 模式 | 实现 | 配置 |
|------|------|------|
| **熔断器** | CLOSED→OPEN→HALF_OPEN 三状态机 | 失败率阈值、恢复超时、半开探测数 |
| **令牌桶限流** | 令牌按速率补充，桶容量上限 | rate（每秒令牌数）、capacity（桶容量） |
| **舱壁隔离** | 信号量计数，acquire/release | max_concurrent（最大并发数） |
| **指数退避重试** | delay = base × 2^attempt + jitter | max_retries、base_delay、max_delay |

**使用方式**：所有跨进程调用（HTTP/gRPC/数据库）必须包裹弹性层。

### 7.5 统一算法核心（UnifiedAlgoCore）

**位置**：`shared/mox-unified-algo-core/`

**算法分类**：

| 类别 | 算法 | 状态 | 用途 |
|------|------|------|------|
| **相似度** | 余弦相似度 | ✅ 已实现 | 专家画像匹配、文档向量检索 |
| | Jaccard 相似度 | ✅ 已实现 | 标签重叠、共同邻居 |
| | Levenshtein 编辑距离 | ✅ 已实现 | 名称模糊匹配 |
| | 加权混合相似度 | ✅ 已实现 | 多维度融合匹配 |
| **排序** | 加权评分排序 | ✅ 已实现 | 专家排序、结果排序 |
| **图算法** | PageRank | ⚠️ feature gate | 知识图谱节点重要性 |
| | 中心性 | ⚠️ feature gate | 节点中心度 |
| | 社区发现 | ⚠️ feature gate | 图谱聚类 |
| | 最短路径 | ⚠️ feature gate | 路径查询 |
| **聚类** | K-Means | ❌ 存根 | 专家聚类 |
| | DBSCAN | ❌ 存根 | 密度聚类 |

**统一 Algorithm trait**：所有算法必须实现 `id()`, `name()`, `version()`, `description()`。

---

## 8. 数据模型设计

### 8.1 核心实体

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Tenant    │     │    User     │     │   Project   │
│  (租户)      │1───*│  (用户)      │     │  (项目)      │
└──────┬──────┘     └─────────────┘     └──────┬──────┘
       │                                           │
       │                                           │
┌──────▼──────┐     ┌─────────────┐     ┌──────▼──────┐
│  Expert     │     │  Alliance   │     │  Task       │
│  (专家)      │*───*│ (联盟运行)   │1───*│  (任务)      │
└──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                     │                     │
       │                     │                     │
┌──────▼──────┐     ┌──────▼──────┐     ┌──────▼──────┐
│ExpertOpinion│     │ AllianceEvent│     │ Collaboration│
│  (专家观点)   │     │  (联盟事件)   │     │  Plan(协作计划)│
└─────────────┘     └─────────────┘     └─────────────┘
```

### 8.2 关键表设计

**专家表（experts）**：
| 字段 | 类型 | 说明 |
|------|------|------|
| expert_id | UUID | 专家唯一标识 |
| tenant_id | UUID | 所属租户 |
| name | VARCHAR(128) | 专家名称 |
| dimension | VARCHAR(32) | 领域维度（security/permission/algorithm...） |
| description | TEXT | 专家描述 |
| capabilities | JSONB | 能力标签数组 |
| gate_a_rate_30d | FLOAT | 近30天A级通过率 |
| total_runs | INTEGER | 总运行次数 |
| status | VARCHAR(16) | 状态（active/inactive/maintenance） |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

**联盟运行表（alliance_runs）**：
| 字段 | 类型 | 说明 |
|------|------|------|
| run_id | UUID | 运行唯一标识（trace_id） |
| tenant_id | UUID | 租户 |
| user_id | UUID | 发起用户 |
| project_id | UUID | 所属项目 |
| query | TEXT | 用户查询 |
| intent | VARCHAR(32) | 意图分类 |
| team_size | INTEGER | 团队规模 |
| consensus | FLOAT | 共识度 |
| gate_grade | CHAR(1) | 质量等级（A/B/C/D） |
| gate_score | FLOAT | 质量分数 |
| synthesis | TEXT | 合成结果（Markdown） |
| status | VARCHAR(16) | 状态（running/completed/failed） |
| started_at | TIMESTAMP | 开始时间 |
| completed_at | TIMESTAMP | 完成时间 |
| latency_ms | INTEGER | 总耗时 |

**联盟事件表（alliance_events）**：
| 字段 | 类型 | 说明 |
|------|------|------|
| event_id | UUID | 事件唯一标识 |
| run_id | UUID | 关联运行 |
| phase | VARCHAR(16) | 阶段（intent/team/debate/...） |
| trace_id | UUID | 全链路追踪ID |
| payload | JSONB | 事件载荷 |
| latency_ms | INTEGER | 阶段耗时 |
| degraded | BOOLEAN | 是否降级 |
| degrade_reason | VARCHAR(256) | 降级原因 |
| created_at | TIMESTAMP | 创建时间 |

**任务表（tasks）**：
| 字段 | 类型 | 说明 |
|------|------|------|
| task_id | UUID | 任务唯一标识 |
| tenant_id | UUID | 租户 |
| user_id | UUID | 发起用户 |
| project_id | UUID | 所属项目 |
| title | VARCHAR(256) | 任务标题 |
| description | TEXT | 任务描述 |
| task_type | VARCHAR(32) | 任务类型 |
| priority | VARCHAR(16) | 优先级 |
| mode | VARCHAR(16) | 执行模式（parallel/sequential） |
| fusion_strategy | VARCHAR(16) | 融合策略 |
| status | VARCHAR(16) | 状态（pending/planning/running/paused/completed/failed/cancelled） |
| progress | FLOAT | 进度（0-1） |
| plan | JSONB | 协作计划（DAG） |
| started_at | TIMESTAMP | 开始时间 |
| completed_at | TIMESTAMP | 完成时间 |
| duration_ms | INTEGER | 执行耗时 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 9. API 设计规范

### 9.1 REST API 规范

**URL 规范**：
- 基础路径：`/api/{domain}/{resource}`
- 资源名用复数：`/api/experts`, `/api/alliance/runs`
- 嵌套资源：`/api/projects/{project_id}/tasks`
- 动作用子路径：`/api/tasks/{task_id}/cancel`, `/api/tasks/{task_id}/pause`

**HTTP 方法**：
| 方法 | 用途 | 幂等 |
|------|------|------|
| GET | 查询资源 | ✅ |
| POST | 创建资源/触发动作 | ❌ |
| PUT | 全量更新 | ✅ |
| PATCH | 部分更新 | ❌ |
| DELETE | 删除资源 | ✅ |

**统一响应信封**：
```json
{
  "code": 0,
  "msg": "success",
  "data": { ... },
  "trace_id": "uuid",
  "latency_ms": 123
}
```
- `code=0` 表示成功，非0表示错误
- `data` 为业务数据，无数据时可省略
- `trace_id` 全链路追踪ID，必须透传
- `latency_ms` 服务端处理耗时

**错误码规范**：
- 格式：`{DOMAIN}-{NUMBER}`，如 `ALLIANCE-GATE_BLOCKED`
- HTTP 状态码与业务错误码分离
- 错误消息必须中文，面向用户

### 9.2 SSE 流式 API 规范

**端点**：`GET /api/alliance/stream?query=...`

**事件格式**：
```
event: phase_started
data: {"phase":"intent","trace_id":"uuid"}

event: phase_data
data: {"phase":"intent","trace_id":"uuid","payload":{...},"latency_ms":5}

event: progress
data: {"phase":"debate","trace_id":"uuid","current":1,"total":4,"message":"第 1/4 位专家咨询中"}

event: complete
data: {"trace_id":"uuid","total_ms":1234,"gate_passed":true,"gate_grade":"A"}

event: error
data: {"trace_id":"uuid","code":"GATE_BLOCKED","message":"质量门禁 D 级阻断"}
```

**事件类型**：
| 事件 | 说明 |
|------|------|
| `phase_started` | 阶段开始 |
| `phase_data` | 阶段数据（每阶段一个） |
| `progress` | 进度更新（辩论阶段用） |
| `complete` | 全部完成 |
| `error` | 错误中断 |

### 9.3 gRPC 契约规范（Rust ↔ Python）

**服务定义**（Protobuf）：
```protobuf
service LLMInferenceService {
  rpc Chat(ChatRequest) returns (stream ChatResponse);
  rpc Embed(EmbedRequest) returns (EmbedResponse);
}

service VectorStoreService {
  rpc Upsert(UpsertRequest) returns (UpsertResponse);
  rpc Search(SearchRequest) returns (SearchResponse);
  rpc Delete(DeleteRequest) returns (DeleteResponse);
}
```

**Rust 侧**：实现 `ExpertConsultant` trait，内部为 gRPC client
**Python 侧**：实现 gRPC server，调用 vLLM/FAISS 等

---

## 10. 安全架构

### 10.1 认证与授权

| 层级 | 机制 | 说明 |
|------|------|------|
| **认证** | JWT Bearer Token | 前端 http.js 自动注入 Authorization 头 |
| **授权** | RBAC（基于角色的访问控制） | 用户→角色→权限，前端 permission.store + v-permission 指令 |
| **租户隔离** | tenant_id 强制校验 | 所有查询必须带 tenant_id，跨租户返回 403 |
| **项目隔离** | project_id 自动注入 | 前端 http.js 自动注入当前 project_id |
| **API 令牌** | OUS_API_TOKEN | 服务间调用，生产环境禁用默认令牌 |

### 10.2 数据安全

- **传输加密**：全链路 HTTPS/TLS
- **存储加密**：敏感字段（密钥、密码）加密存储
- **脱敏**：日志中不记录敏感信息（token、密码、密钥）
- **审计**：所有敏感操作记录审计日志（谁、何时、做了什么、结果）

### 10.3 输入校验

- **前端**：Zod 运行时校验 API 响应，Element Plus 表单校验用户输入
- **后端**：serde 反序列化校验，自定义 validator 校验业务规则
- **SQL 注入**：使用参数化查询，禁止字符串拼接 SQL
- **XSS**：前端 markdown-it 配置安全选项，输出转义

---

## 11. 可观测性架构

### 11.1 三大支柱

| 支柱 | 技术 | 说明 |
|------|------|------|
| **日志** | tracing + 结构化 JSON | 全链路结构化日志，含 trace_id |
| **指标** | Prometheus + metrics crate | QPS、延迟、错误率、资源使用 |
| **追踪** | OpenTelemetry + Jaeger | 分布式链路追踪，跨服务透传 trace_id |

### 11.2 全链路追踪

**trace_id 生成与透传**：
1. 前端生成 `X-Request-Id`（crypto.randomUUID）
2. 后端网关接收，作为 `trace_id` 透传到所有下游服务
3. 联盟引擎每个阶段事件携带 `trace_id`
4. 日志、指标、追踪全部关联 `trace_id`
5. 前端展示 `trace_id`，用户可用于问题排查

### 11.3 健康检查

**管理面端点**（`/actuator/*`）：
| 端点 | 说明 |
|------|------|
| `/actuator/health` | 健康检查（liveness + readiness） |
| `/actuator/metrics` | 指标导出（Prometheus 格式） |
| `/actuator/info` | 服务信息（版本、构建时间、Git commit） |

前端 `actuator.api.js` 独立实例，与业务面分离。

---

## 12. 部署架构

### 12.1 部署拓扑

```
                    ┌─────────────┐
                    │   Nginx     │  反向代理 + 静态资源
                    │  (前端+网关) │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────▼─────┐ ┌───▼──────┐ ┌──▼─────────┐
        │  Gateway   │ │ Frontend │ │  Docs      │
        │  (Rust)    │ │ (静态)   │ │ (静态)     │
        └─────┬─────┘ └──────────┘ └────────────┘
              │
    ┌─────────┼─────────┬──────────┐
    │         │         │          │
┌───▼───┐ ┌──▼───┐ ┌───▼──┐ ┌───▼────┐
│AI Svc │ │Alliance│ │KG Svc│ │Other   │
│(Rust) │ │ Svc   │ │(Rust)│ │ Svcs   │
└───┬───┘ └──┬────┘ └──┬───┘ └───┬────┘
    │         │         │          │
    └─────────┴────┬────┴──────────┘
                     │ gRPC
              ┌──────▼──────┐
              │ Python 智能层 │
              │ (LLM/向量/RAG)│
              └──────┬──────┘
                     │
              ┌──────▼──────┐
              │  数据层       │
              │ PG/Redis/对象存储│
              └─────────────┘
```

### 12.2 环境划分

| 环境 | 用途 | 数据 |
|------|------|------|
| **dev** | 开发调试 | Mock 数据 |
| **test** | 测试验证 | 测试数据集 |
| **staging** | 预发布验证 | 生产数据脱敏副本 |
| **prod** | 生产运行 | 真实数据 |

### 12.3 配置管理

- 配置文件：YAML 格式，按环境分离
- 敏感配置：环境变量注入，不入库
- 动态配置：etcd/Consul 配置中心，支持热更新
- 配置校验：启动时校验配置完整性，非法配置拒绝启动

---

## 13. 开发规范

### 13.1 Rust 开发规范

| 规范 | 说明 |
|------|------|
| **错误处理** | 统一使用 `mox-error` 错误体系，禁止 `unwrap()`/`panic!()`（测试除外） |
| **异步** | 使用 `tokio` 运行时，`async_trait` 定义异步 trait |
| **依赖注入** | 依赖通过 trait 抽象，构造函数注入，禁止全局单例（配置除外） |
| **测试** | 单元测试内联在 `#[cfg(test)] mod tests`，集成测试在 `tests/` 目录 |
| **文档** | 公共项必须有 `///` 文档注释，包含示例 |
| **格式化** | `cargo fmt` 统一格式，`cargo clippy` 静态检查 |
| **不安全代码** | `unsafe` 必须有安全注释说明原因 |

### 13.2 前端开发规范

| 规范 | 说明 |
|------|------|
| **组件命名** | PascalCase，多词（避免与 HTML 元素冲突） |
| **组合式 API** | 使用 `<script setup>` 语法，优先使用 composables 复用逻辑 |
| **状态管理** | 全局状态用 Pinia，组件局部状态用 ref/reactive |
| **API 调用** | 必须通过 `api/` 模块，禁止在组件中直接使用 axios |
| **样式** | scoped 样式 + CSS 变量，大组件样式提取为独立 CSS |
| **类型** | 使用 JSDoc + Zod 运行时校验，关键数据结构定义在 `types.js` |
| **测试** | 组件用 Vitest + @vue/test-utils，E2E 用 Playwright |
| **格式化** | ESLint + Prettier 统一格式 |

### 13.3 文档规范

| 规范 | 说明 |
|------|------|
| **Markdown** | 标准 Markdown，标题层级不超过4级 |
| **图表** | Mermaid 流程图，架构图用 HTML/SVG |
| **编号** | 企业级文档按 `XX-名称.md` 编号 |
| **版本** | 每篇文档头部有版本、日期、状态、变更记录 |
| **ADR** | 架构决策必须有 ADR 记录，状态明确 |

---

## 14. 测试策略

### 14.1 测试金字塔

```
        ┌─────────────┐
        │   E2E 测试   │  5%  Playwright，关键用户旅程
        ├─────────────┤
        │  集成测试    │  15%  跨模块/跨服务集成
        ├─────────────┤
        │  单元测试    │  80%  函数/模块级，核心算法100%覆盖
        └─────────────┘
```

### 14.2 后端测试

| 类型 | 工具 | 覆盖要求 |
|------|------|----------|
| **单元测试** | Rust `#[test]` + `cargo test` | 核心算法（归一化/共识度/相似度）100%覆盖 |
| **集成测试** | `tests/` 目录 | 联盟引擎端到端、调度器全生命周期 |
| **基准测试** | `criterion` | 关键路径性能基准（辩论并行、相似度计算） |
| **故障注入** | 模拟网络延迟/错误 | 弹性模式验证（熔断/限流/重试） |

### 14.3 前端测试

| 类型 | 工具 | 覆盖要求 |
|------|------|----------|
| **单元测试** | Vitest + @vue/test-utils + happy-dom | composables、utils、stores 全覆盖 |
| **组件测试** | Vitest + @vue/test-utils | 通用组件（common/）全覆盖 |
| **E2E 测试** | Playwright | 关键用户旅程（登录→创建任务→联盟分析→查看结果） |
| **组件文档** | Storybook | 通用组件必须有 stories |
| **性能审计** | Lighthouse | 首屏性能、可访问性、最佳实践 |

### 14.4 验证报告归档

每次验证必须生成报告，归档到 `docs/working-reports/`：
- 测试覆盖率报告
- 性能基准报告
- 缺陷清单与修复状态
- 验收结论

---

## 15. 性能与扩展性

### 15.1 性能目标

| 指标 | 目标 | 说明 |
|------|------|------|
| **API P99 延迟** | < 500ms | 非LLM调用的普通API |
| **联盟分析总耗时** | < 30s | 4专家团队，含LLM调用 |
| **SSE 首字节延迟** | < 1s | 从请求到第一个事件 |
| **并发用户数** | > 1000 | 单集群支持 |
| **前端首屏加载** | < 2s | 生产环境，gzip压缩 |

### 15.2 扩展性设计

| 维度 | 方案 |
|------|------|
| **水平扩展** | 无状态服务，多实例部署，负载均衡 |
| **专家并行** | DebateEngine 使用 `futures::join_all` 并行咨询专家 |
| **任务分片** | 大任务拆分为子任务，分布式执行 |
| **缓存** | Redis 缓存热点数据（专家列表、配置、会话） |
| **异步处理** | 长耗时任务异步化，立即返回 task_id，SSE/轮询获取结果 |
| **数据库扩展** | 读写分离、分库分表（按 tenant_id 分片） |

### 15.3 前端性能优化

- **路由级代码分割**：Vite 自动按路由分割 chunk
- **重型依赖懒加载**：three.js、echarts、mermaid、vexflow 按需加载
- **组件懒加载**：`defineAsyncComponent` 懒加载非首屏组件
- **图片优化**：WebP 格式、懒加载、响应式尺寸
- **虚拟列表**：长列表使用虚拟滚动
- **Gzip/Brotli**：生产环境压缩静态资源

---

## 16. 开发路线图

### Phase 1：核心可用（2周）

**目标**：让系统真正"有大脑"，联盟引擎接入LLM

- [ ] Python 侧：`llm-inference-svc`（vLLM + gRPC，流式Chat + Embed）
- [ ] Rust 侧：`LLMExpertConsultant` 实现 `ExpertConsultant` trait
- [ ] Rust 侧：gRPC client 集成到联盟引擎
- [ ] 前端：`llm.api.js` 模型管理，ChatView 展示真实LLM输出
- [ ] 端到端验证：用户查询 → 6阶段管线 → LLM专家观点 → 合成输出
- [ ] 文档：`LLM-INTEGRATION-GUIDE.md`

**验收标准**：专家观点不再是模板字符串，而是LLM生成的真实分析；SSE流式展示正常。

### Phase 2：生产级加固（3周）

**目标**：满足生产环境要求，数据不丢失，高并发不阻塞

- [ ] Rust 侧：`DatabaseTaskRepository`（PostgreSQL），替换内存存储
- [ ] Rust 侧：调度器异步化，`submit_task` 立即返回
- [ ] Rust 侧：分布式执行器桥接（HTTP远程执行）
- [ ] 前端：`useSSE` composable，处理重连/超时/取消
- [ ] 前端：Zod API 响应运行时校验
- [ ] 前端：路由级代码分割 + 重型依赖懒加载
- [ ] 运维：Docker Compose 部署脚本，Nginx 反向代理配置
- [ ] 监控：Prometheus 指标采集，Grafana 仪表盘
- [ ] 文档：`DEPLOYMENT-GUIDE.md`、`OPERATIONS-MANUAL.md`

**验收标准**：服务重启数据不丢失；100并发下API P99 < 500ms；Docker一键部署。

### Phase 3：P1域填充（4周）

**目标**：KG/Cloud/Flow 三大域核心功能可用

- [ ] KG域：图算法核心（PageRank/社区发现/最短路径）接入统一算法核心
- [ ] KG域：图谱可视化前端（3d-force-graph 集成）
- [ ] Cloud域：知识库云盘核心（文档上传/向量索引/检索）
- [ ] Cloud域：前端知识库面板接入真实API
- [ ] Flow域：工作流引擎核心（DAG定义/执行/状态追踪）
- [ ] Flow域：前端工作流编辑器
- [ ] 文档：各域架构文档 + API文档

**验收标准**：P1域核心功能可端到端运行，前端移除Mock降级。

### Phase 4：企业级高级能力（4-6周）

**目标**：达到企业级最优，支持大规模部署

- [ ] 多租户隔离增强：数据层租户隔离 + RBAC 增强
- [ ] 学习排序：LambdaMART 动态调整质量权重
- [ ] 模型微调：LoRA 微调领域专家模型
- [ ] 可观测性：OpenTelemetry 全链路 trace 接入 Jaeger
- [ ] 高可用：多节点部署，服务发现，故障自动转移
- [ ] 安全：安全审计日志，敏感数据加密，渗透测试
- [ ] 性能：性能基准测试，瓶颈优化，缓存策略优化
- [ ] 文档：完整企业级文档体系

**验收标准**：支持多节点部署；可观测性完善；通过安全审计；专家模型可领域微调。

---

## 附录

### A. 术语表

| 术语 | 说明 |
|------|------|
| **OUS** | Operator Unified System，算子统一系统 |
| **DDD** | Domain-Driven Design，领域驱动设计 |
| **ADR** | Architecture Decision Record，架构决策记录 |
| **SSOT** | Single Source Of Truth，单一事实来源 |
| **SSE** | Server-Sent Events，服务器推送事件 |
| **RAG** | Retrieval-Augmented Generation，检索增强生成 |
| **RBAC** | Role-Based Access Control，基于角色的访问控制 |
| **EWMA** | Exponentially Weighted Moving Average，指数加权移动平均 |

### B. 参考文档

- `docs/enterprise/02-architecture.md` — 企业级架构
- `docs/architecture/NORMALIZED_ARCHITECTURE.md` — 归一化架构
- `docs/expert-alliance/v3/01-architecture-optimization.md` — 联盟架构优化
- `docs/standards/` — 标准规范系列
- `docs/microservices/` — 微服务系列

---

*文档结束*
