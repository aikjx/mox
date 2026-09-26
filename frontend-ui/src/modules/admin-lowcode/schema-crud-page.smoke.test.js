// SchemaCrudPage 引擎路径冒烟：走 useCrudPage + SchemaCrudPage 编排真实 pageSchema，
// 只隔离重型子组件（SearchForm/DataTable/FormDialog/SchemaRenderer）与 @/api 网络层，
// 验证「渲染不崩 / 列表加载态 / 增删改入口 / 错误态 / 两个 schema 页（config 服务端分页 + access 小数据量）」。
// stub 风格对齐 expert-alliance/components/plaza-components.test.js：手写透传替身，不拉整个 Element Plus。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { h, provide, nextTick } from 'vue'
import { mount, flushPromises } from '@vue/test-utils'
import SchemaCrudPage from './engine/SchemaCrudPage.vue'
import { configPage } from './pages/config.page.js'
import { accessPage } from './pages/access.page.js'

// —— 网络层替身：本文件内整体替换 @/api，调用结果由各用例自行控制 ——
const api = vi.hoisted(() => ({
  getConfigList: vi.fn(),
  createConfig: vi.fn(),
  updateConfig: vi.fn(),
  deleteConfig: vi.fn(),
  refreshConfigCache: vi.fn(),
  getApiKeys: vi.fn(),
  createApiKey: vi.fn(),
  revokeApiKey: vi.fn(),
}))
vi.mock('@/api', () => api)

// —— ElMessage / ElMessageBox 替身：捕获调用即可，不弹真实气泡 ——
const msgs = vi.hoisted(() => ({ error: vi.fn(), success: vi.fn(), warning: vi.fn() }))
const mbxs = vi.hoisted(() => ({ confirm: vi.fn(), alert: vi.fn() }))
vi.mock('element-plus/es/components/message/index', () => ({ ElMessage: msgs }))
vi.mock('element-plus/es/components/message-box/index', () => ({ ElMessageBox: mbxs }))

// —— 重型子组件替身 ——
// DataTable：按 props.data 逐行渲染，把 #actions 作用域插槽落到每行；loading/rows 暴露为 data-* 便于断言。
const ROWS = Symbol('dt-rows')
const DataTableStub = {
  name: 'DataTable',
  props: ['data', 'columns', 'loading', 'total', 'serverPagination', 'showPagination', 'rowKey'],
  setup(props, { slots, emit }) {
    provide(ROWS, () => props.data || [])
    return () => h('div', {
      class: 'dt',
      'data-loading': String(!!props.loading),
      'data-rows': String((props.data || []).length),
    }, (props.data || []).map((row, i) => h('div', {
      class: 'dt-row',
      key: row.id ?? i,
      onClick: () => emit('row-click', row),
    }, slots.actions ? slots.actions({ row, index: i }) : null)))
  }
}
const SearchFormStub = {
  name: 'SearchForm',
  props: ['fields', 'showKeyword', 'keywordPlaceholder'],
  emits: ['search'],
  setup(_props, { emit }) {
    return () => h('div', { class: 'sf' }, [
      h('button', { class: 'sf-search', onClick: () => emit('search', { keyword: '' }) }, '搜索')
    ])
  }
}
// FormDialog：v-model:visible；visible 时渲染标题 + 提交按钮（回写 onSubmit）
const FormDialogStub = {
  name: 'FormDialog',
  props: ['visible', 'title', 'submitting', 'formSchema', 'editData', 'width', 'labelWidth'],
  emits: ['update:visible', 'submit'],
  setup(props, { emit }) {
    return () => props.visible
      ? h('div', { class: 'fd' }, [
          h('div', { class: 'fd-title' }, props.title || ''),
          h('button', {
            class: 'fd-submit',
            onClick: () => emit('submit', { name: 'n', key: 'k', value: 'v' })
          }, '提交')
        ])
      : h('div', { class: 'fd hidden' })
  }
}
const SchemaRendererStub = { name: 'SchemaRenderer', setup: () => () => h('span') }
const ElButtonStub = {
  name: 'el-button',
  props: ['type', 'icon', 'loading', 'disabled', 'size', 'link'],
  setup(props, { slots }) {
    return () => h('button', { class: 'el-btn', disabled: props.disabled ? '' : null }, slots.default?.())
  }
}

// DataTable/SearchForm/FormDialog/SchemaRenderer 都是 SchemaCrudPage 显式 import 的组件，
// 必须走 global.stubs 覆盖（global.components 只对模板里按字符串解析的未注册组件生效）。
const mountPage = (pageSchema) => mount(SchemaCrudPage, {
  props: { pageSchema },
  global: {
    stubs: {
      DataTable: DataTableStub,
      SearchForm: SearchFormStub,
      FormDialog: FormDialogStub,
      SchemaRenderer: SchemaRendererStub,
      'el-button': ElButtonStub,
    }
  }
})

