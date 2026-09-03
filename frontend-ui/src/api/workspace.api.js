// 工作台域 API - 通知、KPI、项目成员、阶段、文件、白板、历史、任务
import http from './http'

// ===== 通知未读数 =====
export const getUnreadCount = () => http.get('/notifications/unread-count')

// ===== 工作台 KPI 聚合统计 =====
export const getWorkspaceKpi = () => http.get('/workspace/kpi')

// ===== 项目成员 =====
export const getProjectMembers = (projectId) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/members`)

// ===== 项目阶段 =====
export const getProjectPhases = (projectId) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/phases`)

// ===== 项目文件 =====
export const getProjectFiles = (projectId, params) =>
  http.get(`/projects/${encodeURIComponent(projectId)}/files`, { params })

// ===== 项目文件上传 =====
export const uploadProjectFile = (projectId, file) => {
  const formData = new FormData()
  formData.append('file', file)
  return http.post(`/projects/${encodeURIComponent(projectId)}/files/upload`, formData, {
    headers: { 'Content-Type': 'multipart/form-data' }
  })
}

// ===== 文件预览 =====
export const getFilePreview = (fileId) =>
  http.get(`/files/${encodeURIComponent(fileId)}/preview`)

// ===== 文件下载 =====
export const getFileDownload = (fileId) =>
  http.get(`/files/${encodeURIComponent(fileId)}/download`, { responseType: 'blob' })

// ===== 白板持久化 =====
export const saveWhiteboard = (sessionId, data) =>
  http.post(`/whiteboard/${encodeURIComponent(sessionId)}/save`, data)

// ===== 工作台历史记录 =====
export const getWorkspaceHistory = (params) => http.get('/workspace/history', { params })

// ===== 任务智能拆解 =====
export const decomposeTask = (data) => http.post('/tasks/decompose', data)

// ===== 任务执行 =====
export const executeTask = (taskId, payload) =>
  http.post(`/tasks/${encodeURIComponent(taskId)}/execute`, payload)
