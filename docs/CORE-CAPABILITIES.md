# MOX 核心能力与解决的问题（CORE-CAPABILITIES）

> **编号**：CORE-CAPABILITIES-001 · **权威等级**：🟢权威 · **适用**：整个 infotopograph 仓库
> **一句话核心**：MOX 是「**知识图谱 + 多智能体 + 统一服务化**」三合一的企业级 AI 基础设施平台——用 Rust 原生实现，解决企业数据割裂、AI 服务化难、智能协作无编排标准三大问题。

---

## 一、我们解决什么问题（Problem Statement）

### P1 · 企业数据割裂：结构化数据与关系数据无法融合查询

企业的数据从来不是单一形态：交易明细在 SQL 表里，组织/知识/依赖关系天然是**图**。传统方案被迫二选一——要么用关系型数据库硬建模图（多表 JOIN 指数爆炸），要么引入图数据库（Neo4j 等）造成第二套存储、两套查询语言、数据双写。

**MOX 的答案**：自研知识图谱引擎（kg 域）与 SQL 体系同仓同构，提供**图谱 + SQL 融合查询**——同一入口既查关系数据，又做多跳图遍历、中心性分析、社区发现，不再需要第二套存储。

### P2 · AI 应用服务化难：库模式嵌入进程，无法多语言/微服务化

主流 AI 框架（LangChain/CrewAI 等）是**库模式**——SDK 嵌入应用进程。带来的问题：多语言客户端必须各自重写、服务无法独立扩缩容、无统一入口治理、灰度/限流/鉴权散落在每个应用里。

**MOX 的答案**：**服务化架构**——gRPC 契约层 + 8080 统一网关单入口，98 条路由、7 域统一治理；多语言客户端只依赖契约，不依赖具体实现；限流、鉴权、CORS、可观测在网关层一次完成。

### P3 · 智能协作无编排标准：多智能体各做各的

大模型 Agent 单打独斗有用，但企业级任务是**协作**：任务分解、专家调度、状态追踪、结果汇合。没有编排框架时，每个团队自造轮子，无法统一治理。

**MOX 的答案**：**专家联盟（alliance 域）**——scheduler（调度编排）/ executor（执行引擎）/ gateway（统一入口）三件套，11 条联盟路由，任务从下发到回收全程可追踪。

### P4 · 数据权限粗放：只有表级/接口级，没有字段级

企业数据安全要求**字段级**：同一张表，不同角色看到不同列。传统方案要么整表放行，要么为每个角色建视图，治理成本爆炸。

**MOX 的答案**：字段级权限内建于数据链路（system 域 45 条路由承载权限/安全面），配合 JWT 认证与 CORS 白名单，权限模型随查询下推。

### P5 · 可观测性缺失：生产事故无法定位

**MOX 的答案**：Prometheus 指标端点（`/metrics`，请求计数/延迟直方图/错误/活跃数/uptime）+ Actuator 管理面（health/mappings/logs/API 启停）+ Grafana 面板（`deploy/docs/trace-8stages-dashboard.json`），全链路可观测。

---

## 二、核心功能（Core Capabilities）

### C1 · 知识图谱引擎（kg 域）——自研图底座

| 组件 | 能力 |
|---|---|
| `mox-kg-algo-core`（图算法） | 加权有向图（petgraph 基座）：邻接矩阵、图拉普拉斯、中心性分析、社区发现、最短路径、PageRank（含分布式）、图嵌入、智能推荐 |
| `mox-kg-meta-core`（元数据） | Raft 高可用元数据服务：Schema 管理、权限鉴权、分区路由（状态机全自研，async-raft 仅作协议 driver） |
| 存储 | CSR 稀疏矩阵表示，支撑大图内存高效计算 |

### C2 · 图谱 + SQL 融合查询

KG 动态 SQL 架构（`docs/architecture/07-KG-DYNAMIC-SQL-ARCHITECTURE.md`）：结构化查询与图遍历同入口，kg 域 6 条路由（`/kg/v1/neighborhood` 等领域接口）直接服务于上层业务。

### C3 · 专家联盟多智能体编排（alliance 域）

- **scheduler-svc**（3100）：任务分解与调度编排
- **executor-svc**（3200）：执行引擎
- **gateway**（3300 桥接 / 33080 本地）：统一入口
- 11 条联盟路由（`/alliance/v1/tasks` 等），本地一键启动 `scripts/start-alliance-local.ps1`

### C4 · 统一服务化网关（8080 唯一入口）

- 98 条路由 · 7 域（kg 6 / ai 4 / alliance 11 / kb 16 / platform 6 / system 45 / actuator 10）
- 中间件分层：可观测 → CORS → 限流 → 鉴权
- gRPC 契约层（domains/*/proto）支撑多语言客户端

### C5 · 企业级安全与治理

- 字段级权限（system 域）、JWT 认证、CORS 白名单（禁 Any）
- 端口注册表（`docs/api/PORT-REGISTRY.md`）+ `verify-ports.py` CI 门禁
- 报告/文档归档规范（AGENTS.md）

### C6 · 可观测性

`/metrics`（Prometheus 文本）+ `/actuator/*`（health/info/mappings/env/loggers/logs/API 启停）+ Grafana 面板 + 全链路请求指标。

### C7 · AI 驱动交互（v3.0 产品形态）

对话中心单入口 + 四向弹层（Drawer / Top Sheet / Bottom Sheet / Modal），AI 理解意图 → 规划 → 调度 Agent → 执行 → 结果送界面（见 `docs/architecture/architecture.md`）。

---

## 三、问题 → 功能映射（速查）

| 问题 | 核心功能 | 落地位置 |
|---|---|---|
| P1 数据割裂 | C1 + C2 图谱引擎与融合查询 | `platform/domains/kg/`、`07-KG-DYNAMIC-SQL` |
| P2 AI 服务化难 | C4 统一网关 + gRPC 契约 | `platform/gateway/`、`domains/*/proto` |
| P3 协作无标准 | C3 专家联盟 | `platform/domains/alliance/` |
| P4 权限粗放 | C5 字段级权限 | system 域、`auth.rs` |
| P5 不可观测 | C6 指标 + 管理面 | `o11y.rs`、`actuator.rs` |

---

## 四、关键事实（数据基准 2026-09-06）

- Rust workspace **143 crates** · 6 层架构 · 12 业务域
- 网关 **98 条路由 / 7 域** · 唯一入口 **8080**
- kg 域 **2 个核心 crate**：图算法（algo）+ Raft 元数据（meta）
- 可观测性：`/metrics` + 11 档延迟直方图 + Actuator 管理面 + Grafana 面板
- 部署：Docker / K8s / Helm / Systemd，端口治理 CI 门禁全绿
