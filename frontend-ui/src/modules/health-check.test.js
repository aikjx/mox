// 模块治理体检的可重复入口：跑 collectHealth() 打印四要素台账，并用 expect 守住硬不变量。
// 用法：npx vitest run src/modules/health-check.test.js
// 台账本体见 reports/markdown/module-governance.md；本测试只守住「不退化」，缺口以报告为准。
import { describe, it, expect } from 'vitest'
import { collectHealth } from './health-check.js'

describe('模块治理体检（四要素台账）', () => {
  const h = collectHealth()

  it('注册表每个模块的 nav 项都已挂进对应侧栏（不出现"注册了但导航不出现"）', () => {
    for (const m of h.modules) {
      expect(m._navUnmounted ?? [], `模块 ${m.module} 有 nav 项未挂载`).toEqual([])
    }
  })

  it('每个已注册模块都有路由与导航（注册即应可进可达）', () => {
    for (const m of h.modules.filter((x) => x.registered)) {
      expect(m.routes, `${m.module} 无路由`).not.toContain('❌')
      expect(m.nav, `${m.module} 无导航`).not.toContain('❌')
    }
  })

  it('注册表派生的导航项全部进图标侧栏（导航注册→挂载零漂移）', () => {
    // 死链判定由 wiring.test.js 全量守护；此处守住「注册表 nav 项必须在 ICON_NAV_GROUPS 可见」
    const missing = h.navDrift.iconMissingModulePages
    expect(missing, `图标侧栏缺失注册表模块页: ${missing.join(', ')}`).toEqual([])
  })

  it('打印台账（人工核对与归档用）', () => {
    console.log('\n=== 模块四要素 ===')
    console.table(h.modules.map(({ module, registered, routes, nav, endpoints, tests }) => ({ module, registered, routes, nav, endpoints, tests })))
    console.log('\n=== admin 面板 ===')
    console.table(h.adminPanels)
    console.log('\n=== 导航漂移 ===')
    console.table(h.navDrift.handwritten)
    console.log('注册了但未挂载:', h.navDrift.registeredNotMounted)
    console.log('图标侧栏缺失的模块页:', h.navDrift.iconMissingModulePages)
    // 仅打印，不断言：缺口以 module-governance.md 为准
    expect(true).toBe(true)
  })
})
