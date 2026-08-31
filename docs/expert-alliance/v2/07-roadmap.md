---
title: 07 - 实施路线图
version: V2.0
authority: 🟢权威
doc_id: EA-DOC-017
last_updated: 2026-08-31
source_of_truth: V2.0目标架构实施路线图（未落地）
---

# 07 - 实施路线图

> 版本：v2.0 | 日期：2026-08-26 | 状态：企业级草案
>
> 前置：[00-需求分析](docs/expert-alliance/v2/00-requirements.md) | [01-架构设计](docs/expert-alliance/v2/01-architecture.md)

---

## 一、总体路线

专家联盟在微服务架构**阶段四（企业级增强）完成后**启动，分 4 个阶段共 16 周交付：

```
微服务架构（W1-20）
  ├── 阶段一：基础建设（W1-4）
  ├── 阶段二：核心拆分（W5-10）
  ├── 阶段三：服务化推进（W11-16）
  └── 阶段四：企业级增强（W17-20）

专家联盟（W21-36）
  ├── 阶段一：基础建设（W21-24）★ 共享库+Proto+基础设施
  ├── 阶段二：核心能力（W25-28）★ 注册中心+Agent运行时+知识图谱
  ├── 阶段三：联盟核心（W29-32）★ 调度+编排+执行+融合+记忆
  └── 阶段四：企业级交付（W33-36）★ 多协议+安全+可观测+场景验证
```

---

## 二、阶段一：基础建设（W21-24）

### 目标
建设专家联盟专用共享库、Proto 契约、基础设施，为后续开发奠定基础。

### 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W21 | 目录结构创建：`services/expert-*/` `libs/mox-expert-*/` `proto/expert/` | 目录结构 | 架构师 |
| W21 | Proto 定义：common/v1 + alliance/v1 + registry/v1 + agent/v1 + kg/v1 | proto/ 目录 | 全组 |
| W21 | 代码生成：prost + tonic 代码生成配置（build.rs） | 生成代码 | 后端 |
| W22 | mox-expert-core：专家定义/Agent trait/工具调用/记忆接口 | libs/mox-expert-core/ | 后端 |
| W22 | mox-mcp：MCP协议实现（标准方法/工具描述/转码） | libs/mox-mcp/ | 后端 |
| W22 | mox-alliance-client：联盟SDK（任务创建/进度订阅/结果获取） | libs/mox-alliance-client/ | 后端 |
| W23 | PostgreSQL 迁移：tasks/task_nodes/experts/capabilities/tools/domains/cases/audit_logs | migrations/ | 后端 |
| W23 | Redis 数据模型实现：Key规范/序列化/分布式锁/限流 | libs/mox-expert-core/src/cache/ | 后端 |
| W23 | NATS 事件主题定义 + 发布订阅封装 | libs/mox-expert-core/src/events/ | 后端 |
| W24 | K8s 部署模板：5个新服务的Deployment/Service/HPA/PDB/ConfigMap | deploy/k8s/expert-*/ | 运维 |
| W24 | CI/CD：GitHub Actions（lint/test/build/push）+ ArgoCD Application | .github/workflows/ | 运维 |
| W24 | 开发环境搭建：本地K8s/PostgreSQL/Redis/NATS/MinIO | deploy/dev/ | 运维 |

### 里程碑
- **M1（W22末）**：Proto 定义完成，代码生成跑通，共享库框架搭建
- **M2（W24末）**：所有共享库可用，数据库迁移完成，CI/CD跑通，开发环境就绪

### 验收标准
- [ ] 5个服务的 Proto 定义完成并通过 lint
- [ ] mox-expert-core/mox-mcp/mox-alliance-client 编译通过
- [ ] PostgreSQL 迁移脚本可执行，表结构正确
- [ ] Redis/NATS 封装可用
- [ ] K8s 部署模板可部署空服务
- [ ] CI/CD 跑通（lint→test→build→push→deploy）

---

## 三、阶段二：核心能力（W25-28）

### 目标
完成专家注册中心、Agent 运行时、知识图谱服务三个基础服务。

