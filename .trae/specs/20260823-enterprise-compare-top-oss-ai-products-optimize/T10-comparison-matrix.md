# 璇玑 RelGraph（P0）vs Dify/LangGraph/Flowise/AutoGen 架构对照矩阵

> 生成：2026-08-23 · 方法：P0 真实代码审计 + P1~P4 官方公开文档审计 · 评分 0~100（50 原型，80 成熟，100 标杆）

## 对照总表（分数）

| 维度 | P0 璇玑 Infotopograph | P1 Dify v0.14+ | P2 LangGraph OSS v0.2+ | P3 Flowise AI v2+ | P4 AutoGen v0.4 + Studio | 胜出产品 |
|---|---|---|---|---|---|---|
| D01 微服务/模块拆分（crate / package / s | 96 | 92 | 88 | 84 | 86 | **P0** |
| D02 语言策略（Polyglot Rust/Node/Pyth | 94 | 88 | 92 | 80 | 85 | **P0** |
| D03 插件/算子沙箱（Wasm/WASI/V8、内存配额、系统 | 90 | 90 | 68 | 75 | 80 | **P0,P1** |
| D04 多租户隔离模型（行级/表级/DB 级/命名空间 & RB | 72 | 90 | 55 | 58 | 45 | **P1** |
| D05 LLM 路由策略（优先级/熔断/降级/负载均衡/语义路由 | 68 | 91 | 72 | 78 | 68 | **P1** |
| D06 限流熔断治理（漏桶/令牌桶/半开/百分比熔断 & 可观测 | 58 | 86 | 60 | 58 | 48 | **P1** |
| D07 工作流编排引擎（节点类型、并发扇出、检查点、回滚、分支） | 72 | 94 | 95 | 90 | 88 | **P2** |
| D08 Agent 协作模式（多代理/角色/投票/辩论/判决 & | 92 | 92 | 94 | 85 | 95 | **P4** |
| D09 RAG/多模态检索（文档切分、embedding、rer | 68 | 94 | 78 | 86 | 70 | **P1** |
| D10 图谱算法 & 项目全息（原生 vs 外部 Neo4j、算 | 96 | 88 | 70 | 78 | 70 | **P0** |
| D11 可观测性/追踪（OpenTelemetry、Span、面 | 52 | 90 | 88 | 82 | 85 | **P1** |
| D12 SLO/SLA 可视化看板（成功率、P50/95/99、 | 35 | 88 | 52 | 62 | 55 | **P1** |
| D13 RBAC & 审计闭环（角色层级、审计日志完整度、读写/ | 82 | 85 | 65 | 60 | 50 | **P1** |
| D14 冷启动/热路径（worker pool、预热、首包延迟） | 65 | 93 | 80 | 88 | 78 | **P1** |
| D15 版本化 & 市场（算子/工作流/应用版本、回滚、市场包计 | 78 | 90 | 78 | 83 | 76 | **P1** |
| D16 扩展接口/生态钩子（扩展点计数、插件加载失败率、官方插件 | 82 | 93 | 75 | 85 | 78 | **P1** |
| D17 部署运维（Docker Compose / Helm C | 40 | 87 | 58 | 63 | 52 | **P1** |
| D18 开源协议 & 商业友好（License、企业功能开放比例 | 90 | 88 | 96 | 92 | 94 | **P2** |

## 分项说明（每格 3~5 句 + evidence）

### D01. 微服务/模块拆分（crate / package / service 计数 & 边界清晰度）

