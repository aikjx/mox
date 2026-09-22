# 项目注册表 V2 — 一切皆是项目 · 全维跟进

> 标准编号：PR-STD-V2.0 | 生效日期：2026-08-26
> 治理原则：一切皆是项目 — 所有对话、需求、开发、验证均归属具体项目跟进，无游离资产

---

## 0 · 核心范式：一切皆是项目

| 原则 | 含义 | 强制执行点 |
|------|------|-----------|
| P-01 项目归属唯一 | 每一次对话、需求、代码、文档必须归属且仅归属一个项目 | 对话创建时强制选择项目；无项目对话自动归入「AI对话 - 默认对话」 |
| P-02 项目即治理单元 | 项目 = 聚合业务域/引擎/算法/数据/文档/流程的顶层治理容器 | W10 机器验证：孤儿域阻断、重复归属阻断 |
| P-03 全维跟进 | 每个项目按「需求→架构→业务→算法→开发→验证→交付」七维全链路跟进 | 项目全景页展示七维进度 |
| P-04 默认兜底 | 未显式创建项目的对话/需求自动归入默认项目，后续可批量迁移 | 迁移操作触发 W10 复验 |

---

## 1 · 项目总览（9 项目 + 1 默认兜底）

| # | 项目 ID | 项目名称 | 状态 | 核心定位 |
|---|---------|---------|------|---------|
| 0 | proj-default-dialogue | AI对话 - 默认对话 | active | 所有未创建项目的对话兜底容器；支持批量迁移至正式项目 |
| 1 | proj-mox-core | 璇玑核心平台 | maintaining | OUS 算子统一系统核心底座：运行时/网关/安全/模块管理 |
| 2 | proj-knowledge | 知识图谱与知识库 | maintaining | 关图引擎 + 知识库 + 图谱算法 + 语义搜索 |
| 3 | proj-ai-dialogue | AI 对话协作 | delivered | 多引擎对话编排 + 浏览器自动化 + 工作流推理 |
| 4 | proj-dev-expert-alliance | 开发专家联盟 | maintaining | 全维分析优化 · 企业级 · 算法验证最优需求业务处理流程图 |
| 5 | proj-ai-engine | AI 引擎编排 | maintaining | LLM Provider 路由 + 多模型编排 + A5 激活扩散意图识别 |
| 6 | proj-ai-platform | AI 平台生态 | delivered | 算子商城 + 模板市场 + 任务调度 + 浏览器市场 |
| 7 | proj-auto-dev | 自动开发引擎 | building | 代码生成 + 优化器 + 旋律简谱 + 制品管理 |
| 8 | proj-graph-infra | 图谱基础设施 | building | 项目全息图谱 + 引擎宇宙 + 引擎内核 + 自同步 |

---

## 2 · 项目 0：AI对话 - 默认对话

| 字段 | 值 |
|------|-----|
| 项目 ID | proj-default-dialogue |
| 项目名称 | AI对话 - 默认对话 |
| 状态 | active（永久活跃，不可归档） |
| 定位 | 所有未显式创建项目的 AI 对话的兜底容器。用户发起对话时若未指定项目，系统自动归入本项目。支持将对话批量迁移至正式项目。 |
| 生命周期 | 特殊：不可归档、不可删除；仅可将其中对话迁移出去 |
| 健康度量 | 对话数 / 已迁移率 / 平均停留时长 |

迁移规则：单条对话迁移指定 target_project_id；批量迁移用 ids[] + target_project_id；迁移后原对话保留引用指针，W10 复验通过。

---

## 3 · 项目 4：开发专家联盟（全维优化 · 企业级 · 算法验证）

| 字段 | 值 |
|------|-----|
| 项目 ID | proj-dev-expert-alliance |
| 项目名称 | 开发专家联盟 |
| 曾用名 | proj-expert-alliance（专家联盟） |
| 状态 | maintaining |
| 核心定位 | 全维分析优化 · 企业级 · 通过算法验证最优需求业务处理流程图 |
| 版本 | v3.0（全维优化版，2026-08-26） |

### 三大核心标签