### 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W25 | mox-expert-registry-svc：专家CRUD/定义验证/版本管理/状态管理 | services/expert-registry/ | 后端 |
| W25 | 专家健康检查：心跳/成功率/延迟/错误率统计 | services/expert-registry/src/health/ | 后端 |
| W26 | 专家匹配算法：图谱推理+综合评分+排序筛选 | services/expert-registry/src/matcher/ | 后端 |
| W26 | 工具自动注册：gRPC Server Reflection 扫描+Tool描述生成 | services/expert-registry/src/tool_discovery/ | 后端 |
| W27 | mox-expert-agent-svc：Agent实例管理/ReAct循环实现 | services/expert-agent/ | 后端 |
| W27 | 工具调用器：gRPC调用+超时/重试/熔断+参数映射 | services/expert-agent/src/tool_executor/ | 后端 |
| W27 | AI推理对接：Python sidecar（Unix Domain Socket）+流式输出 | services/expert-agent/src/ai_client/ | AI |
| W28 | mox-expert-kg-svc：图存储gRPC客户端封装+顶点/边CRUD | services/expert-kg/ | 后端 |
| W28 | 图谱推理：专家匹配查询/协作组合推荐/案例检索/相似度计算 | services/expert-kg/src/query/ | 后端 |
| W28 | 图谱初始化：领域树/能力定义/工具注册/10个内置专家 | services/expert-kg/src/init/ | 后端 |

### 里程碑
- **M3（W26末）**：注册中心可用，专家匹配算法完成，工具自动发现完成
- **M4（W28末）**：Agent运行时可用，知识图谱服务可用，10个内置专家注册完成

### 验收标准
- [ ] 专家注册/更新/注销/查询/搜索/匹配 API 可用
- [ ] 专家定义验证（工具存在性/能力完整性/命名冲突）
- [ ] 专家健康检查（心跳/成功率/延迟）
- [ ] 工具自动注册（从gRPC反射发现，生成MCP Tool描述）
- [ ] Agent ReAct 循环（理解→规划→执行→观察→审核）
- [ ] 工具调用器（gRPC+超时+重试+熔断）
- [ ] AI推理对接（Python sidecar+流式输出）
- [ ] 知识图谱顶点/边CRUD
- [ ] 图谱推理（专家匹配/协作推荐/案例检索）
- [ ] 图谱初始化（领域树+能力+工具+10个专家）
- [ ] 3个服务独立部署，健康检查通过

---

## 四、阶段三：联盟核心（W29-32）

### 目标
完成联盟核心服务（调度/编排/执行/融合/记忆），端到端跑通。

### 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W29 | mox-expert-alliance-svc：任务管理（创建/取消/查询/列表/详情） | services/expert-alliance/src/task/ | 后端 |
| W29 | 任务解析：NLP领域提取+能力识别+输入输出类型判断 | services/expert-alliance/src/parser/ | 后端 |
| W30 | 协作计划生成：DAG生成+依赖分析+6种模式+拓扑排序 | services/expert-alliance/src/planner/ | 后端 |
| W30 | 计划验证：无环检测/可达性/输入完整性 | services/expert-alliance/src/planner/validator.rs | 后端 |
| W31 | DAG执行引擎：拓扑调度/并行执行/依赖管理/状态追踪 | services/expert-alliance/src/executor/ | 后端 |
| W31 | 节点执行：调用agent-svc+超时/重试/替代专家/降级跳过 | services/expert-alliance/src/executor/node_runner.rs | 后端 |
| W31 | 进度推送：WebSocket实时推送+NATS事件 | services/expert-alliance/src/progress/ | 后端 |
| W32 | 结果融合：6种策略（投票/加权/拼接/择优/辩论/迭代） | services/expert-alliance/src/fusion/ | 后端 |
| W32 | 协作记忆：工作记忆/会话记忆/长期记忆/案例提升 | services/expert-alliance/src/memory/ | 后端 |
| W32 | 图谱学习：任务完成→更新边权重/统计/案例 | services/expert-alliance/src/memory/graph_learner.rs | 后端 |
| W32 | 人工干预：暂停/恢复/修改计划/指定专家/跳过节点 | services/expert-alliance/src/intervention/ | 后端 |

