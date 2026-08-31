---
title: Mox 专家联盟 — 智能自动化信息知识图谱关联关系系统
version: V2.0
authority: 🟡参考
doc_id: EA-DOC-018
last_updated: 2026-08-31
source_of_truth: 参考（导航页）
---

# Mox 专家联盟 — 智能自动化信息知识图谱关联关系系统

> **版本**：v2.0（企业级全维版）
> **日期**：2026-08-26
> **状态**：设计定稿
> **基于**：infotopograph 微服务架构 v3.0

---

## 一、系统定位

**专家联盟（Expert Alliance）** 是构建在 infotopograph 微服务架构之上的**智能编排层**，将 31 个底层微服务的工具能力，升级为 10+ 领域专家 Agent 自动协作的智能团队。

用户只需用自然语言描述目标，系统自动完成：**专家识别 → 协作编排 → 多专家执行 → 结果融合 → 记忆沉淀**，端到端交付复杂任务成果。

### 核心差异化

| 维度 | 传统工具平台 | Mox 专家联盟 |
|------|-------------|-------------|
| 交互方式 | 手动调用每个工具 | 自然语言描述目标，自动协作 |
| 能力组织 | 工具集合（扁平） | 专家 Agent（领域化+人格化） |
| 跨域协作 | 用户手动编排 | 自动 DAG 编排（6种协作模式） |
| 知识驱动 | 无 | 六元关联图谱驱动匹配/编排/融合/学习 |
| 持续学习 | 无 | 每次任务更新图谱边权重，案例自动积累 |
| 协议支持 | REST | gRPC + JSON-RPC + MCP + REST + WebSocket |

---

## 二、文档导航

| # | 文档 | 核心内容 |
|---|------|----------|
| 00 | [全维需求分析](docs/expert-alliance/v2/00-requirements.md) | 业务场景/功能需求/非功能需求/用户故事/约束假设/验收标准 |
| 01 | [企业级架构设计](docs/expert-alliance/v2/01-architecture.md) | 七层架构/5个新服务拆分/多协议网关/部署架构/与现有服务集成 |
| 02 | [归一化领域模型](docs/expert-alliance/v2/02-domain-model.md) | 统一术语/9个核心实体/状态机/数据契约/领域事件 |
| 03 | [全路径业务流程](docs/expert-alliance/v2/03-business-flow.md) | 端到端主流程/4个核心子流程/异常处理/MCP调用/时序图 |
| 04 | [归一化接口设计](docs/expert-alliance/v2/04-api-design.md) | Proto契约/多协议映射/JSON-RPC规范/MCP规范/REST规范/转码机制/WebSocket |
| 05 | [数据架构](docs/expert-alliance/v2/05-data-architecture.md) | 存储选型/PostgreSQL模型/知识图谱模型/Redis模型/数据一致性/迁移备份 |
| 06 | [安全与可观测性](docs/expert-alliance/v2/06-security-observability.md) | 安全四件套/多租户/审计/加密/三大支柱/仪表盘/告警/弹性七件套/SLA |
| 07 | [实施路线图](docs/expert-alliance/v2/07-roadmap.md) | 4阶段16周/里程碑/验收标准/团队配置/风险应对/持续演进 |

---

## 三、架构总览

### 3.1 七层架构

```
L7 接入层     多协议单端口网关（gRPC/JSON-RPC/MCP/REST/WebSocket）
                │
L6 专家联盟层   5个新服务：alliance / registry / agent / kg + ai-inference-sidecar
                │
L5 业务服务层   现有31个微服务（AI/图谱/数据/流程/治理/平台能力）
                │
L4 共享库层     mox-rpc/config/o11y/db/tenant/auth/resilience/mcp/expert-core
                │
L3 数据层       PostgreSQL / 自研图存储(RocksDB+Raft) / Redis / NATS / MinIO / pgvector
                │
L2 容器编排层   Kubernetes / Deployment / StatefulSet / HPA / PDB / Istio(可选)
                │
L1 基础设施层   计算/存储/网络/负载均衡/DNS/TLS/监控/日志/CI-CD
```

### 3.2 新增服务清单

| 服务 | 职责 | 部署 | 扩缩依据 |
|------|------|------|----------|
| **mox-gateway-svc** | 多协议接入/认证/限流/路由/协议转码 | Deployment(3) | QPS |
| **mox-expert-alliance-svc** | 任务管理/协作编排/DAG执行/结果融合/协作记忆 | Deployment(3) | 任务队列长度 |
| **mox-expert-registry-svc** | 专家CRUD/匹配/健康检查/工具自动发现 | Deployment(2) | QPS |
| **mox-expert-agent-svc** | Agent运行时/ReAct循环/工具调用/AI推理 | Deployment(3+) | 并发Agent数 |
| **mox-expert-kg-svc** | 专家联盟知识图谱/关联推理/案例库/持续学习 | Deployment(3) | 图谱查询QPS |
| **ai-inference-sidecar** | Python AI推理（与agent-svc同Pod） | Sidecar | - |

