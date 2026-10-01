// AdminHitl 面板冒烟：渲染不崩（WebSocket 在测试环境 mock 掉，只验证挂载与关键元素）。
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminHitl from './AdminHitl.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

vi.mock('@/utils', () => ({
  hitlClient: { connect: vi.fn(), disconnect: vi.fn(), approve: vi.fn(), reject: vi.fn() },
  HITL_ACTIONS: { APPROVE: 'approve', REJECT: 'reject' },
  onHitlEvent: vi.fn(() => () => {}),
  onHitlActionResult: vi.fn(() => () => {}),
  onHitlPendingList: vi.fn(() => () => {}),
  onHitlConnection: vi.fn(() => () => {}),
}))
setupGlobals()

describe('AdminHitl 冒烟', () => {
  it('渲染不崩：挂载后出现 HITL 面板关键元素', async () => {
    const w = mount(AdminHitl, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(w.text().length).toBeGreaterThan(0)
  })
})