### 里程碑
- **M5（W30末）**：任务管理+计划生成完成
- **M6（W32末）**：DAG执行+结果融合+协作记忆完成，端到端跑通

### 验收标准
- [ ] 任务创建/取消/查询/列表/详情 API 可用
- [ ] 任务解析（领域提取/能力识别）
- [ ] 协作计划生成（6种模式+DAG+依赖分析）
- [ ] 计划验证（无环/可达/输入完整）
- [ ] DAG执行引擎（拓扑调度/并行/依赖/状态）
- [ ] 节点执行（超时/重试/替代专家/降级）
- [ ] 进度推送（WebSocket+NATS事件）
- [ ] 结果融合（6种策略）
- [ ] 协作记忆（工作/会话/长期/案例提升）
- [ ] 图谱学习（边权重更新/统计）
- [ ] 人工干预（暂停/恢复/修改/指定/跳过）
- [ ] 端到端跑通：创建任务→自动匹配→计划生成→DAG执行→结果融合→返回结果

---

## 五、阶段四：企业级交付（W33-36）

### 目标
完成多协议网关、安全加固、可观测性、5个核心场景验证，达到企业级交付标准。

### 任务清单

| 周 | 任务 | 交付物 | 负责人 |
|----|------|--------|--------|
| W33 | 多协议网关：gRPC/JSON-RPC/MCP/REST/WebSocket单端口共存 | gateway多协议模块 | 后端 |
| W33 | JSON-RPC→gRPC自动转码：路由表生成+参数转换+错误映射 | gateway转码模块 | 后端 |
| W33 | MCP适配：标准方法实现+工具自动发现+tools/call转码 | gateway MCP模块 | 后端 |
| W34 | 安全加固：JWT认证+RBAC/ABAC+多租户RLS+审计日志 | 所有服务 | 全组 |
| W34 | 加密：TLS/mTLS+字段加密+敏感脱敏+密码哈希 | 所有服务 | 后端 |
| W34 | 弹性容错：限流+熔断+降级+重试+超时+舱壁+DLQ | 所有服务 | 后端 |
| W35 | 可观测性：日志(结构化)+指标(Prometheus)+链路(OTel+Jaeger) | 所有服务 | 运维 |
| W35 | Grafana仪表盘：全局/任务/专家/协作/MCP/基础设施 | deploy/grafana/ | 运维 |
| W35 | 告警：P0-P3规则+多渠道通知（飞书/短信/邮件） | deploy/alerting/ | 运维 |
| W36 | 场景验证1：智能图谱构建（CSV→图谱+质量评估+安全审计） | 测试报告 | 测试 |
| W36 | 场景验证2：跨领域智能分析（客户流失分析+挽回方案） | 测试报告 | 测试 |
| W36 | 场景验证3：自动化数据治理（新数据源全流程治理） | 测试报告 | 测试 |
| W36 | 场景验证4：AI辅助开发（需求→流程图+代码+测试） | 测试报告 | 测试 |
| W36 | 场景验证5：MCP集成（Claude Desktop调用专家联盟工具） | 测试报告 | 测试 |
| W36 | 性能压测：并发100任务/P99延迟/资源使用 | 压测报告 | 测试 |
| W36 | Bug修复+性能调优+文档完善 | - | 全组 |

### 里程碑
- **M7（W34末）**：多协议网关完成，安全加固完成
- **M8（W36末）**：可观测性完成，5个场景验证通过，达到企业级交付标准

