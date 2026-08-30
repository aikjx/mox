# 开发专家联盟 · 全维整合总览（企业级）

> **文档性质**：唯一权威入口（Master Index）。把散落在 `docs/expert-alliance/`（系统设计 v1/v2/v3）、`docs/enterprise/26-*`（SaaS 化方案 V1.0/V1.1）、`proto/expert-alliance/v1`（契约）、`expert-alliance-cyber|design/`（可视化）的开发专家联盟内容，归一为一份可迭代、可追踪的分层全景。
> **版本**：V1.0（整合稿）| 日期：2026-08-29 | 状态：权威

---

## 0. 这是什么

**开发专家联盟（Expert Alliance）** 是构建在 infotopograph / Mox 微服务架构之上的**智能编排层**：把底层 31 个微服务的能力升级为 10+ 领域专家 Agent 自动协作的"智能团队"。

用户只需自然语言描述目标，系统自动完成 **专家识别 → 协作编排 → 多专家执行 → 结果融合 → 记忆沉淀**，端到端交付复杂任务成果。

一句话：让系统从"工具集合"升级为"智能团队"。

---

## 1. 文档资产全景（本主题全部素材）

| 资产 | 路径 | 内容 | 状态 |
|------|------|------|------|
| 设计总览 v1 | `docs/expert-alliance/README.md` | 系统定位/架构/专家模型/知识图谱/协作引擎/API/场景/路线 | 设计草案 |
| 设计 v2 | `docs/expert-alliance/v2/`（00~07 共 8 篇） | 全维需求/架构/领域模型/业务流程/接口/数据/安全/路线图 | 设计定稿 |
| 设计 v3 | `docs/expert-alliance/v3/`（3 篇） | 架构优化/需求矩阵/业务流程图（16 张 Mermaid） | 优化定稿 |
| 专项 | `docs/expert-alliance/expert-registry-and-protocol.md` | 专家注册中心 + 协作协议 | v1.0 |
| 专项 | `docs/expert-alliance/knowledge-graph-schema.md` | 知识图谱关联关系设计（六元网络） | v1.0 |
| SaaS 方案 V1.0 | `docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.0.md` | 源码取证诊断 + 四阶段路线图 | 首版 |
| SaaS 方案 V1.1 | `docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.1-补充修订版.md` | 修正 5 大模块低估 + 执行矩阵/风险/ROI/30 天里程碑 | 修订版 |
| 契约 | `proto/expert-alliance/v1/`（7 个 .proto） | 调度/执行/融合/注册/Agent/记忆/公共 | 对应 v3 |
| 可视化 | `expert-alliance-cyber/` · `expert-alliance-design/` | CYBERPUNK 版 / Element Plus 全维度设计方案 HTML | 展示稿 |

> 版本演进主线：**v1 草案 → v2 企业级全维 → v3 优化定稿 → SaaS 方案 V1.0 → V1.1 修订**。本索引统一口径。

---

## 2. 分层架构全景（v3 定稿 · 七层）

```
L7 应用层    前端工作台（ExpertCenterView 黄金比例三栏：阶段导航+AI工作区+图谱追踪）
L6 接入层    gateway-http(:8080 REST/JSON-RPC/MCP/WS) + gateway-grpc(:50051 内部)
L5 联盟核心  7 服务 + 1 Sidecar（见 §3）
L4 专家能力  10+ 领域专家 Agent（见 §4）
L3 微服务底座 31 个底层服务（ai/graph/flow/search/storage/compliance/fusion/...）
L2 数据层    PostgreSQL(RLS) · 知识图谱 · Redis(会话/缓存) · 事件总线
L1 基础设施  Docker Compose → K8s+Istio · OpenTelemetry · 灰度流量切换
```

---

## 3. 服务拆分演进（5 → 7 服务 + 1 Sidecar）

### 3.1 v2（5 服务）→ v3（7 服务 + 1 Sidecar）对照

