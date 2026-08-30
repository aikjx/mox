// 用户 Store - 用户信息、认证状态、权限、偏好等
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

const USER_KEY = 'mox-user'
const TOKEN_KEY = 'mox-token'

function getStoredUser() {
  if (typeof localStorage === 'undefined') return null
  try {
    const raw = localStorage.getItem(USER_KEY)
    return raw ? JSON.parse(raw) : null
  } catch {
    return null
  }
}

function setStoredUser(user) {
  if (typeof localStorage === 'undefined') return
  if (user) {
    localStorage.setItem(USER_KEY, JSON.stringify(user))
  } else {
    localStorage.removeItem(USER_KEY)
  }
}

function getStoredToken() {
  if (typeof localStorage === 'undefined') return ''
  return localStorage.getItem(TOKEN_KEY) || ''
}

function setStoredToken(token) {
  if (typeof localStorage === 'undefined') return
  if (token) {
    localStorage.setItem(TOKEN_KEY, token)
  } else {
    localStorage.removeItem(TOKEN_KEY)
  }
}

export const useUserStore = defineStore('user', () => {
  // ===== State =====

  // 用户基本信息
  const profile = ref(getStoredUser() || {
    id: 'u_001',
    username: 'admin',
    nickname: '璇玑管理员',
    email: 'admin@mox.local',
    avatar: '',
    phone: '',
    department: '研发中心',
    position: '系统架构师',
  })

  // 认证令牌
  const token = ref(getStoredToken())

  // 登录状态
  const isLoggedIn = ref(!!token.value)

  // 用户角色
  const roles = ref(['admin', 'developer'])

  // 用户权限列表
  const permissions = ref([
    'project:read', 'project:write', 'project:delete',
    'expert:read', 'expert:write',
    'workflow:read', 'workflow:write', 'workflow:execute',
    'graph:read', 'graph:write',
    'market:read', 'market:install',
    'admin:read', 'admin:write',
    'ai:chat', 'ai:algorithm',
  ])

  // 用户偏好设置
  const preferences = ref({
    language: 'zh-CN',
    timezone: 'Asia/Shanghai',
    sidebarCollapsed: false,
    notificationEnabled: true,
    soundEnabled: false,
    autoSave: true,
    aiSuggestions: true,
  })

  // 最近访问的项目
  const recentProjects = ref([])

  // 加载状态
  const loading = ref(false)

  // ===== Getters =====

  const displayName = computed(() => profile.value.nickname || profile.value.username || '未登录用户')

  const initials = computed(() => {
    const name = displayName.value
    if (!name) return '?'
    return name.charAt(0).toUpperCase()
  })

  const isAdmin = computed(() => roles.value.includes('admin'))

  const isDeveloper = computed(() => roles.value.includes('developer'))

  const avatarUrl = computed(() => {
    if (profile.value.avatar) return profile.value.avatar
    // 使用默认头像（首字母彩色背景）
    return null
  })

  function hasPermission(perm) {
    if (isAdmin.value) return true
    return permissions.value.includes(perm)
  }

  function hasAnyPermission(perms) {
    if (isAdmin.value) return true
    return perms.some(p => permissions.value.includes(p))
  }

  function hasRole(role) {
    return roles.value.includes(role)
  }

  // ===== Actions =====

  function setProfile(data) {
    profile.value = { ...profile.value, ...data }
    setStoredUser(profile.value)
  }

  function setToken(newToken) {
    token.value = newToken
    isLoggedIn.value = !!newToken
    setStoredToken(newToken)
  }

  function login(userData, userToken) {
    profile.value = { ...profile.value, ...userData }
    token.value = userToken
    isLoggedIn.value = true
    setStoredUser(profile.value)
    setStoredToken(userToken)
  }

  function logout() {
    profile.value = {}
    token.value = ''
    isLoggedIn.value = false
    roles.value = []
    permissions.value = []
    recentProjects.value = []
    setStoredUser(null)
    setStoredToken('')
  }

  function updatePreferences(prefs) {
    preferences.value = { ...preferences.value, ...prefs }
  }

  function addRecentProject(project) {
    const list = recentProjects.value.filter(p => p.id !== project.id)
    list.unshift({ id: project.id, name: project.name, lastVisit: Date.now() })
    // 最多保留 10 个
    recentProjects.value = list.slice(0, 10)
  }

  function clearRecentProjects() {
    recentProjects.value = []
  }

  async function fetchUserInfo() {
    loading.value = true
    try {
      // TODO: 调用真实 API 获取用户信息
      // const data = await api.getUserInfo()
      // profile.value = data
      return profile.value
    } catch (e) {
      console.error('获取用户信息失败:', e)
      throw e
    } finally {
      loading.value = false
    }
  }

  return {
    // State
    profile,
    token,
    isLoggedIn,
    roles,
    permissions,
    preferences,
    recentProjects,
    loading,
    // Getters
    displayName,
    initials,
    isAdmin,
    isDeveloper,
    avatarUrl,
    // Methods
    hasPermission,
    hasAnyPermission,
    hasRole,
    setProfile,
    setToken,
    login,
    logout,
    updatePreferences,
    addRecentProject,
    clearRecentProjects,
    fetchUserInfo,
  }
})