| 标签 | 内涵 | 验证基准 |
|------|------|---------|
| 全维分析优化 | 双璇玑十四维（业务七维 + 开发七维）并行诊断；v3 架构 7 服务拆分；16 张 Mermaid 全维流程图 | expert-alliance/v3/03-business-flow-diagrams.md |
| 企业级 | 多租户三档隔离；RBAC 四权分离；审计哈希链；SLA + 成本预算治理闸门；K8s + HPA | expert-alliance/v3/01-architecture-optimization.md |
| 算法验证最优 | flow-ai 已验证算法栈：冒险分析并行化 + CPM + RCPSP + 冲突修复 + Dijkstra；璇玑验证网关 5 项阻断级检查 | 璇玑-全维需求业务处理流程图-归一化企业级.md |

### 归属域

| 域 ID | 域名称 | 核心能力 |
|-------|--------|---------|
| domain-expert-alliance | 专家联盟核心 | 7 服务：scheduler/executor/fusion/registry/agent/memory/gateway |
| domain-expert-graph | 专家知识图谱 | 7 顶点 + 12 边关联网络；图谱驱动匹配/编排/融合/学习 |
| domain-mox-expert | 璇玑验证引擎 | 双璇玑十四维治理；归一化 IR；裁决/验证/审计三汇；最高权限否决 |
| domain-flow-ai | 全维流程优化 | CPM + RCPSP + 冲突修复 + 代码生成；已验证最优求解 |

### 需求业务处理流程图（算法验证最优）

8 阶段 · 4 道强制闸门 · 全链路算法验证：

S1 需求接入 → S2 归一化 → S3 双璇玑并行诊断 → S4 归一化裁决
→ S5 flow-ai 最优求解 → S6 璇玑验证网关 → S7 治理闸门 → S8 出码/出图

4 道闸门不可降级旁路：
- G0 归一化闸门：IR 可拓扑排序 + 维度着色完整 + 孤儿/悬空边 = 0
- G1 裁决闸门：硬约束（Blocking）优先落地；冲突平手升级为 Risk(Blocking)
- G2 璇玑否决：任一阻断级检查失败 → vetoed=true → 强制 Blocked（不可覆盖）
- G3 治理闸门：approved = !algorithm_veto ∧ can_emit() ∧ blocking==0 ∧ sla_ok ∧ budget_ok

### v3 架构优化要点

| 优化项 | v2 | v3 | 收益 |
|--------|----|----|------|
| 服务拆分 | 5 服务 | 7 服务 | 独立扩缩/故障隔离 |
| 专家匹配 | 2 次 RPC | 1 次 RPC | 匹配延迟降 60% |
| 记忆管理 | 分散 3 处 | 统一 expert-memory | 一致性提升 |
| 有状态服务 | agent-svc 有状态 | 全无状态 | Pod 可随时重启/HPA 自由缩容 |
| 协议端口 | 单端口混合 | 双端口分流 | 内部 gRPC 性能最优 |
| 结果融合 | 同步阻塞 | 独立 fusion 异步 | 不阻塞主链路 |
| 并发任务 | 预估 100 | 预估 200 | 提升 100% |

### 16 张全维业务流程图

系统总体架构图 / 端到端主流程 / 专家匹配流程 / 协作计划生成 / DAG执行引擎 / Agent ReAct循环 / 结果融合 / 协作记忆与图谱学习 / 异常处理 / 人工干预 / MCP调用 / 多协议网关路由 / 服务间调用时序图 / 知识图谱关联关系图 / 部署架构图 / 状态机总图

---

## 4 · 其余项目简述

### 项目 1：璇玑核心平台（proj-mox-core）
OUS 算子统一系统核心底座：Rust 运行时 / Axum 网关 / 安全鉴权 / 模块管理 / 多数据库后端（SQLite/PG/MySQL 零代码切换）。归属域：system/services/security/modules-admin/mod-storage。

### 项目 2：知识图谱与知识库（proj-knowledge）
关图引擎（加权有向图 + PageRank + 拉普拉斯矩阵）+ 知识库管理 + 八大图算法家族 + 统一语义搜索。归属域：graph/mod-graph/kb。

