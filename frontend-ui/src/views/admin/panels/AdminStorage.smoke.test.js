// AdminStorage 面板冒烟：渲染不崩 + 状态/提供方/模块并行加载。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminStorage from './AdminStorage.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getStorageProviders: vi.fn(), switchStorageProvider: vi.fn(),
  getStorageStatus: vi.fn(), getModules: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

beforeEach(() => {
  vi.clearAllMocks()
  api.getStorageStatus.mockResolvedValue({ provider: 'local', used_pct: 12 })
  api.getStorageProviders.mockResolvedValue([{ name: 'local', selected: true }])
  api.getModules.mockResolvedValue([])
})

describe('AdminStorage 冒烟', () => {
  it('渲染不崩：挂载后并行加载存储状态，存在按钮入口', async () => {
    const w = mount(AdminStorage, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getStorageStatus).toHaveBeenCalledTimes(1)
  })
})
