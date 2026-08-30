// 需求编译 (Caomei) API
import http from './http'

export const caomeiCompile = (payload) => http.post('/caomei/compile', payload)
export const caomeiRefine = (payload) => http.post('/caomei/refine', payload)
export const caomeiTemplates = (params) => http.get('/caomei/templates', { params })
