---
doc_id: EA-BENCH-011
title: 专家联盟竞品对标
version: V1.0
authority: 🟡 外部研究稿（竞品事实均带来源 URL；我方事实引自内部权威文档）
last_updated: 2026-09-29
search_date: 2026-09（外部事实核证日期，时效信息以该日为准）
---

# 11 专家联盟竞品对标

> 本文为「专家联盟」在多智能体协作赛道的外部竞品对标。所有竞品功能事实均来自 2026-09-29 的 `general_search` 核证，逐条附来源 URL；搜不到的事实标「未核」。我方结论以 [CURRENT-ARCHITECTURE.md](CURRENT-ARCHITECTURE.md) V1.1、[08-normalized-architecture.md](08-normalized-architecture.md)、[09-deployment-templates.md](09-deployment-templates.md) 为依据。

---

## 一、对标口径

- **我方定位**：多智能体协作 + 知识图谱 + 专家编排，自研 Rust 后端（16 crates、3 服务 + 网关），面向政务/企业内网、信创与等保场景。
- **对标维度**：定位 / 核心能力 / 企业级能力（权限·审计·多租户·部署形态）/ 生态（插件·市场）/ 我们强 / 我们弱 / 借鉴点。
- **竞品分四类**：
  1. 多智能体框架：CrewAI、AutoGen（已并入 Microsoft Agent Framework）、LangGraph
  2. 可视化编排平台：Dify、Coze（扣子）、n8n、Airflow
  3. 专家/技能市场：Coze 商店、Dify 插件市场、Microsoft Copilot Studio（连接器/MCP）
  4. 企业级 Agent 平台：Microsoft Copilot Studio、钉钉 AI、飞书 aily
- **判定符号**：✅ 有且成熟 / 🟡 部分或受限 / 🔴 无或明显弱。
- **数据来源与日期**：竞品事实核证于 2026-09-29；版本号、价格、认证等时效信息随厂商迭代变化，引用时以官网为准。我方事实为 2026-09-24~29 代码实测。

---

## 二、竞品逐个分析

### 2.1 多智能体框架

#### 2.1.1 CrewAI

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 开源多智能体编排框架，双层架构 Flows（确定性长流程）+ Crews（多 Agent 协作）；配套企业平台 CrewAI AMP（Agent Management Platform）做生产部署/监控/扩缩 | https://docs.crewai.com/en/enterprise/introduction ; https://blog.crewai.com/orchestrating-self-evolving-agents-with-crewai-and-nvidia-nemoclaw/ |
| 核心能力 | Agent/Crew/Flow 抽象；guardrails、memory、knowledge、observability 内建；Flows 支持 HITL（人工介入）；a2a 协议、异步链；当前线版本约 v1.15.x（v1.15.11 发布于 2026-08） | https://docs.crewai.com/v1.15.15/en/changelog ; https://docs.crewai.com/v1.15.2/en/changelog.md |
| 企业级能力 | AMP 企业版：SSO、RBAC、组织级管控；Tool Repository（组织内发布/共享工具、从 registry 装社区工具）；Factory 自托管（在自有基础设施跑 AMP，数据驻留与合规控制）；云/VPC/自有基础设施三态部署；PII 脱敏、策略治理 | https://docs.crewai.com/v1.15.0/en/guides/coding-tools/build-with-ai ; https://crewai.com/pricing |
| 生态 | 工具 registry（社区 + 组织内发布）；与 NVIDIA NemoClaw 等集成；生态以开发者社区为主，无面向终端业务人员的可视化市场 | https://docs.crewai.com/v1.15.0/en/guides/coding-tools/build-with-ai |
| 我们强 | ① 内建**专家关系知识图谱 + 最优团队推荐**（graph_nodes/edges/meta + optimal-team），CrewAI 无专家关系图，靠 prompt 约定角色分工；② 国密 SM4-GCM 全链路加密一键开关（`MOX_API_CRYPTO=sm4`），CrewAI 企业版未见国密合规项；③ 嵌入式 SQLite 零外部依赖即可跑全链路，CrewAI AMP 自托管仍需配套基础设施 |
| 我们弱 | ① CrewAI 有成熟的 **HITL（Flows 人工介入）**，我们 DAG 无人工审批节点（仅有 booking 预约，见 08 矩阵 #14/N 系列）；② 其 AMP 提供**部署-监控-扩缩一体化控制台**，我们仅有 SSE 日志流 + JSON 指标，无 tracing/eval 控制台；③ Tool Repository 的组织内工具共享机制我们尚无可插拔工具注册市场 |
| 借鉴点 | ① 为 DAG 增加**人工审批/介入节点**（HITL），对齐其 Flows HITL；② 规划 tracing/运行监控面板（即使先做轻量 trace，而非从零造 LangSmith）；③ 设计可插拔工具/专家的注册-共享协议（先内部 registry，后市场） |

