// 模块治理体检器（只读聚合，不改内核、不改注册表）。
// 对每个 src/modules/* 目录与注册表运行态做四要素交叉核对，输出结构化台账。
// 运行：npx vitest run src/modules/health-check.test.js
//   —— 该测试会 console.table 打印本台账，并用 expect 守住硬不变量（导航不漂移、侧栏不死链）。
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import path from 'node:path'

import { listModules, collectNav, collectRoutes } from './index.js'
import { MODULE_SIDEBAR_CONFIG, ICON_NAV_GROUPS } from '@/constants'

// vitest 下 import.meta.url 可能是 http 形式，沿 wiring.test.js 的做法向上爬目录定位 src 根
function findSrcRoot(start) {
  let dir = start
  for (let i = 0; i < 8; i++) {
    if (existsSync(path.join(dir, 'src', 'router', 'index.js'))) return path.join(dir, 'src')
    const parent = path.dirname(dir)
    if (parent === dir) break
    dir = parent
  }
  throw new Error(`未找到 src 根（起始 ${start}）`)
}
const SRC = findSrcRoot(process.cwd())

function listTestFiles(dir) {
  const out = []
  if (!existsSync(dir)) return out
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) out.push(...listTestFiles(full))
    else if (entry.name.endsWith('.test.js')) out.push(path.relative(SRC, full).replace(/\\/g, '/'))
  }
  return out
}

// 运行态注册表 → name
const runtimeNames = new Set(listModules().map((m) => m.name))
const runtimeRoutes = collectRoutes().map((r) => r.path)
const navFromRegistry = collectNav()

function collectModuleRow(dirName) {
  const dir = path.join(SRC, 'modules', dirName)
  const hasIndex = existsSync(path.join(dir, 'index.js'))
  const hasEndpoints = existsSync(path.join(dir, 'contract', 'endpoints.js'))
  const hasContractTest = existsSync(path.join(dir, 'contract', 'contract.test.js'))
  const tests = listTestFiles(dir)

  const mod = listModules().find((m) => m.name === dirName)
  const routesCount = mod ? mod.routes.length : 0
  const navCount = mod ? mod.nav.length : 0

  // nav 挂载核对：该模块 nav 项是否都落进 MODULE_SIDEBAR_CONFIG[item.module]
  let navMounted = true
  const navUnmounted = []
  for (const item of navFromRegistry.filter((n) => n.module === undefined || true)) {
    // 只核对属于本目录模块的 nav 项
  }
  if (mod) {
    for (const item of mod.nav) {
      const sections = MODULE_SIDEBAR_CONFIG[item.module]?.sections ?? []
      const mounted = sections.flatMap((s) => s.items).some((i) => i.path === item.path)
      if (!mounted) { navMounted = false; navUnmounted.push(item.path) }
    }
  }

  return {
    module: dirName,
    registered: runtimeNames.has(dirName),
    routes: routesCount > 0 ? `✅ ${routesCount} 条` : '❌ 无路由',
    nav: navCount > 0 ? (navMounted ? `✅ ${navCount} 项已挂载` : `⚠️ ${navUnmounted.length} 项未挂载`) : 'N/A 页内导航（无侧栏）',
    endpoints: hasEndpoints ? (hasContractTest ? '✅ contract/endpoints.js + 往返测试' : '⚠️ 有 endpoints.js 无 contract.test.js') : '❌ 无契约端点文件',
    tests: tests.length > 0 ? `✅ ${tests.length} 个测试文件` : '❌ 无测试',
    _navUnmounted: navUnmounted
  }
}

function collectAdminPanels() {
  const dir = path.join(SRC, 'views', 'admin', 'panels')
  const out = []
  if (!existsSync(dir)) return out
  const adminTestRefs = listTestFiles(path.join(SRC, 'views', 'admin')).join('\n')
  for (const f of readdirSync(dir).filter((f) => f.endsWith('.vue')).sort()) {
    const src = readFileSync(path.join(dir, f), 'utf8')
    const usesUnifiedApi = /from '@\/api/.test(src) || /from "@\/api/.test(src)
    const hasTest = adminTestRefs.length > 0 && new RegExp(f.replace(/\.vue$/, '')).test(adminTestRefs)
    out.push({
      panel: f,
      route: '/admin/' + f.replace(/^Admin/, '').replace(/\.vue$/, '').replace(/([A-Z])/, '-$1').toLowerCase(),
      tests: hasTest ? '✅' : '❌ 无测试',
      unifiedApi: usesUnifiedApi ? '✅ @/api' : '⚠️ 未走统一封装'
    })
  }
  return out
}

function collectNavDrift() {
  // 1) 注册了但没挂进侧栏
  const registeredNotMounted = navFromRegistry
    .filter((item) => {
      const sections = MODULE_SIDEBAR_CONFIG[item.module]?.sections ?? []
      return !sections.flatMap((s) => s.items).some((i) => i.path === item.path)
    })
    .map((i) => i.path)

  // 2) 侧栏里手写（非注册表派生）且带 path 的条目
  const registryPaths = new Set(navFromRegistry.map((i) => i.path))
  const handwritten = []
  for (const [modKey, cfg] of Object.entries(MODULE_SIDEBAR_CONFIG)) {
    for (const section of cfg.sections ?? []) {
      for (const item of section.items ?? []) {
        if (item.path && !registryPaths.has(item.path)) {
          handwritten.push({ module: modKey, path: item.path, label: item.label })
        }
      }
    }
  }

  // 3) 图标侧栏（IconSidebar）完全没有收录注册表模块页（/alliance/*）——这是导航漂移，不是死链。
  //    图标侧栏 path 是否有真实路由由 wiring.test.js 全量守护，此处不重复判定。
  const iconPaths = ICON_NAV_GROUPS.flatMap((g) => g.items).map((i) => i.path)
  const iconMissingModulePages = navFromRegistry.map((i) => i.path).filter((p) => !iconPaths.includes(p))

  return { registeredNotMounted, handwritten, iconMissingModulePages }
}

export function collectHealth() {
  const moduleDirs = readdirSync(path.join(SRC, 'modules'), { withFileTypes: true })
    .filter((d) => d.isDirectory() && d.name !== '_kernel')
    .map((d) => d.name)
    .sort()

  return {
    generatedAt: new Date().toISOString(),
    modules: moduleDirs.map(collectModuleRow),
    adminPanels: collectAdminPanels(),
    navDrift: collectNavDrift()
  }
}
