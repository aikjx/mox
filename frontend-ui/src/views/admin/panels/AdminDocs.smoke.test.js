// AdminDocs 面板冒烟：渲染不崩 + 端点目录可见。
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminDocs from './AdminDocs.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()),
  getHealth: vi.fn().mockResolvedValue({ status: 'up' }),
  getStatus: vi.fn().mockResolvedValue({}),
  getFullStatus: vi.fn().mockResolvedValue({}),
  getLogs: vi.fn().mockResolvedValue([]),
  getOperators: vi.fn().mockResolvedValue([]),
}))
setupGlobals()

describe('AdminDocs 冒烟', () => {
  it('渲染不崩：端点目录渲染，存在搜索框与端点条目', async () => {
    const w = mount(AdminDocs, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(w.text()).toContain('GET')
  })
})
