// AdminAccess（手写旧版）面板冒烟：渲染不崩 + 凭证列表加载 + 按钮入口。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { useAuthStore } from '@/stores'
import AdminAccess from './AdminAccess.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getApiKeys: vi.fn(), createApiKey: vi.fn(), revokeApiKey: vi.fn(), validateApiKey: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

beforeEach(() => {
  setActivePinia(createPinia())
  useAuthStore().accessToken = 'existing-unit-fixture'
  vi.clearAllMocks()
  api.getApiKeys.mockResolvedValue({ items: [{ id: 1, name: '默认key', active: true, eligibility: 'eligible' }], total: 1, page: 1, page_size: 20 })
})

describe('AdminAccess(旧) 冒烟', () => {
  it('渲染不崩：挂载后加载 API 凭证，存在按钮入口', async () => {
    const w = mount(AdminAccess, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getApiKeys).toHaveBeenCalledTimes(1)
    expect(w.findAll('button').length).toBeGreaterThan(0)
  })
})
