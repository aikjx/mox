// 云盘知识库 API
import http from './http'

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
export const kbSearchEntities = (params) => http.get('/kb/entities/search', { params })
export const kbLinkEntity = (docId, payload) =>
  http.post(`/kb/documents/${encodeURIComponent(docId)}/entities`, payload, { projectContext: false })
export const kbUnlinkEntity = (docId, payload) =>
  http.delete(`/kb/documents/${encodeURIComponent(docId)}/entities`, { data: payload, projectContext: false })
// 挂图/解图都是文档级动作，两个 handler 都只取 Path(:id)，不收请求体
export const kbGraphLink = (id) => http.post(`/kb/documents/${encodeURIComponent(id)}/graph-link`)
export const kbGraphUnlink = (id) => http.delete(`/kb/documents/${encodeURIComponent(id)}/graph-link`)
export const kbGetStats = () => http.get('/kb/stats')
export const kbGetDocHistory = (id) => http.get(`/kb/documents/${encodeURIComponent(id)}/history`)
export const kbGetHistory = (params) => http.get('/kb/history', { params })

// One atomic edit: the server archives the old content and advances the version.
export function kbSaveDocumentEdit(data) {
  if (!data.id || typeof data.current_version !== 'string' || !data.current_version.trim()) {
    throw new Error('请重新读取完整文档后编辑')
  }
  return kbUpdateDocument(data.id, {
    title: data.title, content: data.content, category: data.category, tags: data.tags,
    expected_current_version: data.current_version,
    version_note: (data.version_note || '').trim()
  })
}