### 验收标准
- [ ] 多协议单端口共存（gRPC/JSON-RPC/MCP/REST/WebSocket）
- [ ] JSON-RPC→gRPC自动转码
- [ ] MCP协议支持（initialize/tools/list/tools/call/resources/list）
- [ ] MCP工具自动发现（从gRPC反射）
- [ ] JWT认证+RBAC/ABAC+数据权限
- [ ] 多租户L1隔离（tenant_id+RLS+图存储VID前缀）
- [ ] 不可篡改审计日志
- [ ] TLS/mTLS+字段加密+敏感脱敏
- [ ] 限流+熔断+降级+重试+超时+舱壁+DLQ
- [ ] 结构化日志+Prometheus指标+OTel链路追踪
- [ ] Grafana仪表盘（6个）
- [ ] P0-P3告警规则
- [ ] 5个核心场景验证通过
- [ ] MCP集成验证（Claude Desktop可调用工具）
- [ ] 性能压测达标（并发100/P99<2s不含AI）
- [ ] 服务可用性99.95%
- [ ] 完整文档（8篇架构文档+API文档+运维手册）

---

## 六、团队配置

| 角色 | 人数 | 职责 |
|------|------|------|
| 架构师 | 1 | 整体架构/Proto设计/技术选型/关键模块 |
| Rust后端工程师 | 3-4 | 5个服务开发/共享库/转码/安全 |
| AI工程师（Python） | 1 | AI推理sidecar/提示工程/RAG |
| 运维/DevOps | 1 | K8s/CI-CD/监控/告警/压测 |
| 测试工程师 | 1 | 场景验证/集成测试/压测 |
| 产品经理 | 1 | 需求管理/优先级/验收 |

**总计：8-10人，16周**

---

## 七、风险与应对

| 风险 | 概率 | 影响 | 应对 |
|------|------|------|------|
| gRPC服务反射不可用（部分服务未实现reflection） | 中 | 高 | 工具注册支持手动配置+反射双模式；优先为核心服务添加reflection |
| 专家匹配准确率不足 | 中 | 高 | 先规则匹配+图谱匹配双轨；持续收集反馈优化评分算法；人工标注训练数据 |
| DAG执行复杂度超预期 | 中 | 中 | 先实现串行+并行两种核心模式，辩论/分层/迭代/动态后置；限制单任务最大节点数（≤20） |
| AI推理sidecar性能瓶颈 | 中 | 中 | 模型量化/批处理/缓存；sidecar水平扩展；非AI任务不走sidecar |
| 多协议转码边界case多 | 中 | 中 | 优先支持gRPC+REST，JSON-RPC/MCP后置；转码层充分测试；提供降级直连 |
| 图谱学习效果不明显 | 低 | 中 | 边权重更新先做统计展示，不直接影响匹配；后续A/B测试优化 |
| 团队Rust技能不足 | 中 | 高 | 共享库降低开发门槛；Code Review；培训；关键模块由架构师主导 |
| 16周工期紧张 | 高 | 中 | MVP先行（核心3服务+2种模式+REST）；企业级特性后置；可随时暂停交付可用版本 |

---

## 八、持续演进（W37+）

| 方向 | 内容 |
|------|------|
| 专家扩展 | 持续新增领域专家（法务/财务/HR/医疗等） |
| 协作优化 | 基于案例库的自动模式选择；动态重编排增强 |
| 图谱学习 | 边权重自动优化；专家推荐算法迭代；案例自动聚类 |
| 多模态 | 支持图片/音频/视频输入输出的专家协作 |
| 联邦学习 | 跨租户专家协作（数据不出域，模型/策略共享） |
| 专家市场 | 第三方专家注册/审核/交易平台 |
| 混沌工程 | 主动注入故障验证弹性；定期演练 |
| 成本优化 | 资源使用率分析；AI推理成本优化；自动扩缩容策略 |

---

*文档导航：[README](docs/expert-alliance/v2/README.md) | [00-需求分析](docs/expert-alliance/v2/00-requirements.md) | [01-架构设计](docs/expert-alliance/v2/01-architecture.md) | [02-领域模型](docs/expert-alliance/v2/02-domain-model.md) | [03-业务流程](docs/expert-alliance/v2/03-business-flow.md) | [04-接口设计](docs/expert-alliance/v2/04-api-design.md) | [05-数据架构](docs/expert-alliance/v2/05-data-architecture.md) | [06-安全可观测](docs/expert-alliance/v2/06-security-observability.md) | [07-路线图](docs/expert-alliance/v2/07-roadmap.md)*
