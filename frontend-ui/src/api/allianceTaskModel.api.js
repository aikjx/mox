// The gateway wraps native task DTOs in a timing envelope. Keep that transport
// shape and the scheduler's 0..1 progress out of view components.
export function unwrapTaskPayload(value) {
  for (let depth = 0; depth < 4 && value && !Array.isArray(value); depth++) {
    if (!Object.hasOwn(value, 'data')) break
    value = value.data
  }
  return value
}

export function normalizeTask(value) {
  const task = unwrapTaskPayload(value)
  if (!task || !(task.task_id || task.id)) throw new Error('任务服务未返回有效的任务编号')
  const progress = Number(task.progress ?? 0)
  return {
    ...task,
    id: task.task_id || task.id,
    name: task.title ?? task.name ?? '',
    // Native DTO: fraction; historical display DTO: percentage.
    progress: Number.isFinite(progress) ? Math.round(Math.max(0, Math.min(100, task.task_id ? progress * 100 : progress))) : 0,
    status: task.status || 'unknown',
  }
}

export function normalizeTaskList(value) {
  const data = unwrapTaskPayload(value)
  const items = Array.isArray(data) ? data : data?.tasks ?? data?.items
  if (!Array.isArray(items)) throw new Error('任务服务返回的列表格式不正确')
  return items.map(normalizeTask)
}

export function normalizeTaskLogs(value) {
  const data = unwrapTaskPayload(value)
  const items = Array.isArray(data) ? data : data?.logs ?? data?.items
  if (!Array.isArray(items)) throw new Error('任务日志格式不正确')
  return items.map(log => ({ ...log, level: String(log.level || 'info').toLowerCase(), time: log.ts || log.time || '' }))
}

export function normalizeTaskDag(value) {
  const data = unwrapTaskPayload(value)
  if (!Array.isArray(data?.nodes)) throw new Error('任务流程数据格式不正确')
  const nodes = data.nodes.map((node, index) => ({
    ...node, id: node.id || node.node_id, name: node.name || node.label || node.node_id,
    x: Number(node.x ?? node.position?.x ?? 100 + index * 160),
    y: Number(node.y ?? node.position?.y ?? 70),
  }))
  const byId = new Map(nodes.map(node => [node.id, node]))
  const edges = (data.edges || []).flatMap(edge => {
    const source = byId.get(edge.source), target = byId.get(edge.target)
    if (!source || !target) return []
    return [{ ...edge, x1: source.x, y1: source.y, x2: target.x, y2: target.y, status: target.status }]
  })
  return { nodes, edges }
}

export function normalizeTaskFusion(value) {
  const data = unwrapTaskPayload(value)
  if (!data || data.fusion_status === 'pending' || data.status === 'pending') return null
  const result = data.fusion_result ?? data.result ?? data
  if (!result || typeof result !== 'object') throw new Error('任务结果格式不正确')
  const outputs = result.content?.outputs || []
  const expertResults = outputs.map(item => ({
    expert: item.expert,
    answer: (item.output?.steps || []).filter(step => step.startsWith('[结论] ')).map(step => step.slice(5)).join('\n\n'),
  })).filter(item => item.answer)
  const emptyReport = outputs.some(item => (item.output?.steps || []).some(step => step.includes('空报告')))
  return { ...result, expertResults, emptyReport, raw: data }
}
