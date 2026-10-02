// 联盟模块端点清单唯一来源。
// 每条 path 必须能在 docs/API-REGISTRY.md 找到同名字面量（contract.test.js 守护）；
// nesting 标明该 handler 的信封层数：alliance.rs 全部多套一层 data。

export const ENDPOINTS = Object.freeze({
  runtime: { registry: 'alliance.runtime', method: 'GET', path: '/api/alliance/runtime', nesting: 'nested' },

  tasksList: { registry: 'alliance.tasks.list', method: 'GET', path: '/api/alliance/tasks', nesting: 'nested' },
  taskCreate: { registry: 'alliance.tasks.list', method: 'POST', path: '/api/alliance/tasks', nesting: 'nested' },
  taskDetail: { registry: 'alliance.tasks.detail', method: 'GET', path: '/api/alliance/tasks/:id', nesting: 'nested' },
  taskPause: { registry: 'alliance.tasks.pause', method: 'POST', path: '/api/alliance/tasks/:id/pause', nesting: 'nested' },
  taskResume: { registry: 'alliance.tasks.resume', method: 'POST', path: '/api/alliance/tasks/:id/resume', nesting: 'nested' },
  taskCancel: { registry: 'alliance.tasks.cancel', method: 'POST', path: '/api/alliance/tasks/:id/cancel', nesting: 'nested' },
  taskRetry: { registry: 'alliance.tasks.retry', method: 'POST', path: '/api/alliance/tasks/:id/retry', nesting: 'nested' },
  // 标记完成：本地可逆（Completed→Running 回 toggled:false），远程任务单向、已完成再点回 409
  taskToggleDone: { registry: 'alliance.tasks.toggle_done', method: 'PUT', path: '/api/alliance/tasks/:id/toggle-done', nesting: 'nested' },
  taskPlan: { registry: 'alliance.tasks.plan', method: 'GET', path: '/api/alliance/tasks/:id/plan', nesting: 'nested' },
  taskExecutionStatus: { registry: 'alliance.tasks.execution_status', method: 'GET', path: '/api/alliance/tasks/:id/execution-status', nesting: 'nested' },
  taskNodes: { registry: 'alliance.tasks.nodes', method: 'GET', path: '/api/alliance/tasks/:id/nodes', nesting: 'nested' },
  taskLogs: { registry: 'alliance.tasks.logs', method: 'GET', path: '/api/alliance/tasks/:id/logs', nesting: 'nested' },
  taskLogStream: { registry: 'alliance.tasks.logs_stream', method: 'GET', path: '/api/alliance/tasks/:id/logs/stream', nesting: 'nested' },

  // ── T4 事件总线对外出口（experts_streams.rs；与上面的任务日志流为独立通道）──
  // SSE 事件帧流：不经 http 工厂，由 composables/useAllianceEventStream.js 以 fetch 直连。
  allianceEventStream: { registry: 'alliance.events.stream', method: 'GET', path: '/api/alliance/events/stream', nesting: 'flat' },
  // webhook CRUD（POST/GET /api/alliance/events/webhooks、DELETE .../:id）后端已真实落地并 E2E 验证，
  // 但属运维管理面，本前端模块不挂 UI，归 contract.test.js 的 DOC_UNREGISTERED_PENDING「欠登记」，不在此登记。
  taskDag: { registry: 'alliance.tasks.dag', method: 'GET', path: '/api/alliance/tasks/:id/dag', nesting: 'nested' },
  taskFusion: { registry: 'alliance.tasks.fusion', method: 'GET', path: '/api/alliance/tasks/:id/fusion-result', nesting: 'nested' },
  expertSearch: { registry: 'alliance.experts.search', method: 'POST', path: '/api/alliance/experts/search', nesting: 'nested' },

  // registry 为 docs/API-REGISTRY.md 的行 ID；该文档按路径登记，同一路径的 GET/POST 共用一行。
  expertsList: { registry: 'experts.registry.list', method: 'GET', path: '/api/experts', nesting: 'flat' },
  expertRegister: { registry: 'experts.registry.register', method: 'POST', path: '/api/experts', nesting: 'flat' },
  expertDetail: { registry: 'experts.registry.detail', method: 'GET', path: '/api/experts/:id', nesting: 'flat' },
  // 维护两条与 GET 共用 ANY 行 experts.registry.detail。语义都在 contract/registry.js：
  // PUT 是合并式且要求 enabled，DELETE 是软删且无反向端点。
  expertUpdate: { registry: 'experts.registry.detail', method: 'PUT', path: '/api/experts/:id', nesting: 'flat' },
  expertDelete: { registry: 'experts.registry.detail', method: 'DELETE', path: '/api/experts/:id', nesting: 'flat' },
  expertsStats: { registry: 'experts.registry.stats', method: 'GET', path: '/api/experts/stats', nesting: 'flat' },
  // 能力目录：只统计 enabled 专家，后端按 capability id 升序输出（experts_registry.rs:445-469）
  expertCapabilities: { registry: 'experts.registry.capabilities', method: 'GET', path: '/api/experts/capabilities', nesting: 'flat' },
  // 单专家派生指标：对已停用的专家同样返回 404（experts_registry.rs:565-566）
  expertMetrics: { registry: 'experts.registry.detail_metrics', method: 'GET', path: '/api/experts/:id/metrics', nesting: 'flat' },
  sessionsList: { registry: 'experts.session.list', method: 'GET', path: '/api/experts/sessions', nesting: 'flat' },
  // ── 会话面（handler 源 alliance/experts_session.rs:154-624，信封一律 flat）──
  // detail/update/delete 三条共用 actuator 的 ANY 行 experts.session.detail，
  // 该行是本次补登记的产物：路由早已挂载在 modules.rs:160，注册表却漏了它。
  sessionCreate: { registry: 'experts.session.create', method: 'POST', path: '/api/experts/sessions', nesting: 'flat' },
  sessionStats: { registry: 'experts.session.stats', method: 'GET', path: '/api/experts/sessions/stats', nesting: 'flat' },
  sessionDetail: { registry: 'experts.session.detail', method: 'GET', path: '/api/experts/sessions/:id', nesting: 'flat' },
  sessionUpdate: { registry: 'experts.session.detail', method: 'PUT', path: '/api/experts/sessions/:id', nesting: 'flat' },
  sessionDelete: { registry: 'experts.session.detail', method: 'DELETE', path: '/api/experts/sessions/:id', nesting: 'flat' },
  sessionMessages: { registry: 'experts.session.messages', method: 'POST', path: '/api/experts/sessions/:id/messages', nesting: 'flat' },
  sessionSimilarSearch: { registry: 'experts.session.similar_search', method: 'POST', path: '/api/experts/sessions/:id/similar-search', nesting: 'flat' },
  sessionExport: { registry: 'experts.session.export', method: 'GET', path: '/api/experts/sessions/:id/export', nesting: 'flat' },
  sessionArchive: { registry: 'experts.session.archive', method: 'POST', path: '/api/experts/sessions/:id/archive', nesting: 'flat' },
  semanticSearch: { registry: 'experts.session.semantic_search', method: 'POST', path: '/api/experts/semantic-search', nesting: 'flat' },
  dispatcherStatus: { registry: 'experts.dispatch.status', method: 'GET', path: '/api/experts/dispatcher/status', nesting: 'flat' },
  // 合并式配置读写，字段规格与 400 边界见 contract/dispatcher.js
  dispatcherConfig: { registry: 'experts.dispatch.get_config', method: 'GET', path: '/api/experts/dispatcher/config', nesting: 'flat' },
  dispatcherConfigUpdate: { registry: 'experts.dispatch.update_config', method: 'PUT', path: '/api/experts/dispatcher/config', nesting: 'flat' },
  // 分发实跑：与配置面读同一把 dispatcher_config 锁的真实策略引擎（experts_dispatcher.rs:552-607），
  // 会写一条进程内调度记录并发审计事件，且**不改** current_load（只有 reset* 才清零）
  dispatcherRun: { registry: 'experts.dispatch.dispatch', method: 'POST', path: '/api/experts/dispatcher/dispatch', nesting: 'flat' },
  // 负载重置两条（破坏性面）：单专家那条的签名带 Json<ResetBody>，**必须发 JSON 对象**（reason 可选）；
  // 全量那条没有 body 提取器，响应也只有计数与 id 清单。二者对未知 id 都不 404，详见 contract/dispatcher.js
  dispatcherReset: { registry: 'experts.dispatch.reset', method: 'POST', path: '/api/experts/dispatcher/reset/:id', nesting: 'flat' },
  dispatcherResetAll: { registry: 'experts.dispatch.reset_all', method: 'POST', path: '/api/experts/dispatcher/reset-all', nesting: 'flat' },
  // ── 编排面（handler 源 alliance/experts_orchestration.rs:966-973，信封一律 flat）──
  // 出参键集、逐字段来源与"200 里藏失败"的判据都在 contract/orchestration.js。
  orchestrate: { registry: 'experts.orch.orchestrate', method: 'POST', path: '/api/experts/orchestrate', nesting: 'flat' },
  orchPlanGenerate: { registry: 'experts.orch.plan_generate', method: 'POST', path: '/api/experts/plan/generate', nesting: 'flat' },
  orchPlanExecute: { registry: 'experts.orch.plan_execute', method: 'POST', path: '/api/experts/plan/execute', nesting: 'flat' },
  orchStats: { registry: 'experts.orch.stats', method: 'GET', path: '/api/experts/orchestration/stats', nesting: 'flat' },
  orchHistory: { registry: 'experts.orch.history', method: 'GET', path: '/api/experts/orchestration/history', nesting: 'flat' },
  graphOverview: { registry: 'experts.graph.overview', method: 'GET', path: '/api/expert-graph', nesting: 'flat' },  // 协作图谱家族：handler 源 alliance/experts_graph.rs:909-916，全部 flat 信封
  graphStats: { registry: 'experts.graph.stats', method: 'GET', path: '/api/expert-graph/stats', nesting: 'flat' },
  graphNeighbors: { registry: 'experts.graph.neighbors', method: 'GET', path: '/api/expert-graph/neighbors/:id', nesting: 'flat' },
  graphCollaborators: { registry: 'experts.graph.collaborators', method: 'GET', path: '/api/expert-graph/collaborators/:id', nesting: 'flat' },
  graphPath: { registry: 'experts.graph.path', method: 'GET', path: '/api/expert-graph/path/:source/:target', nesting: 'flat' },
  graphCommunities: { registry: 'experts.graph.communities', method: 'GET', path: '/api/expert-graph/communities', nesting: 'flat' },
  optimalTeam: { registry: 'experts.graph.optimal_team', method: 'POST', path: '/api/expert-graph/optimal-team', nesting: 'flat' },
  graphRebuild: { registry: 'experts.graph.rebuild', method: 'POST', path: '/api/expert-graph/rebuild', nesting: 'flat' },
  // ── 图谱节点级 CRUD（N4，管理写面，后端强制 RBAC graph.mutate；画布 U1 留待接线）──
  graphNodeCreate: { registry: 'experts.graph.node_create', method: 'POST', path: '/api/expert-graph/nodes', nesting: 'flat' },
  graphNodeUpdate: { registry: 'experts.graph.node_update', method: 'PUT', path: '/api/expert-graph/nodes/:id', nesting: 'flat' },
  graphNodeDelete: { registry: 'experts.graph.node_delete', method: 'DELETE', path: '/api/expert-graph/nodes/:id', nesting: 'flat' },
  graphEdgeCreate: { registry: 'experts.graph.edge_create', method: 'POST', path: '/api/expert-graph/edges', nesting: 'flat' },
  graphEdgeUpdate: { registry: 'experts.graph.edge_update', method: 'PUT', path: '/api/expert-graph/edges/:seq', nesting: 'flat' },
  graphEdgeDelete: { registry: 'experts.graph.edge_delete', method: 'DELETE', path: '/api/expert-graph/edges/:seq', nesting: 'flat' },
  // ── T2 图 RAG：多跳邻域扩展（读面公开；handler 源 alliance/experts_graph.rs 七-C，权重乘积聚合）──
  graphRagExpand: { registry: 'experts.graph.rag_expand', method: 'POST', path: '/api/expert-graph/rag/expand', nesting: 'flat' },

  // ── 广场交互（预约 / 收藏 / 即时咨询 / 咨询室 / 团队）─────────────
  // handler 源：alliance/experts_ext.rs、alliance/experts_registry.rs，信封一律 flat
  bookingsMine: { registry: 'experts.ext.bookings_mine', method: 'GET', path: '/api/experts/bookings/mine', nesting: 'flat' },
  bookingCreate: { registry: 'experts.ext.bookings_create', method: 'POST', path: '/api/experts/bookings', nesting: 'flat' },
  bookingCancel: { registry: 'experts.ext.bookings_cancel', method: 'PUT', path: '/api/experts/bookings/:id/cancel', nesting: 'flat' },
  expertFavorite: { registry: 'experts.ext.favorite', method: 'POST', path: '/api/experts/:id/favorite', nesting: 'flat' },
  consultNow: { registry: 'experts.registry.consult_now', method: 'POST', path: '/api/experts/:id/consult-now', nesting: 'flat' },
  consultRoom: { registry: 'experts.registry.consult_room', method: 'GET', path: '/api/experts/bookings/:id/consult-room', nesting: 'flat' },
  joinTeam: { registry: 'experts.registry.team', method: 'POST', path: '/api/experts/team', nesting: 'flat' },

  // ── 智能协作（handler 源 experts_collaboration.rs:1856-1868，信封一律 flat）──
  // 入参字段与边界见 contract/collab.js，出参归一化见 model/normalize.js
  expertConsult: { registry: 'experts.collab.consult', method: 'POST', path: '/api/experts/:id/consult', nesting: 'flat' },
  multiConsult: { registry: 'experts.collab.multi_consult', method: 'POST', path: '/api/experts/multi-consult', nesting: 'flat' },
  expertDebate: { registry: 'experts.collab.debate', method: 'POST', path: '/api/experts/debate', nesting: 'flat' },
  expertRoute: { registry: 'experts.collab.route', method: 'POST', path: '/api/experts/route', nesting: 'flat' },
  intelligentConsult: { registry: 'experts.collab.intelligent_consult', method: 'POST', path: '/api/experts/intelligent-consult', nesting: 'flat' },
  algorithmAnalysis: { registry: 'experts.collab.algorithm_analysis', method: 'POST', path: '/api/experts/algorithm-analysis', nesting: 'flat' }
})

