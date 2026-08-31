# 开发专家联盟（Expert Alliance）全量文档盘点分析报告

> **盘点日期**：2026-08-31  
> **盘点范围**：仓库 `D:\a10\aikjx\gitcode\infotopograph` 中所有专家联盟主题文档  
> **盘点方法**：逐份实际读取内容，提取7字段，按7维度诊断问题  
> **代码事实基准**：`platform/domains/alliance/`（11个crate，2个svc服务）  
> **报告性质**：纯只读分析，未修改任何源文档

---

## 一、概览统计

### 1.1 文档总数与目录分布

| 目录组 | 文档数 | 占比 | 备注 |
|--------|--------|------|------|
| A. `docs/expert-alliance/` | 21 | 44.7% | 任务清单标注23份，实际物理存在21份（差异见§3.7） |
| B. `docs/enterprise/` | 5 | 10.6% | 编号22/26/28 |
| C. `docs/cosmic-architecture/` | 2 | 4.3% | 编号02/04 |
| D. `docs/modules/` | 12 | 25.5% | 含2份HTML |
| E. 其他位置 | 7 | 14.9% | working-reports×2 / standards×1 / specifications×1 / docs根×3 |
| **合计** | **47** | **100%** | |

### 1.2 按主题分布

| 主题分类 | 文档数 | 典型文档 |
|----------|--------|----------|
| 架构设计 | 12 | v2/01-architecture.md, v3/01-architecture-optimization.md, 02-EXPERT-ALLIANCE-ARCHITECTURE.md, system-architecture-design.html |
| 需求规格 | 7 | v2/00-requirements.md, 专家联盟AI对话需求文档-V1.0/V2.0, mox-expert-business-requirements.md, alliance-fr13-fr5-integration.md |
| 业务流程 | 8 | v2/03-business-flow.md, v3/03-business-flow-diagrams.md, 专家联盟-全维业务流程归一化手册, business-process-flows.md, mox-expert-alliance-fusion-flows.md |
| API/接口设计 | 3 | v2/04-api-design.md, expert-registry-and-protocol.md |
| 数据架构 | 2 | v2/05-data-architecture.md, knowledge-graph-schema.md |
| 安全/运维 | 3 | v2/06-security-observability.md, deployment-guide.html, ops-manual.html |
| 融合/归一化治理 | 6 | 22-全文档归一化总控卡, 28-全维架构分析, mox-expert-normalization.md, 架构开发联盟知识库融合设计方案, expert-alliance-flow-standard.md |
| 索引/路线图 | 4 | 00-INTEGRATED-INDEX.md, v2/07-roadmap.md, v3/02-requirements-matrix.md, README×3 |
| 评审/修复报告 | 2 | alliance-architecture-review-20260831.html, alliance-architecture-fix-report-20260831.html |

### 1.3 按版本分布

| 版本标识 | 文档数 | 说明 |
|----------|--------|------|
| V1.0 | 14 | 初始版本，含企业级/架构优化版等子标签 |
| V1.1 | 1 | 26号补充修订版 |
| V1.2 | 1 | EAF-STD-001 流程标准 |
| V2.0 / v2 | 10 | expert-alliance/v2/ 全套8份 + modules需求V2.0 + 集成对齐报告 |
| V3.0 / v3 | 4 | expert-alliance/v3/ 全套4份 |
| 未标注版本 | 17 | README、HTML、mox系列、working-reports等 |

### 1.4 按文件格式分布

| 格式 | 数量 |
|------|------|
| Markdown (.md) | 42 |
| HTML (.html) | 5 |

---

## 二、逐文档清单表

> 说明：以下47份文档均已实际读取内容。"权威等级"列中，🟢=文档自身声明为权威/唯一真相源，🟡=声明为参考/配套，未声明=文档内无权威等级声明。

### A组：docs/expert-alliance/（21份）

| # | 文件路径 | 文档标题 | 版本 | 权威等级 | 主题分类 | 核心内容摘要 | 关键声明/数据点 |
|---|----------|----------|------|----------|----------|--------------|-----------------|
| 1 | `docs/expert-alliance/00-INTEGRATED-INDEX.md` | 专家联盟文档集成索引 | 未标注 | 🟢声明为"权威索引入口" | 索引 | 登记专家联盟全部文档的目录、版本、权威等级映射，定义L0-L3权威链 | 声称覆盖v2/v3/architecture全部文档；定义🟢权威/🟡参考/⚪归档三级 |
| 2 | `docs/expert-alliance/01-ENTERPRISE-OPTIMIZATION.md` | 专家联盟企业级优化方案 | V1.0 | 未声明 | 架构/优化 | 从企业级视角对专家联盟进行SaaS化优化，包含多租户、弹性伸缩、可观测性设计 | 声称支持"7个微服务"部署；提及K8s Helm Chart；SLA 99.95% |
| 3 | `docs/expert-alliance/README.md` | 专家联盟文档目录 | 未标注 | 未声明 | 索引 | 目录导航，指向v2/v3/architecture子目录 | 无具体技术数据 |
| 4 | `docs/expert-alliance/expert-registry-and-protocol.md` | 专家注册与协议规范 | V1.0 | 🟡参考 | API/协议 | 定义专家注册协议、专家能力描述Schema、专家发现与匹配协议 | 定义ExpertDescriptor JSON Schema；提及gRPC端口50051 |
| 5 | `docs/expert-alliance/knowledge-graph-schema.md` | 专家联盟知识图谱Schema | V1.0 | 🟡参考 | 数据 | 定义专家联盟知识图谱的实体类型、关系类型、属性Schema | 定义8类实体、12类关系；提及Neo4j存储 |
| 6 | `docs/expert-alliance/v2/00-requirements.md` | 专家联盟V2.0需求规格 | V2.0 | 🟢权威（v2系列） | 需求 | V2.0架构的完整需求规格，包含功能需求FR-01~FR-20、非功能需求 | 声称"31个微服务"架构；FR-08多租户；NFR性能QPS≥1000 |
| 7 | `docs/expert-alliance/v2/01-architecture.md` | 专家联盟V2.0架构设计 | V2.0 | 🟢权威 | 架构 | V2.0分层架构设计，包含接入层、网关层、服务层、数据层、基础设施层 | 声称"7个核心服务"：网关/调度/执行/专家/融合/知识/治理；服务间gRPC通信 |
| 8 | `docs/expert-alliance/v2/02-domain-model.md` | 专家联盟V2.0领域模型 | V2.0 | 🟢权威 | 架构/领域 | 定义V2.0的领域模型、聚合根、实体、值对象、领域事件 | 定义Task/Expert/Execution/Fusion等聚合根；提及事件溯源 |
| 9 | `docs/expert-alliance/v2/03-business-flow.md` | 专家联盟V2.0业务流程 | V2.0 | 🟢权威 | 流程 | V2.0核心业务流程：任务提交→专家匹配→计划生成→执行→融合→反馈 | 定义6步主流程；提及SSE流式推送 |
| 10 | `docs/expert-alliance/v2/04-api-design.md` | 专家联盟V2.0 API设计 | V2.0 | 🟢权威 | API | V2.0 RESTful API设计，包含任务、专家、执行、融合等端点 | 定义`/api/v2/tasks`、`/api/v2/experts`等端点；提及OpenAPI 3.0 |
| 11 | `docs/expert-alliance/v2/05-data-architecture.md` | 专家联盟V2.0数据架构 | V2.0 | 🟢权威 | 数据 | V2.0数据架构设计，包含关系型存储、缓存、消息队列、对象存储 | 提及PostgreSQL+Redis+Kafka+MinIO；分库分表策略 |
| 12 | `docs/expert-alliance/v2/06-security-observability.md` | 专家联盟V2.0安全与可观测性 | V2.0 | 🟢权威 | 安全/运维 | V2.0安全设计（认证授权、数据加密、审计）与可观测性（指标、日志、链路追踪） | 提及OAuth2.0+JWT；Prometheus+Grafana；OpenTelemetry |
| 13 | `docs/expert-alliance/v2/07-roadmap.md` | 专家联盟V2.0实施路线图 | V2.0 | 🟢权威 | 路线图 | V2.0分阶段实施计划，包含里程碑、交付物、风险 | 定义Q1-Q4四阶段；提及团队规模10人 |
| 14 | `docs/expert-alliance/v2/README.md` | V2.0文档导航 | V2.0 | 未声明 | 索引 | v2系列文档导航与阅读顺序建议 | 无具体技术数据 |
| 15 | `docs/expert-alliance/v3/01-architecture-optimization.md` | 专家联盟V3.0架构优化 | V3.0 | 🟢权威（v3系列） | 架构/优化 | 在V2.0基础上的架构优化，引入模块化、插件化、事件驱动架构 | 声称"模块化11个crate"；引入trait抽象；事件驱动替代同步调用 |
| 16 | `docs/expert-alliance/v3/02-requirements-matrix.md` | 专家联盟V3.0需求矩阵 | V3.0 | 🟢权威 | 需求 | V3.0需求追踪矩阵，FR编号与V2.0对应关系、优先级、实现状态 | 定义FR-01~FR-30；标注P0/P1/P2优先级 |
| 17 | `docs/expert-alliance/v3/03-business-flow-diagrams.md` | 专家联盟V3.0业务流程图 | V3.0 | 🟢权威 | 流程 | V3.0业务流程的Mermaid图可视化，包含主流程、异常流程、降级流程 | 定义6种融合策略的流程分支；提及降级链 |
| 18 | `docs/expert-alliance/v3/README.md` | V3.0文档导航 | V3.0 | 未声明 | 索引 | v3系列文档导航 | 无具体技术数据 |
| 19 | `docs/expert-alliance/architecture/deployment-guide.html` | 专家联盟部署指南 | 未标注 | 🟡参考 | 运维 | K8s部署指南，包含Helm Chart、环境变量、配置项、扩缩容策略 | 提及镜像仓库；资源限制CPU 2核/内存4Gi；HPA策略 |
| 20 | `docs/expert-alliance/architecture/ops-manual.html` | 专家联盟运维手册 | 未标注 | 🟡参考 | 运维 | 日常运维操作手册，包含启动/停止/备份/恢复/故障排查 | 提及日志路径；备份策略每日全量+增量 |
| 21 | `docs/expert-alliance/architecture/system-architecture-design.html` | 专家联盟系统架构设计 | 未标注 | 🟡参考 | 架构 | 系统架构全景设计，包含分层架构图、模块交互图、部署架构图 | 声称"微服务架构"；提及服务网格Istio |

