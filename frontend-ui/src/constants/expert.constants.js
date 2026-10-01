// 专家联盟相关常量

// 专家类型映射
// 键集合与 EXPERT_COLORS / EXPERT_GRADIENTS / EXPERT_EMOJIS 严格一致（16 键）。
// custom 是其中唯一"注册时选不到、但存量数据里会出现"的桶：ExpertCenterView 的类型筛选
// 下拉必须能筛出自定义专家，缺了它 custom 就只在三张表里有名字、在标签表里没有名字。
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
  fusion: '融合专家',
  custom: '自定义专家'
}

// 紧凑标签：图例 / 胶囊这类宽度受限的面用短名，其余面用上面的全名。
// 原先只有 ExpertEnterprisePanel 自带这份短名表（视图里第 5 份"类型→视觉属性"表），
// 键数还比全名少一个 custom。搬进单源后它与其他表同键集。
export const EXPERT_TYPE_SHORT_LABELS = {
  algorithm: '算法', architecture: '架构', data: '数据', ai: 'AI',
  workflow: '工作流', operator: '算子', graph: '图谱', security: '安全',
  performance: '性能', monitor: '监控', market: '商业', mcp: 'MCP',
  automation: '自动化', requirement: '需求', fusion: '融合',
  custom: '自定义'
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

// 专家类型的色/图标/渐变单源。此前 expertColor 有 7 份逐字副本、expertEmoji 同样 7 份、
// expertGradient 2 份（按 key=value 规范化后 SHA1 相同才敢合并）。
// 色值沿用副本原字面量 —— 本步只把"七份"收成"一份"，不改任何像素。
// 待办（会改像素，需逐页截图复核）：把这些字面量换成随皮肤走的 --cat-N 档位。
export const EXPERT_COLOR_FALLBACK = '#6366f1'
export const EXPERT_GRADIENT_FALLBACK = 'linear-gradient(135deg, #7c3aed, #06b6d4)'
export const EXPERT_EMOJI_FALLBACK = '👤'

export const EXPERT_COLORS = {
  algorithm: '#6366f1', architecture: '#6366f1', data: '#10b981',
  ai: '#ec4899', workflow: '#f59e0b', graph: '#06b6d4',
  security: '#ef4444', performance: '#f97316', monitor: '#14b8a6',
  market: '#8b5cf6', mcp: '#0ea5e9', automation: '#84cc16',
  requirement: '#f43f5e', fusion: '#a855f7', operator: '#64748b',
  custom: '#64748b'
}

export const EXPERT_GRADIENTS = {
  algorithm: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
  architecture: 'linear-gradient(135deg, #6366f1, #06b6d4)',
  data: 'linear-gradient(135deg, #10b981, #14b8a6)',
  ai: 'linear-gradient(135deg, #ec4899, #8b5cf6)',
  workflow: 'linear-gradient(135deg, #f59e0b, #ef4444)',
  graph: 'linear-gradient(135deg, #06b6d4, #3b82f6)',
  security: 'linear-gradient(135deg, #ef4444, #f97316)',
  performance: 'linear-gradient(135deg, #f97316, #f59e0b)',
  monitor: 'linear-gradient(135deg, #14b8a6, #10b981)',
  market: 'linear-gradient(135deg, #8b5cf6, #ec4899)',
  mcp: 'linear-gradient(135deg, #0ea5e9, #06b6d4)',
  automation: 'linear-gradient(135deg, #84cc16, #10b981)',
  requirement: 'linear-gradient(135deg, #f43f5e, #ec4899)',
  fusion: 'linear-gradient(135deg, #a855f7, #7c3aed)',
  operator: 'linear-gradient(135deg, #64748b, #475569)',
  custom: 'linear-gradient(135deg, #64748b, #475569)'
}

export const EXPERT_EMOJIS = {
  algorithm: '🧮', architecture: '🏗️', data: '🔗',
  ai: '🤖', workflow: '⚡', graph: '🕸️',
  security: '🔒', performance: '🚀', monitor: '📊',
  market: '📈', mcp: '🔌', automation: '🤖',
  requirement: '📋', fusion: '🔀', operator: '⚙️',
  custom: '👤'
}

// 类型 → Element Plus 图标名。原只有 ExpertCenterView 一份（且该函数零调用，属死账），
// 收进来是为了让"视图里不许再有按类型编号的表"这条守卫不需要开例外。
export const EXPERT_ICONS = {
  algorithm: 'TrendCharts', architecture: 'Grid', data: 'Coin',
  ai: 'MagicStick', workflow: 'Operation', operator: 'Cpu',
  graph: 'Share', security: 'Lock', performance: 'Lightning',
  monitor: 'DataLine', market: 'Shop', mcp: 'Link',
  automation: 'MagicStick', requirement: 'Tickets', fusion: 'Aim',
  custom: 'User'
}

export function expertColor(type) {
  return EXPERT_COLORS[type] || EXPERT_COLOR_FALLBACK
}

export function expertGradient(type) {
  return EXPERT_GRADIENTS[type] || EXPERT_GRADIENT_FALLBACK
}

export function expertEmoji(type) {
  return EXPERT_EMOJIS[type] || EXPERT_EMOJI_FALLBACK
}
