'use strict';

/**
 * 项目全息图谱 · 业务资产注册表（domain 层 · 静态值对象 · 零 IO）
 * ------------------------------------------------------------------
 * 29 个业务域 + 4 个可插拔模块的唯一权威定义。
 * 每条登记：身份 / 核心功能 / 代码路径 / 依赖引擎 / 数据资产 / 关联文档。
 * 全部功能自研（可借鉴业界架构思想，实现零外部框架依赖）。
 */

const DOMAINS = [
  {
    id: 'system', name: '系统与状态', codePath: 'src/routes/system.js',
    keyFeatures: ['健康检查与运行状态', '服务元信息自描述', '运行日志查询'],
    engines: ['knowledge-graph'], dataAssets: ['settings.json', 'logs.json'], docs: ['docs/architecture.md']
  },
  {
    id: 'studio', name: '璇玑工作台', codePath: 'src/routes/studio.js',
    keyFeatures: ['豆包式场景卡片与全局搜索（零门槛直通）', 'JS 代码 vm 沙箱在线运行（3s 守卫零 IO）', 'API 游乐场与图谱体检项目看板'],
    engines: ['project-atlas'], dataAssets: [], docs: ['docs/standards/xuanji-studio.md']
  },
  {
    id: 'graph', name: '知识图谱', codePath: 'src/routes/graph.js',
    keyFeatures: ['图谱节点/边 CRUD 与检索', '图算法分析（PageRank/中心性/社区/路径）', '图谱统计与结构洞察'],
    engines: ['knowledge-graph', 'ai-integration-engine'], dataAssets: ['graph_nodes.json', 'graph_edges.json'], docs: ['docs/modules/business-process-flowcharts.md']
  },
  {
    id: 'chat', name: 'AI 对话', codePath: 'src/routes/chat.js',
    keyFeatures: ['多会话对话管理', '会话记忆语义检索', '联网搜索上下文增强'],
    engines: ['llm-gateway', 'session-store', 'web-search-service'], dataAssets: ['dialogue_sessions.json'], docs: ['docs/对话开发系统-全维分析与业务流程图.md']
  },
  {
    id: 'web-search', name: '联网搜索', codePath: 'src/routes/web-search.js',
    keyFeatures: ['多搜索引擎配置管理', '统一搜索入口与引用结构化', '搜索连通性测试'],
    engines: ['web-search-service'], dataAssets: ['settings.json', 'web_search_config.json'], docs: ['docs/AI-UNIFIED-OPTIMIZATION-PLAN.md']
  },
  {
    id: 'artifacts', name: '本地制品', codePath: 'src/routes/artifacts.js',
    keyFeatures: ['制品注册表（按文件名去重）', '安全文件落盘（路径逃逸校验）', '制品预览与下载'],
    engines: ['auto-dev-engine'], dataAssets: ['artifacts.json'], docs: ['docs/modules/local-artifact-agent.md']
  },
  {
    id: 'optimizer', name: '无穷维度优化', codePath: 'src/routes/optimizer.js',
    keyFeatures: ['CEM 高维配置寻优', '多引擎对比验证矩阵', '收敛曲线与维度敏感度可视化'],
    engines: ['infinite-dimension-optimizer'], dataAssets: ['infinite_optimization_runs.json'], docs: ['docs/modules/infinite-dimension-optimization.md']
  },
  {
    id: 'ai-platform', name: 'AI 平台资源', codePath: 'src/routes/ai-platform.js',
    keyFeatures: ['工作流定义与管理', '算子注册与编排', '资源池管理'],
    engines: ['ai-engine'], dataAssets: ['workflows.json', 'operators.json', 'resources.json', 'flows.json'], docs: ['docs/modules/automation-module.md']
  },
  {
    id: 'browser-market', name: '浏览器与市场', codePath: 'src/routes/browser-market.js',
    keyFeatures: ['智能体市场', '流水线注册与交易', '插件生态管理'],
    engines: ['llm-gateway'], dataAssets: ['market.json', 'registered_agents.json', 'registered_pipelines.json', 'plugins.json'], docs: ['docs/modules/market-module.md']
  },
  {
    id: 'integration', name: '集成通道', codePath: 'src/routes/integration.js',
    keyFeatures: ['多 LLM 提供商管理', '路由权重配置', '用量统计'],
    engines: ['llm-gateway'], dataAssets: ['llm_config.json', 'llm_routing.json', 'llm_usage.json'], docs: ['docs/modules/ai-engine-master-analysis.md']
  },
  {
    id: 'expert-alliance', name: '专家联盟', codePath: 'src/routes/expert-alliance.js',
    keyFeatures: ['专家全生命周期与咨询编排', '多专家辩论与会话链', '智能路由与指标反馈', '六阶段流水线（门禁重试/辩论降级/安全强制组队）', 'trace 审计回溯与学习技能沉淀'],
    engines: ['expert-alliance', 'expert-alliance-engine', 'expert-dispatcher'], dataAssets: ['experts.json', 'expert_sessions.json', 'expert_chat_history.json', 'alliance_intent_priors.json', 'alliance_traces.jsonl', 'dispatcher_config.json', 'alliance_learned_skills.json'], docs: ['docs/modules/专家联盟AI对话需求文档-V2.0-架构优化版.md', 'docs/modules/xuanji-expert-alliance-fusion-flows.md']
  },
  {
    id: 'mcp', name: 'MCP 协议服务', codePath: 'src/routes/mcp.js',
    keyFeatures: ['JSON-RPC 2.0 标准协议（initialize/tools/list/tools/call/ping）', '专家联盟七大工具标准暴露（供 Claude Code/Cursor 等 MCP 客户端调用）', '批量请求与通知语义合规（202 无响应体）'],
    engines: ['expert-alliance', 'expert-alliance-engine'], dataAssets: [], docs: ['docs/standards/expert-alliance-flow-standard.md']
  },
  {
    id: 'expert-graph', name: '专家图谱', codePath: 'src/routes/expert-graph.js',
    keyFeatures: ['专家能力图三级建边', 'CNM 社区聚类', '协同增益计算'],
    engines: ['expert-graph'], dataAssets: ['expert_capability_graph.json'], docs: ['docs/modules/专家联盟V2.0-集成对齐分析报告.md']
  },
  {
    id: 'orchestration', name: '编排协作', codePath: 'src/routes/orchestration.js',
    keyFeatures: ['插件化编排流水线', '检查点与事务回放', '编排统计'],
    engines: ['orchestration-engine'], dataAssets: ['plugins.json'], docs: ['docs/modules/automation-module.md']
  },
  {
    id: 'ai-enhanced', name: '16 模块 AI 增强', codePath: 'src/routes/ai-enhanced.js',
    keyFeatures: ['16 个 AI 增强模块', '模板化内容生成', '算子级 AI 任务'],
    engines: ['llm-gateway'], dataAssets: ['caomei_templates.json', 'operators.json'], docs: ['docs/modules/algorithm-verification.md']
  },
  {
    id: 'tasks', name: '任务管理', codePath: 'src/routes/tasks.js',
    keyFeatures: ['任务 CRUD 与状态机', '任务分配与追踪', '任务关联会话'],
    engines: ['expert-alliance'], dataAssets: ['tasks.json'], docs: ['docs/enterprise/04-business-processing.md']
  },
  {
    id: 'kb', name: '知识库', codePath: 'src/routes/kb.js',
    keyFeatures: ['文档全生命周期与版本快照', 'AI 文档分析（实体/关键词/分类）', '文档实体与图谱互链', '全维实体抽取与图谱自动化管道（doc→实体→域 三层绑定）'],
    engines: ['kb'], dataAssets: ['kb_documents.json', 'kb_categories.json', 'kb_versions.json', 'kb_history.json', 'doc_graph_links.json'], docs: ['docs/DOC-NORMALIZATION-REPORT.md']
  },
  {
    id: 'engine-universe', name: '引擎宇宙图谱', codePath: 'src/routes/engine-universe.js',
    keyFeatures: ['17 引擎节点化与关联边查询', '需求归一化链服务映射', '全链路 113 项机器验证'],
    engines: ['engine-universe'], dataAssets: [], docs: ['docs/standards/engine-universe.md']
  },
  {
    id: 'engine-kernel', name: '引擎内核', codePath: 'src/routes/engine-kernel.js',
    keyFeatures: ['槽位契约架构（一切皆可插件化，切换引擎零代码改动）', '瞬间切换与失败自动回滚（探活保障银行级不宕机）', '三层插件商城（系统内置/云端目录/本地清单）', 'AI 自动配置引擎组合（自然语言需求→绑定方案）'],
    engines: ['engine-kernel', 'llm-gateway', 'web-search-service'], dataAssets: ['engine_bindings.json', 'engine_plugins.json', 'engine_marketplace.json'], docs: ['docs/standards/engine-kernel.md']
  },
  {
    id: 'atlas', name: '项目全息图谱', codePath: 'src/routes/atlas.js',
    keyFeatures: ['全项目机器图谱化（域/引擎/算法/数据/文档统一关联）', '无破窗验证（动态比对真实代码库，含 self-sync 自动登记层）', 'AI 架构师图谱增强对话', '自管理：self-sync 自发现/自登记/自愈（自己管理自己）', '全维归一化：需求归一化流水线 + 代码图谱桥接 + 全域治理看板'],
    engines: ['project-atlas', 'expert-alliance', 'engine-universe'], dataAssets: ['atlas_auto_registry.json', 'atlas_auto_registry_rust.json', 'normalization_runs.json', 'code_graph_bindings.json'], docs: ['docs/standards/project-atlas.md']
  },
  {
    id: 'auto-tasks', name: '自动任务', codePath: 'src/routes/auto-tasks.js',
    keyFeatures: ['自动化任务调度', '任务执行引擎', '执行历史与重试'],
    engines: ['orchestration-engine'], dataAssets: ['automation.json'], docs: ['docs/enterprise/08-全维自动化处理明确书.md']
  },
  {
    id: 'modules-admin', name: '模块与存储管理', codePath: 'src/routes/modules-admin.js',
    keyFeatures: ['可插拔模块管理', '存储提供方切换', '数据迁移', '系统管理区存储与模块面板承载（providers/switch/status 实时可视）'],
    engines: ['knowledge-graph'], dataAssets: ['settings.json'], docs: ['docs/specs/PT-Primi-架构规范-V1.0-完整版.md']
  },
  {
    id: 'security', name: '安全审计', codePath: 'src/routes/security.js',
    keyFeatures: ['操作审计日志', '安全状态检查', '密钥加密管理', 'API Key 凭证生命周期（创建一次性明文/吊销/校验）', '系统管理区承载（凭证/审计/HITL 审批面板，frontend-ui /admin 统一入口）'],
    engines: ['llm-gateway'], dataAssets: ['logs.json'], docs: ['docs/enterprise/12-RBAC审计全链路闭环验收报告.md']
  },
  {
    id: 'ai-engine', name: 'AI 引擎核心', codePath: 'src/routes/ai-engine.js',
    keyFeatures: ['统一编排五步流水线', '意图识别（激活扩散）', '能力矩阵自描述'],
    engines: ['ai-engine-core', 'ai-engine'], dataAssets: ['graph_nodes.json', 'graph_edges.json'], docs: ['docs/modules/ai-engine-master-analysis.md']
  },
  {
    id: 'ai-integrated', name: '智能集成引擎', codePath: 'src/routes/ai-integrated.js',
    keyFeatures: ['个性化 PageRank 图计算', '符号图 LLM 交互', 'token 预算裁剪'],
    engines: ['ai-integration-engine'], dataAssets: ['llm_usage.json', 'learned_skills.json'], docs: ['docs/modules/mathematical-foundation.md']
  },
  {
    id: 'ai-ultimate', name: '终极 AI 引擎', codePath: 'src/routes/ai-ultimate.js',
    keyFeatures: ['向量记忆语义检索', '多步推理与置信度评估', '推理规则管理'],
    engines: ['ultimate-ai-engine'], dataAssets: ['ultimate_reasoning_rules.json'], docs: ['docs/modules/ai-engine-master-analysis.md']
  },
  {
    id: 'auto-dev', name: '自动开发引擎', codePath: 'src/routes/auto-dev.js',
    keyFeatures: ['需求→架构图谱→代码全自动流水线', '确定性代码渲染（无幻觉）', '制品预览与注册'],
    engines: ['auto-dev-engine', 'llm-gateway'], dataAssets: ['artifacts.json'], docs: ['docs/modules/PrimiFlow-设计蓝图.md']
  },
  {
    id: 'services', name: '服务管理', codePath: 'src/routes/services.js',
    keyFeatures: ['外部服务注册与探活', '服务依赖管理', '优雅启停'],
    engines: ['llm-gateway'], dataAssets: ['settings.json'], docs: ['docs/enterprise/02-architecture.md']
  },
  {
    id: 'internal', name: '内部端点（sidecar 调用 · 禁止公网暴露）', codePath: 'src/routes/internal.js',
    keyFeatures: [
      'Node 侧 sidecar 内部接口（127.0.0.1 仅监听，意图分类、图算法等运维专用）',
      '与 Rust runtime 的 NodeSidecarClient 形成双向契约：`/internal/intent`、`/internal/graph/algo`',
      '安全：Nginx/网关必须在边界拦截 /internal/*，仅内部 sidecar 调用。'
    ],
    engines: ['knowledge-graph'], dataAssets: ['settings.json', 'logs.json'],
    docs: ['docs/enterprise/03-top-master-l0-governance.md', 'docs/enterprise/02-architecture.md']
  },
  {
    id: 'projects', name: '项目中心', codePath: 'src/routes/projects.js',
    keyFeatures: ['项目 CRUD 与项目化统计总览', '全维类型注册表（11 项目类别 + 18 资源类型，单一真相源）', '全维资源目录实时聚合（模块/MCP/插件/智能体/技能/任务/算子/知识库跨域采集）', '项目-资源绑定管理（"一切皆是项目"运行时归类入口）'],
    engines: ['project-atlas'], dataAssets: ['projects.json'], docs: ['docs/standards/project-atlas.md']
  },
  // ===== Rust crate 静态登记（璇玑图谱 · 三注册表联动 · 跨语言承载） =====
  // 标记 auto=true：
  //   - 豁免 W1 路由比对（Rust 域为原生后端内部服务，无 Node routes/ 入口）
  //   - 豁免 W6 文档全域覆盖（Rust 内聚文档由 Cargo 项目 README / DESIGN.md 承载，不强制放入 DOCS 注册表）
  {
    id: 'domain-rust-operator-core', name: 'Rust/OperatorCore',
    codePath: '../services/operator-core/src',
    keyFeatures: ['算子 Monad 与资源容器（operator-core monad.rs/resource.rs）', '守恒律校验引擎（conservation.rs · 算子输入输出质量守恒）', '算子注册表（registry.rs）+ 类别体系（category.rs）'],
    engines: ['engine-rust-operator-core'], kind: 'rust-crate', scope: 'platform/services/operator-core',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-operator-wasm', name: 'Rust/OperatorWasm',
    codePath: '../services/operator-wasm/src',
    keyFeatures: ['算子 WASM 沙箱（Wasmer + Cranelift AOT）', '算子二进制沙箱加载与内存限制', 'WASM 算子导出接口（mod-rust-operator-wasm 模块复用）'],
    engines: ['mod-rust-operator-wasm'], kind: 'rust-crate', scope: 'platform/services/operator-wasm',
    module_ids: ['mod-rust-operator-wasm'], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-graph-algorithms', name: 'Rust/GraphAlgorithms',
    codePath: '../services/graph-algorithms/src',
    keyFeatures: ['PageRank 推模型（转置图，Rust primary · 与 ai-integration-engine co_impl）', 'CNM 模块度贪心凝聚、Brandes 介数 / Harmonic 紧密 / 度中心性', '模块度 / 密度 图结构指标 全量实现'],
    engines: ['engine-rust-graph-algorithms'], kind: 'rust-crate', scope: 'platform/services/graph-algorithms',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-optimizer', name: 'Rust/Optimizer',
    codePath: '../services/optimizer/src',
    keyFeatures: ['算子图优化 Pass（融合 / 重排 / 公共子表达式消除）', 'Cost-based 优化器（cost.rs 启发式 + 数据驱动搜索）', '优化计划序列化与热路径应用'],
    engines: ['engine-rust-optimizer'], kind: 'rust-crate', scope: 'platform/services/optimizer',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-flow-ai', name: 'Rust/FlowAI',
    codePath: '../services/flow-ai/src',
    keyFeatures: ['数据流与控制流建模（dataflow.rs / topology.rs）', '关键路径分析（critpath.rs）与调度（schedule.rs）', '代码生成（codegen.rs）与冲突检测（conflict.rs）'],
    engines: ['engine-rust-flow-ai'], kind: 'rust-crate', scope: 'platform/services/flow-ai',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-xuanji-expert', name: 'Rust/XuanjiExpert',
    codePath: '../services/xuanji-expert/src',
    keyFeatures: ['15 专家能力画像 + RBAC/审计（experts/ + audit/）', '六阶段流水线验证器（verify/ topology/conflict/data_dep）', '执行器与治理（executor.rs/govern.rs）+ 多租户策略'],
    engines: ['engine-rust-xuanji-expert'], kind: 'rust-crate', scope: 'platform/services/xuanji-expert',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-hermes-flow-bridge', name: 'Rust/HermesFlowBridge',
    codePath: '../services/hermes-flow-bridge/src',
    keyFeatures: ['Hermes 协议桥接（bridge/router/state）', '会话录制与回放（recorder.rs + session_e2e 测试）', 'mini Hermes 兼容层 + 插件容器（plugin/hooks）'],
    engines: ['mod-rust-hermes-flow-bridge'], kind: 'rust-crate', scope: 'platform/services/hermes-flow-bridge',
    module_ids: ['mod-rust-hermes-flow-bridge'], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-business-catalog', name: 'Rust/BusinessCatalog',
    codePath: '../services/business-catalog/src',
    keyFeatures: ['业务螺旋目录（spiral.rs · 分面分类与索引）', 'catalog 可执行二进制 + REST/CLI 发布', '领域目录 JSON Schema 校验与版本化'],
    engines: ['mod-rust-business-catalog'], kind: 'rust-crate', scope: 'platform/services/business-catalog',
    module_ids: ['mod-rust-business-catalog'], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-ai-agent', name: 'Rust/AIAgent',
    codePath: '../services/ai-agent/src',
    keyFeatures: ['对话图 + 工作流 + 工具总线（dialogue_graph/workflow_engine/plugin_bus）', '需求编译器与多智能体协作（requirement_compiler / multi_agent）', '浏览器自动化 + 资源管理器（browser_automation / resource_manager）'],
    engines: ['engine-rust-ai-agent'], kind: 'rust-crate', scope: 'platform/services/ai-agent',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-template-market', name: 'Rust/TemplateMarket',
    codePath: '../services/template-market/src',
    keyFeatures: ['模板市场核心数据结构与交易合约', '模板版本管理 + 元数据索引', '与浏览器市场模块的桥接（mod-rust-template-market 导出）'],
    engines: ['mod-rust-template-market'], kind: 'rust-crate', scope: 'platform/services/template-market',
    module_ids: ['mod-rust-template-market'], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-runtime', name: 'Rust/Runtime (Gateway)',
    codePath: '../gateway/runtime/src',
    keyFeatures: ['HITL 人机协同审批 WebSocket + RBAC 中间件（handlers/hitl + rbac_middleware）', 'Cordis 插件内核（bundle/lifecycle/event_bus/seam）', '治理台、市场、Agent 路由与 OpenAPI 标准（routes/* + openapi.rs）'],
    engines: ['engine-rust-runtime'], kind: 'rust-crate', scope: 'platform/gateway/runtime',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-xuanji-system', name: 'Rust/XuanjiSystem',
    codePath: '../services/xuanji-system/src',
    keyFeatures: ['服务编排器（orchestrator.rs）+ 多后端存储（repo/*：sqlite/mysql/postgres）', 'RBAC + 加密 + 限流（rbac/crypto/ratelimit）', '配置/错误/指标/事件 全栈底座（config/error/metrics/event）'],
    engines: ['engine-rust-xuanji-system'], kind: 'rust-crate', scope: 'platform/services/xuanji-system',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-primiflow-core', name: 'Rust/PrimiFlowCore',
    codePath: '../services/primiflow-core/src',
    keyFeatures: ['DSL 解析与代码生成（parse.rs + generate.rs + gen/*）', '执行器与持久化（executor/persistence/server）', 'Trace Matrix 与 Schema 生成（trace_matrix/schema）'],
    engines: ['engine-rust-primiflow-core'], kind: 'rust-crate', scope: 'platform/services/primiflow-core',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-primiflow-fusion', name: 'Rust/PrimiFlowFusion',
    codePath: '../services/primiflow-fusion/src',
    keyFeatures: ['六维融合体系（sixdim.rs）与统一包络（envelope/unified）', '平台服务注册（registry.rs）+ PTDoc 产线（ptdoc.rs）', '可观测性（observability.rs）+ 服务端入口（server/main）'],
    engines: ['engine-rust-primiflow-fusion'], kind: 'rust-crate', scope: 'platform/services/primiflow-fusion',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  {
    id: 'domain-rust-kg-hub', name: 'Rust/KGHub',
    codePath: '../services/kg-hub/src',
    keyFeatures: ['知识图谱接入与摄入（ingest.rs/index.rs）', '本体管理与推理（ontology/reason + urn.rs）', '合并器（consolidator.rs）与治理（govern.rs）+ 循环引擎（loop_engine.rs）'],
    engines: ['engine-rust-kg-hub'], kind: 'rust-crate', scope: 'platform/services/kg-hub',
    module_ids: [], domain_owner: 'Rust 子项目', auto: true,
    dataAssets: [], docs: []
  },
  // ===== Rust 16 crate 正式条目（璇玑三注册表联动 · 跨语言） =====
  //   auto: true → 不入 W1 路由比对（Rust 后端 crate 不暴露 Node 端路由入口）
  //   注：auto=true 的 Rust 域须在 PROJECTS 中显式归属（对应 proj-* domains 列表），否则 W10 孤儿拦截。
  {
    id: 'rust::ai-agent', kind: 'rust', auto: true,
    codePath: 'platform/services/ai-agent',
    owns_domain: ['ai-agent', '对话图', '工作流', '工具总线'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '00374bdd-cc60-55bf-8970-a879afbfe443'
  },
  {
    id: 'rust::business-catalog', kind: 'rust', auto: true,
    codePath: 'platform/services/business-catalog',
    owns_domain: ['business-catalog', '业务螺旋目录', '分面索引'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '62b2cca1-d98f-5e41-b26e-8d2a43966117'
  },
  {
    id: 'rust::flow-ai', kind: 'rust', auto: true,
    codePath: 'platform/services/flow-ai',
    owns_domain: ['flow-ai', '数据流', '控制流', '关键路径', '代码生成'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '2fcd3eac-e894-5876-b007-fb33c56c0d65'
  },
  {
    id: 'rust::graph-algorithms', kind: 'rust', auto: true,
    codePath: 'platform/services/graph-algorithms',
    owns_domain: ['graph-algorithms', 'PageRank', 'CNM', 'Brandes', 'Harmonic', '度中心性', '图密度', 'RAW_EXPAND'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: 'fbd31c6a-41cd-5274-be2f-2a28066eaf0a'
  },
  {
    id: 'rust::hermes-flow-bridge', kind: 'rust', auto: true,
    codePath: 'platform/services/hermes-flow-bridge',
    owns_domain: ['hermes-flow-bridge', 'Hermes 协议', '会话录制', '插件总线'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '9bfaf43b-385a-5a44-9fb2-65b4003ee80d'
  },
  {
    id: 'rust::kg-hub', kind: 'rust', auto: true,
    codePath: 'platform/services/kg-hub',
    owns_domain: ['kg-hub', 'KG 接入', '本体', '推理', '合并器', '治理'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: 'cb909f06-c0df-55ec-b397-543623a8c349'
  },
  {
    id: 'rust::operator-core', kind: 'rust', auto: true,
    codePath: 'platform/services/operator-core',
    owns_domain: ['operator-core', '算子 Monad', '守恒律', '算子注册表', '算子内核'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L6Kernel'],
    crateId: 'acf14283-3931-5528-adce-2c0cd3815363'
  },
  {
    id: 'rust::operator-wasm', kind: 'rust', auto: true,
    codePath: 'platform/services/operator-wasm',
    owns_domain: ['operator-wasm', 'WASM 算子沙箱', 'Wasmer ABI'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '5a1df407-b217-5340-a5ae-5f4535d1e6de'
  },
  {
    id: 'rust::optimizer', kind: 'rust', auto: true,
    codePath: 'platform/services/optimizer',
    owns_domain: ['optimizer', '算子图优化', 'Cost-based 搜索', '热路径'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: 'e56676c7-ec1f-5415-9587-ba8249d0178a'
  },
  {
    id: 'rust::primiflow-core', kind: 'rust', auto: true,
    codePath: 'platform/services/primiflow-core',
    owns_domain: ['primiflow-core', 'DSL 解析', '代码生成', '执行器', '持久化', 'Trace Matrix'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '8c8d2382-6f9f-5218-894e-a07a43aa9554'
  },
  {
    id: 'rust::primiflow-fusion', kind: 'rust', auto: true,
    codePath: 'platform/services/primiflow-fusion',
    owns_domain: ['primiflow-fusion', '六维融合', '统一包络', 'PTDoc 产线', '可观测性'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '75238345-b48b-534b-818b-8d9abe083a41'
  },
  {
    id: 'rust::template-market', kind: 'rust', auto: true,
    codePath: 'platform/services/template-market',
    owns_domain: ['template-market', '模板版本化', '市场合约', '交易签名'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '4d2e50c1-9d64-525d-86cf-2d7d610a27b9'
  },
  {
    id: 'rust::xuanji-expert', kind: 'rust', auto: true,
    codePath: 'platform/services/xuanji-expert',
    owns_domain: ['xuanji-expert', '15 专家画像', 'RBAC', '审计', '六阶段管线'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L4Services'],
    crateId: '50bb6200-04c5-5e4c-8354-4c6e1b230024'
  },
  {
    id: 'rust::xuanji-system', kind: 'rust', auto: true,
    codePath: 'platform/services/xuanji-system',
    owns_domain: ['xuanji-system', '服务编排', '多后端存储', 'RBAC', '加密', '限流', '持久化 Provider'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L7Infrastructure'],
    crateId: 'b81eec75-22ff-5155-ac49-19edf6f6b5ab'
  },
  {
    id: 'rust::xuanji-common-meta', kind: 'rust', auto: true,
    codePath: 'platform/services/xuanji-common-meta',
    owns_domain: ['xuanji-common-meta', 'CrateMeta', 'AIS 分层', 'CrateId 注册表'],
    keyFeatures: ['CrateMeta 元数据建模（crate_meta.rs 与分层标签）', '16 crate 全局注册表与 CrateId 校验（id_registry.rs）', 'AIS 架构分层 tag 统一（L3/L4/L5/L6/L7）'],
    engines: ['engine::xuanji_common_meta'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L5Domain'],
    crateId: '34a20231-1a80-5426-b392-40d7a2ddd9f7'
  },
  {
    id: 'rust::runtime', kind: 'rust', auto: true,
    codePath: 'platform/gateway/runtime',
    owns_domain: ['runtime', 'Cordis 插件运行时', 'HITL', '治理路由', '市场路由', 'Agent 路由', 'OpenAPI 标准'],
    version: '3.0.0-ai-powered',
    tags: ['rust', 'ais::L3Orchestration'],
    crateId: 'a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b'
  }
];

const MODULES = [
  {
    id: 'mod-graph', name: '图谱模块', codePath: 'src/modules/graph.js',
    keyFeatures: ['图谱批量导入导出', '节点详情与邻居查询', '图谱检索'],
    engines: ['knowledge-graph'], dataAssets: ['graph_nodes.json', 'graph_edges.json'], docs: ['docs/graph/guantu.req.json']
  },
  {
    id: 'mod-task', name: '任务模块', codePath: 'src/modules/task.js',
    keyFeatures: ['任务 REST CRUD', '任务状态流转', '任务列表查询'],
    engines: ['expert-alliance'], dataAssets: ['tasks.json'], docs: ['docs/enterprise/04-business-processing.md']
  },
  {
    id: 'mod-storage', name: '存储模块', codePath: 'src/modules/storage.js',
    keyFeatures: ['SQLite + JSON 双写存储', '存储提供方切换', '历史数据迁移'],
    engines: ['knowledge-graph'], dataAssets: ['settings.json'], docs: ['docs/specs/OUS-业务功能规划与架构数据关系分析.md']
  },
  {
    id: 'mod-melody2score', name: '旋律转谱模块', codePath: 'src/modules/melody2score.js',
    keyFeatures: ['旋律→乐谱工业级转换（8/8 样本全对）', '多音高检测后端自动降级', 'MusicXML/简谱双输出'],
    engines: ['llm-gateway'], dataAssets: [], docs: ['docs/modules/algorithm-verification.md']
  },
  // ===== Rust 可插拔模块（4 个桥接型 crate） =====
  {
    id: 'mod-rust-operator-wasm', name: 'Rust/WASM 算子沙箱模块',
    codePath: '../services/operator-wasm/src',
    keyFeatures: ['Wasmer 引擎装载 + 沙箱内存限额', '算子 ABI 导入导出（lib.rs 绑定）', '跨语言算子合约（operator-core → WASM 二进制互通）'],
    engines: ['engine-rust-operator-core'], dataAssets: [], docs: []
  },
  {
    id: 'mod-rust-hermes-flow-bridge', name: 'Rust/Hermes 流程桥接模块',
    codePath: '../services/hermes-flow-bridge/src',
    keyFeatures: ['Hermes Shim 集成（integration/hermes_shim.rs）', '会话录制与事件分发（recorder/live）', '插件总线 YAML 流程装载（plugin.yaml + hooks）'],
    engines: ['orchestration-engine'], dataAssets: [], docs: []
  },
  {
    id: 'mod-rust-business-catalog', name: 'Rust/业务目录模块',
    codePath: '../services/business-catalog/src',
    keyFeatures: ['业务分面螺旋索引（spiral.rs）', 'catalog 可执行二进制导出（bin/catalog.rs）', '业务域 ↔ 数据/流程映射查询 API'],
    engines: ['project-atlas'], dataAssets: [], docs: []
  },
  {
    id: 'mod-rust-template-market', name: 'Rust/模板市场模块',
    codePath: '../services/template-market/src',
    keyFeatures: ['模板登记与版本化目录（lib.rs 注册表）', '模板市场 JSON 协议适配（与 data/market.json 兼容）', '交易签名校验模板完整性'],
    engines: ['llm-gateway'], dataAssets: ['market.json'], docs: []
  }
];

module.exports = { DOMAINS, MODULES };
