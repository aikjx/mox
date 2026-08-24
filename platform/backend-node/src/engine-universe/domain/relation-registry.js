'use strict';

/**
 * 引擎宇宙关系注册表（domain 层 · 静态值对象 · 零 IO）
 * ------------------------------------------------------------------
 * 边类型（与流程图谱引擎四类边对齐 + 扩展）：
 *   depends_on    运行时依赖（A 装配/调用 B）
 *   delegates_to  能力委托（A 把某能力委托 B 执行）
 *   degrades_to   降级链（A 失败时单向回退到 B）
 *   data_flows_to 数据流（A 的产出流入 B）
 *   serves        需求服务（引擎服务需求归一化链的某一环）
 *   flows_to      需求链内部流转（与知识图谱 n_* 节点一致）
 *
 * 需求归一化链（与知识图谱 graph_nodes.json 中 n_* 节点严格对应）：
 *   n_ingest 需求采集 → n_norm 归一化 IR → n_disp 双联盟十四维特派
 *   → n_rec 归一化裁决 → n_gate 璇玑验证网关
 */

// ---------- 需求归一化链节点（引用知识图谱既有节点，单一真相源声明） ----------
const REQUIREMENT_NODES = [
  { id: 'n_ingest', label: '需求采集', stage: 'ingest' },
  { id: 'n_norm', label: '归一化 IR', stage: 'normalize' },
  { id: 'n_disp', label: '双联盟十四维特派', stage: 'dispatch' },
  { id: 'n_rec', label: '归一化裁决', stage: 'resolve' },
  { id: 'n_gate', label: '璇玑验证网关', stage: 'gate' }
];