| v2 服务 | v3 服务 | 优化点 |
|---------|---------|--------|
| mox-gateway-svc（多协议混单端口） | **gateway-http**(:8080) + **gateway-grpc**(:50051) | 协议分流，消除 ALPN 兼容问题 |
| mox-expert-alliance-svc（过重：调度/编排/执行/融合/记忆 5 大能力耦合） | **alliance-scheduler** + **alliance-executor** + **alliance-fusion** | 职责单一化，独立扩缩/故障隔离 |
| mox-expert-registry-svc | **expert-registry**（精简，匹配移走） | 匹配合并进 scheduler，RPC 由 2 次→1 次，延迟降 60% |
| mox-expert-agent-svc（有状态） | **expert-agent**（全无状态，会话外部化 Redis） | Pod 可随时重启，HPA 自由缩容 |
| mox-expert-kg-svc | 图谱查询内嵌 scheduler | 减少跳数（P3 修复） |
| — | **expert-memory**（统一记忆服务） | 记忆三层分散→统一抽象 |
| — | **ai-inference**（Python Sidecar，UDS 通信） | AI 推理隔离，与 agent 同 Pod |

### 3.2 关键数据流（v3）

```
gateway-http → alliance-scheduler（任务解析+专家匹配[内嵌图谱推理]+计划生成+案例检索）
             → alliance-executor（DAG 执行/节点调度/进度推送/人工干预）
             → expert-agent（ReAct 循环/工具调用/AI 推理）→ ai-inference(Sidecar)
             → alliance-fusion（6 种融合策略/质量评估/迭代精炼）
             → expert-memory（案例库/图谱学习/边权重更新）
```

---

## 4. 专家模型（领域实体）

### 4.1 专家定义（ExpertDefinition 核心字段）

```
expert_id · name · description · role · domains · capabilities
· tools(服务+方法绑定) · knowledge(图谱子图引用) · personality
· memory(config) · priority(冲突仲裁) · status · metadata
```

### 4.2 内置专家清单（10 个）

| 专家 | 核心能力 | 调用服务 |
|------|---------|---------|
| 图谱构建专家 | 本体设计/实体关系抽取/图谱构建/质量评估 | graph-svc, ai-svc, etl-svc |
| 数据分析专家 | 探查/统计/趋势/异常/可视化 | dataplane-svc, ai-svc |
| AI 推理专家 | 生成/摘要/翻译/分类/RAG/多模态 | ai-svc, search-svc |
| 安全审计专家 | 权限审计/脱敏/合规/漏洞/风险 | compliance-svc, auth-svc |
| 流程自动化专家 | 流程设计/编排/执行/优化 | flow-svc, operator-svc |
| 数据治理专家 | 标准/质量/血缘/元数据/目录 | catalog-svc, dataplane-svc |
| 知识融合专家 | 实体对齐/属性融合/冲突解决/补全 | fusion-svc, graph-svc |
| 搜索推荐专家 | 语义搜索/图谱检索/推荐/排序 | search-svc, graph-svc |
| 运维监控专家 | 指标/告警/故障/容量/调优 | o11y |
| 联盟协调专家 | 任务分解/调度/仲裁/评估 | alliance-svc（自引用） |

---

## 5. 知识图谱（六元关联网络）

**"专家-能力-领域-工具-数据-案例"六元关联图谱**驱动全部协作决策：

```
Expert ─has_capability→ Capability ─produces→ Data
  │                        │                      ▲
  │ operates_in            │ requires_tool        │ contains_data
  ▼                        ▼                      │
Domain ←──────────────── Tool ─operates_on───────┘
  ▲                                              ▲
  │ solved_by                                    │ used_capability
  └────────────── Case ←similar_to── Case ───────┘
  + collaborates_with(Expert→Expert) · depends_on(Capability→Capability)
```

**关联关系驱动**：专家识别（图谱 6 步查询）→ 协作编排（DAG 依赖推理）→ 结果融合（案例模式对比）→ 协作记忆（每次任务写回图谱更新边权重）。

---

## 6. 协作引擎

### 6.1 六种协作模式

| 模式 | 说明 | 适用 |
|------|------|------|
| 串行 Pipeline | A→B→C 数据流水线 | 抽取→清洗→融合→入库 |
| 并行 Fan-out/Fan-in | 多专家同时处理再融合 | 多视角分析 |
| 辩论 Debate | 多专家质询 | 风险评估/方案选择 |
| 分层 Hierarchical | 协调专家分解，子专家执行 | 复杂任务 |
| 迭代 Iterative | 生成→审核→不通过重做 | 高质量内容 |
| 动态 Dynamic | 按中间结果动态调度 | 研究/排查 |

