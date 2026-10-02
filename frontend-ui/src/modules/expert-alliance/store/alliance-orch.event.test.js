// T4 SSE 帧 → orch store：证明 store 因真实形状的帧而变（liveEvents 立即写入 +
// 带 plan_id 的帧防抖真拉统计/历史），不是测试桩自己 set 状态。
// 帧信封形状逐字对齐后端 experts_events.rs（serde(tag="type") + flatten 的扁平 JSON）。
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

const { api } = vi.hoisted(() => ({
  api: {
    orchestrate: vi.fn(),
    generateOrchPlan: vi.fn(),
    executeOrchPlan: vi.fn(),
    getOrchStats: vi.fn(),
    getOrchHistory: vi.fn()
  }
}))
vi.mock('../api/alliance.api.js', () => ({ allianceApi: api }))

const { useAllianceOrchStore } = await import('./alliance-orch.store.js')

// 真实后端帧形状（取自 experts_events.rs 单测 event_kind_serializes_with_type_tag）：
// type 标签在信封顶层，payload 字段被 flatten 拍平，execution_id 为 Option（可 null）。
const planStatusChanged = {
  id: 'evt-111',
  type: 'PlanStatusChanged',
  plan_id: 'plan-9',
  from: 'draft',
  to: 'running',
  execution_id: 'exec-1',
  source: 'execute_plan_handler',
  tenant: 'tenant-a',
  occurred_at: '2026-10-02T10:00:00Z'
}
const planCreated = {
  id: 'evt-222',
  type: 'PlanCreated',
  plan_id: 'plan-10',
  task_type: 'research',
  title: '做一次图谱调研',
  source: 'generate_plan_handler',
  tenant: 'tenant-a',
  occurred_at: '2026-10-02T10:01:00Z'
}
const expertDisabled = {
  id: 'evt-333',
  type: 'ExpertDisabled',
  expert_id: 'exp-1',
  source: 'delete_expert',
  tenant: 'tenant-a',
  occurred_at: '2026-10-02T10:02:00Z'
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.clearAllMocks()
  vi.useFakeTimers()
  api.getOrchStats.mockResolvedValue({})
  api.getOrchHistory.mockResolvedValue({ records: [], total: 0, page: 1, pageSize: 20 })
})
afterEach(() => {
  vi.useRealTimers()
})

describe('T4 帧 → store', () => {
  it('PlanStatusChanged 帧立即写入 liveEvents，snake_case 信封字段映射成 store camelCase', () => {
    const store = useAllianceOrchStore()
    expect(store.liveEvents).toHaveLength(0)

    const ev = store.applyAllianceEvent('PlanStatusChanged', planStatusChanged)

    // store 因帧而变：不是测试桩自己 push，是 applyAllianceEvent 消费真实信封后写的
    expect(store.liveEvents).toHaveLength(1)
    expect(ev).toMatchObject({
      kind: 'PlanStatusChanged',
      planId: 'plan-9',
      from: 'draft',
      to: 'running',
      executionId: 'exec-1',
      source: 'execute_plan_handler',
      occurredAt: '2026-10-02T10:00:00Z'
    })
    // 新帧插在最前（实时流倒序）
    expect(store.liveEvents[0].planId).toBe('plan-9')
  })

  it('PlanCreated 帧带 plan_id → 800ms 防抖后真拉统计与历史（合并突发帧）', () => {
    const store = useAllianceOrchStore()

    store.applyAllianceEvent('PlanCreated', planCreated)
    store.applyAllianceEvent('PlanStatusChanged', planStatusChanged)

    // 防抖窗口内不应发请求
    expect(api.getOrchStats).not.toHaveBeenCalled()
    vi.advanceTimersByTime(800)
    // 两帧只触发一次重拉（防抖合并）
    expect(api.getOrchStats).toHaveBeenCalledTimes(1)
    expect(api.getOrchHistory).toHaveBeenCalledTimes(1)
  })

  it('不带 plan_id 的专家帧（ExpertDisabled）只入事件流，不牵动编排读数', () => {
    const store = useAllianceOrchStore()

    store.applyAllianceEvent('ExpertDisabled', expertDisabled)
    expect(store.liveEvents).toHaveLength(1)
    expect(store.liveEvents[0]).toMatchObject({ kind: 'ExpertDisabled', expertId: 'exp-1', planId: '' })

    vi.advanceTimersByTime(2000)
    expect(api.getOrchStats).not.toHaveBeenCalled()
    expect(api.getOrchHistory).not.toHaveBeenCalled()
  })

  it('liveEvents 最多保留 30 条，超出丢弃最旧', () => {
    const store = useAllianceOrchStore()
    for (let i = 0; i < 35; i++) {
      store.applyAllianceEvent('ExpertRegistered', { id: `evt-${i}`, expert_id: `exp-${i}` })
    }
    expect(store.liveEvents).toHaveLength(30)
    // 最新的 evt-34 在最前
    expect(store.liveEvents[0].id).toBe('evt-34')
  })

  it('clearLiveEvents 清空事件流并挂起未触发的防抖重拉', () => {
    const store = useAllianceOrchStore()
    store.applyAllianceEvent('PlanStatusChanged', planStatusChanged)
    store.clearLiveEvents()
    expect(store.liveEvents).toHaveLength(0)
    vi.advanceTimersByTime(2000)
    // clear 时已清掉 timer，之后不应再有重拉
    expect(api.getOrchStats).not.toHaveBeenCalled()
  })
})
