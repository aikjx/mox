// 前端模块内核：模块声明的唯一入口。业务模块必须经 defineModule 注册，
// 以便外壳、导航、路由三处从同一份描述派生（取代 constants/nav.config.js 的手工副本）。

const registry = new Map()
// name → 声明它的源文件。用于区分「同一文件被 Vite HMR 重新执行」与「两个文件抢占同一模块名」。
const owners = new Map()

const REQUIRED_ROUTE_META = ['title', 'module', 'layout']
const ALLOWED_LAYOUTS = ['default', 'blank']

// dev 下 import.meta.url 带 ?t=时间戳，去掉查询才是稳定的源身份
const sourceOf = (url) => (typeof url === 'string' ? url.split('?')[0] : null)

function assertRoute(name, route, idx) {
  const where = `模块 ${name} 路由[${idx}] ${route?.path ?? '?'}`
  if (!route || typeof route.path !== 'string') throw new Error(`${where}: 缺少 path`)
  if (typeof route.component !== 'function' && typeof route.component !== 'object') throw new Error(`${where}: component 必须是懒加载函数或组件对象`)
  if (!route.meta) throw new Error(`${where}: 缺少 meta`)
  for (const key of REQUIRED_ROUTE_META) {
    if (route.meta[key] === undefined) throw new Error(`${where}: 缺少 meta.${key}`)
  }
  if (!ALLOWED_LAYOUTS.includes(route.meta.layout)) throw new Error(`${where}: meta.layout 只能是 ${ALLOWED_LAYOUTS.join(' | ')}`)
}

/**
 * 声明并登记一个前端模块。
 * @param {{name:string, title:string, version:string, routes?:Array, nav?:Array, endpoints?:Object}} descriptor
 * @param {string} [selfUrl] 调用方的 import.meta.url；同一源文件重复声明按热更新替换处理
 */
export function defineModule(descriptor, selfUrl) {
  const { name, title, version } = descriptor
  if (!name || !title || !version) throw new Error('模块描述必须包含 name / title / version')
  const source = sourceOf(selfUrl)
  if (registry.has(name) && !(source && source === owners.get(name))) throw new Error(`模块重复注册: ${name}`)

  const routes = descriptor.routes ?? []
  routes.forEach((r, i) => assertRoute(name, r, i))

  const frozen = Object.freeze({
    ...descriptor,
    routes: Object.freeze(routes.map((r) => Object.freeze({ ...r }))),
    nav: Object.freeze(descriptor.nav ?? []),
    endpoints: Object.freeze(descriptor.endpoints ?? {})
  })
  registry.set(name, frozen)
  owners.set(name, source)
  return frozen
}

export function listModules() {
  return [...registry.values()]
}

export function getModule(name) {
  return registry.get(name) ?? null
}

/**
 * 汇总所有模块路由，供 router 直接 spread。
 * @param {string} [only] 只取某模块
 */
export function collectRoutes(only) {
  const mods = only ? [registry.get(only)].filter(Boolean) : listModules()
  return mods.flatMap((m) => m.routes.map((r) => ({ ...r, meta: { ...r.meta, moduleName: m.name } })))
}

/**
 * 汇总侧栏/图标栏导航项，按 group 排序。
 */
export function collectNav(only) {
  const mods = only ? [registry.get(only)].filter(Boolean) : listModules()
  return mods.flatMap((m) => m.nav)
}

export function resetModules() {
  registry.clear()
  owners.clear()
}
