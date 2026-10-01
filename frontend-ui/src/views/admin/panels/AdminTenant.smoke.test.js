// AdminTenant 面板冒烟：渲染不崩 + 列表落地 + 新建入口。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminTenant from './AdminTenant.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getTenantList: vi.fn(), createTenant: vi.fn(), updateTenant: vi.fn(),
  deleteTenant: vi.fn(), switchTenant: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

const row = (over = {}) => ({
  id: 1, code: 'T001', name: '默认租户', mode: 'logical', plan: 'enterprise',
  status: 'active', createdAt: '2026-01-01 10:00:00', ...over,
})

beforeEach(() => {
  vi.clearAllMocks()
  api.getTenantList.mockResolvedValue([row(), row({ id: 2, code: 'T002', name: '二号租户' })])
})

describe('AdminTenant 冒烟', () => {
  it('渲染不崩：挂载后列表数据落地，新建租户入口存在', async () => {
    const w = mount(AdminTenant, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getTenantList).toHaveBeenCalledTimes(1)
    expect(w.find('.el-table-stub').attributes('data-rows')).toBe('2')
    expect(w.text()).toContain('二号租户')
    expect(w.findAll('button').map((b) => b.text())).toContain('新建租户')
  })
})
