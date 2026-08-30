// HTTP 核心实例 - axios 配置、拦截器、项目ID注入
import axios from 'axios'
import { ElMessage } from 'element-plus'

// 后端运行时地址：开发环境走 Vite 代理，生产环境直连
const http = axios.create({
  baseURL: '/api',
  timeout: 30000,
  headers: { 'Content-Type': 'application/json' }
})

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
  (err) => {
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
    if (status === 401 || status === 503) {
      msg =
        status === 401
          ? '鉴权失败：请检查 API 令牌（OUS_API_TOKEN）是否已配置'
          : '服务暂不可用：后端未启动或正在重启，请稍后重试'
      ElMessage.error(msg)
    } else if (err.code === 'ECONNABORTED') {
      msg = '请求超时：后端处理较慢，请稍后重试'
      ElMessage.warning(msg)
    } else if (status === 400 || status === 404 || status === 409) {
      ElMessage.warning(msg)
    } else if (status && status >= 500) {
      ElMessage.error('服务端异常：' + msg)
    } else if (!err.response) {
      msg = '无法连接后端：请确认服务已启动（http://localhost:3010）'
      ElMessage.error(msg)
    }
    return Promise.reject(new Error(msg))
  }
)

// 全局项目注入（璇玑：所有请求自动带上当前 project_id，后端忽略未知参数即安全）
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

// 请求拦截：自动注入 API 令牌（兼容后端 OUS_API_TOKEN 鉴权）
// 开发期默认令牌可由 .env 的 VITE_API_TOKEN 覆盖，否则回退本地存储 / 开发令牌
const DEV_TOKEN = 'dev-secret-token'
http.interceptors.request.use((config) => {
  const token =
    (typeof localStorage !== 'undefined' &&
      (localStorage.getItem('ous_api_token') || localStorage.getItem('ous_token'))) ||
    (typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.VITE_API_TOKEN) ||
    DEV_TOKEN
  config.headers = config.headers || {}
  if (token) config.headers['Authorization'] = 'Bearer ' + token
  // 兜底：未显式设置 Content-Type 时默认 JSON
  if (!config.headers['Content-Type'] && config.method && String(config.method).toLowerCase() !== 'get') {
    config.headers['Content-Type'] = 'application/json'
  }
  injectProjectToConfig(config)
  return config
})

export default http
