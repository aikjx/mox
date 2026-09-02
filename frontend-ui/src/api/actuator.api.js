/**
 * 网关管理面（Actuator）API —— Spring Boot Actuator 风格管理端点
 *
 * 对应后端：platform/gateway/mox-platform-gateway-svc/src/actuator.rs
 * 统一经独立实例 actuatorHttp（baseURL=/actuator）访问，与业务面（/api）隔离，
 * 共享 http.js 的鉴权注入 / 自动重试 / 错误归一化拦截器。
 *
 * 端点清单：
 * - GET  /actuator                      管理面索引
 * - GET  /actuator/health               健康检查
 * - GET  /actuator/info                 构建信息
 * - GET  /actuator/env                  网关配置（密钥脱敏）
 * - GET  /actuator/metrics              运行时指标
 * - GET  /actuator/mappings             API 注册表（?layer&domain&status&q&only_enabled）
 * - GET  /actuator/api/:id              单个 API 详情
 * - POST /actuator/api/:id/enable       启用 API
 * - POST /actuator/api/:id/disable      停用 API（管理面端点防自锁）
 * - GET  /actuator/loggers              当前日志级别
 * - POST /actuator/loggers              {level} 动态调整日志级别
 * - GET  /actuator/logs                 在线日志查询（?level&search&limit&offset，最新在前）
 * - DELETE /actuator/logs               清空日志缓冲
 * - GET  /actuator/logs/tail            SSE 实时日志流（?limit 回放条数）
 */
import { actuatorHttp } from './http'
import { getToken } from '@/utils/secureStorage'

// ===== 管理面基础 =====
export const getActuatorIndex = () => actuatorHttp.get('/actuator')
export const getActuatorHealth = () => actuatorHttp.get('/actuator/health')
export const getActuatorInfo = () => actuatorHttp.get('/actuator/info')
export const getActuatorEnv = () => actuatorHttp.get('/actuator/env')
export const getActuatorMetrics = () => actuatorHttp.get('/actuator/metrics')

// ===== API 注册表（接口管理）=====
export const getApiMappings = (params) => actuatorHttp.get('/actuator/mappings', { params })
export const getApiDetail = (id) => actuatorHttp.get(`/actuator/api/${encodeURIComponent(id)}`)
export const enableApi = (id) => actuatorHttp.post(`/actuator/api/${encodeURIComponent(id)}/enable`)
export const disableApi = (id) => actuatorHttp.post(`/actuator/api/${encodeURIComponent(id)}/disable`)

// ===== 在线日志 =====
export const getLoggers = () => actuatorHttp.get('/actuator/loggers')
export const setLoggerLevel = (level) => actuatorHttp.post('/actuator/loggers', { level })
export const getOnlineLogs = (params) => actuatorHttp.get('/actuator/logs', { params })
export const clearOnlineLogs = () => actuatorHttp.delete('/actuator/logs')

/**
 * 建立 SSE 实时日志流连接（/actuator/logs/tail）
 * 说明：SSE 需保持长连接，无法走 axios 的常规响应处理，故用原生 fetch 返回 Response，
 * 由调用方通过 ReadableStream 逐事件解析。?limit 指定回放的最近条数。
 * @param {Object} [params] { limit?, level? }
 * @returns {Promise<Response>}
 */
export async function openLogTail(params = {}) {
  const qs = new URLSearchParams()
  Object.entries(params).forEach(([k, v]) => {
    if (v !== undefined && v !== null && v !== '') qs.set(k, String(v))
  })
  const q = qs.toString()
  const headers = { Accept: 'text/event-stream' }
  const token = getToken()
  if (token) headers['Authorization'] = `Bearer ${token}`
  // 请求追踪 ID，与 http.js 的链路追踪约定保持一致
  headers['X-Request-Id'] =
    (typeof crypto !== 'undefined' && crypto.randomUUID?.()) ||
    `req_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
  return fetch(`/actuator/logs/tail${q ? `?${q}` : ''}`, { headers })
}
