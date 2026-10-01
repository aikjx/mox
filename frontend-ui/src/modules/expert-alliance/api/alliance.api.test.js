// API 层契约测试：路径/方法/信封层数/字段归一，全部用假 http 客户端，不打真实网关。
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { createAllianceApi } from './alliance.api.js'
import { emptyExpertDraft } from '../contract/registry.js'

const nested = (data, params = {}) => ({ status: 200, data: { code: 0, msg: 'ok', data: { elapsed_ms: 1, params, data } } })
const flat = (data) => ({ status: 200, data: { code: 0, msg: 'ok', data } })

function makeClient(next) {
  const request = vi.fn(async () => (typeof next === 'function' ? next() : next))
  return { client: { request }, request }
}

describe('alliance api', () => {
  let client
  beforeEach(() => {
    client = makeClient(nested({})).client
  })

  it('listTasks 走 /alliance/tasks 并按 tasks 归一', async () => {
    const { client: c, request } = makeClient(nested({ tasks: [{ task_id: 't1', title: 'A', status: 'running', progress: 30 }], total: 1, page: 1, page_size: 20 }))
    const api = createAllianceApi(c)
    const out = await api.listTasks()
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ url: '/alliance/tasks', method: 'GET' }))
    expect(out.items[0]).toMatchObject({ id: 't1', status: 'running', progress: 30 })
    expect(out.total).toBe(1)
  })

  it('createTask 提交 snake_case 且 mode 用传输名', async () => {
    const { client: c, request } = makeClient(nested({ task_id: 't9', title: 'x', status: 'pending' }, { description: 'd', task_type: 'general', priority: 'high', mode: 'parallel', fusion_strategy: 'weighted' }))
    const api = createAllianceApi(c)
    const out = await api.createTask({ title: 'x', description: 'd', priority: 'high', mode: 'parallel', fusionStrategy: 'weighted' })
    const cfg = request.mock.calls[0][0]
    expect(cfg.method).toBe('POST')
    expect(cfg.data).toEqual({ title: 'x', description: 'd', task_type: undefined, priority: 'high', mode: 'parallel', fusion_strategy: 'weighted' })
    expect(out.id).toBe('t9')
    expect(out.modeDisplay).toBe('expert_alliance')
  })

  it('任务详情与节点/日志/DAG/融合均正确取双层 payload', async () => {
    const calls = []
    const c = { request: vi.fn(async (cfg) => { calls.push(cfg); return nested(cfg.url.endsWith('/dag') ? { nodes: [{ id: 'n1', status: 'completed' }], edges: [], stats: {} } : cfg.url.endsWith('/logs') ? { task_id: 't1', logs: [{ seq: 1, level: 'info', message: 'm' }], total: 1 } : cfg.url.endsWith('/fusion-result') ? { task_id: 't1', fusion_result: { summary: 's', confidence: 0.85 } } : { task_id: 't1', status: 'running' }) }) }
    const api = createAllianceApi(c)
    await api.getTask('t1')
    const dag = await api.getDag('t1')
    const logs = await api.getLogs('t1')
    const fusion = await api.getFusion('t1')
    expect(calls.map((x) => x.url)).toEqual(['/alliance/tasks/t1', '/alliance/tasks/t1/dag', '/alliance/tasks/t1/logs', '/alliance/tasks/t1/fusion-result'])
    expect(dag.nodes[0].id).toBe('n1')
    expect(logs.items[0].message).toBe('m')
    expect(fusion.grade).toBe('B')
  })

  it('controlTask 映射到独立动词路径，未知动词报错', async () => {
    const { client: c, request } = makeClient(nested({ success: true, message: 'ok' }))
    const api = createAllianceApi(c)
    await api.controlTask('t1', 'pause')
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ url: '/alliance/tasks/t1/pause', method: 'POST' }))
    await expect(api.controlTask('t1', 'explode')).rejects.toThrow(/不支持的任务动作/)
  })

  it('toggleTaskDone 走 PUT toggle-done，并分清本地/远程两条分支的形状', async () => {
    const { client: c, request } = makeClient(nested({
      task_id: 't1',
      previous_status: 'running',
      current_status: 'completed',
      toggled: true,
      completed_at: '2026-09-23T07:00:00Z',
      message: '任务 t1 已标记为完成'
    }))
    const api = createAllianceApi(c)
    const out = await api.toggleTaskDone('t1')
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ url: '/alliance/tasks/t1/toggle-done', method: 'PUT' }))
    expect(out).toMatchObject({ branch: 'local', direction: 'completed', currentStatus: 'completed', completedAt: '2026-09-23T07:00:00Z' })
  })

  it('远程分支只回 success+message：不伪造状态，也不从 params 里捞 task_id', async () => {
    const { client: c } = makeClient({
      status: 200,
      data: {
        code: 0, msg: 'ok',
        // alliance_remote.rs:543-552 —— data 只有两键，task_id 在信封同级的 params 里
        data: { elapsed_ms: 3, params: { task_id: 't1', action: 'Complete' }, data: { success: true, message: '任务 t1 已标记为完成' } }
      }
    })
    const api = createAllianceApi(c)
    const out = await api.toggleTaskDone('t1')
    expect(out).toMatchObject({ branch: 'remote', direction: 'completed', currentStatus: '', taskId: '' })
  })

  it('toggled:false 是"重新打开"而不是"标记完成"，缺 toggled 也不许猜方向', async () => {
    const reopened = createAllianceApi(makeClient(nested({ toggled: false, previous_status: 'completed', current_status: 'running' })).client)
    await expect(reopened.toggleTaskDone('t1')).resolves.toMatchObject({ direction: 'reopened' })
    const shapeless = createAllianceApi(makeClient(nested({ message: '后端换了形状' })).client)
    const out = await shapeless.toggleTaskDone('t1')
    expect(out).toMatchObject({ branch: 'unknown', direction: 'unknown', message: '后端换了形状' })
  })

  it('runDispatch 只发 handler 认识的键：空 expert_ids 不占位，constraints 永不出现', async () => {
    const { client: c, request } = makeClient(flat({
      dispatch_id: 'disp-1',
      task_type: 'code_review',
      assigned_experts: [{ id: 'e1', name: '玄枢', match_score: 0.812, load_ratio: 0.25 }],
      strategy_used: 'best_match',
      match_scores: { e1: 0.812 },
      status: 'dispatched',
      created_at: '2026-09-23T07:00:00Z'
    }))
    const api = createAllianceApi(c)
    const out = await api.runDispatch({ taskType: ' code_review ', input: '前端 架构', expertIds: ['', '  '], constraints: { top_k: 3 } })
    const cfg = request.mock.calls[0][0]
    expect(cfg).toMatchObject({ url: '/experts/dispatcher/dispatch', method: 'POST' })
    expect(cfg.data).toEqual({ task_type: 'code_review', input: '前端 架构' })
    expect(out).toMatchObject({ dispatchId: 'disp-1', strategyUsed: 'best_match', status: 'dispatched' })
    expect(out.assigned[0]).toMatchObject({ id: 'e1', name: '玄枢', matchScore: 0.812, loadRatio: 0.25 })
  })

  it('指定专家非空才进请求体，且后端 filter_map 丢掉的人不会被 match_scores 补回来', async () => {
    const { client: c, request } = makeClient(flat({
      dispatch_id: 'disp-2',
      task_type: 't',
      assigned_experts: [],
      strategy_used: 'specified',
      // 后端 scores 的键来自 assigned_ids，但 assigned_experts 会因注册表查不到而 filter_map 掉
      match_scores: { ghost: 0.4 },
      status: 'dispatched',
      created_at: ''
    }))
    const api = createAllianceApi(c)
    const out = await api.runDispatch({ taskType: 't', input: 'x', expertIds: [' ghost ', 'e2'] })
    expect(request.mock.calls[0][0].data).toEqual({ task_type: 't', input: 'x', expert_ids: ['ghost', 'e2'] })
    expect(out.assigned).toEqual([])
    expect(out.strategyUsed).toBe('specified')
  })

  it('路径参数缺失即抛错，不发请求', async () => {
    const { client: c, request } = makeClient(nested({}))
    const api = createAllianceApi(c)
    await expect(api.graphPath('e1')).rejects.toThrow(/target/)
    expect(request).not.toHaveBeenCalled()
  })

  it('日志流 URL 带 /api 前缀供 EventSource 使用', () => {
    const api = createAllianceApi(client)
    expect(api.taskLogStreamUrl('t/1')).toBe('/api/alliance/tasks/t%2F1/logs/stream')
  })

  it('experts 族为单层信封，查询串只带后端认识的参数', async () => {
    const { client: c, request } = makeClient(flat({
      experts: [{
        id: 'e1',
        name: 'N',
        domains: ['architecture'],
        availability: { status: 'online', last_active: '', avg_response_minutes: 3.2, current_load: 1, max_concurrent: 4 },
        metrics: { avg_rating: 4.6, rating_count: 12, total_consultations: 88, resolution_rate: 0.92 },
        capabilities: [{ id: 'c1', name: '架构评审', domain: 'architecture', proficiency: 90 }]
      }],
      total: 1
    }))
    const api = createAllianceApi(c)
    const out = await api.listExperts({ search: '架构', expertType: '', status: 'online', page: 2, pageSize: 24 })
    expect(request).toHaveBeenCalledWith(expect.objectContaining({
      url: '/experts', method: 'GET', params: { search: '架构', status: 'online', page: 2, page_size: 24 }
    }))
    expect(out.items[0]).toMatchObject({ id: 'e1', status: 'online', online: true })
    // availability/capabilities 在 Rust 侧是结构体，归一后必须是对象数组而非 "[object Object]"
    expect(out.items[0].availability).toMatchObject({ status: 'online', currentLoad: 1, maxConcurrent: 4, loadRatio: 0.25 })
    expect(out.items[0].capabilities[0]).toMatchObject({ name: '架构评审', proficiency: 90 })
    expect(out.items[0].metrics.totalConsultations).toBe(88)
  })

  it('能力目录走扁平信封并按 id 次序原样交给上层', async () => {
    const { client: c, request } = makeClient(flat({
      capabilities: [
        { id: 'cap-arch', name: '架构评审', domain: 'architecture', expert_count: 3, avg_proficiency: 88.5 },
        { id: 'cap-kb', name: '知识建模', domain: 'kb', expert_count: 1, avg_proficiency: 70 }
      ],
      total: 2,
      domains: ['architecture', 'kb']
    }))
    const api = createAllianceApi(c)
    const out = await api.listExpertCapabilities()
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ url: '/experts/capabilities', method: 'GET', silent: true }))
    expect(out.total).toBe(2)
    expect(out.domains).toEqual(['architecture', 'kb'])
    expect(out.items[0]).toEqual({ id: 'cap-arch', name: '架构评审', domain: 'architecture', expertCount: 3, avgProficiency: 88.5 })
    // 后端按 id 升序，前端不重排
    expect(out.items.map((i) => i.id)).toEqual(['cap-arch', 'cap-kb'])
  })

  it('单专家派生指标填路径参数并归一出 derived 三值', async () => {
    const { client: c, request } = makeClient(flat({
      expert_id: 'e/1',
      metrics: { total_consultations: 180, avg_rating: 4.7, rating_count: 56, resolution_rate: 0.93 },
      availability: { status: 'online', current_load: 3, max_concurrent: 4 },
      derived: { rank_percentile: 62.5, load_ratio: 0.75, efficiency_score: 0.7836 }
    }))
    const api = createAllianceApi(c)
    const out = await api.getExpertMetrics('e/1')
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ url: '/experts/e%2F1/metrics', method: 'GET' }))
    expect(out.expertId).toBe('e/1')
    expect(out.derived).toEqual({ rankPercentile: 62.5, loadRatio: 0.75, efficiencyScore: 0.7836 })
    expect(out.metrics).toMatchObject({ totalConsultations: 180, avgRating: 4.7, resolutionRate: 0.93 })
    expect(out.availability).toMatchObject({ status: 'online', currentLoad: 3, maxConcurrent: 4, loadRatio: 0.75 })
  })

  // ── 广场交互端点：路径/方法/请求体逐一对齐 experts_ext.rs 与 experts_registry.rs ──
  describe('广场交互', () => {
    it('我的预约取 bookings 与四态计数', async () => {
      const { client: c, request } = makeClient(flat({
        bookings: [{ id: 'b1', expert_id: 'e1', expert_name: 'N', topic: '架构', scheduled_at: '2026-09-24T00:00:00Z', duration_minutes: 60, status: 'pending', created_at: '' }],
        total: 1, pending: 1, confirmed: 0, completed: 0, cancelled: 0
      }))
      const out = await createAllianceApi(c).listMyBookings()
      expect(request).toHaveBeenCalledWith(expect.objectContaining({ url: '/experts/bookings/mine', method: 'GET' }))
      expect(out.items[0]).toMatchObject({ id: 'b1', expertId: 'e1', status: 'pending', cancellable: true })
      expect(out.counts.pending).toBe(1)
    })

    it('创建预约提交 snake_case，可选项省略时不发送', async () => {
      const { client: c, request } = makeClient(flat({ id: 'b2', expert_id: 'e1', expert_name: 'N', status: 'pending', topic: 'x' }))
      await createAllianceApi(c).createBooking({ expertId: 'e1', topic: 'x' })
      expect(request.mock.calls[0][0]).toMatchObject({ url: '/experts/bookings', method: 'POST', data: { expert_id: 'e1', topic: 'x' } })
      expect(request.mock.calls[0][0].data.scheduled_at).toBeUndefined()
    })

    it('取消预约走 PUT，收藏走 POST 到专家路径', async () => {
      const { client: c, request } = makeClient(flat({ booking_id: 'b1', status: 'cancelled', cancelled_at: 't', message: 'ok' }))
      const api = createAllianceApi(c)
      await api.cancelBooking('b1')
      expect(request).toHaveBeenLastCalledWith(expect.objectContaining({ url: '/experts/bookings/b1/cancel', method: 'PUT' }))
      const fav = makeClient(flat({ expert_id: 'e1', favorite: true, action: 'favorited', updated_at: 't' }))
      expect(await createAllianceApi(fav.client).toggleFavorite('e1')).toEqual({ expertId: 'e1', favorite: true, action: 'favorited', updatedAt: 't' })
    })

    it('即时咨询：不在线时 sessionId 为 null 而非抛错', async () => {
      const { client: c, request } = makeClient(flat({ expert_id: 'e1', session_id: null, status: 'unavailable', channel: 'text', topic: '即时咨询', question: null, expert_online: false, chat_url: null, created_at: 't', message: '专家当前不在线' }))
      const out = await createAllianceApi(c).consultNow('e1', { topic: '即时咨询', question: 'q' })
      expect(request.mock.calls[0][0]).toMatchObject({ url: '/experts/e1/consult-now', method: 'POST', data: { topic: '即时咨询', question: 'q', channel: 'text' } })
      expect(out).toMatchObject({ sessionId: null, expertOnline: false, chatUrl: null, message: '专家当前不在线' })
    })

    it('咨询室解包 webrtc 配置，团队申请保留审批语义', async () => {
      const room = makeClient(flat({ booking_id: 'b1', room_id: 'r1', room_token: 'tk', join_url: '/consult/room/r1', webrtc_config: { ice_servers: [{ urls: 'stun:a' }] }, expert_info: null, status: 'waiting', expires_in: 3600, created_at: 't' }))
      const r = await createAllianceApi(room.client).consultRoom('b1')
      expect(r).toMatchObject({ roomId: 'r1', joinUrl: '/consult/room/r1', iceServers: ['stun:a'], expertInfo: null })
      const team = makeClient(flat({ application_id: 'a1', status: 'pending_approval', team_id: 't1', expert_id: 'e1', role: 'member', applied_at: 't', estimated_review_hours: 24, message: 'm' }))
      const t = await createAllianceApi(team.client).joinTeam({ teamId: 't1', expertId: 'e1' })
      expect(t).toMatchObject({ status: 'pending_approval', estimatedReviewHours: 24, applicationId: 'a1' })
    })
  })

  // ── 注册中心写面 ──────────────────────────────────────────────
  // 三条端点的响应形状互不相同（POST 回 id、PUT 不回 id、DELETE 不回 expert），
  // 这里按后端实形写死：形状一变，store 的「按 expert.id 定位行」就会误判成功。
  describe('专家注册中心写面', () => {
    it('注册只发非空且不等于 minimal 默认值的键，响应 expert 走同一个归一器', async () => {
      const { client: c, request } = makeClient(flat({
        expert: {
          id: 'exp-9', name: '张三', domains: ['kb'],
          availability: { status: 'busy', current_load: 0, max_concurrent: 3 },
          metrics: { total_consultations: 7 }
        },
        created: true, id: 'exp-9'
      }))
      const out = await createAllianceApi(c).registerExpert({
        ...emptyExpertDraft(), name: '张三', domains: ['kb'], availabilityStatus: 'busy', maxConcurrent: 3
      })
      const cfg = request.mock.calls[0][0]
      expect(cfg).toMatchObject({ url: '/experts', method: 'POST', silent: true })
      expect(cfg.data).toEqual({ name: '张三', domains: ['kb'], availability: { status: 'busy', max_concurrent: 3 } })
      expect(out).toMatchObject({ created: true, id: 'exp-9' })
      expect(out.expert).toMatchObject({ id: 'exp-9', name: '张三', availability: { status: 'busy', maxConcurrent: 3 } })
      expect(out.expert.metrics.totalConsultations).toBe(7)
    })

    it('编辑走 PUT 到带 id 的路径；后端不回 id，所以定位靠 expert.id', async () => {
      const { client: c, request } = makeClient(flat({ expert: { id: 'exp-9', name: '张三' }, updated: true }))
      const out = await createAllianceApi(c).updateExpert('exp-9', { title: '资深' })
      expect(request.mock.calls[0][0]).toMatchObject({ url: '/experts/exp-9', method: 'PUT', data: { title: '资深' } })
      expect(out.updated).toBe(true)
      expect(out.id).toBe('')
      expect(out.expert.name).toBe('张三')
    })

    it('停用走 DELETE，响应没有 expert 字段故归一为 null 而不是空对象', async () => {
      const { client: c, request } = makeClient(flat({ id: 'exp-9', deleted: true, soft_delete: true, message: 'expert has been soft-deleted' }))
      const out = await createAllianceApi(c).deleteExpert('exp-9')
      expect(request.mock.calls[0][0]).toMatchObject({ url: '/experts/exp-9', method: 'DELETE' })
      expect(request.mock.calls[0][0].data).toBeUndefined()
      expect(out).toEqual({
        expert: null, id: 'exp-9', created: false, updated: false, deleted: true, softDelete: true,
        message: 'expert has been soft-deleted'
      })
    })

    it('已停用的专家 PUT 必 404：冒泡 ApiError，不折叠成 updated=false', async () => {
      const c = { request: vi.fn(async () => ({ status: 200, data: { code: 404, msg: 'expert not found: exp-9', data: null } })) }
      await expect(createAllianceApi(c).updateExpert('exp-9', { title: 'x' })).rejects.toMatchObject({ name: 'ApiError', msg: 'expert not found: exp-9' })
    })

    it('路径参数为空时不发半截 URL', async () => {
      const { client: c, request } = makeClient(flat({}))
      await expect(createAllianceApi(c).updateExpert('', { title: 'x' })).rejects.toThrow(/缺少路径参数/)
      expect(request).not.toHaveBeenCalled()
    })
  })

  it('专家匹配提交 query/domains/limit', async () => {
    const { client: c, request } = makeClient(nested({ experts: [{ expert_id: 'e1', name: 'N', match_score: 0.7 }], total: 1 }))
    const api = createAllianceApi(c)
    const out = await api.searchExperts({ query: '需求分析', domains: ['requirement'], limit: 5 })
    expect(request.mock.calls[0][0].data).toEqual({ query: '需求分析', domains: ['requirement'], limit: 5 })
    expect(out.items[0]).toMatchObject({ id: 'e1', matchScore: 0.7 })
  })

  it('业务错误码冒泡为 ApiError', async () => {
    const c = { request: vi.fn(async () => ({ status: 200, data: { code: 404, msg: 'task not found', data: null } })) }
    const api = createAllianceApi(c)
    await expect(api.getTask('nope')).rejects.toMatchObject({ name: 'ApiError', msg: 'task not found' })
  })

  it('runtime 缺字段时按本地预览处理', async () => {
    const c = { request: vi.fn(async () => nested({ execution_ready: false, mode: 'local_preview', message: '未配置远端' })) }
    const api = createAllianceApi(c)
    expect(await api.getRuntime()).toMatchObject({ simulated: true, schedulerReady: false })
  })

  // http.js 的响应拦截器已经剥掉 {code,msg,data}，真实运行时本层收到的是解过一层的对象。
  // 若 unwrap 按声明层数硬剥，此处会被多剥一层得到 undefined —— 这条测试就是防这个回归。
  it('信封已被 http 拦截器解过一层时不多剥', async () => {
    const stripped = (data, params = {}) => ({ elapsed_ms: 7, params, data })
    const c = { request: vi.fn(async (cfg) => (cfg.url === '/experts' ? { experts: [{ id: 'e1', name: 'N', status: 'active' }], total: 1 } : stripped({ task_id: 't1', status: 'running', progress: 40 }))) }
    const api = createAllianceApi(c)
    expect(await api.getTask('t1')).toMatchObject({ id: 't1', status: 'running', progress: 40 })
    expect(await api.listExperts()).toMatchObject({ total: 1 })
  })

  it('请求标记 silent，错误提示由本模块单点负责', async () => {
    const { client: c, request } = makeClient(nested({ task_id: 't1' }))
    const api = createAllianceApi(c)
    await api.getTask('t1')
    expect(request).toHaveBeenCalledWith(expect.objectContaining({ silent: true }))
  })

  describe('collaborate 单入口', () => {
    const payloadBy = {
      route: { query: 'q', matched_experts: [], routing_decision: { recommended_expert_id: null, reason: '', alternative_ids: [] }, total_scanned: 0, ts: 't' },
      single: { session_id: 's', expert_id: 'e1', expert_name: 'N', question: 'q', answer: { analysis: 'a', solution: 's', references: [], confidence: 0.5 }, created_at: 't' },
      multi: { session_id: 's', question: 'q', experts: [], fused_answer: { summary: 'x', consensus_score: 0, dominant_view: '', alternative_views: [], confidence: 0 }, created_at: 't' },
      debate: { debate_id: 'd', topic: 't', rounds: 1, participants: [], debate_log: [], verdict: { winner: '', summary: '', key_points: [], consensus_level: '' }, created_at: 't' },
      smart: { consultation_id: 'c', question: 'q', intent: 'architecture', matched_expert: null, answer: { analysis: 'a', solution: 's', action_items: [], risk_assessment: {}, references: [], confidence: 0 }, related_experts: [], created_at: 't' },
      algorithm: { analysis_id: 'g', algorithm_description: 'd', input_constraints: null, requirements: null, complexity: {}, feasibility: {}, recommended_experts: [], optimization_suggestions: [], created_at: 't' }
    }

    it('六个模式各自落到自己的原生端点，全部 POST 且静默', async () => {
      for (const mode of Object.keys(payloadBy)) {
        const { client: c, request } = makeClient(flat(payloadBy[mode]))
        await createAllianceApi(c).collaborate(mode, { [mode === 'debate' ? 'topic' : mode === 'algorithm' ? 'algorithm_description' : 'question']: 'x', expertIds: ['e1', 'e2'] })
        const cfg = request.mock.calls[0][0]
        expect(cfg.method).toBe('POST')
        expect(cfg.silent).toBe(true)
        expect(cfg.url).toBe({
          route: '/experts/route',
          single: '/experts/e1/consult',
          multi: '/experts/multi-consult',
          debate: '/experts/debate',
          smart: '/experts/intelligent-consult',
          algorithm: '/experts/algorithm-analysis'
        }[mode])
      }
    })

    it('返回按端点归一，模式各自的形状不混', async () => {
      const route = await createAllianceApi(makeClient(flat(payloadBy.route)).client).collaborate('route', { question: 'q' })
      expect(route).toMatchObject({ mode: 'route', query: 'q', candidates: [], totalScanned: 0 })
      const smart = await createAllianceApi(makeClient(flat(payloadBy.smart)).client).collaborate('smart', { question: 'q' })
      expect(smart).toMatchObject({ mode: 'smart', intent: 'architecture', answer: { analysis: 'a' } })
      const algo = await createAllianceApi(makeClient(flat(payloadBy.algorithm)).client).collaborate('algorithm', { algorithm_description: 'd' })
      expect(algo).toMatchObject({ mode: 'algorithm', description: 'd', complexity: {}, suggestions: [] })
    })

    it('body 由契约生成：camelCase 控件名不会漏进请求', async () => {
      const { client: c, request } = makeClient(flat(payloadBy.multi))
      await createAllianceApi(c).collaborate('multi', { question: 'q', expertIds: ['e1', 'e2'], maxExperts: 99, domain: 'arch' })
      expect(request.mock.calls[0][0].data).toEqual({ question: 'q', expert_ids: ['e1', 'e2'], domain: 'arch', max_experts: 3 })
    })

    // 路由的三条筛选是 route_query 真实消费的嵌套约束，存量两份实现都没发过
    it('路由约束按嵌套 constraints 发出，默认值不占位', async () => {
      const on = makeClient(flat(payloadBy.route))
      await createAllianceApi(on.client).collaborate('route', { question: 'q', min_rating: 4, require_online: true })
      expect(on.request.mock.calls[0][0].data).toEqual({ question: 'q', max_experts: 5, constraints: { min_rating: 4, require_online: true } })

      const off = makeClient(flat(payloadBy.route))
      await createAllianceApi(off.client).collaborate('route', { question: 'q', min_rating: 0, max_response_time: null, require_online: false })
      expect(off.request.mock.calls[0][0].data).toEqual({ question: 'q', max_experts: 5 })
    })

    it('单专家咨询的专家只走路径参数，缺专家就抛错不发请求', async () => {
      const { client: c, request } = makeClient(flat(payloadBy.single))
      const out = await createAllianceApi(c).collaborate('single', { question: 'q', expertIds: ['e7'] })
      expect(request.mock.calls[0][0]).toMatchObject({ url: '/experts/e7/consult', data: { question: 'q' } })
      expect(out).toMatchObject({ mode: 'single', expertId: 'e1', answer: { solution: 's' } })

      const idle = makeClient(flat(payloadBy.single))
      await expect(createAllianceApi(idle.client).collaborate('single', { question: 'q' })).rejects.toThrow(/选择 1 位专家/)
      expect(idle.request).not.toHaveBeenCalled()
    })

    it('未知模式直接抛错，避免向后端发空 body', async () => {
      const { client: c, request } = makeClient(flat({}))
      await expect(createAllianceApi(c).collaborate('telepathy', { question: 'q' })).rejects.toThrow(/未知协作模式/)
      expect(request).not.toHaveBeenCalled()
    })
  })

  // ── 协作图谱：8 个 flat 端点。store 测试整体 mock 了本层，路径/参数/信封只能在这里守 ──
  describe('协作图谱', () => {
    const overview = {
      nodes: [
        {
          id: 'e1', label: '甲', node_type: 'expert',
          properties: { title: '架构师', domains: ['architecture'], avg_rating: 4.6, status: 'online' }
        },
        { id: 'domain-architecture', label: 'architecture', node_type: 'domain', properties: { domain: 'architecture' } }
      ],
      edges: [
        { source: 'e1', target: 'domain-architecture', edge_type: 'has_domain', weight: 1, properties: {} },
        {
          source: 'e1', target: 'e2', edge_type: 'collaborates_with', weight: 0.33,
          properties: { shared_domains: ['architecture'], similarity: 0.33 }
        }
      ],
      stats: { node_count: 2, edge_count: 2, expert_count: 1, domain_count: 1, avg_degree: 2, density: 1 },
      built_at: '2026-09-23T00:00:00Z',
      version: 7
    }

    it('八个方法各自的路径与动词精确对齐路由表', async () => {
      const probes = [
        ['graphOverview', [], '/expert-graph'],
        ['graphStats', [], '/expert-graph/stats'],
        ['graphNeighbors', ['e1'], '/expert-graph/neighbors/e1'],
        ['graphCollaborators', ['e1'], '/expert-graph/collaborators/e1'],
        ['graphPath', ['e1', 'domain-ai'], '/expert-graph/path/e1/domain-ai'],
        ['graphCommunities', [], '/expert-graph/communities'],
        ['optimalTeam', [{ requiredDomains: ['ai'] }], '/expert-graph/optimal-team'],
        ['rebuildGraph', [], '/expert-graph/rebuild']
      ]
      const methods = []
      for (const [name, args, url] of probes) {
        const { client: c, request } = makeClient(flat({}))
        await createAllianceApi(c)[name](...args)
        const cfg = request.mock.calls[0][0]
        expect(cfg.url, name).toBe(url)
        expect(cfg.silent, name).toBe(true)
        methods.push(cfg.method)
      }
      expect(methods).toEqual(['GET', 'GET', 'GET', 'GET', 'GET', 'GET', 'POST', 'POST'])
    })

    it('图概览归一出节点/边/统计，version 保持数值，域节点 id 前缀不被改写', async () => {
      const { client: c } = makeClient(flat(overview))
      const g = await createAllianceApi(c).graphOverview()
      expect(g.version).toBe(7)
      expect(g.nodes[0]).toMatchObject({
        id: 'e1', nodeType: 'expert', title: '架构师', domains: ['architecture'], avgRating: 4.6, availability: 'online'
      })
      // 域节点没有 avg_rating，归一为 null 而不是 0：0 会被界面当成"评分为零的专家"
      expect(g.nodes[1].id).toBe('domain-architecture')
      expect(g.nodes[1].avgRating).toBe(null)
      expect(g.edges[1]).toMatchObject({ edgeType: 'collaborates_with', weight: 0.33, sharedDomains: ['architecture'] })
      expect(g.stats).toMatchObject({ nodeCount: 2, edgeCount: 2, domainCount: 1, density: 1 })
      expect(g.builtAt).toBe('2026-09-23T00:00:00Z')
    })

    it('统计面的中心性清单与聚类系数逐字段落地', async () => {
      const { client: c } = makeClient(flat({
        total_nodes: 2, total_edges: 2, expert_nodes: 1, domain_nodes: 1,
        collaboration_edges: 1, domain_edges: 1, avg_clustering_coefficient: 0.25,
        connected_components: 1, largest_component_size: 2,
        top_centrality_experts: [{ id: 'e1', name: '甲', degree: 2, degree_centrality: 0.5, betweenness: 1 }],
        density: 0.5, ts: 't'
      }))
      const s = await createAllianceApi(c).graphStats()
      expect(s).toMatchObject({
        totalNodes: 2, collaborationEdges: 1, domainEdges: 1,
        avgClusteringCoefficient: 0.25, largestComponentSize: 2, density: 0.5
      })
      expect(s.topCentralityExperts[0]).toEqual({ id: 'e1', name: '甲', degree: 2, degreeCentrality: 0.5, betweenness: 1 })
    })

    it('limit 只有非缺省正整数才进查询串，截断事实取 total_collaborators', async () => {
      const payload = {
        expert_id: 'e1',
        collaborators: [{ id: 'e2', name: '乙', collaboration_weight: 0.4, shared_domains: ['ai'], collaboration_rank: 1 }],
        total_collaborators: 9
      }
      for (const [given, want] of [
        [undefined, undefined], [10, undefined], [20, { limit: 20 }],
        [0, undefined], ['abc', undefined], [7.8, { limit: 7 }]
      ]) {
        const { client: c, request } = makeClient(flat(payload))
        const out = await createAllianceApi(c).graphCollaborators('e1', given)
        expect(request.mock.calls[0][0].params, `limit=${given}`).toEqual(want)
        expect(out.totalCollaborators).toBe(9)
        expect(out.collaborators[0]).toEqual({ rank: 1, id: 'e2', name: '乙', collaborationWeight: 0.4, sharedDomains: ['ai'] })
      }
    })

    it('邻居面保留 direction 与 shared_domains，这是边的唯一出处', async () => {
      const { client: c } = makeClient(flat({
        node_id: 'e1', node_label: '甲', node_type: 'expert',
        neighbors: [{
          id: 'domain-ai', label: 'ai', node_type: 'domain', edge_type: 'has_domain',
          weight: 1, direction: 'out', properties: {}
        }],
        neighbor_count: 1
      }))
      const n = await createAllianceApi(c).graphNeighbors('e1')
      expect(n).toMatchObject({
        nodeId: 'e1', neighborCount: 1,
        neighbors: [{ id: 'domain-ai', edgeType: 'has_domain', direction: 'out', weight: 1 }]
      })
    })

    it('不可达是 200 + found:false，节点 id 里的斜杠要编码后再进路径', async () => {
      const { client: c, request } = makeClient(flat({
        source: 'e1', target: 'e9', path: [], path_length: 0, total_weight: 0, found: false
      }))
      const p = await createAllianceApi(c).graphPath('e1', 'e9')
      expect(p).toMatchObject({ found: false, path: [], pathLength: 0, totalWeight: 0 })
      expect(request).toHaveBeenCalledWith(expect.objectContaining({ url: '/expert-graph/path/e1/e9' }))

      const slash = makeClient(flat({ source: 'a/b', target: 'c', path: [], path_length: 0, total_weight: 0, found: false }))
      await createAllianceApi(slash.client).graphPath('a/b', 'c')
      expect(slash.request.mock.calls[0][0].url).toBe('/expert-graph/path/a%2Fb/c')
    })

    it('社区面回传的算法名与收敛标志进界面口径', async () => {
      const { client: c } = makeClient(flat({
        communities: [{
          community_id: '0', size: 2, member_ids: ['e1', 'e2'], member_labels: ['甲', '乙'],
          internal_edges: 1, external_edges: 0, modularity_contribution: 0.1
        }],
        total_communities: 1, modularity: 0.4, algorithm: 'label_propagation', iterations: 3, converged: true
      }))
      const g = await createAllianceApi(c).graphCommunities()
      expect(g).toMatchObject({
        totalCommunities: 1, modularity: 0.4, algorithm: 'label_propagation', iterations: 3, converged: true,
        communities: [{ communityId: '0', size: 2, memberLabels: ['甲', '乙'], internalEdges: 1, modularityContribution: 0.1 }]
      })
    })

    it('组队请求体由契约生成：snake_case、缺省值不占位、goal 让位显式需求、constraints 永不发送', async () => {
      const { client: c, request } = makeClient(flat({
        team_id: 'team-1',
        team_members: [{ id: 'e1', name: '甲', role: 'domain_lead', covered_domains: ['ai'], match_score: 0.9, avg_rating: 4.6 }],
        coverage: { required_total: 1, covered_count: 1, coverage_ratio: 1, missing_skills: [], missing_domains: [] },
        team_score: 0.9, selection_strategy: 'weighted_set_cover_greedy', created_at: 't'
      }))
      const out = await createAllianceApi(c).optimalTeam({
        requiredSkills: [], requiredDomains: ['ai'], maxMembers: 5, minRating: 4,
        goal: '为数据仓库组队', constraints: { budget: 1 }
      })
      expect(request.mock.calls[0][0].data).toEqual({ required_domains: ['ai'] })
      expect(out).toMatchObject({
        teamId: 'team-1', teamScore: 0.9, strategy: 'weighted_set_cover_greedy',
        members: [{ role: 'domain_lead', coveredDomains: ['ai'], matchScore: 0.9 }],
        coverage: { requiredTotal: 1, coveredCount: 1, coverageRatio: 1 }
      })
    })

    it('重建是写操作，返回新版本与耗时', async () => {
      const { client: c, request } = makeClient(flat({
        rebuilt: true, previous_version: 7, new_version: 8, node_count: 2, edge_count: 2,
        expert_count: 1, built_at: 't', duration_ms: 12
      }))
      const r = await createAllianceApi(c).rebuildGraph()
      expect(request.mock.calls[0][0]).toMatchObject({ url: '/expert-graph/rebuild', method: 'POST', silent: true })
      expect(r).toMatchObject({ rebuilt: true, previousVersion: 7, newVersion: 8, nodeCount: 2, durationMs: 12 })
    })

    it('路径参数缺失即抛错不发请求，节点 id 不会被拼成空段', async () => {
      const { client: c, request } = makeClient(flat({}))
      const api = createAllianceApi(c)
      await expect(api.graphNeighbors('')).rejects.toThrow(/缺少路径参数/)
      await expect(api.graphCollaborators(null)).rejects.toThrow(/缺少路径参数/)
      await expect(api.graphPath('e1')).rejects.toThrow(/缺少路径参数/)
      expect(request).not.toHaveBeenCalled()
    })
  })

  // ── 会话面：experts_session.rs 的 11 个 (path, method) 对 ──────────────
  // 这里守的是"发出去的字节"与"读回来的键"：后端字段名写错不报错，只被 serde 静默丢弃，
  // 所以请求体必须逐键断言（用 toEqual 而不是 toMatchObject），缺一键和多一键都要红。
  describe('会话面', () => {
    const sessionWire = {
      id: 's-1', title: '数据仓库选型', expert_ids: ['e1'], user_id: 'anonymous',
      session_type: 'multi', status: 'active', topic: '选型',
      tags: ['dw'], metadata: { budget: 3 }, messages: [{
        id: 'm1', session_id: 's-1', role: 'user', sender_id: '', sender_name: '',
        content: '选哪个？', msg_type: 'text', attachments: [], rating: null, created_at: '2026-09-20T00:00:00Z'
      }],
      created_at: '2026-09-20T00:00:00Z', last_active_at: '2026-09-21T00:00:00Z', archived_at: null
    }

    it('十一个方法各自的路径与动词精确对齐路由表', async () => {
      const probes = [
        ['listSessions', [{}], '/experts/sessions', 'GET'],
        ['sessionStats', [], '/experts/sessions/stats', 'GET'],
        ['createSession', [{ title: 'x' }], '/experts/sessions', 'POST'],
        ['getSession', ['s-1'], '/experts/sessions/s-1', 'GET'],
        ['updateSession', ['s-1', { status: 'closed' }], '/experts/sessions/s-1', 'PUT'],
        ['deleteSession', ['s-1'], '/experts/sessions/s-1', 'DELETE'],
        ['appendSessionMessage', ['s-1', { role: 'user', content: 'hi' }], '/experts/sessions/s-1/messages', 'POST'],
        ['sessionSimilarSearch', ['s-1', { query: 'q' }], '/experts/sessions/s-1/similar-search', 'POST'],
        ['exportSession', ['s-1'], '/experts/sessions/s-1/export', 'GET'],
        ['archiveSession', ['s-1'], '/experts/sessions/s-1/archive', 'POST'],
        ['semanticSearch', [{ query: 'q' }], '/experts/semantic-search', 'POST']
      ]
      for (const [name, args, url, method] of probes) {
        const { client: c, request } = makeClient(flat({ ...sessionWire, sessions: [] }))
        await createAllianceApi(c)[name](...args)
        const cfg = request.mock.calls[0][0]
        expect(cfg.url, name).toBe(url)
        expect(cfg.method, name).toBe(method)
        // silent 是"错误只在页面呈现一处"的前提，任何一条漏掉都会双重弹提示
        expect(cfg.silent, name).toBe(true)
      }
    })

    it('列表查询串：空白值一律不发，page/page_size 钳位后由后端认识的键名上送', async () => {
      const { client: c, request } = makeClient(flat({ sessions: [], total: 0, page: 1, page_size: 20 }))
      const api = createAllianceApi(c)
      await api.listSessions({})
      expect(request.mock.calls[0][0].params).toEqual({})

      await api.listSessions({
        status: '', sessionType: 'multi', expertId: '  ', userId: 'u1', search: '  架构  ',
        page: 0, pageSize: 999
      })
      expect(request.mock.calls[1][0].params).toEqual({
        session_type: 'multi', user_id: 'u1', search: '架构', page: 1, page_size: 200
      })
      // 后端把 ?status= 当有效过滤（Some("") 参与比较），发空串等于把结果清成空表；
      // limit 只是 page_size 的别名，两套键同发会互相覆盖，所以这里永不出现
      const seen = Object.keys(request.mock.calls[1][0].params)
      expect(seen).not.toContain('limit')
      expect(seen).not.toContain('status')
      expect(seen).not.toContain('expert_id')

      const page2 = makeClient(flat({ sessions: [], total: 0, page: 3, page_size: 50 }))
      const out = await createAllianceApi(page2.client).listSessions({ page: 3, pageSize: 50 })
      expect(page2.request.mock.calls[0][0].params).toEqual({ page: 3, page_size: 50 })
      expect(out).toEqual({ items: [], total: 0, page: 3, pageSize: 50 })
    })

    it('列表项与详情共用一份归一：message_count 与 messages 两种形态都落成同一个口径', async () => {
      const { client: c } = makeClient(flat({
        sessions: [{
          id: 's-1', title: 'A', expert_ids: ['e1'], user_id: '', session_type: 'single', status: 'active',
          topic: '', message_count: 7, tags: [], metadata: {}, created_at: 't', last_active_at: 't', archived_at: null
        }],
        total: 1, page: 1, page_size: 20
      }))
      const listed = await createAllianceApi(c).listSessions({})
      // session_to_list_view 剥掉 messages，归一后必须为空数组而不是 undefined，界面才能直接 v-for
      expect(listed.items[0].messages).toEqual([])
      expect(listed.items[0].messageCount).toBe(7)
      expect(listed.items[0].archived).toBe(false)

      const { client: d } = makeClient(flat(sessionWire))
      const one = await createAllianceApi(d).getSession('s-1')
      // 详情是整份 ExpertSession：没有 message_count 这个键，条数只能由 messages 推出
      expect(one.messageCount).toBe(1)
      expect(one.messages[0]).toMatchObject({ id: 'm1', role: 'user', content: '选哪个？', rating: null })
      expect(one.metadata).toEqual([{ key: 'budget', value: 3 }])
      expect(one.sessionType).toBe('multi')
    })

    it('创建请求体：缺省值与空白字段都不占位', async () => {
      const { client: c, request } = makeClient(flat(sessionWire))
      const api = createAllianceApi(c)
      await api.createSession({
        title: '  数据仓库选型  ', sessionType: 'single', topic: '   ', expertIds: [], tags: [], metadata: {}
      })
      expect(request.mock.calls[0][0].data).toEqual({ title: '数据仓库选型' })

      await api.createSession({
        sessionType: 'multi', expertIds: ['e1', '', 'e2'], tags: ['dw'], userId: 'u1', topic: '选型', metadata: { k: 'v' }
      })
      expect(request.mock.calls[1][0].data).toEqual({
        expert_ids: ['e1', 'e2'], user_id: 'u1', session_type: 'multi', topic: '选型', tags: ['dw'], metadata: { k: 'v' }
      })
      // session_type 的缺省就是 single：同值回写没有信息量，也与 dispatchPatch 同一口径
      expect(request.mock.calls[0][0].data.session_type).toBeUndefined()
    })

    it('更新是差分直传：api 层不补键、不改名，路径参数里的斜杠要编码', async () => {
      const { client: c, request } = makeClient(flat(sessionWire))
      const api = createAllianceApi(c)
      await api.updateSession('s/1', { status: 'closed', metadata: { a: '1' } })
      expect(request.mock.calls[0][0].url).toBe('/experts/sessions/s%2F1')
      expect(request.mock.calls[0][0].data).toEqual({ status: 'closed', metadata: { a: '1' } })
      // 空 patch 由 store 挡在发出之前；这里若出现 data: {} 说明差分逻辑漏到了 api 层
      expect(Object.keys(request.mock.calls[0][0].data).length).toBe(2)
    })

    it('追加消息：role/content 必填上送，msg_type 等于缺省则不发，0 分与无分可区分', async () => {
      const { client: c, request } = makeClient(flat({
        id: 'm2', session_id: 's-1', role: 'expert', sender_id: '', sender_name: '李',
        content: '选 A', msg_type: 'markdown', attachments: [], rating: 0, created_at: 't'
      }))
      const msg = await createAllianceApi(c).appendSessionMessage('s-1', {
        role: 'expert', content: '选 A', msgType: 'markdown', senderName: '李', senderId: '', rating: 0
      })
      expect(request.mock.calls[0][0].data).toEqual({
        role: 'expert', content: '选 A', sender_name: '李', msg_type: 'markdown', rating: 0
      })
      expect(msg.rating).toBe(0)
      expect(msg.msgType).toBe('markdown')

      const plain = makeClient(flat({ id: 'm3', role: 'user', content: '', rating: null }))
      const m2 = await createAllianceApi(plain.client).appendSessionMessage('s-1', { role: 'user', content: '' })
      expect(plain.request.mock.calls[0][0].data).toEqual({ role: 'user', content: '' })
      // rating 是 Option<u8>：缺失/null 归 null，界面据此区分"没评分"和"0 分"
      expect(m2.rating).toBe(null)
    })

    it('字面相似检索：与后端缺省同值的 top_k/min_score 不进请求体', async () => {
      const { client: c, request } = makeClient(flat({
        query: '架构', session_id: 's-1', total_found: 9,
        results: [{ message: { id: 'm1', role: 'user', content: '架构怎么定' }, similarity_score: 0.4, rank: 1 }]
      }))
      const api = createAllianceApi(c)
      await api.sessionSimilarSearch('s-1', { query: '  架构  ', topK: 5, minScore: 0.1 })
      expect(request.mock.calls[0][0].data).toEqual({ query: '架构' })
      await api.sessionSimilarSearch('s-1', { query: '架构', topK: 3, minScore: 0.5 })
      expect(request.mock.calls[1][0].data).toEqual({ query: '架构', top_k: 3, min_score: 0.5 })

      const out = await api.sessionSimilarSearch('s-1', { query: '架构' })
      expect(out).toMatchObject({ query: '架构', sessionId: 's-1', totalFound: 9 })
      expect(out.results[0]).toEqual({
        message: {
          id: 'm1', role: 'user', senderId: '', senderName: '', content: '架构怎么定',
          msgType: '', attachments: [], rating: null, createdAt: ''
        },
        similarityScore: 0.4, rank: 1
      })
    })

    it('全域检索没有 min_score 这个字段，发出去只会被 serde 丢弃', async () => {
      const { client: c, request } = makeClient(flat({
        query: 'q', total_sessions_scanned: 12, total_messages_scanned: 88,
        results: [{ session_id: 's-1', session_title: 'A', message: { id: 'm1', content: 'c' }, similarity_score: 0.2 }]
      }))
      await createAllianceApi(c).semanticSearch({
        query: 'q', topK: 10, minScore: 0.9, sessionType: 'multi', expertId: 'e1'
      })
      expect(request.mock.calls[0][0].data).toEqual({ query: 'q', session_type: 'multi', expert_id: 'e1' })
      expect(Object.keys(request.mock.calls[0][0].data).sort()).toEqual(['expert_id', 'query', 'session_type'])

      const out = await createAllianceApi(c).semanticSearch({ query: 'q' })
      // top_k 等于后端缺省 10 时不发；score 门槛是后端的 > 0.0，前端无权配
      expect(request.mock.calls[1][0].data).toEqual({ query: 'q' })
      expect(out).toMatchObject({ totalSessionsScanned: 12, totalMessagesScanned: 88 })
      expect(out.results[0]).toMatchObject({ sessionId: 's-1', sessionTitle: 'A', similarityScore: 0.2 })
    })

    it('统计面：session_type_distribution 是 map，归一成数组才不会漏类型', async () => {
      const { client: c } = makeClient(flat({
        total_sessions: 4, active_sessions: 3, archived_sessions: 1, closed_sessions: 0,
        total_messages: 20, avg_messages_per_session: 5.0, avg_session_duration_minutes: 0,
        sessions_today: 1, top_experts_by_sessions: [{ expert_id: 'e1', count: 3 }],
        session_type_distribution: { single: 3, multi: 1 }, ts: 't'
      }))
      const s = await createAllianceApi(c).sessionStats()
      expect(s.typeDistribution).toEqual([{ type: 'single', count: 3 }, { type: 'multi', count: 1 }])
      expect(s.topExpertsBySessions).toEqual([{ expertId: 'e1', count: 3 }])
      // 无数据时后端给 0.0，归一化不得把它伪造成 null
      expect(s.avgSessionDurationMinutes).toBe(0)
      expect(s).toMatchObject({ totalSessions: 4, activeSessions: 3, archivedSessions: 1, closedSessions: 0, sessionsToday: 1 })
    })

    it('导出保留 downloadUrl 为 null，归档与删除只回后端给的那几个键', async () => {
      const { client: c, request } = makeClient(flat({
        session_id: 's-1', format: 'json', exported_at: '2026-09-21T00:00:00Z',
        content: sessionWire, download_url: null, message_count: 1
      }))
      const ex = await createAllianceApi(c).exportSession('s-1')
      expect(request.mock.calls[0][0].method).toBe('GET')
      // 恒 null（:586 写死）：归一成 '' 会让界面画出个点了没反应的假链接
      expect(ex.downloadUrl).toBe(null)
      expect(ex.content.messages[0].content).toBe('选哪个？')
      expect(ex).toMatchObject({ sessionId: 's-1', format: 'json', messageCount: 1 })

      const { client: a } = makeClient(flat({ session_id: 's-1', status: 'archived', archived_at: 't2', message_count: 1 }))
      expect(await createAllianceApi(a).archiveSession('s-1'))
        .toEqual({ sessionId: 's-1', status: 'archived', archivedAt: 't2', messageCount: 1 })

      const { client: d, request: rd } = makeClient(flat({ deleted: true, session_id: 's-1' }))
      expect(await createAllianceApi(d).deleteSession('s-1')).toEqual({ deleted: true, sessionId: 's-1' })
      expect(rd.mock.calls[0][0].data).toBeUndefined()
    })

    it('七个带 :id 的端点缺 id 即抛错，不会拼出 /experts/sessions/ 这种空段', async () => {
      const { client: c, request } = makeClient(flat({}))
      const api = createAllianceApi(c)
      const ids = ['', null, undefined]
      for (const id of ids) {
        await expect(api.getSession(id)).rejects.toThrow(/sessionDetail 缺少路径参数/)
        await expect(api.updateSession(id, { status: 'closed' })).rejects.toThrow(/sessionUpdate 缺少路径参数/)
        await expect(api.deleteSession(id)).rejects.toThrow(/sessionDelete 缺少路径参数/)
        await expect(api.appendSessionMessage(id, { role: 'user', content: 'x' })).rejects.toThrow(/sessionMessages 缺少路径参数/)
        await expect(api.sessionSimilarSearch(id, { query: 'q' })).rejects.toThrow(/sessionSimilarSearch 缺少路径参数/)
        await expect(api.exportSession(id)).rejects.toThrow(/sessionExport 缺少路径参数/)
        await expect(api.archiveSession(id)).rejects.toThrow(/sessionArchive 缺少路径参数/)
      }
      expect(request).not.toHaveBeenCalled()
    })
  })

  describe('编排面五个方法：请求由契约拼装，分页参数不得在 api 层丢掉', () => {
    it('五个 URL/动词逐字对齐端点表，请求体与查询串都取自 orchestration.js', async () => {
      const calls = []
      const c = { request: vi.fn(async (cfg) => { calls.push(cfg); return flat({}) }) }
      const api = createAllianceApi(c)
      await api.orchestrate({ task: '  做一件事  ', taskType: '', maxExperts: 0, expertIds: ['', '  ', 'e1'] })
      await api.generateOrchPlan({ task: 't', fusionStrategy: 'voting' })
      await api.executeOrchPlan({ planId: 'plan-1', stepIds: [] })
      await api.getOrchStats()
      await api.getOrchHistory({ page: 2, pageSize: 5000, status: '  ' })
      expect(calls.map((x) => `${x.method} ${x.url}`)).toEqual([
        'POST /experts/orchestrate',
        'POST /experts/plan/generate',
        'POST /experts/plan/execute',
        'GET /experts/orchestration/stats',
        'GET /experts/orchestration/history'
      ])
      // task 去空白；空 expert_ids 项被剔除非空项保留；max_experts=0 必须仍在（0 与不发是两种结果）
      expect(calls[0].data).toEqual({ task: '做一件事', max_experts: 0, expert_ids: ['e1'] })
      expect(calls[1].data).toEqual({ task: 't', fusion_strategy: 'voting' })
      // step_ids 的空数组必须发出去，plan_id 无 serde default 所以恒发
      expect(calls[2].data).toEqual({ plan_id: 'plan-1', step_ids: [] })
      expect(calls[3].data).toBeUndefined()
      expect(calls[3].params).toBeUndefined()
      // 分页由 orchHistoryQuery 夹取：越界回 200，空白 status 不发
      expect(calls[4].params).toEqual({ page: 2, page_size: 200 })
    })

    it('五个响应都是 flat 信封，编排结果按摊平后的模型返回', async () => {
      const { client: c } = makeClient(flat({
        orchestration_id: 'exec-1', task: 't', task_type: 'general', experts: [],
        plan: { plan_id: 'plan-1', steps: [] },
        execution: { status: 'completed', steps_completed: 0, steps_total: 0, duration_ms: 12 },
        result: null, created_at: 'c'
      }))
      const api = createAllianceApi(c)
      const out = await api.orchestrate({ task: 't' })
      expect(out.orchestrationId).toBe('exec-1')
      expect(out.planId).toBe('plan-1')
      expect(out.execution.status).toBe('completed')
      expect(out.steps).toEqual([])
      expect(out.result).toBe(null)
    })
  })
})
