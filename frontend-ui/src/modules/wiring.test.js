// 装配层测试：模块登记 → 路由 → 侧栏导航，三处必须同源，任一手抄漏改在此失败。
import { existsSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { describe, it, expect } from 'vitest'
import { collectNav, collectRoutes, listModules } from './index.js'
import { MODULE_SIDEBAR_CONFIG } from '@/constants/nav.config.js'

// vitest 下 import.meta.url 可能是 http 形式，故用向上爬目录定位 src 根
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
const ROUTER_SRC = readFileSync(path.join(SRC, 'router', 'index.js'), 'utf8')

function allRegisteredRoutePaths() {
  const files = ['alliance.js', 'project.js', 'graph.js', 'ai.js', 'workflow.js', 'market.js', 'operators.js', 'system.js', 'public.js', 'fallback.js']
  return files
    .map((f) => readFileSync(path.join(SRC, 'router', 'modules', f), 'utf8'))
    .join('\n')
}

describe('模块装配单源', () => {
  it('已登记的模块均带版本与标题', () => {
    const mods = listModules()
    expect(mods.length).toBeGreaterThan(0)
    for (const m of mods) {
      expect(m.name).toBeTruthy()
      expect(m.title).toBeTruthy()
      expect(m.version).toMatch(/^\d+\.\d+\.\d+$/)
    }
  })

  it('模块路由带上 moduleName，且 meta 完整', () => {
    const routes = collectRoutes()
    const console_ = routes.find((r) => r.path === '/alliance/console')
    expect(console_).toBeTruthy()
    expect(console_.meta).toMatchObject({ module: 'expert', layout: 'default', title: '联盟控制台', moduleName: 'expert-alliance' })
  })

  it('router 在 fallback 通配之前展开模块路由', () => {
    const moduleIdx = ROUTER_SRC.indexOf('...moduleRoutes')
    const fallbackIdx = ROUTER_SRC.indexOf('...fallbackRoutes')
    expect(moduleIdx).toBeGreaterThan(-1)
    expect(fallbackIdx).toBeGreaterThan(-1)
    expect(moduleIdx).toBeLessThan(fallbackIdx)
  })

  it('模块导航项自动挂进对应模块侧栏并可跳转', () => {
    for (const item of collectNav()) {
      const sections = MODULE_SIDEBAR_CONFIG[item.module]?.sections ?? []
      const mounted = sections.flatMap(s => s.items).find(i => i.path === item.path)
      expect(mounted, `导航项 ${item.key} 未挂载到模块 ${item.module}`).toBeTruthy()
      expect(mounted.label).toBe(item.label)
    }
  })

  it('侧栏里出现的 path 必须都有真实路由，不再存在点了没反应的条目', () => {
    const routerPaths = allRegisteredRoutePaths()
    const modulePaths = collectRoutes().map(r => r.path)
    for (const cfg of Object.values(MODULE_SIDEBAR_CONFIG)) {
      for (const section of cfg.sections ?? []) {
        for (const item of section.items ?? []) {
          if (!item.path) continue
          const known = modulePaths.includes(item.path) || routerPaths.includes(`'${item.path}'`)
          expect(known, `侧栏条目 ${item.label} 指向不存在的路由 ${item.path}`).toBe(true)
        }
      }
    }
  })

  it('侧栏不再展示编造的计数与角标', () => {
    const src = readFileSync(path.join(SRC, 'constants', 'nav.config.js'), 'utf8')
    expect(src).not.toMatch(/\bcount:\s*\d+/)
    expect(src).not.toMatch(/\bbadge:\s*\d+/)
  })
})