### 6.2 结果融合策略

多数投票 · 加权投票 · 拼接合并 · 择优选择 · 辩论仲裁 · 迭代精炼。

### 6.3 执行引擎

DAG 调度器（拓扑排序/依赖检查/并行调度）→ 节点执行器（调用专家/工具/超时重试）→ 状态管理器（pending/running/success/failed/skipped）→ 事件总线（实时进度推送）→ 协作记忆。

---

## 7. 协议与契约（proto/expert-alliance/v1 · 7 文件）

| proto | 服务 | 职责 |
|-------|------|------|
| `common.proto` | — | 公共类型（15.5KB，最大） |
| `alliance_scheduler.proto` | ExpertAllianceSchedulerService | 任务调度/专家匹配/计划生成/案例检索 |
| `alliance_executor.proto` | ExpertAllianceExecutorService | DAG 执行/节点调度/进度/人工干预 |
| `alliance_fusion.proto` | ExpertAllianceFusionService | 6 种融合策略/质量评估 |
| `expert_registry.proto` | ExpertRegistryService | 专家 CRUD/3 项验证/健康心跳 |
| `expert_agent.proto` | ExpertAgentService | ReAct 5 步循环/工具调用（无状态，会话外部化） |
| `expert_memory.proto` | ExpertMemoryService | 统一记忆/案例库/图谱学习 |

> 多协议支持：gRPC（内部）+ JSON-RPC + MCP + REST + WebSocket（对外），经 gateway 转码。

---

## 8. SaaS 化诊断与方案（docs/enterprise/26-*）

### 8.1 已做对的 8 件事（架构红线，优化禁止破坏）

1. 以项目为根的 5 阶段 φ 生命周期模型（S1→S5 流水线）
2. 项目上下文单例 + 跨视图联动（projectContext.js）
3. Rust Workspace 分层模型（42 crates，教科书级）
4. 全维资源目录聚合（18 类资源）
5. 专家联盟标准化实体定义（15 种专家 + 8 预设 + 11 项目形态）
6. 快捷键与命令面板体系
7. 服务管理器 + 健康检查体系
8. 企业级文档体系（25+ 篇）

### 8.2 问题分级（P0-P4）

| 级别 | 问题 |
|------|------|
| 🔴 P0 | P0-1 零多租户隔离（无 tenant_id，admin123 硬编码）· P0-2 AI 零计量零成本归因 · P0-3 敏感信息暴露 |
| 🟠 P1 | 根目录 47 垃圾文件 · 87MB graph.json 违规入仓 · 运行时数据边界模糊 · 三语构建无统一入口 |
| 🟡 P2 | JSON Store→多租户 DB 迁移路径缺失 · AI 编排分层不透明 · 插件 Manifest 缺失 · 三目录命名可读性差 · P2-5 服务定义双源冲突 |
| 🟢 P3 | 项目选择器未按租户过滤 · 无全链路 Trace 可视化 · 专家智能匹配算法未见实现 |
| 🔵 P4 | 单文件图谱瓶颈 · Rust 42 crates 冷编译时间 |

### 8.3 V1.1 重大修订（5 大已实现模块，工作量 -40%）

| 模块 | V1.0 误判 | V1.1 真实状态 |
|------|----------|--------------|
| RBAC | 需从零写 | `rbac/policy.rs` 已完整（6 内置角色/通配符/继承链/自动审计） |
| AI 编排 | 需独立重做 | `ai-engine-core.js` 4 统一入口 + 5 步流水线 100% 符合硬约束 |
| 配额限流 | 需从零做 | `security.js` 4 档配额 + 双 TokenBucket 已完成 |
| 激活扩散 | 需从零写 | JS 钩子 detectIntentBySpread 已留，Rust 侧补算法 + napi 绑定即可 |
| 插件 | 需新造框架 | plugins.json 已有 3 内置插件，补 Manifest + 权限拦截即可 |

> 总工期：V1.0 16 周 → **V1.1 9-10 周**，压缩 40%。

### 8.4 四阶段路线图（总 48-49 人日）

