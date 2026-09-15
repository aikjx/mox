# 01 - 服务边界优化

> 版本：v1.0 | 日期：2026-08-26 | 状态：草案
>
> 前置阅读：[00-核心原则](./00-principles.md)

## 一、现状诊断

### 1.1 当前模块清单（36 个服务模块）

当前 `platform/domains/` 下有 36 个模块，命名不统一，职责存在重叠：

| 序号 | 模块名 | 前缀 | 职责描述 | 问题 |
|------|--------|------|----------|------|
| 1 | operator-core | 无 | 算子核心 | 命名缺前缀 |
| 2 | operator-wasm | 无 | WASM 算子 | 命名缺前缀，是库不是服务 |
| 3 | graph-algorithms | 无 | 图算法 | 命名缺前缀 |
| 4 | optimizer | 无 | 优化器 | 命名缺前缀，职责模糊 |
| 5 | flow-ai | 无 | AI 流程 | 命名缺前缀，与 mox-ai-core 重叠 |
| 6 | hermes-flow-bridge | 无 | 流程桥接 | 命名缺前缀，与 primiflow-fusion 重叠 |
| 7 | ai-agent | 无 | AI Agent | 命名缺前缀 |
| 8 | template-market | 无 | 模板市场 | 命名缺前缀 |
| 9 | business-catalog | 无 | 业务目录 | 命名缺前缀 |
| 10 | kg-hub | 无 | 知识图谱中心 | 命名缺前缀，与 mox-graph-service 重叠 |
| 11 | mox-ai-core | mox- | AI 核心 | 与 flow-ai 职责重叠 |
| 12 | mox-expert | mox- | 专家系统 | |
| 13 | mox-graph-meta | mox- | 图谱元数据 | |
| 14 | mox-graph-service | mox- | 图谱服务 | 与 kg-hub 职责重叠 |
| 15 | mox-graph-spark | mox- | 图谱 Spark | 与 graph-algorithms 重叠 |
| 16 | mox-graph-storage | mox- | 图谱存储 | ★自研分布式引擎★ |
| 17 | mox-graph-streams | mox- | 图谱流处理 | |
| 18 | mox-cloud-drive-master | mox- | 云盘主控制 | 4模块可合并 |
| 19 | mox-cloud-drive-volume | mox- | 云盘卷 | 4模块可合并 |
| 20 | mox-cloud-drive-s3 | mox- | 云盘S3 | 4模块可合并 |
| 21 | mox-cloud-drive-filer | mox- | 云盘文件 | 4模块可合并 |
| 22 | mox-common-meta | mox- | 通用元数据 | 应移入 libs/ |
| 23 | mox-compliance | mox- | 合规审计 | |
| 24 | mox-data-plane | mox- | 数据平面 | |
| 25 | mox-domain-abstractions | mox- | 领域抽象 | 应移入 libs/ |
| 26 | mox-etl-wasm | mox- | ETL WASM | wasm 是实现细节 |
| 27 | mox-fusion | mox- | 融合引擎 | |
| 28 | mox-server | mox- | 主服务器 | 单体入口，应拆为网关 |
| 29 | mox-standards | mox- | 标准/国密 | 应移入 libs/ |
| 30 | mox-system | mox- | 系统管理 | 薄弱，需增强 RBAC/审计 |
| 31 | mox-t21-harness | mox- | 测试框架 | 应移入 libs/ |
| 32 | primiflow-core | 无 | 流程核心 | 命名缺前缀 |
| 33 | primiflow-fusion | 无 | 流程融合 | 命名缺前缀，与 hermes-flow-bridge 重叠 |
| 34 | mox-formulas-core | mox- | 公式计算 | 应移入 crates/（计算核心） |
| 35 | mox-norm-core | mox- | 归一化 | 应移入 crates/ |
| 36 | mox-intent-core | mox- | 意图识别 | 应移入 crates/ |

### 1.2 核心问题

| 问题 | 数量 | 说明 |
|------|------|------|
| **命名不统一** | 13个模块缺 mox- 前缀 | operator-core, graph-algorithms, flow-ai, ai-agent 等 |
| **职责重叠** | 4组重叠 | flow-ai↔mox-ai-core, kg-hub↔mox-graph-service, graph-algorithms↔mox-graph-spark, primiflow-fusion↔hermes-flow-bridge |
| **可合并模块** | 4组 | mox-cloud-drive-* (4合1), flow-ai+mox-ai-core (2合1), kg-hub+mox-graph-service (2合1), primiflow-fusion+hermes-flow-bridge (2合1) |
| **层级错位** | 7个 | 计算核心(crates)、共享库(libs)、服务(services) 混在 services/ 下 |
| **缺失服务** | 5个 | 网关(从mox-server拆)、认证、租户、计量、通知 |