#### 2.1.2 AutoGen → Microsoft Agent Framework（MAF）

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 微软将 AutoGen 与 Semantic Kernel 合并为统一的 **Microsoft Agent Framework（MAF）**，开源 SDK + 运行时，.NET 与 Python 双语言同 API | https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-version-1-0/ |
| 核心能力 | Agent、Harness Agent（内建规划/todo、上下文压缩、文件访问、记忆、工具审批、可观测）、functional 与 graph 两种 Workflow；稳定编排模式：Sequential、concurrent、group chat、handoff、magentic；多 Provider 模型；A2A 与 MCP 跨运行时互操作 | https://learn.microsoft.com/fil-ph/agent-framework/overview/ ; https://devblogs.microsoft.com/agent-framework/category/announcement/ |
| 里程碑 | MAF 1.0 GA 于 2026-04-02（.NET/Python），承诺长期支持；Build 2026 推出 Agent Harness、Hosted Agents、CodeAct；Azure AI Foundry Agent Service 已 GA | https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-version-1-0/ ; https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-at-build-2026-announce/ ; https://news.microsoft.com/build-2025-book-of-news/ |
| 企业级能力 | 托管于 Azure AI Foundry；Foundry hosted agent 隔离（用户隔离 + 会话隔离）；提示注入防护；与 Entra/Power Platform 治理打通；自托管可跑在 AKS/Container Apps/App Service 等 .NET 宿主 | https://devblogs.microsoft.com/agent-framework/ ; https://www.langchain.com/resources/langchain-vs-autogen |
| 生态 | MCP/A2A 开放协议；深度绑定微软云与 Copilot 生态；非微软技术栈采纳门槛高 | https://news.microsoft.com/build-2025-book-of-news/ |
| 我们强 | ① 我们是**可完全内网气隙部署、不绑定公有云**的 Rust 实现，MAF 生产形态深度依赖 Azure Foundry；② 国密 SM4 与嵌入式存储更贴合政务信创内网；③ 我们把多专家协作模式（辩论/投票/层级）与结果融合（6 策略）做成开箱内核，MAF 是 SDK 需开发者自行拼装 |
| 我们弱 | ① MAF 1.0 双语言 SDK + 长期支持承诺，工程成熟度与生态远高于我们；② **MCP/A2A 互操作协议**已稳定，我们目标态才规划 MCP（P3，08 §十）；③ Harness Agent 的上下文压缩、工具审批、记忆等电池全含，我们 memory 仍为网关内联（P2 独立 memory-svc） |
| 借鉴点 | ① 优先落地 **MCP 协议接入**（我们 08 §十已列 P3，建议提前），以获得与外部 Agent 生态互操作；② 借鉴 Harness Agent 的「上下文压缩 + 工具审批」设计，补长任务内存管理；③ 编排模式命名可对齐业界稳定范式（group chat/handoff），降低用户认知 |

#### 2.1.3 LangGraph（LangChain）

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | LangChain 旗下有状态多智能体编排运行时，用 graph 状态机表达复杂控制流与人机协同；配套 Deep Agents（agent harness：任务规划、子 Agent 派生、长期记忆、上下文管理）与 LangSmith（可观测/评测） | https://www.langchain.com/blog/nvidia-enterprise ; https://www.langchain.com/resources/ai-agent-frameworks |
| 里程碑 | LangGraph 1.0 GA 于 2025-10；Deep Agents 持续迭代（v0.5 异步子 Agent、v0.6 harness 画像/流式） | https://www.langchain.com/resources/langchain-vs-autogen ; https://www.langchain.com/blog/deep-agents-v0-5 |
| 企业级/部署 | LangSmith Deployment 一键部署、持久执行、任务队列、状态持久化、水平扩缩；可**自托管 standalone Agent Server**（Docker/Compose/K8s，无控制面也能跑，trace 发往自托管或云 LangSmith）；LangSmith trace 采用对象存储、无状态，便于自托管/多云 | https://docs.langchain.com/langsmith/deploy-standalone-server ; https://www.langchain.com/blog/interrupt-2026-overview |
| 生态 | LangChain 组件生态庞大；LangSmith 做 tracing/eval/调试；沙箱提供商可插拔（Daytona/Modal/Runloop） | https://www.langchain.com/blog/runtime-behind-production-deep-agents |
| 我们强 | ① LangGraph 的「graph」是**执行状态机**，不是专家关系知识图谱；我们的专家能力图谱 + 最优组队是差异化；② 我们开箱即有辩论/投票等协作模式与 6 种融合策略，LangGraph 需手写节点与边；③ 国密 + 嵌入式存储更适合纯内网 |
| 我们弱 | ① **可观测与评测（LangSmith 级 tracing/eval）是我们最大短板**：我们 scheduler/executor `/metrics` 还是 JSON 快照（08 N7），无 run 级 trace、无离线评测；② 持久执行/断点续跑（durable execution）我们仅 sqlite 模式部分具备，LangSmith Deployment 是一等公民；③ 子 Agent 异步派生（Deep Agents v0.5）我们 Dynamic 模式刚起步 |
| 借鉴点 | ① **最高优先级补 tracing/可观测**：先做 run 级结构化事件 + 时间线视图（哪怕不自建 eval 平台）；② 持久执行与断点续跑对齐 durable execution；③ 沙箱/工具隔离可借鉴其「单一配置切换沙箱提供商」的抽象 |

---

### 2.2 可视化编排平台

#### 2.2.1 Dify

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 开源 LLMOps/Agentic Workflow 平台：可视化工作流编排 + Prompt IDE + RAG + Agent + 可观测，一个可自托管平台从原型到生产；GitHub 157K+ star | https://dify.ai/zh ; https://agenticindex.io/vendors/dify |
| 核心能力 | 可视化拖拽工作流构建器；RAG 知识库（30+ 向量库，chunk+embedding+向量检索）；应用一键发布为 API；Dify 1.12.0 推 Summary Index（轻量替代复杂 GraphRAG），原生 RAG 基于向量，**不把 Neo4j 等图数据库作为替换选项** | https://deepwiki.com/langgenius/dify-docs/8-knowledge-base-and-rag-system ; https://dify.ai/blog/dify-1.12.0-summary-index-from-fragmented-retrieval-to-full-context ; https://qiita.com/kanataken/items/c4b10cdb9bdb560a8bdb |
| 企业级能力 | 企业版：自托管/VPC；SSO/SAML、RBAC、审计日志；SOC 2 Type II + ISO 27001；Helm Chart for K8s、Terraform、**气隙（air-gapped）部署**；BYOK 加密、数据驻留、模型训练不泄漏；**多租户隔离**（一套集群多业务单元，租户级配额/计费/访问策略） | https://dify.ai/zh ; https://dify.ai/dify-enterprise |
| 生态 | Dify Marketplace 插件生态（2026-08 起建立插件治理：准入、可达范围、版本审查、证据变更处置）；Creator Center + Template Marketplace（工作流模板一键采用、分成） | https://dify.ai/blog/trust-is-a-feature-how-dify-is-governing-a-growing-plugin-ecosystem ; https://dify.ai/blog |
| 我们强 | ① 我们有**专家关系知识图谱 + 最优组队**，Dify 知识库是文档向量 RAG，不建模专家/能力关系；② 国密 SM4-GCM 全链路是 Dify 未提供的政务合规项；③ 我们多专家辩论/投票/层级协作与结果融合是内核能力，Dify 以工作流节点串联为主，无内建辩论/投票语义 |
| 我们弱 | ① **可视化拖拽编排画布是我们明显短板**：我们是固定步骤表 + Kahn 拓扑（08 矩阵 #7/#13），业务人员无法画 DAG；② 多租户、SSO/SAML、租户配额我们完全没有；③ 插件市场与模板生态成熟度差距大；④ 企业版合规认证（SOC2/ISO27001）我们没有 |
| 借鉴点 | ① **补可视化编排画布**（拖拽节点/边，导出为我们的 DAG）——这是面向业务方最大的体验差距；② 抽象工作区/租户模型，为多组织隔离预留；③ 插件/工具准入治理机制（参考其插件信任治理） |

