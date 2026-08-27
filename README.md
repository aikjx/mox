# 璇玑 RelGraph · 全域归一化知识图谱协同平台（父系统 OUS = 算子统一系统）

> **对外产品名**：**璇玑 RelGraph**（唯一产品标识 · ADR-DOC-011）｜底层父系统代号：**OUS（Operator Unified System）**｜内部图谱别名：**关图**
> **版本与权威**：v3.1.0-ai-powered（M0 全域归一化阶段）· **🟢 第一级权威（L0 TOP-MASTER）** → [`docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`](docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md)（三联盟共同签署 · 12 章统摄全局）
> **治理组织**：**三联盟模式（产品联盟 / 算法联盟 / 开发联盟）** — 铁律：产品开口 · 算法量尺 · 开发出手（All-01 三联盟铁规）
>
> 基于 6 大数学公理构建的通用计算框架，实现「万物皆算子 + 一切皆是图 = 归一化唯一事实基准」的范畴论统一抽象。
> 一个面向企业级生产环境的 **算子编排 / 流程优化 / 知识图谱 / AI 智能体 + 三联盟协同治理** 一体化平台：
> - 通过**一张八层图谱（L0~L7）× 14 节点族 × 19 边族**作为唯一事实基准（ADR-DOC-004），实现一改全链联动、零重复造轮子；
> - 通过**十大标准业务流程（BP-1~10）· 四归三连铁律**，使需求↔架构↔业务↔文档四方同步不漂移；
> - 通过**八大算法家族（A1~A8）· 九级里程碑（M0~M8）· 三级验收门槛**，保证架构/算法/工程每步都最优、每步可验证；
> - 通过**统一 AI 入口 Rust Gateway 四端点** + **激活扩散 A5 意图路由** + **CEM A7 交叉熵持续优化**，使 AI 查询如本地查询般丝滑。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/lang-Rust%202021-orange.svg)](https://www.rust-lang.org/)
[![Vue3](https://img.shields.io/badge/frontend-Vue3%20%2B%20Three.js%20%2B%20ECharts-green.svg)](https://vuejs.org/)
[![三联盟](https://img.shields.io/badge/Governance-产品联盟%20%2F%20算法联盟%20%2F%20开发联盟-purple.svg)](#三联盟治理--四归三连铁律---)
[![里程碑 M0](https://img.shields.io/badge/Roadmap-M0~M8-brightgreen.svg)](#9-里程碑规划--对齐-docsenterprise05-iteration-roadmapmd)
[![算法 A1~A8](https://img.shields.io/badge/%E5%85%AB%E5%A4%A7%E7%AE%97%E6%B3%95-CNM%20Brandes%20Harmonic%20PageRank%20Spread%20RRF%20CEM%20CPM%E3%83%BBRCPSP-8A2BE2)](#-八大算法家族-a1a8--docsenterprise02-architecturemd-02---对齐-docsenterprise18-v)
[![TOP-MASTER](https://img.shields.io/badge/%F0%9F%9F%A2%20L0%20%E6%9C%80%E9%AB%98%E6%9D%83%E5%A8%81-18%20TOP--MASTER-00b894)](docs/enterprise/18-全域顶层总设计-三联盟模式-V1.0.md)

---

## 📋 项目元信息（Project Metadata）

| 项 | 内容 |
|---|---|
| **对外产品名** | 璇玑 RelGraph |
| **底层父系统** | OUS（Operator Unified System · 算子统一系统） |
| **仓库代号** | infotopograph（关图） |
| **开发周期** | 2026-08-06 起 · 持续迭代中（最新提交 2026-08-27） |
| **治理组织** | 三联盟模式（产品联盟 / 算法联盟 / 开发联盟） |
| **开源协议** | MIT License（详见 [LICENSE](LICENSE)） |
| **版权所有** | © 2026 璇玑 RelGraph · 算子统一系统（OUS）· 三联盟 |

### 📦 Git 仓库地址

| 平台 | URL |
|---|---|
| **GitHub（主仓）** | <https://github.com/aikjx/mox.git> |
| **GitCode（镜像）** | <https://gitcode.com/aikjx/mox> |

```bash
# 克隆主仓
git clone https://github.com/aikjx/mox.git infotopograph

# 或克隆镜像
git clone https://gitcode.com/aikjx/mox.git infotopograph
```

---

## 🌟 项目定位

算子统一系统（OUS）是一套 **企业级通用计算与编排底座**（技术父系统代号），其上承载的对外产品「璇玑 RelGraph」实现：

- 以范畴论（Category Theory）、希尔伯特空间（Hilbert Space）、单子（Monad）为数学内核；
- 将任意业务操作抽象为「算子（Operator / Morphism）」，通过「八层图谱（L0~L7）× 14 节点族 × 19 边族」描述全链路关联关系；
- 提供 DAG 调度、关键路径分析、资源约束优化的执行引擎；
- 支持 WASM 插件沙箱，安全扩展第三方算子；
- 内置 AI 智能体（LLM 编排、浏览器自动化、工作流推理）与 ⛨璇玑（多专家协同求解 · 最高权限验证网关）。
- 通过「三联盟治理」+「四归三连铁律」+「P9 先判重后立项」从组织层根治重复造轮子。

适用于：流程自动化、RPA、数据/算法编排、企业知识图谱、低代码算子平台等企业级场景。

---

## 🤝 三联盟治理 & 四归三连铁律（All-01~04 · 对齐 docs/enterprise/07 §三联盟四条铁规）

| 三联盟 | 负责（铁律 All-01） | 自证自验（铁律 All-04） | 主要入口（docs/enterprise/*） |
|--------|-------------------|----------------------|:--:|
| **🎯 产品联盟**（Product Alliance） | 需求「要不要做」一票决定；交付项口径定义 & 客户沟通 | 每条需求必须带**验收断言**（可测）；没有就退回产品联盟自己补 | 07 全维需求明确书 / 15 产品规范标准 / 10 交付清单 / BP-1 3 9 |
| **🔬 算法联盟**（Algorithm Alliance） | 「做不做得对」一票否决：复杂度、Δ 对账、图建模合规 | 算法选型必须带**Δ≤1e-6 对账报告**（8 大算法家族 × 7 数据集）；不达标就迭代算法，不准开发调参 | 02 §0.2 八大算法家族 / 18 TOP-MASTER §五 / BP-6 9 |
| **⚙️ 开发联盟**（Development Alliance） | 「做不做得稳」工程落地：代码、部署、稳定性、安全、可观测 | 代码必须 **clippy 0 告警 + 相关 crate 单测全绿**；否则不准甩锅「需求不清楚」 | 08 自动化 8 步责任 / 14 §3.3 模块表 / BP-2 4 5 8 10 |

**铁律 All-02（先判重 · 再立项）**：任何新模块 / 新算子 / 新需求 / 新璇玑立项 → 先跑 BP-9 `tools/info-graph dedup` → Match Score ≥ 0.85 必须复用 → 零匹配才允许写新 REQ 根。
**铁律 All-03（四归三连全程不脱钩）**：每次改动必须同步：**四归**=需求↔架构↔业务↔文档四方更新；**三连**=联盟责任（06 §5 明确）· 流程（04 BP-xx）· 代码（路径真实）。缺一方 PR 阻断。

详见：[docs/enterprise/07-全维需求明确书.md §三联盟四条铁规 All-01~04](docs/enterprise/07-全维需求明确书.md#三联盟四条铁规alliance--4--三联盟协同闭环的硬约束v11-新增) ·
[docs/enterprise/06-requirements-architecture-map.md §5 三联盟 RACI 矩阵](docs/enterprise/06-requirements-architecture-map.md#5--三联盟责任映射矩阵raci--对齐-adr-doc-002-四归三连)

---

## 🎯 统一 AI 网关四端点（Rust Gateway · ADR-DOC-009 · 对齐 docs/enterprise/02 §6 集成视图）

> 所有 AI 能力统一路由至 `platform/gateway/runtime/`（Axum），路由语义遵循 **AC-10（静态路由优先于参数化路由）**，避免路径匹配歧义。
> 意图识别使用 **A5 激活扩散（个性化 PageRank，d=0.85，30 轮收敛）** 在关图上做个性化排序。

| 方法 | 端点 | 三联盟角色 | 说明 |
|------|------|:--:|------|
| `POST` | `/ai/engine/process` | 开发联盟 R + 算法联盟（A5 路由） | **自动意图识别 → 能力路由**：请求 `{intent, context, principal}` → A5 激活扩散在能力图谱上打分 → 返回最优 capability.route + 执行结果 + route_trace |
| `POST` | `/ai/engine/analyze` | 开发联盟 R + 调用方指定 | **显式能力执行**：请求 `{capability_id, params}` → 直接执行指定能力。绕过意图识别，适合确定性任务。 |
| `GET`  | `/ai/engine/capabilities` | 产品联盟 C + 开发联盟 | **能力矩阵自描述**：返回所有 capability {id, in/out_schema, sla_latency_ms, required_permissions, owners{PA/AA/DA}} 的 JSON 矩阵。前端据此动态渲染能力面板。 |
| `GET`  | `/ai/engine/metrics` | 开发联盟 · 运维 R | **指标端点**：返回三联盟 SLO 指标——`success_rate`（总成功率）、`degrade_rate`（降级率）、`latency_p50/p95/p99_ms`、`capability_breakdown[]`。监控台 MonitorView 对接。 |

---

## 🧠 对话自动知识图谱整理（全自动）

系统支持将**对话内容自动整理进知识图谱并优化布局**，默认开启、零人工干预：

1. **自动落库**：每一次对话（会话/消息）持久化到 SQLite（`operator_dialogue.db`），替代纯前端 localStorage，支持长期积累。
2. **智能抽取**：对话发生时自动调用 LLM 从内容中识别**算子 / 算法 / 概念 / 能力 / 工作流**等实体及其关系；LLM 不可用时降级为关键词规则抽取。
3. **自动优化布局**：抽取结果写入 `operator-graph` 知识图谱，并自动重算 PageRank 中心性与社区发现，回写 `pagerank` / `community` 元数据供前端力导向布局。
4. **统一搜索**：`GET /api/graph/search?q=` 同时检索对话内容与图谱节点。
5. **一键导入导出 / 迁移**：`GET /api/graph/export` 导出「对话 + 知识图谱」打包为单个 JSON 迁移文件；`POST /api/graph/import` 幂等合并恢复，导入后自动重算布局。

| 能力 | 接口 | 说明 |
|------|------|------|
| 自动同步开关 | `POST /api/graph/auto-sync/toggle` / `GET /api/graph/auto-sync/status` | 默认开启，前端 `ChatView` 可切换 |
| 对话会话列表 | `GET /api/dialogue/sessions` | 列出历史会话 |
| 统一搜索 | `GET /api/graph/search?q=&limit=` | 对话 + 图谱节点 |
| 导出迁移包 | `GET /api/graph/export` | 单文件 JSON |
| 导入迁移包 | `POST /api/graph/import` | 幂等合并，自动优化布局 |

前端入口：对话页 `ChatView.vue` 顶部「自动入图」开关 + 导出/导入按钮；图谱页 `GraphView.vue` 顶部搜索框（对话 + 节点统一检索）。

---

## 🧩 六大数学公理

| # | 公理 | 含义 |
|---|------|------|
| 1 | **万物皆算子** | 所有操作抽象为范畴论中的态射 |
| 2 | **状态高维向量** | 系统状态表示为希尔伯特空间中的向量 |
| 3 | **关联关系加权有向图** | 知识图谱基于图论建模 |
| 4 | **范畴论态射规则** | 算子组合满足结合律、单位律 |
| 5 | **资源约束优化** | 基于 DAG 的调度和资源管理 |
| 6 | **扩展性闭包** | 单子模式封装副作用，支持 WASM 插件 |

---

## 📊 全业务流程图

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                              接入层 (Ingress)                                  │
│   前端 Vue3/Three.js          REST API            WebSocket / SSE 实时流        │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                          运行时 (runtime / Axum)                                │
│   HTTP Server ──▶ 鉴权/限流 ──▶ 算子路由 ──▶ 会话状态机 ──▶ 观测(日志/指标/追踪) │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                            编排与优化层 (Orchestration)                          │
│  flow-ai: 拓扑/数据流/关键路径/冲突消解/调度 ──▶ optimizer: DAG 调度 & 资源约束   │
│  ai-agent: 工作流引擎 / 对话 / 浏览器自动化 / 插件总线 ──▶ mox-expert: 多专家│
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                             算子内核 (Core)                                     │
│  operator-core: 算子 trait / 高维向量 / 范畴论 / 单子 / 守恒律                  │
│  operator-graph: 加权有向图 / PageRank / 拉普拉斯矩阵                           │
│  operator-wasm: WASM 沙箱执行 / 热加载插件                                      │
│  hermes-flow-bridge: 与外部流系统(Hermes)对接的桥接/录制/回放                   │
└───────────────┬───────────────────────┬───────────────────────┬──────────────┘
                │                        │                       │
                ▼                        ▼                       ▼
┌──────────────────────────────────────────────────────────────────────────────┐
│                          数据 / 执行 / 外系统                                    │
│   业务目录(business-catalog)   外部算子(plugins/*.wasm)    第三方 API / DB / 消息 │
└──────────────────────────────────────────────────────────────────────────────┘
```

**端到端流程**：`用户请求 → 接入层 → 运行时鉴权/路由 → 编排层构建并执行算子 DAG → 内核执行算子(可调用 WASM 插件/外部系统) → 观测回流 → 结果经 WebSocket 实时推回前端`。

> 📈 **流程图可视化**：6 大企业级处理流程模板（发票核验/入职/采购/报销/合同/法务合规）、端到端时序图、SUPER_EXPERT 全维工作流、业务流程设计飞轮，均已绘制为可渲染的 Mermaid 图，见 [`docs/business-process-flowcharts.md`](docs/business-process-flowcharts.md)；企业级流程执行引擎与流程卡规范见 [`docs/business-process-flows.md`](docs/business-process-flows.md)。

---

## 🛠️ 技术栈

| 层 | 技术 |
|----|------|
| 后端核心 | Rust + Tokio + Axum |
| 线性代数 / 图算法 | nalgebra + petgraph |
| 插件沙箱 | WASM (wasmer) |
| AI 智能体 | LLM Client / 浏览器自动化 / 工作流引擎 |
| 前端 | Vue3 + Three.js + ECharts + 3D 力导向图 |
| 数学内核 | 范畴论 + 希尔伯特空间 + 单子论 |
| 脚本 / 验证 | Python 3 (公理自洽性验证) |

---

## 📁 项目结构（6层8域DDD矩阵 · 路径零老化 · 对齐 ADR-DOC-008 / GLOSSARY §5 · 三联盟责任标注）

> **架构模型 v3.0**：6层8域DDD矩阵 — **L0 Foundation**（横切基础：类型/错误/配置/路径管理/可观测）/ **L1 Gateway**（网关：路由+鉴权+限流，仅依赖 L0+L2）/ **L2 API**（8域域间 trait 契约，纯接口仅依赖 L0）/ **L3 Core**（8域领域模型）/ **L4 Svc**（8域应用服务）/ **L5 SDK**（8域对外 FFI 绑定）。8域 = ai / cloud / data / flow / kg / market / platform / voice。
> **旧→新迁移映射**：完整17旧crate→54新crate映射见 [`docs/enterprise/ARCHITECTURE-MIGRATION.md`](docs/enterprise/ARCHITECTURE-MIGRATION.md)。旧路径 `platform/services/`、`crates/` 已废弃。
> **架构-数据分离**：`platform/` 为纯代码只读边界，运行时数据/插件/第三方模型统一放在 `data/`/`plugins/`/`third_party/`，通过 `mox-platform-paths` 统一管理，详见 [`docs/standards/architecture-data-separation.md`](docs/standards/architecture-data-separation.md)。

```
infotopograph/  （= 璇玑 RelGraph 项目仓；根历史遗留 operator-unified-system 名仅作为底层父系统 OUS 代号，对外产品名统一为璇玑 RelGraph）
├── platform/                     # 🟢 平台服务层（后端核心 · Rust 73 crate workspace · 6层8域DDD矩阵 · 开发联盟主责）│   ├── foundation/               #   L0 横切基础层（通用类型/错误处理/配置/路径管理/可观测/云基础设施抽象）
│   │   ├── mox-platform-foundation/    # 平台基础库
│   │   ├── mox-cloud-foundation/       # 云基础设施基础库
│   │   ├── mox-platform-observability/ # 可观测性（日志/指标/追踪）
│   │   └── mox-platform-paths/         # 🆕 统一路径管理（架构-数据分离铁律）
│   ├── gateway/                  #   L1 网关层（模块化：路由/鉴权/限流分离，仅依赖 L0+L2 API）
│   │   └── mox-platform-gateway-svc/   # API网关 · operator-server 二进制入口
│   ├── arch-test/                #   🆕 架构测试（分层规则/跨域依赖/环检测/API纯度/架构-数据分离 8项不变量）
│   │   └── mox-arch-test/
│   ├── framework/                #   插件框架层（扩展点定义/插件注册 · mox-framework 库）
│   └── domains/                  #   8域 × 5层矩阵（core/svc/sdk/api/svcapi）
│       ├── ai/                   #   🟢 AI域（算法联盟R · 开发联盟C）
│       │   ├── api/              #     🆕 L2 域间契约：IntentRouter/CapabilityRegistry/ActivationDiffusion（纯trait）
│       │   ├── core/             #     L3 领域模型：mox-ai-core, mox-ai-intent-core
│       │   ├── svc/              #     L4 应用服务：mox-ai-agent-svc(对话/浏览器自动化/MultiAgent), mox-ai-expert-svc(⛨璇玑14专家/验证/审计/RBAC), mox-ai-flow-svc(流程AI/代码生成)
│       │   └── sdk/              #     L5 对外类型（规划中）
│       ├── kg/                   #   🟢 知识图谱域（算法联盟R · 最大域 · 9 crate）
│       │   ├── core/             #     L2：mox-kg-algo-core(八大算法A1~A8), mox-kg-meta-core(本体/Schema/14节点族×19边族)
│       │   ├── svc/              #     L3：mox-kg-storage-svc, mox-kg-service-svc, mox-kg-streams-svc, mox-kg-spark-svc, mox-kg-hub-svc(混合索引+URN+8段5连接器), mox-kg-fusion-svc(RRF融合/实体对齐)
│       │   └── sdk/              #     L4：mox-kg-sdk
│       ├── flow/                 #   🟢 流程/算子域（开发联盟R · 算法联盟C）
│       │   ├── core/             #     L2：mox-flow-operator-core(算子代数/守恒律/范畴论/单子), mox-flow-optimizer-core(CPM/RCPSP/CEM)
│       │   ├── svc/              #     L3：mox-flow-operator-wasm-svc(WASM沙箱), mox-flow-primiflow-svc(解析/代码生成/8类骨架), mox-flow-fusion-svc(六维融合/守恒闸门), mox-flow-bridge-svc(Hermes桥接)
│       │   └── sdk/              #     L4（规划中）
│       ├── data/                 #   🟡 数据域（开发联盟R）
│       │   ├── core/             #     L2：mox-data-formula-core(高精度公式), mox-data-norm-core(归一化IR), mox-data-standards-core(数据标准)
│       │   ├── svc/              #     L3：mox-data-plane-svc, mox-data-etl-svc, mox-data-compliance-svc(PII/脱敏), mox-data-catalog-svc(6预置FlowGraph)
│       │   └── sdk/              #     L4：mox-data-formula-native(napi), mox-data-norm-intent-native(napi)
│       ├── platform/             #   🟡 平台横切域（开发联盟R）
│       │   ├── core/             #     L2：mox-platform-system-core(成员/任务/权限/通信), mox-platform-iam-core(身份/令牌/访问控制), mox-platform-meta-core(AisLayer/CrateMeta/all_crate_metas), mox-platform-datastore-core(多后端SQLite/PG/MySQL), mox-platform-orchestrator-core(DAG编排/事件反应器)
│       │   ├── svc/              #     L3：mox-platform-enterprise-svc(企业服务/多后端), mox-platform-orchestrator-svc(编排器服务)
│       │   └── sdk/              #     L4：mox-platform-test-harness
│       ├── cloud/                #   ⚪ 云基础设施域（开发联盟R·运维）
│       │   ├── svc/              #     L3：mox-cloud-master-svc, mox-cloud-volume-svc, mox-cloud-s3-svc, mox-cloud-filer-svc
│       │   └── sdk/              #     L4：mox-cloud-sdk
│       ├── market/               #   ⚪ 模板市场域（产品联盟R · 开发联盟C）
│       │   └── svc/              #     L3：mox-market-template-svc(发布/加载/评分/Fork/2种子)
│       └── voice/                #   🔴 语音域（独立产品形态评估中 · 含桌面应用）
│           ├── core/             #     L2：mox-voice-dsp-core(响度归一/软限幅/Aho-Corasick/SIMD)
│           ├── svc/              #     L3：mox-voice-core-svc, mox-voice-asr-svc, mox-voice-intent-svc, mox-voice-operator-svc, mox-voice-desktop-app(全局热键/BallWidget/键鼠自动化)
│           └── sdk/              #     L4：mox-voice-dsp-py(PyO3)
├── config/                       # 🆕 📁 配置文件（Git追踪：gateway.yaml/paths.env.example，模板化）
├── data/                         # 🆕 💾 运行时数据（.gitignore：storage/cache/logs/uploads/exports）
├── plugins/                      # 🔌 第三方插件（.gitignore：wasm/scripts/extensions，按需加载）
├── third_party/                  # 📦 第三方源码/模型（.gitignore或submodule：CosyVoice/models）
├── .runtime/                     # 🆕 ⚡ 运行时状态（.gitignore：pid/sock/lock）
├── shared/                       # 跨端共享资源：常量、Schema、配置
├── frontend-ui/                  # 🟢 用户端：Vue3 + Three.js + ECharts 前端单应用（28 视图 + /admin 5 面板，产品联盟主责）
│   │                            #   管理区 5 面板：凭证 / 审计 / HITL（人机回环）/ 存储 / 总览
│   │                            #   🔴 旧 frontend-admin-ui 目录已裁撤（ADR-DOC-005 M0）；管理入口：/admin?tab=<tab>
├── tools/
│   ├── info-graph/               #     关图工具（含 P9 判重 dedup 子命令；对齐 BP-9 + enterprise/16 验收）
│   └── guantu_gate.py            #     P9 CI 门禁脚本（阻断未判重立项；对齐 All-02 铁规）
├── shared/                       # 跨端共享资源：常量、Schema、配置
├── docs/                         # 🔶 企业级文档（专题分区；治理中心 = docs/enterprise/00-INDEX.md · 三联盟必读首件 = enterprise/18 TOP-MASTER）
│   ├── enterprise/               # 🟢 企业级文档 00~28（共 29 份 · 分级权威 L0~L4）
│   │   ├── 18-全域顶层总设计-三联盟模式-V1.0.md  # 🟢🟢 L0 第一级权威（TOP-MASTER，三联盟签署）
│   │   ├── 28-全维架构分析与文档归一化报告-V1.0.md # 🟢 L1 治理枢纽（架构分析+优化+文档归一化 · 开发联盟R）
│   │   ├── ARCHITECTURE-MIGRATION.md             # 🟢 L2 迁移基准（旧17crate→新54crate完整映射 · 路径替换速查表）
│   │   ├── 00-INDEX.md                        # L1 治理索引 + RACI + 三联盟阅读路径
│   │   ├── 01-requirements.md  # 需求规格 SRS + §9 ADR-DOC-001~012 决策注册
│   │   ├── 02-architecture.md  # 七视图架构 + 六层金字塔锚点 + 八大算法家族 + **6层8域54+crate矩阵**（v2.0）
│   │   ├── 04-business-processing.md  # 10 大标准 BP-1~10（6 字段齐）
│   │   ├── 05-iteration-roadmap.md  # M0~M8 9 里程碑 + L0/L1/L2 三级验收
│   │   ├── 06-requirements-architecture-map.md  # 五向追溯 + §5 三联盟 RACI 矩阵
│   │   ├── 07-全维需求明确书.md  # 四闸门 + 双收口 + All-01~04 三联盟四条铁规
│   │   ├── 08-全维自动化处理明确书.md  # mox_optimize 8 步 + 每步主责联盟列
│   │   ├── 10-企业级交付清单.md  # 对外签署（三联盟 + 客户 + 审计五签）
│   │   ├── 14-愿景总纲.md        # 北极星方法论总纲
│   │   ├── 15-产品规范标准.md    # P1~P10 人人爱用十大原则
│   │   ├── 16-P9判重闸门验收.md  # P9 落地 + 关图治理 D1~D4 修复
│   │   ├── 22-全文档归一化总控卡.md  # 单源映射总控卡（⚠️ 基于旧架构，待v2.0重写）
│   │   └── 其他 03设计/09归档/11~13 验收报告/19~21/23~27
│   ├── README.md                 # 关图/全维专题快捷导航（三联盟差异化入口 · docs/README）
│   ├── GLOSSARY.md               # 🟢 唯一术语事实源（DOC-GLOSSARY-V1.1 · 7 新术语）
│   ├── specs/                    # 🟢 企业级规范：PT-STD / GR-STD / OUS 业务规划
│   ├── standards/                # 🆕 🟢 架构标准：architecture-data-separation.md（架构-数据分离铁律）
│   ├── full-dimensional/         # 🟡 全维分析专题：AA-STD / 关图骨架 / 治理台 API / 文档归档
│   ├── graph/                    # 关图机读产物（graph.json / graph.enterprise.json / guantu.req.json + requests/ 判重入口）
│   ├── ai-architecture/          # AI 架构专题（AUS · L4 Agentic 闭环）
│   ├── 璇玑-全维需求业务处理流程图-归一化企业级.md/.html/.mmd  # 🟢 AA-STD 融合域唯一事实基准
│   ├── architecture.md           # 🟡 OUS 父系统总架构（v7.0 · 归档参考，以02-architecture.md v2.0为准）
│   ├── enterprise-architecture-analysis.md  # 🟡 双璇玑十四维能力矩阵（归档参考）
│   ├── modules/mathematical-foundation.md  # 六大数学公理 & 算法联盟对账基准
│   └── *.html / *.mmd            # 🟡 可视化产物（以同名 .md 为源；同位存放）
├── benches/                      # 性能基准
├── tests/                        # 集成测试
├── scripts/
│   └── manage.py             # 统一运维脚本（服务管理 + Web 面板 + 公理验证，单文件整合）
├── start.sh                      # 一键启动脚本
└── README.md
```

> 注：`target/`（Rust 构建产物）、`frontend-ui/dist/`、`node_modules/` 等**不纳入版本库**，请从源码构建。
> **路径铁律（ADR-DOC-008 v2.0）**：所有 crate 均位于 `platform/domains/{域}/{层}/` 或 `platform/{foundation,gateway,framework}/`；旧路径 `platform/services/`（15 crate扁平模型）、`crates/`、`platform/gateway/runtime/` 已废弃，禁止在新代码/文档中使用。旧→新完整映射见 `docs/enterprise/ARCHITECTURE-MIGRATION.md`。
> **Node 边缘入口**：`platform/backend-node/`（零依赖 Node）作为边缘入口占 `:3000`，托管 `frontend-ui/dist` 并将 `/api` 反向代理到 Rust 网关 `:3001`；M0 后规划更名为 `edge-node/` 并瘦身至 4 文件。

---

## 🔒 架构-数据分离（铁律 · CI 强制执行）

> 完整规范见 [`docs/standards/architecture-data-separation.md`](docs/standards/architecture-data-separation.md)

### 三层分离模型

```
┌─────────────────────────────────────────────────────────┐
│  架构代码层  platform/          ← Git 追踪，只读边界    │
│  配置文件层  config/            ← Git 追踪，模板化      │
│  运行时数据层  data/plugins/    ← .gitignore，不入库    │
└─────────────────────────────────────────────────────────┘
```

### 四条铁律

1. **`platform/` 是纯代码只读边界**，禁止存放任何运行时数据（`.db`/`.sqlite`/`.log`/`.pid` 等）
2. **代码禁止硬编码相对路径**（如 `"./data/"`、`"./config/"`），必须通过 `mox-platform-paths` crate 管理
3. **所有路径可通过环境变量覆盖**：`MOX_DATA_DIR` / `MOX_PLUGINS_DIR` / `MOX_THIRD_PARTY_DIR` / `MOX_RUNTIME_DIR` / `MOX_CONFIG_DIR`
4. **第三方插件/模型/源码必须在 `platform/` 之外**

### 统一路径管理（`mox-platform-paths`）

```rust
use mox_platform_paths::ProjectRoot;

let root = ProjectRoot::detect();       // 自动定位项目根
root.verify_separation()?;               // 验证架构-数据不重叠
root.ensure_all_dirs()?;                 // 创建 data/plugins/.runtime 目录

let db_path = root.domain_db_path("kg"); // data/storage/kg.db
let log_dir = root.logs_dir();           // data/logs/
let plugin_dir = root.wasm_plugins_dir();// plugins/wasm/
```

### 架构测试（CI 必过）

```bash
# 8 项架构不变量测试（分层规则/跨域依赖/环检测/API纯度/架构-数据分离）
cargo test -p mox-arch-test
```

| 测试 | 验证内容 |
|---|---|
| `test_layering_rules` | L0~L5 分层依赖约束 |
| `test_cross_domain_dependencies_go_through_api` | 跨域依赖必须经 L2 API 层 |
| `test_no_circular_dependencies` | DFS 环检测 |
| `test_api_crates_are_pure` | L2 API crate 仅依赖 L0 |
| `test_architecture_data_separation` | `platform/` 下无数据文件 |
| `test_no_hardcoded_data_paths` | 代码无硬编码相对路径 |
| `test_plugins_outside_platform` | 插件文件不在 `platform/` 内 |
| `test_third_party_outside_platform` | 第三方目录不在 `platform/` 内 |

---

## 🚀 快速开始

### 1. 后端（Rust）

```bash
# 安装 Rust 工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 编译
cargo build --release

# 启动服务
./target/release/operator-server
```

### 2. 用户端前端（Vue3 / Node）

```bash
cd frontend-ui
npm install
npm run build      # 产物输出到 frontend-ui/dist/（已 gitignore）
# 或 npm run dev 本地开发
```

### 3. 企业级后台管理

系统管理区已并入用户端（frontend-ui），访问 **http://localhost:3020/#/admin**（管理总览 / 访问凭证 / 审计日志 / 存储与模块 / HITL 审批；大模型与知识库管理复用既有页面）。

### 4. 一键启动

```bash
./start.sh
```

启动后访问：
- 用户端：**http://localhost:3021**
- 系统管理区：**http://localhost:3021/#/admin**
- API 网关：**http://localhost:3000**

---

## 🧮 数学自洽性验证

运行 6 大公理验证（含范畴论定律、守恒律、图算法）：

```bash
python3 scripts/manage.py verify
```

验证内容：
- ✅ 公理 1：算子抽象与组合
- ✅ 公理 2：希尔伯特空间向量性质
- ✅ 公理 3：图论算法（邻接矩阵、拉普拉斯、PageRank）
- ✅ 公理 4：范畴论定律（单位律、结合律、函子性）
- ✅ 公理 5：资源约束与关键路径
- ✅ 公理 6：单子三定律
- ✅ 守恒律检查（概率守恒、能量守恒）

---

## 🔌 插件开发（WASM）

WASM 插件需导出算子执行函数：

```c
// input_ptr: 输入 f64 数组指针
// output_ptr: 输出 f64 数组指针
// n: 向量维度
// 返回值: 0 成功，非 0 错误
int operator_apply(double* input, double* output, int n);
```

将编译好的 `.wasm` 放入 `plugins/` 目录，系统自动加载。

---

## 📐 REST API

| 接口 | 方法 | 说明 |
|------|------|------|
| `/api/health` | GET | 健康检查 |
| `/api/operators` | GET | 获取算子列表 |
| `/api/execute` | POST | 执行算子工作流 |
| `/api/graph` | GET | 获取知识图谱数据 |
| `/api/graph/node` | POST | 添加知识图谱节点 |
| `/api/graph/edge` | POST | 添加知识图谱边 |
| `/api/plugins` | GET | 获取已加载插件列表 |
| `/api/logs` | GET | 获取执行日志 |
| `/api/status` | GET | 获取系统状态 |
| `/api/market/` | GET | 算子商城：算子包列表（支持 `?category`/`?tag`/`?q`） |
| `/api/market/random` | GET | 随机返回一个算子包 |
| `/api/market/:id` | GET | 获取算子包详情（需求 + 可编辑流程图 + 功能点） |
| `/api/market/upload` | POST | 上传算子包（`name`+`requirement` 必填） |
| `/api/market/:id` | POST | 更新算子包核心字段 |
| `/api/market/:id/clone` | POST | 克隆（fork）算子包 |
| `/api/market/:id` | DELETE | 删除算子包 |
| **🎯 /ai/engine/process** | POST | **自动意图识别 → 能力路由**（A5 激活扩散 d=0.85，30 轮收敛） |
| **🎯 /ai/engine/analyze** | POST | **显式能力执行**（capability_id → 直接执行） |
| **🎯 /ai/engine/capabilities** | GET | **能力矩阵自描述**（前端动态渲染能力面板用） |
| **🎯 /ai/engine/metrics** | GET | **三联盟 SLO 指标**（成功率/降级率/p95 延迟等） |

> 标 **🎯** 的 4 个端点 = 璇玑 RelGraph 统一 AI 编排入口（ADR-DOC-009），是 AI 查询 & AI 能力对外的推荐通道。
> 算子商城（需求/流程图资产市场）的完整数据模型、API 契约与前端编辑器说明见 [`docs/market-module.md`](docs/market-module.md)。

---

## 🏢 企业级特性

- **可观测性**：结构化日志、指标、链路追踪（experts/observability、runtime 观测层）
- **治理与权限**：多专家治理、权限与安全管理（experts/govern、permission、security）
- **资源约束**：基于 DAG 的资源管理与调度优化（optimizer、resource）
- **外部系统桥接**：与外部流处理系统对接、会话录制回放（hermes-flow-bridge）
- **插件沙箱隔离**：WASM 安全执行第三方算子
- **记忆一致性**：知识图谱 + 业务目录统一建模
- **算子商城（资产复用）**：将"需求 + 可编辑业务流程图 + 功能点"作为算子包沉淀，支持随机浏览、克隆后继续编辑，形成"需求驱动 → 流程可快速改"的知识复利闭环（见 `docs/market-module.md`）
- **多数据库后端（12-Factor 配置）**：璇玑系统可在 `SQLite / PostgreSQL / MySQL` 三种后端间**零代码切换**，默认 `SQLite`（开箱即用、零外部依赖）。方言差异（`INSERT OR REPLACE` / `ON CONFLICT DO UPDATE` / `ON DUPLICATE KEY UPDATE` 等）统一在 `repo/schema.rs` 按 `sea-query` 方言生成，业务层对后端无感知。
- **生产级 fail-fast**：`MOX_STRICT_PERSIST=1` 下，若连不上配置的数据库（连接失败 **或** 建表失败）则**启动时直接中止**，杜绝"连不上库却照常起服务、数据只写进内存、进程一重启就丢"的静默故障。默认关闭、保持与演示/测试的兼容。
- **企业级6层架构 + 零改动扩展**：L1基础/L2平台核心/L3领域服务/L4对接能力(AI/插件/政企/连接器)/L5统一集成/L6接入，所有扩展通过 **Trait + Factory + 配置** 实现，核心代码零改动。详见 [`ARCHITECTURE.md`](ARCHITECTURE.md) 和 [`docs/architecture/`](docs/architecture/README.md)。

### 数据库后端切换（璇玑系统 `mox-system`）

```powershell
# 默认：SQLite，零配置开箱即用
cargo run -p mox-system

# PostgreSQL 生产（连不上库直接中止启动，而非带病运行）
$env:MOX_PERSIST="true"; $env:MOX_STRICT_PERSIST="true"
$env:MOX_BACKEND="postgres"
$env:MOX_DB_URL="postgres://admin:***@db.internal:5432/mox"
cargo run -p mox-system
```

> 📖 **完整配置矩阵与语义（唯一权威基准）**：见 [`docs/enterprise/02-architecture.md` §7.4](docs/enterprise/02-architecture.md)。
> 环境变量全集、推荐组合、方言归一化实现、fail-fast 错误路径均以该节为准，本处仅作快速上手示例。

---

## 📝 许可证

本项目以 **MIT License** 开源，详见 [LICENSE](LICENSE)。