// ---------- 引擎间关系边 ----------
const ENGINE_EDGES = [
  // 编排核心的装配依赖与能力委托
  { from: 'ai-engine-core', to: 'llm-gateway', type: 'depends_on', note: '网关装配 + chat 兜底' },
  { from: 'ai-engine-core', to: 'ai-engine', type: 'delegates_to', note: 'graph/workflow 能力委托' },
  { from: 'ai-engine-core', to: 'ultimate-ai-engine', type: 'delegates_to', note: 'reasoning/memory 能力委托' },
  { from: 'ai-engine-core', to: 'expert-alliance-engine', type: 'delegates_to', note: 'expert 能力委托' },
  { from: 'ai-engine-core', to: 'ai-flow-graph', type: 'depends_on', note: '流程图谱注入能力矩阵（激活扩散意图识别）' },
  { from: 'ai-engine-core', to: 'llm-gateway', type: 'degrades_to', note: '能力失败降级 chat（不变式②）' },

  // 图谱智能链
  { from: 'ai-engine', to: 'ai-integration-engine', type: 'delegates_to', note: 'PageRank 单源委托（A18）' },
  { from: 'ai-engine', to: 'llm-gateway', type: 'depends_on', note: 'AI 结论生成' },
  { from: 'ai-engine', to: 'expert-alliance', type: 'delegates_to', note: '图谱问题转专家协作（延迟边）' },
  { from: 'ai-integration-engine', to: 'llm-gateway', type: 'depends_on', note: '符号图 LLM 交互' },

  // 记忆与推理链
  { from: 'ultimate-ai-engine', to: 'llm-gateway', type: 'depends_on', note: '推理步骤 LLM 调用' },
  { from: 'session-store', to: 'ultimate-ai-engine', type: 'data_flows_to', note: '会话语义索引复用向量记忆实现' },

  // 专家协作链
  { from: 'expert-alliance-engine', to: 'expert-alliance', type: 'depends_on', note: '联盟域包装配' },
  { from: 'expert-alliance-engine', to: 'expert-graph', type: 'depends_on', note: '协同增益沿能力图边计算' },
  { from: 'expert-alliance-engine', to: 'expert-dispatcher', type: 'depends_on', note: '负载均衡权重来源' },
  { from: 'expert-alliance-engine', to: 'llm-gateway', type: 'degrades_to', note: '辩论失败降级单专家咨询' },
  { from: 'expert-alliance', to: 'llm-gateway', type: 'delegates_to', note: 'consult/multiExpertConsult 委托网关' },
  { from: 'expert-alliance', to: 'orchestration-engine', type: 'depends_on', note: 'V2 编排代理底座' },
  { from: 'expert-graph', to: 'ai-engine', type: 'delegates_to', note: 'CNM 社区检测单源委托（A19）' },

  // 自动化开发链
  { from: 'auto-dev-engine', to: 'llm-gateway', type: 'depends_on', note: '架构图谱 JSON 生成' },
  { from: 'auto-dev-engine', to: 'knowledge-graph', type: 'data_flows_to', note: '架构图谱存入图谱引擎统一管理' },
  { from: 'auto-dev-engine', to: 'kb', type: 'data_flows_to', note: '制品与文档沉淀知识库' },

  // 优化与搜索链
  { from: 'infinite-dimension-optimizer', to: 'llm-gateway', type: 'depends_on', note: '基准任务评测调用' },
  { from: 'infinite-dimension-optimizer', to: 'llm-gateway', type: 'data_flows_to', note: '最优配置（激活引擎/softmax 路由权重/温度）持久化生效到网关' },
  { from: 'web-search-service', to: 'llm-gateway', type: 'data_flows_to', note: '搜索结果注入网关上下文' },
  { from: 'llm-gateway', to: 'session-store', type: 'data_flows_to', note: '对话产出沉淀会话记忆（sessionId 历史链）' },

  // 流程图谱与知识层
  { from: 'ai-flow-graph', to: 'ai-integration-engine', type: 'delegates_to', note: 'F8 激活扩散委托图计算（延迟边）' },
  { from: 'kb', to: 'knowledge-graph', type: 'data_flows_to', note: '文档实体与图谱节点互链（graphLinks）' },
  { from: 'engine-universe', to: 'knowledge-graph', type: 'data_flows_to', note: '引擎宇宙图谱由图谱引擎统一承载' },

  // W8 兜底：flow-engine（Rust 侧流程引擎）接入主连通分量 → 编排引擎 + ai-engine-core（避免孤岛）
  { from: 'orchestration-engine', to: 'flow-engine', type: 'delegates_to', note: '复杂流程状态机委托 Rust flow-engine（xuanji-system 协同）' },
  { from: 'flow-engine', to: 'ai-engine-core', type: 'depends_on', note: '流程引擎复用 AI Engine Core 的 LLM/会话侧车' },

  // ========== [P1-2 V6 孤岛消除] engine-rust-* ↔ engine::* 等价边 16 条（跨语言一对一承接，双注册表联动）==========
  { from: 'engine-rust-ai-agent',         to: 'engine::ai_agent',           type: 'equivalent_to', note: 'JS 层 Rust AI Agent 承接点 ↔ Rust crate 正式条目' },
  { from: 'engine-rust-xuanji-expert',    to: 'engine::xuanji_expert',      type: 'equivalent_to', note: 'Rust 璇玑专家平台 JS 视图 ↔ Rust crate' },
  { from: 'engine-rust-xuanji-system',    to: 'engine::xuanji_system',      type: 'equivalent_to', note: 'Rust 系统底座 ↔ Rust crate 正式条目' },
  { from: 'engine-rust-runtime',          to: 'engine::runtime',            type: 'equivalent_to', note: 'Rust 网关运行时 Cordis ↔ Rust crate engine::runtime' },
  { from: 'engine-rust-graph-algorithms', to: 'engine::graph_algorithms',   type: 'equivalent_to', note: 'Rust 图算法引擎 ↔ Rust crate engine::graph_algorithms' },
  { from: 'engine-rust-flow-ai',          to: 'engine::flow_ai',            type: 'equivalent_to', note: 'Rust FlowAI 流程智能 ↔ Rust crate engine::flow_ai' },
  { from: 'engine-rust-optimizer',        to: 'engine::optimizer',          type: 'equivalent_to', note: 'Rust 算子优化器 ↔ Rust crate engine::optimizer' },
  { from: 'engine-rust-operator-core',    to: 'engine::operator_core',      type: 'equivalent_to', note: 'Rust 算子核心 ↔ Rust crate engine::operator_core' },
  { from: 'engine-rust-primiflow-core',   to: 'engine::primiflow_core',     type: 'equivalent_to', note: 'Rust PrimiFlow 核心 ↔ Rust crate engine::primiflow_core' },
  { from: 'engine-rust-primiflow-fusion', to: 'engine::primiflow_fusion',   type: 'equivalent_to', note: 'Rust PrimiFlow 六维融合 ↔ Rust crate engine::primiflow_fusion' },
  { from: 'engine-rust-kg-hub',           to: 'engine::kg_hub',             type: 'equivalent_to', note: 'Rust 知识图谱中枢 ↔ Rust crate engine::kg_hub' },
  // engine::* 中另外 5 条 engine-rust 条目尚未建（business_catalog / hermes_flow_bridge / operator_wasm / template_market / xuanji_common_meta），这里补接入
  { from: 'engine-rust-xuanji-system',    to: 'engine::business_catalog',   type: 'delegates_to', note: '业务目录作为 xuanji-system 的子能力（跨源数据统一目录）' },
  { from: 'engine-rust-flow-ai',          to: 'engine::hermes_flow_bridge', type: 'data_flows_to', note: 'FlowAI 事件流通过 Hermes Bridge 跨进程分发' },
  { from: 'engine-rust-operator-core',    to: 'engine::operator_wasm',      type: 'depends_on', note: '算子核心依赖 WASM 沙箱执行用户算子' },
  { from: 'engine-rust-graph-algorithms', to: 'engine::template_market',    type: 'data_flows_to', note: '算法模板发布通过 Template Market 进行分发' },
  { from: 'engine-rust-xuanji-system',    to: 'engine::xuanji_common_meta', type: 'depends_on', note: '系统底座依赖公共元数据常量与 URN 解析' },

  // ========== [P1-2 V6 孤岛消除] JS 侧 5 个追加节点（appendFlowEngines）接入主链 ==========
  { from: 'ai-engine-core', to: 'engine-kernel',   type: 'depends_on', note: 'AI Core 装配引擎内核：能力执行安全切换与热替换' },
  { from: 'gateway-runtime', to: 'engine::runtime', type: 'depends_on', note: 'JS 网关运行时视图装配 Rust Cordis runtime 实现（通过 engine::* 索引）' },
  { from: 'gateway-runtime', to: 'ai-engine-core',  type: 'data_flows_to', note: 'HITL 审批决议回传到 AI Engine Core 能力执行继续' },
  { from: 'project-atlas',   to: 'kb',              type: 'data_flows_to', note: '项目资产扫描结果入知识库（文档/版本 diff）' },
  { from: 'project-atlas',   to: 'knowledge-graph', type: 'data_flows_to', note: '项目资产四分类 / 需求归一化流水线 映射为图谱节点与边' },
  { from: 'project-atlas',   to: 'ai-engine-core',  type: 'serves', note: 'Project Atlas 为 AI 核心扫描与重建全系统资产依赖图（采集→归一化链上游）' },

  // ========== [P1-2 V6 孤岛消除] FlowAI / Hermes / xuanji_expert 跨层桥（主分量 → Rust 分量）==========
  { from: 'orchestration-engine',   to: 'engine-rust-flow-ai',       type: 'delegates_to', note: 'V2 编排把关键路径/SAGA 调度委托 Rust FlowAI（性能敏感）' },
  { from: 'ai-integration-engine',  to: 'engine-rust-graph-algorithms', type: 'delegates_to', note: 'JS PageRank 图计算超时 → 委托 Rust 加速（A18 双实现收口）' },
  { from: 'expert-alliance-engine', to: 'engine-rust-xuanji-expert', type: 'delegates_to', note: '联盟咨询的验证管线（拓扑/冲突/数据依赖）委托 Rust' },
  { from: 'infinite-dimension-optimizer', to: 'engine-rust-optimizer', type: 'delegates_to', note: 'CEM 高维寻优在 Rust 算子级 Pass 管线运行（冷启动 warmup）' },
  { from: 'knowledge-graph',        to: 'engine-rust-kg-hub',        type: 'data_flows_to', note: 'JS 知识图谱向 Rust KG Hub 增量同步做跨源合并' },
  { from: 'auto-dev-engine',        to: 'engine-rust-primiflow-core', type: 'delegates_to', note: '确定性代码渲染委托 PrimiFlow 多目标代码生成（gen/*）' },
  { from: 'engine-rust-primiflow-core', to: 'engine-rust-primiflow-fusion', type: 'depends_on', note: 'PrimiFlow 核心执行依赖六维融合做统一包络对外服务' },
  { from: 'ai-engine-core',          to: 'engine-rust-ai-agent',      type: 'delegates_to', note: '跨工具对话 / 工作流 / 浏览器自动化 委托 Rust AI Agent（性能安全隔离）' },
  { from: 'engine-kernel',          to: 'engine-rust-operator-core', type: 'depends_on', note: '内核切换槽位的算子执行通过 Rust operator-core（守恒律校验）' },
  { from: 'engine-kernel',          to: 'engine-rust-xuanji-system', type: 'depends_on', note: '内核 RBAC / 加密 / 限流 依赖 Rust xuanji-system 组件' },
  { from: 'flow-engine',            to: 'engine::flow_ai',           type: 'equivalent_to', note: 'JS flow-engine 别名 ↔ Rust FlowAI crate（跨语言同一能力）' },
  { from: 'orchestration-engine',   to: 'engine::hermes_flow_bridge', type: 'data_flows_to', note: 'V2 编排跨进程事件通过 Hermes Bridge（Rust）可靠传递' },
  { from: 'session-store',          to: 'engine::runtime',           type: 'data_flows_to', note: '会话历史回放通过 Cordis Gateway WS 广播给多端协同' },
  { from: 'kb',                     to: 'engine::business_catalog',  type: 'data_flows_to', note: 'KB 文档分类 → 业务目录中心统一索引目录树 & 倒排' },
  { from: 'engine::kg_hub',         to: 'kb',                       type: 'data_flows_to', note: '跨源 KG 合并后，实体与文档通过 KB 的 graphLinks 关联' },
];

