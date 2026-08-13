import axios from 'axios'

const http = axios.create({
  baseURL: '/api',
  timeout: 60000,
})

// 统一错误包装
http.interceptors.response.use(
  (res) => res.data,
  (err) => {
    const msg = err.response?.data?.error || err.message || '网络请求失败'
    return Promise.reject(new Error(msg))
  }
)

/**
 * 发送对话消息
 * @param {string|null} sessionId
 * @param {string} message
 */
export function aiChat(sessionId, message) {
  return http.post('/ai/chat', { session_id: sessionId, message })
}

/** 获取会话历史 */
export function chatHistory(session) {
  return http.get(`/ai/chat/history/${encodeURIComponent(session)}`)
}

/** 知识图谱 */
export function getGraph() {
  return http.get('/graph')
}

/** 图谱统计 */
export function getGraphStats() {
  return http.get('/graph/stats')
}

/** 算子列表 */
export function listOperators() {
  return http.get('/operators')
}

/** 流程列表 */
export function listFlows() {
  return http.get('/ai/flows')
}

/** 执行流程 */
export function executeFlow(flowId, input = {}) {
  return http.post('/ai/flows/execute', { flow_id: flowId, input })
}

/** 系统状态 */
export function getStatus() {
  return http.get('/status')
}

/** 完整状态 */
export function getFullStatus() {
  return http.get('/status/full')
}

export default http