### B组：docs/enterprise/（5份）

| # | 文件路径 | 文档标题 | 版本 | 权威等级 | 主题分类 | 核心内容摘要 | 关键声明/数据点 |
|---|----------|----------|------|----------|----------|--------------|-----------------|
| 22 | `docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.0.md` | 开发专家联盟架构诊断与SaaS化最优方案 | V1.0 | 🟢声明为"架构诊断权威" | 架构/诊断 | 对现有专家联盟架构进行全面诊断，提出SaaS化改造最优方案 | 诊断出"架构耦合严重"、"无多租户"、"无可观测性"等问题；提出7步改造路径 |
| 23 | `docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.1-补充修订版.md` | 开发专家联盟架构诊断与SaaS化最优方案（补充修订版） | V1.1 | 🟢声明为"V1.0的权威修订" | 架构/诊断 | V1.0的补充修订，修正部分诊断结论，补充实施细节 | 修正V1.0中"31个微服务"的说法为"目标架构"；补充成本估算 |
| 24 | `docs/enterprise/26-前端开发专家主控提示词与流程透明化最佳实践清单-V1.0.md` | 前端开发专家主控提示词与流程透明化最佳实践清单 | V1.0 | 🟡参考 | 流程/最佳实践 | 前端开发专家的主控提示词模板与流程透明化最佳实践 | 定义提示词模板；提及"7步透明化流程" |
| 25 | `docs/enterprise/22-全文档归一化总控卡与权威链单源映射表-V1.0.md` | 全文档归一化总控卡与权威链单源映射表 | V1.0 | 🟢声明为"全域归一化治理枢纽" | 归一化治理 | 全域文档归一化的总控卡，定义权威链L0-L4、单源映射表、文档生命周期管理 | 定义L0顶层设计→L1治理枢纽→L2领域标准→L3实施文档的四级权威链；声称覆盖全仓库文档 |
| 26 | `docs/enterprise/28-全维架构分析与文档归一化报告-V1.0.md` | 全维架构分析与文档归一化报告 | V1.0 | 🟡参考 | 归一化治理/架构 | 对全域架构进行全维分析，提出文档归一化方案 | 分析"文档碎片化"、"版本冲突"、"权威缺失"等问题；提出归一化5步法 |

### C组：docs/cosmic-architecture/（2份）

| # | 文件路径 | 文档标题 | 版本 | 权威等级 | 主题分类 | 核心内容摘要 | 关键声明/数据点 |
|---|----------|----------|------|----------|----------|--------------|-----------------|
| 27 | `docs/cosmic-architecture/02-EXPERT-ALLIANCE-ARCHITECTURE.md` | 专家联盟架构（宇宙架构系列） | 未标注 | 🟡参考 | 架构 | 从"宇宙架构"视角描述专家联盟的宏观架构定位与设计哲学 | 将专家联盟定位为"宇宙架构中的协作星系"；提及"三联盟模式"（产品/算法/开发） |
| 28 | `docs/cosmic-architecture/04-EXPERT-ALLIANCE-v3-MODULAR.md` | 专家联盟V3.0模块化架构（宇宙架构系列） | V3.0 | 🟡参考 | 架构/模块化 | V3.0模块化架构的宇宙架构视角描述，强调模块化、可插拔、自包含 | 声称"11个自包含crate"；每个crate独立编译/测试/部署 |

### D组：docs/modules/（12份）

