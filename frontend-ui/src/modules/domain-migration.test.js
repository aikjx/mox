// 8 个 legacy 业务域迁入 defineModule 后的装配守护：
// collectRoutes 必须含各域的代表路由，meta 四要素（title/module/layout）齐备。
import { describe, it, expect } from 'vitest'
import { collectRoutes, listModules } from './index.js'

const ROUTE_PATTERNS = [
  ['project', '/dashboard'],
  ['project', '/projects'],
  ['project', '/tasks'],
  ['project', '/resources'],
  ['project', '/workbench'],
  ['ai', '/ai'],
  ['ai', '/share/:token'],
  ['ai', '/caomei'],
  ['ai', '/algolab'],
  ['ai', '/infinite-optimizer'],
  ['ai', '/botCenter'],
  ['ai', '/melody2score'],
  ['graph', '/graph'],
  ['graph', '/mox-fusion'],
  ['graph', '/flow-graph'],
  ['workflow', '/workflow'],
  ['workflow', '/browser'],
  ['market', '/market'],
  ['market', '/market/:id'],
  ['operators', '/operators'],
  ['system', '/admin'],
  ['expert-alliance', '/expert-workspace'],
  ['expert-alliance', '/expert-center'],
  ['expert-alliance', '/expert-config'],
  ['expert-alliance', '/expert-plaza'],
]

describe('legacy 域迁入内核', () => {
  it('各域代表路由已由 collectRoutes 收集', () => {
    const paths = collectRoutes().map((r) => r.path)
    for (const [, p] of ROUTE_PATTERNS) {
      expect(paths, `缺少路由 ${p}`).toContain(p)
    }
  })

  it('收集到的模块路由 meta 四要素齐备', () => {
    const byPath = Object.fromEntries(collectRoutes().map((r) => [r.path, r]))
    for (const [, p] of ROUTE_PATTERNS) {
      const r = byPath[p]
      expect(r, `路由 ${p} 未收集`).toBeTruthy()
      expect(r.meta.title, `${p} 缺 title`).toBeTruthy()
      expect(r.meta.module, `${p} 缺 module`).toBeTruthy()
      expect(['default', 'blank'], `${p} layout=${r.meta.layout}`).toContain(r.meta.layout)
    }
  })

  it('已登记模块数量较迁移前增长（新增 7 个域模块）', () => {
    const names = listModules().map((m) => m.name)
    for (const n of ['project', 'ai', 'graph', 'workflow', 'market', 'operators', 'system']) {
      expect(names, `模块 ${n} 未登记`).toContain(n)
    }
  })

  it('/share 裸页保留 bare 标记且 layout=blank', () => {
    const r = collectRoutes().find((x) => x.path === '/share/:token')
    expect(r).toBeTruthy()
    expect(r.meta.bare).toBe(true)
    expect(r.meta.shareMode).toBe(true)
    expect(r.meta.layout).toBe('blank')
  })
})
