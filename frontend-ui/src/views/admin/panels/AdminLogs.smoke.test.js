// AdminLogs 面板冒烟：渲染不崩 + logger/在线日志加载（SSE 在测试环境不启动）。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import AdminLogs from './AdminLogs.vue'
import { tableStubs, setupGlobals } from './_smoke.js'

const api = vi.hoisted(() => ({
  getLoggers: vi.fn(), setLoggerLevel: vi.fn(), getOnlineLogs: vi.fn(),
  clearOnlineLogs: vi.fn(), openLogTail: vi.fn(),
}))
// 部分 mock：整表自写会让采集期替**别人家**的账（projectContext / expert-alliance 在采集期就要 `registerProjectIdGetter`、`http`），漏一个即 Failed Suite、零用例。
vi.mock(import('@/api'), async (importOriginal) => ({ ...(await importOriginal()), ...api }))
setupGlobals()

beforeEach(() => {
  vi.clearAllMocks()
  api.getLoggers.mockResolvedValue({ data: [{ name: 'app', level: 'INFO' }] })
  api.getOnlineLogs.mockResolvedValue({ logs: [], nextSeq: 0 })
})

describe('AdminLogs 冒烟', () => {
  it('渲染不崩：挂载后加载 logger 列表与在线日志', async () => {
    const w = mount(AdminLogs, { global: { stubs: tableStubs } })
    await flushPromises()
    expect(w.exists()).toBe(true)
    expect(api.getLoggers).toHaveBeenCalledTimes(1)
  })
})