#### 2.2.2 Coze（扣子）

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 字节跳动/火山引擎旗下，低代码智能体开发平台（扣子编程）+ 职场 AI 伙伴（扣子）；零基础业务人员可搭 Agent、工作流、插件、卡片 | https://www.volcengine.com/product/coze-pro ; https://coze.cn |
| 核心能力 | 可视化搭 Agent；插件、工作流、知识库；多人多 Agent 同空间协作；技能一次配置全员共享；支持把本地 Agent（Claude Code/Codex CLI/Hermes/OpenClaw）接入扣子 | https://coze.cn ; https://docs.coze.cn/cozespace_local_agent |
| 企业级能力 | 企业版：SSO 单点登录、VPC 内网连接；私网模式插件安全访问内网服务；自定义密钥对会话数据加密；企业插件商店（可设「仅允许企业插件」）；企业商店应用仅本组织可见；对接火山引擎 IAM 成员权限 | https://docs.coze.cn/guides_edition ; https://docs.coze.cn/recent-updates ; https://docs.coze.cn/coze_pro_premium_package_faq ; https://docs.coze.cn/guides_set_pro_subusers |
| 部署形态 | 商业扣子企业版为**云 SaaS + VPC 连接**；另开源 Coze Studio/Coze Loop（含 Agent、对话、工作流、API/SDK、观测、评测），可本地/服务器 Docker 自部署（建议 4C8G+），veFaaS 支持云上一键私有化 | https://www.byteplus.com/en/activity/coze-dev/ve ; https://www.volcengine.com/docs/6662/1756919 |
| 生态 | 插件商店、技能商店、企业插件商店；OpenAPI/Webhook/Stream 生态 | https://docs.coze.cn/cozespace_agent_management ; https://www.volcengine.com/product/coze-pro |
| 我们强 | ① 专家关系图谱 + 最优组队；② 国密 SM4；③ 我们是完整独立可气隙的后端系统，商业 Coze 企业版依赖火山云（VPC 连接≠气隙内网） |
| 我们弱 | ① **低代码可视化体验与插件/技能商店**成熟度差距大；② 企业 SSO/IAM/会话加密等企业治理项我们未做；③ 开源版自带观测与评测（Coze Loop），我们缺；④ 面向业务人员的「全员开发」范式我们没有产品化 |
| 借鉴点 | ① 「企业插件商店 + 仅允许企业插件」的管控模式可直接借鉴为我们专家/工具的白名单治理；② 私网模式插件（安全访问内网服务）的设计；③ 本地 Agent 接入统一入口的产品形态 |

#### 2.2.3 n8n

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 开源 fair-code 的 AI 工作流自动化平台，可视化连接应用与 AI Agent；强调数据完全自有、可审计源码、完全自托管 | https://n8n.io/ ; https://n8n.io/enterprise/ |
| 核心能力 | 可视化工作流画布；AI Agent 节点；400+ 应用集成；Git 版本控制；Secret 加密存储 | https://n8n.io/ |
| 企业级能力 | SSO/SAML、LDAP、用户 provisioning、IdP 组→实例/项目角色映射、2FA、组织级共享策略；RBAC 权限；**审计日志 + 日志流送到 SIEM**；工作流历史、实时告警、用量看板；完全 on-prem | https://n8n.io/enterprise/ ; https://n8n.io/ |
| 生态 | 庞大集成节点社区 + 自定义节点；无独立「专家市场」，以集成连接器生态为主 | https://n8n.io/ |
| 我们强 | ① 我们聚焦**多专家协作推理**（辩论/投票/融合），n8n 偏系统集成自动化，无多智能体协作语义与结果融合；② 专家关系图谱；③ 国密 SM4；④ 我们的 registry 10:1:1 分级心跳聚合面向大规模专家注册，n8n 无此概念 |
| 我们弱 | ① **企业安全治理套件成熟**（SSO/SAML/LDAP/2FA/RBAC/审计送 SIEM），我们仅网关 JWT + 哈希链审计，且下游 svc 无 JWT（08 N1）、审计 Actor 硬编码 system（N2）；② 可视化画布；③ Git 版本控制工作流；④ 400+ 连接器生态 |
| 借鉴点 | ① **审计日志送 SIEM + 工作流历史 + 实时告警**——我们审计已是哈希链 NDJSON，补「送 SIEM + 真实用户 Actor」即可对齐；② IdP 组→角色映射（SSO/LDAP）；③ Secret 加密存储与版本控制 |