---

## 二、优化后服务边界（31 服务 + 1 Sidecar）

### 2.1 服务分类总览

| 类别 | 服务数 | 服务列表 |
|------|--------|----------|
| **接入层** | 1 | mox-gateway-svc |
| **平台能力** | 5 | mox-auth-svc, mox-tenant-svc, mox-metering-svc, mox-notification-svc, mox-system-svc |
| **AI 引擎** | 3 | mox-ai-svc, mox-agent-svc, mox-expert-svc |
| **知识图谱** | 5 | mox-graph-svc, mox-graph-storage-svc, mox-graph-algo-svc, mox-graph-streams-svc, mox-graph-meta-svc |
| **数据与存储** | 4 | mox-storage-svc, mox-etl-svc, mox-dataplane-svc, mox-search-svc |
| **流程与算子** | 3 | mox-flow-svc, mox-flow-fusion-svc, mox-operator-svc |
| **业务与治理** | 5 | mox-compliance-svc, mox-fusion-svc, mox-catalog-svc, mox-market-svc, mox-optimizer-svc |
| **Sidecar** | 1 | ai-inference (Python, GPU) |
| **合计** | **31 + 1** | |

### 2.2 重命名/合并映射表

| 原模块 | 新服务 | 操作 | 说明 |
|--------|--------|------|------|
| mox-server + gateway/runtime | mox-gateway-svc | 合并+重命名 | 主服务器收敛为网关，多协议单端口 |
| (新增) | mox-auth-svc | 新增 | 从 gateway/system 抽取认证授权 |
| (新增) | mox-tenant-svc | 新增 | 租户管理+配额 |
| mox-system | mox-system-svc | 重命名 | 增强 RBAC/审计 |
| flow-ai + mox-ai-core | mox-ai-svc | 合并+重命名 | AI 编排服务，消除重叠 |
| ai-agent | mox-agent-svc | 重命名 | |
| mox-expert | mox-expert-svc | 重命名 | |
| kg-hub + mox-graph-service | mox-graph-svc | 合并+重命名 | 图谱核心服务，消除重叠 |
| mox-graph-storage | mox-graph-storage-svc | 重命名 | ★自研引擎保持不变★ |
| graph-algorithms + mox-graph-spark | mox-graph-algo-svc | 合并+重命名 | 图算法服务 |
| mox-graph-streams | mox-graph-streams-svc | 重命名 | |
| mox-graph-meta | mox-graph-meta-svc | 重命名 | |
| mox-cloud-drive-{master,volume,s3,filer} | mox-storage-svc | 4合1+重命名 | 云存储服务 |
| mox-etl-wasm | mox-etl-svc | 重命名 | wasm 是实现细节 |
| mox-data-plane | mox-dataplane-svc | 重命名 | |
| (新增) | mox-search-svc | 新增 | 全文+向量+图谱联合搜索 |
| primiflow-core | mox-flow-svc | 重命名 | |
| primiflow-fusion + hermes-flow-bridge | mox-flow-fusion-svc | 合并+重命名 | 流程融合，消除重叠 |
| operator-core | mox-operator-svc | 重命名 | WASM 算子管理 |
| operator-wasm | mox-operator-wasm | 移入 libs/ | 库，不是服务 |
| mox-compliance | mox-compliance-svc | 重命名 | |
| mox-fusion | mox-fusion-svc | 重命名 | |
| business-catalog | mox-catalog-svc | 重命名 | |
| template-market | mox-market-svc | 重命名 | 模板+插件市场 |
| optimizer | mox-optimizer-svc | 重命名 | 查询优化/执行计划 |
| (新增) | mox-metering-svc | 新增 | 用量计量+计费 |
| (新增) | mox-notification-svc | 新增 | 通知推送 |
| mox-common-meta | mox-common | 移入 libs/ | 共享库 |
| mox-domain-abstractions | mox-domain | 移入 libs/ | 共享库 |
| mox-standards | mox-standards | 移入 libs/ | 共享库 |
| mox-t21-harness | mox-testkit | 移入 libs/ | 测试工具 |
| mox-formulas-core | (保持) | 移入 crates/ | 计算核心 |
| mox-norm-core | (保持) | 移入 crates/ | 计算核心 |
| mox-intent-core | (保持) | 移入 crates/ | 计算核心 |
| xiaobai-dsp | (保持) | 移入 crates/ | 计算核心 |

