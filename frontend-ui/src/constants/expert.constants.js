// 专家联盟相关常量

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
