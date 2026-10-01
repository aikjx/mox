# 分目录架构设计与低代码归一化矩阵

> LC-DIR-01 · V1.0 · 2026-10-01 · 目录职责设计与工作顺序。结构治理仍以 `docs/ARCHITECTURE-OF-DOCS.md` 为准；本文不迁移现有文件。

<a id="directories"></a>
## 1. 活动主题目录逐层设计

每个目录设计卡统一为：输入→拥有的事实/模型→处理与依赖→输出→验收；本表给出职责，目录入口中的设计卡给出本目录接缝。设计文档的完成不等于该目录所有旧文档已重核；旧快照/报告保留边界。

| 顺序/层 | 目录 | 拥有的架构内容 | 输入→输出 | 依赖/验收 |
|---|---|---|---|---|
| 01 L0 | docs根 | 总导航、文档结构、L1概览 | 产品目标→主题权威链 | 链接门禁与目录覆盖 |
| 02 L5 | standards | 全维配置语义、覆盖/版本/发布规则 | 能力限制→可判定规范 | LC-STD-001，不重复API/DDL |
| 03 L2 | architecture | 平台六层、域职责、装配总图 | 产品约束→逻辑/运行/部署视图 | 现行归一化架构+各主题证据 |
| 04 L2 | architecture/meta | 元数据、控制面与配置编译 | 配置草稿→发布闭包/执行模型 | 类型/依赖/权限/确定性 |
| 05 L4 | api | 配置与命令契约、错误、兼容 | 类型模型→接口schema与版本 | 既有路由核对，未知能力拒绝 |
| 06 L4 | database | 配置版本、指针、快照、迁移 | 模型→存储/事务/恢复策略 | 实际存储与目标模板分开 |
| 07 L4 | database/mox_sys | 稳定平台数据契约与模块主源 | 跨模块需求→身份/资源/事件映射 | 不复制IAM；owner写入 |
| 08 L3 | modules | 业务系统/行业包装配与端到端流程 | 能力与配置→业务包与样例 | 可安装/升级/撤回，不同引擎适配 |
| 09 L2 | architecture/frontend | 页面定义、绑定、外壳与模块出口 | 纯数据定义→受控本地Schema | 不eval，缓存隔离与可访问性 |
| 10 L3 | expert-alliance | 专家配方、计划、执行、融合与证据 | 需求+配方→任务/成果回执 | 主源与版本固定、预算/恢复 |
| 11 L2 | architecture/ai | 模型/单Agent/工具运行适配 | 模型/工具引用→受控调用能力 | 联盟编排与单Agent边界 |
| 12 L2 | architecture/rust-enterprise | 六层编码与消费契约映射 | 逻辑模块→api/proto/core/svc/sdk实现接缝 | 依赖单向、边界与定向测试 |
| 13 L2 | architecture/graph | 结构/关系模型与机器图产物 | 业务证据→可溯源关联图 | 不将引用边当调用边 |
| 14 L2 | architecture/graph/requests | 稳定需求ID与判重来源 | 新需求→需求卡/能力复用关系 | 需求有验收/owner，先判重 |
| 15 L2 | architecture/full-dimensional | 需求与质量多维追踪 | 需求/模块/配置/测试→TraceMatrix | 来源与证据，不复制交易状态 |
| 16 L2 | architecture/microservices | 部署边界与演进容量档 | 性能/故障隔离证据→拆分方案 | 测量驱动，不按页面数拆服务 |
| 17 L2 | architecture/plugin | 扩展协议、能力白名单、兼容 | 插件manifest→注册可用能力 | 禁任意代码加载，权限/禁用/升级 |
| 18 L3工具 | expert-alliance/scripts | 部署后一致性检查与诊断 | 配置环境→验证证据 | 工具不另立运行事实 |
| 19 L5 | normalization | BP/API/ARC/VAL/TPL与来源映射 | 主题源→可检索目录与追踪 | 一个事实一个源，索引不复制内容 |
| 20 L5 | normalization/business | 行业业务设计样例 | 通用包→行业参数与正常/异常流程 | 样例存在不等于生产已部署 |
| 21 L6 | enterprise | 组织交付、ADR、RACI、风险与发布评审 | 方案/证据→决策与交付判定 | Proposed/Accepted分开，无虚构会签 |
| 22 L6 | specifications | 需求规格与契约变更 | 设计→可执行验收与任务范围 | scope/兼容/质量条件齐全 |
| 23 L6 | specifications/tasks | 各历史及新增任务包 | spec→tasks→review | 日期快照、未测项待核，不重复长期规范 |
| 24 L0 | docs-hub | 可视化导航与来源展示 | 主题索引→读者入口 | 不维护第二份架构或状态表 |

## 2. 资产、证据、归档目录

architecture/assets、各_data、_shared及字体/JS子目录只拥有资源/结构化产物；working-reports及其verification/audits子目录拥有带版本的过程证据；_verification/_shots为既有核验/截图，不作为业务设计主源。新增报告遵守仓库约定放reports；本轮不大规模移动历史证据。

所有_archive及其子目录只读，graphify-out/cache为生成缓存；specifications/tasks下各日期目录按任务包规则管理。全目录枚举与文件数由 `reports/data/docs-architecture-corpus.json` 提供，避免为每个历史/资产叶目录再造一份架构规范。

<a id="document-contract"></a>
## 3. 每份架构文档的最小设计契约

1. ID、版本、日期、owner角色、现状/目标/证据类型，不能只写“最优”。
2. 问题、使用者、范围、既有复用来源；不重复另一个目录的事实。
3. 上下文/模块边界、输入输出、依赖方向、主源、契约版本。
4. 正常/失败/恢复流程；每条图标出目标能力与已核实能力。
5. 全维配置矩阵对应维度、schema归属、覆盖与发布方式。
6. 质量场景、兼容、风险、取舍、待核项和验收证据入口。
7. 上游需求与下游接口/数据/页面/测试关联，登记到目录入口。

配置模型语义只由 `docs/standards/lowcode-dynamic-configuration.md#model` 定义；目录文档只维护接缝和本域决策。接口不引用前端函数作为网络契约，DDL不推导业务已运行，配置发布不推导业务恢复完成。

<a id="progress"></a>
## 4. 本轮逐目录设计完成与下一步

| 层级交付 | 文档设计状态 | 实施/验证状态 |
|---|---|---|
| standards统一语义 | LC-STD-001已编写 | 跨引擎实现待任务化 |
| meta控制/运行面 | 05架构已编写 | 原子发布/版本编译未验收 |
| API契约 | 配置操作/版本/错误已编写 | 新端点未声明为已存在 |
| database存储 | owner/事务/快照/恢复已编写 | 无DDL迁移或数据切换 |
| modules低代码装配 | 包单位/业务样例/升级退出已编写 | 样例业务E2E未运行 |
| frontend配置渲染 | 纯JSON→本地Schema适配已编写 | 远程Schema适配未实现 |
| expert-alliance配方 | 19配置与执行边界已编写 | 配方快照与发布联动待实施 |
| 其余活动主题目录 | 目录设计卡与统一接缝已补齐 | 原有每篇历史正文尚非全量重核 |

实施顺序标准→模型→契约→存储→适配→样例→恢复/发布，以LC-Q01–12和EA-Q01–16作为出口。每一步完成证据再更新状态，不将目标设计标成生产ready。
