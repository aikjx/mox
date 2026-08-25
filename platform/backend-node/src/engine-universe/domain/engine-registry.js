'use strict';

/**
 * 引擎宇宙注册表（domain 层 · 静态值对象 · 零 IO）
 * ------------------------------------------------------------------
 * 全系统 17 个引擎的唯一权威定义：身份、类别、关键功能、代码路径、能力清单。
 * 用户问题「记忆引擎/计算引擎/分析引擎/文档编写/自动化引擎怎么协同」的答案：
 * 每个引擎在这里节点化，关联边在 relation-registry.js，技术图谱是唯一管理中枢。
 */

const ENGINES = [
  {
    id: 'llm-gateway',
    name: 'LLM 网关',
    category: 'infrastructure',
    layer: '基础设施',
    codePath: 'src/llm-gateway.js',
    keyFunctions: [
      '多 AI 引擎接入（OpenAI/Claude/豆包/千问/Kimi/DeepSeek/智谱/Gemini），密钥加密管理',
      '自动优选激活引擎，listAvailableProviders 供全系统枚举可用引擎',
      '统一 chat 出口：全系统唯一 LLM 调用收口（实时日期注入 + 联网搜索上下文注入）'
    ],
    capabilities: ['chat']
  },
  {
    id: 'ai-engine-core',
    name: 'AI 引擎统一编排核心',
    category: 'orchestration',
    layer: '编排层',
    codePath: 'src/ai-engine-core.js',
    keyFunctions: [
      '五步流水线收口：意图识别（激活扩散）→ 能力路由 → 引擎执行 → 质量校验 → 指标反馈',
      '能力矩阵自描述（GET /ai/engine/capabilities）：expert/reasoning/memory/graph/workflow/chat',
      '降级不变式：任何能力执行失败单向降级到 chat，请求绝不安然空手而归'
    ],
    capabilities: ['process', 'analyze']
  },
  {
    id: 'ai-engine',
    name: '图谱与工作流引擎',
    category: 'intelligence',
    layer: '智能层',
    codePath: 'src/ai-engine.js',
    keyFunctions: [
      '图谱分析：统计 + PageRank + 社区检测 + 中心性 + AI 结论生成',
      '工作流顺序执行：步骤链编排，关键步中断保护',
      'PageRank 单源委托 ai-integration-engine（A18 归一化收口）'
    ],
    capabilities: ['graph', 'workflow']
  },
  {
    id: 'ai-integration-engine',
    name: '图智能计算引擎',
    category: 'intelligence',
    layer: '智能层',
    codePath: 'src/ai-integration-engine.js',
    keyFunctions: [
      '个性化 PageRank 统一实现：边权重 / 收敛容差 / 悬挂节点处理（全系统唯一定义）',
      '符号图构建与 token 预算裁剪：大图安全送入 LLM 上下文',
      '激活扩散意图识别底座（个性化 PageRank 特例：spread, d=0.85, 30 轮收敛）'
    ],
    capabilities: ['graph.compute']
  },
  {
    id: 'ultimate-ai-engine',
    name: '记忆与深度推理引擎',
    category: 'intelligence',
    layer: '智能层',
    codePath: 'src/ultimate-ai-engine.js',
    keyFunctions: [
      'VectorMemoryStore 向量记忆：embedding 生成 / 持久化 / 语义检索 / 过滤',
      'ReasoningEngine 多步推理：LLM 逐步推演 + 洞察提取 + 置信度评估',
      '归一化裁决：需求链终端的推理裁决能力（n_rec 节点承接）'
    ],
    capabilities: ['memory', 'reasoning']
  },
  {
    id: 'expert-alliance-engine',
    name: '专家联盟处理引擎',
    category: 'collaboration',
    layer: '协作层',
    codePath: 'src/expert-alliance-engine.js',
    keyFunctions: [
      '六阶段流水线：classifyIntent → composeTeam → deliberate → synthesize → qualityGate → learn',
      '多目标最优组队：能力匹配分 + 图谱协同增益 + Dispatcher 负载均衡',
      '辩论收敛：加权表决 + 共识度（一致率/方差）+ 少数派保留，失败降级单咨询'
    ],
    capabilities: ['expert']
  },
  {
    id: 'expert-alliance',
    name: '专家联盟域包',
    category: 'collaboration',
    layer: '协作层',
    codePath: 'src/expert-alliance/index.js',
    keyFunctions: [
      '专家全生命周期：注册/更新/下线/能力画像（15 专家 × 多类型）',
      '咨询编排：单专家 / 多专家并行 / 多轮辩论综合（domain 纯算法综合）',
      '会话链：顺序链（上下文传递）与并行链，历史交互持久化'
    ],
    capabilities: ['expert.consult', 'expert.debate', 'expert.chain']
  },
  {
    id: 'expert-graph',
    name: '专家能力图谱引擎',
    category: 'collaboration',
    layer: '协作层',
    codePath: 'src/expert-graph.js',
    keyFunctions: [
      '三级建边：包含式强边 + 2-gram 语义邻接边 + 相似关联边（密度 0.019→20 边）',
      'CNM 模块度贪心社区检测：专家聚类（当前 6 社区）',
      '协同增益计算：组队时沿图谱边权评估专家间协作增益'
    ],
    capabilities: ['expert.graph']
  },
  {
    id: 'expert-dispatcher',
    name: '专家调度引擎',
    category: 'collaboration',
    layer: '协作层',
    codePath: 'src/expert-dispatcher.js',
    keyFunctions: [
      '注册表式调度策略（STRATEGY_TYPES）：负载均衡 / 能力优先 / 历史成功率',
      '专家级运行时指标：成功率 / 平均耗时 / 置信度轨迹',
      '联盟组队的负载均衡权重来源'
    ],
    capabilities: ['expert.dispatch']
  },
  {
    id: 'orchestration-engine',
    name: 'V2 编排引擎',
    category: 'orchestration',
    layer: '编排层',
    codePath: 'src/orchestration-engine.js',
    keyFunctions: [
      '插件化编排：planner/executor/reflector 插件流水线（plan_act 模式）',
      '检查点与学习：runTurn 事务化执行 + 失败回放',
      '联盟 V2 代理：expert-alliance.orchestrate 的引擎底座'
    ],
    capabilities: ['orchestrate']
  },
  {
    id: 'auto-dev-engine',
    name: '自动开发引擎',
    category: 'automation',
    layer: '自动化层',
    codePath: 'src/auto-dev-engine.js',
    keyFunctions: [
      '全自动开发流水线：需求 → LLM 生成架构图谱 JSON → 规范校验 → 确定性代码渲染 → 安全落盘 → 预览',
      'LLM 只生成架构图谱，代码由确定性渲染器输出（可校验可复现无幻觉）',
      '安全边界：路径逃逸 / 编码逃逸校验，制品注册表按文件名去重'
    ],
    capabilities: ['auto-dev', 'doc.generate']
  },
  {
    id: 'infinite-dimension-optimizer',
    name: '无穷维度优化引擎',
    category: 'optimization',
    layer: '优化层',
    codePath: 'src/infinite-dimension-optimizer.js',
    keyFunctions: [
      'CEM 交叉熵高维寻优：动态构建优化维度（温度/路由强度/上下文深度/引擎权重）',
      '多目标加权评分：0.55×质量 + 0.20×速度 + 0.10×token 效率 + 0.15×稳定性',
      '收敛判据：σ̄<0.06 或 3 轮无改进停止；最优配置持久化生效'
    ],
    capabilities: ['optimize']
  },
  {
    id: 'web-search-service',
    name: '联网搜索服务',
    category: 'infrastructure',
    layer: '基础设施',
    codePath: 'src/web-search-service.js',
    keyFunctions: [
      '多搜索引擎接入（Bing 默认）与密钥加密管理',
      '统一 search() 入口 + 就绪校验 + 引用来源结构化返回',
      '搜索上下文注入 LLM 网关（联网开关 → 实时信息增强）'
    ],
    capabilities: ['web_search']
  },
  {
    id: 'session-store',
    name: '会话记忆引擎',
    category: 'infrastructure',
    layer: '基础设施',
    codePath: 'src/session-store.js',
    keyFunctions: [
      '会话持久化：历史消息加载与会话生命周期管理',
      '语义检索：历史问题构建向量索引，semanticSearch 基于 embedding 相似度召回',
      'AI 对话的记忆底座（"之前说过/历史知识"类问题的数据源）'
    ],
    capabilities: ['session', 'memory.recall']
  },
  {
    id: 'ai-flow-graph',
    name: '流程图谱引擎',
    category: 'orchestration',
    layer: '编排层',
    codePath: 'src/ai-flow-graph.js',
    keyFunctions: [
      '业务流程与算法流程统一承载：step/keyword/capability/engine 四类节点',
      '四类关系边：flows_to / triggers / delegates_to / degrades_to（降级链显式建模）',
      'F8 激活扩散意图识别：个性化 PageRank 特例（与旧打分 top-1 决策一致性已验证）'
    ],
    capabilities: ['flow.graph', 'intent.detect']
  },
  {
    id: 'kb',
    name: '知识库域包',
    category: 'knowledge',
    layer: '知识层',
    codePath: 'src/kb/index.js',
    keyFunctions: [
      '文档全生命周期：CRUD + 版本快照 + LCS 版本 diff + 软删除',
      '文档智能分析：实体抽取 / 关键词打分 / 分类建议 / 阅读指标（domain 纯算法）',
      '图谱关联：文档实体与知识图谱节点互链（graphLinks）'
    ],
    capabilities: ['kb.document', 'kb.analyze']
  },
  {
    id: 'knowledge-graph',
    name: '知识图谱引擎',
    category: 'knowledge',
    layer: '知识层',
    codePath: 'src/lib/graph-algos.js',
    keyFunctions: [
      '图谱数据中枢：graph_nodes/graph_edges 统一存储，节点/边 CRUD 与检索',
      '图算法库：邻接构建 / BFS 最短路 / PageRank / 度中心性 / Brandes 介数 / LPA 社区 / 激活扩散',
      '技术图谱管理所有链接：引擎宇宙、需求归一化链、业务/数据/算法流程图的统一承载底座'
    ],
    capabilities: ['graph.crud', 'graph.algos']
  },
  // ===== Rust crate 引擎节点（璇玑图谱 · 三注册表联动；11 个引擎型 crate） =====
  {
    id: 'engine-rust-ai-agent',
    name: 'Rust AI Agent 引擎',
    category: 'intelligence',
    layer: '智能层（Rust）',
    codePath: '../services/ai-agent/src/lib.rs',
    keyFunctions: [
      '对话图 + 工作流 + 工具总线（dialogue_graph/workflow_engine/plugin_bus）',
      '多智能体协作引擎（multi_agent.rs · 角色分工 + 消息总线）',
      '浏览器自动化与资源管理（browser_automation / resource_manager）'
    ],
    capabilities: ['agent.dialogue', 'agent.tool_use', 'agent.multi_agent']
  },
  {
    id: 'engine-rust-mox-expert',
    name: 'Rust 璇玑专家引擎',
    category: 'collaboration',
    layer: '协作层（Rust）',
    codePath: '../services/mox-expert/src/lib.rs',
    keyFunctions: [
      '15 专家画像 + 15× 验证管线（experts/* + verify/*）',
      '六阶段流水线：RBAC 门禁 → 管线装载 → 执行器 → 拓扑/冲突/数据依赖/增益 验证',
      '审计与多租户（audit/ + tenant_policy.rs）'
    ],
    capabilities: ['expert.audit', 'expert.pipeline', 'expert.validate']
  },
  {
    id: 'engine-rust-mox-system',
    name: 'Rust 璇玑系统底座',
    category: 'infrastructure',
    layer: '基础设施（Rust）',
    codePath: '../services/mox-system/src/lib.rs',
    keyFunctions: [
      '服务编排器 + 多后端存储（orchestrator.rs + repo/*）',
      'RBAC / 加密 / 限流（rbac/crypto/ratelimit）',
      '配置/错误/指标/事件 全栈底座（config/error/metrics/event）'
    ],
    capabilities: ['system.orchestration', 'system.storage', 'system.rbac']
  },
  {
    id: 'engine-rust-runtime',
    name: 'Rust 网关运行时（Cordis）',
    category: 'infrastructure',
    layer: '基础设施（Rust）',
    codePath: '../gateway/runtime/src/lib.rs',
    keyFunctions: [
      'Cordis 插件运行时：bundle/lifecycle/event_bus/seam/profile',
      'HITL 人机协同 WebSocket + RBAC 中间件（handlers/hitl + rbac_middleware）',
      '治理 / 市场 / Agent 路由 + OpenAPI 标准（routes/* + openapi.rs）'
    ],
    capabilities: ['runtime.plugin', 'runtime.hitl', 'runtime.routes']
  },
  {
    id: 'engine-rust-graph-algorithms',
    name: 'Rust 图算法引擎',
    category: 'intelligence',
    layer: '智能层（Rust）',
    codePath: '../services/graph-algorithms/src/lib.rs',
    keyFunctions: [
      'PageRank 推模型（转置图，Rust primary；与 ai-integration-engine co_impl）',
      'CNM 模块度贪心凝聚 + Brandes 介数 / Harmonic 紧密 / 度中心性',
      '模块度、密度 图结构指标（flow_graph.rs 稀疏图表示）'
    ],
    capabilities: ['graph.pagerank', 'graph.community', 'graph.centrality']
  },
  {
    id: 'engine-rust-flow-ai',
    name: 'Rust FlowAI 流程智能引擎',
    category: 'orchestration',
    layer: '编排层（Rust）',
    codePath: '../services/flow-ai/src/lib.rs',
    keyFunctions: [
      '数据流/控制流建模（dataflow.rs + topology.rs）',
      '关键路径分析与调度（critpath.rs / schedule.rs）',
      '代码生成与冲突检测（codegen.rs / conflict.rs）'
    ],
    capabilities: ['flow.schedule', 'flow.critpath', 'flow.codegen']
  },
  {
    id: 'engine-rust-optimizer',
    name: 'Rust 算子优化器引擎',
    category: 'optimization',
    layer: '优化层（Rust）',
    codePath: '../services/optimizer/src/lib.rs',
    keyFunctions: [
      '算子图融合 / 重排 / CSE（公共子表达式消除）Pass',
      'Cost-based 搜索（cost 启发式 + 基于运行 trace 的学习）',
      '优化计划序列化与热路径应用'
    ],
    capabilities: ['optimize.graph', 'optimize.cost_based']
  },
  {
    id: 'engine-rust-operator-core',
    name: 'Rust 算子核心引擎',
    category: 'orchestration',
    layer: '编排层（Rust）',
    codePath: '../services/operator-core/src/lib.rs',
    keyFunctions: [
      '算子 Monad + 资源容器（operator.rs + monad.rs + resource.rs）',
      '守恒律校验引擎（conservation.rs · 算子输入输出质量守恒单源）',
      '算子注册表、类别体系与执行引擎（registry.rs + engine.rs + category.rs）'
    ],
    capabilities: ['operator.exec', 'operator.conservation', 'operator.registry']
  },
  {
    id: 'engine-rust-primiflow-core',
    name: 'Rust PrimiFlow 核心引擎',
    category: 'automation',
    layer: '自动化层（Rust）',
    codePath: '../services/primiflow-core/src/lib.rs',
    keyFunctions: [
      'DSL 解析与代码生成（parse.rs + generate.rs + gen/* 多目标）',
      '执行器与持久化（executor.rs + persistence.rs + server.rs）',
      'Trace Matrix 与 Schema 生成（trace_matrix/schema）'
    ],
    capabilities: ['primiflow.gen', 'primiflow.exec', 'primiflow.persistence']
  },
  {
    id: 'engine-rust-primiflow-fusion',
    name: 'Rust PrimiFlow 六维融合引擎',
    category: 'automation',
    layer: '自动化层（Rust）',
    codePath: '../services/primiflow-fusion/src/lib.rs',
    keyFunctions: [
      '六维融合体系（sixdim.rs）+ 统一包络（envelope/unified）',
      '平台服务注册（registry.rs）+ PTDoc 产线（ptdoc.rs）',
      '可观测性与服务端（observability.rs + server.rs）'
    ],
    capabilities: ['fusion.sixdim', 'fusion.registry', 'fusion.observability']
  },
  {
    id: 'engine-rust-kg-hub',
    name: 'Rust 知识图谱中枢',
    category: 'knowledge',
    layer: '知识层（Rust）',
    codePath: '../services/kg-hub/src/lib.rs',
    keyFunctions: [
      'KG 接入与摄入（ingest.rs + index.rs）',
      '本体 + 推理（ontology/reason）+ URN（urn.rs）',
      '合并器（consolidator.rs）+ 治理 + 循环引擎（loop_engine.rs）'
    ],
    capabilities: ['kg.ingest', 'kg.reason', 'kg.governance']
  },
  // ===== Rust 16 crate engine:: 正式条目（璇玑三注册表联动 · 跨语言 · 全字段补齐 · 三闸门合规） =====
  // [P1-2] 字段合规模板：{ id, name, category, layer, codePath(相对 platform/ 根，ROOT 校验存在), keyFunctions≥3, capabilities, kind:'rust', engineName, crateId }
  {
    id: 'engine::ai_agent',
    name: 'Rust 引擎 · AI Agent 对话图谱',
    engineName: 'mox::ai_agent',
    kind: 'rust',
    category: 'intelligence',
    layer: '智能层（Rust crate）',
    crateId: '00374bdd-cc60-55bf-8970-a879afbfe443',
    codePath: 'services/ai-agent/src/lib.rs',
    keyFunctions: [
      '对话图状态机（dialogue_graph）：回合持久化 + 角色分离 + 对话上下文回溯',
      '工作流引擎（workflow_engine）：DAG 节点化 + 关键守卫 + 失败补偿队列',
      '插件总线（plugin_bus）：WASM / 原生算子双形态加载 + 版本隔离 + 熔断',
      '多智能体协作：消息总线 / 角色分工 / 投票收敛（multi_agent.rs）',
    ],
    capabilities: ['agent.dialogue', 'agent.workflow', 'agent.plugin_bus', 'agent.multi_agent']
  },
  {
    id: 'engine::business_catalog',
    name: 'Rust 引擎 · 业务目录中心',
    engineName: 'mox::business_catalog',
    kind: 'rust',
    category: 'knowledge',
    layer: '知识层（Rust crate）',
    crateId: '62b2cca1-d98f-5e41-b26e-8d2a43966117',
    codePath: 'services/business-catalog/src/lib.rs',
    keyFunctions: [
      '业务词条 CRUD + 多版本快照 + 幂等导入（catalog.rs）',
      '分类标签体系：多对多关联 / 分类树 / 标签推荐算法（taxonomy.rs）',
      '全文检索：倒排索引 + 前缀命中 + 语义向量双召回（search.rs）',
      '治理审计：创建/变更/下线 事件追溯 + 审批流钩子（governance.rs）',
    ],
    capabilities: ['catalog.crud', 'catalog.taxonomy', 'catalog.search', 'catalog.governance']
  },
  {
    id: 'engine::flow_ai',
    name: 'Rust 引擎 · FlowAI 流程智能',
    engineName: 'mox::flow_ai',
    kind: 'rust',
    category: 'orchestration',
    layer: '编排层（Rust crate）',
    crateId: '2fcd3eac-e894-5876-b007-fb33c56c0d65',
    codePath: 'services/flow-ai/src/lib.rs',
    keyFunctions: [
      '数据流/控制流双模型（dataflow.rs + controlflow.rs）：令牌驱动 + 回压',
      '关键路径分析：A* + 启发式，调度器公平队列（critpath.rs + schedule.rs）',
      '代码生成：冲突检测 + 算子融合 + 目标语言输出（codegen.rs + conflict.rs）',
      '可视化 DAG：拓扑布局 + 性能条带 + 热路径标注（dag/render.rs）',
    ],
    capabilities: ['flow.dataflow', 'flow.schedule', 'flow.critpath', 'flow.codegen']
  },
  {
    id: 'engine::graph_algorithms',
    name: 'Rust 引擎 · 图算法（生产级）',
    engineName: 'mox::graph_algorithms',
    kind: 'rust',
    category: 'intelligence',
    layer: '智能层（Rust crate）',
    crateId: 'fbd31c6a-41cd-5274-be2f-2a28066eaf0a',
    codePath: 'services/graph-algorithms/src/lib.rs',
    keyFunctions: [
      'PageRank 推模型（转置邻接 + 幂迭代 + 悬挂补偿，生产级 Δ≤1e-9）',
      'CNM 模块度贪心凝聚（社区检测，模块度稳定 ≥ karate 0.35）',
      'Brandes 介数中心性 + Harmonic 紧密中心性 + 度中心性（CNL 归一化）',
      'RAW 边展开：无向边双向展开避免度中心性减半（flow_graph.rs）',
    ],
    capabilities: ['graph.pagerank', 'graph.community_cnm', 'graph.centrality', 'graph.raw_sparse']
  },
  {
    id: 'engine::hermes_flow_bridge',
    name: 'Rust 引擎 · Hermes 流程桥',
    engineName: 'mox::hermes_flow_bridge',
    kind: 'rust',
    category: 'orchestration',
    layer: '编排层（Rust crate）',
    crateId: '9bfaf43b-385a-5a44-9fb2-65b4003ee80d',
    codePath: 'services/hermes-flow-bridge/src/lib.rs',
    keyFunctions: [
      '事件流桥接：Node.js 侧工作流 ↔ Rust FlowAI 双向事件分发（bridge.rs）',
      '事务 SAGA：补偿事务 / Outbox 模式 / 幂等去重（saga.rs + outbox.rs）',
      '可靠性：重试退避 / 死信队列 / 背压协议（reliability.rs）',
      '追踪 OpenTelemetry：B3/TraceContext 注入，跨进程链路（tracing.rs）',
    ],
    capabilities: ['bridge.events', 'bridge.saga', 'bridge.reliability', 'bridge.otel']
  },
  {
    id: 'engine::kg_hub',
    name: 'Rust 引擎 · 知识图谱中枢',
    engineName: 'mox::kg_hub',
    kind: 'rust',
    category: 'knowledge',
    layer: '知识层（Rust crate）',
    crateId: 'cb909f06-c0df-55ec-b397-543623a8c349',
    codePath: 'services/kg-hub/src/lib.rs',
    keyFunctions: [
      'KG 摄入流水线：ETL + 增量 / 版本快照 + URN 持久化（ingest.rs + index.rs）',
      '本体与推理：TBox/ABox 分层 + 规则推理（ontology/ + reason/）',
      '合并治理：跨源实体融合 + 冲突裁决 + 治理门（consolidator.rs + governance.rs）',
      '循环引擎：持续摄入 → 推理 → 质量评估 → 再摄入闭环（loop_engine.rs）',
    ],
    capabilities: ['kg.ingest', 'kg.reason', 'kg.consolidate', 'kg.governance']
  },
  {
    id: 'engine::operator_core',
    name: 'Rust 引擎 · 算子核心',
    engineName: 'mox::operator_core',
    kind: 'rust',
    category: 'orchestration',
    layer: '编排层（Rust crate）',
    crateId: 'acf14283-3931-5528-adce-2c0cd3815363',
    codePath: 'services/operator-core/src/lib.rs',
    keyFunctions: [
      '算子 Monad + 资源容器：纯函数式组合（operator.rs + monad.rs + resource.rs）',
      '守恒律校验：输入/输出/副作用三变量质量守恒（conservation.rs）',
      '算子注册表：按类别 / 能力标签索引（registry.rs + category.rs）',
      '执行引擎：热路径 JIT 调度 / 冷路径解释执行（engine.rs）',
    ],
    capabilities: ['operator.monad', 'operator.conservation', 'operator.registry', 'operator.exec']
  },
  {
    id: 'engine::operator_wasm',
    name: 'Rust 引擎 · 算子 WASM 沙箱',
    engineName: 'mox::operator_wasm',
    kind: 'rust',
    category: 'automation',
    layer: '自动化层（Rust crate）',
    crateId: '5a1df407-b217-5340-a5ae-5f4535d1e6de',
    codePath: 'services/operator-wasm/src/lib.rs',
    keyFunctions: [
      'WASM 算子装载：wat/wasm 双格式 + 签名校验 + 版本锁定（loader.rs）',
      '沙箱隔离：线性内存 / 能力限制 / CPU 指令计数限频（sandbox.rs）',
      'Host 接口回调：I/O / 随机 / 时间 三类白名单（host_api.rs）',
      '沙箱指标：调用计数 / 峰值内存 / 执行耗时（metrics.rs）',
    ],
    capabilities: ['op_wasm.loader', 'op_wasm.sandbox', 'op_wasm.host_api', 'op_wasm.metrics']
  },
  {
    id: 'engine::optimizer',
    name: 'Rust 引擎 · 算子优化器',
    engineName: 'mox::optimizer',
    kind: 'rust',
    category: 'optimization',
    layer: '优化层（Rust crate）',
    crateId: 'e56676c7-ec1f-5415-9587-ba8249d0178a',
    codePath: 'services/optimizer/src/lib.rs',
    keyFunctions: [
      'Pass 管线：图融合 / 重排 / 公共子表达式消除 CSE / 强度削弱（passes/）',
      'Cost-based 搜索：运行 trace 学习 + 代价模型启发式（cost/ + learn/）',
      '热路径应用：优化计划序列化 + AOT + 运行时回滚（serial.rs + apply.rs）',
      '性能画像：基准 + 抖动 + 异常检测（profile.rs）',
    ],
    capabilities: ['optimize.passes', 'optimize.cost_based', 'optimize.serial', 'optimize.profile']
  },
  {
    id: 'engine::primiflow_core',
    name: 'Rust 引擎 · PrimiFlow 核心',
    engineName: 'mox::primiflow_core',
    kind: 'rust',
    category: 'automation',
    layer: '自动化层（Rust crate）',
    crateId: '8c8d2382-6f9f-5218-894e-a07a43aa9554',
    codePath: 'services/primiflow-core/src/lib.rs',
    keyFunctions: [
      'DSL 解析：PrimiFlow 文法 + 错误恢复 + LSP 式诊断（parse.rs + diagnostic.rs）',
      '代码生成：多目标 gen/*（TS / Py / SQL / Rust）+ 语义检查（generate.rs）',
      '执行器与持久化：executor.rs 增量 checkpoint + persistence.rs journal',
      'Trace Matrix & Schema：执行轨迹 & 合约结构生成（trace_matrix/ + schema/）',
    ],
    capabilities: ['primiflow.parse', 'primiflow.generate', 'primiflow.exec', 'primiflow.trace_schema']
  },
  {
    id: 'engine::primiflow_fusion',
    name: 'Rust 引擎 · PrimiFlow 六维融合',
    engineName: 'mox::primiflow_fusion',
    kind: 'rust',
    category: 'automation',
    layer: '自动化层（Rust crate）',
    crateId: '75238345-b48b-534b-818b-8d9abe083a41',
    codePath: 'services/primiflow-fusion/src/lib.rs',
    keyFunctions: [
      '六维融合体系：结构/时序/上下文/语义/能力/形态（sixdim.rs）',
      '统一包络：envelope/* 与 unified/* 多协议适配（统一对外 gRPC/REST/SSE）',
      '平台服务注册：服务发现 + 健康探活 + 版本对齐（registry.rs）',
      'PTDoc 产线：六维融合自动文档 + 诊断报告（ptdoc.rs + observability.rs）',
    ],
    capabilities: ['fusion.sixdim', 'fusion.envelope', 'fusion.registry', 'fusion.ptdoc']
  },
  {
    id: 'engine::template_market',
    name: 'Rust 引擎 · 模板市场',
    engineName: 'mox::template_market',
    kind: 'rust',
    category: 'knowledge',
    layer: '知识层（Rust crate）',
    crateId: '4d2e50c1-9d64-525d-86cf-2d7d610a27b9',
    codePath: 'services/template-market/src/lib.rs',
    keyFunctions: [
      '模板注册：签名校验 / 语义标签 / 版本化（template_register.rs）',
      '分发与部署：一键发布 / 灰度 / 回滚（deploy.rs + release.rs）',
      '评分与评论：加权评分（质量×新颖度×实用性）+ 反灌水（rating.rs）',
      '市场索引：倒排 + 推荐 + 热度（search.rs + recommend.rs）',
    ],
    capabilities: ['market.register', 'market.deploy', 'market.rating', 'market.search']
  },
  {
    id: 'engine::mox_expert',
    name: 'Rust 引擎 · 璇玑专家平台',
    engineName: 'mox::mox_expert',
    kind: 'rust',
    category: 'collaboration',
    layer: '协作层（Rust crate）',
    crateId: '50bb6200-04c5-5e4c-8354-4c6e1b230024',
    codePath: 'services/mox-expert/src/lib.rs',
    keyFunctions: [
      '15 专家画像 + 15× 验证管线（experts/* + verify/*）',
      '六阶段流水线：RBAC 门禁 → 管线装载 → 执行器 → 拓扑/冲突/数据依赖/增益 验证',
      '审计与多租户：操作日志 / 租户策略 / 配额（audit/ + tenant_policy.rs）',
      '管线指标：成功率 / 平均耗时 / 并发度（metrics.rs）',
    ],
    capabilities: ['expert.pipeline', 'expert.audit', 'expert.tenant', 'expert.validate']
  },
  {
    id: 'engine::mox_system',
    name: 'Rust 引擎 · 璇玑系统底座',
    engineName: 'mox::mox_system',
    kind: 'rust',
    category: 'infrastructure',
    layer: '基础设施（Rust crate）',
    crateId: 'b81eec75-22ff-5155-ac49-19edf6f6b5ab',
    codePath: 'services/mox-system/src/lib.rs',
    keyFunctions: [
      '服务编排器：多后端存储（Postgres/SQLite/Sled）抽象（orchestrator.rs + repo/*）',
      'RBAC / 加密 / 限流：角色权限 + AES-256-GCM + 令牌桶（rbac/crypto/ratelimit）',
      '配置/错误/指标/事件：全栈统一 crate（config/error/metrics/event）',
      '优雅启停：预关闭钩子 + 健康检查 + 连接池回收（lifecycle.rs）',
    ],
    capabilities: ['system.storage', 'system.rbac_crypto', 'system.ratelimit', 'system.lifecycle']
  },
  {
    id: 'engine::mox_common_meta',
    name: 'Rust 引擎 · 璇玑公共元数据',
    engineName: 'mox::mox_common_meta',
    kind: 'rust',
    category: 'infrastructure',
    layer: '基础设施（Rust crate）',
    crateId: '34a20231-1a80-5426-b392-40d7a2ddd9f7',
    codePath: 'services/mox-common-meta/src/lib.rs',
    keyFunctions: [
      '共享类型：能力 ID / 能力矩阵 / 意图清单 常量定义（types/*.rs）',
      '统一格式：URN 解析 / 序列化 / 校验（urn.rs）',
      '跨版本兼容：语义版本 + 兼容矩阵（semver.rs + compat_matrix.rs）',
      '语言无关元契约：OpenAPI / AsyncAPI / TraceContext 生成器（contracts.rs）',
    ],
    capabilities: ['meta.types', 'meta.urn', 'meta.semver', 'meta.contracts']
  },
  {
    id: 'engine::runtime',
    name: 'Rust 引擎 · 网关运行时（Cordis）',
    engineName: 'mox::runtime',
    kind: 'rust',
    category: 'infrastructure',
    layer: '基础设施（Rust crate）',
    crateId: 'a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b',
    codePath: 'gateway/runtime/src/lib.rs',
    keyFunctions: [
      'Cordis 插件运行时：bundle 打包 / lifecycle 生命周期 / event_bus / seam 解耦 / profile',
      'HITL 人机协同 WebSocket + 决议回传 + RBAC 中间件（handlers/hitl + rbac_middleware）',
      '治理 / 市场 / Agent 三域路由：统一 REST 规范 + 响应 envelope（routes/*）',
      'OpenAPI 3.0 自动生成 + Schema 校验（openapi.rs + validator.rs）',
    ],
    capabilities: ['runtime.plugin', 'runtime.hitl', 'runtime.routes', 'runtime.openapi']
  }
];

