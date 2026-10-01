// AdminLlm 面板冒烟：挂载 + KPI 条真的渲染出**令牌**色档。
// 历史卡点 1：AdminLlm.vue 用了未导入的 `catFill`（渲染期 ReferenceError，vite build 看不见），
//   当时只能退化为"模块可加载"。缺陷已修（补 `import { catFill } from '@/constants'`），故恢复挂载。
// 历史卡点 2：`vi.mock('@/api', () => new Proxy({}, { get: () => vi.fn() }))` 会让模块命名空间成为
//   thenable（`then` 被答成函数）⇒ Vite SSR interop 在采集期 await 它且永不 settle，本文件 90s 无任何上报。
// 历史卡点 3：同一 Proxy 对**所有**端点回 `{}`，而 loadAll 把 providers/presets/logs 当数组用
//   （AdminLlm.vue:587 `providers.value = providersRes || []`，`{}` 是真值）⇒ 渲染期
//   `providers.value.filter is not a function`。故此处按面板的真实出参形状逐个给，未知端点才回 `{}`。
import { describe, it, expect, vi } from 'vitest'
import { shallowMount, flushPromises } from '@vue/test-utils'
import AdminLlm from './AdminLlm.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

// mock 工厂会被提升到 import 之前执行，共享数据必须走 vi.hoisted
const shaped = vi.hoisted(() => ({
  PROVIDERS: [
    { id: 'p1', name: '渠道A', enabled: true, has_key: true, active: true, type: 'deepseek', models: [] },
    { id: 'p2', name: '渠道B', enabled: false, has_key: false, active: false, type: 'qwen', models: [] },
  ],
}))

const api = vi.hoisted(() => ({
  getLlmProviders: vi.fn(async () => shaped.PROVIDERS),
  getLlmPresets: vi.fn(async () => []),
  getLlmHealth: vi.fn(async () => ({})),
  getLlmStats: vi.fn(async () => ({})),
  getLlmUsage: vi.fn(async () => ({})),
  getLlmLogs: vi.fn(async () => []),
}))

// 部分 mock：面板 loadAll 用到的 6 个端点按真实出参形状替死（providers/presets/logs 是**数组**，
// health/stats/usage 是对象），其余命名导出走真实模块。
// 为什么不能用整表 Proxy：懒答一切键的 Proxy 若 target 非空，vitest 的 assertMissingExport 会按
// `in` 判缺（本文件实测 `No "http" export is defined on the "@/api" mock`，alliance.api.js:53）；
// 而 target 为空的 Proxy 又会把 `then` 答成函数 ⇒ 采集期互操作永久 await。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))

// 面板 script 里 ElMessage 是全局自由变量（缺失 import 的存量缺陷，另案待裁决），先兜 no-op
setupGlobals()

describe('AdminLlm 冒烟', () => {
  it('浅挂载不崩，KPI 条按 providers 真实出参计数', async () => {
    const w = shallowMount(AdminLlm, { global: { stubs: tableStubs } })
    await flushPromises()
    const labels = w.findAll('.kpi-label').map((n) => n.text())
    const values = w.findAll('.kpi-value').map((n) => n.text().trim())
    expect(labels).toEqual(['渠道总数', '已启用', '已配置 Key', '当前使用'])
    expect(values).toEqual(['2', '1', '1', '渠道A'])
    w.unmount()
  })

  it('KPI 进度条颜色走令牌出口而非裸 hex（catFill 缺 import 时本用例必红）', async () => {
    const w = shallowMount(AdminLlm, { global: { stubs: tableStubs } })
    await flushPromises()
    const bars = w.findAll('.kpi-bar i')
    expect(bars).toHaveLength(4)
    for (const b of bars) {
      const bg = b.attributes('style') || ''
      expect(bg).toContain('var(--cat-')
      expect(bg).not.toMatch(/#[0-9a-fA-F]{3,8}/)
    }
    w.unmount()
  })
})