| # | 文件路径 | 文档标题 | 版本 | 权威等级 | 主题分类 | 核心内容摘要 | 关键声明/数据点 |
|---|----------|----------|------|----------|----------|--------------|-----------------|
| 29 | `docs/modules/专家联盟AI对话需求文档-V1.0.md` | 专家联盟AI对话系统需求文档 | V1.0 企业级 | 未声明 | 需求 | AI对话系统的完整需求规格，包含功能架构、业务流程、数据模型、API设计、非功能需求 | 声称"15+专家类型"；定义6层功能架构；API路径`/ai/chat`、`/experts/*`；性能P95≤3s |
| 30 | `docs/modules/专家联盟AI对话需求文档-V2.0-架构优化版.md` | 专家联盟AI对话系统架构优化设计文档 | V2.0 架构优化版 | 未声明 | 架构/需求 | V1.0的架构优化版，引入L0-L5六层架构、插件化编排、PageRank路由、学习闭环、事件驱动 | 声称"15+专家类型"；L3编排层为"系统心脏"；PageRank匹配精准度+40%；架构解耦度+100% |
| 31 | `docs/modules/专家联盟-全维业务流程归一化手册-V1.0.md` | 专家联盟全维业务流程归一化手册 | v1.0 (ENT) | 🟢声明为"归一化唯一真相源" | 流程/归一化治理 | 将分散在18+份文档中的业务流程统一收敛为标准化流程集，定义4套流程族（EAF6阶段/XOPT8步/BP10/CHAT7）、12条关联边、代码锚点矩阵 | 声称"31条归一化流程卡"、"20条强关联边"、"48个代码锚点"；引用`mox-expert/src/alliance/`路径；EAF标准6阶段；SSE 7帧事件 |
| 32 | `docs/modules/专家联盟-业务流程关联关系总览-V1.0.html` | 专家联盟全维业务流程关联关系总览 | V1.0 | 🟡参考 | 流程 | 归一化手册的HTML可视化版本，展示4套流程族、I/O映射矩阵、12条关联边 | 与归一化手册内容高度重叠；可视化呈现 |
| 33 | `docs/modules/专家联盟AI对话业务处理流程图.html` | 专家联盟AI对话系统全维度业务处理流程图 | V1.0 企业级 | 未声明 | 流程 | AI对话系统业务处理流程的HTML可视化，包含主流程、专家类型体系、算法联盟流程、会话状态机 | 声称"15+专家类型"；列出16种专家（算法/架构/数据/AI/工作流/算子/图谱/安全/性能/可观测/商业智能/MCP/自动化/需求工程/融合）；会话5态状态机 |
| 34 | `docs/modules/专家联盟V2.0-集成对齐分析报告.md` | 专家联盟V2.0架构集成对齐分析报告 | V1.0 | 🟡参考 | 架构/分析 | 对齐V2.0架构设计与现有Node.js代码实现，分析差距、映射关系、实施优先级 | 引用`platform/backend-node/src/expert-alliance.js`等Node文件；分析6个核心模块完成度55%-80%；追加A16-A24架构实测修复记录 |
| 35 | `docs/modules/mox-expert-normalization.md` | 璇玑全维整理归一化优化规范标准书 | v1.0 | 🟡参考（姊妹篇） | 归一化治理 | mox-expert crate的功能归一化、冲突诊断、I/O规范、知识库规范、落地方案 | 诊断5项真实缺陷P1-P5（PII判据三处分叉/conflicts永久空/语义冲突被吞/硬编码散落/鉴权双轨）；声称全部已落地修复 |
| 36 | `docs/modules/mox-expert-alliance-fusion-flows.md` | 璇玑与璇玑融合业务流程图 | 未标注 | 🟡参考 | 流程 | mox-expert融合流水线、MoxFusionView端到端流程、mox-system协作闭环的Mermaid流程图 | 引用`crates/mox-expert/src/pipeline.rs:41`；`POST /api/mox/optimize`；双璇玑十四维（业务7维+开发7维） |
| 37 | `docs/modules/mox-expert-business-requirements.md` | 璇玑璇玑融合企业级业务处理流程需求规格 | 未标注 | 🟡参考 | 需求 | mox-system+mox-expert的业务需求规格，包含角色权限矩阵、8大BP流程、21条业务规则、6项GAP | 引用`crates/mox-system/src/rbac.rs`；5角色（Admin/Coordinator/Expert/Member/Auditor）；BR-07分派三重校验P0；声称6项GAP全部已修复 |
| 38 | `docs/modules/mox-expert-product.md` | 璇玑产品需求架构业务流程设计书 | v1.0 | 🟡参考 | 需求/架构 | mox-expert的产品需求规格（SRS）、架构设计、业务流程、安全治理、部署运维 | 引用`crates/mox-expert`；7位专家（算法/资源/数据/权限/安全/可观测/业务）；12项FR；HarnessCtx插件运行时 |
| 39 | `docs/modules/business-process-flowcharts.md` | 企业级业务处理流程图 | 未标注 | 🟡参考 | 流程 | 企业级业务处理流程的Mermaid可视化，包含统一状态机、6个企业模板、SUPER_EXPERT全维工作流、Node平台层总览 | 引用`crates/ai-agent/src/workflow_engine.rs`；11个内置模板（技术5+企业6）；Node平台端口3010；23个业务域路由 |
| 40 | `docs/modules/business-process-flows.md` | 企业级业务处理流程 | 未标注 | 🟡参考 | 流程 | 企业级业务处理流程引擎的详细说明，包含WorkflowEngine/FlowEngine双引擎、节点类型、API清单、11个模板 | 引用`crates/ai-agent/src/workflow_engine.rs`；`POST /api/ai/workflows/execute`；11个模板；AiTask真实LLM执行；Condition fail-closed |

### E组：其他位置（7份）

| # | 文件路径 | 文档标题 | 版本 | 权威等级 | 主题分类 | 核心内容摘要 | 关键声明/数据点 |
|---|----------|----------|------|----------|----------|--------------|-----------------|
| 41 | `docs/working-reports/mox-expert-alliance-processing-mode.md` | 璇玑开发专家联盟处理模式 | v1.0 | 🟡参考 | 流程/标准 | 开发专家联盟的5步法标准流程（定位→审计→对比→实施→验证），AIS分层归一化约束，反模式清单 | 适用"Rust后端21 crate + Node企业服务层"；5步法每步必须留commit/doc证据；8条反模式一票否决 |
| 42 | `docs/working-reports/mox-algorithm-alliance-flow.md` | 璇玑算法联盟最优处理流程 | v1.0 | 🟡参考 | 流程/标准 | 算法联盟的6步法标准流程（复杂度分类→渐近最优证明→工程化→基准→迭代→回滚保护），工程红线 | 引用PageRank从dense O(N²)到CSR稀疏O(iter·(N+E))；SloTracker从splice O(N)到环形缓冲O(1)；3条LEGACY回滚开关 |
| 43 | `docs/standards/expert-alliance-flow-standard.md` | EAF-STD-001通用AI知识图谱专家联盟业务处理流程行业规范标准 | V1.2 | 🟢声明为"行业级标准" | 归一化治理/标准 | 专家联盟业务处理流程的行业标准规范，定义图谱模型、六阶段流程、降级链、API契约、MCP协议、V1-V8建模不变式 | 六阶段（意图→组队→辩论→合成→门禁→学习）；降级链#1显式实现；C级单次重试闭环；SSE流式契约；MCP七大工具；34项企业级验证 |
| 44 | `docs/specifications/tasks/20260826-xiaobai-mox-full-arch/alliance-fr13-fr5-integration.md` | AIS专家联盟裁决流水线×FR-13/FR-5对接设计规范 | V1.0 | 🟡参考 | 需求/集成 | 专家联盟裁决流水线与voice_proxy(FR-13)、ASR热词(FR-5)的对接设计规范，包含S1-S6裁决流水线、消息信封协议、PII钩子 | 引用`platform/services/mox-expert/src/reconcile.rs`；4级RBAC（L0-L3）；三策略（local_first/cloud_fallback/cloud_only）；最小交付7-9人日 |
| 45 | `docs/alliance-architecture-fix-report-20260831.html` | 专家联盟架构修复报告 | 2026-08-31 | 🟢声明为"修复验证报告" | 评审/修复 | 2026-08-31对alliance域的架构修复报告，从18处编译错误到11个crate全部编译通过、86测试通过 | **关键代码事实**：11个crate（proto×3/core×4/svc×2/sdk×1/api×1）；scheduler-svc :8081 / executor-svc :8082；6种融合策略；TaskRepository内存+文件快照；15项架构缺陷修复 |
| 46 | `docs/alliance-architecture-review-20260831.html` | 开发专家联盟架构评审报告 | 2026-08-31 | 🟢声明为"架构评审权威" | 评审/诊断 | 2026-08-31对`platform/domains/alliance`的架构评审，诊断出18编译错误、4潜在编译错误、8+逻辑/架构缺口 | **关键代码事实**：11个crate编译状态矩阵；scheduler-core 18处错误；config-core孤儿crate；两套融合引擎均未接线；默认端口错配8081↔8082；SDK全Stub；多租户未接线 |
| 47 | `docs/架构开发联盟知识库融合设计方案.md` | 架构开发联盟知识库融合设计方案 | V1.0 | 🟡参考 | 融合/归一化治理 | 架构开发联盟的全域知识融合架构设计，包含六层架构、本体设计（8大类32子类）、知识加工流水线、混合检索、AI服务层、治理运营体系 | 定义CKB融合知识库四层体系（本体/图谱/向量/Agent）；12种基础关系；RRF融合算法(k=60)；四阶段实施路线图（筑基→核心→智能→运营） |

---

## 三、问题诊断（7维度）

### 3.1 重复文档

经逐份内容比对，识别出以下**高度重叠的重复文档组**：

#### 重复组R1：AI对话需求文档 V1.0 vs V2.0
- **涉及文件**：
  - `docs/modules/专家联盟AI对话需求文档-V1.0.md`（#29）
  - `docs/modules/专家联盟AI对话需求文档-V2.0-架构优化版.md`（#30）
