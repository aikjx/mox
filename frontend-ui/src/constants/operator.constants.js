// 算子相关常量

// 算子分类
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

// 内置标准算子
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

// 节点类型配色
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
