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

// ===== 权限管理 =====
// 获取当前用户权限信息（权限列表、角色、菜单、数据权限）
export const getPermissions = () => http.get('/system/permissions')
// 获取菜单树
export const getMenuTree = () => http.get('/system/menus')
// 获取角色列表
export const getRoles = (params) => http.get('/system/roles', { params })

// ===== 企业级系统管理 =====
// 部门管理
export const getDeptList = (params) => http.get('/system/dept', { params })
export const getDeptTree = (params) => http.get('/system/dept/tree', { params })
export const getDeptDetail = (id) => http.get(`/system/dept/${id}`)
export const createDept = (data) => http.post('/system/dept', data)
export const updateDept = (id, data) => http.put(`/system/dept/${id}`, data)
export const deleteDept = (id) => http.delete(`/system/dept/${id}`)
export const getDeptUserList = (deptId, params) => http.get(`/system/dept/${deptId}/users`, { params })

// 岗位管理
export const getPostList = (params) => http.get('/system/post', { params })
export const getPostByDept = (deptId) => http.get(`/system/post/dept/${deptId}`)
export const getPostDetail = (id) => http.get(`/system/post/${id}`)
export const createPost = (data) => http.post('/system/post', data)
export const updatePost = (id, data) => http.put(`/system/post/${id}`, data)
export const deletePost = (id) => http.delete(`/system/post/${id}`)

// 用户管理
export const getUserList = (params) => http.get('/system/user', { params })
export const getUserDetail = (id) => http.get(`/system/user/${id}`)
export const createUser = (data) => http.post('/system/user', data)
export const updateUser = (id, data) => http.put(`/system/user/${id}`, data)
export const deleteUser = (id) => http.delete(`/system/user/${id}`)
export const resetUserPwd = (id, data) => http.put(`/system/user/${id}/resetPwd`, data)
export const changeUserStatus = (id, status) => http.put(`/system/user/${id}/changeStatus`, { status })
export const getUserRoles = (id) => http.get(`/system/user/${id}/roles`)
export const assignUserRoles = (id, data) => http.put(`/system/user/${id}/roles`, data)

// 角色管理
export const getRoleList = (params) => http.get('/system/role', { params })
export const getRoleDetail = (id) => http.get(`/system/role/${id}`)
export const createRole = (data) => http.post('/system/role', data)
export const updateRole = (id, data) => http.put(`/system/role/${id}`, data)
export const deleteRole = (id) => http.delete(`/system/role/${id}`)
export const getRoleMenuPerms = (id) => http.get(`/system/role/${id}/menuPerms`)
export const assignRoleMenuPerms = (id, data) => http.put(`/system/role/${id}/menuPerms`, data)
export const getRoleDataPerms = (id) => http.get(`/system/role/${id}/dataPerms`)
export const assignRoleDataPerms = (id, data) => http.put(`/system/role/${id}/dataPerms`, data)
export const getRoleUsers = (id, params) => http.get(`/system/role/${id}/users`, { params })
export const copyRole = (id, data) => http.post(`/system/role/${id}/copy`, data)

// 菜单管理
export const getMenuTree = (params) => http.get('/system/menu/tree', { params })
export const getMenuList = (params) => http.get('/system/menu', { params })
export const getMenuDetail = (id) => http.get(`/system/menu/${id}`)
export const createMenu = (data) => http.post('/system/menu', data)
export const updateMenu = (id, data) => http.put(`/system/menu/${id}`, data)
export const deleteMenu = (id) => http.delete(`/system/menu/${id}`)

// 字典类型
export const getDictTypeList = (params) => http.get('/system/dict/type', { params })
export const getDictTypeAll = () => http.get('/system/dict/type/all')
export const getDictTypeDetail = (id) => http.get(`/system/dict/type/${id}`)
export const createDictType = (data) => http.post('/system/dict/type', data)
export const updateDictType = (id, data) => http.put(`/system/dict/type/${id}`, data)
export const deleteDictType = (id) => http.delete(`/system/dict/type/${id}`)

// 字典数据
export const getDictDataList = (params) => http.get('/system/dict/data', { params })
export const getDictDataByType = (dictType) => http.get(`/system/dict/data/type/${dictType}`)
export const getDictDataDetail = (id) => http.get(`/system/dict/data/${id}`)
export const createDictData = (data) => http.post('/system/dict/data', data)
export const updateDictData = (id, data) => http.put(`/system/dict/data/${id}`, data)
export const deleteDictData = (id) => http.delete(`/system/dict/data/${id}`)

// 参数配置
export const getConfigList = (params) => http.get('/system/config', { params })
export const getConfigDetail = (id) => http.get(`/system/config/${id}`)
export const getConfigByKey = (key) => http.get(`/system/config/key/${key}`)
export const createConfig = (data) => http.post('/system/config', data)
export const updateConfig = (id, data) => http.put(`/system/config/${id}`, data)
export const deleteConfig = (id) => http.delete(`/system/config/${id}`)
export const refreshConfigCache = () => http.delete('/system/config/refresh-cache')

// 操作日志
export const getOperLogList = (params) => http.get('/system/operlog', { params })
export const getOperLogDetail = (id) => http.get(`/system/operlog/${id}`)
export const deleteOperLog = (id) => http.delete(`/system/operlog/${id}`)
export const cleanOperLog = () => http.delete('/system/operlog/clean')
export const exportOperLog = (params) => http.get('/system/operlog/export', { params, responseType: 'blob' })

// 登录日志
export const getLoginLogList = (params) => http.get('/system/logininfor', { params })
export const deleteLoginLog = (id) => http.delete(`/system/logininfor/${id}`)
export const cleanLoginLog = () => http.delete('/system/logininfor/clean')
export const exportLoginLog = (params) => http.get('/system/logininfor/export', { params, responseType: 'blob' })