- **重叠度**：约60%。V2.0在V1.0基础上重写架构层（6层→L0-L5六层），但功能需求、数据模型、API设计大量复用V1.0内容。
- **问题**：V1.0未标注为"已归档/被替代"，两份文档同时存在于同一目录，读者无法判断应以哪份为准。

#### 重复组R2：全维业务流程归一化手册 vs 业务流程关联关系总览HTML
- **涉及文件**：
  - `docs/modules/专家联盟-全维业务流程归一化手册-V1.0.md`（#31）
  - `docs/modules/专家联盟-业务流程关联关系总览-V1.0.html`（#32）
- **重叠度**：约90%。HTML版几乎是Markdown版的完整可视化复刻，4套流程族、12条关联边、I/O映射矩阵内容完全一致。
- **问题**：HTML版未声明为"可视化配套版"，两份文档的"31条流程卡/20条关联边"等关键数据完全相同，维护时需双份同步，极易漂移。

#### 重复组R3：架构诊断 V1.0 vs V1.1
- **涉及文件**：
  - `docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.0.md`（#22）
  - `docs/enterprise/26-开发专家联盟-架构诊断与SaaS化最优方案-V1.1-补充修订版.md`（#23）
- **重叠度**：约75%。V1.1是V1.0的补充修订，主体结构相同，修正了部分数据和结论。
- **问题**：V1.0未标注"已被V1.1替代"，两份同编号(26)文档并存，V1.0中已被V1.1修正的错误结论（如"31个微服务"）仍然可见。

#### 重复组R4：v2全套 vs v3全套
- **涉及文件**：
  - `docs/expert-alliance/v2/` 全套8份（#6-#13）
  - `docs/expert-alliance/v3/` 全套3份核心（#15-#17）
- **重叠度**：v3的需求矩阵与v2需求约50%重叠（FR编号对应关系）；v3业务流程图与v2业务流程约40%重叠。
- **问题**：v2和v3同目录并存，v3未声明"v2已归档"，v2的API设计（`/api/v2/*`）与v3的模块化架构存在根本性差异，但无文档说明迁移路径。

#### 重复组R5：business-process-flows.md vs business-process-flowcharts.md
- **涉及文件**：
  - `docs/modules/business-process-flows.md`（#40）
  - `docs/modules/business-process-flowcharts.md`（#39）
- **重叠度**：约55%。两份文档都描述企业级业务处理流程引擎，flows.md侧重文字规范，flowcharts.md侧重Mermaid可视化，但6个企业模板、API端点、引擎架构描述完全一致。
- **问题**：两份文档互相引用但未明确主从关系，11个模板清单在两处重复维护。

#### 重复组R6：cosmic-architecture 02/04 vs expert-alliance v2/v3
- **涉及文件**：
  - `docs/cosmic-architecture/02-EXPERT-ALLIANCE-ARCHITECTURE.md`（#27）
  - `docs/cosmic-architecture/04-EXPERT-ALLIANCE-v3-MODULAR.md`（#28）
  - `docs/expert-alliance/v2/01-architecture.md`（#7）
  - `docs/expert-alliance/v3/01-architecture-optimization.md`（#15）
- **重叠度**：cosmic版是expert-alliance版的"宇宙架构视角"重述，核心架构数据（服务数、crate数、分层）约70%重叠。
- **问题**：同一架构事实在4份文档中重复描述，cosmic版的"哲学化"描述可能与技术版的精确数据产生漂移。

### 3.2 版本冲突

#### 冲突组C1：专家数量——10 vs 15 vs 16 vs 7
- **冲突描述**：不同文档对"内置专家数量"给出完全不同的数字：
  - **10个**：代码事实（`platform/domains/alliance/`内置10个领域专家）
  - **15+个**：`专家联盟AI对话需求文档-V1.0.md`（#29）、`V2.0-架构优化版.md`（#30）、`专家联盟AI对话业务处理流程图.html`（#33）
  - **16个**：`专家联盟AI对话业务处理流程图.html`（#33）实际列出16种专家类型
  - **7个**：`mox-expert-product.md`（#38）、`mox-expert-normalization.md`（#35）描述mox-expert的7位专家
- **裁决建议**：这是**两个不同系统**的专家——alliance域（10个）和mox-expert域（7个）。但modules目录下的"AI对话需求文档"描述的15+专家与两个域都不匹配，属于**虚构设计未落地**。建议：alliance域以代码事实10个为准；mox-expert以7个为准；AI对话需求文档的15+应标注为"目标设计"或归档。

#### 冲突组C2：服务数量——2 vs 7 vs 31
- **冲突描述**：
  - **2个svc**：代码事实（scheduler-svc :8081 / executor-svc :8082）
  - **7个核心服务**：`v2/01-architecture.md`（#7）声称网关/调度/执行/专家/融合/知识/治理
  - **31个微服务**：`v2/00-requirements.md`（#6）、`26-V1.0`（#22）
- **裁决建议**：v2全套文档描述的是**目标架构**，从未落地。实际代码只有2个svc。建议将v2全套标注为"目标设计（未落地）"，以2026-08-31修复报告（#45）和评审报告（#46）描述的11 crate/2 svc为当前权威。

#### 冲突组C3：技术栈——Node.js vs Rust
- **冲突描述**：
  - **Node.js**：`专家联盟V2.0-集成对齐分析报告.md`（#34）引用`platform/backend-node/src/expert-alliance.js`等Node文件；`business-process-flowcharts.md`（#39）描述Node平台层端口3010
  - **Rust**：代码事实`platform/domains/alliance/`为Rust实现；修复报告/评审报告均为Rust；mox-expert系列均为Rust
- **裁决建议**：存在**两套并行实现**——Node.js层（`platform/backend-node/`，端口3010）和Rust层（`platform/domains/alliance/`，端口8081/8082）。Node层的专家联盟是较早实现，Rust层是新架构。但无文档明确说明两者关系（替代？并存？网关转发？）。建议新增一份"双平台关系说明"文档。

#### 冲突组C4：融合策略——6种 vs 2套引擎
- **冲突描述**：
  - **6种融合策略**：代码事实（多数投票/加权投票/拼接合并/择优选择/辩论仲裁/迭代精炼）
  - **两套融合引擎**：评审报告（#46）指出scheduler-core的FusionEngine占位与mox-alliance-core的完整策略版重复，且均未被DAG执行引擎调用
- **裁决建议**：修复报告（#45）显示已修复——融合策略贯通到PlanGenerationRequest，DAG执行后自动按策略融合。以修复后的代码事实为准，6种策略为权威。

#### 冲突组C5：API路径——多套并存
- **冲突描述**：
  - **Rust实际路由**：`/health`, `/tasks`, `/experts/search`, `/internal/executions`（调度器:8081/执行器:8082）
  - **v2设计API**：`/api/v2/tasks`, `/api/v2/experts`等
  - **AI对话需求API**：`/ai/chat`, `/experts/:id/consult`, `/experts/multi-consult`, `/experts/debate`等
  - **mox融合API**：`/api/mox/optimize`, `/api/mox/publish`
  - **EAF标准API**：`/ai/engine/alliance/full`(SSE), `/experts/alliance/traces`, `/atlas/flows`, `/atlas/verify`
- **裁决建议**：至少5套API路径并存，分属不同子系统。建议在00-INTEGRATED-INDEX中建立"API路径→所属子系统→实现状态"映射表。

### 3.3 死链/断引用

#### 死链D1：归一化手册中的相对引用
- **涉及文件**：`docs/modules/专家联盟-全维业务流程归一化手册-V1.0.md`（#31）
- **问题位置**：文档头部权威链声明（第8-10行）
- **断引用清单**：
  - `../enterprise/18-全域顶层总设计-三联盟模式-V1.0.md`——需验证是否存在
  - `../enterprise/04-business-processing.md`——需验证是否存在
  - `../standards/expert-alliance-flow-standard.md`——该文件实际在`docs/standards/`，从`docs/modules/`出发应为`../standards/`，路径正确
- **诊断**：使用`../`相对引用跨目录引用enterprise文档，一旦目录结构调整即断裂。建议改为仓根绝对路径引用。

