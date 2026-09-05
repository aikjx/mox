import { beforeEach, describe, expect, it, vi } from 'vitest'
vi.mock('./http', () => ({ default: { get: vi.fn(), post: vi.fn() } }))
import http from './http'
import { createAllianceTask, getAllianceTasks, getAllianceTaskDag, getAllianceTaskLogs, getAllianceTaskStatus, getAllianceFusionResult } from './alliance'

describe('alliance gateway contracts', () => {
  beforeEach(() => vi.clearAllMocks())
  it('submits title and supported fields, never an ignored expert selection', async () => {
    http.post.mockResolvedValue({ elapsed_ms: 2, data: { task_id: 'new', title: '目标', status: 'pending' } })
    const task = await createAllianceTask({ name: '目标', description: '具体目标', fusion_strategy: 'weighted', expert_ids: ['ignored'] })
    expect(http.post).toHaveBeenCalledWith('/alliance/tasks', { title: '目标', description: '具体目标', fusion_strategy: 'weighted' }, { _retry: 0, silent: true })
    expect(task).toMatchObject({ id: 'new', name: '目标' })
  })
  it('unwraps native tasks and converts fractional progress exactly once', async () => {
    http.get.mockResolvedValue({ elapsed_ms: 1, data: { tasks: [{ task_id: 'a', title: '分析', status: 'running', progress: 0.25 }] } })
    expect(await getAllianceTasks()).toEqual([expect.objectContaining({ id: 'a', name: '分析', progress: 25 })])
  })
  it('rejects malformed lists instead of disguising them as empty', async () => {
    http.get.mockResolvedValue({ unexpected: [] })
    await expect(getAllianceTasks()).rejects.toThrow('列表格式')
  })
  it('does not infer success from 100% progress', async () => {
    http.get.mockResolvedValue({ data: { task_id: 'a', progress: 1, status: 'failed' } })
    expect(await getAllianceTaskStatus('a')).toMatchObject({ status: 'failed', progress: 100 })
  })
  it('maps actual node positions and dependency edges', async () => {
    http.get.mockResolvedValue({ data: { nodes: [{ id: 'a', position: { x: 80, y: 60 } }, { id: 'b', position: { x: 250, y: 100 } }], edges: [{ source: 'a', target: 'b' }] } })
    const dag = await getAllianceTaskDag('a')
    expect(dag.edges[0]).toMatchObject({ x1: 80, y1: 60, x2: 250, y2: 100 })
  })
  it('maps log timestamps and uppercase levels', async () => {
    http.get.mockResolvedValue({ data: { logs: [{ ts: '2026-09-05', level: 'ERROR', message: '节点失败' }] } })
    expect((await getAllianceTaskLogs('a'))[0]).toMatchObject({ time: '2026-09-05', level: 'error' })
  })
  it('exposes actual fusion output and keeps absence distinct', async () => {
    http.get.mockResolvedValueOnce({ data: { fusion_status: 'pending' } })
      .mockResolvedValueOnce({ data: { fusion_status: 'completed', fusion_result: { summary: '结论', evidence: [1] } } })
    expect(await getAllianceFusionResult('a')).toBeNull()
    expect(await getAllianceFusionResult('a')).toMatchObject({ summary: '结论', evidence: [1] })
  })
})
