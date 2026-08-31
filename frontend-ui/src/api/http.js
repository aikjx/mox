// HTTP 核心实例 - axios 配置、拦截器、项目ID注入、重试机制
import axios from 'axios'
import { ElMessage } from 'element-plus'
import { getToken } from '@/utils/secureStorage'

// ========== 配置常量 ==========
const DEFAULT_RETRY_COUNT = 2       // 网络错误/5xx 默认重试次数
const DEFAULT_RETRY_DELAY = 500     // 重试间隔（ms），指数退避基数
const RETRYABLE_STATUS = [502, 503, 504]  // 可重试的服务端状态码

// 后端运行时地址：开发环境走 Vite 代理，生产环境直连
const http = axios.create({
  baseURL: '/api',
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' }
})

// ========== 工具函数 ==========

/**
 * 判断错误是否可重试
 * @param {Error} err axios 错误对象
 * @returns {boolean}
 */
function isRetryableError(err) {
  // 网络错误 / 超时 / 连接被拒
  if (!err.response) return true
  if (err.code === 'ECONNABORTED') return true
  // 特定 5xx 服务端错误
  const status = err.response.status
  return RETRYABLE_STATUS.includes(status)
}

/**
 * 指数退避延迟
 * @param {number} attempt 当前尝试次数（从0开始）
 * @param {number} baseDelay 基础延迟
 */
function exponentialBackoff(attempt, baseDelay = DEFAULT_RETRY_DELAY) {
  const delay = baseDelay * Math.pow(2, attempt)
  // 增加 0~50% 随机抖动，避免惊群效应
  const jitter = delay * (Math.random() * 0.5)
  return new Promise(resolve => setTimeout(resolve, delay + jitter))
}

// ========== 响应拦截器 ==========

// 统一响应处理：剥离 axios 包裹，自动解包 {success, data} 格式
http.interceptors.response.use(
  (resp) => {
    const body = resp.data
    if (body && typeof body === 'object' && 'success' in body) {
      // {success:false}: 失败路径统一带 code 前缀，便于诊断
      if (!body.success) {
        const code = body.code ? `[${body.code}] ` : ''
        return Promise.reject(new Error(code + (body.error || body.message || body.detail || '请求失败')))
      }
      // 标准信封 { success, data }: 返回 data 本体
      if ('data' in body) return body.data
      // 兼容 { success: true, ...rest }: 返回整包（保留 latency/provider/message 等额外字段）
      return body
    }
    return body
  },
  async (err) => {
    const config = err.config || {}

    // ===== 自动重试机制 =====
    const retryCount = config._retry ?? DEFAULT_RETRY_COUNT
    const currentAttempt = config._attempt ?? 0

    if (currentAttempt < retryCount && isRetryableError(err)) {
      config._attempt = currentAttempt + 1
      config._retry = retryCount

      // 幂等请求（GET/HEAD/OPTIONS）自动重试，非幂等需要显式配置
      const method = String(config.method || 'get').toLowerCase()
      const isIdempotent = ['get', 'head', 'options'].includes(method)

      if (isIdempotent || config._retryOnPost) {
        await exponentialBackoff(currentAttempt)
        // 清除可能存在的取消令牌，重新创建
        delete config.cancelToken
        return http.request(config)
      }
    }

    // ===== 错误消息处理 =====
    const status = err.response && err.response.status
    const data = err.response && err.response.data
    const extractMsg = (d) => {
      if (!d) return ''
      if (typeof d === 'string') return d
      return d.error || d.message || d.detail || d.title || (typeof d.data === 'string' ? d.data : '')
    }
    let msg = extractMsg(data) || err.message || '网络请求失败'
    const codePrefix =
      data && typeof data === 'object' && data.code ? `[${data.code}] ` : ''
    if (codePrefix && msg.indexOf(codePrefix) !== 0) msg = codePrefix + msg

    if (status === 401) {
      msg = '鉴权失败：请检查 API 令牌（OUS_API_TOKEN）是否已配置'
      ElMessage.error(msg)
      // 触发全局登出事件，由应用层处理
      window.dispatchEvent(new CustomEvent('mox:auth-failed', { detail: { reason: '401' } }))
    } else if (status === 503) {
      msg = '服务暂不可用：后端未启动或正在重启，请稍后重试'
      ElMessage.error(msg)
    } else if (err.code === 'ECONNABORTED') {
      msg = '请求超时：后端处理较慢，请稍后重试'
      ElMessage.warning(msg)
    } else if (status === 400 || status === 404 || status === 409) {
      ElMessage.warning(msg)
    } else if (status && status >= 500) {
      ElMessage.error('服务端异常：' + msg)
    } else if (!err.response) {
      msg = '无法连接后端：请确认服务已启动'
      ElMessage.error(msg)
    }

    // 携带原始错误信息，方便调试
    const error = new Error(msg)
    error.status = status
    error.code = data?.code
    error.original = err
    return Promise.reject(error)
  }
)

