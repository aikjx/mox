// AdminUser 手写面板轻量冒烟：渲染不崩 + 关键元素存在。
// 只隔离 @/api 网络层与显式 import 的 FormDialog；el-table/el-table-column 用仓库既有的
// provide-rows 替身模式驱动行（见 expert-alliance plaza-components.test.js），其余 Element Plus
// 标签按未知元素渲染为原生节点，不拉整个 EP 进测试环境。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { h, provide, inject } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import AdminUser from './AdminUser.vue'

const api = vi.hoisted(() => ({
  getUserList: vi.fn(), getUserDetail: vi.fn(), createUser: vi.fn(), updateUser: vi.fn(),
  deleteUser: vi.fn(), resetUserPwd: vi.fn(), changeUserStatus: vi.fn(),
  getUserRoles: vi.fn(), assignUserRoles: vi.fn(), getDeptTree: vi.fn(),
  getPostList: vi.fn(), getRoleList: vi.fn(), uploadUserAvatar: vi.fn(),
}))
vi.mock('@/api', () => api)

// 面板 script 里 ElMessage/ElMessageBox 是全局自由变量（成功路径不触发），兜底为 no-op
globalThis.ElMessage = { error: vi.fn(), success: vi.fn(), warning: vi.fn() }
globalThis.ElMessageBox = { confirm: vi.fn(), alert: vi.fn() }

// el-table 把行数据 provide 给每个 el-table-column，列逐行渲染作用域插槽
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
// FormDialog：v-model:visible；visible 时只渲染标题
const FormDialog = {
  name: 'FormDialog',
  props: ['visible', 'title', 'submitting', 'formSchema', 'editData', 'width', 'labelWidth'],
  setup(props) {
    return () => props.visible
      ? h('div', { class: 'fd' }, [h('div', { class: 'fd-title' }, props.title || '')])
      : h('div', { class: 'fd hidden' })
  }
}

const mountPanel = () => mount(AdminUser, {
  global: {
    stubs: {
      FormDialog,
      'el-table': ElTable,
      'el-table-column': ElTableColumn,
      'el-button': ElButton,
    }
  }
})

const userRow = (over = {}) => ({
  id: 1, username: 'admin', nickname: '管理员', deptName: '总经办', postName: '总监',
  phone: '13800000000', email: 'admin@infotopograph.dev', status: 1,
  createdAt: '2026-01-01 10:00:00', userType: 'admin', ...over
})

beforeEach(() => {
  vi.clearAllMocks()
  api.getUserList.mockResolvedValue({ list: [userRow(), userRow({ id: 2, username: 'alice', nickname: '爱丽丝' })], total: 2 })
  api.getDeptTree.mockResolvedValue([])
  api.getPostList.mockResolvedValue({ list: [] })
})

describe('AdminUser 面板冒烟', () => {
  it('渲染不崩：挂载后列表数据落地，总数徽标与新增入口存在', async () => {
    const w = mountPanel()
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getUserList).toHaveBeenCalledTimes(1)
    // 总数徽标
    expect(w.text()).toContain('共 2 位用户')
    // 新增用户入口
    expect(w.findAll('button').map((b) => b.text())).toContain('新增用户')
  })

  it('行渲染：两行用户数据可见，详情/编辑行操作存在', async () => {
    const w = mountPanel()
    await flushPromises()
    expect(w.find('.el-table-stub').attributes('data-rows')).toBe('2')
    expect(w.text()).toContain('admin')
    expect(w.text()).toContain('爱丽丝')
    const rowButtons = w.findAll('button').map((b) => b.text())
    expect(rowButtons).toContain('详情')
    expect(rowButtons).toContain('编辑')
  })

  it('点「新增用户」打开用户表单对话框，标题为新增', async () => {
    const w = mountPanel()
    await flushPromises()
    expect(w.find('.fd').classes()).toContain('hidden')
    await w.findAll('button').find((b) => b.text() === '新增用户').trigger('click')
    expect(w.find('.fd').classes()).not.toContain('hidden')
    expect(w.find('.fd-title').text()).toBe('新增用户')
  })
})
