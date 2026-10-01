// AdminMonitor 面板冒烟：只测浅挂载不崩（ECharts/轮询在测试环境不真实启动）。
import { describe, it, expect, vi } from 'vitest'
import { shallowMount, flushPromises } from '@vue/test-utils'
import { ref } from 'vue'
import AdminMonitor from './AdminMonitor.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

vi.mock('@/echarts', () => ({
  init: vi.fn(() => ({ setOption: vi.fn(), resize: vi.fn(), dispose: vi.fn(), on: vi.fn() })),
}))
// useProject 的真实出参（src/composables/projectContext.js:123-133）。旧替身自造了 projectId/projectName
// 两个不存在的键，面板实际解构的 onChange/ensureProjectContext 反而是 undefined ⇒ 挂载后 unhandled rejection。
vi.mock('@/composables', () => ({
  useProject: () => ({
    currentProject: ref(null), projectList: ref([]), listLoading: ref(false), projectReady: ref(false),
    setCurrentProject: vi.fn(), ensureProjectContext: vi.fn(async () => null),
    createAndSelect: vi.fn(async () => null), loadProjectList: vi.fn(async () => []),
    onChange: vi.fn(() => () => {}),
  }),
}))
// 同 AdminLlm 冒烟：懒桩不能把 `then` 答成函数，否则命名空间 thenable ⇒ 采集期 await 永不 settle。
vi.mock('@/api', () => new Proxy({}, {
  get: (t, k) => (k === 'then' || k === '__esModule' || typeof k === 'symbol'
    ? (k === '__esModule' ? true : undefined)
    : (t[k] || (t[k] = vi.fn().mockResolvedValue({})))),
}))
setupGlobals()

describe('AdminMonitor 冒烟', () => {
  it('浅挂载不崩：组件模块可加载并完成初始渲染', async () => {
    const w = shallowMount(AdminMonitor, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    w.unmount()
  })
})
