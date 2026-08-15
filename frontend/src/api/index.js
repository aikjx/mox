import axios from 'axios'

// 后端运行时地址：开发期走 Vite 代理 /api -> http://localhost:3000
const http = axios.create({
  baseURL: '/api',
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' }
})

// 统一响应处理：剥离 axios 包裹，直接返回 data
http.interceptors.response.use(
  (resp) => resp.data,
  (err) => {
    const data = err.response && err.response.data
    const msg =
      (data && (data.detail || data.title || data.message)) ||
      err.message ||
      '网络请求失败'
    return Promise.reject(new Error(msg))
  }
)

// 请求拦截：自动注入 API 令牌（兼容后端 OUS_API_TOKEN 鉴权）
// 开发期默认令牌可由 .env 的 VITE_API_TOKEN 覆盖，否则回退本地存储 / 开发令牌
const DEV_TOKEN = 'dev-secret-token'
http.interceptors.request.use((config) => {
  const token =
    (typeof localStorage !== 'undefined' && localStorage.getItem('ous_api_token')) ||
    (typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.VITE_API_TOKEN) ||
    DEV_TOKEN
  config.headers = config.headers || {}
  config.headers['Authorization'] = 'Bearer ' + token
  return config
})

// ===== 系统 =====
export const getHealth = () => http.get('/health')
export const getStatus = () => http.get('/status')
export const getFullStatus = () => http.get('/status/full')
export const getLogs = () => http.get('/logs')
export const getPlugins = () => http.get('/plugins')

// ===== 算子 =====
export const getOperators = () => http.get('/operators')
export const registerOperator = (payload) => http.post('/operators/register', payload)
export const executeWorkflow = (payload) => http.post('/execute', payload)

// ===== 知识图谱 =====
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
export const listDialogueSessions = () => http.get('/dialogue/sessions')
// 导出：对话 + 知识图谱 打包为单文件迁移包（返回 JSON 文本）
export const graphExport = () => http.get('/graph/export')
// 导入：从迁移包恢复对话 + 知识图谱（幂等合并）
export const graphImport = (bundle) => http.post('/graph/import', bundle)

// ===== AI 对话 =====
export const aiChat = (payload) => http.post('/ai/chat', payload)
export const getChatHistory = (session) => http.get(`/ai/chat/history/${encodeURIComponent(session)}`)
export const analyzeAlgorithm = (payload) => http.post('/ai/analyze-algorithm', payload)
export const getAlgorithmTypes = () => http.get('/ai/algorithm-types')
export const analyzeSpiral = (payload) => http.post('/analyze/spiral', payload)

// ===== 资源 =====
export const getResources = () => http.get('/ai/resources')
export const getResourceHealth = () => http.get('/ai/resources/health')

// ===== AI 插件 =====
export const getAiPlugins = () => http.get('/ai/plugins')
export const registerAiPlugin = (payload) => http.post('/ai/plugins/register', payload)
export const sendPluginMessage = (payload) => http.post('/ai/plugins/send-message', payload)

// ===== 工作流 =====
export const getWorkflowTemplates = () => http.get('/ai/workflows/templates')
export const getWorkflows = () => http.get('/ai/workflows')
export const saveWorkflow = (payload) => http.post('/ai/workflows/save', payload)
export const executeWorkflowDef = (payload) => http.post('/ai/workflows/execute', payload)
export const getWorkflowInstances = () => http.get('/ai/workflows/instances')

// ===== 流程图 (FlowGraph IR) =====
export const getFlows = () => http.get('/ai/flows')
export const createFlow = (payload) => http.post('/ai/flows', payload)
export const getFlow = (id) => http.get(`/ai/flows/${encodeURIComponent(id)}`)
export const deleteFlow = (id) => http.delete(`/ai/flows/${encodeURIComponent(id)}`)
export const validateFlow = (payload) => http.post('/ai/flows/validate', payload)
export const executeFlow = (payload) => http.post('/ai/flows/execute', payload)
export const getFlowNodeTypes = () => http.get('/ai/flows/node-types')

// ===== LLM 配置 =====
export const getLlmConfig = () => http.get('/ai/llm/config')
export const updateLlmConfig = (payload) => http.post('/ai/llm/config', payload)
export const testLlm = () => http.post('/ai/llm/test')

// ===== 浏览器自动化 =====
export const getBrowserTemplates = () => http.get('/ai/browser/templates')
export const getBrowserSessions = () => http.get('/ai/browser/sessions')
export const getBrowserSession = (id) => http.get(`/ai/browser/sessions/${encodeURIComponent(id)}`)
export const closeBrowserSession = (id) =>
  http.delete(`/ai/browser/sessions/${encodeURIComponent(id)}`)
export const executeBrowserTask = (payload) => http.post('/ai/browser/execute-task', payload)
export const executeBrowserSteps = (payload) => http.post('/ai/browser/execute-steps', payload)
export const executeBrowserAction = (payload) => http.post('/ai/browser/execute-action', payload)
export const browserNatural = (payload) => http.post('/ai/browser/natural', payload)

// ===== 算子商城 (Operator Market) =====
export const marketList = (params) => http.get('/market', { params })
export const marketRandom = () => http.get('/market/random')
export const marketGet = (id) => http.get(`/market/${encodeURIComponent(id)}`)
export const marketUpload = (payload) => http.post('/market/upload', payload)
export const marketUpdate = (id, payload) => http.post(`/market/${encodeURIComponent(id)}`, payload)
export const marketDelete = (id) => http.delete(`/market/${encodeURIComponent(id)}`)
export const marketClone = (id) => http.post(`/market/${encodeURIComponent(id)}/clone`)

// ===== MCP 兼容层 (Model Context Protocol) =====
// 把系统内算子与插件以标准 MCP 协议暴露，兼容开源 MCP 客户端
export const mcpListTools = () =>
  http.post('/mcp', { jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} })
export const mcpCall = (name, args) =>
  http.post('/mcp', { jsonrpc: '2.0', id: 2, method: 'tools/call', params: { name, arguments: args } })

// ===== AI 自动化中枢 (需求驱动端到端闭环) =====
// 对话生成业务处理流程图 + 全栈代码 + 自动测试 + RBAC，沙箱实跑异常自动修复回写
export const automationList = () => http.get('/automation')
export const automationChat = (payload) => http.post('/automation/chat', payload)
export const automationRefine = (id, payload) => http.post(`/automation/${encodeURIComponent(id)}/refine`, payload)
export const automationRun = (id, payload) => http.post(`/automation/${encodeURIComponent(id)}/run`, payload)
export const automationPermissions = (id) => http.get(`/automation/${encodeURIComponent(id)}/permissions`)
export const automationUpdate = (id, payload) => http.put(`/automation/${encodeURIComponent(id)}`, payload)

export default http
