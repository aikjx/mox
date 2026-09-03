/**
 * 认证 Store - auth.store.js
 *
 * 企业级认证状态管理：
 * - state: token, userInfo, permissions, loginMode
 * - actions: login, logout, refreshToken, checkAuth, setToken
 * - getters: isLoggedIn, userRoles
 *
 * 支持两种登录模式：jwt / oauth2
 * 所有 token 操作通过 secureStorage 安全存储
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  setToken as secureSetToken,
  getToken as secureGetToken,
  removeToken as secureRemoveToken,
  hasValidToken,
  secureSetItem,
  secureGetItem,
  secureRemoveItem,
} from '@/utils/secureStorage'
import http from '@/api/http'

// ===== 常量 =====

const USER_INFO_KEY = 'mox-user-info'
const PERMISSIONS_KEY = 'mox-permissions'
const LOGIN_MODE_KEY = 'mox-login-mode'

// 支持的登录模式
export const LOGIN_MODES = {
  JWT: 'jwt',
  OAUTH2: 'oauth2',
}

// ===== 辅助函数 =====

function _getStoredLoginMode() {
  // 优先级：localStorage > 环境变量 > 默认 jwt
  const stored = secureGetItem(LOGIN_MODE_KEY, { tryLegacy: true })
  if (stored && Object.values(LOGIN_MODES).includes(stored)) {
    return stored
  }
  const envMode =
    typeof import.meta !== 'undefined' &&
    import.meta.env &&
    import.meta.env.VITE_LOGIN_MODE
  if (envMode && Object.values(LOGIN_MODES).includes(envMode)) {
    return envMode
  }
  return LOGIN_MODES.JWT
}

function _getStoredUserInfo() {
  const raw = secureGetItem(USER_INFO_KEY, { tryLegacy: true })
  if (!raw) return null
  try {
    // 如果是 JSON 字符串则解析
    if (typeof raw === 'string' && (raw.startsWith('{') || raw.startsWith('['))) {
      return JSON.parse(raw)
    }
    // 向后兼容：旧版 mox-user 存储
    if (typeof raw === 'object') return raw
    return { username: raw }
  } catch {
    return { username: raw }
  }
}

function _getStoredPermissions() {
  const raw = secureGetItem(PERMISSIONS_KEY, { tryLegacy: true })
  if (!raw) return []
  try {
    if (typeof raw === 'string') {
      return JSON.parse(raw)
    }
    if (Array.isArray(raw)) return raw
    return []
  } catch {
    return []
  }
}

// ===== Store 定义 =====

export const useAuthStore = defineStore('auth', () => {
  // ===== State =====

  // 认证令牌
  const token = ref(secureGetToken() || '')

  // 用户信息
  const userInfo = ref(_getStoredUserInfo() || {
    id: '',
    username: '',
    nickname: '',
    email: '',
    avatar: '',
  })

  // 权限列表
  const permissions = ref(_getStoredPermissions())

  // 登录模式：jwt / oauth2
  const loginMode = ref(_getStoredLoginMode())

  // 加载状态
  const loading = ref(false)

  // 认证检查状态（是否已完成初始检查）
  const initialized = ref(false)

  // ===== Getters =====

  /** 是否已登录 */
  const isLoggedIn = computed(() => {
    // 优先基于 token 判断
    if (token.value) return true
    // 检查存储中是否有有效 token
    return hasValidToken()
  })

  /** 用户角色列表（从 userInfo.roles 或 permissions 中提取） */
  const userRoles = computed(() => {
    if (userInfo.value && Array.isArray(userInfo.value.roles)) {
      return userInfo.value.roles
    }
    // 从 permissions 中推断角色（简单映射）
    const perms = permissions.value
    const roles = []
    if (perms.some(p => p.startsWith('admin:'))) roles.push('admin')
    if (perms.some(p => p.startsWith('project:'))) roles.push('developer')
    return roles
  })

  /** 显示名称 */
  const displayName = computed(() => {
    return userInfo.value?.nickname || userInfo.value?.username || '未登录用户'
  })

  /** 是否为管理员 */
  const isAdmin = computed(() => userRoles.value.includes('admin'))

  // ===== Actions =====

  /**
   * 设置 token（统一入口）
   * @param {string} newToken 新 token
   * @param {number} expiresIn 有效期（秒），默认 24 小时
   */
  function setToken(newToken, expiresIn = 86400) {
    token.value = newToken
    if (newToken) {
      secureSetToken(newToken, expiresIn)
    } else {
      secureRemoveToken()
    }
  }

  /**
   * 设置登录模式
   * @param {string} mode 登录模式
   */
  function setLoginMode(mode) {
    if (Object.values(LOGIN_MODES).includes(mode)) {
      loginMode.value = mode
      secureSetItem(LOGIN_MODE_KEY, mode)
    }
  }

  /**
   * 设置用户信息
   * @param {object} info 用户信息
   */
  function setUserInfo(info) {
    userInfo.value = { ...userInfo.value, ...info }
    try {
      secureSetItem(USER_INFO_KEY, JSON.stringify(userInfo.value))
    } catch (e) {
      console.warn('[authStore] 存储用户信息失败:', e)
    }
  }

  /**
   * 设置权限列表
   * @param {string[]} perms 权限列表
   */
  function setPermissions(perms) {
    permissions.value = Array.isArray(perms) ? perms : []
    try {
      secureSetItem(PERMISSIONS_KEY, JSON.stringify(permissions.value))
    } catch (e) {
      console.warn('[authStore] 存储权限失败:', e)
    }
  }

  /**
   * 检查是否有指定权限
   * @param {string} perm 权限标识
   * @returns {boolean}
   */
  function hasPermission(perm) {
    if (isAdmin.value) return true
    return permissions.value.includes(perm)
  }

  /**
   * 检查是否有任一权限
   * @param {string[]} perms 权限标识列表
   * @returns {boolean}
   */
  function hasAnyPermission(perms) {
    if (isAdmin.value) return true
    return perms.some(p => permissions.value.includes(p))
  }

  /**
   * 检查是否有指定角色
   * @param {string} role 角色名
   * @returns {boolean}
   */
  function hasRole(role) {
    return userRoles.value.includes(role)
  }

  /**
   * 登录（统一入口，根据模式分派）
   * @param {object} credentials 登录凭证
   * @param {string} credentials.username 用户名
   * @param {string} credentials.password 密码
   * @param {string} [mode] 登录模式，默认使用当前 loginMode
   * @returns {Promise<{ token: string, userInfo: object }>}
   */
  async function login(credentials, mode) {
    const useMode = mode || loginMode.value
    loading.value = true

    try {
      let result

      switch (useMode) {
        case LOGIN_MODES.JWT:
          result = await _jwtLogin(credentials)
          break
        case LOGIN_MODES.OAUTH2:
          result = await _oauth2Login(credentials)
          break
        default:
          throw new Error(`不支持的登录模式: ${useMode}`)
      }

      // 统一处理登录结果
      if (result && result.token) {
        setToken(result.token, result.expiresIn || 86400)
        if (result.userInfo) {
          setUserInfo(result.userInfo)
        }
        if (result.permissions) {
          setPermissions(result.permissions)
        }
        loginMode.value = useMode
        secureSetItem(LOGIN_MODE_KEY, useMode)
      }

      return result
    } finally {
      loading.value = false
    }
  }

  /**
   * JWT 模式登录 - 调用真实 API
   */
  async function _jwtLogin(credentials) {
    try {
      const response = await http.post('/auth/login', {
        username: credentials.username,
        password: credentials.password,
      })

      // 兼容不同的响应格式
      const data = response || {}
      const accessToken =
        data.access_token ||
        data.accessToken ||
        data.token ||
        ''
      const expiresIn = data.expires_in || data.expiresIn || 86400

      return {
        token: accessToken,
        expiresIn,
        userInfo: data.user || data.userInfo || {
          username: credentials.username,
        },
        permissions: data.permissions || data.roles || [],
      }
    } catch (e) {
      // 错误已在 http 拦截器中处理，这里重新抛出供上层处理
      throw e
    }
  }

  /**
   * OAuth2 模式登录 - SSO 框架
   * 实际使用时，通常是跳转到授权服务器，然后在回调页处理
   */
  async function _oauth2Login(credentials) {
    // OAuth2 标准流程通常是：
    // 1. 前端跳转到授权服务器
    // 2. 用户登录授权
    // 3. 授权服务器重定向回回调页，携带 code
    // 4. 前端用 code 换取 token
    //
    // 这里提供框架代码，具体实现根据 OAuth 提供商调整

    if (credentials?.code) {
      // 授权码模式：用 code 换 token
      try {
        const response = await http.post('/auth/oauth2/token', {
          code: credentials.code,
          redirect_uri: credentials.redirectUri || window.location.origin + '/oauth2/callback',
        })
        return {
          token: response.access_token || response.token,
          expiresIn: response.expires_in || 3600,
          userInfo: response.user || {},
          permissions: response.permissions || [],
        }
      } catch (e) {
        throw e
      }
    }

    // 如果没有 code，抛出需要跳转的信号
    const authUrl =
      (typeof import.meta !== 'undefined' &&
        import.meta.env &&
        import.meta.env.VITE_OAUTH2_AUTH_URL) ||
      '/oauth2/authorize'

    const error = new Error('OAUTH2_REDIRECT_REQUIRED')
    error.authUrl = authUrl
    error.code = 'OAUTH2_REDIRECT_REQUIRED'
    throw error
  }

  /**
   * 登出
   * @param {object} options
   * @param {boolean} options.callApi 是否调用后端登出 API
   */
  async function logout(options = {}) {
    const { callApi = false } = options

    try {
      if (callApi && token.value) {
        // 尝试调用后端登出 API（不阻塞，失败也继续本地登出）
        http.post('/auth/logout').catch(() => {})
      }
    } finally {
      // 清除本地状态
      token.value = ''
      userInfo.value = {
        id: '',
        username: '',
        nickname: '',
        email: '',
        avatar: '',
      }
      permissions.value = []
      initialized.value = false

      // 清除安全存储
      secureRemoveToken()
      secureRemoveItem(USER_INFO_KEY)
      secureRemoveItem(PERMISSIONS_KEY)
      // 注意：不清除 loginMode，保留用户选择
    }
  }

  /**
   * 刷新 Token
   * @returns {Promise<string>} 新 token
   */
  async function refreshToken() {
    try {
      const response = await http.post('/auth/refresh')
      const newToken = response.access_token || response.token || ''

      if (newToken) {
        setToken(newToken, response.expires_in || 86400)
      }

      return newToken
    } catch (e) {
      console.warn('[authStore] 刷新 token 失败:', e)
      // 刷新失败，执行登出
      await logout()
      throw e
    }
  }

  /**
   * 检查认证状态（初始化时调用）
   * @returns {boolean} 是否已认证
   */
  async function checkAuth() {
    try {
      // 1. 检查本地 token 是否存在且有效
      const storedToken = secureGetToken()
      if (!storedToken) {
        initialized.value = true
        return false
      }

      // 同步到 state
      token.value = storedToken

      // 2. 尝试调用后端验证接口（可选，静默失败）
      try {
        const userData = await http.get('/auth/me')
        if (userData) {
          setUserInfo(userData.user || userData)
          if (userData.permissions) {
            setPermissions(userData.permissions)
          }
        }
      } catch (e) {
        // 后端验证失败，不立即登出，可能是网络问题
        console.debug('[authStore] 后端认证校验失败:', e?.message)
      }

      initialized.value = true
      return true
    } catch (e) {
      console.error('[authStore] checkAuth 异常:', e)
      initialized.value = true
      return false
    }
  }

  // ===== 返回 =====

  return {
    // State
    token,
    userInfo,
    permissions,
    loginMode,
    loading,
    initialized,
    // Getters
    isLoggedIn,
    userRoles,
    displayName,
    isAdmin,
    // Actions
    setToken,
    setLoginMode,
    setUserInfo,
    setPermissions,
    hasPermission,
    hasAnyPermission,
    hasRole,
    login,
    logout,
    refreshToken,
    checkAuth,
  }
})