// ---------- 需求归一化链内部流转 ----------
const REQUIREMENT_EDGES = [
  { from: 'n_ingest', to: 'n_norm', type: 'flows_to', note: '原始需求进入归一化' },
  { from: 'n_norm', to: 'n_disp', type: 'flows_to', note: '归一化 IR 特派双联盟' },
  { from: 'n_disp', to: 'n_rec', type: 'flows_to', note: '联盟结论进入裁决' },
  { from: 'n_rec', to: 'n_gate', type: 'flows_to', note: '裁决结果过验证网关' }
];

// ---------- 引擎服务需求链（serves） ----------
const SERVICE_EDGES = [
  { from: 'web-search-service', to: 'n_ingest', type: 'serves', note: '联网检索采集外部需求信息' },
  { from: 'kb', to: 'n_ingest', type: 'serves', note: '文档/需求文本入库与实体抽取' },
  { from: 'ai-engine-core', to: 'n_norm', type: 'serves', note: '意图识别把原始请求归一化为能力 IR' },
  { from: 'ai-flow-graph', to: 'n_norm', type: 'serves', note: 'F8 激活扩散意图归一化' },
  { from: 'expert-alliance-engine', to: 'n_disp', type: 'serves', note: '双联盟多专家特派处理' },
  { from: 'expert-alliance', to: 'n_disp', type: 'serves', note: '专家个体咨询与辩论' },
  { from: 'ultimate-ai-engine', to: 'n_rec', type: 'serves', note: '多步推理归一化裁决' },
  { from: 'ai-engine-core', to: 'n_gate', type: 'serves', note: '质量校验门禁（非空/降级率/延迟）' },
  { from: 'knowledge-graph', to: 'n_gate', type: 'serves', note: '图谱结构化校验与追溯' }
];

