// LLM 网关 API
import http from './http'

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

// LLM 配置（旧接口兼容）
export const getLlmConfig = () => http.get('/ai/llm/config')
export const updateLlmConfig = (payload) => http.post('/ai/llm/config', payload)
export const testLlm = () => http.post('/ai/llm/test')