#### 2.2.4 Apache Airflow

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 老牌开源**数据/任务 DAG 编排调度器**（偏数据工程，非 AI 多智能体）；Airflow 3.0 GA 转向面向服务架构、事件驱动与 ML 工作流、现代化 React UI | https://airflow.apache.org/docs/apache-airflow/3.1.1/release_notes.html |
| 里程碑 | 3.3.2（2026-09-17）、3.3.0（2026-07-06）、3.2.0（2026-04-07）；3.2 多团队隔离（实验）；3.3 任务/资产状态存储（AIP-103）、Java/Go Task SDK（AIP-108）、可插拔重试策略 | https://airflow.apache.org/docs/apache-airflow/stable/release_notes.html ; https://airflow.apache.org/blog/airflow-3.2.0/ ; https://airflow.apache.org/blog/tags/release/ |
| 企业级能力 | RBAC；多团队隔离（一套部署内各团队独立 DAG/连接/变量/池/executor，资源与权限隔离）；Helm Chart；商业发行版 Astronomer 支持 Dag 级角色最小权限 | https://airflow.apache.org/blog/airflow-3.2.0/ ; https://www.astronomer.io/docs/astro/release-notes/ |
| 生态 | 海量 Operator/Provider 生态；现已出现 MCP server（如 Cloud Composer 远程 MCP server 预览）供 AI 应用调用 | https://docs.cloud.google.com/composer/docs/release-notes?authuser=31 |
| 我们强 | ① Airflow **不是 Agent 框架**：无 LLM 驱动的多专家协作、无辩论/投票/融合，它是确定性任务调度；我们定位更贴近 AI 协作；② 我们有专家能力图谱与 LLM 路由，Airflow 无；③ 国密与嵌入式轻量部署更适合边缘内网 |
| 我们弱 | ① Airflow 在**DAG 调度的可靠性、可重放、补数（backfill）、任务级状态存储**上极其成熟；我们 DAG 无并行度信号量上限（08 N8）、无补数/重跑语义；② 多团队/多租户隔离我们没有；③ 生态 Operator 数量级差距；④ 任务 SDK 多语言（Java/Go）我们仅 Rust |
| 借鉴点 | ① DAG 引擎补**并行度信号量 + 失败重试/补跑/重放**（我们已有节点重试3次+退避，缺跨任务重放与并行限流 N8）；② 任务级持久状态存储（我们 sqlite 模式已部分具备，对标 AIP-103 做一等公民）；③ 多团队资源隔离模型 |

---

### 2.3 专家 / 技能市场

> 本节市场类竞品与 2.2/2.4 有重叠，此处聚焦其「市场/生态」这一维度。

#### 2.3.1 Coze 商店（插件商店 / 技能商店 / 企业插件商店）

| 项 | 内容 | 来源 |
|---|---|---|
| 形态 | 面向 Agent 的插件/技能市场：官方插件 + 企业自定义插件入企业插件商店；可强制「仅允许企业插件商店内插件」；企业商店应用仅本组织可见 | https://docs.coze.cn/recent-updates ; https://docs.coze.cn/cozespace_agent_management |
| 治理 | 企业版支持私网模式插件访问内部服务；自定义密钥加密会话数据 | https://docs.coze.cn/coze_pro_premium_package_faq |
| 我们强 | 我们专家是**带能力声明、可被图谱关联与匹配的一等实体**（skills/capabilities、模块化权重匹配），Coze 插件偏工具调用单元，不建专家关系网络 |
| 我们弱 | 我们**没有对外/对内的插件/技能市场与分发、版本、白名单治理**；专家是内置种子 + 本地 CRUD，无生态 |
| 借鉴点 | ① 「企业插件商店 + 仅允许企业插件」的白名单管控；② 插件版本与凭据按工作区管理；③ 技能/工具的一键安装到 Agent |

#### 2.3.2 Dify 插件市场（Marketplace）

| 项 | 内容 | 来源 |
|---|---|---|
| 形态 | Dify Marketplace：插件市场 + Creator Center + Template Marketplace（工作流模板一键采用、可选分成） | https://dify.ai/blog |
| 治理 | 2026-08 起建立插件信任治理：包准入、插件可达范围、版本审查、证据变更处置 | https://dify.ai/blog/trust-is-a-feature-how-dify-is-governing-a-growing-plugin-ecosystem |
| 我们强 | 我们的专家注册中心（registry-svc + 10:1:1 聚合）为「专家注册与发现」打了底，比 Dify 插件更接近「服务化专家」 |
| 我们弱 | 无市场、无模板复用、无创作者/分发机制 |
| 借鉴点 | ① 把我们的专家/工作流沉淀为可分享模板；② 插件/工具准入与版本治理清单 |

#### 2.3.3 Microsoft Copilot Studio（连接器 / MCP 生态）

| 项 | 内容 | 来源 |
|---|---|---|
| 形态 | Agent 可接业务数据、调 API、编排工作流；连接器治理（管理员控制 Agent 可连哪些系统）；2026-07 起每个新 Agent 自动分配 Entra Agent ID；支持把 workflow/MCP server 作为工具，MCP server 可提交微软认证 | https://learn.microsoft.com/en-us/microsoft-copilot-studio/whats-new ; https://learn.microsoft.com/de-de/microsoft-365/copilot/extensibility/copilot-studio-experience |
| 治理 | DLP、环境级 DLP/RBAC/监控、Information Barriers、敏感信息不跨 Agent 转发、URL 治理 | https://learn.microsoft.com/zh-cn/microsoft-copilot-studio/security-and-governance ; https://learn.microsoft.com/sv-se/power-platform/released-versions/copilotstudio/2026.5.5 |
| 我们强 | 我们可纯内网气隙，Copilot Studio 是 Power Platform 云 SaaS；国密 SM4 为其不具备 |
| 我们弱 | 治理粒度（DLP/Information Barriers/连接器白名单/Agent 身份）与 MCP 认证生态我们远未达到 |
| 借鉴点 | ① 为每个专家/Agent 分配可审计身份（对标 Entra Agent ID）；② 连接器/工具白名单治理；③ DLP 式敏感数据出口管控 |

---

### 2.4 企业级 Agent 平台