### 2.3 每个服务的职责定义

#### 接入层

**mox-gateway-svc**（API 网关）
- 多协议单端口入口：REST / gRPC-Web / WebSocket / SSE
- 租户识别（subdomain / JWT / X-Tenant-Id）
- 统一认证入口（登录/登出/Token 刷新）
- 限流熔断（按租户/用户/接口）
- 路由分发（REST→gRPC 转码 / gRPC-Web→gRPC）
- 灰度发布（按 header/cookie/租户比例）
- API 文档（Swagger + gRPC Reflection）
- 健康检查

#### 平台能力

**mox-auth-svc**（认证授权）
- 用户认证（用户名密码/SSO/OIDC/OAuth2）
- JWT 签发/验证/刷新/黑名单
- RBAC 权限管理（角色/权限/菜单）
- 数据权限（行级/字段级）
- 会话管理

**mox-tenant-svc**（租户管理）
- 租户 CRUD（创建/查询/更新/删除/停用/激活）
- 租户配额管理（用户数/项目数/存储/AI Token/并发）
- 配额检查（网关/拦截器调用）
- 租户用量统计
- 租户隔离级别配置（共享/Schema/独立集群）

**mox-metering-svc**（计量计费）
- 用量采集（API调用/AI Token/存储/流量）
- 配额扣减与检查
- 账单生成
- 套餐管理
- 用量告警

**mox-notification-svc**（通知服务）
- 多渠道通知（站内信/邮件/短信/Webhook/飞书/钉钉）
- 通知模板管理
- 通知订阅管理
- 通知发送记录

**mox-system-svc**（系统管理）
- 用户/角色/部门/菜单/字典/参数管理
- 操作审计日志
- 系统配置管理
- 导入导出

#### AI 引擎

**mox-ai-svc**（AI 编排）
- Prompt 管理（版本化/A-B测试/变量渲染）
- 模型路由（按任务/成本/延迟/租户配额选择模型）
- 多 Agent 编排（DAG 工作流）
- RAG 管道（文档切分→向量化→检索→重排序→生成）
- 语义缓存（相似问题复用回答）
- Guardrails（输出安全校验/事实一致性/PII脱敏）
- 成本追踪（每请求 Token 消耗/费用归因）
- 流式输出（gRPC server streaming）
- 调用 Python 推理 sidecar

**mox-agent-svc**（AI Agent）
- Agent 生命周期管理（创建/配置/启动/停止/销毁）
- 工具调用（工具注册/执行/结果处理）
- 多 Agent 协作（主管/子 Agent 模式）
- Agent 记忆管理（短期/长期/会话）
- Agent 执行轨迹记录

**mox-expert-svc**（专家系统）
- 规则引擎（规则定义/匹配/执行）
- 知识库管理（专家知识/案例库）
- 推理引擎（前向/后向推理）
- 解释生成（推理过程可解释）

#### 知识图谱

**mox-graph-svc**（图谱核心服务）
- 图谱 CRUD（创建/查询/更新/删除图谱）
- 高阶查询（类 Cypher 查询/自然语言查询）
- 本体管理（类/属性/关系/约束）
- 数据摄入（从文档/数据库/API 抽取实体关系）
- 实体合并（消歧/对齐/融合）
- 数据治理（质量校验/血缘/审批）
- 组合 graph-storage + graph-algo + graph-meta 提供业务 API

**mox-graph-storage-svc**（图谱存储引擎）★自研★
- 顶点/边 CRUD（7 个标准存储 API）
- 邻居遍历查询
- 全扫描（流式）
- CDC 变更数据捕获（流式订阅）
- 分片管理（VID hash 分片 + Raft 共识）
- 分片再平衡（16→32 分片）
- 热缓存（Hot Vertex LRU 100k）
- 集群状态管理
- **底层引擎零修改，仅增加 gRPC Server 薄包装层**

**mox-graph-algo-svc**（图算法服务）
- 路径算法（最短路径/所有点对最短路径/K最短路径）
- 中心性算法（PageRank/Betweenness/Degree/Closeness）
- 社区发现（Louvain/Label Propagation/Girvan-Newman）
- 连通性（连通分量/强连通分量）
- 子图提取
- 相似度计算（节点相似度/图相似度）
- 从 graph-storage 拉取子图，内存中 petgraph+rayon 并行计算