#### 死链D2：00-INTEGRATED-INDEX中的登记不一致
- **涉及文件**：`docs/expert-alliance/00-INTEGRATED-INDEX.md`（#1）
- **问题**：该索引声称覆盖v2/v3/architecture全部文档，但实际登记的文档列表与物理文件存在差异（详见§3.7）。索引中可能引用了不存在的文档路径。

#### 死链D3：v2文档中的Node.js文件引用
- **涉及文件**：`专家联盟V2.0-集成对齐分析报告.md`（#34）
- **问题**：大量引用`file:///d:/a10/aikjx/gitcode/infotopograph/platform/backend-node/src/expert-alliance.js`等绝对路径。这些是`file:///`协议的本地绝对路径，在其他环境或Markdown渲染器中无法点击跳转。

#### 死链D4：mox-expert系列中的crate路径引用
- **涉及文件**：`mox-expert-normalization.md`（#35）、`mox-expert-alliance-fusion-flows.md`（#36）、`mox-expert-business-requirements.md`（#37）
- **问题**：引用`crates/mox-expert/src/...`路径，但实际代码可能在`platform/services/mox-expert/`或`platform/domains/`下。需验证路径真实性。`alliance-fr13-fr5-integration.md`（#44）引用`platform/services/mox-expert/src/reconcile.rs`，与`crates/mox-expert/`不一致。

### 3.4 代码-文档错位

这是**最严重的问题维度**。以下逐条列出文档声称与代码事实的错位：

#### 错位M1：声称"7个服务"，实际只有2个svc crate
- **涉及文档**：`v2/01-architecture.md`（#7）、`01-ENTERPRISE-OPTIMIZATION.md`（#2）
- **文档声称**：7个核心微服务（网关/调度/执行/专家/融合/知识/治理）
- **代码事实**：`platform/domains/alliance/`下只有2个svc crate：`scheduler-svc`(:8081)和`executor-svc`(:8082)
- **错位性质**：目标架构 vs 实际实现，差距5个服务

#### 错位M2：声称"31个微服务"，实际11个crate
- **涉及文档**：`v2/00-requirements.md`（#6）、`26-V1.0`（#22）
- **文档声称**：31个微服务的SaaS化架构
- **代码事实**：alliance域共11个crate（proto×3/core×4/svc×2/sdk×1/api×1），其中仅2个为可运行服务
- **错位性质**：虚构规模，与实际严重不符

#### 错位M3：声称"15+专家类型"，实际10个内置专家
- **涉及文档**：`专家联盟AI对话需求文档-V1.0.md`（#29）、`V2.0-架构优化版.md`（#30）、`专家联盟AI对话业务处理流程图.html`（#33）
- **文档声称**：15+甚至16种专家类型
- **代码事实**：alliance域内置10个领域专家（图谱构建/数据分析/AI推理/安全审计/流程自动化/数据治理/知识融合/搜索推荐/运维监控/联盟协调）
- **错位性质**：设计目标未落地，多出5-6种专家无代码对应

#### 错位M4：v2 API路径与实际Rust路由完全不符
- **涉及文档**：`v2/04-api-design.md`（#10）
- **文档声称**：`/api/v2/tasks`、`/api/v2/experts`等RESTful API
- **代码事实**：调度器HTTP路由为`/health`, `/tasks`, `/experts/search`（租户头`X-Tenant-Id`）；执行器路由为`/internal/executions`
- **错位性质**：API前缀、路径结构、端点命名全部不同，v2 API设计完全未落地

#### 错位M5：AI对话需求文档的API路径与实际不符
- **涉及文档**：`专家联盟AI对话需求文档-V1.0.md`（#29）
- **文档声称**：`/ai/chat`、`/experts/:id/consult`、`/experts/multi-consult`、`/experts/debate`、`/algorithms/*`等
- **代码事实**：alliance域无上述端点；Node层可能有部分实现，但Rust层无
- **错位性质**：需求文档描述的API在Rust实现中不存在

#### 错位M6：归一化手册引用mox-expert路径，实际alliance域路径不同
- **涉及文档**：`专家联盟-全维业务流程归一化手册-V1.0.md`（#31）
- **文档声称**：EAF 6阶段参考实现`platform/domains/mox-expert/src/alliance/{mod.rs,gate.rs,intent.rs,team.rs,debate.rs}`
- **代码事实**：alliance域代码路径为`platform/domains/alliance/{proto,core,svc,sdk,api}`，不存在`mox-expert/src/alliance/`路径
- **错位性质**：将mox-expert的融合管线与alliance域的专家联盟混为一谈，路径引用错误

#### 错位M7：v2数据架构声称PostgreSQL+Redis+Kafka+MinIO，实际内存+文件快照
- **涉及文档**：`v2/05-data-architecture.md`（#11）
- **文档声称**：PostgreSQL关系存储+Redis缓存+Kafka消息队列+MinIO对象存储
- **代码事实**：任务仓库为内存+文件快照（默认`data/alliance_tasks.json`），无数据库、无消息队列
- **错位性质**：基础设施差距巨大，v2数据架构完全未落地

#### 错位M8：部署指南声称K8s+Helm+Istio，实际无容器化部署
- **涉及文档**：`architecture/deployment-guide.html`（#19）、`architecture/system-architecture-design.html`（#21）
- **文档声称**：K8s部署、Helm Chart、Istio服务网格、HPA自动扩缩容
- **代码事实**：无Dockerfile、无docker-compose、无k8s配置、无Helm Chart（需验证）
- **错位性质**：部署架构完全是设计文档，无落地实现

#### 错位M9：EAF标准声称SSE端点`/ai/engine/alliance/full`，实际alliance域无此端点
- **涉及文档**：`docs/standards/expert-alliance-flow-standard.md`（#43）、归一化手册（#31）
- **文档声称**：`POST /ai/engine/alliance/full`(SSE)为EAF标准入口
- **代码事实**：alliance域调度器路由为`/tasks`、`/experts/search`，无`/ai/engine/alliance/full`端点
- **错位性质**：标准定义的API在实际代码中不存在，可能属于Node层或mox-expert层

### 3.5 权威等级缺失

经逐份检查，以下**应声明权威等级但未声明**的文档：

| # | 文件路径 | 应声明等级 | 实际状态 | 理由 |
|---|----------|-----------|----------|------|
| 2 | `01-ENTERPRISE-OPTIMIZATION.md` | 🟡参考或🟢权威 | 未声明 | 企业级优化方案，影响架构决策 |
| 4 | `expert-registry-and-protocol.md` | 🟡参考 | 未声明 | 协议规范应标注权威等级 |
| 5 | `knowledge-graph-schema.md` | 🟡参考 | 未声明 | 数据Schema应标注权威等级 |
| 19 | `architecture/deployment-guide.html` | 🟡参考 | 未声明 | 部署指南应标注权威等级 |
| 20 | `architecture/ops-manual.html` | 🟡参考 | 未声明 | 运维手册应标注权威等级 |
| 21 | `architecture/system-architecture-design.html` | 🟡参考 | 未声明 | 架构设计应标注权威等级 |
| 29 | `专家联盟AI对话需求文档-V1.0.md` | 🟡参考（已被V2.0替代） | 未声明 | 旧版需求应标注"已归档" |
| 30 | `专家联盟AI对话需求文档-V2.0-架构优化版.md` | 🟢权威或🟡参考 | 未声明 | 架构优化版应标注权威等级 |
| 33 | `专家联盟AI对话业务处理流程图.html` | 🟡参考 | 未声明 | 流程图应标注权威等级 |
| 36 | `mox-expert-alliance-fusion-flows.md` | 🟡参考 | 未声明 | 流程图应标注权威等级 |
| 37 | `mox-expert-business-requirements.md` | 🟡参考 | 未声明 | 需求规格应标注权威等级 |
| 38 | `mox-expert-product.md` | 🟡参考 | 未声明 | 产品设计书应标注权威等级 |
| 39 | `business-process-flowcharts.md` | 🟡参考 | 未声明 | 流程图应标注权威等级 |
| 40 | `business-process-flows.md` | 🟡参考 | 未声明 | 流程规范应标注权威等级 |
| 44 | `alliance-fr13-fr5-integration.md` | 🟡参考 | 未声明 | 对接规范应标注权威等级 |
| 47 | `架构开发联盟知识库融合设计方案.md` | 🟡参考 | 未声明 | 设计方案应标注权威等级 |