#### 2.4.1 Microsoft Copilot Studio（企业平台视角）

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 微软 Power Platform 上的企业级 Agent 构建与管理平台：自然语言建 Agent、连业务数据、发布到 Teams/网站等渠道；通过 Power Platform 管理中心统一治理 | https://www.microsoft.com/en/microsoft-copilot/microsoft-copilot-studio |
| 核心能力 | 经典编排 vs Generative 编排可选；自主（trigger-based）/ 对话式 Agent；智能工作流；连外部系统；GitHub Copilot harness + Microsoft IQ 接企业数据 | https://adoption.microsoft.com/files/copilot-studio/Microsoft-Copilot-Studio_Governance-and-security-guide.pdf ; https://learn.microsoft.com/gl-es/microsoft-copilot-studio/whats-new |
| 企业级能力 | 地理数据驻留、DLP、多项合规认证、环境路由、区域自定义；专用开发环境、Agent 生命周期管理；环境级 DLP/RBAC/监控；Information Barriers；Entra Agent ID | https://learn.microsoft.com/zh-cn/microsoft-copilot-studio/security-and-governance ; https://www.microsoft.com/en/microsoft-copilot/microsoft-copilot-studio |
| 部署 | 云 SaaS（Power Platform），含 Azure 美政云等政府云；**不提供传统意义的客户内网气隙自托管**（未核：是否有 fully on-prem 版本） | https://learn.microsoft.com/sv-se/power-platform/released-versions/copilotstudio/2026.5.3 |
| 我们强 | ① 纯内网气隙 + 国密 SM4，契合政务等保内网；② 专家关系图谱；③ 无云绑定 |
| 我们弱 | 治理/合规/渠道分发/生命周期管理成熟度差距巨大；无 SSO/DLP/环境隔离 |
| 借鉴点 | ① Agent 生命周期管理（开发环境→发布→共享→退役）；② 环境隔离与发布管控；③ 渠道分发（IM/网站/API） |

#### 2.4.2 钉钉 AI（DEAP / 炼丹炉 / 悟空）

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 钉钉企业内 AI 办公与 Agent 体系：AI 助理、知识集、炼丹炉企业大模型平台（基于通义千问的训练/微调/部署）；2026-03 阿里发布企业级 Agent 平台「悟空」，原生调用钉钉账号/权限/应用体系 | https://m.dingtalk.com/ ; https://open.dingtalk.com/document/aipass/introduction-of-large-model-platform-of-dingtalk-blast-furnace-1 ; https://36kr.com/p/3726501820807554 |
| 核心能力 | 知识集（文档/网页/术语/FAQ，按归属部门与可用人员范围）；智能体按部门/人员授权、共同管理员、全链路 trace 调试；悟空以 skills 形式接入集团企业能力，在授权范围内调用而非模拟点击 | https://open.dingtalk.com/document/aipass/automatically-generate-an-ai-assistant-1 ; https://open.dingtalk.com/document/aipass/distribution-of-ai-assistant-1 ; https://36kr.com/p/3726501820807554 |
| 企业级能力 | 2700万+ 企业组织、7亿用户、等保三级、数据全链路加密；接入企业账号/权限体系 | https://m.dingtalk.com/ |
| 部署 | 商业钉钉为云 SaaS；专有/私有化部署面向政企存在但 AI 能力是否随私有化完整交付**未核** | 未核 |
| 我们强 | ① 我们是可嵌入任意政务系统的独立后端，不绑定钉钉 IM；② 专家关系图谱 + 多专家协作内核；③ 国密 |
| 我们弱 | ① 钉钉直接复用企业**账号/组织/权限体系**做授权，我们下游 svc 无 JWT（N1）、审计无用户身份（N2）；② 等保三级是其既有资质，我们在申报路径上；③ 海量用户与组织治理沉淀 |
| 借鉴点 | ① Agent 授权**复用企业既有组织/权限体系**（对接 SSO/组织架构），而非自建账号；② 知识/智能体按部门/人员范围授权；③ 全链路 trace 调试 |

#### 2.4.3 飞书 aily（智能伙伴）

| 项 | 内容 | 来源 |
|---|---|---|
| 定位 | 飞书企业工作场景通用智能体平台：自定义智能体（角色/提示词 + 知识空间 + 工具/API）；aily 工作助手任务模式支持任务拆解、专属电脑、调工具完成复杂任务 | https://www.feishu.cn/hc/zh-CN/articles/790732948604 ; https://www.feishu.cn/content/article/7585126677299137754 |
| 核心能力 | 知识空间（结构化+非结构化，可见范围控制，直连知识额外校验用户文档可搜权限）；MCP 市场（MCP 服务注册、企业内流通、被智能体调用）；代理通道直连企业内网系统 | https://aily.feishu.cn/hc/1u7kleqg/t8gwpqxg ; https://bytedance.larkoffice.com/wiki/QqdPwGhHuicn12km20hcNeU0nHd ; https://aily.feishu.cn/hc/1u7kleqg/ef4valfx |
| 企业级能力 | 表/行/列动态权限 + 知识库权限隔离；**记忆隔离**（每人/每群独立记忆物理隔离）；**中心化凭证**（Agent 不持有密钥，系统统一保管）；多渠道发布（IM/服务台/官网/API） | https://www.feishu.cn/paid/ai ; https://aily.feishu.cn/hc/1u7kleqg/ef4valfx |
| 部署 | 绑定飞书生态的 SaaS；私有化部署能力**未核** | 未核 |
| 我们强 | ① 专家关系图谱 + 多专家协作/融合内核；② 可独立气隙部署、不绑飞书；③ 国密 |
| 我们弱 | ① **记忆隔离 + 中心化凭证**的安全设计很扎实，我们 memory 尚在网关内联、无每用户隔离与密钥托管；② 知识空间的细粒度权限（表/行/列 + 文档级可搜权限校验）我们图谱节点级都还不能 CRUD（N4）；③ MCP 市场我们未做 |
| 借鉴点 | ① **中心化凭证托管**（Agent 不持有下游密钥）——直接对齐我们 llm_router/专家调用的密钥管理；② 每用户/每群记忆隔离；③ 知识访问做「用户级权限二次校验」而非 Agent 全量可见 |

---

## 三、横向对标矩阵

> 评级：✅ 有且成熟｜🟡 部分/受限/默认关闭｜🔴 无或明显弱。「我们 vs 该行」一句话给出相对位置。