// ========== 全局项目注入 ==========
// 璇玑：所有请求自动带上当前 project_id，后端忽略未知参数即安全
let _projectIdGetter = null
/** 给 projectContext 用：在 setCurrentProject 后把 id 暴露给请求层 */
export function registerProjectIdGetter(getter) {
  _projectIdGetter = typeof getter === 'function' ? getter : null
}

function injectProjectToConfig(config) {
  if (!_projectIdGetter) return config
  let pid
  try { pid = _projectIdGetter() } catch {}
  if (!pid) return config
  // GET/HEAD: params；其他：body；不覆盖显式传值
  const method = String(config.method || 'get').toLowerCase()
  if (method === 'get' || method === 'head') {
    config.params = config.params ? { ...config.params } : {}
    if (config.params.project_id == null && config.params.projectId == null) {
      config.params.project_id = pid
    }
  } else {
    const isForm =
      config.headers && (config.headers['Content-Type'] || '').indexOf('multipart') >= 0
    if (!isForm && config.data && typeof config.data === 'object' && !Array.isArray(config.data)) {
      if (config.data.project_id == null && config.data.projectId == null) {
        // 拷贝一层避免修改外部引用
        config.data = { ...config.data, project_id: pid }
      }
    } else if (config.data == null) {
      config.data = { project_id: pid }
    }
  }
  return config
}

// ========== 请求拦截器 ==========

// 安全策略：生产环境禁用默认令牌，仅开发环境允许配置回退
const isProd = typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.PROD

http.interceptors.request.use((config) => {
  // 优先从安全存储读取（自动兼容旧版 localStorage key）
  const token =
    getToken() ||
    (typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.VITE_API_TOKEN) ||
    (typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.VITE_OUS_API_TOKEN) ||
    // 仅开发环境允许使用默认令牌，生产环境必须显式配置
    (isProd ? '' : 'dev-secret-token')

  config.headers = config.headers || {}
  if (token) config.headers['Authorization'] = 'Bearer ' + token

  // 兜底：未显式设置 Content-Type 时默认 JSON
  if (!config.headers['Content-Type'] && config.method && String(config.method).toLowerCase() !== 'get') {
    config.headers['Content-Type'] = 'application/json'
  }

  // 注入请求追踪 ID，便于链路追踪
  config.headers['X-Request-Id'] =
    (typeof crypto !== 'undefined' && crypto.randomUUID?.()) ||
    `req_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`

  injectProjectToConfig(config)

  return config
})

// ========== 便捷方法扩展 ==========

/**
 * 带取消令牌的请求包装
 * @returns {{ request: Promise, cancel: Function }}
 */
export function withCancel(fn) {
  const controller = new AbortController()
  const request = fn(controller.signal)
  return {
    request,
    cancel: (reason = 'canceled') => controller.abort(reason)
  }
}

/**
 * 批量并行请求（带并发控制）
 * @param {Array<Function>} tasks 任务函数数组
 * @param {number} concurrency 并发数
 */
export async function batchRequest(tasks, concurrency = 3) {
  const results = []
  let index = 0

  async function worker() {
    while (index < tasks.length) {
      const current = index++
      try {
        results[current] = await tasks[current]()
      } catch (e) {
        results[current] = { error: e }
      }
    }
  }

  const workers = Array.from(
    { length: Math.min(concurrency, tasks.length) },
    () => worker()
  )
  await Promise.all(workers)
  return results
}

export default http
