// AdminConfig 面板冒烟：渲染不崩 + 列表落地 + 新增入口。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminConfig from './AdminConfig.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getConfigList: vi.fn(), createConfig: vi.fn(), updateConfig: vi.fn(),
  deleteConfig: vi.fn(), refreshConfigCache: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

beforeEach(() => {
  vi.clearAllMocks()
  api.getConfigList.mockResolvedValue({ list: [{ id: 1, configName: '系统标题', configKey: 'sys.title', configValue: 'Mox' }], total: 1 })
})

describe('AdminConfig 冒烟', () => {
  it('渲染不崩：挂载后加载配置列表，新增入口存在', async () => {
    const w = mount(AdminConfig, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getConfigList).toHaveBeenCalledTimes(1)
    expect(w.findAll('button').length).toBeGreaterThan(0)
  })
})