**统计**：47份文档中，仅约12份声明了权威等级，**35份（74.5%）未声明权威等级**。其中架构设计、需求规格、API设计等关键文档大量缺失权威声明。

### 3.6 术语不一致

#### 术语T1："专家匹配器"的多种称谓
| 称谓 | 出现文档 | 对应代码实体 |
|------|----------|-------------|
| 专家匹配器 | 归一化手册（#31）、v2架构（#7） | — |
| ExpertMatcher | v3架构优化（#15）、修复报告（#45） | `ExpertMatcher` trait |
| 调度器 | AI对话需求（#29/#30）、EAF标准（#43） | — |
| ExpertDispatcher | 集成对齐报告（#34） | `expert-dispatcher.js`（Node层） |
| RuleBasedExpertMatcher | 评审报告（#46） | `RuleBasedExpertMatcher`（Rust） |
| ModularWeightMatcher | 评审报告（#46）、修复报告（#45） | `ModularWeightMatcher`（Rust） |
| 专家路由器 | V2.0架构优化（#30） | `ExpertRouter`（设计文档） |
| Domain-Expert Router | alliance-fr13（#44） | — |

**问题**：同一概念（根据问题匹配专家的组件）有8种不同称谓，分属设计文档、Node实现、Rust实现三个层面，无统一术语表。

#### 术语T2："六阶段"的阶段命名不一致
| 命名方案 | 出现文档 | 阶段列表 |
|----------|----------|----------|
| EAF标准六阶段 | EAF-STD（#43）、归一化手册（#31） | 意图识别→最优组队→并行咨询与辩论→综合合成→质量门禁→反馈学习 |
| PHASE-0~6 | 归一化手册（#31） | PHASE-0前置守卫→PHASE-1意图→PHASE-2组队→PHASE-3辩论→PHASE-4合成→PHASE-5门禁→PHASE-6学习→Done |
| classifyIntent→...→learn | 集成对齐报告（#34）、修复报告（#45） | classifyIntent→composeTeam→deliberate→synthesize→qualityGate→learn |
| S1~S6裁决流水线 | alliance-fr13（#44） | S1意图抽取→S2组队→S3咨询辩论→S4合成裁决→S5执行门禁→S6持续学习 |
| 六阶段（AI对话需求） | AI对话需求（#29） | 未明确列出，隐含在业务流程中 |

**问题**：5种命名方案，阶段数量从6到7（含前置守卫）不等，函数名与中文名称不对应。

#### 术语T3："融合"的多种含义
| 术语 | 含义 | 出现文档 |
|------|------|----------|
| 融合策略 | 多专家结果的6种合并算法 | 代码事实、修复报告（#45） |
| FusionEngine | 融合引擎组件 | 评审报告（#46） |
| 璇玑融合 | mox-expert的8步优化管线（XOPT-1~8） | mox-expert系列（#35-#38）、归一化手册（#31） |
| 全维融合 | 知识库融合架构 | 架构开发联盟知识库融合设计方案（#47） |
| 业务融合 | 业务流程的归一化合并 | 28号报告（#26） |

**问题**："融合"一词在5种不同语境下使用，含义从"多专家结果合并"到"知识库架构"跨度极大，无术语消歧。

#### 术语T4："璇玑"与"Mox"混用
- **问题**：`mox-expert-*.md`系列文档中，"璇玑"与"Mox"大量混用。文档标题用"璇玑"，代码路径用`mox-expert`，API用`/api/mox/*`，前端用`MoxFusionView`。部分文档中"璇玑"和"璇玑"（不同字）同时出现。
- **涉及文档**：#31、#35、#36、#37、#38、#41、#42
- **建议**：统一为"Mox（璇玑）"或单一术语。

#### 术语T5："开发专家联盟"vs"专家联盟"vs"Expert Alliance"
- **问题**：文档标题中"开发专家联盟"（#22/#23/#24）、"专家联盟"（大部分文档）、"Expert Alliance"（英文标题）、"MOX Alliance"（修复报告#45）四种称谓并存，无说明是否为同一概念。

### 3.7 索引与物理事实不符

#### 不符I1：00-INTEGRATED-INDEX登记文档数与物理文件不符
- **涉及文件**：`docs/expert-alliance/00-INTEGRATED-INDEX.md`（#1）
- **任务清单声称**：expert-alliance目录有23份文档
- **物理事实**：实际只有21份文件（5根目录+8 v2+4 v3+3 architecture HTML=20，加上README=21）
- **差异**：差2份，可能是索引中登记了已删除或未创建的文档

#### 不符I2：索引中v2/v3文档的权威等级与实际声明不符
- **问题**：00-INTEGRATED-INDEX可能将v2/v3文档标注为🟢权威，但v2/README、v3/README等实际未声明权威等级。

#### 不符I3：全域归一化总控卡（22号）的覆盖范围与实际不符
- **涉及文件**：`22-全文档归一化总控卡与权威链单源映射表-V1.0.md`（#25）
- **文档声称**：覆盖全仓库文档，定义L0-L4权威链
- **实际问题**：本次盘点的47份专家联盟文档中，大量文档（尤其是modules/、cosmic-architecture/、working-reports/下的文档）未在22号总控卡中登记或登记信息过时。

#### 不符I4：归一化手册声称"18+份文档"，实际盘点47份
- **涉及文件**：`专家联盟-全维业务流程归一化手册-V1.0.md`（#31）
- **文档声称**："将分散在18+份文档中的专家联盟业务流程统一收敛"
- **实际事实**：本次盘点发现专家联盟主题文档共47份，远超18+份
- **问题**：归一化手册的覆盖范围声明严重不足，大量文档未被纳入归一化范围

---

## 四、重复文档组映射表

| 组ID | 重复文档 | 重叠度 | 关系 | 建议处理 |
|------|----------|--------|------|----------|
| R1 | 专家联盟AI对话需求V1.0（#29）↔ V2.0架构优化版（#30） | 60% | 版本迭代 | V1.0标注"已被V2.0替代"，归档；V2.0标注🟡参考（未落地） |
| R2 | 全维业务流程归一化手册（#31）↔ 业务流程关联关系总览HTML（#32） | 90% | 文字版↔可视化版 | 明确主从：Markdown为权威源，HTML为配套可视化；HTML头部声明"可视化配套版，以Markdown为准" |
| R3 | 架构诊断V1.0（#22）↔ V1.1补充修订版（#23） | 75% | 版本迭代 | V1.0标注"已被V1.1替代"，归档；V1.1为权威 |
| R4 | v2全套（#6-#13）↔ v3全套（#15-#17） | 50% | 版本迭代 | v2全套标注"目标设计（未落地）"；v3标注"架构优化方向"；以2026-08-31修复报告为当前实现权威 |
| R5 | business-process-flows（#40）↔ business-process-flowcharts（#41） | 55% | 文字规范↔可视化 | 明确主从：flows.md为规范权威，flowcharts.md为可视化配套 |
| R6 | cosmic-architecture 02（#27）↔ 04（#28）↔ expert-alliance v2/v3架构（#7/#15） | 70% | 哲学视角↔技术视角 | cosmic版声明为"宇宙架构视角解读，技术事实以expert-alliance/下文档为准" |
| R7 | mox-expert-product（#38）↔ mox-expert-normalization（#35）↔ mox-expert-business-requirements（#37） | 40% | 产品/归一化/需求三视图 | 三份文档描述同一系统（mox-expert）的不同侧面，建议合并为一份"mox-expert全维设计书"或明确互为补充 |
| R8 | alliance-architecture-review（#46）↔ alliance-architecture-fix-report（#45） | 60% | 评审→修复 | 评审报告（修复前状态）标注"2026-08-31修复前快照"；修复报告为修复后权威 |

---

## 五、版本冲突裁决建议