| 阶段 | 周期 | 目标 |
|------|------|------|
| 一 · 工程卫生 | 0.5-1 周（7 任务） | 垃圾清理 / git 历史瘦身 / 三目录重命名 / 三语构建统一脚本 / 双源修复 / sccache |
| 二 · 多租户 DNA | 2-3 周（6 任务） | 四层身份模型（Tenant→Org→User→Project）/ 23 路由域 tenant_guard / 计量接入 / 激活扩散算法 |
| 三 · SaaS 能力 | 3-4 周（6 任务） | PostgreSQL RLS 迁移 / RBAC 绑定租户 / 插件 Manifest / OpenTelemetry 三语 / 可观测性页面 |
| 四 · 规模化 | 2-3 周（5 任务） | Docker Compose 8 服务 / SDK 三语 / Webhook / 计费 / 模板市场 MVP |

### 8.5 风险矩阵（16 项，8 项高风险 ≥1.2 需前置）

高风险：R2（git 历史改造冲突）/ R5（双写数据不一致）/ R6（重命名致 CI 红）/ R8（迁移写坏数据）/ R11（JWT secret）/ R12（计费定价）/ R13（模板生态冷启动）/ R16（健康检查假阳性）。

### 8.6 ROI（¥41 万投入 · 3 档）

| 档位 | 12 月 MRR 对应 | ROI | 回本 |
|------|---------------|-----|------|
| 保守 | 2,000 免费用户/3% 转化 | -14% | 第 15 个月 |
| 中性 | 5,000/5% | **+544%** | 第 4 个月 |
| 乐观 | 10,000/8% | +3195% | 第 2 个月 |

> 不做 SaaS 的机会成本：私有化一年最多 ¥70 万收入，比中性 SaaS 化（¥266 万）少赚 3.8×。

### 8.7 30 天快速里程碑（D1-D30）

D1-D2 清垃圾 → D3 双源修复 → D4-D5 脚本+CI → D6-D7 目录重命名 → D8 凭据环境变量化 → D9-D13 身份模型+tenant_guard → D14-D15 ProjectPicker → D16 迁移 → D17-D21 计量+激活扩散 → D22 全覆盖 → D23-D24 Demo 准备 → **D25 M1 种子客户 Demo 日** → D26-D30 修复+PG 起步+代码冻结。

---

## 9. 验收清单（四阶段 26 条 · 摘要）

- **阶段一**：`git status` 无未跟踪文件 · `git ls-files` 无 graph.json · 三目录重命名无残留 · setup-dev/check-all 全绿
- **阶段二**：A/B 租户数据隔离 · ProjectPicker 租户切换 · llm_usage.jsonl 10 条含 tenant_id · 无凭据环境变量禁止启动 · 23 路由域含"无 tenantId→401"用例
- **阶段三**：PG 6 表 RLS · 插件权限弹窗+403 · Grafana 全链路 Trace · 激活扩散 30 轮收敛 σ̄<0.06
- **阶段四**：docker compose 全流程 · SDK 5 行代码建项目 · 100 并发 P95<500ms · 灰度优雅切流

---

## 10. 下一步行动（3 件事 · 待用户"开工"指令）

| # | 动作 | 触发条件 | 产出 |
|---|------|---------|------|
| 1 | 执行阶段一 1-1+1-2（清垃圾 + graph.json 排查） | 用户回「开工」 | 干净 git status + 瘦身对比表 |
| 2 | 拉 2 小时技术评审过 24 项执行矩阵 | 用户回「安排评审」 | 评审纪要（任务调整+签字） |
| 3 | 在飞书/Notion 建 24 张任务卡 + 30 天里程碑看板 | 用户回「拆任务卡」 | 看板 URL |

> ⚠️ 阶段门禁原则：当前处于"设计阶段"，只输出方案与执行矩阵，**不擅自进入代码修改**。

---

## 11. 相关文档索引

- 专家联盟系统设计：`docs/expert-alliance/README.md` + `v2/` + `v3/`
- SaaS 化方案：`docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.0.md` / `-V1.1-补充修订版.md`
- 契约：`proto/expert-alliance/v1/*.proto`
- 可视化：`expert-alliance-cyber/` · `expert-alliance-design/`
- 平台全景：`docs/architecture/14-REPOSITORY-FULL-MAP.md` · `13-PLATFORM-CODEBASE-GUIDE.md`

---

**维护**：本索引随 v1/v2/v3/26 文档更新同步；新增专家联盟相关文档须在此登记。
**归一化状态**: ✅ 已归一化