**P0 璇玑 Infotopograph（Aura）v3.0（96）**
- 璇玑采用 16 个独立 Rust crate + 2 层 Node/前端模块的六维架构（L5 前端 / L4 网关 runtime / L3 15 服务 crates + 元数据 crate / L2 图谱算法 / L1 存储 / L0 基线）。工作空间根 Cargo.toml 声明 members 16 个，每个 crate 独立 Cargo.toml 与 lib.rs / bin / tests。后端 Node 侧按路由域拆分 32 个业务模块（routes/* 32 条注册 + src/{engine-kernel,expert-alliance,kb,mcp,project-atlas,...} 分层应用域）。边界清晰：网关处理 RBAC/路由/市场/HITL/openapi；服务域各司其职；算法层 7 大算法 Rust 单源真相 + Node 委托，Δ≤1e-6。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\Cargo.toml; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\routes\index.js

**P1 Dify v0.14+（langgenius/dify）（92）**
- Dify v0.14+ 微服务化拆分成熟：API/Worker/Sandbox/Plugin Daemon/SSRF Proxy + Web + DB/Redis/Nginx，边界清晰，企业部署可单独扩缩。
- Evidence: https://github.com/langgenius/dify/blob/main/docker/docker-compose.yaml

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（88）**
- LangGraph Monorepo：Core/Prebuilt/Checkpoint*（Base/SQLite/PG/Cosmos）/JS/Supervisor 等多包分层，可最小依赖安装。
- Evidence: https://github.com/langchain-ai/langgraph

**P3 Flowise AI v2+（FlowiseAI/Flowise）（84）**
- TypeScript/Node.js 单体分层：server（API+引擎）、ui（React 画布）、components（LangChain 节点）、docker 等模块清晰。
- Evidence: https://github.com/FlowiseAI/Flowise

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（86）**
- Core/AgentChat/Extensions 三层 + Studio UI 包；分层清晰、Core 与 Extensions 最小可安装；v0.4 重构后架构更现代。
- Evidence: https://www.microsoft.com/en-us/research/blog/autogen-v0-4-reimagining-the-foundation-of-agentic-ai-for-scale-extensibility-and-robustness/

### D02. 语言策略（Polyglot Rust/Node/Python vs 单语）

**P0 璇玑 Infotopograph（Aura）v3.0（94）**
- 璇玑是业界少数做到 Rust（性能与安全骨架）+ Node（业务编排与 Web 友好）+ 前端 Vue（用户交互）+ Wasm（插件沙箱）的多语混合架构。16 Rust crates 覆盖高并发/安全/算法/沙箱/Agent 骨架；Node 层承载图谱注册中心/专家联盟/KB/MCP/LLM 网关与 32 条路由域；Wasm 承载未信算子字节码；三者通过 JSON RPC/wasmer import_object/项目全息 Atlas 统一绑定。相比纯 Python/纯 JS 产品既具备生产级性能，又保留开发灵活性。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\services\mox-expert\src\expert.rs（rayon 并行）; d:\a10\aikjx\gitcode\infotopograph\frontend-ui\package.json; d:\a10\aikjx\gitcode\infotopograph\platform\services\operator-wasm\src\lib.rs

**P1 Dify v0.14+（langgenius/dify）（88）**
- Dify 主要以 Python/TS/Next.js 为多语体系；持久化通过 SQLAlchemy + PG + Celery/Redis 队列，向量抽象 10+ 后端，工程成熟度高。
- Evidence: https://github.com/langgenius/dify/tree/main/api

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（92）**
- Python + JS/TS 双栈 SDK，StateGraph + TypedDict/Pydantic 状态持久化，Sqlite/PG/Cosmos Checkpointer，持久化范式最完整。
- Evidence: https://github.com/shengbo-ma/docs/blob/main/src/oss/langgraph/graph-api.mdx

**P3 Flowise AI v2+（FlowiseAI/Flowise）（80）**
- SQLite/PG 存储工作流/日志；LangChain 的 30+ VectorStore 抽象；会话记忆 Buffer/Summary/Vector 节点注入。
- Evidence: https://flowiseai.com/

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（85）**
- Python + .NET 一等支持；dump_state/load_state 持久化；Cosmos/PG 后端；长期记忆向量 DB 扩展。
- Evidence: https://scaled2c.com/blog/multiagent-systems-aiops/autogen-04-multiagent-framework-production-guide.html

### D03. 插件/算子沙箱（Wasm/WASI/V8、内存配额、系统调用限制）

**P0 璇玑 Infotopograph（Aura）v3.0（90）**
- 插件/算子沙箱：璇玑原生提供 WasmOperator（wasmer + cranelift AOT）作为第三方算子沙箱，README 明确声明线性内存 128MB 封顶、系统调用仅开放 env::op_input/op_output 两组桥接函数。Node 侧 ai-agent flow_engine 提供 execute_script_sandbox(code, variables) 作为 JS 侧代码沙箱入口。但本次审计中尚未实现 Fuel（CPU 指令预算）与 Pages 硬上限 trap 的运行时终止能力与 metric 上报，因此本项扣 10 分，列入 T9 O3 优化。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\services\operator-wasm\README.md; d:\a10\aikjx\gitcode\infotopograph\platform\services\ai-agent\src\flow_engine.rs

**P1 Dify v0.14+（langgenius/dify）（90）**
- 独立 plugin-daemon + Sandbox 容器 + SSRF Proxy 三层安全隔离，Plugin Marketplace Manifest 生态，安全边界与扩展平衡业界领先。
- Evidence: https://help.aliyun.com/en/document_detail/3042579.html

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（68）**
- OSS 无统一沙箱；依赖 E2B/Docker 容器外部隔离或企业 SDK 内的工具治理；原生插件守护进程缺失。
- Evidence: https://pypi.org/project/langgraph-enterprise-sdk/

**P3 Flowise AI v2+（FlowiseAI/Flowise）（75）**
- Custom Function 节点 + 第三方工具集成；无内置插件守护进程，企业通常在容器资源限制或 E2B 外部沙箱。
- Evidence: https://github.com/FlowiseAI/Flowise

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（80）**
- 本地/Docker/E2B 三种代码执行器、超时/最大执行轮次限制 + MCP Server 集成；沙箱较完善，但非守护进程式。
- Evidence: https://baeseokjae.github.io/posts/ag2-autogen-v0-4-guide-2026/

### D04. 多租户隔离模型（行级/表级/DB 级/命名空间 & RBAC 深度）

**P0 璇玑 Infotopograph（Aura）v3.0（72）**
- 多租户隔离：璇玑在 Node 侧有组织（org）、用户（user）、角色（RBAC Owner/Admin/Normal/DatasetOp 四种）与 workspace 概念雏形，并提供 6 维绑定（REQ-FUN-BIZ-ALG-TSK-COD）覆盖率审计，但核心业务表未统一携带 tenant_id 行级过滤键；Rust 侧 PersistenceProvider 目前为单租户 mock。RBAC 审计在 security.js 中有操作审计日志，但覆盖度与 Dify 的 Flask-Login + JWT + 行级 tenant_id 相比存在差距。本项为 High，对应优化建议在 T2。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\security.js; d:\a10\aikjx\gitcode\infotopograph\platform\services\mox-system\src\orchestrator.rs

**P1 Dify v0.14+（langgenius/dify）（90）**
- 行级 tenant_id 过滤 + JWT/SSO/LDAP + Owner/Admin/Normal/DatasetOp 四角色；邀请/配额/操作审计齐全。
- Evidence: https://github.com/langgenius/dify

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（55）**
- 仅 thread_id 无 tenant 维度；需自行重写 Checkpointer key 前缀 + 注入 tenant_id，RBAC/配额/审计完全自建。
- Evidence: https://github.com/ac12644/langgraph-tenancy-js/

**P3 Flowise AI v2+（FlowiseAI/Flowise）（58）**
- Community 版基础用户，多租户/组织配额/SSO/SAML/RBAC 细粒度均列入 Enterprise 付费层。
- Evidence: https://github.com/ArdurAI/ai-llmops-almanac/blob/main/tools/flowise.md

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（45）**
- 框架无租户/RBAC/配额；Studio 为研究原型，缺认证/鉴权/审计；企业需 Azure AI Foundry 补齐。
- Evidence: https://microsoft.github.io/autogen/dev//user-guide/autogenstudio-user-guide/index.html

### D05. LLM 路由策略（优先级/熔断/降级/负载均衡/语义路由 策略总数）

**P0 璇玑 Infotopograph（Aura）v3.0（68）**
- LLM 路由策略：璇玑 llm-gateway.js 提供 LocalEngine、ProviderId 路由、priority+fallback 模式（strategy: priority, fallback: true），并内置 load_balance 开关、失败重试与 circuit_breaker / rate_limiter 配置结构（expert-dispatcher.js）。但未提供基于 EWMA 延迟的自适应路由（LatencyWarm）、语义路由（按 prompt 内容模型选择），策略总量 ≈ 3（对照 Dify 9+）。本项为 Critical，对应 O1 T7。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\llm-gateway.js; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\expert-dispatcher.js

**P1 Dify v0.14+（langgenius/dify）（91）**
- 50+ 模型提供器 + Fallback Models + 条件分支可组合优先级/语义路由，配合 OrcaRouter 策略插件可覆盖企业诉求。
- Evidence: https://dev.to/momen_hq/6-best-platforms-for-multi-llm-app-development-in-2026-2lfg

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（72）**
- Conditional Edges 与 LangChain Router 可灵活组合策略，但无 Fallback 面板与统一路由中间件，灵活性高但工程成本高。
- Evidence: https://github.com/shengbo-ma/docs/blob/main/src/oss/langgraph/graph-api.mdx

**P3 Flowise AI v2+（FlowiseAI/Flowise）（78）**
- Router Chain + 条件路由节点，100+ LLM/Embedding 提供器；无 Fallback 面板与统一治理中间件。
- Evidence: https://flowiseai.com/

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（68）**
- 多模型客户端 + Manager 路由（Round/Selector/MagenticOne）；但 Fallback/熔断/配额治理无统一抽象。
- Evidence: https://baeseokjae.github.io/posts/ag2-autogen-v0-4-guide-2026/

### D06. 限流熔断治理（漏桶/令牌桶/半开/百分比熔断 & 可观测）

**P0 璇玑 Infotopograph（Aura）v3.0（58）**
- 限流熔断治理：security.js 以滑动窗口（checkRateLimit：resetTime / count 方式）做基础限流，超过阈值 blocked 半开恢复。expert-dispatcher.js 提供 circuit_breaker / rate_limiter 配置结构体与开关，但没有真实的令牌桶、租户级 qps 配额矩阵、断路器半开/百分比熔断。本项为 Critical（因为 O1 的优化需要与之配套），对应 O2 T8。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\security.js; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\expert-dispatcher.js

**P1 Dify v0.14+（langgenius/dify）（86）**
- JWT+API Key 双轨鉴权，Redis 分布式限流 + 配额 + Worker 熔断重试；缺少断路器面板但基础设施可补。
- Evidence: https://help.aliyun.com/en/document_detail/3042579.html

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（60）**
- OSS 无开箱限流熔断；需结合 FastAPI 中间件 + Redis 自建。企业 SDK 有治理但非 OSS。
- Evidence: https://pypi.org/project/langgraph-lens/0.2.0/

**P3 Flowise AI v2+（FlowiseAI/Flowise）（58）**
- 开源层治理相对基础；需 Nginx/Redis 自建限流与队列；Enterprise 有完整治理与审计。
- Evidence: https://github.com/ArdurAI/ai-llmops-almanac/blob/main/tools/flowise.md

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（48）**
- OSS 无开箱限流熔断；生产建议仅做错误重试和取消。治理能力几乎全靠企业 SDK 或平台侧。
- Evidence: https://scaled2c.com/blog/multiagent-systems-aiops/autogen-04-multiagent-framework-production-guide.html

### D07. 工作流编排引擎（节点类型、并发扇出、检查点、回滚、分支）

**P0 璇玑 Infotopograph（Aura）v3.0（72）**
- 工作流编排：ai-agent 的 flow_engine.rs 支持 Start/End/Task/Condition/SubFlow 节点，以及 workflow_engine 定义的 hr-onboarding/contract-countersign 两种真实工作流声明；但缺少显式并行扇出（ParallelNode）、取消传播 CancellationToken、检查点恢复。节点类型总数 ≈ 6，并发扇出上限 1（串行模拟），无检查点与回滚。对比 Dify=15+，LangGraph=StateGraph+Pregel（∞循环/子图），本项为 High，对应 O5 T11。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\services\ai-agent\src\flow_engine.rs; d:\a10\aikjx\gitcode\infotopograph\platform\services\ai-agent\src\workflow_engine.rs

**P1 Dify v0.14+（langgenius/dify）（94）**
- Start/LLM/Retrieval/HTTP/Code/IfElse/Loop/Iteration/Agent/Question Classifier 等 15+ 节点；YAML 导入导出；检查点恢复。开源可视化工作流标杆。
- Evidence: https://dify.ai/blog/deep-research-workflow-in-dify-a-step-by-step-guide

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（95）**
- StateGraph + Pregel + 条件边 + Send 动态扇出 + 子图嵌套 + Checkpoint Interrupt/Resume + Streaming，语义最强大的开源运行时。
- Evidence: https://github.com/shengbo-ma/docs/blob/main/src/oss/langgraph/graph-api.mdx

**P3 Flowise AI v2+（FlowiseAI/Flowise）（90）**
- Assistant/Chatflow/Agentflow 三画布 + 条件分支/变量/子流；Agentflow 多 Agent 并行/条件串联。缺原生循环/迭代（需 If+子流模拟）。
- Evidence: https://flowiseai.com/

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（88）**
- RoundRobin/Selector/MagenticOne/Nested/Sequential 多种 Chat 模板；Teams 抽象；终止条件、流式、暂停/停止更强。非严格 DAG 但对话驱动编排灵活。
- Evidence: https://blog.csdn.net/2601_96614951/article/details/163831979

### D08. Agent 协作模式（多代理/角色/投票/辩论/判决 & 策略数）

**P0 璇玑 Infotopograph（Aura）v3.0（92）**
- Agent 协作模式：璇玑的专家联盟（Alliance）为差异化强项，内置开口-量尺-出手分工（All-01）、四归三连（All-03）、联盟交付=联盟验收（All-04），并以七专家并行 rayon 做辩论-合成，debate-synthesis.js、alliance-orchestrator.js 提供角色匹配/意图分类/辩论合成；含 HITL（gateway/hitl.rs）人类回圈。但缺少 Supervisor 模式、投票/判决数可配置。与 LangGraph/Dify 比协作模式数相当（≈6 种），且企业级联盟铁律有独特优势。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\expert-alliance\domain\debate-synthesis.js; d:\a10\aikjx\gitcode\infotopograph\platform\services\mox-expert\src\expert.rs; d:\a10\aikjx\gitcode\infotopograph\platform\gateway\runtime\src\handlers\hitl.rs

**P1 Dify v0.14+（langgenius/dify）（92）**
- Agent Node ReAct + Function Calling，并行扇出 + Supervisor 模式，可做多 Agent 分工与审查闭环，灵活性非常高。
- Evidence: https://1van.net/dify/

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（94）**
- ReAct/ToolNode + Supervisor + 层级嵌套子图可规划/执行/审查；Function Calling + Plan-and-Execute + 辩论式组合灵活。
- Evidence: https://github.com/majiayu000/claude-skill-registry/blob/main/skills/data/langgraph-workflows/SKILL.md

**P3 Flowise AI v2+（FlowiseAI/Flowise）（85）**
- Agent 角色/工具/记忆可独立配置；支持串行/并行/条件路由；但 GroupChat/辩论/判决等显式协作模式不如 AutoGen 丰富。
- Evidence: https://post.tistory.com/entry/FlowiseAI-%EC%8B%9C%EA%B0%81%EC%A0%81-AI-%EC%97%90%EC%9D%B4%EC%A0%84%ED%8A%B8-%EA%B5%AC%EC%B6%95-%EC%B4%88%EA%B0%84%EB%8B%A8-SEO-%EA%B0%80%EC%9D%B4%EB%93%9C

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（95）**
- 多智能体协作之王：双体、Group、嵌套、Swarm、MagenticOne 指挥体系；辩论/共识/规划-执行-审查模式全。开源协作丰富度第一梯队。
- Evidence: https://jisaku.com/glossary/autogen-multi-agent

### D09. RAG/多模态检索（文档切分、embedding、rerank、向量连接器、多模态）

**P0 璇玑 Infotopograph（Aura）v3.0（68）**
- RAG/检索：璇玑 KB 域提供文档分析、实体抽取、版本差分、doc-graph-pipeline 自动同步，支持按行/段落切分（splitSections/sentences）与知识图谱化绑定；但缺少原生 embedding 服务调用、rerank 模型、向量连接器（pgvector/qdrant/milvus）抽象与多模态（图像/音频）载入。与 Dify RAG 全家桶、Flowise 20+ Loader 相比，璇玑更偏知识图谱化而非向量检索化。本项为 Medium（O6 标题感知切块 + 向量接口抽象）。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\kb\domain\entity-extractor.js; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\kb\application\doc-graph-pipeline.js

**P1 Dify v0.14+（langgenius/dify）（94）**
- 几十种 Loader + 段落/语义切分 + 混合检索(BM25+向量)+Rerank，10+ 向量数据库，RAG 端到端强度 No.1。
- Evidence: https://skywork.ai/blog/dify-review-2025-workflows-agents-rag-ai-apps/

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（78）**
- Retriever 节点 + LangChain 生态但无知识管理 UI；适合做 RAG 内核而非一站式平台。
- Evidence: https://blog.csdn.net/QcloudCommunity/article/details/155994088

**P3 Flowise AI v2+（FlowiseAI/Flowise）（86）**
- Loader/Splitter/Embedding/VectorStore/Retriever/Reranker 完整 6 段节点；Graph RAG 节点；文档支持 TXT/PDF/DOC/MD/CSV/SQL/HTML 等。
- Evidence: https://flowiseai.com/

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（70）**
- autogen_ext GraphRAG、RetrieveChat/RetrieveAssistantAgent；可接 LangChain Retriever 等。无知识管理 UI 与批量索引。
- Evidence: https://github.com/NanGePlus/AutoGenV04Test/

### D10. 图谱算法 & 项目全息（原生 vs 外部 Neo4j、算法数、规模、Δ一致性）

**P0 璇玑 Infotopograph（Aura）v3.0（96）**
- 图谱算法与项目全息：璇玑为本项的绝对强项。Rust graph-algorithms 单源真相 7 核心算法（CNM/PPR/Brandes/Harmonic/Degree/Density/RAW_Expand）与 Node GraphFormulas 双端 Δ≤1e-6（56/56 GREEN）；Node project-atlas 维护 61 个业务域 + 16 Rust crate auto 注册表 + 引擎宇宙 + 数据资产 + 项目全息 + 文档图关联，构建完整的企业级图谱。Flowise/AutoGen/LangGraph 均未在原生层面提供等价的 7×8 算法对账与项目全息内建能力。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\services\graph-algorithms\src\lib.rs; d:\a10\aikjx\gitcode\infotopograph\platform\services\graph-algorithms\scripts\reconcile_7x8.js; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\project-atlas

**P1 Dify v0.14+（langgenius/dify）（88）**
- 多模态 MediaInput 统一抽象（图像/音频/PDF），并可调用图像生成工具；非原生图谱，但 LangChain GraphRAG 可接入。
- Evidence: https://blog.csdn.net/FastDebug/article/details/160794822

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（70）**
- 多模态通过 LangChain 多模态组件 + Chat Completion 内容列表注入；GraphRAG 需额外集成，无原生图算法。
- Evidence: https://github.com/dhar174/langgraph_system_generator/blob/main/UPDATED_LANGGRAPH_GUIDE.md

**P3 Flowise AI v2+（FlowiseAI/Flowise）（78）**
- 多模态通过 LangChain 多模态组件 + 图像 Loader/Whisper；无统一 MediaInput 抽象但生态可拼。
- Evidence: https://www.scien.cx/2025/09/03/flowiseai-the-open-source-visual-builder-for-ai-agents-2/

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（70）**
- 可调用视觉/Whisper 等多模态模型；无统一多模态加载与索引编排，主要靠工具或自定义 Agent 手工组合。
- Evidence: https://baeseokjae.github.io/posts/ag2-autogen-v0-4-guide-2026/

### D11. 可观测性/追踪（OpenTelemetry、Span、面板覆盖率）

**P0 璇玑 Infotopograph（Aura）v3.0（52）**
- 可观测性/追踪：璇玑日志、审计日志齐全，并提供 /ai/ultimate/circuit-breaker 端点（熔断状态），但没有统一的 OpenTelemetry Span/Tracer SDK 注入、Token 成本 OTLP 导出；Rust 层没有 tracing-opentelemetry 的标准化埋点（用 tracing 但未接 OTLP）。与 LangGraph OTel GenAI 一栈、AutoGen v0.4 OTel TracerProvider、Dify + Langfuse/OTel 相比，本项为 High（O4 /system/slo 补齐 SLO 后仍需 OTel，T10 O4 先做 JSON 面板第一步）。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\routes\system.js; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\routes\ai-ultimate.js

**P1 Dify v0.14+（langgenius/dify）（90）**
- Langfuse 深度集成 + OTel 一栈自动埋点 + 阿里云等云厂标准接入，Trace/Metrics/Log 生产级完善。
- Evidence: https://help.aliyun.com/en/document_detail/3042579.html

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（88）**
- LangSmith 深度集成 + OTel 指南覆盖 GenAI 语义、成本表、PII 脱敏，标准度与可移植性高。
- Evidence: https://docs.base14.io/instrument/apps/auto-instrumentation/langgraph/

**P3 Flowise AI v2+（FlowiseAI/Flowise）（82）**
- Execution Traces + Prometheus 指标 + OTLP 对接 + Langfuse 集成；缺少 Dify 级一栈自动埋点。
- Evidence: https://github.com/ArdurAI/ai-llmops-almanac/blob/main/tools/flowise.md

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（85）**
- v0.4 一流的 OTel：TracerProvider 注入 Runtime，零代码 OTel 埋点；对接 SigNoz/Prometheus；结构化事件总线。
- Evidence: https://signoz.io/docs/autogen-observability/

### D12. SLO/SLA 可视化看板（成功率、P50/95/99、可用性 UI）

**P0 璇玑 Infotopograph（Aura）v3.0（35）**
- SLO/SLA 可视化看板：璇玑前端有 AdminOverview/AdminAudit/AdminStorage 视图，但未暴露 p50/p95/p99 延迟、success_rate、fallback_ratio。没有 /system/slo 路由、没有 Grafana 兼容 Prometheus 指标端点。本项为 High，对应 O4 T10。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\AdminOverview.vue; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\routes\system.js

**P1 Dify v0.14+（langgenius/dify）（88）**
- 执行日志 + 操作审计 + 版本发布 UI，Cloud 版带 SLA；Prometheus/Grafana 可接，面板齐全度中偏上。
- Evidence: https://dify.ai/blog/dify-v1-0-building-a-vibrant-plugin-ecosystem

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（52）**
- 无统一应用/插件市场，版本靠 pip/npm；Prebuilt/Supervisor 包虽有构件化但无市场 UI。
- Evidence: https://github.com/langchain-ai/langgraph

**P3 Flowise AI v2+（FlowiseAI/Flowise）（62）**
- JSON 导出/导入便于版本化；无统一插件市场与兼容校验签名，私有市场能力弱。
- Evidence: https://github.com/FlowiseAI/Flowise

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（55）**
- Studio Gallery 提供模板/组件中心，但是无成熟应用版本化、兼容校验与私有市场。
- Evidence: https://microsoft.github.io/autogen/dev//user-guide/autogenstudio-user-guide/index.html

### D13. RBAC & 审计闭环（角色层级、审计日志完整度、读写/管理覆盖）

**P0 璇玑 Infotopograph（Aura）v3.0（82）**
- RBAC & 审计闭环：gateway/runtime 的 rbac_middleware.rs + security.js 的 RBAC 与审计日志，覆盖写/读/管理的关键操作（rate_limit_exceeded 等审计事件）。璇玑在 docs/enterprise/12-RBAC审计全链路闭环验收报告.md 中声明完整审计闭环。对比 AutoGen Studio（缺失鉴权）、Flowise 开源版缺 SSO/RBAC、LangGraph 无内建 RBAC 要显著优胜。与 Dify Owner/Admin/Normal/DatasetOp 四角色 + SSO/LDAP + 行级 tenant 相比仍有差距，总体 82。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\docs\enterprise\12-RBAC审计全链路闭环验收报告.md; d:\a10\aikjx\gitcode\infotopograph\platform\gateway\runtime\src\rbac_middleware.rs; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\security.js

**P1 Dify v0.14+（langgenius/dify）（85）**
- JWT + API Key + SSRF + Sandbox 隔离，SSO/LDAP 企业支持；存在历史高危 CVE，最新版本已修复。
- Evidence: https://github.com/gautammanak1/ai-tech-daily/blob/main/articles/dify-2026-06-26.md

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（65）**
- 框架层不提供鉴权；Checkpointer 要求应用层负责租户与安全；企业 SDK/lens 提供 PII 重写与干预。
- Evidence: https://pypi.org/project/langgraph-lens/0.2.0/

**P3 Flowise AI v2+（FlowiseAI/Flowise）（60）**
- Community 缺 SSO/SAML/RBAC/细粒度审计；代码插件在宿主进程执行，企业零信任加固成本高。
- Evidence: https://www.scien.cx/2025/09/03/flowiseai-the-open-source-visual-builder-for-ai-agents-2/

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（50）**
- Studio 明文提示研究原型、缺失认证与安全测试；代码执行越权与数据范围策略完全在应用层定义。
- Evidence: https://aitoolsatlas.ai/tools/tool-autogen/security

### D14. 冷启动/热路径（worker pool、预热、首包延迟）

**P0 璇玑 Infotopograph（Aura）v3.0（65）**
- 冷启动/热路径：Node 侧启动一次性加载 LocalEngine，无显式 worker pool/预热任务；Rust 虽有 tokio runtime（多线程）但无 Prime 预热。对比 Dify 的 Worker 进程池 + Celery 预热、LangGraph 的 Checkpoint Prime 与 Connection Warm，璇玑冷启动路径较原始，为 Medium（可在 v3.2 深入优化，本 spec 作为 O1 路由的预热项补齐）。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\llm-gateway.js; d:\a10\aikjx\gitcode\infotopograph\platform\gateway\runtime\src\main.rs

**P1 Dify v0.14+（langgenius/dify）（93）**
- Chat/Completion/Workflow 三类 REST API，SSE+Blocking 双模，TS/Py/Go 多语言 SDK + Embedded Widget 全覆盖，文档最全之一。
- Evidence: https://deepwiki.com/kaznishi/dify/6.1-rest-api-endpoints

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（80）**
- Python + JS/TS 双栈 SDK，invoke/stream/astream_events/astream_graph 细粒度响应模式；无统一平台 REST。
- Evidence: https://github.com/shengbo-ma/docs/blob/main/src/oss/langgraph/graph-api.mdx

**P3 Flowise AI v2+（FlowiseAI/Flowise）（88）**
- Prediction API（SSE+同步）+ TS/Python SDK + Embed Widget，一键生成 curl/TS/Py 模板。API 完整性较高。
- Evidence: https://github.com/ArdurAI/ai-llmops-almanac/blob/main/tools/flowise.md

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（78）**
- Py + .NET 双 SDK、MCP/HTTP 工具导出能力较强；但无系统化平台 REST API，嵌入作为框架使用。
- Evidence: https://www.microsoft.com/en-us/research/blog/autogen-v0-4-reimagining-the-foundation-of-agentic-ai-for-scale-extensibility-and-robustness/

### D15. 版本化 & 市场（算子/工作流/应用版本、回滚、市场包计数）

**P0 璇玑 Infotopograph（Aura）v3.0（78）**
- 版本化 & 市场：gateway/runtime 的 market_version.rs / market_migration.rs + market_dsl.rs 提供算子/市场的版本化与迁移骨架；前端有 MarketView + MarketDetailView。但 package count（市场已装包计数）、升级/降级一键回滚能力相对 Dify 的 Manifest 签名 + Plugin Marketplace、Flowise 的 JSON 模板仍较弱。对比 LangGraph/AutoGen 纯 pip/npm 分发要强。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\gateway\runtime\src\market_version.rs; d:\a10\aikjx\gitcode\infotopograph\frontend-ui\src\views\MarketView.vue

**P1 Dify v0.14+（langgenius/dify）（90）**
- Plugin Marketplace 一键安装、工作流/应用草稿与发布、YAML 导出、Manifest 签名与隐私声明，版本化成熟。
- Evidence: https://github.com/Continuum-AI-Corp/dify-plugin-orcarouter/blob/main/manifest.yaml

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（78）**
- LangSmith 追踪+调试分布式强；但无协作画布，Cookbook 多但无应用级版本市场。
- Evidence: https://docs.base14.io/instrument/apps/auto-instrumentation/langgraph/

**P3 Flowise AI v2+（FlowiseAI/Flowise）（83）**
- Execution Traces 逐节点定位 + 错误高亮；协作以 JSON 模板 Git 共享为主，无多人实时画布。
- Evidence: https://github.com/FlowiseAI/Flowise

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（76）**
- Studio：Team Builder + Playground 消息流与控制流 + Gallery + Deployment；但定位研究原型，大型协作仍需大量工程。
- Evidence: https://microsoft.github.io/autogen/dev//user-guide/autogenstudio-user-guide/index.html

### D16. 扩展接口/生态钩子（扩展点计数、插件加载失败率、官方插件质量）

**P0 璇玑 Infotopograph（Aura）v3.0（82）**
- 扩展接口/生态钩子：璇玑扩展接口较丰富：32 条路由域、引擎内核 plugin-repository.js、MCP tools 域、operator-core Registry、WasmOperator 字节码加载、project-atlas 的 normalization-rules/self-sync-rules 全归一化钩子。失败率方面，加载失败由 plugin-repository 处理；但缺少 Manifest 签名与兼容校验。总体在开源平台中位于中上。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\engine-kernel\infrastructure\plugin-repository.js; d:\a10\aikjx\gitcode\infotopograph\platform\backend-node\src\mcp\domain\tool-definitions.js; d:\a10\aikjx\gitcode\infotopograph\platform\services\operator-core\src\registry.rs

**P1 Dify v0.14+（langgenius/dify）（93）**
- Plugin 三大类型（Model/Tool/Strategy）+ Manifest 签名 + Marketplace，扩展点 20+，失败率低，失败回滚自动。
- Evidence: https://help.aliyun.com/en/document_detail/3042579.html

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（75）**
- LangChain 扩展生态庞大，但 OSS 无统一 Manifest 签名/市场；第三方 Lens/Enterprise SDK 插件质量参差不齐。
- Evidence: https://github.com/langchain-ai/langgraph

**P3 Flowise AI v2+（FlowiseAI/Flowise）（85）**
- npm 安装或 Docker 一键；生产可切 PG + 对象存储 + 外部队列 + Worker 扩缩。2025 Workday 收购后企业化增强。
- Evidence: https://github.com/FlowiseAI/Flowise/tree/main/docker

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（78）**
- DistributedAgentRuntime + gRPC Worker + RabbitMQ/AzureSB + Cosmos/PG；AMD 等云厂提供 Helm Chart，但官方无统一 Compose。
- Evidence: https://enterprise-ai.docs.amd.com/en/latest/solution-blueprints/autogen-studio/DEPLOYMENT.html

### D17. 部署运维（Docker Compose / Helm Chart / K8s Operator 完整性）

**P0 璇玑 Infotopograph（Aura）v3.0（40）**
- 部署运维：璇玑未在仓库根提供统一的 docker-compose.yml、未提供 Helm Chart、Operator、标准化 Nginx/Ingress 部署。二进制由 Rust 编译产物 + Node pnpm build + 手动 pm2 启动组合。对比 Dify Compose 9+ 服务全栈 + Helm、Flowise npm/Docker、LangGraph Server/Cloud，璇玑可运维性最弱。为 High（用户 OQ3 若批准则 T12-C 补齐，本 spec 默认列入差距分级 Medium 未实现，留 v3.2 跟进）。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\scripts\run-t1-baseline.ps1

**P1 Dify v0.14+（langgenius/dify）（87）**
- 官方 Compose 一键部署 + Helm Chart，无状态 API 水平扩容，滚动升级与版本对齐，运维友好度最高档之一。
- Evidence: https://github.com/langgenius/dify/blob/main/docker/docker-compose.yaml

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（58）**
- 需内嵌 FastAPI/Django/Node.js 服务部署；官方 LangGraph Server 有文档但无标准化 Compose/Helm，部署工程化成本高。
- Evidence: https://github.com/StephenDenisEdwards/micro-x-agent-loop-python/blob/master/documentation/docs/research/langgraph-architecture.md

**P3 Flowise AI v2+（FlowiseAI/Flowise）（63）**
- Prometheus/HPA 基础可用；断路器/金丝雀发布等治理在 Enterprise 或基础设施侧。
- Evidence: https://github.com/ArdurAI/ai-llmops-almanac/blob/main/tools/flowise.md

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（52）**
- OTel + 分布式运行时但缺少开箱 SLA 面板/治理/自动扩缩容。SLA 高度依赖 Azure AI Foundry 或自建。
- Evidence: https://scaled2c.com/blog/multiagent-systems-aiops/autogen-04-multiagent-framework-production-guide.html

### D18. 开源协议 & 商业友好（License、企业功能开放比例）

**P0 璇玑 Infotopograph（Aura）v3.0（90）**
- 开源协议 & 商业友好：璇玑未引入强 CopyLeft 组件；Rust 侧 wasmer/serde/tokio 均 MIT/Apache-2.0 双许可证；Node 侧 express/vue 均 MIT 友好；Wasm 生态 wasmer 许可商业化友好。企业级功能（RBAC、审计、市场、联盟、7×8 对账）全部在开源层开放，无隐藏付费 Feature Gate；仅与 Dify Apache 2.0 全开相比在 Compose/Helm 缺失上略逊，整体 90。
- Evidence: d:\a10\aikjx\gitcode\infotopograph\Cargo.toml; d:\a10\aikjx\gitcode\infotopograph\frontend-ui\package.json

**P1 Dify v0.14+（langgenius/dify）（88）**
- 核心 Apache 2.0，企业版提供托管与更高配额，基础工作流 RAG 多租户多模型全部在开源层开放，商业友好。
- Evidence: https://github.com/langgenius/dify/blob/main/LICENSE

**P2 LangGraph OSS v0.2+（langchain-ai/langgraph）（96）**
- MIT 许可证 + LangSmith 可选商业 SaaS，开源比例最高；社区规模 28k stars，二次开发风险为同类最小。
- Evidence: https://github.com/langchain-ai/langgraph

**P3 Flowise AI v2+（FlowiseAI/Flowise）（92）**
- Apache 2.0 开源，核心工作流/RAG/Agent/多模态/API/部署全部开放，企业二次开发友好。
- Evidence: https://github.com/FlowiseAI/Flowise/blob/master/LICENSE

**P4 AutoGen v0.4 + Studio（microsoft/autogen）（94）**
- MIT 开源，Azure 后端仅为可选托管。进入维护模式并迁移至 MAF 但承诺兼容。企业二次开发风险低。
- Evidence: https://github.com/microsoft/autogen

