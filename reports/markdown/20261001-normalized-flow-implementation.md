# 专家联盟业务流程归一化实现与分析

> 2026-10-01 · 范围：计划契约、执行接纳、恢复入口和拓扑一致性。保留工作区既有改动，未提交、推送或部署。

本轮将此前模块化文档的一个完整执行接缝落实到Rust代码：七种协作模式共用计划契约，生成后校验，新任务执行与恢复扫描共用任务绑定检查，非法配置不进入控制队列。设计及业务流程图见 [20统一计划与执行](../../docs/expert-alliance/20-normalized-plan-execution.md#flow)。

## 总结分析

核心问题是运行入口的约束不足，而不是缺少另一套流程引擎。已有CollaborationPlan与执行器足以承载共用流程；继续添加独立DSL、调度器或微服务会增加重复规则和状态主源。本轮复用既有类型与服务，仅补强契约和接纳边界。

| 问题 | 已实现行为 | 实际收益与限制 |
|---|---|---|
| 空计划、重复ID、错任务、异常权重未统一拒绝 | common-proto的validate集中校验 | 防止错误计划误接纳；不代替资源授权 |
| 未知动态条件可能被忽略 | 限定运算符、路径、决策、分支及依赖后继 | 错误配置在执行前拒绝；复用原条件引擎 |
| 恢复入口可能绕过执行校验 | 恢复与新执行共用validate_for_task | 错误计划保留诊断，不派发；正常计划继续扫描 |
| 同一DAG根节点顺序随机 | 按节点声明顺序入队，拒绝重复依赖 | 同输入拓扑与分层可复现；不保证模型输出确定性 |
| 模式分别测试不足以证明共用执行契约 | 七模式经过生成、接纳、执行、节点终态和融合验证 | 验证本地引擎；节点调用使用测试执行器 |

值类型约束在proto、计划生成在scheduler-core、通用算法在alliance-core、执行和恢复在executor-core、传输与存储装配在svc。没有新增crate、库依赖、HTTP端点或第二份业务状态。旧JSON省略动态路由/权重继续有效；此前非法输入被拒绝是明确的行为收紧。

## 开发与验证

测试先复现空计划/非有限权重被接受、根节点顺序变化、重复依赖被接受、错任务入队及错误计划恢复，再实现修复。之后补齐七模式共用执行与正常恢复场景。

| 检查 | 最终结果 | 证据 |
|---|---|---|
| 四个相关crate单元测试 | 282项通过：common 9、core 115、executor 43、scheduler 115 | [检查0](../data/20261001-flow-check-0.txt) |
| 计划契约集成测试 | 3项通过，含18类字段/结构变异及非有限权重 | [检查1](../data/20261001-flow-check-1.txt) |
| 执行集成测试 | 6项通过，新增一项覆盖七种模式 | [检查2](../data/20261001-flow-check-2.txt) |
| Clippy | 四crate all-targets、-D warnings通过 | [检查3](../data/20261001-flow-check-3.txt) |
| 服务兼容编译 | scheduler-svc与executor-svc通过 | [检查4](../data/20261001-flow-check-4.txt) |
| 文档关联与流程图 | 锚点检查通过；新增1张Mermaid解析通过；链接断链0 | [锚点](../data/20261001-flow-anchor-verification.json)、[流程图](../data/20261001-flow-mermaid-verification.json)、[链接](../data/20261001-flow-doc-links.json) |

上述最终业务检查合计291项测试。命令、耗时、退出码由 [机器记录](../data/20261001-flow-implementation-checks.json) 提供。链接门禁仍有207项既有代码路径告警及132处file协议债务，不宣称全库无警告。Mermaid仅做语法解析，未做浏览器视觉核验。

一次初始扩大测试意外运行了既有round7性能基准，其任务节点使用了不同task_id，不满足新契约；该运行产生的性能数据仅保留为诊断，不用于成功率或性能结论。已修正基准节点的任务绑定，并将历史docs基准数据恢复为运行前内容；修正后的基准已编译及Clippy检查，本轮未再次耗时运行。诊断保存在reports/data/20261001-flow-benchmark-diagnostic.json，不计入最终291项业务测试。

## 变更与实施边界

本轮代码涉及common-proto/types.rs及新增tests/plan_validation.rs、alliance-core/dag.rs、executor-core/dag_engine.rs、tests/integration_e2e.rs和tests/bench_alliance.rs。文档20及README/INDEX/16总账登记对应增量。已有未提交代码保留，仅修改本轮相关函数与测试，不整库格式化。

已实施EA-W03/W05的计划契约和恢复入口部分。全维配置的原子发布、release/recipe/plan版本闭包、真实模型调用、跨进程部署、故障后的外部副作用对账和容量测量仍待分别实施验收。当前恢复跳过错误计划只提供日志诊断，运营待处理面板尚未新增；不将此能力写成完整生产恢复已完成。

后续工作有清楚依赖：先完成精确计划与配置版本绑定，再补可靠接纳/恢复证据，再开展业务包与远程页面配置；服务拆分依赖容量和故障隔离测量。通用原则引用 [低代码标准](../../docs/standards/lowcode-dynamic-configuration.md#scope)，各目录接缝引用 [目录矩阵](../../docs/normalization/DIRECTORY-ARCHITECTURE-PLAN.md#directories)。
