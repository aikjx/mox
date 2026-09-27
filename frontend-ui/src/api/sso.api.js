/**
 * 企业 SSO API —— 嵌套挂载 /api/enterprise/sso
 *
 * 信封由 http.js 统一解包：成功 resolve 为 data 本体，失败 reject(带 [code] 前缀)。
 * 契约来源：platform/gateway/mox-platform-gateway-svc/src/sso/{api,mod}.rs
 *
 * 重要边界：POST /callback 当前固定返回 501 NOT_IMPLEMENTED
 * （外部身份源授权码交换未实现，且有 Rust 单测 external_callback_never_creates_a_mock_session 守护）。
 * 前端登录流程仅负责"发起跳转"（POST /login 换 auth_url），不伪造平台登录态。
 */
import http from './http'

const BASE = '/enterprise/sso'

/** GET /protocols → [{code,name,description}] */
export const getSsoProtocols = () => http.get(`${BASE}/protocols`)

/** GET /providers?status=&protocol= → [SsoProvider]（按 sort_order 升序） */
export const getSsoProviders = (params) => http.get(`${BASE}/providers`, { params })

/** GET /providers/:id → SsoProvider */
export const getSsoProvider = (id) => http.get(`${BASE}/providers/${id}`)

/** POST /providers → {provider_id}（创建后 status 强制 disabled） */
export const createSsoProvider = (payload) => http.post(`${BASE}/providers`, payload)

/** PUT /providers/:id（部分更新） */
export const updateSsoProvider = (id, payload) => http.put(`${BASE}/providers/${id}`, payload)

/** DELETE /providers/:id */
export const deleteSsoProvider = (id) => http.delete(`${BASE}/providers/${id}`)

/** POST /login {provider_id,redirect_uri?,state?} → {auth_url,state} */
export const ssoLogin = (payload) => http.post(`${BASE}/login`, payload)

/** POST /callback {provider_id,code,state} —— 当前 501，仅保留契约 */
export const ssoCallback = (payload) => http.post(`${BASE}/callback`, payload)

/** POST /logout {session_id} */
export const ssoLogout = (payload) => http.post(`${BASE}/logout`, payload)
