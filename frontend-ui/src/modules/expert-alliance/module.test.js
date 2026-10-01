// 模块声明门禁：路由 meta 完备、导航图标为 EP 图标名、禁止写死角标数字。
import { describe, it, expect } from 'vitest'
import { collectNav, collectRoutes, defineModule, getModule, listModules } from '../_kernel/module-registry.js'
import { EXPERT_ALLIANCE_MODULE } from './index.js'

describe('module registry', () => {
  it('联盟模块已注册且版本齐备', () => {
    const m = getModule('expert-alliance')
    expect(m).toBe(EXPERT_ALLIANCE_MODULE)
    expect(m.version).toMatch(/^\d+\.\d+\.\d+$/)
    expect(listModules().map((x) => x.name)).toContain('expert-alliance')
  })

  it('每条路由都声明 layout/module/title', () => {
    for (const r of EXPERT_ALLIANCE_MODULE.routes) {
      expect(['default', 'blank']).toContain(r.meta.layout)
      expect(typeof r.meta.module).toBe('string')
      expect(r.meta.title.length).toBeGreaterThan(1)
    }
  })

  it('collectRoutes 注入 moduleName 且不改原描述', () => {
    const routes = collectRoutes('expert-alliance')
    expect(routes[0].meta.moduleName).toBe('expert-alliance')
    expect(EXPERT_ALLIANCE_MODULE.routes[0].meta.moduleName).toBeUndefined()
  })

  it('导航项用 EP 图标名，不带 emoji、不带写死计数', () => {
    for (const n of collectNav('expert-alliance')) {
      expect(n.icon).toMatch(/^[A-Z][A-Za-z]+$/)
      expect(n.badge).toBeUndefined()
      expect(n.count).toBeUndefined()
      expect(n.path).toMatch(/^\//)
    }
  })

  it('非法模块声明即刻失败', () => {
    expect(() => defineModule({ name: 'bad1', title: 'x', version: '1.0.0', routes: [{ path: '/a', component: {}, meta: { title: 't', module: 'm' } }] })).toThrow(/meta.layout/)
    expect(() => defineModule({ name: 'bad2', title: 'x', version: '1.0.0', routes: [{ path: '/a', component: {}, meta: { title: 't', module: 'm', layout: 'weird' } }] })).toThrow(/meta.layout 只能是/)
    expect(() => defineModule({ name: 'expert-alliance', title: 'dup', version: '1.0.0' })).toThrow(/重复注册/)
    expect(() => defineModule({ title: 'no name', version: '1.0.0' })).toThrow(/必须包含/)
  })

  // dev 下改动 index.js 会让它重新执行一次 defineModule，此时必须按热更新替换而非抛错
  it('同源重复声明按热更新替换，异源抢占同名仍失败', () => {
    const desc = (title) => ({ name: 'hmr-demo', title, version: '1.0.0' })
    const url = 'file:///src/modules/hmr-demo/index.js'
    expect(() => defineModule(desc('首载'), `${url}?t=1`)).not.toThrow()
    expect(() => defineModule(desc('热更'), `${url}?t=2`)).not.toThrow()
    expect(getModule('hmr-demo').title).toBe('热更')
    expect(() => defineModule(desc('劫持'), 'file:///src/modules/elsewhere/index.js')).toThrow(/重复注册/)
    expect(() => defineModule(desc('无源'))).toThrow(/重复注册/)
  })
})
