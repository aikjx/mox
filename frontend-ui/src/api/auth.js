/**
 * MOX 认证 API 模块
 * 统一封装登录、注册、刷新令牌、登出等认证相关接口
 */

// 认证请求必须复用统一 HTTP 实例：鉴权头、租户/项目上下文、响应信封和错误归一化
// 都由这里集中处理，避免维护第二套已经不存在的 request 工具。
import request from './http'

/**
 * 登录
 * @param {Object} data - 登录数据
 * @param {string} data.username - 用户名
 * @param {string} data.password - 密码
 * @param {string} [data.tenant_id] - 租户ID
 * @returns {Promise<Object>} 登录响应（access_token, refresh_token, user）
 */
export function login(data) {
  return request({
    url: '/auth/login',
    method: 'post',
    data
  })
}

/**
 * 刷新访问令牌
 * @param {string} refreshToken - 刷新令牌
 * @returns {Promise<Object>} 新的令牌对
 */
export function refreshToken(refreshToken) {
  return request({
    url: '/auth/refresh',
    method: 'post',
    data: { refresh_token: refreshToken }
  })
}

/**
 * 获取当前用户信息
 * @returns {Promise<Object>} 用户信息
 */
export function getCurrentUser(token) {
  return request({
    url: '/auth/me',
    method: 'get',
    headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    _retry: 0,
    silent: true
  })
}

/**
 * 注册新用户
 * @param {Object} data - 注册数据
 * @param {string} data.username - 用户名
 * @param {string} data.email - 邮箱
 * @param {string} data.password - 密码
 * @param {string} [data.tenant_id] - 租户ID
 * @returns {Promise<Object>} 注册响应
 */
export function register(data) {
  return request({
    url: '/auth/register',
    method: 'post',
    data
  })
}

/**
 * 发送重置密码邮件
 * @param {string} email - 邮箱地址
 * @returns {Promise<Object>} 发送结果
 */
export function forgotPassword(email) {
  return request({
    url: '/auth/forgot-password',
    method: 'post',
    data: { email }
  })
}

/**
 * 重置密码
 * @param {Object} data - 重置数据
 * @param {string} data.token - 重置令牌
 * @param {string} data.new_password - 新密码
 * @returns {Promise<Object>} 重置结果
 */
export function resetPassword(data) {
  return request({
    url: '/auth/reset-password',
    method: 'post',
    data
  })
}

/**
 * 登出（前端清除令牌，后端可加入黑名单）
 * @returns {Promise<Object>} 登出结果
 */
export function logout() {
  return request({
    url: '/auth/logout',
    method: 'post'
  })
}

/**
 * 验证用户名是否已存在
 * @param {string} username - 用户名
 * @returns {Promise<Object>} 验证结果
 */
export function checkUsername(username) {
  return request({
    url: `/auth/check-username/${username}`,
    method: 'get'
  })
}

export default {
  login,
  refreshToken,
  getCurrentUser,
  register,
  forgotPassword,
  resetPassword,
  logout,
  checkUsername
}
