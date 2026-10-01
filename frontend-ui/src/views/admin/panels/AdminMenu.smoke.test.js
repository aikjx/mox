// AdminMenu 面板冒烟：渲染不崩 + 菜单树加载 + 新增入口。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminMenu from './AdminMenu.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getMenuTree: vi.fn(), createMenu: vi.fn(), updateMenu: vi.fn(), deleteMenu: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

beforeEach(() => {
  vi.clearAllMocks()
  api.getMenuTree.mockResolvedValue([{ menuId: 1, menuName: '系统管理', path: '/admin', children: [] }])
})

describe('AdminMenu 冒烟', () => {
  it('渲染不崩：挂载后加载菜单树，存在按钮入口', async () => {
    const w = mount(AdminMenu, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getMenuTree).toHaveBeenCalledTimes(1)
    expect(w.findAll('button').length).toBeGreaterThan(0)
  })
})
