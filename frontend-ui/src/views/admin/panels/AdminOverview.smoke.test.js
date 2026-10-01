// AdminOverview 面板冒烟：渲染不崩 + 各状态摘要接口加载。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminOverview from './AdminOverview.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getSecurityStatus: vi.fn(), getStorageStatus: vi.fn(), getModules: vi.fn(),
  getLlmStats: vi.fn(), getFullStatus: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

beforeEach(() => {
  vi.clearAllMocks()
  api.getSecurityStatus.mockResolvedValue({})
  api.getStorageStatus.mockResolvedValue({})
  api.getModules.mockResolvedValue([])
  api.getLlmStats.mockResolvedValue({})
  api.getFullStatus.mockResolvedValue({ uptime_secs: 0 })
})

describe('AdminOverview 冒烟', () => {
  it('渲染不崩：挂载后加载各状态摘要', async () => {
    const w = mount(AdminOverview, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getSecurityStatus).toHaveBeenCalled()
  })
})
