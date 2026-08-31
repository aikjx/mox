// 知识图谱 API
import http from './http'

export const getGraph = () => http.get('/graph')
export const getGraphStats = () => http.get('/graph/stats')
export const getCentrality = () => http.get('/graph/centrality')
export const getCommunities = () => http.get('/graph/communities')
export const getPagerank = () => http.get('/graph/pagerank')
export const getNeighbors = (id) => http.get(`/graph/neighbors/${encodeURIComponent(id)}`)
export const getShortestPath = (source, target) =>
  http.get('/graph/path', { params: { source, target } })
export const recommendNodes = (payload) => http.post('/graph/recommend', payload)
export const addGraphNode = (payload) => http.post('/graph/node', payload)
export const addGraphEdge = (payload) => http.post('/graph/edge', payload)
// 激活传播：从种子节点沿边扩散激活能量，返回各节点激活值
export const propagateActivation = (seedNodes, iterations = 10) =>
  http.post('/graph/activate', { seed: seedNodes, iterations })

// ===== 对话自动→知识图谱 自动整理 =====
// 统一搜索：对话内容 + 知识图谱节点
export const graphSearch = (q, limit = 20) =>
  http.get('/graph/search', { params: { q, limit } })
// 切换全自动同步开关
export const toggleAutoSync = (enabled) =>
  http.post('/graph/auto-sync/toggle', { enabled })
// 查询全自动同步状态
export const getAutoSyncStatus = () => http.get('/graph/auto-sync/status')
// 列出对话会话
export const getDialogueSessions = () => http.get('/dialogue/sessions')
/** @deprecated 请使用 getDialogueSessions */
export const listDialogueSessions = getDialogueSessions
// 导出：对话 + 知识图谱 打包为单文件迁移包（返回 JSON 文本）
export const graphExport = () => http.get('/graph/export')
// 导入：从迁移包恢复对话 + 知识图谱（幂等合并）
export const graphImport = (bundle) => http.post('/graph/import', bundle)

// AI 图谱增强
export const aiGraphInsights = (payload) => http.post('/graph/ai-insights', payload)
