'use strict'
/**
 * 初始化示例数据。仅当对应 collection 为空时写入，避免覆盖用户数据。
 */
const TYPE_COLOR = {
  operator: '#5B8FF9',
  ai_task: '#61DDAA',
  condition: '#F6BD16',
  data: '#7262FD',
  monitor: '#ff7875',
  system: '#73D13D',
  market: '#FF9D4D',
  plugin: '#FF99C3',
  resource: '#42b983',
  workflow: '#9270CA',
  fusion: '#ffc53d',
  league: '#13c2c2'
}
function node(id, label, type, size = 8) {
  return { id, label, type, node_type: type, color: TYPE_COLOR[type] || '#999', size: size + Math.floor(Math.random() * 4) }
}

const GRAPH_NODES = [
  node('d04', '璇玑融合引擎 D04', 'fusion', 16),
  node('ea', '算法联盟 EA', 'league', 14),
  node('biz', '业务联盟', 'league', 14),
  node('col', '协作治理域', 'system', 12),
  node('n_ingest', '需求采集', 'operator', 8),
  node('n_norm', '归一化 IR', 'operator', 9),
  node('n_disp', '双联盟十四维特派', 'ai_task', 10),
  node('n_rec', '归一化裁决', 'ai_task', 9),
  node('n_gate', '⛨璇玑验证网关', 'monitor', 12),
  node('n_gov', '治理闸门', 'condition', 10),
  node('n_opt', '优化出图', 'operator', 9),
  node('n_pub', '融合发布', 'market', 10),
  node('n_graph', '知识图谱算子', 'operator', 9),
  node('n_pagerank', 'PageRank', 'operator', 7),
  node('n_comm', '社区发现', 'operator', 7),
  node('n_act', '激活传播', 'operator', 7),
  node('n_mcp', 'MCP 兼容层', 'plugin', 9),
  node('n_auto', '自动化中枢', 'workflow', 10),
  node('n_caomei', 'Caomei 编译器', 'ai_task', 8),
  node('n_market', '算子商城', 'market', 11),
  node('n_resource', '资源管理', 'resource', 8),
  node('n_chat', 'AI 对话', 'ai_task', 8),
  node('n_data', '数据资产库', 'data', 9)
]

const GRAPH_EDGES = [
  ['d04', 'ea', 1],
  ['d04', 'biz', 1],
  ['ea', 'n_disp', 1],
  ['biz', 'n_disp', 1],
  ['col', 'd04', 1],
  ['n_ingest', 'n_norm', 1],
  ['n_norm', 'n_disp', 1],
  ['n_disp', 'n_rec', 1],
  ['n_rec', 'n_gate', 1],
  ['n_gate', 'n_gov', 1],
  ['n_gov', 'n_opt', 1],
  ['n_opt', 'n_pub', 1],
  ['n_pub', 'n_market', 1],
  ['n_graph', 'n_pagerank', 1],
  ['n_graph', 'n_comm', 1],
  ['n_graph', 'n_act', 1],
  ['n_market', 'n_graph', 1],
  ['d04', 'n_mcp', 1],
  ['n_auto', 'n_disp', 1],
  ['n_auto', 'n_pub', 1],
  ['n_caomei', 'n_norm', 1],
  ['n_market', 'n_resource', 1],
  ['n_chat', 'n_disp', 1],
  ['n_data', 'n_norm', 1],
  ['n_pub', 'n_data', 1],
  ['n_gov', 'n_pub', 1],
  ['n_mcp', 'n_market', 1],
  ['ea', 'n_graph', 1],
  ['biz', 'n_chat', 1]
].map(([source, target, weight]) => ({ source, target, weight }))

const OPERATORS = [
  { id: 'op_pagerank', name: 'PageRank 中心性', type: 'algorithm', category: 'graph', desc: '带阻尼的 PageRank 迭代，识别影响力节点', version: '1.2.0', status: 'active', tags: ['graph', 'rank'] },
  { id: 'op_community', name: '标签传播社区发现', type: 'algorithm', category: 'graph', desc: 'Label Propagation 无监督社区划分', version: '1.0.3', status: 'active', tags: ['graph', 'community'] },
  { id: 'op_activate', name: '激活传播', type: 'algorithm', category: 'graph', desc: '从种子沿边衰减扩散激活能量', version: '1.1.0', status: 'active', tags: ['graph', 'spread'] },
  { id: 'op_normalize', name: '归一化 IR', type: 'pipeline', category: 'fusion', desc: '将异构流程归一化为统一中间表示', version: '2.0.0', status: 'active', tags: ['xuanji', 'ir'] },
  { id: 'op_xuanji', name: '双璇玑十四维治理', type: 'pipeline', category: 'fusion', desc: '业务7+开发7 并行治理 → 闸门 → 出图', version: '3.1.0', status: 'active', tags: ['xuanji', 'govern'] },
  { id: 'op_caomei', name: 'Caomei 需求编译器', type: 'ai', category: 'compiler', desc: '自然语言 → 流程蓝图 → 精化', version: '0.9.1', status: 'beta', tags: ['nlp', 'compiler'] }
]

