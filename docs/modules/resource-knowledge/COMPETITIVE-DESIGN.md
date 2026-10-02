# 竞品参考与企业能力改进设计

> RK-COMP-01 · 2026-10-01 · 官方资料核验与本仓库设计映射。采用分项标杆，不宣称某产品在所有维度最佳，也不把文档核验标记为竞品实机验收。

模块实现状态唯一来源为[实施台账](docs/modules/resource-knowledge/IMPLEMENTATION-STATUS.md#status)，业务全链路见[实际流程](docs/modules/resource-knowledge/BUSINESS-FLOWS.md#implemented)。下面的产品事实来自官方文档；本仓库差距与优先级是基于源码和测试的工程判断。

## 逐项核验与设计映射

| 参考对象 | 已核验的官方能力/边界 | 本仓库采用的设计与验收 | 工作包 |
|---|---|---|---|
| Dify | 知识流水线包含数据源、解析/分块、知识节点、输入参数、测试与发布 | 配置发布与运行版本分离，任务冻结 parser/chunker/model 配置；不能将通用元数据过滤等同于资源 ACL | R4/R5 |
| Glean | 连接器获取内容与权限信息；删除传播依连接器事件和全量抓取机制而异 | 源内容、权限和删除分别记录同步游标/回执；定义撤权与删除传播时限，不把连接器活跃计数作为数据一致性完成 | R2/R4/R7 |
| Azure AI Search | 支持安全过滤，部分原生身份/ACL 能力为 preview | 本系统以认证身份建立权限范围，授权先于召回/排名/统计；角色字符串匹配不能冒充身份提供方授权 | R2/R5 |
| Nextcloud | 文件规则可覆盖读写/同步；所查 latest 文档指向 upcoming 36，且明确 Context Chat 不遵循该文件规则 | 所有字节、索引、问答和联盟引用逐出口验收，禁止从文件权限通过推断知识上下文权限通过；稳定发行版与应用组合须另行实机核验 | R2/R3/R5 |
| n8n（2026-10-02 增量核验） | 队列模式拆分主节点与 worker，Redis 传递执行 ID；不支持队列模式使用文件系统保存二进制内容，SQLite 不建议用于 queue mode | 作业意图/结果由事务主源持久化；二进制内容走对象存储，工作队列不能承担唯一业务主源职责；本仓库 SQLite 站内箱仍限定同主机 | R3/R4 |
| Temporal（2026-10-02 增量核验） | Activity 任务丢失依靠超时检测和重试；取消依赖 heartbeat，Activity 可接受或忽略取消 | 外部动作需要稳定业务幂等键和提供方回执；取消请求不能立即标为已取消，必须拒绝过期执行器提交；重试不证明外部副作用只发生一次 | R4/R7 |

官方来源分别为 [Dify 流水线](https://docs.dify.ai/en/cloud/use-dify/knowledge/knowledge-pipeline/knowledge-pipeline-orchestration)、[Glean 搜索权限](https://docs.glean.com/administration/search/faq)、[Glean 删除与抓取](https://docs.glean.com/connectors/crawling-faq)、[Azure 查询授权](https://learn.microsoft.com/en-us/azure/search/search-document-level-access-overview)、[Nextcloud 文件访问规则](https://docs.nextcloud.com/server/latest/admin_manual/file_workflows/access_control.html)。资料核验日期不代表全部功能的发布日期；Azure preview、Nextcloud upcoming 不作为稳定生产能力已验收证据。

增量来源：[n8n 官方队列模式文档源码](https://github.com/n8n-io/n8n-docs/blob/main/docs/deploy/host-n8n/configure-n8n/scaling/enable-queue-mode.md)、[Temporal Activity 执行](https://docs.temporal.io/activity-execution)。上述限制是部署及一致性取舍，不是对产品整体优劣的实测结论；本仓库也未完成对应集群能力。站内箱事务落地见[消息中心](../../README.md)。

## 本轮实现的设计改进

- 统计/标签从一次当前主源扫描聚合，查询不写 KV 索引，也不缓存权限。统计字节数是可见文档 JSON 逻辑字节，不是内容去重后的磁盘用量或云盘配额。跨服务实例修改 ACL 后，新统计请求仍重新读取主源。
- 文档检索引用包含 document_id、version、field 和原始 Unicode 字符半开区间，可从该版本正文或标题重建片段。未版本化摘要不作为稳定引用；获取引用源始终按当前 ACL。
- 空白/过长查询和无界 limit 在文档候选召回前拒绝（首次路由投影初始化仍独立读取主源）；同分文档排序以 ID 稳定决胜。Unicode 大小写展开不会把变换后的字节偏移拿去切原文。

操作次数及本地时间测量见 reports/data/20261001-competitive-optimization/。性能门禁约束对象读取预算和权限新鲜度，不用不稳定的绝对毫秒数作为 CI 成败，也不与未实测竞品比较速度。

## 全维改进优先级与验证方法

| 优先级 | 设计项 | 需要的真实验证 |
|---|---|---|
| P0 | 单一知识主源迁移与源权限继承 | ID/版本/归属映射、备份/恢复、双写禁止、跨租户/源撤权全出口 |
| P0 | 事务索引与持久化加工 | 崩溃注入、租约/幂等/取消、generation、过期作业拒绝、索引修复回执 |
| P1 | 低代码版本化配置 | schema/依赖校验、冻结执行配置、灰度、回滚及实际结果改变 |
| P1 | 云盘版本与来源证据 | 目录/文件版本、知识引用、页码/块定位、回收/恢复/保留一致 |
| P1 | 多源检索与质量评测 | 权限先筛选、已发布版本、关键词/向量/图、标注集召回和引用准确率 |
| P1 | 真实 OSS 和对象迁移 | PUT/HEAD/GET/RANGE/LIST/DELETE、STS、位置切换、旧对象可读 |
| P2 | 企业运行治理 | 审计、trace、容量/成本、灾备、升级/回滚、实际 SLO 与安全审查 |

全维检查逐项复用[20 维交付检查](./DELIVERY-MATRIX.md)，退出条件沿用台账 R2–R7。持久化索引仍需后续开发，当前 O(n) 扫描虽减少重复 IO，仍不是大规模查询架构的最终形态。