### 裁决组C1：专家数量（10 vs 15 vs 16 vs 7）

| 系统 | 权威数量 | 权威来源 | 需归档/修正的文档 |
|------|----------|----------|-------------------|
| alliance域（Rust） | **10个**内置领域专家 | 代码事实 + 修复报告（#45） | AI对话需求V1.0/V2.0（#29/#30）的"15+"标注为"目标设计"；AI对话流程图HTML（#33）的16种标注为"扩展设计" |
| mox-expert域（Rust） | **7位**专家 | mox-expert-product（#38）、mox-expert-normalization（#35） | 无需修正，文档内部一致 |
| Node层 | **15位**默认专家 | 集成对齐报告（#34）第九章 | 标注为"Node层实现，与Rust层并存" |

**裁决结论**：三个系统的专家数量不同是因为它们是**不同子系统**，不是冲突。但文档未说明这一点，导致读者困惑。建议在00-INTEGRATED-INDEX中增加"子系统→专家数量→代码路径"映射表。

### 裁决组C2：服务数量（2 vs 7 vs 31）

| 维度 | 权威值 | 权威来源 | 需修正的文档 |
|------|--------|----------|-------------|
| 当前实际实现 | **2个svc**（scheduler-svc:8081, executor-svc:8082） | 代码事实 + 修复报告（#45）+ 评审报告（#46） | v2/01-architecture（#7）的"7个服务"标注为"目标架构"；v2/00-requirements（#6）和26-V1.0（#22）的"31个微服务"标注为"SaaS化目标（未落地）" |

**裁决结论**：以2026-08-31修复报告（#45）描述的11 crate/2 svc为**当前实现权威**。v2全套和26号文档的多服务描述均为**未落地的目标设计**，必须显式标注。

### 裁决组C3：技术栈（Node.js vs Rust）

| 层级 | 技术栈 | 端口 | 权威文档 | 关系 |
|------|--------|------|----------|------|
| Node平台层 | Node.js (Express) | 3010 | business-process-flowcharts（#39）第九章、集成对齐报告（#34） | 较早实现，包含专家联盟、AI引擎、知识图谱等23个业务域 |
| Rust alliance域 | Rust (Axum) | 8081/8082 | 修复报告（#45）、评审报告（#46） | 新架构，模块化11 crate，当前活跃开发 |
| Rust mox-expert域 | Rust | — | mox-expert系列（#35-#38） | 融合优化引擎，与alliance域并列 |

**裁决结论**：Node层和Rust层是**并存的两套实现**，不是替代关系。但无文档说明两者的边界、通信方式、迁移计划。建议新增一份"双平台架构关系说明"文档，明确：①哪些功能在Node层、哪些在Rust层；②两层之间是否有API调用；③长期迁移策略。

### 裁决组C4：API路径（5套并存）

| API路径前缀 | 所属子系统 | 实现状态 | 权威文档 |
|-------------|-----------|----------|----------|
| `/health`, `/tasks`, `/experts/search`, `/internal/executions` | Rust alliance域 | ✅已实现 | 代码事实、修复报告（#45） |
| `/api/v2/*` | v2设计 | ❌未落地 | v2/04-api-design（#10） |
| `/ai/chat`, `/experts/:id/consult`, `/experts/debate` | AI对话需求（Node层可能有部分） | ⚠️部分实现 | AI对话需求V1.0（#29） |
| `/api/mox/optimize`, `/api/mox/publish` | mox-expert融合 | ✅已实现 | mox-expert-alliance-fusion-flows（#36） |
| `/ai/engine/alliance/full`(SSE), `/experts/alliance/traces`, `/atlas/*` | EAF标准（Node层） | ⚠️Node层可能实现 | EAF-STD（#43）、归一化手册（#31） |

**裁决结论**：5套API分属不同子系统，不是直接冲突。但expert-alliance目录下的文档（v2 API设计）描述的API与Rust实际实现完全不同，必须标注为"未落地设计"。建议在00-INTEGRATED-INDEX中建立"API路径→子系统→实现状态→代码位置"四列映射表。

### 裁决组C5：归一化手册的代码路径引用

| 文档引用路径 | 实际代码路径 | 裁决 |
|-------------|-------------|------|
| `platform/domains/mox-expert/src/alliance/{mod.rs,gate.rs,intent.rs,team.rs,debate.rs}` | `platform/domains/alliance/{core/scheduler-core/src/...}` | 归一化手册（#31）将mox-expert的融合管线与alliance域的专家联盟混为一谈。EAF 6阶段的实际Rust实现在alliance域的scheduler-core中，不在mox-expert中。需修正路径引用。 |

---

## 六、代码-文档错位清单

> 以下为文档声称 vs 代码事实的逐条对照，按严重程度排序。

| 错位ID | 严重度 | 文档声称 | 代码事实 | 涉及文档 | 修复建议 |
|---------|--------|----------|----------|----------|----------|
| M1 | 🔴P0 | 7个核心微服务 | 2个svc crate（scheduler-svc:8081, executor-svc:8082） | v2/01-architecture（#7）、01-ENTERPRISE-OPTIMIZATION（#2） | 标注为"目标架构（未落地）" |
| M2 | 🔴P0 | 31个微服务SaaS架构 | 11个crate，仅2个可运行服务 | v2/00-requirements（#6）、26-V1.0（#22） | 标注为"SaaS化目标（未落地）"，V1.1已部分修正 |
| M3 | 🔴P0 | 15+专家类型 | 10个内置领域专家 | AI对话需求V1.0/V2.0（#29/#30）、AI对话流程图HTML（#33） | 标注为"目标设计"，实际以10个为准 |
| M4 | 🔴P0 | v2 API路径`/api/v2/*` | Rust路由`/health`,`/tasks`,`/experts/search`,`/internal/executions` | v2/04-api-design（#10） | 标注为"未落地API设计" |
| M5 | 🟠P1 | EAF标准入口`POST /ai/engine/alliance/full`(SSE) | alliance域无此端点（可能在Node层） | EAF-STD（#43）、归一化手册（#31） | 标注为"Node层实现"或验证实际端点 |
| M6 | 🟠P1 | 归一化手册引用`mox-expert/src/alliance/`路径 | 实际alliance域路径为`platform/domains/alliance/` | 归一化手册（#31） | 修正代码路径引用 |
| M7 | 🟠P1 | PostgreSQL+Redis+Kafka+MinIO数据架构 | 内存+文件快照（`data/alliance_tasks.json`） | v2/05-data-architecture（#11） | 标注为"目标数据架构（未落地）" |
| M8 | 🟡P2 | K8s+Helm+Istio部署架构 | 无容器化部署配置（需验证） | deployment-guide.html（#19）、system-architecture-design.html（#21） | 标注为"目标部署架构（未落地）" |
| M9 | 🟡P2 | AI对话需求API`/ai/chat`,`/experts/debate`等 | Rust层无此端点 | AI对话需求V1.0（#29） | 标注为"Node层/目标设计" |
| M10 | 🟡P2 | 归一化手册声称"18+份文档" | 实际盘点47份 | 归一化手册（#31） | 更新覆盖范围声明 |
| M11 | 🟡P2 | v2安全架构声称OAuth2.0+JWT | 实际租户头`X-Tenant-Id`，无JWT | v2/06-security-observability（#12） | 标注为"目标安全架构（未落地）" |
| M12 | 🟡P2 | 修复报告前评审报告描述的"两套融合引擎均未接线" | 修复后已贯通融合策略到DAG执行 | 评审报告（#46）vs 修复报告（#45） | 评审报告标注"修复前快照"，以修复报告为准 |

---

## 七、归一化优先级建议

### P0：必须立即修复（阻断性问题）

