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
  }
];

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
