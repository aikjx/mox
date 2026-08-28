// 璇玑信息知识图谱关联关系系统 - 全局类型与常量定义

export const APP_NAME = '璇玑信息知识图谱关联关系系统'
export const APP_SHORT = '璇玑系统'
export const APP_VERSION = '3.0.0'

// 算子是系统的一部分，用于图谱分析和业务处理
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
  { key: 'graph_analysis', label: '知识图谱关联分析' },
  { key: 'graph_reasoning', label: '图谱推理与发现' },
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

// 顶部菜单模块（以知识图谱为核心，算子为辅助工具）
export const NAV_MODULES = [
  { key: 'dashboard', label: '璇玑门户', icon: 'Odometer', path: '/dashboard', color: '#4f46e5', bg: '#eef2ff' },
  { key: 'projects', label: '项目中心', icon: 'Folder', path: '/projects', color: '#0d9488', bg: '#ccfbf1' },
  { key: 'graph', label: '璇玑图谱', icon: 'Share', path: '/graph', color: '#06b6d4', bg: '#ecfeff' },
  { key: 'operators', label: '算子引擎', icon: 'Cpu', path: '/operators', color: '#6366f1', bg: '#eef2ff' },
  { key: 'expert-center', label: '专家联盟', icon: 'User', path: '/expert-center', color: '#7c3aed', bg: '#ede9fe' },
  { key: 'ai', label: 'AI 助手', icon: 'ChatDotRound', path: '/ai', color: '#ec4899', bg: '#fce7f3' },
  { key: 'tasks', label: '任务管理', icon: 'List', path: '/tasks', color: '#0ea5e9', bg: '#e0f2fe' },
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
  { key: 'infinite-optimizer', label: '无穷维度优化', icon: 'Compass', path: '/infinite-optimizer', color: '#0e7490', bg: '#ecfeff' },
  { key: 'mox-fusion', label: '全维融合', icon: 'Aim', path: '/mox-fusion', color: '#7c3aed', bg: '#ede9fe' },
  { key: 'knowledge-base', label: '云盘知识库', icon: 'Collection', path: '/knowledge-base', color: '#0d9488', bg: '#ccfbf1' },
  { key: 'llm-config', label: '大模型配置', icon: 'Setting', path: '/llm-config', color: '#6366f1', bg: '#eef2ff' },
  { key: 'expert-orchestrator', label: 'V2编排引擎', icon: 'Promotion', path: '/expert-orchestrator', color: '#ec4899', bg: '#fce7f3' },
  { key: 'admin', label: '系统管理', icon: 'Lock', path: '/admin', color: '#475569', bg: '#f1f5f9' }
]

// 璇玑专家联盟 5 阶段生命周期导航（以项目为根 · 按项目开发流程分组）
// ┌─ 📌 项目总览 ────────────────────────────────────────────────┐
// │ dashboard · projects · tasks · resources                     │
// ├─ 📋 需求阶段（S1）───────────────────────────────────────────┤
// │ ai(项目对话) · caomei(需求编译) · knowledge-base(知识库)    │
// │ llm-config(大模型配置)                                        │
// ├─ 🏗️ 架构阶段（S2）───────────────────────────────────────────┤
// │ graph(知识图谱) · expert-center(专家联盟) · mox-fusion(融合)│
// │ expert-enterprise(企业专家) · expert-orchestrator(V2编排)   │
// ├─ ⚙️ 开发阶段（S3-S4）────────────────────────────────────────┤
// │ operators(算子) · workflow(工作流) · plugins(插件) · mcp    │
// │ automation(自动化) · browser(浏览器自动化) · algolab(算法)  │
// │ infinite-optimizer(无穷优化) · botCenter(机器人)             │
// ├─ 🚀 发布阶段（S5）───────────────────────────────────────────┤
// │ monitor(监控) · docs(API文档) · admin(系统管理)              │
// └───────────────────────────────────────────────────────────────┘
export const NAV_GROUPS = [
  { key: 'overview',   label: '📌 项目总览',   order: 0, items: ['dashboard', 'projects', 'tasks', 'resources'] },
  { key: 's1-require', label: '📋 需求阶段',   order: 1, items: ['ai', 'caomei', 'knowledge-base', 'llm-config'] },
  { key: 's2-arch',    label: '🏗️ 架构阶段',   order: 2, items: ['graph', 'expert-center', 'mox-fusion', 'expert-enterprise', 'expert-orchestrator'] },
  { key: 's3-dev',     label: '⚙️ 开发阶段',   order: 3, items: ['operators', 'workflow', 'plugins', 'mcp', 'automation', 'browser', 'algolab', 'infinite-optimizer', 'botCenter'] },
  { key: 's4-release', label: '🚀 发布阶段',   order: 4, items: ['monitor', 'docs', 'admin'] },
  { key: 'market',     label: '🛒 资源商城',   order: 5, items: ['market'] }
]

