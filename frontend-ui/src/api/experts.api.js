// 专家联盟 API - 专家、企业协作、编排引擎、专家图谱
import http from './http'

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
