# 系统、图谱、云盘、知识库与OSS设计交付

> 2026-10-01 · 文档设计与静态源码复核；未修改业务代码、数据库、对象内容或运行配置，未部署或连接远程提供方。

完成6份关联设计：能力地图、系统架构、数据契约、接口契约、业务流程和Proposed ADR-19；关联9处既有入口与数据库现状。入口为 [资源知识主题](../../docs/modules/resource-knowledge/README.md#navigation)。

## 核心判断

系统策略、对象内容、云盘文件资源、知识版本、加工、图谱与检索是7个逻辑模块。它们共享身份与精确版本引用，各自拥有事实，不要求新建7个微服务。统一体验建立在一致所有权和授权链路上，不能把云盘、OSS、知识库与图数据库视为一个存储概念。

| 定向复核发现 | 设计处理 |
|---|---|
| 网关KB用mox-kb-svc，独立kb-server已有SQLite但模型不同 | 保留主链，显式模型/版本/租户映射；修正旧“纯内存无SQLite”结论 |
| KbState挂图为独立内存GraphStore | 图明确为来源事实的投影；补持久化或按精确版本重建 |
| 企业文件上传有本地文件，元数据内存/样例初始化 | 文件/目录/版本/共享元数据需成为受控持久化主源 |
| storage/switch只探测可达，未切实际写入后端 | 提供方验证、新对象默认、存量迁移拆为三个操作 |
| 对话沉淀逐步写KB、图和Markdown | 幂等job与阶段回执；云盘失败仅重试导出阶段 |
| 现有S3客户端使用path-style，而OSS官方兼容要求virtual-hosted | 提供方逐项能力/地址/认证验证，不能凭Oss枚举标ready |

源码证据覆盖16个文件，含SHA-256与匹配行：[机器记录](../data/20261001-resource-knowledge-source-evidence.json)。它是静态快照，不构成已部署证明或全系统漏洞报告。

官方技术来源：[OSS兼容范围](https://www.alibabacloud.com/help/en/oss/developer-reference/compatibility-with-amazon-s3)支持地址模式与兼容差异判断；[S3完整性说明](https://docs.aws.amazon.com/AmazonS3/latest/userguide/checking-object-integrity-upload.html)支持ETag不能统一作为整对象MD5的约束。供应商文档与现有代码不一致时，本轮登记适配差距，没有修改客户端或尝试使用现有密钥连接。

## 设计内容

| 交付 | 位置 |
|---|---|
| 能力地图与推进顺序 | [主题入口](../../docs/modules/resource-knowledge/README.md#map) |
| 系统上下文、现状差距、全维配置与12项RK-Q验收 | [架构](../../docs/architecture/RESOURCE-KNOWLEDGE-ARCHITECTURE.md#scope) |
| 资源、位置、文档、片段、断言、作业与状态主源 | [数据](../../docs/database/RESOURCE-KNOWLEDGE-DATA-CONTRACT.md#ownership) |
| 业务操作、幂等、错误、提供方能力与检索引用 | [契约](../../docs/api/RESOURCE-KNOWLEDGE-CONTRACT.md#operations) |
| 上传、知识加工、挂图检索、对话沉淀、删除、迁移6条流程 | [流程](../../docs/modules/resource-knowledge/BUSINESS-FLOWS.md#flows) |
| 原因、取舍、接受条件 | [ADR-19](../../docs/enterprise/47-资源知识主源与存储适配归一-ADR-19.md#decision) |

模型区分文件可读取、知识已发布、投影可检索与提供方已验证；明确源版本固定、旧作业隔离、撤权实时阻断、删除回执、孤儿对象清理、去重保护域、迁移位置对账。低代码配置继续复用LC-STD-001，不复制第二份配置规范。

建议首个样例为两个租户同名文件上传→目录移动→知识加工发布→授权搜索/图追踪→联盟引用→新版与旧作业并发→撤权→回收/恢复/删除。先本地关键词/图闭环，再分别验证真实S3与OSS；容量与检索质量由同一环境和数据集测量，未测不写性能承诺。

## 验证与边界

新6份专题的18处锚点有效，4张Mermaid语法通过；链接门禁检查2334处引用，0断链，207项既有代码路径警告与132处file协议债务保留。图仅解析，未做浏览器视觉核验。证据：[锚点](../data/20261001-resource-knowledge-anchor-verification.json)、[图](../data/20261001-resource-knowledge-mermaid-verification.json)、[链接](../data/20261001-resource-knowledge-doc-links.json)。

docs全量清单为592文件、565文本、166处Mermaid来源、3301引用、1803条显式内部引用边；清单重新生成并check通过。显式引用边不是运行调用关系。新增报告遵守reports归档，无目录迁移；保留历史参考方案，数据库架构只定向更新KB结论，其他历史状态不冒认已全量重核。

本轮完成document_ready；RK-Q01–12均待实施验收。原子元数据/对象接纳、知识版本闭包、持久化加工/删除job、授权检索、图重建和真实提供方迁移仍为设计工作项。没有新增运行时能力、真实模型结果或生产ready声明，未提交或推送。