| 竞品 | 多智能体编排 | 知识图谱 | 可视化编排 | 权限/审计 | 私有化/气隙部署 | 生态/市场 | 我们 vs 该行 |
|---|---|---|---|---|---|---|---|
| **专家联盟（我方）** | ✅ 7 协作模式 + 6 融合策略（开箱内核） | ✅ 专家关系图 + 最优组队（graph_nodes/edges/meta） | 🔴 固定步骤表+Kahn，无拖拽画布 | 🟡 网关 JWT + 哈希链审计；下游无 JWT、审计 Actor=system、无 SSO/多租户（N1/N2/G3） | ✅ 嵌入式 SQLite 零外部依赖，可气隙；SM4 国密 | 🔴 无插件/技能市场，内置专家 | —（基线） |
| CrewAI | ✅ Crews+Flows | 🔴 无专家关系图 | 🟡 AMP 控制台 | 🟡 SSO/RBAC（企业版） | ✅ Factory 自托管/VPC | 🟡 工具 registry | 我方：图谱+国密+气隙轻量占优；HITL/可观测/市场落后 |
| AutoGen→MAF | ✅ 多模式稳定（.NET/Python） | 🔴 无 | 🔴 代码 SDK | 🟡 Azure Foundry 治理/Entra | 🟡 自托管 .NET 宿主但绑云 | 🟡 MCP/A2A | 我方：气隙独立+开箱协作占优；协议/工程成熟度落后 |
| LangGraph | ✅ 有状态图编排 | 🔴 graph=状态机非专家图 | 🟡 LangSmith | 🟡 LangSmith 自托管 | ✅ standalone server Docker/K8s | 🟡 LangChain 生态 | 我方：专家图谱+融合占优；tracing/eval/持久执行明显落后 |
| Dify | 🟡 工作流+Agent（弱辩论语义） | 🔴 向量 RAG（无原生图） | ✅ 拖拽画布 | ✅ SSO/SAML/RBAC/审计/多租户 | ✅ 自托管+气隙+Helm | ✅ Marketplace+模板 | 我方：专家图谱+国密+协作内核占优；可视化/多租户/市场/合规认证落后 |
| Coze 扣子 | 🟡 多 Agent 空间协作 | 🔴 文档知识库 | ✅ 低代码 | ✅ SSO/VPC/IAM/企业插件管控 | 🟡 云 SaaS+VPC；开源版可本地 | ✅ 插件/技能商店 | 我方：独立气隙后端+图谱占优；低代码体验/企业治理/市场落后 |
| n8n | 🟡 AI 节点（集成导向） | 🔴 无 | ✅ 可视化画布 | ✅ SSO/LDAP/2FA/RBAC/审计送 SIEM | ✅ 完全 on-prem | ✅ 400+ 连接器 | 我方：多专家协作推理+图谱占优；安全治理套件/画布/集成生态落后 |
| Airflow | 🔴 数据 DAG，非 Agent | 🔴 无 | 🟡 UI 但代码定义 DAG | ✅ RBAC/多团队隔离 | ✅ 自托管+Helm | ✅ 海量 Operator | 我方：定位（AI 协作 vs 数据调度）不同；调度可靠性/重放/并行限流落后 |
| Copilot Studio | 🟡 编排/自主/对话 Agent | 🔴 无 | ✅ 低代码 | ✅ DLP/Entra/信息屏障/环境治理 | 🔴 云 SaaS（政府云；气隙未核） | ✅ 连接器+MCP 认证 | 我方：气隙+国密占优；企业治理/渠道/生命周期管理落后 |
| 钉钉 AI | 🟡 悟空 skills | 🔴 无 | 🟡 配置式 | ✅ 复用组织权限/等保三级 | 🟡 云 SaaS；私有化 AI 未核 | 🟡 钉钉生态/炼丹炉 | 我方：独立可嵌入+图谱占优；组织权限复用/等保资质落后 |
| 飞书 aily | 🟡 任务模式 Agent | 🔴 知识空间文档 | 🟡 配置式 | ✅ 行列权限/记忆隔离/中心化凭证 | 🔴 绑定飞书 SaaS（私有化未核） | 🟡 MCP 市场 | 我方：独立部署+多专家融合占优；凭证托管/记忆隔离/细粒度知识权限落后 |

---

## 四、对标结论与差距清单

### 4.1 我们强（可当卖点，附依据）

1. **唯一带「专家关系知识图谱 + 最优组队」的多智能体内核。** 主流竞品（Dify/Coze 文档向量 RAG、CrewAI/LangGraph 代码编排、Copilot/钉钉/飞书）均不建模专家/能力关系；我方有 graph_nodes/edges/meta 表 + experts_graph.rs + optimal-team（08 矩阵 #10）。这是与「向量 RAG」路线的本质差异。
2. **国密 SM4-GCM 全链路加密一键开关。** 我方 `MOX_API_CRYPTO=sm4` 在 6 个挂载点全链路 gzip+SM4-GCM（09 §2.1、08 #21）；竞品仅见 BYOK/加密 Secret/DLP，无国密合规项。这是政务等保/信创场景的硬卖点。
3. **零外部依赖的气隙内网可部署性。** 我方嵌入式 SQLite、无 Redis/PostgreSQL/pgvector 依赖即可跑全链路（CURRENT §5），4 容器 compose / k8s 模板齐备（09 §三/§四）；Dify/n8n 需 PG/Redis/向量库，Copilot/钉钉/飞书为云 SaaS。
4. **开箱即用的多专家协作模式与结果融合。** 7 种协作模式（Sequential/Parallel/Debate/Hierarchical/Iterative/Voting/Dynamic）+ 6 种融合策略（08 #8/#9），DAG 引擎拓扑并行执行；CrewAI/LangGraph 需自行拼装，Airflow 无 LLM 协作语义。
5. **面向大规模专家注册的分级心跳聚合 + HA 选主。** registry 10:1:1 分级聚合（入流降 100 倍）+ 租约选主/fencing（08 #16/#18）；竞品无此专家注册规模设计。

### 4.2 我们弱（需追赶，附借鉴路径）

