// 响应信封归一层：网关两条 handler 族使用不同嵌套，UI 侧只允许通过本模块取 payload。
// flat  : {code,msg,data}                                   —— experts_* / kg 等族
// nested : {code,msg,data:{elapsed_ms,params,data:{…}}}     —— alliance.rs 全部 handler

export class ApiError extends Error {
  constructor(message, { code = null, status = null, params = null, msg = '' } = {}) {
    super(message)
    this.name = 'ApiError'
    this.code = code
    this.status = status
    this.params = params
    this.msg = msg
  }
}

function isRecord(v) {
  return !!v && typeof v === 'object' && !Array.isArray(v)
}

function isNestedShell(v) {
  return isRecord(v) && 'data' in v && ('params' in v || 'elapsed_ms' in v)
}

/**
 * 接受 axios 响应或已解析 body，统一返回 ApiResponse body。
 */
export function toBody(res) {
  if (isRecord(res) && isRecord(res.data) && 'code' in res.data && 'msg' in res.data) return res.data
  if (isRecord(res) && typeof res.status === 'number' && 'data' in res) return isRecord(res.data) ? res.data : { code: 0, msg: 'ok', data: res.data }
  return res
}

/**
 * 剥离信封取出业务 payload。对「已被 src/api/http.js 拦截器解过一层」的输入同样成立，
 * 即幂等：重复调用不会多剥。
 * @param {object} res axios 响应或 body
 * @param {{nesting?: 'auto'|'flat'|'nested'}} [opts] 'flat' 声明该族无内层壳
 * @returns {any} payload，缺省为 null
 */
export function unwrap(res, opts = {}) {
  const body = toBody(res)
  if (isRecord(body) && typeof body.code === 'number' && body.code !== 0) {
    throw new ApiError(body.msg || '接口返回错误', {
      code: body.code,
      msg: body.msg || '',
      params: isRecord(body.data) ? body.data.params ?? null : null
    })
  }

  let payload = body
  // 外层 {code,msg,data}：只在确认为信封时剥离，避免把业务字段 data 误当作壳
  if (isRecord(payload) && 'data' in payload && ('code' in payload || 'msg' in payload)) payload = payload.data
  const nesting = opts.nesting ?? 'auto'
  if (nesting !== 'flat' && isNestedShell(payload)) payload = payload.data
  return payload ?? null
}

/**
 * 取信封附带信息（耗时与回显参数），用于可观测性面板，不抛错。
 */
export function envelopeMeta(res) {
  const body = toBody(res)
  if (!isRecord(body)) return { elapsedMs: null, params: null, code: null, msg: '' }
  const shell = isNestedShell(body) ? body : (isRecord(body.data) && isNestedShell(body.data) ? body.data : null)
  return {
    elapsedMs: shell ? shell.elapsed_ms ?? null : null,
    params: shell ? shell.params ?? null : null,
    code: body.code ?? null,
    msg: body.msg ?? ''
  }
}

/**
 * 列表字段兜底：后端各族列表键名不统一（tasks/experts/sessions/nodes/logs）。
 */
export function unwrapList(payload, key) {
  if (Array.isArray(payload)) return payload
  if (!isRecord(payload)) return []
  if (Array.isArray(payload[key])) return payload[key]
  const candidates = ['items', 'list', 'records', 'tasks', 'experts', 'sessions', 'nodes', 'logs', 'data']
  for (const k of candidates) if (Array.isArray(payload[k])) return payload[k]
  return []
}