// GET /api/experts 的查询参数：键为前端过滤器字段，值为 wire 名，
// 逐一对应 list_experts（experts_registry.rs:247-254）。后端未实现的参数一律不得出现在此，
// 否则会出现"前端以为在筛选、后端其实忽略"的假象。
// 特别注意：后端不读 capability_id，也不读 min_rating——能力目录点一行只能落到 domain 这一层
// 真实过滤（列表侧的 skills 是子串匹配，与结构化能力项不是一个来源）。
export const EXPERT_QUERY_KEYS = Object.freeze({
  domain: 'domain',
  skill: 'skill',
  status: 'status',
  expertType: 'expert_type',
  search: 'search',
  sort: 'sort',
  page: 'page',
  pageSize: 'page_size'
})

/** 过滤器 → 查询串：空值一律不发（`?status=` 会被后端当有效过滤） */
export function expertListQuery(filters = {}) {
  const query = {}
  for (const [local, wire] of Object.entries(EXPERT_QUERY_KEYS)) {
    const v = filters[local]
    if (v === undefined || v === null || v === '') continue
    query[wire] = v
  }
  return query
}

// sort 仅这三个取值生效，其余静默按注册表插入序返回。
export const EXPERT_SORT = Object.freeze(['rating', 'consultations', 'name'])