1. **无可视化拖拽编排画布。** 现状为固定步骤表 + Kahn 拓扑（08 #7/#13），业务人员无法画 DAG。借鉴 Dify/Coze/n8n：补拖拽节点-边画布，导出为我方 DAG。
2. **无可观测/评测/tracing 控制台。** scheduler/executor `/metrics` 还是 JSON（08 N7），无 run 级 trace、无 eval。借鉴 LangSmith：先做 run 级结构化事件 + 时间线视图。
3. **企业治理三件套缺失：SSO/多租户/真实用户审计。** 下游三 svc 无 JWT（N1）、审计 Actor 硬编码 system（N2）、前端无角色/按钮级权限（G3）、无 SSO/多工作区。借鉴 n8n/Dify：补 JWT 到下游、审计注入真实用户、SSO/SAML、工作区/租户隔离。
4. **无插件/技能市场与工具分发生态。** 专家为内置种子 + 本地 CRUD。借鉴 Coze 企业插件商店/Dify Marketplace：先做工具/专家注册-共享-白名单协议，再谈市场。
5. **无 HITL 人工审批节点。** 仅有 booking 预约，DAG 无人工介入分支。借鉴 CrewAI Flows HITL / Copilot：为 DAG 加审批/人工确认节点。
6. **无中心化凭证托管与记忆隔离。** 专家调用密钥分散、memory 网关内联无每用户隔离。借鉴飞书 aily：Agent 不持密钥、系统统一托管；每用户/每群记忆隔离。
7. **DAG 工程化能力不足。** 无并行度信号量上限（N8）、无补数/重放/schema 版本迁移（N3）。借鉴 Airflow：Semaphore 限流、任务重放、`PRAGMA user_version` 迁移。
8. **开放协议滞后。** 尚无 MCP/A2A（08 §十列 P3），而 MAF/Copilot/飞书/钉钉已支持或认证 MCP。建议把 MCP 接入从 P3 提前。**（2026-10-01 进展：MCP Server 已落地——stdio 自实现 + 3 个真实工具 `expert_search/optimal_team/graph_expand`，读面已可被外部 Agent 调用；A2A、MCP Client 与中心化凭证托管仍留待。）**

### 4.3 总体判断

- **已具企业级竞争力的场景**：政务/企业**内网气隙、等保/信创、国密合规、多专家协作推理**的场景。在这些场景下，我方的「嵌入式零依赖 + SM4 国密 + 专家关系图谱 + 开箱协作融合」组合，是 Dify/Coze（需外部依赖/云绑定）、Copilot/钉钉/飞书（云 SaaS）、CrewAI/LangGraph（代码框架无治理）都不能同时满足的。
- **必须补的短板**：面向**业务人员的可视化编排**与**企业治理（SSO/RBAC/多租户/真实审计/DLP）**、以及**可观测评测与 MCP 生态**。这三块不补，我方只能停留在「技术团队自建的内网后端」，无法成为业务人员自助、可被企业 IT 统一治理的平台。
- **差异化路线**：不必与 Dify/Coze 比「通用低代码 + 插件市场规模」，而应把「专家知识图谱 + 国密气隙 + 多专家协作融合」做深，作为信创政务场景的垂直竞争力，再补可视化与治理以扩大受众。

---

## 五、竞品核证来源清单

> 核证日期均为 2026-09-29（除注明外）。