**mox-graph-streams-svc**（图谱流处理）
- 订阅 graph-storage CDC 事件
- 实时增量计算（度数更新/社区变化/异常检测）
- 流式图处理（窗口/触发器）
- 图变更通知

**mox-graph-meta-svc**（图谱元数据）
- 图谱 Schema 管理
- 索引管理（属性索引/全文索引/向量索引）
- 约束管理（唯一性/存在性/值域）
- 图谱版本管理
- 图谱统计信息

#### 数据与存储

**mox-storage-svc**（对象存储）
- 文件/对象 CRUD
- 桶管理（创建/删除/配置）
- 分片上传/断点续传
- 国密加密（SM4/SM2/SM3）
- 纠删码/副本
- S3 兼容接口
- 文件分享/权限
- 存储用量统计

**mox-etl-svc**（数据抽取转换加载）
- 数据源管理（数据库/API/文件/消息队列）
- ETL 管道定义（DAG）
- ETL 执行（WASM 算子插件）
- 数据转换/清洗/映射
- 调度/重试/监控
- 数据质量校验

**mox-dataplane-svc**（数据平面）
- 数据路由（按规则路由到不同存储）
- 数据同步（跨存储/跨租户/跨区域）
- 数据血缘追踪
- 数据脱敏/加密
- 数据访问审计

**mox-search-svc**（联合搜索）
- 全文搜索（基于 PostgreSQL tsvector / Tantivy）
- 向量搜索（基于 pgvector / Qdrant）
- 图谱搜索（基于 graph-storage 遍历）
- 联合搜索（全文+向量+图谱融合排序）
- 索引管理（创建/更新/删除/重建）
- 搜索结果高亮/聚合/过滤

#### 流程与算子

**mox-flow-svc**（工作流引擎）
- 工作流定义（DAG/状态机）
- 工作流执行（调度/重试/超时/补偿）
- 节点管理（任务/条件/并行/子流程）
- 连线/数据流
- 执行历史/审计
- 工作流版本管理

**mox-flow-fusion-svc**（流程融合）
- 多流程编排（跨系统/跨服务）
- 事件驱动流程（订阅事件触发流程）
- 流程桥接（不同流程引擎间桥接）
- 流程监控/告警

**mox-operator-svc**（算子服务）
- 算子注册/发现
- WASM 算子管理（上传/编译/部署/版本）
- 算子执行（沙箱隔离/资源限制）
- 算子市场（搜索/安装/评分）
- 算子 SDK（Rust/AssemblyScript 开发模板）

#### 业务与治理

**mox-compliance-svc**（合规审计）
- 操作审计日志（不可篡改）
- 数据合规检查（GDPR/个人信息保护法）
- 数据主体请求（访问/删除/导出）
- 合规报告生成
- 风险评估

**mox-fusion-svc**（数据融合）
- 多源数据融合（数据库/API/文件/图谱）
- 实体对齐（同一实体在不同源的识别）
- 冲突解决（数据冲突的自动/人工解决）
- 数据质量评分
- 融合结果版本管理

**mox-catalog-svc**（业务目录）
- 数据资产目录（数据集/表/字段/API）
- 业务术语管理
- 数据血缘可视化
- 数据资产评分/标签
- 数据资产搜索/发现

**mox-market-svc**（模板/插件市场）
- 模板管理（工作流模板/Agent模板/图谱模板）
- 插件管理（上传/审核/发布/搜索/安装/评分）
- 插件版本管理/回滚
- 开发者管理
- 计费分成

**mox-optimizer-svc**（优化器）
- 查询优化（SQL/Cypher/图查询的执行计划生成）
- 成本模型（基于统计信息的成本估算）
- 执行计划缓存
- 查询重写/优化
- 性能基线/瓶颈分析

---

## 三、限界上下文映射（Context Map）

### 3.1 上下文分组

