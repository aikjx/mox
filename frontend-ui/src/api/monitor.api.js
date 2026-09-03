// 监控域 API - 系统监控、告警、时序指标
import http from './http'

// ===== 系统资源指标详情 =====
export const getMetricsDetail = (params) => http.get('/monitor/metrics/detail', { params })

// ===== 服务质量指标 =====
export const getMonitorQuality = () => http.get('/monitor/quality')

// ===== 业务指标聚合 =====
export const getMonitorBusiness = () => http.get('/monitor/business')

// ===== 告警统计摘要 =====
export const getAlertsSummary = () => http.get('/monitor/alerts/summary')

// ===== 服务节点状态（服务发现） =====
export const getMonitorNodes = () => http.get('/monitor/nodes')

// ===== 服务节点日志 =====
export const getNodeLogs = (nodeName, params) =>
  http.get(`/monitor/nodes/${encodeURIComponent(nodeName)}/logs`, { params })

// ===== 服务节点链路追踪 =====
export const getNodeTrace = (nodeName, traceId) =>
  http.get(`/monitor/nodes/${encodeURIComponent(nodeName)}/trace`, { params: { trace_id: traceId } })

// ===== 告警规则 CRUD =====
export const getAlertRules = (params) => http.get('/monitor/alert-rules', { params })
export const createAlertRule = (data) => http.post('/monitor/alert-rules', data)
export const updateAlertRule = (id, data) => http.put(`/monitor/alert-rules/${encodeURIComponent(id)}`, data)
export const deleteAlertRule = (id) => http.delete(`/monitor/alert-rules/${encodeURIComponent(id)}`)
export const toggleAlertRule = (id, enabled) =>
  http.put(`/monitor/alert-rules/${encodeURIComponent(id)}/toggle`, { enabled })

// ===== 时序指标查询 =====
export const getTimeseries = (params) => http.get('/monitor/timeseries', { params })

// ===== 业务量时序查询 =====
export const getBusinessTimeseries = (params) => http.get('/monitor/business/timeseries', { params })
