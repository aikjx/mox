// 项目、任务、资源 API
import http from './http'

// ===== 项目中心 =====
export const getProjects = () => http.get('/projects')
export const getProjectTypes = () => http.get('/projects/types')
export const getProjectCatalog = () => http.get('/projects/catalog')
export const getProjectStats = () => http.get('/projects/stats')
export const getProject = (id) => http.get(`/projects/${encodeURIComponent(id)}`)
export const createProject = (payload) => http.post('/projects', payload)
export const updateProject = (id, payload) => http.put(`/projects/${encodeURIComponent(id)}`, payload)
export const deleteProject = (id) => http.delete(`/projects/${encodeURIComponent(id)}`)
export const bindProjectResources = (id, payload) =>
  http.post(`/projects/${encodeURIComponent(id)}/resources`, payload)
export const unbindProjectResource = (id, rid) =>
  http.delete(`/projects/${encodeURIComponent(id)}/resources/${encodeURIComponent(rid)}`)
export const updateProjectResourceNote = (id, rid, payload) =>
  http.put(`/projects/${encodeURIComponent(id)}/resources/${encodeURIComponent(rid)}`, payload)
export const getProjectsByResource = (type, resourceId) =>
  http.get('/projects/by-resource', { params: { type, id: resourceId } })

// ===== 任务管理 =====
export const getTasks = () => http.get('/tasks')
export const getTask = (id) => http.get(`/tasks/${encodeURIComponent(id)}`)
export const createTask = (payload) => http.post('/tasks', payload)
export const updateTask = (id, payload) => http.put(`/tasks/${encodeURIComponent(id)}`, payload)
export const deleteTask = (id) => http.delete(`/tasks/${encodeURIComponent(id)}`)
export const convertChatToTask = (payload) => http.post('/tasks/from-chat', payload)
export const convertTaskToChat = (id) => http.post(`/tasks/${encodeURIComponent(id)}/to-chat`)
export const executeTask = (id, payload) => http.post(`/tasks/${encodeURIComponent(id)}/execute`, payload)
export const autoCreateTask = (payload) => http.post('/tasks/auto', payload)

// ===== 资源 =====
export const getResources = () => http.get('/ai/resources')
export const getResourceHealth = () => http.get('/ai/resources/health')