```
┌─────────────────────────────────────────────────────────────────┐
│                     接入上下文 (Gateway Context)                   │
│                     mox-gateway-svc                               │
└───────────────────────────┬─────────────────────────────────────┘
                            │ gRPC
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ 身份与访问上下文│  │  AI 引擎上下文  │  │ 知识图谱上下文  │
│ auth-svc      │  │ ai-svc        │  │ graph-svc     │
│ tenant-svc    │  │ agent-svc     │  │ graph-storage │
│ system-svc    │  │ expert-svc    │  │ graph-algo    │
│ metering-svc  │  │               │  │ graph-streams │
│ notification  │  │               │  │ graph-meta    │
└───────┬───────┘  └───────┬───────┘  └───────┬───────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────┐
│ 数据与存储上下文│  │ 流程与算子上下文│  │ 业务与治理上下文│
│ storage-svc   │  │ flow-svc      │  │ compliance-svc│
│ etl-svc       │  │ flow-fusion   │  │ fusion-svc    │
│ dataplane-svc │  │ operator-svc  │  │ catalog-svc   │
│ search-svc    │  │               │  │ market-svc    │
│               │  │               │  │ optimizer-svc │
└───────────────┘  └───────────────┘  └───────────────┘
```

### 3.2 上下文关系

| 上游 | 下游 | 关系类型 | 说明 |
|------|------|----------|------|
| gateway-svc | 所有服务 | Open Host Service | 网关对外提供统一 API |
| auth-svc | 所有服务 | Conformist | 所有服务遵循认证标准 |
| tenant-svc | 所有服务 | Conformist | 所有服务遵循租户隔离标准 |
| graph-storage-svc | graph-svc | Customer-Supplier | 图谱核心消费存储引擎 |
| graph-storage-svc | graph-algo-svc | Customer-Supplier | 算法服务拉取子图计算 |
| graph-storage-svc | graph-streams-svc | Customer-Supplier (CDC) | 流处理订阅 CDC 事件 |
| ai-svc | agent-svc | Partnership | AI 编排与 Agent 协同演进 |
| ai-svc | graph-svc | Customer-Supplier | AI RAG 调用图谱检索 |
| ai-svc | search-svc | Customer-Supplier | AI 调用联合搜索 |
| flow-svc | operator-svc | Customer-Supplier | 流程执行调用算子 |
| flow-fusion-svc | flow-svc | Customer-Supplier | 流程融合编排子流程 |
| etl-svc | operator-svc | Customer-Supplier | ETL 执行 WASM 算子 |
| metering-svc | 所有服务 | Customer-Supplier | 所有服务上报用量 |
| notification-svc | 所有服务 | Customer-Supplier | 所有服务触发通知 |

---

## 四、服务粒度校验

### 4.1 粒度检查清单

每个服务必须通过以下检查：

| 检查项 | 标准 | 不通过的处理 |
|--------|------|-------------|
| 单一职责 | 描述不需要用"和"连接两个以上业务概念 | 拆分 |
| 独立数据 | 拥有自己的数据库/Schema，其他服务不直接访问 | 拆分或合并 |
| 独立部署 | 可以单独编译、测试、发布、回滚 | 拆分依赖 |
| 独立团队 | 可以由一个 2-5 人团队全生命周期负责 | 合并或拆分 |
| API 稳定 | 对外 API 变更频率低，向后兼容 | 拆分易变部分 |
| 负载特征 | CPU/IO/内存/GPU 特征一致 | 按负载特征拆分 |

### 4.2 粒度校验结果

| 服务 | 单一职责 | 独立数据 | 独立部署 | 负载特征 | 结论 |
|------|----------|----------|----------|----------|------|
| mox-gateway-svc | ✅ | ✅(配置/路由) | ✅ | IO密集 | 通过 |
| mox-auth-svc | ✅ | ✅ | ✅ | IO密集 | 通过 |
| mox-tenant-svc | ✅ | ✅ | ✅ | IO密集 | 通过 |
| mox-ai-svc | ✅ | ✅(Prompt/缓存) | ✅ | CPU+网络密集 | 通过 |
| mox-graph-storage-svc | ✅ | ✅(★自研★) | ✅ | IO+CPU密集 | 通过 |
| mox-graph-svc | ✅ | ✅(本体/摄入) | ✅ | CPU密集 | 通过 |
| mox-storage-svc | ✅ | ✅(元数据) | ✅ | IO密集 | 通过 |
| mox-flow-svc | ✅ | ✅(流程定义/实例) | ✅ | CPU密集 | 通过 |
| mox-operator-svc | ✅ | ✅(算子元数据) | ✅ | CPU密集 | 通过 |
| mox-market-svc | ✅ | ✅ | ✅ | IO密集 | 通过 |

所有服务通过粒度校验，无纳米服务（过细）或分布式单体（过粗）。

---

## 五、服务间调用拓扑

### 5.1 核心调用链