### 3.3 10个内置专家

| 专家 | 领域 | 核心能力 |
|------|------|----------|
| 图谱构建专家 | 知识图谱 | 本体设计/实体抽取/关系抽取/图谱构建/质量评估 |
| 数据分析专家 | 数据分析 | 数据探查/统计分析/趋势预测/异常检测 |
| AI推理专家 | 人工智能 | 文本生成/摘要/翻译/RAG检索/多模态理解 |
| 安全审计专家 | 安全合规 | 权限审计/数据脱敏/合规检查/风险评估 |
| 流程自动化专家 | 工作流 | 流程设计/任务编排/自动化执行/异常处理 |
| 数据治理专家 | 数据治理 | 数据标准/质量规则/元数据/血缘分析 |
| 知识融合专家 | 知识融合 | 实体对齐/属性融合/冲突解决/知识补全 |
| 搜索推荐专家 | 搜索推荐 | 语义搜索/图谱检索/个性化推荐 |
| 运维监控专家 | 运维 | 指标监控/告警分析/故障定位/容量规划 |
| 联盟协调专家 | 协调 | 任务分解/专家调度/冲突仲裁/结果评估 |

---

## 四、核心机制

### 4.1 知识图谱驱动

六元关联网络（7种顶点 + 12种边）驱动整个系统：

```
Expert ──has_capability──→ Capability ──requires_tool──→ Tool
  │                            │                            │
  │ operates_in                │ depends_on                 │ operates_on
  ▼                            ▼                            ▼
Domain ──contains_data──→ Data                      (底层gRPC方法)

Case ──solved_by──→ Expert
Case ──used_capability──→ Capability
Case ──similar_to──→ Case
Expert ──collaborates_with──→ Expert
```

**关联关系的作用**：
- 专家匹配：`operates_in` + `has_capability` + `requires_tool` → 找到合适专家
- 协作编排：`collaborates_with` + `depends_on` → 推荐最佳专家组合
- 结果融合：`solved_by` + `used_capability` → 评估专家贡献度设置权重
- 持续学习：每次任务更新边权重（频率/成功率/效果）

### 4.2 六种协作模式

| 模式 | 说明 | 适用场景 |
|------|------|----------|
| **串行 Serial** | A→B→C 流水线 | 数据处理：抽取→清洗→融合→入库 |
| **并行 Parallel** | 多专家同时执行→汇总 | 多视角分析：各领域独立分析后融合 |
| **辩论 Debate** | 多专家观点→互相质询→仲裁 | 决策类：风险评估/方案选择 |
| **分层 Hierarchical** | 协调专家分解→子专家执行→汇总 | 复杂任务：联盟协调专家主导 |
| **迭代 Iterative** | 生成→审核→不通过重做 | 高质量内容：生成+审核循环 |
| **动态 Dynamic** | 根据中间结果动态决定下一步 | 探索性任务：研究/问题排查 |

### 4.3 六种融合策略

多数投票 / 加权投票 / 拼接合并 / 择优选择 / 辩论仲裁 / 迭代精炼

### 4.4 多协议共存

单端口同时支持 5 种协议，通过 Content-Type + Path 路由：

| 协议 | Path/标识 | 用途 |
|------|-----------|------|
| gRPC | `Content-Type: application/grpc` | 内部服务间高性能通信 |
| JSON-RPC 2.0 | `/rpc` | 对外灵活API/浏览器 |
| MCP | `/mcp` | AI模型工具调用标准（Claude/Cursor） |
| REST | `/api/v1/*` | 兼容现有前端 |
| WebSocket | `/ws` | 实时进度推送/流式输出 |

**JSON-RPC→gRPC 自动转码**：基于 .proto 反射生成路由表，自动完成 JSON↔Protobuf 转换，零手写代码。

---

## 五、端到端流程

```
用户自然语言描述目标
    │
    ▼
网关（认证/限流/租户解析/协议转码）
    │
    ▼
联盟核心
    ├── 1. 任务解析（NLP提取领域/能力/数据需求）
    ├── 2. 专家匹配（图谱推理+综合评分→Top N专家）
    ├── 3. 案例检索（相似历史案例→协作模式建议）
    ├── 4. 协作计划生成（DAG：节点=专家调用，边=数据依赖）
    │
    ├── 5. DAG执行（拓扑调度+并行执行+依赖管理）
    │     └── 每个节点：Agent运行时（ReAct循环）
    │           ├── 理解→规划→执行（工具调用/AI推理）→观察→审核
    │           └── 流式输出实时推送
    │
    ├── 6. 结果融合（6种策略）
    ├── 7. 质量评估
    │
    ├── 8. 协作记忆更新
    │     ├── 工作记忆归档
    │     ├── 会话记忆更新
    │     ├── 评分≥4→提升为案例（写入图谱）
    │     └── 图谱边权重更新（频率/成功率/效果）
    │
    └── 9. 结果交付（JSON/导出/通知）
```

