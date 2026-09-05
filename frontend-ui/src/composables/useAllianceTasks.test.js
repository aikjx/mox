import { afterEach, describe, expect, it, vi } from 'vitest'
import { useAllianceTasks } from './useAllianceTasks'
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r }); return { promise, resolve } }
function setup() {
  const api = {
    getAllianceTasks: vi.fn().mockResolvedValue([{ id: 'a', status: 'running', progress: 10 }, { id: 'b', status: 'paused', progress: 20 }]),
    getAllianceTaskLogs: vi.fn().mockResolvedValue([]), getAllianceTaskDag: vi.fn().mockResolvedValue({ nodes: [], edges: [] }),
    getAllianceFusionResult: vi.fn().mockResolvedValue({ summary: '真实结果' }),
    getAllianceTaskStatus: vi.fn().mockResolvedValue({ status: 'running', progress: 25 }),
    resumeAllianceTask: vi.fn().mockResolvedValue({}), pauseAllianceTask: vi.fn().mockResolvedValue({}), cancelAllianceTask: vi.fn().mockResolvedValue({}),
  }
  return { api, state: useAllianceTasks(api, { intervalMs: 100 }) }
}
afterEach(() => vi.useRealTimers())
describe('task workspace lifecycle', () => {
  it('ignores late details after switching tasks', async () => {
    const { state, api } = setup(), first = deferred()
    api.getAllianceTaskLogs.mockImplementation(id => id === 'a' ? first.promise : Promise.resolve([{ message: 'B' }]))
    const old = state.selectTask({ id: 'a', status: 'running' })
    await state.selectTask({ id: 'b', status: 'running' })
    first.resolve([{ message: 'A' }]); await old
    expect(state.logs.value[0].message).toBe('B')
    state.dispose()
  })
  it('completion automatically refreshes the real result', async () => {
    const { state, api } = setup(); await state.loadTasks()
    api.getAllianceTaskStatus.mockResolvedValue({ status: 'completed', progress: 100 })
    await state.poll()
    expect(state.selectedTask.value.status).toBe('completed')
    expect(state.fusionResult.value.summary).toBe('真实结果')
    state.dispose()
  })
  it('running at 100% still polls and never becomes completed locally', async () => {
    const { state, api } = setup(); await state.loadTasks()
    api.getAllianceTaskStatus.mockResolvedValue({ status: 'running', progress: 100 })
    await state.poll(); await state.poll()
    expect(api.getAllianceTaskStatus).toHaveBeenCalledTimes(2)
    expect(state.selectedTask.value.status).toBe('running')
    expect(api.getAllianceFusionResult).not.toHaveBeenCalled()
    state.dispose()
  })
  it('an action updates its original task after the user switches selection', async () => {
    const { state, api } = setup(), pending = deferred(); await state.loadTasks()
    api.pauseAllianceTask.mockReturnValue(pending.promise)
    api.getAllianceTaskStatus.mockResolvedValue({ status: 'paused', progress: 10 })
    const action = state.performAction('pause')
    await state.selectTask(state.tasks.value[1])
    pending.resolve({}); await action
    expect(state.selectedTask.value.id).toBe('b')
    expect(state.selectedTask.value.progress).toBe(20)
    expect(state.tasks.value[0].status).toBe('paused')
    state.dispose()
  })
  it('prevents duplicate actions while a request is pending', async () => {
    const { state, api } = setup(), pending = deferred(); await state.loadTasks()
    api.pauseAllianceTask.mockReturnValue(pending.promise)
    const action = state.performAction('pause')
    expect(await state.performAction('pause')).toBe(false)
    pending.resolve({}); await action
    expect(api.pauseAllianceTask).toHaveBeenCalledTimes(1)
    state.dispose()
  })
  it('keeps the last valid list when the server fails', async () => {
    const { state, api } = setup(); await state.loadTasks()
    api.getAllianceTasks.mockRejectedValue(new Error('服务离线'))
    await state.loadTasks()
    expect(state.tasks.value).toHaveLength(2)
    expect(state.tasksError.value).toBe('服务离线')
    state.dispose()
  })
  it('does not overlap polling and does not restart after unmount', async () => {
    vi.useFakeTimers()
    const { state, api } = setup(), pending = deferred(); await state.loadTasks()
    api.getAllianceTaskStatus.mockReturnValue(pending.promise)
    state.startPolling(); await vi.advanceTimersByTimeAsync(100)
    await vi.advanceTimersByTimeAsync(1000)
    expect(api.getAllianceTaskStatus).toHaveBeenCalledTimes(1)
    state.dispose(); pending.resolve({ status: 'completed', progress: 100 })
    await vi.advanceTimersByTimeAsync(1000)
    expect(api.getAllianceTaskStatus).toHaveBeenCalledTimes(1)
    expect(state.tasks.value[0].status).toBe('running')
  })
})