```
用户请求
  → gateway-svc (认证/限流/路由)
    → ai-svc (AI 编排)
      → auth-svc (权限检查)
      → tenant-svc (配额检查)
      → search-svc (RAG 检索)
        → graph-storage-svc (图谱检索)
      → Python sidecar (模型推理, gRPC streaming)
      → metering-svc (用量记录)
      → notification-svc (完成通知)
```

```
图谱查询
  → gateway-svc
    → graph-svc (高阶查询翻译)
      → graph-storage-svc (存储遍历)
      → graph-algo-svc (算法计算, 拉取子图)
      → graph-meta-svc (Schema/索引查询)
      → metering-svc (用量记录)
```

### 5.2 调用深度控制

| 规则 | 标准 |
|------|------|
| 同步调用链深度 | ≤ 3 跳（gateway → 业务服务 → 平台服务） |
| 异步事件 | 非核心路径用 NATS 事件，不增加同步调用深度 |
| 聚合查询 | 在网关或 BFF 层做并行调用聚合，不串联 |
| 跨服务事务 | 用 Saga 模式（事件驱动），不用分布式事务 |

---

## 六、渐进式拆分策略

### 6.1 拆分优先级

按业务价值和拆分难度排序：

| 优先级 | 服务 | 拆分理由 | 拆分难度 |
|--------|------|----------|----------|
| **P0 立即** | mox-gateway-svc | 高并发、入口、故障影响大 | 中（从 mox-server 拆） |
| **P0 立即** | mox-graph-storage-svc | ★自研引擎★、有状态、独立扩缩容 | 低（已有独立模块） |
| **P0 立即** | mox-ai-svc | 高并发、GPU依赖、负载特征不同 | 中（合并 flow-ai+mox-ai-core） |
| **P1 随后** | mox-auth-svc | 所有服务依赖、需独立稳定 | 中（从 gateway/system 抽） |
| **P1 随后** | mox-tenant-svc | 多租户核心、所有服务依赖 | 中（新增） |
| **P1 随后** | mox-graph-svc | 图谱业务核心、高变更 | 中（合并 kg-hub+mox-graph-service） |
| **P1 随后** | mox-agent-svc | AI Agent、独立扩缩容 | 低（已有独立模块） |
| **P2 逐步** | mox-storage-svc | 云存储、IO密集 | 中（4合1） |
| **P2 逐步** | mox-flow-svc | 流程引擎、CPU密集 | 低（已有独立模块） |
| **P2 逐步** | mox-expert-svc | 专家系统 | 低（已有独立模块） |
| **P2 逐步** | mox-graph-algo-svc | 图算法、CPU密集 | 中（合并 graph-algorithms+mox-graph-spark） |
| **P3 最后** | 其余业务服务 | 业务价值较低、可暂留模块化单体 | 低 |

### 6.2 拆分步骤（每个服务）

```
步骤 1：划清边界
  - 明确服务职责、API 契约、数据归属
  - 在 proto/ 中定义 gRPC 接口
  - 在代码中标记内部调用点

步骤 2：提取数据
  - 创建独立数据库 Schema
  - 数据迁移（双写过渡期）
  - 建立数据同步机制

步骤 3：提取服务
  - 代码移动到独立服务目录
  - 内部调用改为 gRPC 调用
  - 独立编译/测试通过

步骤 4：并行运行
  - 新旧实现并行运行（流量灰度）
  - 对比结果一致性
  - 监控指标对齐

步骤 5：切换流量
  - 网关路由切换到新服务
  - 旧实现保留 2 个版本周期
  - 监控无异常后删除旧实现

步骤 6：独立部署
  - 独立 K8s Deployment/Service/HPA
  - 独立 CI/CD 流水线
  - 独立监控仪表盘/告警
```

---

## 七、总结

服务边界优化的核心是**"合并重叠、拆分单体、统一命名、划清边界"**：

1. **从 36 个模块优化为 31 个服务 + 1 Sidecar**，消除 4 组职责重叠
2. **统一命名规范**：所有服务 `mox-{domain}-svc`，共享库移入 `libs/`，计算核心移入 `crates/`
3. **新增 5 个平台能力服务**：网关、认证、租户、计量、通知
4. **自研图存储引擎零修改**，仅增加 gRPC Server 薄包装层
5. **渐进式拆分**：P0 先拆网关/AI/图谱存储，P3 最后拆业务服务
6. **每个服务通过粒度校验**，无纳米服务或分布式单体

---

*下一篇：[02-通信架构优化](./02-communication.md)*