---

## 六、企业级保障

| 维度 | 标准 |
|------|------|
| **可用性** | 99.95% SLA，RTO<15min，RPO<1min |
| **性能** | 任务创建P99<500ms，专家匹配P99<200ms，节点执行P99<2s（不含AI） |
| **并发** | ≥100并发任务 |
| **安全** | JWT+OIDC+MFA，RBAC+ABAC，mTLS，字段加密，等保三级 |
| **多租户** | L1逻辑隔离（默认）→ L2 Schema → L3集群，三档可升级 |
| **可观测** | 结构化日志+Prometheus指标+OTel全链路追踪，6个Grafana仪表盘，P0-P3告警 |
| **弹性** | 限流+熔断+降级+重试+超时+舱壁+死信队列，专家故障自动切换 |
| **数据** | PostgreSQL WAL+图存储CDC+MinIO跨区域复制，11个9持久性 |
| **审计** | 不可篡改审计日志（哈希链+WORM存储），全操作记录 |

---

## 七、实施路线

**4阶段16周**（在微服务架构W20完成后启动）：

| 阶段 | 时间 | 核心交付 |
|------|------|----------|
| 一、基础建设 | W21-24 | 共享库+Proto契约+数据库迁移+CI/CD+开发环境 |
| 二、核心能力 | W25-28 | 注册中心+Agent运行时+知识图谱服务+10个内置专家 |
| 三、联盟核心 | W29-32 | 任务管理+协作编排+DAG执行+结果融合+协作记忆+人工干预 |
| 四、企业级交付 | W33-36 | 多协议网关+安全加固+可观测性+5个场景验证+性能压测 |

**团队**：8-10人（架构师1 + Rust后端3-4 + AI工程师1 + 运维1 + 测试1 + 产品1）

详见 [07-实施路线图](docs/expert-alliance/v2/07-roadmap.md)

---

## 八、与现有架构的关系

专家联盟**不重复开发**底层能力，全部通过 gRPC 复用现有 31 个微服务：

- AI推理 → `mox-ai-svc` + Python sidecar
- 图谱操作 → `mox-graph-svc` + `mox-graph-storage-svc`
- 图算法 → `mox-graph-algo-svc`
- 搜索 → `mox-search-svc`
- 工作流 → `mox-flow-svc`
- 数据处理 → `mox-etl-svc`
- 对象存储 → `mox-storage-svc`
- 安全审计 → `mox-compliance-svc`
- 数据融合 → `mox-fusion-svc`
- 认证/租户/计量/通知 → 对应平台服务

图存储引擎**零修改**，仅通过 gRPC 薄包装层调用，多租户用 VID 前缀方案隔离。

---

## 九、快速开始

### 开发者接入（REST）

```bash
# 创建任务
curl -X POST http://localhost:8080/api/v1/expert/tasks \
  -H "Authorization: Bearer <jwt>" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "图谱构建",
    "description": "把data.csv构建成知识图谱，做质量评估和安全审计",
    "preference": {"mode": "AUTO"}
  }'

# 响应: {"task_id": "task-xxx", "status": "planning"}

# 查询任务
curl http://localhost:8080/api/v1/expert/tasks/task-xxx \
  -H "Authorization: Bearer <jwt>"

# WebSocket实时进度
ws://localhost:8080/ws/v1/expert/tasks/task-xxx/progress?token=<jwt>
```

### MCP 接入（Claude Desktop）

```json
// ~/.claude.json
{
  "mcpServers": {
    "mox-expert": {
      "url": "http://localhost:8080/mcp",
      "headers": { "Authorization": "Bearer <jwt>" }
    }
  }
}
```

配置后 Claude Desktop 自动发现专家联盟的所有工具（图谱构建/数据分析/AI推理等），可直接在对话中调用。

---

## 十、版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v1.0 | 2026-08-26 | 初始设计：3篇文档（总览/注册协议/图谱Schema） |
| v2.0 | 2026-08-26 | 企业级全维版：8篇文档，全维需求/归一化模型/全路径流程/多协议/数据/安全/路线图 |

---

*文档导航：[00-需求分析](docs/expert-alliance/v2/00-requirements.md) | [01-架构设计](docs/expert-alliance/v2/01-architecture.md) | [02-领域模型](docs/expert-alliance/v2/02-domain-model.md) | [03-业务流程](docs/expert-alliance/v2/03-business-flow.md) | [04-接口设计](docs/expert-alliance/v2/04-api-design.md) | [05-数据架构](docs/expert-alliance/v2/05-data-architecture.md) | [06-安全可观测](docs/expert-alliance/v2/06-security-observability.md) | [07-路线图](docs/expert-alliance/v2/07-roadmap.md)*
