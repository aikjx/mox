// 系统 API - 健康检查、状态、日志、插件、安全、存储、配置
import http from './http'

// ===== 系统 =====
export const getHealth = () => http.get('/health')
export const getStatus = () => http.get('/status')
export const getFullStatus = () => http.get('/status/full')
export const getLogs = () => http.get('/logs')
export const getPlugins = () => http.get('/plugins')

// ===== 系统管理区（安全凭证 / 审计日志 / 存储 / 模块）=====
// 凭证：创建返回一次性明文 key（后端仅存哈希），吊销按 id
export const getSecurityStatus = () => http.get('/security/status')
export const getApiKeys = () => http.get('/security/api-keys')
export const createApiKey = (payload) => http.post('/security/api-keys', payload)
export const revokeApiKey = (id) => http.delete(`/security/api-keys/${encodeURIComponent(id)}`)
export const validateApiKey = (apiKey) => http.post('/security/validate', { api_key: apiKey })
// 审计：支持 action / actor / since / limit 过滤
export const getAuditLogs = (params) => http.get('/security/audit-log', { params })
// 存储与模块
export const getStorageProviders = () => http.get('/storage/providers')
export const switchStorageProvider = (provider) => http.post('/storage/switch', { provider })
export const getStorageStatus = () => http.get('/storage/status')
export const getModules = () => http.get('/modules')
// 系统配置（只读）
export const getSystemConfig = () => http.get('/config')
