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

  // 图标是 DB 驱动的字符串，而侧栏只认 _kernel/nav-icons.js 的封闭集：
  // 名单外的名字解析成 null，旧写法 <component :is="row.icon" /> 就渲染成空白单元格。
  // 本例钉住「两个渲染位点 + 录入位点」三条边。
  it('图标：已登记名画得出 svg，未登记名回落成原始文本，录入端只能从封闭集里选', async () => {
    api.getMenuTree.mockResolvedValue([
      { id: 1, name: '已登记菜单', icon: 'Aim', children: [] },
      { id: 2, name: '未登记菜单', icon: 'ThisIconIsNotInTheRegistry', children: [] },
      { id: 3, name: '没配图标的菜单', icon: '', children: [] }
    ])
    const w = mount(AdminMenu, { global: { stubs: tableStubs } })
    await flushPromises()

    const iconCol = w.find('[data-label="图标"]')
    expect(iconCol.exists()).toBe(true)
    // 三行里只有第 1 行画得出图标；第 2 行走文本回落，第 3 行走显式占位
    expect(iconCol.findAll('svg').length).toBe(1)
    expect(iconCol.text()).toContain('ThisIconIsNotInTheRegistry')
    expect(iconCol.text()).toContain('-')

    // 菜单名称列的前置图标位是同一份 DB 名的第二个消费者，不许留裸 :is
    const nameCol = w.find('[data-label="菜单名称"]')
    expect(nameCol.findAll('svg').length).toBe(1)
    expect(nameCol.text()).not.toContain('ThisIconIsNotInTheRegistry')

    // 录入端：自由文本输入框换成封闭集下拉（打错字母就是一件看不见产出的东西）
    const html = w.html()
    expect(html).toContain('<el-select')
    expect(html).not.toContain('请输入图标名称')
  })
})