// ---- 自动追加：flow-registry 中引用但清单里尚未登记的引擎（保持 T9 契约一致）----
(function appendFlowEngines(list) {
  const present = new Set(list.map(e => e.id));
  const need = [
    {
      id: 'project-atlas',
      name: '项目全息图谱引擎（Project Atlas）',
      category: 'orchestration',
      layer: '编排层',
      codePath: 'src/project-atlas/index.js',
      keyFunctions: [
        '资产四分类扫描 / 差量计算 / 自动登记 / 图谱重建 / 无破窗复验（W1-W13）',
        '需求归一化流水线（NR1-NR7 全阶）',
        '代码图谱联动：扫描 → 绑定 → 三方对账 → 自愈归一',
      ],
      capabilities: ['scan', 'verify', 'normalize', 'self-sync'],
    },
    {
      id: 'engine-universe',
      name: '引擎宇宙（能力路由承接层）',
      category: 'orchestration',
      layer: '编排层',
      codePath: 'src/engine-universe/index.js',
      keyFunctions: [
        '承接引擎注册表枚举 / 能力路由 / 多引擎协同调度',
        '归一化流量分发到专家/图谱/流程等能力域',
        '全域节点索引 NODE_INDEX + ALL_NODES 唯一真相，按 ID 毫秒级查询',
        '边全量校验 + 需求链连通性 + 降级链收敛性 一键验证（verifyFullChain）',
      ],
      capabilities: ['list-engines', 'route', 'decompose', 'verify'],
    },
    {
      id: 'engine-kernel',
      name: '引擎内核（安全切换 / 槽位绑定）',
      category: 'infrastructure',
      layer: '基础设施',
      codePath: 'src/engine-kernel/index.js',
      keyFunctions: [
        '切换安全流程：校验→切换→探活→回滚→优雅切流 五段式',
        '市场/插件/契约三位一体联动：槽位 1..4 版本对齐 + 冲突检测',
        '热替换钩子：预启动 / afterReady / beforeShutdown 三挂钩',
        '内核指标：成功率 p99 / 切换耗时 / 回滚次数 / 槽位占用率',
      ],
      capabilities: ['switch', 'validate', 'probe', 'rollback'],
    },
    {
      id: 'gateway-runtime',
      name: '网关运行时（接入 HITL / WebSocket / action 决议）',
      category: 'infrastructure',
      layer: '基础设施',
      codePath: 'gateway/runtime/src/lib.rs',
      keyFunctions: [
        'HITL 待审事项登记/广播/决议回传（RBAC 按资源+动作维度鉴权）',
        '接入层 WebSocket 重连/指数退避 / AI Engine 处理握手',
        '统一 REST/gRPC/SSE 三协议网关：限流熔断 + 路由分域',
        'OpenAPI 3.0 + AsyncAPI 自动契约 + 网关级请求追踪（X-Request-ID）',
      ],
      capabilities: ['hitl', 'ws-broadcast', 'ai-engine-handler', 'openapi'],
    },
    {
      id: 'flow-engine',
      name: '流程引擎（Flow Engine · Rust 侧）',
      category: 'orchestration',
      layer: '编排层',
      codePath: 'services/ai-agent/src/flow_engine.rs',
      keyFunctions: [
        'Rust 侧流程编排：节点化 DAG + 守卫 Guards + 状态机 State Machine',
        'Hermes Bridge 集成事件流：跨进程异步 StepComplete 信号回传',
        '执行快照 + 断点续跑 + 失败补偿 SAGA 队列',
        '流程指标：成功率 p95 / 步均耗时 / 瓶颈节点 Top-N / 补偿队列深度',
      ],
      capabilities: ['run-flow', 'guards', 'state-machine', 'saga-compensate'],
    },
  ];
  for (const e of need) if (!present.has(e.id)) list.push(e);
})(ENGINES);

const ENGINE_INDEX = Object.fromEntries(ENGINES.map(e => [e.id, e]));

/** 按类别分组（供分组查询） */
const CATEGORY_ORDER = [
  ['infrastructure', '基础设施'],
  ['orchestration', '编排层'],
  ['intelligence', '智能层'],
  ['collaboration', '协作层'],
  ['automation', '自动化层'],
  ['optimization', '优化层'],
  ['knowledge', '知识层']
];

function getEngine(id) {
  return ENGINE_INDEX[id] || null;
}

function listEngines(filters = {}) {
  let list = ENGINES;
  if (filters.category) list = list.filter(e => e.category === filters.category);
  if (filters.capability) list = list.filter(e => (e.capabilities || []).includes(filters.capability));
  return list;
}

module.exports = { ENGINES, ENGINE_INDEX, CATEGORY_ORDER, getEngine, listEngines };
