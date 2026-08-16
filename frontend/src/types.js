// 算子统一系统 - 全局类型与常量定义

export const APP_NAME = '算子统一系统'
export const APP_VERSION = '3.0.0'

// 算子类型（用于分类筛选与配色）
export const OPERATOR_CATEGORIES = [
  { key: 'all', label: '全部' },
  { key: 'core', label: '核心算子' },
  { key: 'math', label: '数学算子' },
  { key: 'ai', label: 'AI 算子' },
  { key: 'graph', label: '图算子' },
  { key: 'signal', label: '信号算子' },
  { key: 'data', label: '数据算子' },
  { key: 'custom', label: '自定义算子' }
]

// 调用后端可用的标准算子链（用于执行工作流示例）
export const BUILTIN_OPERATORS = [
  { id: 'identity', name: '恒等', desc: '输出与输入完全一致' },
  { id: 'linear', name: '线性变换', desc: '对角缩放矩阵（参数 scale）' },
  { id: 'normalize', name: 'L2 归一化', desc: '向量归一化为单位范数' },
  { id: 'normalize_l1', name: 'L1 归一化', desc: '归一化为概率分布' },
  { id: 'relu', name: 'ReLU', desc: '修正线性单元，负截断为 0' },
  { id: 'sigmoid', name: 'Sigmoid', desc: '逻辑斯蒂压缩至 (0,1)' },
  { id: 'tanh', name: 'Tanh', desc: '双曲正切压缩至 (-1,1)' },
  { id: 'softmax', name: 'Softmax', desc: '指数归一化为概率分布' },
  { id: 'scale', name: '标量缩放', desc: '逐元素乘以 factor' }
]

export const NODE_TYPE_COLORS = {
  core: '#6366f1',
  activation: '#f59e0b',
  math: '#10b981',
  signal: '#ef4444',
  data: '#8b5cf6',
  ai: '#ec4899',
  graph: '#06b6d4',
  optimizer: '#84cc16',
  loss: '#f97316',
  regularization: '#a855f7',
  normalization: '#14b8a6',
  custom: '#64748b'
}

export const AI_CAPABILITIES = [
  { key: 'ai_chat', label: 'AI 智能对话' },
  { key: 'intent_recognition', label: '意图识别' },
  { key: 'operator_recommendation', label: '算子推荐' },
  { key: 'algorithm_analysis', label: '算法分析归一化' },
  { key: 'flow_normalization', label: '流程图标准化' },
  { key: 'complexity_analysis', label: '复杂度分析' },
  { key: 'resource_management', label: '全资源管理' },
  { key: 'plugin_bus', label: '插件互通总线' },
  { key: 'workflow_automation', label: '业务流程自动化' },
  { key: 'parallel_execution', label: '并行执行' },
  { key: 'bpmn_engine', label: 'BPMN 引擎' }
]

// 顶部菜单模块（企业级中后台导航）
export const NAV_MODULES = [
  { key: 'dashboard', label: '工作台', icon: 'Odometer', path: '/dashboard', color: '#4f46e5', bg: '#eef2ff' },
  { key: 'operators', label: '算子中心', icon: 'Cpu', path: '/operators', color: '#6366f1', bg: '#eef2ff' },
  { key: 'graph', label: '知识图谱', icon: 'Share', path: '/graph', color: '#06b6d4', bg: '#ecfeff' },
  { key: 'ai', label: 'AI 助手', icon: 'ChatDotRound', path: '/ai', color: '#ec4899', bg: '#fce7f3' },
  { key: 'resources', label: '资源管理', icon: 'Coin', path: '/resources', color: '#10b981', bg: '#ecfdf5' },
  { key: 'workflow', label: '工作流编排', icon: 'Operation', path: '/workflow', color: '#f59e0b', bg: '#fffbeb' },
  { key: 'plugins', label: 'AI 插件', icon: 'Connection', path: '/plugins', color: '#8b5cf6', bg: '#f3e8ff' },
  { key: 'browser', label: '浏览器自动化', icon: 'Monitor', path: '/browser', color: '#0ea5e9', bg: '#e0f2fe' },
  { key: 'monitor', label: '系统监控', icon: 'DataLine', path: '/monitor', color: '#ef4444', bg: '#fef2f2' },
  { key: 'docs', label: 'API 文档', icon: 'Document', path: '/docs', color: '#14b8a6', bg: '#ccfbf1' },
  { key: 'market', label: '算子商城', icon: 'Shop', path: '/market', color: '#f43f5e', bg: '#ffe4e6' },
  { key: 'mcp', label: 'MCP 兼容', icon: 'Link', path: '/mcp', color: '#8b5cf6', bg: '#f3e8ff' },
  { key: 'automation', label: 'AI 自动化', icon: 'MagicStick', path: '/automation', color: '#f97316', bg: '#ffedd5' },
  { key: 'caomei', label: '需求编译', icon: 'Tickets', path: '/caomei', color: '#16a34a', bg: '#dcfce7' },
  { key: 'algolab', label: '算法实验室', icon: 'TrendCharts', path: '/algolab', color: '#d97706', bg: '#fef3c7' },
  { key: 'alliance-fusion', label: '全维融合', icon: 'Aim', path: '/alliance-fusion', color: '#7c3aed', bg: '#ede9fe' }
]
