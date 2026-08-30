// AI 对话与全维分析 API
import http from './http'

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

// ===== 无穷维度优化引擎 =====
export const getInfiniteBenchmarks = () => http.get('/ai/infinite-optimize/benchmarks')
export const startInfiniteOptimize = (payload) => http.post('/ai/infinite-optimize/start', payload)
export const stopInfiniteOptimize = () => http.post('/ai/infinite-optimize/stop', {})
export const getInfiniteOptimizeStatus = () => http.get('/ai/infinite-optimize/status')
export const getInfiniteOptimizeResults = () => http.get('/ai/infinite-optimize/results')
export const runProviderComparison = () => http.post('/ai/infinite-optimize/compare', {})
export const getProviderComparison = () => http.get('/ai/infinite-optimize/comparison')
export const applyBestConfig = (runId) => http.post('/ai/infinite-optimize/apply', { run_id: runId })

// ===== 本地制品引擎（文档/代码自动创建） =====
export const getArtifactConfig = () => http.get('/ai/artifact/config')
export const listArtifacts = () => http.get('/ai/artifact/list')
export const createArtifact = (payload) => http.post('/ai/artifact/create', payload)

// ===== 全维智能分析引擎（真实 AI 驱动） =====
export const aiFullAnalysis = (payload) => http.post('/ai/full-analysis', payload)
export const aiGenerateDoc = (payload) => http.post('/ai/generate-doc', payload)
export const aiGenerateFlowDiagram = (payload) => http.post('/ai/generate-flow-diagram', payload)
export const aiDevTestFix = (payload) => http.post('/ai/dev-test-fix', payload)
export const aiFullComplete = (payload) => http.post('/ai/full-complete', payload)
export const aiOptimizeDoc = (payload) => http.post('/ai/optimize-doc', payload)

// ===== 项目需求一体化 =====
// 对话 → 项目：基于当前会话上下文创建项目 + 需求全维建模
export const aiProjectFromChat = (payload) => http.post('/ai/project-from-chat', payload)
// 项目→需求流程图知识图谱
export const aiGenerateProjectGraph = (payload) => http.post('/ai/project-graph', payload)
// 需求↔数据库关联建模
export const aiLinkReqToDb = (payload) => http.post('/ai/req-db-link', payload)
// 产品专家联盟企业级流水线
export const allianceEnterprisePipeline = (payload) => http.post('/ai/alliance-pipeline', payload)
// 项目→云盘：将需求文档/流程图/数据库建模产物写入云盘知识库
export const aiPublishArtifactsToKb = (payload) => http.post('/ai/publish-kb', payload)
// 需求↔数据库 ER 图生成
export const aiGenerateErd = (payload) => http.post('/ai/generate-erd', payload)

// AI 专家对话
export const aiExpertChat = (payload) => http.post('/ai/expert-chat', payload)

// AI 流程图谱
export const getEngineFlowGraph = () => http.get('/ai/engine/flow-graph')

// 16模块 AI 增强端点 - AI 相关
export const aiRecommendOperators = (payload) => http.post('/operators/ai-recommend', payload)
export const aiResourceAnalysis = (payload) => http.post('/resources/ai-analysis', payload)
export const aiGenerateWorkflow = (payload) => http.post('/workflow/ai-generate', payload)
export const aiMarketSearch = (payload) => http.post('/market/ai-search', payload)
export const aiMcpMap = (payload) => http.post('/mcp/ai-map', payload)
export const aiCaomeiParse = (payload) => http.post('/caomei/ai-parse', payload)
export const aiAlgoLabAnalyze = (payload) => http.post('/algolab/ai-analyze', payload)
export const aiFusionGovern = (payload) => http.post('/fusion/ai-govern', payload)
export const aiMonitorDiagnose = (payload) => http.post('/monitor/ai-diagnose', payload)
export const aiDocsExplain = (payload) => http.post('/docs/ai-explain', payload)
export const aiPluginRoute = (payload) => http.post('/plugins/ai-route', payload)
export const aiBrowserInstruct = (payload) => http.post('/browser/ai-instruct', payload)
export const aiAutomationExecute = (payload) => http.post('/automation/ai-execute', payload)
export const getWorkbenchAiOverview = () => http.get('/workbench/ai-overview')