### 项目 3：AI 对话协作（proj-ai-dialogue）
多引擎对话编排（LLM Client + 浏览器自动化 + 工作流推理）+ 对话自动知识图谱整理 + 小白语音 ASR/TTS。归属域：chat/web-search/orchestration。与默认对话的关系：本项目为正式 AI 对话功能开发项目；用户未指定项目的实际对话内容归入 proj-default-dialogue。

### 项目 5：AI 引擎编排（proj-ai-engine）
LLM Provider 统一路由 + 多模型编排 + A5 激活扩散意图识别（d=0.85，30 轮收敛）+ A7 CEM 交叉熵持续优化。归属域：ai-engine/ai-integrated/ai-ultimate/ai-enhanced/integration。

### 项目 6：AI 平台生态（proj-ai-platform）
算子商城（需求+流程图+功能点资产复用）+ 模板市场 + 任务调度 + 浏览器市场 + 自动任务。归属域：ai-platform/browser-market/tasks/auto-tasks/mod-task。

### 项目 7：自动开发引擎（proj-auto-dev）
代码生成（PrimiFlow 六维融合 + 8 类骨架模板）+ 优化器 + 旋律自动简谱/五线谱 + 制品管理。归属域：auto-dev/artifacts/optimizer/mod-melody2score。

### 项目 8：图谱基础设施（proj-graph-infra）
项目全息图谱（Project Atlas）+ 引擎宇宙注册表 + 引擎内核 + Rust crate 自同步 + W1-W10 无破窗验证。归属域：atlas/engine-universe/engine-kernel。

---

## 5 · 项目生命周期状态机

planning → building → delivered → maintaining → archived

| 状态 | 含义 | 可流转至 |
|------|------|---------|
| planning | 需求规划中 | building |
| building | 活跃开发中 | delivered / archived |
| delivered | 已交付上线 | maintaining |
| maintaining | 持续维护迭代 | archived |
| archived | 已归档，只读 | —（不可逆） |

特殊项目：proj-default-dialogue 状态恒为 active，不参与状态机流转。

---

## 6 · 治理不变式（P1-P7，W10 机器验证）

| 编号 | 不变式 | 验证方式 |
|------|--------|---------|
| P1 | 项目身份：id 格式（proj- 前缀）/ name 必填 / 全局唯一 | 注册表扫描 |
| P2 | 域归属唯一：每个域恰好归属一个项目 | owns_domain 边计数 |
| P3 | 引用真实：项目声明的域必须存在于图谱 | 交叉校验 |
| P4 | 状态合法：status 属于状态机合法集（含 default 项目 active 特例） | 枚举校验 |
| P5 | 流转合法：生命周期只能沿状态机合法边正向流转 | 流转历史审计 |
| P6 | 项目内聚：每个正式项目 ≥2 个域（default 项目豁免） | 域计数 |
| P7（新增） | 对话归属：每条对话必须归属一个项目；无显式归属自动归入 proj-default-dialogue | 对话创建时强制写入 project_id |

---

## 7 · 健康度量模型

每个项目健康分 = 资产覆盖 60 分 + 验证全绿 40 分。

| 维度 | 分值 | 计算方式 |
|------|------|---------|
| 域覆盖 | 15 | 归属域数 / 预期域数 |
| 引擎覆盖 | 10 | 域关联引擎数 / 预期引擎数 |
| 数据覆盖 | 10 | 数据资产关联率 |
| 文档覆盖 | 10 | 文档关联率 |
| 流程覆盖 | 15 | 业务流程图谱化率 |
| W1-W10 验证 | 40 | 通过门禁数 / 10 |

---

## 8 · 变更日志

| 日期 | 版本 | 变更内容 |
|------|------|---------|
| 2026-08-22 | v1.0 | 初始 8 基线项目，落地"一切皆是项目"范式 |
| 2026-08-26 | v2.0 | 新增 proj-default-dialogue（AI对话 - 默认对话）；proj-expert-alliance 更名为 proj-dev-expert-alliance（开发专家联盟），定位升级为全维分析优化·企业级·算法验证最优需求业务处理流程图；新增 P7 对话归属不变式；全项目描述优化 |
