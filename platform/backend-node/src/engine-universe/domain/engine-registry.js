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
    codePath: '../services/ai-agent/src',
    keyFunctions: [
      '对话图 + 工作流 + 工具总线（dialogue_graph/workflow_engine/plugin_bus）',
      '多智能体协作引擎（multi_agent.rs · 角色分工 + 消息总线）',
      '浏览器自动化与资源管理（browser_automation / resource_manager）'
    ],
    capabilities: ['agent.dialogue', 'agent.tool_use', 'agent.multi_agent']
  },
  {
    id: 'engine-rust-xuanji-expert',
    name: 'Rust 璇玑专家引擎',
    category: 'collaboration',
    layer: '协作层（Rust）',
    codePath: '../services/xuanji-expert/src',
    keyFunctions: [
      '15 专家画像 + 15× 验证管线（experts/* + verify/*）',
      '六阶段流水线：RBAC 门禁 → 管线装载 → 执行器 → 拓扑/冲突/数据依赖/增益 验证',
      '审计与多租户（audit/ + tenant_policy.rs）'
    ],
    capabilities: ['expert.audit', 'expert.pipeline', 'expert.validate']
  },
  {
    id: 'engine-rust-xuanji-system',
    name: 'Rust 璇玑系统底座',
    category: 'infrastructure',
    layer: '基础设施（Rust）',
    codePath: '../services/xuanji-system/src',
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
    codePath: '../gateway/runtime/src',
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
    codePath: '../services/graph-algorithms/src',
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
    codePath: '../services/flow-ai/src',
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
    codePath: '../services/optimizer/src',
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
    codePath: '../services/operator-core/src',
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
    codePath: '../services/primiflow-core/src',
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
    codePath: '../services/primiflow-fusion/src',
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
    codePath: '../services/kg-hub/src',
    keyFunctions: [
      'KG 接入与摄入（ingest.rs + index.rs）',
      '本体 + 推理（ontology/reason）+ URN（urn.rs）',
      '合并器（consolidator.rs）+ 治理 + 循环引擎（loop_engine.rs）'
    ],
    capabilities: ['kg.ingest', 'kg.reason', 'kg.governance']
  },
  // ===== Rust 16 crate engine:: 正式条目（璇玑三注册表联动 · 跨语言） =====
  {
    id: 'engine::ai_agent',
    engineName: 'xuanji::ai_agent',
    kind: 'rust',
    crateId: '00374bdd-cc60-55bf-8970-a879afbfe443',
    path: 'platform/services/ai-agent/src/lib.rs'
  },
  {
    id: 'engine::business_catalog',
    engineName: 'xuanji::business_catalog',
    kind: 'rust',
    crateId: '62b2cca1-d98f-5e41-b26e-8d2a43966117',
    path: 'platform/services/business-catalog/src/lib.rs'
  },
  {
    id: 'engine::flow_ai',
    engineName: 'xuanji::flow_ai',
    kind: 'rust',
    crateId: '2fcd3eac-e894-5876-b007-fb33c56c0d65',
    path: 'platform/services/flow-ai/src/lib.rs'
  },
  {
    id: 'engine::graph_algorithms',
    engineName: 'xuanji::graph_algorithms',
    kind: 'rust',
    crateId: 'fbd31c6a-41cd-5274-be2f-2a28066eaf0a',
    path: 'platform/services/graph-algorithms/src/lib.rs'
  },
  {
    id: 'engine::hermes_flow_bridge',
    engineName: 'xuanji::hermes_flow_bridge',
    kind: 'rust',
    crateId: '9bfaf43b-385a-5a44-9fb2-65b4003ee80d',
    path: 'platform/services/hermes-flow-bridge/src/lib.rs'
  },
  {
    id: 'engine::kg_hub',
    engineName: 'xuanji::kg_hub',
    kind: 'rust',
    crateId: 'cb909f06-c0df-55ec-b397-543623a8c349',
    path: 'platform/services/kg-hub/src/lib.rs'
  },
  {
    id: 'engine::operator_core',
    engineName: 'xuanji::operator_core',
    kind: 'rust',
    crateId: 'acf14283-3931-5528-adce-2c0cd3815363',
    path: 'platform/services/operator-core/src/lib.rs'
  },
  {
    id: 'engine::operator_wasm',
    engineName: 'xuanji::operator_wasm',
    kind: 'rust',
    crateId: '5a1df407-b217-5340-a5ae-5f4535d1e6de',
    path: 'platform/services/operator-wasm/src/lib.rs'
  },
  {
    id: 'engine::optimizer',
    engineName: 'xuanji::optimizer',
    kind: 'rust',
    crateId: 'e56676c7-ec1f-5415-9587-ba8249d0178a',
    path: 'platform/services/optimizer/src/lib.rs'
  },
  {
    id: 'engine::primiflow_core',
    engineName: 'xuanji::primiflow_core',
    kind: 'rust',
    crateId: '8c8d2382-6f9f-5218-894e-a07a43aa9554',
    path: 'platform/services/primiflow-core/src/lib.rs'
  },
  {
    id: 'engine::primiflow_fusion',
    engineName: 'xuanji::primiflow_fusion',
    kind: 'rust',
    crateId: '75238345-b48b-534b-818b-8d9abe083a41',
    path: 'platform/services/primiflow-fusion/src/lib.rs'
  },
  {
    id: 'engine::template_market',
    engineName: 'xuanji::template_market',
    kind: 'rust',
    crateId: '4d2e50c1-9d64-525d-86cf-2d7d610a27b9',
    path: 'platform/services/template-market/src/lib.rs'
  },
  {
    id: 'engine::xuanji_expert',
    engineName: 'xuanji::xuanji_expert',
    kind: 'rust',
    crateId: '50bb6200-04c5-5e4c-8354-4c6e1b230024',
    path: 'platform/services/xuanji-expert/src/lib.rs'
  },
  {
    id: 'engine::xuanji_system',
    engineName: 'xuanji::xuanji_system',
    kind: 'rust',
    crateId: 'b81eec75-22ff-5155-ac49-19edf6f6b5ab',
    path: 'platform/services/xuanji-system/src/lib.rs'
  },
  {
    id: 'engine::xuanji_common_meta',
    engineName: 'xuanji::xuanji_common_meta',
    kind: 'rust',
    crateId: '34a20231-1a80-5426-b392-40d7a2ddd9f7',
    path: 'platform/services/xuanji-common-meta/src/lib.rs'
  },
  {
    id: 'engine::runtime',
    engineName: 'xuanji::runtime',
    kind: 'rust',
    crateId: 'a6f7ad5c-dbc8-5c27-837f-d8332fd6f27b',
    path: 'platform/gateway/runtime/src/lib.rs'
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
      ],
      capabilities: ['list-engines', 'route', 'decompose'],
    },
    {
      id: 'engine-kernel',
      name: '引擎内核（安全切换 / 槽位绑定）',
      category: 'infrastructure',
      layer: '基础设施',
      codePath: 'src/engine-kernel/index.js',
      keyFunctions: [
        '切换安全流程（校验→切换→探活→回滚→优雅切流）',
        '市场/插件/契约三位一体联动',
      ],
      capabilities: ['switch', 'validate', 'probe', 'rollback'],
    },
    {
      id: 'gateway-runtime',
      name: '网关运行时（接入 HITL / WebSocket / action 决议）',
      category: 'infrastructure',
      layer: '基础设施',
      codePath: 'platform/gateway/runtime/src/lib.rs',
      keyFunctions: [
        'HITL 待审事项登记/广播/决议回传',
        '接入层 WebSocket 重连/退避 / AI Engine 处理',
      ],
      capabilities: ['hitl', 'ws-broadcast', 'ai-engine-handler'],
    },
    {
      id: 'flow-engine',
      name: '流程引擎（Flow Engine）',
      category: 'orchestration',
      layer: '编排层',
      codePath: 'platform/services/ai-agent/src/flow_engine.rs',
      keyFunctions: [
        'Rust 侧流程编排 + 守卫/状态机/多代理协同',
        '与 Hermes Bridge 集成事件流',
      ],
      capabilities: ['run-flow', 'guards', 'state-machine'],
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
