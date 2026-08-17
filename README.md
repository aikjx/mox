# 算子统一系统 (Operator Unified System)

> 基于 6 大数学公理构建的通用计算框架，实现「万物皆算子」的范畴论统一抽象。
> 一个面向企业级生产环境的 **算子编排 / 流程优化 / 知识图谱 / AI 智能体** 一体化平台。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/lang-Rust-orange.svg)](https://www.rust-lang.org/)
[![Vue3](https://img.shields.io/badge/frontend-Vue3%20%2B%20Three.js-green.svg)](https://vuejs.org/)

---

## 🌟 项目定位

算子统一系统（OUS）是一套 **企业级通用计算与编排底座**：

- 以范畴论（Category Theory）、希尔伯特空间（Hilbert Space）、单子（Monad）为数学内核；
- 将任意业务操作抽象为「算子（Operator / Morphism）」，通过有向加权图描述关联关系；
- 提供 DAG 调度、关键路径分析、资源约束优化的执行引擎；
- 支持 WASM 插件沙箱，安全扩展第三方算子；
- 内置 AI 智能体（LLM 编排、浏览器自动化、工作流推理）与璇玑（多专家协同求解）。

适用于：流程自动化、RPA、数据/算法编排、企业知识图谱、低代码算子平台等场景。

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
│  ai-agent: 工作流引擎 / 对话 / 浏览器自动化 / 插件总线 ──▶ xuanji-expert: 多专家│
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

## 📁 项目结构

```
operator-unified-system/
├── crates/
│   ├── operator-core/        # 核心内核：算子 trait、高维向量、范畴论、单子、守恒律
│   ├── operator-graph/       # 知识图谱：加权有向图、PageRank、拉普拉斯矩阵
│   ├── operator-wasm/        # WASM 插件系统：沙箱执行、热加载
│   ├── optimizer/            # 优化器：DAG 调度、关键路径分析、资源约束
│   ├── flow-ai/              # 流程 AI：拓扑/数据流/冲突消解/调度/代码生成
│   ├── ai-agent/             # AI 智能体：对话、浏览器自动化、工作流、插件总线
│   ├── xuanji-expert/      # 璇玑：多专家协同、IR、管线、治理、验证
│   ├── hermes-flow-bridge/   # 外部流系统桥接：对接、录制、回放、状态
│   ├── business-catalog/     # 业务算子目录
│   └── runtime/              # 运行时：Web 服务器与 API 接口（含算子商城模块 market.rs）
├── frontend/                 # Vue3 前端界面 (需 npm install && build 生成 dist/)
├── plugins/                  # WASM 插件目录
├── data/market/              # 算子商城资产（运行态，默认 CWD；生产应置于 $OUS_HOME/market，见 docs/architecture.md §27）
├── docs/                     # 企业级文档：architecture.md / enterprise-architecture-analysis.md / market-module.md / math-design.md / business-process-flows.md / business-process-flowcharts.md
├── benches/                  # 性能基准
├── tests/                    # 集成测试
├── verify_axioms.py          # 6 大公理数学自洽性验证脚本
├── start.sh                  # 一键启动脚本
├── snake.py                  # 辅助脚本
└── README.md
```

> 注：`target/`（Rust 构建产物）、`frontend/dist/`、`node_modules/` 等**不纳入版本库**，请从源码构建。

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

### 2. 前端（Vue3 / Node）

```bash
cd frontend
npm install
npm run build      # 产物输出到 frontend/dist/（已 gitignore）
# 或 npm run dev 本地开发
```

### 3. 一键启动

```bash
./start.sh
```

启动后访问：**http://localhost:3000**

---

## 🧮 数学自洽性验证

运行 6 大公理验证（含范畴论定律、守恒律、图算法）：

```bash
python3 verify_axioms.py
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

---

## 📝 许可证

本项目以 **MIT License** 开源，详见 [LICENSE](LICENSE)。
