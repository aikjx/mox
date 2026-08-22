import axios from 'axios'
import { ElMessage } from 'element-plus'

// 后端运行时地址：开发环境走 Vite 代理，生产环境直连
const http = axios.create({
  baseURL: '/api',
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' }
})

// 统一响应处理：剥离 axios 包裹，自动解包 {success, data} 格式
http.interceptors.response.use(
  (resp) => {
    const body = resp.data
    if (body && typeof body === 'object' && 'success' in body && 'data' in body) {
      if (!body.success) {
        return Promise.reject(new Error(body.error || body.message || '请求失败'))
      }
      return body.data
    }
    return body
  },
  (err) => {
    const status = err.response && err.response.status
    const data = err.response && err.response.data
    let msg =
      (data && (data.detail || data.title || data.message)) ||
      err.message ||
      '网络请求失败'
    // 鉴权/服务类错误的全局友好提示（业务校验错误仍由各视图自行处理）
    if (status === 401 || status === 503) {
      msg = status === 401
        ? '鉴权失败：请检查 API 令牌（OUS_API_TOKEN）是否已配置'
        : '服务暂不可用：后端未启动或正在重启，请稍后重试'
      ElMessage.error(msg)
    } else if (err.code === 'ECONNABORTED') {
      msg = '请求超时：后端处理较慢，请稍后重试'
      ElMessage.warning(msg)
    } else if (!err.response) {
      msg = '无法连接后端：请确认服务已启动（http://localhost:3002）'
      ElMessage.error(msg)
    }
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

// ===== 联网搜索 =====
export const getWebSearchConfig = () => http.get('/web-search/config')
export const updateWebSearchConfig = (payload) => http.post('/web-search/config', payload)
export const testWebSearch = () => http.post('/web-search/test', {})
export const webSearch = (query) => http.post('/web-search', { query })

// ===== 资源 =====
export const getResources = () => http.get('/ai/resources')
export const getResourceHealth = () => http.get('/ai/resources/health')

// ===== AI 插件 =====
export const getAiPlugins = () => http.get('/ai/plugins')
export const registerAiPlugin = (payload) => http.post('/ai/plugins/register', payload)
export const sendPluginMessage = (payload) => http.post('/ai/plugins/send-message', payload)
// 插件拓扑：插件消息总线拓扑（节点 + 订阅/投递关系）
export const getPluginTopology = () => http.get('/ai/plugins/topology')

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
// 市场导出：将算子包导出为可移植 DSL 工程（FlowDefinition JSON）
export const marketExport = (id) => http.get(`/market/${encodeURIComponent(id)}/export`)
// Caomei 需求编译器：自然语言 → 蓝图 / 蓝图精化 / 模板库
export const caomeiCompile = (payload) => http.post('/caomei/compile', payload)
export const caomeiRefine = (payload) => http.post('/caomei/refine', payload)
export const caomeiTemplates = (params) => http.get('/caomei/templates', { params })

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

// ===== 璇玑全维治理 (双璇玑十四维) =====
// 维度清单与璇玑健康度
export const xuanjiHealth = () => http.get('/xuanji/health')
// 传入流程蓝图（FlowGraph）做全维治理，返回 GovernanceReport（专家评分/闸门/璇玑/采纳建议）
// tenant: "default"=普通商业租户 / "gov"=强合规租户（政务/金融，驱动 I-06 治理 8 闸门分层）
export const xuanjiOptimize = (flow, tenant = 'default') =>
  http.post('/xuanji/optimize', { flow, tenant })
// 全维融合发布：归一化 -> 取优化图 -> 一键落盘上传算子市场（插件/应用平台）
export const xuanjiPublish = (payload) => http.post('/xuanji/publish', payload)

// ===== LLM 网关 =====
export const getLlmProviders = () => http.get('/llm/providers')
export const getLlmProviderPresets = () => http.get('/llm/providers/presets')
export const getLlmPresets = () => http.get('/llm/providers/presets')
export const getLlmProvider = (id) => http.get(`/llm/providers/${encodeURIComponent(id)}`)
export const setActiveProvider = (providerId) => http.post('/llm/providers/active', { provider_id: providerId })
export const addLlmProvider = (payload) => http.post('/llm/providers', payload)
export const updateLlmProvider = (id, payload) => http.put(`/llm/providers/${encodeURIComponent(id)}`, payload)
export const removeLlmProvider = (id) => http.delete(`/llm/providers/${encodeURIComponent(id)}`)
export const enableLlmProvider = (id) => http.post(`/llm/providers/${encodeURIComponent(id)}/enable`)
export const disableLlmProvider = (id) => http.post(`/llm/providers/${encodeURIComponent(id)}/disable`)
export const testLlmProvider = (id) => http.post(`/llm/providers/${encodeURIComponent(id)}/test`)
export const discoverLlmModels = (id) => http.post(`/llm/providers/${encodeURIComponent(id)}/discover`)
export const getLlmHealth = () => http.get('/llm/health')
export const getLlmRouting = () => http.get('/llm/routing')
export const updateLlmRouting = (payload) => http.put('/llm/routing', payload)
export const getLlmUsage = () => http.get('/llm/usage')
export const getLlmLogs = (limit = 50) => http.get('/llm/logs', { params: { limit } })
export const getLlmStats = () => http.get('/llm/stats')

// ===== 专家联盟 =====
export const getExperts = (params) => http.get('/experts', { params })
export const getExpert = (id) => http.get(`/experts/${encodeURIComponent(id)}`)
export const registerExpert = (payload) => http.post('/experts', payload)
export const updateExpert = (id, payload) => http.put(`/experts/${encodeURIComponent(id)}`, payload)
export const removeExpert = (id) => http.delete(`/experts/${encodeURIComponent(id)}`)
export const consultExpert = (id, payload) => http.post(`/experts/${encodeURIComponent(id)}/consult`, payload)
export const multiExpertConsult = (payload) => http.post('/experts/multi-consult', payload)
export const expertDebate = (payload) => http.post('/experts/debate', payload)
export const getExpertCapabilities = () => http.get('/experts/capabilities')
export const routeExperts = (payload) => http.post('/experts/route', payload)
export const intelligentConsult = (payload) => http.post('/experts/intelligent-consult', payload)
export const algorithmAnalysis = (payload) => http.post('/experts/algorithm-analysis', payload)
export const getExpertMetrics = () => http.get('/experts/metrics')
export const getExpertOverview = () => http.get('/experts/overview')
export const getSingleExpertMetrics = (id) => http.get(`/experts/${encodeURIComponent(id)}/metrics`)

// ===== 企业级会话持久化 =====
export const createExpertSession = (payload) => http.post('/experts/sessions', payload)
export const listExpertSessions = (params) => http.get('/experts/sessions', { params })
export const getExpertSessionStats = () => http.get('/experts/sessions/stats')
export const getExpertSession = (id) => http.get(`/experts/sessions/${encodeURIComponent(id)}`)
export const updateExpertSession = (id, payload) => http.put(`/experts/sessions/${encodeURIComponent(id)}`, payload)
export const deleteExpertSession = (id) => http.delete(`/experts/sessions/${encodeURIComponent(id)}`)
export const appendSessionMessage = (id, payload) => http.post(`/experts/sessions/${encodeURIComponent(id)}/messages`, payload)
export const sessionSimilarSearch = (id, payload) => http.post(`/experts/sessions/${encodeURIComponent(id)}/similar-search`, payload)
export const expertSemanticSearch = (payload) => http.post('/experts/semantic-search', payload)
export const exportExpertSession = (id) => http.get(`/experts/sessions/${encodeURIComponent(id)}/export`)
export const archiveExpertSession = (id) => http.post(`/experts/sessions/${encodeURIComponent(id)}/archive`)

// ===== 企业级调度策略引擎 =====
export const getDispatcherConfig = () => http.get('/experts/dispatcher/config')
export const updateDispatcherConfig = (payload) => http.put('/experts/dispatcher/config', payload)
export const getDispatcherStatus = () => http.get('/experts/dispatcher/status')
export const dispatcherDispatch = (payload) => http.post('/experts/dispatcher/dispatch', payload)
export const dispatcherConsult = (payload) => http.post('/experts/dispatcher/consult', payload)
export const dispatcherMultiConsult = (payload) => http.post('/experts/dispatcher/multi-consult', payload)
export const resetDispatcherExpert = (id) => http.post(`/experts/dispatcher/reset/${encodeURIComponent(id)}`)
export const resetDispatcherAll = () => http.post('/experts/dispatcher/reset-all')

// ===== 专家能力图谱与协作网络 =====
export const getExpertGraph = () => http.get('/expert-graph')
export const getExpertGraphStats = () => http.get('/expert-graph/stats')
export const getExpertGraphNeighbors = (id) => http.get(`/expert-graph/neighbors/${encodeURIComponent(id)}`)
export const getExpertGraphCollaborators = (id, limit) => http.get(`/expert-graph/collaborators/${encodeURIComponent(id)}`, { params: { limit } })
export const getExpertGraphPath = (source, target) => http.get(`/expert-graph/path/${encodeURIComponent(source)}/${encodeURIComponent(target)}`)
export const getExpertGraphCommunities = () => http.get('/expert-graph/communities')
export const findOptimalTeam = (payload) => http.post('/expert-graph/optimal-team', payload)
export const rebuildExpertGraph = () => http.post('/expert-graph/rebuild')

// ===== 企业级协作端点 =====
export const enterpriseConsult = (payload) => http.post('/experts/enterprise/consult', payload)
export const enterpriseAnalyze = (payload) => http.post('/experts/enterprise/analyze', payload)

// ===== V2 编排引擎 =====
export const expertOrchestrate = (payload) => http.post('/experts/orchestrate', payload)
export const expertGeneratePlan = (payload) => http.post('/experts/plan/generate', payload)
export const expertExecutePlan = (payload) => http.post('/experts/plan/execute', payload)
export const getOrchestrationStats = () => http.get('/experts/orchestration/stats')
export const listOrchestrationPlugins = () => http.get('/experts/orchestration/plugins')
export const getOrchestrationHistory = (params) => http.get('/experts/orchestration/history', { params })

// ===== 任务管理（对话/任务双向转换） =====
export const getTasks = () => http.get('/tasks')
export const getTask = (id) => http.get(`/tasks/${encodeURIComponent(id)}`)
export const createTask = (payload) => http.post('/tasks', payload)
export const updateTask = (id, payload) => http.put(`/tasks/${encodeURIComponent(id)}`, payload)
export const deleteTask = (id) => http.delete(`/tasks/${encodeURIComponent(id)}`)
export const convertChatToTask = (payload) => http.post('/tasks/from-chat', payload)
export const convertTaskToChat = (id) => http.post(`/tasks/${encodeURIComponent(id)}/to-chat`)
export const executeTask = (id, payload) => http.post(`/tasks/${encodeURIComponent(id)}/execute`, payload)
export const autoCreateTask = (payload) => http.post('/tasks/auto', payload)

// ===== 16模块 AI 增强端点 =====
export const getWorkbenchAiOverview = () => http.get('/workbench/ai-overview')
export const aiRecommendOperators = (payload) => http.post('/operators/ai-recommend', payload)
export const aiGraphInsights = (payload) => http.post('/graph/ai-insights', payload)
export const aiExpertChat = (payload) => http.post('/ai/expert-chat', payload)
export const aiResourceAnalysis = (payload) => http.post('/resources/ai-analysis', payload)
export const aiGenerateWorkflow = (payload) => http.post('/workflow/ai-generate', payload)
export const aiPluginRoute = (payload) => http.post('/plugins/ai-route', payload)
export const aiBrowserInstruct = (payload) => http.post('/browser/ai-instruct', payload)
export const aiMonitorDiagnose = (payload) => http.post('/monitor/ai-diagnose', payload)
export const aiDocsExplain = (payload) => http.post('/docs/ai-explain', payload)
export const aiMarketSearch = (payload) => http.post('/market/ai-search', payload)
export const aiMcpMap = (payload) => http.post('/mcp/ai-map', payload)
export const aiAutomationExecute = (payload) => http.post('/automation/ai-execute', payload)
export const aiCaomeiParse = (payload) => http.post('/caomei/ai-parse', payload)
export const aiAlgoLabAnalyze = (payload) => http.post('/algolab/ai-analyze', payload)
export const aiFusionGovern = (payload) => http.post('/fusion/ai-govern', payload)

// ===== 云盘知识库 =====
export const kbListDocuments = (params) => http.get('/kb/documents', { params })
export const kbGetDocument = (id) => http.get(`/kb/documents/${encodeURIComponent(id)}`)
export const kbCreateDocument = (payload) => http.post('/kb/documents', payload)
export const kbUpdateDocument = (id, payload) => http.put(`/kb/documents/${encodeURIComponent(id)}`, payload)
export const kbDeleteDocument = (id) => http.delete(`/kb/documents/${encodeURIComponent(id)}`)
export const kbAnalyzeDocument = (id) => http.post(`/kb/documents/${encodeURIComponent(id)}/analyze`)
export const kbBatchAnalyze = (payload) => http.post('/kb/batch-analyze', payload)
export const kbGetCategories = () => http.get('/kb/categories')
export const kbGetTags = () => http.get('/kb/tags')
export const kbSearch = (payload) => http.post('/kb/search', payload)
export const kbGetVersions = (id) => http.get(`/kb/documents/${encodeURIComponent(id)}/versions`)
export const kbGetVersion = (id, ver) => http.get(`/kb/documents/${encodeURIComponent(id)}/versions/${encodeURIComponent(ver)}`)
export const kbCreateVersion = (id, payload) => http.post(`/kb/documents/${encodeURIComponent(id)}/versions`, payload)
export const kbCompareVersions = (id, payload) => http.post(`/kb/documents/${encodeURIComponent(id)}/versions/compare`, payload)
export const kbRevertVersion = (id, payload) => http.post(`/kb/documents/${encodeURIComponent(id)}/versions/revert`, payload)
export const kbGetEntities = (id) => http.get(`/kb/documents/${encodeURIComponent(id)}/entities`)
export const kbGraphLink = (id, payload) => http.post(`/kb/documents/${encodeURIComponent(id)}/graph-link`, payload)
export const kbGetStats = () => http.get('/kb/stats')
export const kbGetDocHistory = (id) => http.get(`/kb/documents/${encodeURIComponent(id)}/history`)
export const kbGetHistory = (params) => http.get('/kb/history', { params })

// ===== 全维智能分析引擎（真实 AI 驱动） =====
export const aiFullAnalysis = (payload) => http.post('/ai/full-analysis', payload)
export const aiGenerateDoc = (payload) => http.post('/ai/generate-doc', payload)
export const aiGenerateFlowDiagram = (payload) => http.post('/ai/generate-flow-diagram', payload)
export const aiDevTestFix = (payload) => http.post('/ai/dev-test-fix', payload)
export const aiFullComplete = (payload) => http.post('/ai/full-complete', payload)
export const aiOptimizeDoc = (payload) => http.post('/ai/optimize-doc', payload)

// ===== Melody2Score 企业级旋律转谱 =====
export const melodyHealth = () => http.get('/melody2score/health')
export const melodyStatus = () => http.get('/melody2score/status')
export const melodySamples = () => http.get('/melody2score/samples')
export const melodyRecognize = (formData) => http.post('/melody2score/recognize', formData, {
  headers: { 'Content-Type': 'multipart/form-data' },
  timeout: 120000
})
export const melodyRecognizeSample = (formData) => http.post('/melody2score/recognize-sample', formData, {
  headers: { 'Content-Type': 'multipart/form-data' },
  timeout: 120000
})
export const melodyRecognizeRecord = (payload) => http.post('/melody2score/recognize-record', payload, { timeout: 120000 })
export const melodyExportSheet = (payload) => http.post('/melody2score/export-sheet', payload, { timeout: 60000 })
export const melodySaveReport = (payload) => http.post('/melody2score/save-report', payload, { timeout: 30000 })

export default http
