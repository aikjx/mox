# 专家联盟文档入口

> 本页为导航。唯一完整目录与冲突裁决规则见 `docs/expert-alliance/INDEX.md#一文档总表`；本页不另立事实源。

| 阅读目的 | 入口 |
|---|---|
| 先读全局，理解 docs 各套架构和流程的关系 | [全域架构与业务流程图谱](docs/expert-alliance/17-docs-architecture-and-flow-atlas.md#scope) |
| 评估是否最优，按层推进专家联盟设计 | [模块化产品设计与突破路线](docs/expert-alliance/18-modular-product-design.md#goals) |
| 查询当前实现、传输和存储事实 | [当前实现架构](docs/expert-alliance/CURRENT-ARCHITECTURE.md#一物理代码分布) |
| 查询功能、状态枚举、代码归属 | [归一化架构](docs/expert-alliance/08-normalized-architecture.md#二模块清单与物理代码分布三目录打通) |
| 查询任务运行流程 | [端到端业务流程](docs/expert-alliance/13-end-to-end-business-flow.md#一流程总览) |
| 查询验收规范 | [产品规范](docs/expert-alliance/15-product-spec-standard.md#一规范定位) |
| 查询缺口最近状态与证据 | [决策与状态总账](docs/expert-alliance/16-decision-and-state-ledger.md#一总账总表核心交付) |

原有历史文件保持归档；旧入口的现行替代关系见 `docs/ARCHITECTURE-OF-DOCS.md#ea-navigation-20261001`。全量结构摘录与逐图来源在 `reports/markdown/docs-architecture-corpus.md`，机读数据在 `reports/data/docs-architecture-corpus.json`，均为生成证据。

<a id="lowcode-directory-design"></a>
## 目录架构设计卡：低代码与动态配置（2026-10-01）

> 目标设计接缝；既有正文按原日期/类型解释，未实施能力不标已完成。

| 设计项 | 本目录约定 |
|---|---|
| 输入 | 任务目标、受权上下文、专家/模型/流程配方 |
| 处理与边界 | 解析配置闭包、绑定计划版本，经现有匹配/调度/执行/融合与发布链 |
| 输出 | 有版本和来源的成果与节点/任务回执 |
| 维护角色 | alliance模块owner（角色建议，未指派个人） |
| 配置语义 | [统一规范](docs/standards/lowcode-dynamic-configuration.md#model)，本目录不复制覆盖/生命周期规则 |
| 本目录设计 | [详细接缝](docs/expert-alliance/19-lowcode-dynamic-configuration.md#recipe) |
| 验收 | 类型/依赖/权限/版本/异常/恢复按相关LC-Q条目补证；设计完成与运行验证分开 |

全目录关系见 [目录矩阵](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。新增架构文档应符合 [文档设计契约](docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#document-contract)，各主题拥有自己的事实主源。

业务代码增量：[20统一计划与执行](docs/expert-alliance/20-normalized-plan-execution.md#flow)，关联协议、调度、执行与恢复职责。

上游知识与制品来源设计：[资源知识能力地图](docs/modules/resource-knowledge/README.md#map)；联盟消费授权证据，不接管文件或知识主源。当前链路与交付断点见[实际业务流程](docs/modules/resource-knowledge/BUSINESS-FLOWS.md#implemented)及[全维交付检查](docs/modules/resource-knowledge/DELIVERY-MATRIX.md)。
