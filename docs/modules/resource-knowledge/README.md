# 系统、云盘、知识库、知识图谱与对象存储设计入口

> RK-MAP-01 · V1.0 · 2026-10-01 · 目标设计待评审；已开始分模块实施，实际完成范围以[实施台账](docs/modules/resource-knowledge/IMPLEMENTATION-STATUS.md#status)为准。

<a id="map"></a>
## 能力地图与设计顺序

| 模块ID | 唯一职责 | 依赖与验收 |
|---|---|---|
| system-policy | 身份、租户、组织、资源授权、配置发布、审计 | 既有IAM；跨租户及撤权场景 |
| object-storage | 对象字节、物理位置、完整性与提供方适配 | 运维配置；真实往返与能力矩阵 |
| cloud-drive | 目录、文件资源、版本引用、共享与回收站 | IAM+对象引用；移动不搬字节 |
| knowledge-catalog | 知识空间、文档版本、发布策略与源关联 | 文件或内联源；来源可追踪 |
| knowledge-processing | 解析、OCR、分块、抽取与索引作业 | 精确源版本、模型能力、预算；重试/旧作业隔离 |
| knowledge-graph | 实体与关系、事实来源、人工确认及图投影 | 领域引用；不覆盖原文件或业务交易 |
| retrieval | 授权范围内关键词/向量/图检索与引用 | 发布版本/策略；拒绝失效与无权内容 |

设计依赖方向：system-policy→object-storage→cloud-drive/knowledge-catalog→knowledge-processing→knowledge-graph/retrieval。IAM授权、对象读取等共用能力采用提供方契约；浏览图引用目录与知识元数据并不反向获得写入权。内联知识直接建立源对象/版本，不强制先创建云盘目录。

各ID为目标逻辑模块，映射现有crate及网关适配，不要求七个微服务。治理角色建议：系统安全、存储运维、文件产品、知识产品、加工算法、图谱与检索维护者；尚未指派个人。

<a id="navigation"></a>
## 设计文档与权威关系

| 主题 | 唯一设计来源 |
|---|---|
| 系统上下文、模块边界与现状差距 | [统一架构](docs/architecture/RESOURCE-KNOWLEDGE-ARCHITECTURE.md#scope) |
| 所有权、标识、版本、关系及一致性 | [数据契约](docs/database/RESOURCE-KNOWLEDGE-DATA-CONTRACT.md#ownership) |
| 命令、查询、错误与提供方能力 | [接口契约](docs/api/RESOURCE-KNOWLEDGE-CONTRACT.md#operations) |
| 当前端到端链路及状态权威 | [实施台账](docs/modules/resource-knowledge/IMPLEMENTATION-STATUS.md#status) · [实际业务流程](docs/modules/resource-knowledge/BUSINESS-FLOWS.md#implemented) |
| 用户任务、异常与恢复 | [业务流程](docs/modules/resource-knowledge/BUSINESS-FLOWS.md#flows) |
| 竞品参考与优化验收设计 | [分项竞品设计](docs/modules/resource-knowledge/COMPETITIVE-DESIGN.md) |
| 全维交付检查与证据导航 | [20 维交付检查](docs/modules/resource-knowledge/DELIVERY-MATRIX.md) |
| 决策取舍与接受条件 | [ADR-19](docs/enterprise/47-资源知识主源与存储适配归一-ADR-19.md#decision) |

配置语义复用[低代码标准](docs/standards/lowcode-dynamic-configuration.md#model)，运行持久化复核入口见[数据库现状](docs/database/DATABASE-ARCHITECTURE.md#resource-knowledge-20261001)。历史方案保留原日期和参考等级，不据此宣称向量服务、远程OSS或统一检索已部署。

<a id="delivery"></a>
## 阶段交付

1. 冻结所有权与ID映射：梳理三套知识模型、文件目录与对象引用，不删除旧实现。
2. 建立文件资源/版本/授权元数据与对象位置引用：上传、下载、回收、重启恢复作为首个闭环。
3. 建立知识空间/源版本/加工作业：先文本解析和关键词，再按样例增量接入OCR、向量与图投影。
4. 接入多提供方真实往返及迁移对账：默认绑定固定backend，不把可达性当切换完成。
5. 以授权检索、证据引用、撤权、删除传播和恢复演练验收；之后才按容量拆分服务。

出口指标与场景见统一架构RK-Q01–12。每阶段分别记录document_ready、code_ready、runtime_verified、release_ready；当前逐模块实现与验证结果统一引用实施台账，不能从设计文档或目录数量推断 release_ready。
