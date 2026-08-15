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

export default http