// 5 阶段流程（与 PhasePipeline 对齐 · 按项目开发流程）
export const PROJECT_PHASES = [
  { key: 'requirement', label: '需求阶段', desc: '项目对话 · 需求编译 · 知识库', color: '#6366f1', group: 's1-require' },
  { key: 'architecture', label: '架构阶段', desc: '知识图谱 · 专家联盟 · 全维融合', color: '#06b6d4', group: 's2-arch' },
  { key: 'develop', label: '开发阶段', desc: '算子 · 工作流 · 插件 · 自动化', color: '#10b981', group: 's3-dev' },
  { key: 'release', label: '发布阶段', desc: '监控 · 文档 · 系统管理', color: '#f59e0b', group: 's4-release' }
]

// 顶栏⚡新建命令（6 项，按 5 阶段顺序排）
export const QUICK_CREATE_COMMANDS = [
  { key: 'project',    label: '新建项目',     icon: 'Folder',      tip: 'S0 启动跟进',         action: 'event', event: 'mox:open-create-project' },
  { key: 'task',       label: '新建任务',     icon: 'List',        tip: 'Ctrl + Shift + N',    action: 'event', event: 'mox:open-create-task' },
  { key: 'ai-session', label: '新建 AI 对话', icon: 'ChatDotRound',tip: 'AI助手X · φ模式',    action: 'route', route: '/ai', query: { fresh: '1' } },
  { key: 'expert',     label: '注册专家',     icon: 'User',        tip: '专家联盟招募',        action: 'event', event: 'mox:open-register-expert' },
  { key: 'workflow',   label: '新建工作流',   icon: 'Operation',   tip: 'S3 方案设计',         action: 'route', route: '/workflow', query: { action: 'create' } },
  { key: 'market',     label: '上传算子包',   icon: 'Shop',        tip: 'S4 注册算子',         action: 'route', route: '/market', query: { action: 'upload' } }
]

// 产品体验增强：快捷键分组（Shift + ? 弹 Drawer 展示给用户）
export const HOTKEY_GROUPS = [
  {
    group: '全局',
    items: [
      { keys: ['Ctrl', 'K'], desc: '聚焦全局搜索（命令面板）' },
      { keys: ['Ctrl', '⇧', 'P'], desc: '同上（命令面板，兼容 VS Code 用户）' },
      { keys: ['Ctrl', '⇧', 'N'], desc: '弹出新建任务 Dialog（任何页面）' },
      { keys: ['Shift', '?'], desc: '打开 / 关闭本快捷键帮助' },
      { keys: ['Alt', '1..9'], desc: '按导航分组 1-9 顺序跳转到对应模块' }
    ]
  },
  {
    group: '表单与列表',
    items: [
      { keys: ['Esc'], desc: '关闭当前 Dialog / Drawer / 取消搜索' },
      { keys: ['Enter'], desc: '提交聚焦中的表单 / 搜索（已在 10+ 页面启用）' },
      { keys: ['Ctrl', 'Enter'], desc: 'AI 场景下提交长文本（Chat/Automation 中）' },
      { keys: ['⌫ / Backspace'], desc: '在列表内清空筛选（需列表聚焦）' }
    ]
  }
]

// 专家类型映射
export const EXPERT_TYPES = {
  algorithm: '算法专家',
  architecture: '架构专家',
  data: '数据专家',
  ai: 'AI专家',
  workflow: '工作流专家',
  operator: '算子系统专家',
  graph: '知识图谱专家',
  security: '安全专家',
  performance: '性能优化专家',
  monitor: '可观测性专家',
  market: '商业智能专家',
  mcp: 'MCP协议专家',
  automation: '自动化专家',
  requirement: '需求工程专家',
  fusion: '融合专家'
}

// AI 对话专家预设
export const AI_EXPERT_PRESETS = [
  { key: 'general', label: '通用助手', type: null, icon: 'ChatDotRound' },
  { key: 'algorithm', label: '算法专家', type: 'algorithm', icon: 'TrendCharts' },
  { key: 'architecture', label: '架构专家', type: 'architecture', icon: 'Grid' },
  { key: 'operator', label: '算子专家', type: 'operator', icon: 'Cpu' },
  { key: 'graph', label: '图谱专家', type: 'graph', icon: 'Share' },
  { key: 'workflow', label: '工作流专家', type: 'workflow', icon: 'Operation' },
  { key: 'automation', label: '自动化专家', type: 'automation', icon: 'MagicStick' },
  { key: 'fusion', label: '融合专家', type: 'fusion', icon: 'Aim' }
]