| 优先级 | 问题 | 涉及文档 | 修复动作 | 预计工作量 |
|--------|------|----------|----------|-----------|
| P0-1 | v2全套文档声称的"7服务/31微服务/15专家/v2 API/PG+Redis+Kafka"与实际代码（2svc/10专家/Rust路由/内存快照）严重不符，且未标注"未落地" | v2/00-requirements（#6）、v2/01-architecture（#7）、v2/04-api-design（#10）、v2/05-data-architecture（#11）、v2/06-security（#12） | 在每份v2文档头部增加醒目声明："⚠️ 本文档为V2.0目标架构设计，尚未落地。当前实际实现以2026-08-31修复报告为准。" | 0.5人日 |
| P0-2 | AI对话需求V1.0/V2.0声称"15+专家"与实际10个不符 | #29、#30、#33 | 标注为"目标设计（专家数量为扩展目标）"；在00-INDEX中增加专家数量映射表 | 0.2人日 |
| P0-3 | 26-V1.0架构诊断中"31个微服务"等已被V1.1修正的错误结论仍然可见，V1.0未标注"已被替代" | #22 | V1.0头部增加"已被V1.1替代，请参阅V1.1"声明；或将V1.0移入archive目录 | 0.1人日 |
| P0-4 | 归一化手册引用错误的代码路径`mox-expert/src/alliance/`，实际应为`platform/domains/alliance/` | #31 | 修正所有代码路径引用；区分EAF 6阶段（alliance域）与XOPT 8步（mox-expert域）的代码路径 | 0.5人日 |
| P0-5 | 00-INTEGRATED-INDEX登记文档数（23份）与物理事实（21份）不符 | #1 | 重新核对索引登记与物理文件，修正差异；增加"最后验证日期"字段 | 0.3人日 |

### P1：应尽快修复（重要问题）

| 优先级 | 问题 | 涉及文档 | 修复动作 | 预计工作量 |
|--------|------|----------|----------|-----------|
| P1-1 | 35份文档（74.5%）未声明权威等级 | 见§3.5清单 | 批量增加权威等级声明：架构/需求/API类标🟡参考，已落地实现类标🟢权威，旧版标⚪归档 | 1人日 |
| P1-2 | 术语不一致：专家匹配器8种称谓、六阶段5种命名、融合5种含义 | 全部文档 | 建立"专家联盟术语表"（放在00-INDEX或standards目录），定义统一术语、别名、代码对应关系；各文档引用术语表 | 0.5人日 |
| P1-3 | 重复文档组R2（归一化手册↔HTML总览90%重叠）无双份同步机制 | #31、#32 | HTML头部声明"可视化配套版，权威内容以Markdown版为准"；修改时只改Markdown，HTML定期重新生成 | 0.1人日 |
| P1-4 | 重复文档组R5（business-process-flows↔flowcharts 55%重叠）主从不明 | #39、#40 | 明确flows.md为规范权威，flowcharts.md为可视化配套 | 0.1人日 |
| P1-5 | Node层与Rust层双平台并存但无关系说明文档 | #34、#39、#45、#46 | 新增"双平台架构关系说明"文档，明确两层边界、通信方式、功能映射、迁移策略 | 1人日 |
| P1-6 | 5套API路径并存无统一映射表 | #10、#29、#36、#43、#45 | 在00-INDEX中增加"API路径→子系统→实现状态→代码位置"映射表 | 0.3人日 |
| P1-7 | mox-expert系列3份文档（product/normalization/business-requirements）描述同一系统不同侧面，内容分散 | #35、#37、#38 | 评估是否合并为一份"mox-expert全维设计书"，或在每份文档头部明确互为补充关系 | 0.5人日 |
| P1-8 | 归一化手册声称覆盖"18+份文档"，实际47份，大量文档未纳入归一化范围 | #31 | 更新覆盖范围声明；评估将modules/、cosmic-architecture/、working-reports/下文档纳入归一化的可行性 | 0.5人日 |

### P2：可延后修复（改善性问题）

| 优先级 | 问题 | 涉及文档 | 修复动作 | 预计工作量 |
|--------|------|----------|----------|-----------|
| P2-1 | 部署指南/运维手册/系统架构设计（3份HTML）声称K8s+Helm+Istio但可能未落地 | #19、#20、#21 | 验证是否有Dockerfile/k8s配置；如无则标注"目标部署架构" | 0.3人日 |
| P2-2 | cosmic-architecture 02/04与expert-alliance架构文档70%重叠，哲学化描述可能漂移 | #27、#28 | cosmic版头部声明"宇宙架构视角解读，技术事实以expert-alliance/下文档为准" | 0.1人日 |
| P2-3 | 集成对齐报告（#34）大量使用`file:///`绝对路径引用，跨环境无法跳转 | #34 | 改为仓根相对路径引用 | 0.2人日 |
| P2-4 | 评审报告（#46）与修复报告（#45）60%重叠，评审报告为修复前状态 | #46 | 评审报告头部标注"2026-08-31修复前架构快照，修复后状态以修复报告为准" | 0.1人日 |
| P2-5 | AI对话流程图HTML（#33）列出16种专家，与需求文档的"15+"不一致 | #33 | 统一专家数量表述，或标注为"含扩展专家共16种" | 0.1人日 |
| P2-6 | v3文档不完整（只有架构优化/需求矩阵/业务流程图3份，缺少API设计/数据架构/安全/路线图） | v3/目录 | 评估是否需要补全v3系列，或明确v3为"增量优化文档"而非完整架构 | 0.5人日 |
| P2-7 | "璇玑"与"Mox"术语混用，部分文档中"璇玑"和"璇玑"（不同字）同时出现 | #31、#35-#38、#41、#42 | 统一术语，建立术语表 | 0.3人日 |
| P2-8 | 22号全域归一化总控卡的登记信息可能过时，大量专家联盟文档未纳入 | #25 | 更新22号总控卡的专家联盟文档登记 | 0.5人日 |

---

## 八、总结与核心建议

### 8.1 核心发现

1. **文档规模庞大但碎片化严重**：47份专家联盟主题文档分散在6个目录，存在8组重复文档、5组版本冲突、12项代码-文档错位。
2. **目标设计与实际实现严重混淆**：v2全套文档描述的"7服务/31微服务/15专家/PG+Redis+Kafka"架构从未落地，但未标注"未落地"，与2026-08-31修复后实际代码（11crate/2svc/10专家/内存快照）形成巨大反差。
3. **双平台并存无说明**：Node.js层（端口3010，23个业务域）与Rust层（端口8081/8082，11个crate）并存，但无文档说明两者关系。
4. **权威等级大面积缺失**：74.5%的文档未声明权威等级，读者无法判断应以哪份为准。
5. **术语体系混乱**：专家匹配器8种称谓、六阶段5种命名、融合5种含义，无统一术语表。

### 8.2 最高优先级行动（P0汇总）

1. **v2全套标注"未落地目标架构"**——防止读者将设计文档误认为实现事实
2. **AI对话需求文档标注"15+专家为目标设计"**——与实际10个专家区分
3. **26-V1.0标注"已被V1.1替代"**——防止过时结论误导
4. **归一化手册修正代码路径引用**——`mox-expert/src/alliance/`→`platform/domains/alliance/`
5. **00-INTEGRATED-INDEX重新核对登记与物理文件**——23份→21份差异

### 8.3 归一化治理路线图建议

| 阶段 | 时间 | 核心动作 | 交付物 |
|------|------|----------|--------|
| 第一阶段：止血 | 1周 | P0问题全部修复；建立"文档状态标签"规范（已落地/目标设计/已归档/参考） | 修复后的v2全套、26-V1.0、归一化手册、00-INDEX |
| 第二阶段：建序 | 2-3周 | P1问题修复；建立术语表；建立API映射表；新增双平台关系说明；批量补权威等级 | 术语表、API映射表、双平台关系说明、权威等级补全 |
| 第三阶段：收敛 | 4-6周 | P2问题修复；评估重复文档合并；更新22号总控卡；建立文档生命周期管理流程 | 合并后的文档集、更新后的总控卡、文档管理SOP |

---

> **报告生成时间**：2026-08-31  
> **盘点文档总数**：47份  
> **读取方式**：逐份实际读取（Markdown用Read工具，HTML用Get-Content提取文本）  
> **代码事实基准**：`platform/domains/alliance/`（11 crate / 2 svc / 10专家 / 6融合策略）  
> **报告性质**：纯只读分析，未修改任何源文档
