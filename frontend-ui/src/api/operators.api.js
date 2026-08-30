// 算子与算子商城 API
import http from './http'

// ===== 算子 =====
export const getOperators = () => http.get('/operators')
export const registerOperator = (payload) => http.post('/operators/register', payload)
export const executeWorkflow = (payload) => http.post('/execute', payload)

// ===== 算子商城 (Operator Market) =====
export const marketList = (params) => http.get('/market', { params })
export const marketRandom = () => http.get('/market/random')
export const marketGet = (id) => http.get(`/market/${encodeURIComponent(id)}`)
export const marketUpload = (payload) => http.post('/market/upload', payload)
export const marketUpdate = (id, payload) => http.post(`/market/${encodeURIComponent(id)}`, payload)
export const marketDelete = (id) => http.delete(`/market/${encodeURIComponent(id)}`)
export const marketClone = (id) => http.post(`/market/${encodeURIComponent(id)}/clone`)
// 市场导出：将算子包导出为可移植 DSL 工程
export const marketExport = (id) => http.get(`/market/${encodeURIComponent(id)}/export`)