// 明确禁用清单：前端历史代码曾调用/仍调用，但后端不存在或为桩，迁移期一律不得复活。
export const FORBIDDEN_ENDPOINTS = Object.freeze([
  { path: '/api/alliance/stream', reason: 'Rust 侧不存在，仅 store 与 useSSE 自行假设；日志流用 /api/alliance/tasks/:id/logs/stream' },
  { path: '/api/alliance/stats', reason: 'alliance.rs:1100-1115 为硬编码全零桩，KPI 改用 /api/experts/stats' },
  { path: '/api/expert-graph/overview', reason: '未路由，图概览即 GET /api/expert-graph' },
  { path: '/api/ai/engine/alliance/full', reason: '仅编排器 :3001 提供，非网关 :3080 契约' },
  { path: '/api/ai/engine/alliance/capabilities', reason: '同上，须经编排器回退，不作为联盟模块契约' },
  {
    path: '/api/ai/expert-chat',
    reason: '网关已路由但未登记进 docs/API-REGISTRY.md；其 multi/debate 分支读 results、summary 两个子 handler 从不产出的键，content 恒退化为兜底串。协作一律直连 /api/experts/* 原生端点'
  }
])

// 全维覆盖台账：docs/API-REGISTRY.md 里 experts.* / alliance.* 两域中「已 ready 但模块未挂载」的面。
// contract.test.js 断言 注册表 id 全集 == ENDPOINTS 用到的 id ∪ 本表 id（双向、不许多也不许少），
// 所以后端新增路由时本表不会默默放过——必须先定性为 backlog（真能力，待做）或 rejected（有意不做）。
// verdict=rejected 的每条都要能指出后端证据：重复源 / 零值桩 / 模板桩 / 虚假能力面 / 破坏性动作。
export const UNMOUNTED_ROUTES = Object.freeze([
  { registry: 'alliance.tasks.fusion_alias', verdict: 'rejected', reason: '/fusion 与 /fusion-result 挂的是同一个 get_fusion_result（alliance.rs:1818-1819），模块走 taskFusion 一条源；挂两条会让融合结果有两个刷新入口' },
  { registry: 'alliance.stats', verdict: 'rejected', reason: 'handler 无 State 参数，八个键全为硬编码 0/0.0（alliance.rs:1100-1114），已在 FORBIDDEN_ENDPOINTS 内；平台 KPI 用 experts.registry.stats' },
  { registry: 'experts.registry.metrics', verdict: 'rejected', reason: '与 experts.registry.stats 同出一个 compute_platform_metrics（experts_registry.rs:485 与 :608 都调它），并列只会给同一组数字两个口径' },
  { registry: 'experts.collab.enterprise_consult', verdict: 'rejected', reason: '报告正文是 format! 模板（四阶段 roadmap 固定、payback_months 写死 12、confidence=0.82+…，experts_collaboration.rs:1435-1456，无 LLM），却会往会话表落盘；把模板文当企业咨询结论呈现不诚实' },
  { registry: 'experts.collab.enterprise_analyze', verdict: 'rejected', reason: 'findings/SWOT/recommendations 是按 analysis_type 查表写死的文案，overall_score 是 0.72/0.68/0.75/0.70 字面量，匹配到的专家被 let _experts 丢弃（experts_collaboration.rs:1548-1596）' },
  { registry: 'experts.dispatch.consult', verdict: 'rejected', reason: '答案来自 generate_answer 模板桩（固定中文样板，confidence=0.7+rating/5*0.25，experts_dispatcher.rs:350-353），弱于 /api/experts/:id/consult 的真实 LLM 路径；并列会出现两种咨询结果其中一种是假的' },
  { registry: 'experts.dispatch.multi_consult', verdict: 'rejected', reason: '同上模板桩，且 handler 把 strategy_used 写死为 "best_match_multi"（experts_dispatcher.rs:741）不反映用户配置，界面显示"按 X 策略"即是撒谎' },
  { registry: 'experts.orch.plugins', verdict: 'rejected', reason: '纯硬编码 6 条数组、version 一律 2.0.0，无 registry/config 支撑，声明的 webhook_url/retry_count 无人读取（experts_orchestration.rs:839-920）——挂成"插件清单"是虚假能力面' },
  // 概览仪表盘七个键逐个查过第一源，没有一个是它独有的（2026-09-25 逐字段核对，非按文档措辞判定）：
  // experts_registry.rs:494-555 的 experts_count/today_consultations/avg_rating/domain_breakdown
  // 与已挂载的 stats 走同一个 compute_platform_metrics 的同一份 enabled 集合
  // （:176-216 与 :608-644 两个口径逐字相同）；active_sessions_count 与 experts.session.stats 的
  // active_sessions 数的是同一把 sessions 锁（experts_session.rs:262-282）；两个 top-5 榜
  // 用 /api/experts?sort=rating|consultations 的服务端全库排序取前 5 即得（:292-309 先全集排序再分页）。
  { registry: 'experts.registry.overview', verdict: 'rejected', reason: '七个键全是二手汇总，逐键都能从已挂载面拿到（experts_registry.rs:494-555 对比 :176-216/:608-644、experts_session.rs:262-282、列表 sort 段 :292-309）；并列会出现同一组平台数字两个口径' },
  // 单节点详情与列表元素出自同一份 ExecutionState，键集逐字相同（alliance.rs:1021-1041 vs :981-991，
  // 两侧都是 s.ensure_execution(task_id) 后遍历 exec.nodes，:1019/:976），且 get_node 不额外暴露
  // ExecNode.output ⇒ 详情面没有任何列表面没有的事实。
  { registry: 'alliance.tasks.node', verdict: 'rejected', reason: '九个键与 /nodes 列表元素逐字相同、同源于 exec.nodes（alliance.rs:1021-1041 对比 :981-991），详情面独无一物；接上只会让同一节点有两个取数口径' },
  // 轮询面 14 个键逐个查过第一源（alliance.rs:1686-1705，2026-09-25 逐字段核对）：
  // task_id/status/progress/started_at/completed_at 在已挂载的 taskDetail（:703-715），
  // 五个节点计数在已挂载的 taskExecutionStatus（:944-955），current_node/current_node_name 由
  // /nodes 里 status=="running" 那一条即得；current_phase 就是 current_node_name 本身
  // （:1692 取的是 current.name，与 :1694 同一表达式），estimated_remaining_minutes 是
  // (total-completed)*3 的字面量启发式（:1702），updated_at 是 Utc::now() 的墙上时钟（:1703）。
  { registry: 'alliance.tasks.status_poll', verdict: 'rejected', reason: '十四键全为二手或虚构：计数在 taskExecutionStatus、时间戳在 taskDetail、当前节点可由 /nodes 的 running 行推出，另两个键是 (total-completed)*3 与 Utc::now()（alliance.rs:1702-1703）' }
])

/** 台账按定性分组，供文档与页面提示复用 */
export function unmountedByVerdict(verdict) {
  return UNMOUNTED_ROUTES.filter((x) => x.verdict === verdict).map((x) => x.registry)
}

export function endpointNames() {
  return Object.keys(ENDPOINTS)
}

export function getEndpoint(name) {
  const ep = ENDPOINTS[name]
  if (!ep) throw new Error(`未知端点: ${name}`)
  return ep
}

/**
 * 生成 http 实例可直接使用的相对路径（baseURL 已是 /api）。
 * @param {string} name ENDPOINTS 键
 * @param {Record<string,string|number>} [params] 路径参数，键名与 :placeholder 一致
 */
export function requestPath(name, params = {}) {
  const ep = getEndpoint(name)
  const missing = []
  const filled = ep.path.replace(/:([a-zA-Z_]+)/g, (_, k) => {
    const v = params[k]
    if (v === undefined || v === null || v === '') {
      missing.push(k)
      return ''
    }
    return encodeURIComponent(String(v))
  })
  if (missing.length) throw new Error(`端点 ${name} 缺少路径参数: ${missing.join(', ')}`)
  return filled.replace(/^\/api/, '')
}
