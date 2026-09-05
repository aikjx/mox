/**
 * MOX 认证 Store
 * 管理用户登录状态、令牌、用户信息，提供登录/登出/刷新等操作
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import authApi from '../api/auth'
import { registerAuthTokenGetter } from '../api/http'
import { getToken, removeToken } from '../utils/secureStorage'

// 本地存储键名
const TOKEN_KEY = 'mox_access_token'
const REFRESH_TOKEN_KEY = 'mox_refresh_token'
const USER_KEY = 'mox_user_info'

export const useAuthStore = defineStore('auth', () => {
  // ── 状态 ──────────────────────────────────────────────────────────────
  const accessToken = ref(localStorage.getItem(TOKEN_KEY) || getToken() || '')
  const refreshToken = ref(localStorage.getItem(REFRESH_TOKEN_KEY) || '')
  const userInfo = ref(JSON.parse(localStorage.getItem(USER_KEY) || 'null'))
  const loading = ref(false)
  const error = ref('')
  registerAuthTokenGetter(() => accessToken.value)

  // ── 计算属性 ──────────────────────────────────────────────────────────
  const isLoggedIn = computed(() => !!accessToken.value)
  const username = computed(() => userInfo.value?.username || '')
  const userId = computed(() => userInfo.value?.id || '')
  const tenantId = computed(() => userInfo.value?.tenant_id || 'default')
  const roles = computed(() => userInfo.value?.roles || [])
  const isAdmin = computed(() => roles.value.includes('admin'))

  // ── 方法 ──────────────────────────────────────────────────────────────
  // 已有令牌须通过受保护接口验证；仅保存在当前页面内存，刷新后重新连接。
  async function loginWithToken(token) {
    if (loading.value) return
    loading.value = true
    error.value = ''
    try {
      const value = token.trim()
      if (!value) throw new Error('请输入访问令牌')
      let user = await authApi.getCurrentUser(value)
      for (let i = 0; i < 4 && user?.data; i++) user = user.data
      if (!user?.id || !user.enabled) throw new Error('当前身份不可用')
      clearAuth()
      accessToken.value = value
      userInfo.value = user
      return user
    } catch (err) {
      error.value = err.message || '令牌验证失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * 登录
   * @param {string} username - 用户名
   * @param {string} password - 密码
   * @param {string} [tenantId] - 租户ID
   */
  async function login(username, password, tenantId = 'default') {
    loading.value = true
    error.value = ''

    try {
      const response = await authApi.login({
        username,
        password,
        tenant_id: tenantId
      })

      // 保存令牌
      accessToken.value = response.access_token
      refreshToken.value = response.refresh_token
      userInfo.value = response.user

      localStorage.setItem(TOKEN_KEY, response.access_token)
      localStorage.setItem(REFRESH_TOKEN_KEY, response.refresh_token)
      localStorage.setItem(USER_KEY, JSON.stringify(response.user))

      return response
    } catch (err) {
      error.value = err.message || '登录失败'
      throw err
    } finally {
      loading.value = false
    }
  }

  /**
   * 刷新访问令牌
   */
  async function refreshAccessToken() {
    if (!refreshToken.value) {
      throw new Error('无刷新令牌')
    }

    try {
      const response = await authApi.refreshToken(refreshToken.value)

      accessToken.value = response.access_token
      if (response.refresh_token) {
        refreshToken.value = response.refresh_token
        localStorage.setItem(REFRESH_TOKEN_KEY, response.refresh_token)
      }

      localStorage.setItem(TOKEN_KEY, response.access_token)

      return response
    } catch (err) {
      // 刷新失败，清除登录状态
      clearAuth()
      throw err
    }
  }

  /**
   * 获取当前用户信息
   */
  async function fetchCurrentUser() {
    try {
      const user = await authApi.getCurrentUser()
      userInfo.value = user
      localStorage.setItem(USER_KEY, JSON.stringify(user))
      return user
    } catch (err) {
      throw err
    }
  }

  /**
   * 登出
   */
  async function logout() {
    try {
      await authApi.logout()
    } catch (err) {
      // 忽略登出API错误，仍然清除本地状态
      console.warn('登出API调用失败:', err.message)
    } finally {
      clearAuth()
    }
  }

  /**
   * 清除认证状态
   */
  function clearAuth() {
    accessToken.value = ''
    refreshToken.value = ''
    userInfo.value = null
    error.value = ''

    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(REFRESH_TOKEN_KEY)
    localStorage.removeItem(USER_KEY)
    removeToken()
  }

  /**
   * 检查是否有权限
   * @param {string} role - 角色名
   */
  function hasRole(role) {
    return roles.value.includes(role)
  }

  /**
   * 检查是否有任一权限
   * @param {string[]} roleList - 角色列表
   */
  function hasAnyRole(roleList) {
    return roleList.some(role => roles.value.includes(role))
  }

  /**
   * 获取认证头
   */
  function getAuthHeader() {
    return accessToken.value ? { Authorization: `Bearer ${accessToken.value}` } : {}
  }

  return {
    // 状态
    accessToken,
    refreshToken,
    userInfo,
    loading,
    error,
    // 计算属性
    isLoggedIn,
    username,
    userId,
    tenantId,
    roles,
    isAdmin,
    // 方法
    login,
    loginWithToken,
    refreshAccessToken,
    fetchCurrentUser,
    logout,
    clearAuth,
    hasRole,
    hasAnyRole,
    getAuthHeader
  }
})