// ---------- 引擎 ↔ 本地代码路径（code_of：代码归属声明） ----------
// codePath 已在 engine-registry 每个引擎声明；此处声明引擎的关键协作文件（多文件引擎）。
const CODE_ASSOCIATIONS = [
  { engine: 'ai-engine-core', files: ['src/ai-engine-core.js', 'src/routes/ai-engine.js'] },
  { engine: 'expert-alliance', files: ['src/expert-alliance/index.js', 'src/expert-alliance/domain/intent-patterns.js', 'src/expert-alliance/domain/intent-classifier.js', 'src/expert-alliance/domain/expert-matcher.js', 'src/expert-alliance/domain/debate-synthesis.js', 'src/expert-alliance/application/alliance-orchestrator.js', 'src/expert-alliance/infrastructure/expert-repository.js'] },
  { engine: 'kb', files: ['src/kb/index.js', 'src/kb/domain/document-analyzer.js', 'src/kb/domain/version-differ.js', 'src/kb/infrastructure/kb-store.js', 'src/routes/kb.js'] },
  { engine: 'knowledge-graph', files: ['src/lib/graph-algos.js', 'src/routes/graph.js', 'src/lib/json-store.js'] },
  { engine: 'expert-alliance-engine', files: ['src/expert-alliance-engine.js', 'src/routes/expert-alliance.js'] },
  { engine: 'auto-dev-engine', files: ['src/auto-dev-engine.js', 'src/routes/auto-dev.js'] },
  { engine: 'infinite-dimension-optimizer', files: ['src/infinite-dimension-optimizer.js', 'src/routes/optimizer.js'] },
  { engine: 'web-search-service', files: ['src/web-search-service.js', 'src/routes/web-search.js'] },
  { engine: 'ai-flow-graph', files: ['src/ai-flow-graph.js'] },
  { engine: 'ultimate-ai-engine', files: ['src/ultimate-ai-engine.js', 'src/routes/ai-ultimate.js'] },
  { engine: 'ai-engine', files: ['src/ai-engine.js'] },
  { engine: 'ai-integration-engine', files: ['src/ai-integration-engine.js', 'src/routes/ai-integrated.js'] },
  { engine: 'orchestration-engine', files: ['src/orchestration-engine.js', 'src/routes/orchestration.js'] },
  { engine: 'session-store', files: ['src/session-store.js'] },
  { engine: 'llm-gateway', files: ['src/llm-gateway.js', 'src/routes/integration.js'] },
  { engine: 'expert-graph', files: ['src/expert-graph.js', 'src/routes/expert-graph.js'] },
  { engine: 'expert-dispatcher', files: ['src/expert-dispatcher.js'] }
];

const ALL_EDGES = [...ENGINE_EDGES, ...REQUIREMENT_EDGES, ...SERVICE_EDGES];

module.exports = {
  REQUIREMENT_NODES,
  ENGINE_EDGES,
  REQUIREMENT_EDGES,
  SERVICE_EDGES,
  CODE_ASSOCIATIONS,
  ALL_EDGES
};
