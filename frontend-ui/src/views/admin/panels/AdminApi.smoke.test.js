// AdminApi 面板冒烟：渲染不崩 + 路由映射加载 + 按钮入口。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminApi from './AdminApi.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getApiMappings: vi.fn(), getApiDetail: vi.fn(), enableApi: vi.fn(), disableApi: vi.fn(),
  getActuatorMetrics: vi.fn(), getActuatorHealth: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

beforeEach(() => {
  vi.clearAllMocks()
  api.getApiMappings.mockResolvedValue([{ id: 1, path: '/api/health', method: 'GET', enabled: true }])
  api.getActuatorHealth.mockResolvedValue({ status: 'up' })
  api.getActuatorMetrics.mockResolvedValue({})
})

describe('AdminApi 冒烟', () => {
  it('渲染不崩：挂载后加载 API 映射，存在按钮入口', async () => {
    const w = mount(AdminApi, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getApiMappings).toHaveBeenCalled()
    expect(w.findAll('button').length).toBeGreaterThan(0)
  })
})
