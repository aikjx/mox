// AdminRole 手写面板轻量冒烟：渲染不崩 + 关键元素存在。
// 隔离策略同 AdminUser.smoke.test.js：只 mock @/api 与显式 import 的 FormDialog，
// el-table/el-table-column 用 provide-rows 替身驱动行，其余 EP 标签按未知元素渲染。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { h, provide, inject } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import AdminRole from './AdminRole.vue'

const api = vi.hoisted(() => ({
  getRoleList: vi.fn(), getRoleDetail: vi.fn(), createRole: vi.fn(), updateRole: vi.fn(),
  deleteRole: vi.fn(), getRoleMenuPerms: vi.fn(), assignRoleMenuPerms: vi.fn(),
  getRoleDataPerms: vi.fn(), assignRoleDataPerms: vi.fn(), getRoleUsers: vi.fn(),
  copyRole: vi.fn(), getMenuTree: vi.fn(), getDeptTree: vi.fn(), ROLE_TEMPLATES: [],
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（`registerProjectIdGetter` / `http` 等由
// `projectContext.js` 与 `expert-alliance` 在采集期就要），漏一个 ⇒ Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))

globalThis.ElMessage = { error: vi.fn(), success: vi.fn(), warning: vi.fn() }
globalThis.ElMessageBox = { confirm: vi.fn(), alert: vi.fn() }

const ROWS = Symbol('rows')
const ElTable = {
  name: 'el-table',
  props: ['data', 'vLoading'],
  setup(props, { slots }) {
    provide(ROWS, () => props.data || [])
    return () => h('div', { class: 'el-table-stub', 'data-rows': String((props.data || []).length) }, slots.default?.())
  }
}
const ElTableColumn = {
  name: 'el-table-column',
  props: ['label', 'prop'],
  setup(props, { slots }) {
    const rows = inject(ROWS, () => [])
    return () => h('div', { class: 'el-col' }, rows().map((row, i) =>
      h('div', { class: 'el-cell', 'data-i': String(i) }, slots.default ? slots.default({ row, $index: i }) : row[props.prop] ?? '')
    ))
  }
}
const ElButton = {
  name: 'el-button',
  props: ['type', 'icon', 'loading', 'disabled', 'link', 'size'],
  setup(props, { slots }) {
    return () => h('button', { class: 'el-btn', disabled: props.disabled ? '' : null }, slots.default?.())
  }
}
// 关闭态权限弹窗里的 el-tree 带作用域插槽，未解析组件会以空 scope 渲染而崩溃；这里只渲染空节点。
const ElTree = { name: 'el-tree', setup: () => () => h('div', { class: 'el-tree-stub' }) }

const FormDialog = {
  name: 'FormDialog',
  props: ['visible', 'title', 'submitting', 'formSchema', 'editData', 'width', 'labelWidth'],
  setup(props) {
    return () => props.visible
      ? h('div', { class: 'fd' }, [h('div', { class: 'fd-title' }, props.title || '')])
      : h('div', { class: 'fd hidden' })
  }
}

const mountPanel = () => mount(AdminRole, {
  global: {
    stubs: {
      FormDialog,
      'el-table': ElTable,
      'el-table-column': ElTableColumn,
      'el-button': ElButton,
      'el-tree': ElTree,
    }
  }
})

const roleRow = (over = {}) => ({
  id: 1, name: '超级管理员', code: 'SUPER_ADMIN', builtin: true,
  dataScope: 'all', status: 1, sort: 1, createdAt: '2026-01-01 10:00:00', ...over
})

beforeEach(() => {
  vi.clearAllMocks()
  api.getRoleList.mockResolvedValue({ list: [roleRow(), roleRow({ id: 2, name: '普通角色', code: 'NORMAL', builtin: false })], total: 2 })
  api.getDeptTree.mockResolvedValue([])
})

describe('AdminRole 面板冒烟', () => {
  it('渲染不崩：角色列表落地，总数徽标与新增入口存在', async () => {
    const w = mountPanel()
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getRoleList).toHaveBeenCalledTimes(1)
    expect(w.text()).toContain('共 2 个角色')
    expect(w.findAll('button').map((b) => b.text())).toContain('新增角色')
  })

  it('行渲染：两行角色可见，编辑/菜单权限行操作存在', async () => {
    const w = mountPanel()
    await flushPromises()
    expect(w.find('.el-table-stub').attributes('data-rows')).toBe('2')
    expect(w.text()).toContain('超级管理员')
    expect(w.text()).toContain('普通角色')
    const rowButtons = w.findAll('button').map((b) => b.text())
    expect(rowButtons).toContain('编辑')
    expect(rowButtons).toContain('菜单权限')
  })

  it('点「新增角色」打开角色表单对话框，标题为新增', async () => {
    const w = mountPanel()
    await flushPromises()
    expect(w.find('.fd').classes()).toContain('hidden')
    await w.findAll('button').find((b) => b.text() === '新增角色').trigger('click')
    expect(w.find('.fd').classes()).not.toContain('hidden')
    expect(w.find('.fd-title').text()).toBe('新增角色')
  })
})
