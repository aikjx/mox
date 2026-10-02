# MOX 核心能力与业务价值

> CORE-CAPABILITIES-001 · 2026-10-02。本页维护能力定位；实际模块、接口、部署端口和验收结果分别引用对应主源。功能目标与生产完成状态分开记录。

MOX 将知识资源、专家协作、业务流程与统一服务治理组合为企业 AI 平台。用户提出目标，系统在真实身份与资源权限下调用已有能力，返回可追踪的分析、执行回执或明确失败，并把成果关联到业务项目和知识来源。

## 用户问题与模块能力

| 用户问题 | 能力与责任 | 详细需求和核验 |
|---|---|---|
| 企业身份和权限分散 | IAM 管理真实用户/租户/角色/权限；网关从可信凭证绑定上下文 | [IAM 权限](modules/iam/README.md) · [凭证](modules/iam/API-KEYS.md) |
| 多专家协作缺少统一过程 | 专家目录、匹配、计划、调度、执行与结果融合拥有各自事实 | [联盟现状](expert-alliance/CURRENT-ARCHITECTURE.md) · [联盟状态总账](expert-alliance/16-decision-and-state-ledger.md) |
| 文件与知识出处难追踪 | 对象字节、文件元数据、知识版本、图投影和检索引用分层管理 | [资源知识主题](modules/resource-knowledge/README.md) |
| 业务页面和规则重复开发 | 声明式页面复用组件，目标远程配置绑定受控能力与版本 | [页面运行架构](architecture/frontend/LOWCODE-PAGE-RUNTIME.md) · [配置标准](standards/lowcode-dynamic-configuration.md) |
| 消息与通知状态不一致 | 事务收件箱拥有消息和独立回执；通知页面是读取适配 | [消息中心](modules/message-center/README.md) |
| 集成或模型失败被误判为成功 | 真实提供方回执、失败与未知状态明确区分，不用模板或固定计数兜底 | [真实实现台账](modules/REAL-IMPLEMENTATION-STATUS.md) |
| 生产交付缺少证据 | 逐模块验证权限、故障、恢复、容量、依赖、回滚和真实用户流程 | [企业需求与关系总入口](modules/enterprise-capabilities/README.md) |

## 架构边界

领域 api/proto 定义外部契约，core 拥有领域计算，svc 组装 IO 与生命周期，网关承担入口与协议适配；存量 core 和网关中仍有 IO/内联业务，应按实际代码依赖渐进拆分。共享基座避免每个业务包复制认证、错误、配置与存储。模块关系和每模块流程见[企业能力关系](modules/enterprise-capabilities/RELATIONSHIPS.md)与[业务主链](modules/enterprise-capabilities/BUSINESS-FLOWS.md)。

AI 分析、真实工具执行、外部提供方接受、最终交付和用户接受是不同事实。低代码配置只能装配已注册能力，不能绕过授权、创建不存在的适配器或把配置草稿视为已发布。图谱与索引是可重建投影，文件/知识/交易主源由各领域持有。

## 事实与验收主源

- 编译单元、入口和实际依赖：[代码目录](modules/CODE-CATALOG.md)，由 workspace 生成。
- 静态接口登记：[API 注册表](API-REGISTRY.md)；注册元数据不代表全路由已挂载或功能已验收。
- 服务地址、统一网关与进程端口：[端口注册表](api/PORT-REGISTRY.md)，不在概览复制端口和旧路由数量。
- 功能编号、逐模块流程和关系：[企业能力登记](modules/enterprise-capabilities/README.md)。
- 平台实际验证与未完成项：[真实实现台账](modules/REAL-IMPLEMENTATION-STATUS.md)。
- 知识资源验证与后续阶段：[资源知识台账](modules/resource-knowledge/IMPLEMENTATION-STATUS.md)。

跨主机分布式、真实 OSS 往返、完整模型/工具推理、低代码发布、全域授权与生产恢复必须逐项提供运行证据。现有局部测试、设计文档、进程可启动或静态 ready 标记不能证明这些目标已完成。
