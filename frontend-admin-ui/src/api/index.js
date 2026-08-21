import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  timeout: 30000
})

api.interceptors.request.use(config => {
  const token = localStorage.getItem('admin_token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

api.interceptors.response.use(
  response => response.data,
  error => {
    console.error('[Admin API Error]', error.message)
    return Promise.reject(error)
  }
)

export const adminApi = {
  getUsers: (params) => api.get('/admin/users', { params }),
  createUser: (data) => api.post('/admin/users', data),
  updateUser: (id, data) => api.put(`/admin/users/${id}`, data),
  deleteUser: (id) => api.delete(`/admin/users/${id}`),

  getRoles: () => api.get('/admin/roles'),
  createRole: (data) => api.post('/admin/roles', data),
  updateRole: (id, data) => api.put(`/admin/roles/${id}`, data),
  deleteRole: (id) => api.delete(`/admin/roles/${id}`),

  getLlmProviders: () => api.get('/admin/llm/providers'),
  createLlmProvider: (data) => api.post('/admin/llm/providers', data),
  updateLlmProvider: (id, data) => api.put(`/admin/llm/providers/${id}`, data),
  deleteLlmProvider: (id) => api.delete(`/admin/llm/providers/${id}`),
  setDefaultLlm: (providerId, model) => api.post('/admin/llm/default', { providerId, model }),

  getLlmRouting: () => api.get('/admin/llm/routing'),
  saveLlmRouting: (data) => api.put('/admin/llm/routing', data),

  getLlmUsage: (params) => api.get('/admin/llm/usage', { params }),

  getKnowledgeBases: (params) => api.get('/admin/knowledge', { params }),
  createKnowledgeBase: (data) => api.post('/admin/knowledge', data),
  updateKnowledgeBase: (id, data) => api.put(`/admin/knowledge/${id}`, data),
  deleteKnowledgeBase: (id) => api.delete(`/admin/knowledge/${id}`),

  getKnowledgePermissions: (kbId) => api.get(`/admin/knowledge/${kbId}/permissions`),
  setKnowledgePermissions: (kbId, data) => api.put(`/admin/knowledge/${kbId}/permissions`, data),

  getStoragePaths: () => api.get('/admin/storage/paths'),
  configureStoragePath: (data) => api.post('/admin/storage/paths', data),
  setStoragePermissions: (pathId, data) => api.put(`/admin/storage/paths/${pathId}/permissions`, data),

  getAuditLogs: (params) => api.get('/admin/audit/logs', { params }),

  getSystemConfig: () => api.get('/admin/system/config'),
  updateSystemConfig: (data) => api.put('/admin/system/config', data),

  getSystemInfo: () => api.get('/admin/system/info'),
  getSystemSecurity: () => api.get('/admin/system/security'),
  updateSystemSecurity: (data) => api.put('/admin/system/security', data)
}

export default api