beforeEach(() => {
  vi.clearAllMocks()
})

const cfgRow = (over = {}) => ({
  id: 1, name: '系统名称', key: 'sys.name', value: 'Infotopograph',
  isBuiltin: true, status: 1, remark: '', createdAt: '2026-01-01 10:00:00', ...over
})

describe('SchemaCrudPage · config 页（服务端分页 CRUD）', () => {
  it('挂载即渲染：加载中表格进入 loading 态，resolve 后落出行数据', async () => {
    let resolveList
    api.getConfigList.mockImplementation(() => new Promise((res) => { resolveList = res }))
    const w = mountPage(configPage)
    // onMounted 同步触发 loadList：loading=true 的渲染更新在微任务里，先 nextTick 刷到 DOM；
    // 此时 list Promise 仍 pending，故表格应处于 loading 态
    await nextTick()
    expect(w.find('.dt').attributes('data-loading')).toBe('true')
    expect(api.getConfigList).toHaveBeenCalledWith(expect.objectContaining({ pageNum: 1, pageSize: 10 }))

    resolveList({ list: [cfgRow(), cfgRow({ id: 2, name: '会话超时', key: 'sys.session.timeout', isBuiltin: false })], total: 2 })
    await flushPromises()
    expect(w.find('.dt').attributes('data-loading')).toBe('false')
    expect(w.find('.dt').attributes('data-rows')).toBe('2')
  })

  it('工具栏与行操作入口存在：新增按钮 + 每行编辑/删除；点编辑打开编辑对话框', async () => {
    api.getConfigList.mockResolvedValue({ list: [cfgRow()], total: 1 })
    const w = mountPage(configPage)
    await flushPromises()

    // 增：工具栏「新增参数」
    expect(w.findAll('.toolbar .el-btn').map((b) => b.text())).toContain('新增参数')
    // 删/改：行内按钮（内置行删除按 schema 禁用）
    const rowBtns = w.findAll('.dt-row .el-btn').map((b) => b.text())
    expect(rowBtns).toContain('编辑')
    expect(rowBtns).toContain('删除')

    // 点「编辑」→ 对话框打开且标题来自 schema.editTitle
    await w.find('.dt-row').findAll('.el-btn').find((b) => b.text() === '编辑').trigger('click')
    expect(w.find('.fd').classes()).not.toContain('hidden')
    expect(w.find('.fd-title').text()).toBe('编辑参数配置')

    // 点「新增参数」→ 对话框标题切到 createTitle
    await w.findAll('.toolbar .el-btn').find((b) => b.text() === '新增参数').trigger('click')
    expect(w.find('.fd-title').text()).toBe('新增参数配置')
  })

  it('列表请求失败：不渲染行，ElMessage.error 提示加载失败', async () => {
    api.getConfigList.mockRejectedValue(new Error('网关 502'))
    const w = mountPage(configPage)
    await flushPromises()
    expect(w.find('.dt').attributes('data-rows')).toBe('0')
    expect(msgs.error).toHaveBeenCalledWith(expect.stringContaining('加载列表失败'))
  })
})

describe('SchemaCrudPage · access 页（小数据量 + 自定义吊销动作）', () => {
  it('挂载后归一化凭证行，吊销入口仅对活跃凭证出现', async () => {
    api.getApiKeys.mockResolvedValue([
      { id: 1, name: '巡检客户端', status: 'active', scopes: ['read'], createdAt: '2026-09-01', lastUsed: '' },
      { id: 2, name: '已废弃', status: 'revoked', scopes: [], createdAt: '2026-08-01', lastUsed: '2026-08-02' },
    ])
    const w = mountPage(accessPage)
    await flushPromises()
    expect(api.getApiKeys).toHaveBeenCalledTimes(1)
    expect(w.find('.dt').attributes('data-rows')).toBe('2')

    // 第一行（活跃）有「吊销」；第二行（已吊销）按 schema.show 不渲染吊销入口
    const rows = w.findAll('.dt-row')
    expect(rows[0].findAll('.el-btn').map((b) => b.text())).toContain('吊销')
    expect(rows[1].findAll('.el-btn').map((b) => b.text())).not.toContain('吊销')

    // 工具栏「新建凭证」入口存在
    expect(w.findAll('.toolbar .el-btn').map((b) => b.text())).toContain('新建凭证')
  })
})