| 竞品 | 事实 | URL | 核证日期 |
|---|---|---|---|
| CrewAI | AMP 生产部署/监控/扩缩平台 | https://docs.crewai.com/en/enterprise/introduction | 2026-09 |
| CrewAI | SSO/RBAC/组织管控、Tool Repository、Factory 自托管 | https://docs.crewai.com/v1.15.0/en/guides/coding-tools/build-with-ai | 2026-09 |
| CrewAI | 云/VPC/自有基础设施部署、PII 脱敏/策略 | https://crewai.com/pricing | 2026-09 |
| CrewAI | v1.15.11（2026-08）版本迭代 | https://docs.crewai.com/v1.15.15/en/changelog | 2026-09 |
| CrewAI | Flows+Crews 双层架构、HITL | https://blog.crewai.com/orchestrating-self-evolving-agents-with-crewai-and-nvidia-nemoclaw/ | 2026-09 |
| AutoGen/MAF | MAF 1.0 GA（2026-04-02），AutoGen+Semantic Kernel 合并，.NET/Python，A2A/MCP | https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-version-1-0/ | 2026-09 |
| AutoGen/MAF | Build 2026：Agent Harness/Hosted Agents/CodeAct | https://devblogs.microsoft.com/agent-framework/microsoft-agent-framework-at-build-2026-announce/ | 2026-09 |
| AutoGen/MAF | 编排模式稳定（sequential/concurrent/group chat/handoff/magentic） | https://devblogs.microsoft.com/agent-framework/category/announcement/ | 2026-09 |
| AutoGen/MAF | Agent/Harness Agent/Workflow 抽象、多 Provider | https://learn.microsoft.com/fil-ph/agent-framework/overview/ | 2026-09 |
| AutoGen/MAF | Azure AI Foundry Agent Service GA、A2A/MCP | https://news.microsoft.com/build-2025-book-of-news/ | 2026-09 |
| LangGraph | LangGraph 1.0 GA（2025-10）、LangSmith Deployment 一键/持久执行 | https://www.langchain.com/resources/langchain-vs-autogen | 2026-09 |
| LangGraph | 有状态多智能体编排/HITL、Deep Agents harness | https://www.langchain.com/blog/nvidia-enterprise | 2026-09 |
| LangGraph | standalone Agent Server 自托管（Docker/Compose/K8s） | https://docs.langchain.com/langsmith/deploy-standalone-server | 2026-09 |
| LangGraph | LangSmith 对象存储无状态、易自托管/多云 | https://www.langchain.com/blog/interrupt-2026-overview | 2026-09 |
| Dify | 自托管企业版、SSO/SAML/RBAC/审计、SOC2/ISO27001、Helm、157K star | https://dify.ai/zh | 2026-09 |
| Dify | 气隙部署/BYOK/多租户隔离/数据驻留 | https://dify.ai/dify-enterprise | 2026-09 |
| Dify | 插件生态信任治理（2026-08） | https://dify.ai/blog/trust-is-a-feature-how-dify-is-governing-a-growing-plugin-ecosystem | 2026-09 |
| Dify | Creator Center/Template Marketplace | https://dify.ai/blog | 2026-09 |
| Dify | RAG=向量，不接图数据库；1.12 Summary Index 替代 GraphRAG | https://dify.ai/blog/dify-1.12.0-summary-index-from-fragmented-retrieval-to-full-context | 2026-09 |
| Dify | 30+ 向量库知识库 | https://deepwiki.com/langgenius/dify-docs/8-knowledge-base-and-rag-system | 2026-09 |
| Coze | 企业 AI、全链路可管可控可审计、技能商店 | https://www.volcengine.com/product/coze-pro | 2026-09 |
| Coze | 企业版 SSO/VPC 内网连接 | https://docs.coze.cn/guides_edition | 2026-09 |
| Coze | 企业商店/企业插件商店/仅允许企业插件 | https://docs.coze.cn/recent-updates | 2026-09 |
| Coze | 私网模式插件/自定义密钥加密/IAM | https://docs.coze.cn/coze_pro_premium_package_faq ; https://docs.coze.cn/guides_set_pro_subusers | 2026-09 |
| Coze | 开源版可本地/服务器自部署（4C8G） | https://www.byteplus.com/en/activity/coze-dev/ve | 2026-09 |
| Coze | 本地 Agent 接入（Claude Code/Hermes/OpenClaw） | https://docs.coze.cn/cozespace_local_agent | 2026-09 |
| n8n | 完全 on-prem、SSO/SAML/LDAP/provisioning/2FA | https://n8n.io/enterprise/ | 2026-09 |
| n8n | RBAC/审计送 SIEM/工作流历史/Git 版本控制/Secret 加密 | https://n8n.io/ | 2026-09 |
| Airflow | 3.3.2（2026-09）/3.3.0（2026-07）发布 | https://airflow.apache.org/docs/apache-airflow/stable/release_notes.html | 2026-09 |
| Airflow | 3.2 多团队隔离（实验） | https://airflow.apache.org/blog/airflow-3.2.0/ | 2026-09 |
| Airflow | 3.0 GA 面向服务架构/事件驱动/React UI | https://airflow.apache.org/docs/apache-airflow/3.1.1/release_notes.html | 2026-09 |
| Airflow | 3.3 状态存储 AIP-103、Java/Go Task SDK AIP-108 | https://airflow.apache.org/blog/tags/release/ | 2026-09 |
| Airflow | Astronomer Dag 级角色最小权限 | https://www.astronomer.io/docs/astro/release-notes/ | 2026-09 |
| Copilot Studio | DLP/数据驻留/认证/环境路由/区域自定义 | https://learn.microsoft.com/zh-cn/microsoft-copilot-studio/security-and-governance | 2026-09 |
| Copilot Studio | 构建管理 Agent、发布渠道、Power Platform 治理 | https://www.microsoft.com/en/microsoft-copilot/microsoft-copilot-studio | 2026-09 |
| Copilot Studio | Entra Agent ID（2026-07）、MCP 工具/认证 | https://learn.microsoft.com/en-us/microsoft-copilot-studio/whats-new | 2026-09 |
| Copilot Studio | Information Barriers/敏感信息不跨 Agent/DLP 明细 | https://learn.microsoft.com/sv-se/power-platform/released-versions/copilotstudio/2026.5.5 | 2026-09 |
| Copilot Studio | 编排类型/Agent 类型治理 | https://adoption.microsoft.com/files/copilot-studio/Microsoft-Copilot-Studio_Governance-and-security-guide.pdf | 2026-09 |
| 飞书 aily | 数据源/行列动态权限/知识隔离/多渠道 | https://www.feishu.cn/paid/ai | 2026-09 |
| 飞书 aily | 记忆隔离/中心化凭证/代理通道内网 | https://aily.feishu.cn/hc/1u7kleqg/ef4valfx | 2026-09 |
| 飞书 aily | MCP 市场企业内流通 | https://bytedance.larkoffice.com/wiki/QqdPwGhHuicn12km20hcNeU0nHd | 2026-09 |
| 飞书 aily | 任务模式/任务拆解 | https://www.feishu.cn/content/article/7585126677299137754 | 2026-09 |
| 飞书 aily | 知识空间可见范围/文档可搜权限校验 | https://aily.feishu.cn/hc/1u7kleqg/t8gwpqxg | 2026-09 |
| 钉钉 | 2700万+组织/7亿用户/等保三级/全链路加密 | https://m.dingtalk.com/ | 2026-09 |
| 钉钉 | 悟空企业 Agent 平台/原生调用账号权限体系/skills | https://36kr.com/p/3726501820807554 | 2026-09 |
| 钉钉 | 智能体按部门/人员授权、共同管理员、全链路 trace | https://open.dingtalk.com/document/aipass/automatically-generate-an-ai-assistant-1 | 2026-09 |
| 钉钉 | 知识集按归属部门/可用人员范围 | https://open.dingtalk.com/document/aipass/distribution-of-ai-assistant-1 | 2026-09 |
| 钉钉 | 炼丹炉企业大模型平台 | https://open.dingtalk.com/document/aipass/introduction-of-large-model-platform-of-dingtalk-blast-furnace-1 | 2026-09 |

> **未核项说明**：① Copilot Studio 是否提供客户内网 fully on-prem 版本；② 飞书 aily、钉钉 AI 的私有化部署是否完整交付 AI 能力；③ 各竞品精确价格档位。以上均未在本次搜索中取得权威来源，故标注「未核」，不作为结论依据。

---

*本文档为外部竞品研究稿。竞品结论随厂商版本迭代可能变化，重大投标/选型前应按同一来源清单复核时效信息（2026-09）。我方结论以 CURRENT-ARCHITECTURE.md V1.1 为最终裁决。*
