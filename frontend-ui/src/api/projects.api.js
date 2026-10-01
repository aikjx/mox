// 项目、任务、资源 API
import http from './http'

// ===== 项目中心 =====
// `/api/projects` 与 `/api/tasks` 都由网关 misc.rs 的分页 handler 出参（同一套 wire 键），
// 所以"列表"和"服务端分页"是**同一个 URL**，只差查询串。
// （这里曾有两条幻影路由：把 paginated 当子路径去叫，一条 404、一条 502，均已删。）
export const getProjects = (params) => http.get('/projects', { params })
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
export const getTasks = (params) => http.get('/tasks', { params })
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

// ===== 项目扩展端点（Task 2） =====
// 项目动态流
export const getProjectActivities = (projectId, params) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/activities`, { params })

// 项目文档列表
export const getProjectDocuments = (projectId, params) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/documents`, { params })

// 项目成员管理（读 + 三个写动作同属项目域）
export const getProjectMembers = (projectId, params) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/members`, { params })
export const addProjectMember = (projectId, data) =>
  http.post(`/projects/${encodeURIComponent(projectId)}/members`, data)
export const updateProjectMember = (projectId, memberId, data) =>
  http.put(`/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(memberId)}`, data)
export const removeProjectMember = (projectId, memberId) =>
  http.delete(`/projects/${encodeURIComponent(projectId)}/members/${encodeURIComponent(memberId)}`)

// 项目阶段推进
export const advanceProjectPhase = (projectId, data) =>
  http.put(`/projects/${encodeURIComponent(projectId)}/advance-phase`, data)

// 项目阶段进度
export const getProjectPhaseProgress = (projectId) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/phase-progress`)

// 项目收藏切换
export const toggleProjectFavorite = (projectId) =>
  http.post(`/projects/${encodeURIComponent(projectId)}/favorite`)

// 项目分享
export const shareProject = (projectId, data) =>
  http.post(`/projects/${encodeURIComponent(projectId)}/share`, data)

// 项目文档下载
export const downloadProjectDocument = (projectId, docId) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/documents/${encodeURIComponent(docId)}/download`, { responseType: 'blob' })

// 需求图谱
export const getRequirementsGraph = (projectId) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/requirements-graph`)

// AI 智能推荐项目配置
export const aiRecommendProject = (payload) => http.post('/projects/ai-recommend', payload)