const MARKET = [
  { id: 'm_xuanji_fusion', name: '全维融合治理算子', description: '双璇玑十四维治理融合产物，可直接上架', requirement: '专家联盟全维分析需求业务处理', tags: ['xuanji', 'fusion'], tenant: 'default', graph: { nodes: GRAPH_NODES.slice(0, 12), edges: GRAPH_EDGES.slice(0, 14) }, governance_score: 82, governance_gate: 'G3-通过', nodes: 12, edges: 14, downloads: 128, author: 'xuanji' },
  { id: 'm_graph_kit', name: '图谱分析工具包', description: 'PageRank / 社区发现 / 激活传播一站式算子', requirement: '知识图谱关系网络分析', tags: ['graph'], tenant: 'default', graph: { nodes: GRAPH_NODES.filter((n) => ['n_graph', 'n_pagerank', 'n_comm', 'n_act'].includes(n.id)), edges: [] }, governance_score: 76, governance_gate: 'G3-通过', nodes: 4, edges: 0, downloads: 342, author: 'ea' }
]

const PLUGINS = [
  { id: 'pl_mcp', name: 'MCP 兼容层', type: 'protocol', desc: '将系统算子以标准 MCP 协议暴露', status: 'active', endpoints: 5 },
  { id: 'pl_browser', name: '浏览器自动化', type: 'automation', desc: '自然语言驱动的网页操作', status: 'active', sessions: 0 },
  { id: 'pl_flow', name: '流程图引擎', type: 'ir', desc: 'FlowGraph IR 校验与执行', status: 'active', node_types: 8 }
]

const WORKFLOWS = [
  { id: 'wf_demo', name: '需求→治理→发布', desc: '端到端闭环演示', steps: ['采集', '归一化', '双联盟治理', '发布'], status: 'ready' },
  { id: 'wf_graph', name: '图谱分析流水线', desc: '图算法编排', steps: ['导入图谱', 'PageRank', '社区发现'], status: 'ready' }
]

const FLOWS = [
  {
    id: 'flow_alliance',
    name: '专家联盟全维分析流程',
    desc: '归一化 → 双联盟十四维 → 裁决 → ⛨璇玑验证 → 治理闸门 → 出图 → 发布',
    nodes: [
      { id: 'n1', name: '采集需求', type: 'operator' },
      { id: 'n2', name: 'AI 合规审查', type: 'ai_task' },
      { id: 'n3', name: '条件分流', type: 'condition' },
      { id: 'n4', name: '归档', type: 'operator' },
      { id: 'n5', name: '运行监控', type: 'monitor' }
    ],
    edges: [
      { from: 'n1', to: 'n2' },
      { from: 'n2', to: 'n3' },
      { from: 'n3', to: 'n4' },
      { from: 'n1', to: 'n5' }
    ],
    valid: true
  }
]

const RESOURCES = [
  { id: 'r_cpu', name: '算力池', type: 'compute', used: 42, total: 100, unit: '%' },
  { id: 'r_mem', name: '内存', type: 'compute', used: 61, total: 128, unit: 'GB' },
  { id: 'r_model', name: '模型仓库', type: 'model', count: 18, status: 'ok' },
  { id: 'r_dataset', name: '数据集', type: 'data', count: 53, status: 'ok' }
]

const CAOMEI_TEMPLATES = [
  { id: 'ct_1', name: '合规审查流程', prompt: '为 {domain} 业务生成合规审查流程图', slots: ['domain'] },
  { id: 'ct_2', name: '数据流水线', prompt: '生成从 {source} 到 {sink} 的数据处理流水线', slots: ['source', 'sink'] },
  { id: 'ct_3', name: '审批流', prompt: '生成包含 {roles} 的多级审批流程', slots: ['roles'] }
]

function seedAll(store) {
  if (store.all('graph_nodes').length === 0) GRAPH_NODES.forEach((n) => store.insert('graph_nodes', n))
  if (store.all('graph_edges').length === 0) GRAPH_EDGES.forEach((e) => store.insert('graph_edges', e))
  if (store.all('operators').length === 0) OPERATORS.forEach((o) => store.insert('operators', o))
  if (store.all('market').length === 0) MARKET.forEach((m) => store.insert('market', m))
  if (store.all('plugins').length === 0) PLUGINS.forEach((p) => store.insert('plugins', p))
  if (store.all('workflows').length === 0) WORKFLOWS.forEach((w) => store.insert('workflows', w))
  if (store.all('flows').length === 0) FLOWS.forEach((f) => store.insert('flows', f))
  if (store.all('resources').length === 0) RESOURCES.forEach((r) => store.insert('resources', r))
  if (store.all('caomei_templates').length === 0) CAOMEI_TEMPLATES.forEach((t) => store.insert('caomei_templates', t))
  if (store.all('llm_config').length === 0)
    store.insert('llm_config', { id: 'llm_default', provider: 'local', base_url: '', model: 'ous-internal', api_key: '', enabled: false, updated_at: new Date().toISOString() })
  if (store.all('automation').length === 0)
    store.insert('automation', {
      id: 'auto_1',
      name: '需求驱动端到端闭环',
      requirement: '专家联盟全维分析需求业务处理',
      status: 'idle',
      permissions: { read: true, write: true, deploy: false },
      flow: FLOWS[0],
      updated_at: new Date().toISOString()
    })
  if (store.all('dialogue_sessions').length === 0)
    store.insert('dialogue_sessions', { id: 'sess_1', title: '示例对话', messages: [{ role: 'user', content: '分析算法联盟全维治理' }], created_at: new Date().toISOString() })
  if (store.all('settings').length === 0) store.insert('settings', { id: 'auto_sync', enabled: false, updated_at: new Date().toISOString() })
  return {
    graph_nodes: store.all('graph_nodes').length,
    graph_edges: store.all('graph_edges').length,
    operators: store.all('operators').length,
    market: store.all('market').length,
    plugins: store.all('plugins').length,
    workflows: store.all('workflows').length,
    flows: store.all('flows').length,
    resources: store.all('resources').length
  }
}

module.exports = { seedAll, GRAPH_NODES, GRAPH_EDGES }
