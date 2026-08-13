// 后端数据契约（与 crates/runtime + crates/ai-agent/types.rs 对齐）

export const Role = {
  User: 'user',
  Assistant: 'assistant',
  System: 'system',
  Tool: 'tool',
}

/**
 * ChatMessage
 * { id, role, content, timestamp, metadata, referenced_operators }
 */
export function emptyMessage() {
  return {
    id: '',
    role: Role.User,
    content: '',
    timestamp: new Date().toISOString(),
    metadata: {},
    referenced_operators: [],
  }
}

/**
 * ChatResponse
 * { message, suggestions[], recommended_operators[], actions[{id,label,action_type,payload}], workflow_suggestion?[] }
 */
export const ActionType = {
  ExecuteWorkflow: 'execute_workflow',
  ViewOperator: 'view_operator',
  CreateWorkflow: 'create_workflow',
  AnalyzeAlgorithm: 'analyze_algorithm',
  ShowResources: 'show_resources',
  ShowGraph: 'show_graph',
}

/**
 * GraphData
 * { nodes: NodeData[], edges: EdgeData[], stats }
 * NodeData: { id, label, node_type, pagerank, degree_centrality, activation, size, color }
 * EdgeData: { source, target, weight, relation_type }
 */
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
  custom: '#64748b',
}
