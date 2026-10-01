// AdminDepartment 面板冒烟：渲染不崩 + 部门树加载 + 按钮入口。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminDepartment from './AdminDepartment.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getDeptTree: vi.fn(), getDeptDetail: vi.fn(), createDept: vi.fn(), updateDept: vi.fn(), deleteDept: vi.fn(),
  getPostByDept: vi.fn(), createPost: vi.fn(), updatePost: vi.fn(), deletePost: vi.fn(),
  getDeptUserList: vi.fn(), getUserList: vi.fn(),
  // 真实 `@/api` 还再导出这两个注册钩子（src/api/index.js:6），经 @/composables → projectContext 在采集期就被调用；
  // 缺它们本文件在 collect 阶段即 TypeError，而不是断言失败。
  registerProjectIdGetter: vi.fn(), registerAuthTokenGetter: vi.fn(),
  // expert-alliance 在采集期就 `createAllianceApi(http)`（alliance.api.js:398），需要 http 命名导出
  http: { request: vi.fn(async () => ({})) },
}))
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
vi.mock('@/components/common/FormDialog.vue', () => ({ default: { name: 'FormDialog', props: ['visible'], setup: (p, { slots }) => () => p.visible ? slots.default?.() : null, template: '<div/>' } }))
setupGlobals()

beforeEach(() => {
  vi.clearAllMocks()
  api.getDeptTree.mockResolvedValue([{ deptId: 1, deptName: '总公司', children: [] }])
})

describe('AdminDepartment 冒烟', () => {
  it('渲染不崩：挂载后加载部门树，存在按钮入口', async () => {
    const w = mount(AdminDepartment, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getDeptTree).toHaveBeenCalledTimes(1)
  })
})
