// 联盟 API 层：端点取自 contract/endpoints.js，信封与字段取自 model/normalize.js。
// 本层不做 UI 决策、不 catch 业务错误——错误一律以 ApiError 冒泡给 store。
import { http as defaultHttp } from '@/api'
import { ENDPOINTS, expertListQuery, requestPath, createFavoriteRequest, favoriteRequestHeaders, favoriteQueryBody } from '@/modules/expert-alliance/contract'
import { unwrap, unwrapList, envelopeMeta } from '@/modules/_kernel/envelope.js'
import {
  normAction, normAlgorithmAnalysis, normBooking, normBookingCancel, normBookingList, normCapabilities, normCollaborators, normCommunities,
  normConsultNow, normConsultRoom,
  normDag, normDebate, normDispatch, normDispatcherReset, normDispatcherResetAll, normDispatcherStatus, normExpert, normExpertList, normExpertMetrics, normExpertSearch, normExpertStats, normExpertWrite,
  normExecutionStatus, normFavorite, normFusion, normGraph, normGraphPath, normGraphRebuild, normGraphStats,
  normIntelligentConsult, normLogList,
  normMultiConsult, normNeighbors, normNodeList, normOptimalTeam, normOrchExecution, normOrchHistory, normOrchPlan, normOrchStats, normOrchestration, normPlan, normRouteResult,
  normRuntime, normSemanticSearch, normSession, normSessionArchive, normSessionDelete, normSessionExport,
  normSessionList, normSessionMessage, normSessionStats, normSimilarSearch,
  normSingleConsult,
  normTask, normTaskCreate, normTaskList, normTeamApplication, normToggleDone
} from '@/modules/expert-alliance/model'
import { collabBody, collabMode } from '@/modules/expert-alliance/contract'
import { dispatchResetBody, dispatchRunBody } from '@/modules/expert-alliance/contract'
import { orchHistoryQuery, orchestrateBody, planExecuteBody, planGenerateBody } from '@/modules/expert-alliance/contract'
import { collaboratorQuery, optimalTeamBody, ragExpandBody, ragExpandRows } from '@/modules/expert-alliance/contract'
import {
  appendMessageBody, createSessionBody, semanticSearchBody, sessionListQuery, similarSearchBody
} from '@/modules/expert-alliance/contract'
import { registerBody } from '@/modules/expert-alliance/contract'
import { normWebhook, normWebhookList } from '@/modules/expert-alliance/model'

// 模式 → 端点定义在 contract/collab.js，端点 → 归一化器在此收口，两表由 contract.test.js 对齐。
const NORM_BY_ENDPOINT = {
  expertRoute: normRouteResult,
  expertConsult: normSingleConsult,
  multiConsult: normMultiConsult,
  expertDebate: normDebate,
  intelligentConsult: normIntelligentConsult,
  algorithmAnalysis: normAlgorithmAnalysis
}

function call(httpClient, name, { params = {}, body, query, headers, retryOnPost = false } = {}) {
  const ep = ENDPOINTS[name]
  const url = requestPath(name, params)
  return httpClient.request({
    url,
    method: ep.method,
    data: body,
    params: query,
    headers,
    _retryOnPost: retryOnPost,
    // 错误由本模块的 store/view 单点呈现，避免 http.js 拦截器与页面双重弹提示
    silent: true
  }).then((res) => ({
    payload: unwrap(res, { nesting: ep.nesting }),
    meta: envelopeMeta(res)
  }))
}

export function createAllianceApi(httpClient = defaultHttp) {
  const get = (name, opts) => call(httpClient, name, opts)

  return {
    async listWebhooks(page = 1) {
      const { payload } = await get('webhooksList', { query: { page, page_size: 20 } })
      return normWebhookList(payload)
    },
    async createWebhook(input) {
      const { payload } = await get('webhookCreate', { body: { url: input.url.trim(), event_types: [...input.eventTypes] } })
      return normWebhook(payload?.webhook)
    },
    async deleteWebhook(id) {
      const { payload } = await get('webhookDelete', { params: { id } })
      if (payload?.deleted !== id) throw new Error('删除结果未确认，请刷新后核对')
      return id
    },
    // ── 运行时 ────────────────────────────────────────────────
    async getRuntime() {
      const { payload } = await get('runtime')
      return normRuntime(payload)
    },

    // ── 联盟任务 ──────────────────────────────────────────────
    // 后端忽略分页参数（alliance.rs:657-681），故不传 page/page_size
    async listTasks() {
      const { payload } = await get('tasksList')
      return normTaskList(payload)
    },
    async createTask(input) {
      const body = {
        title: input.title,
        description: input.description ?? '',
        task_type: input.taskType,
        priority: input.priority,
        mode: input.mode,
        fusion_strategy: input.fusionStrategy
      }
      const { payload } = await get('taskCreate', { body })
      return normTaskCreate(payload, body)
    },
    async getTask(id) {
      const { payload } = await get('taskDetail', { params: { id } })
      return normTask(payload)
    },
    async controlTask(id, verb) {
      const map = { pause: 'taskPause', resume: 'taskResume', cancel: 'taskCancel', retry: 'taskRetry' }
      const name = map[verb]
      if (!name) throw new Error(`不支持的任务动作: ${verb}`)
      const { payload } = await get(name, { params: { id } })
      return normAction(payload)
    },
    /**
     * 标记完成 / 重新打开：后端两条分支回的形状不同，归一化里保留 branch，
     * 视图不能假定一定拿得到 current_status（远程分支不给）。
     */
    async toggleTaskDone(id) {
      const { payload } = await get('taskToggleDone', { params: { id } })
      return normToggleDone(payload)
    },
    /** 分发实跑：请求体由 dispatchRunBody 生成，只发 handler 认识的键；503 由 store 呈现为"无可用专家" */
    async runDispatch(form) {
      const { payload } = await get('dispatcherRun', { body: dispatchRunBody(form) })
      return normDispatch(payload)
    },
    async getPlan(id) {
      const { payload } = await get('taskPlan', { params: { id } })
      return normPlan(payload)
    },
    async getExecutionStatus(id) {
      const { payload } = await get('taskExecutionStatus', { params: { id } })
      return normExecutionStatus(payload)
    },
    async getNodes(id) {
      const { payload } = await get('taskNodes', { params: { id } })
      return normNodeList(payload)
    },
    async getLogs(id) {
      const { payload } = await get('taskLogs', { params: { id } })
      return normLogList(payload)
    },
    async getDag(id) {
      const { payload } = await get('taskDag', { params: { id } })
      return normDag(payload)
    },
    async getFusion(id) {
      const { payload } = await get('taskFusion', { params: { id } })
      return normFusion(payload)
    },
    /** 日志流为 SSE：EventSource / fetch-stream 需要可直接使用的 URL（baseURL 已含 /api） */
    taskLogStreamUrl(id) {
      return `/api${requestPath('taskLogStream', { id })}`
    },

    // ── 专家匹配 ──────────────────────────────────────────────
    async searchExperts({ query, domains = [], limit = 10 } = {}) {
      const { payload } = await get('expertSearch', { body: { query, domains, limit } })
      return normExpertSearch(payload)
    },

    // ── 专家注册表（experts 族，扁平信封）──────────────────────
    /** filters 只接受 EXPERT_QUERY_KEYS 的本地字段，空值不发 */
    async listExperts(filters) {
      const { payload } = await get('expertsList', { query: expertListQuery(filters) })
      return normExpertList(payload)
    },
    async getExpert(id) {
      const { payload } = await get('expertDetail', { params: { id } })
      return normExpert(payload)
    },
    async registerExpert(draft) {
      // 请求体只带非空键，缺的字段由后端 ExpertDescriptor::minimal 决定默认值，
      // 前端不自造第二套「合理初值」。
      const { payload } = await get('expertRegister', { body: registerBody(draft) })
      return normExpertWrite(payload)
    },
    /** PUT 为合并式更新：patch 由 contract/expertPatch 生成（只发改动过的键），空 patch 不该发 */
    async updateExpert(id, patch) {
      const { payload } = await call(httpClient, 'expertUpdate', { params: { id }, body: patch })
      return normExpertWrite(payload)
    },
    /** 软删（enabled=false）：后端无再启用端点，响应里的 soft_delete 是它自己声明的口径 */
    async deleteExpert(id) {
      const { payload } = await call(httpClient, 'expertDelete', { params: { id } })
      return normExpertWrite(payload)
    },
    async expertsStats() {
      const { payload } = await get('expertsStats')
      return normExpertStats(payload)
    },
    /** 能力目录：后端已按 capability id 升序，且只统计 enabled 专家 */
    async listExpertCapabilities() {
      const { payload } = await get('expertCapabilities')
      return normCapabilities(payload)
    },
    /** 单专家派生指标：专家不存在**或已停用**都 404，不是空数据 */
    async getExpertMetrics(id) {
      const { payload } = await get('expertMetrics', { params: { id } })
      return normExpertMetrics(payload)
    },
    async listSessions(filters) {
      const { payload } = await get('sessionsList', { query: sessionListQuery(filters) })
      return normSessionList(payload)
    },
    async sessionStats() {
      const { payload } = await get('sessionStats')
      return normSessionStats(payload)
    },
    /**
     * 创建会话。七个字段全 Option，空 body 也合法——但那样入库的是空标题 + 空专家，
     * 所以标题/类型一律由 contract/createSessionBody 归一后再发。响应是完整 ExpertSession（含 messages）。
     */
    async createSession(draft) {
      const { payload } = await call(httpClient, 'sessionCreate', { body: createSessionBody(draft) })
      return normSession(payload)
    },
    /** 详情是**唯一**能一次读到全部 messages 的端点（列表只有 message_count） */
    async getSession(id) {
      const { payload } = await get('sessionDetail', { params: { id } })
      return normSession(payload)
    },
    /** PUT 为合并式更新，patch 由 contract/sessionUpdatePatch 生成（只发改动过的键） */
    async updateSession(id, patch) {
      const { payload } = await call(httpClient, 'sessionUpdate', { params: { id }, body: patch })
      return normSession(payload)
    },
    async deleteSession(id) {
      const { payload } = await call(httpClient, 'sessionDelete', { params: { id } })
      return normSessionDelete(payload)
    },
    /**
     * 追加消息。响应只有那条消息本身（:448），没有会话，故调用方负责把它并进线程。
     * role/content 缺任一个后端 422，问题由 contract/appendMessageProblem 提前挡下。
     */
    async appendSessionMessage(id, draft) {
      const { payload } = await call(httpClient, 'sessionMessages', { params: { id }, body: appendMessageBody(draft) })
      return normSessionMessage(payload)
    },
    /** 会话内字面相似检索（bigram Jaccard），只搜本会话 messages，不跨会话 */
    async sessionSimilarSearch(id, draft) {
      const { payload } = await call(httpClient, 'sessionSimilarSearch', { params: { id }, body: similarSearchBody(draft) })
      return normSimilarSearch(payload)
    },
    /** 全域跨会话检索：后端扫的是全部会话（含已归档），过滤只有 session_type / expert_id */
    async semanticSearch(draft) {
      const { payload } = await call(httpClient, 'semanticSearch', { body: semanticSearchBody(draft) })
      return normSemanticSearch(payload)
    },
    /** 导出：download_url 恒 null，真正的下载在前端做（见 contract/exportText） */
    async exportSession(id) {
      const { payload } = await get('sessionExport', { params: { id } })
      return normSessionExport(payload)
    },
    /** 归档＝强制 status=archived + 盖 archived_at，不是软删，也没有反归档端点（只能 PUT status） */
    async archiveSession(id) {
      const { payload } = await call(httpClient, 'sessionArchive', { params: { id } })
      return normSessionArchive(payload)
    },
    async dispatcherStatus() {
      const { payload } = await get('dispatcherStatus')
      return normDispatcherStatus(payload)
    },
    /**
     * 单专家负载重置。请求体由 dispatchResetBody 兜成"至少是 {}"——handler 的
     * Json<ResetBody> 提取器不接受空体，空体会变成非信封的 400 而页面读不到原因。
     */
    async resetDispatcherLoad(id, reason) {
      const { payload } = await call(httpClient, 'dispatcherReset', { params: { id }, body: dispatchResetBody(reason) })
      return normDispatcherReset(payload)
    },
    /** 全量重置：handler 没有 body 提取器（experts_dispatcher.rs:813-815），故不发体 */
    async resetAllDispatcherLoads() {
      const { payload } = await call(httpClient, 'dispatcherResetAll', {})
      return normDispatcherResetAll(payload)
    },
    // ── 编排面（experts_orchestration.rs:966-973）──────────────────────
    // 三个写面的请求体一律由 contract/orchestration.js 生成：哪些键能省、哪些必须发（plan_id 无
    // serde default，step_ids 空数组与不发是两种语义），在契约里已经判过一遍，此处不再拼装。
    /** 一键编排：真实拓扑 + 模拟步骤内容，来源标注见 contract/orchestration.js */
    async orchestrate(form) {
      const { payload } = await call(httpClient, 'orchestrate', { body: orchestrateBody(form) })
      return normOrchestration(payload)
    },
    /** 只生成计划不执行：plans 落在进程内 HashMap，重启即失 */
    async generateOrchPlan(form) {
      const { payload } = await call(httpClient, 'orchPlanGenerate', { body: planGenerateBody(form) })
      return normOrchPlan(payload)
    },
    /** 执行已有计划：成环时后端回 HTTP 200 + status:"failed"，成败只能读 body（见 orchExecuteOutcome） */
    async executeOrchPlan(form) {
      const { payload } = await call(httpClient, 'orchPlanExecute', { body: planExecuteBody(form) })
      return normOrchExecution(payload)
    },
    async getOrchStats() {
      const { payload } = await get('orchStats')
      return normOrchStats(payload)
    },
    /** 编排历史：page/page_size 由 orchHistoryQuery 夹取（本面不走后端 parse_pagination） */
    async getOrchHistory(filters) {
      const { payload } = await get('orchHistory', { query: orchHistoryQuery(filters) })
      return normOrchHistory(payload)
    },
    // 调度配置：响应即 DispatcherConfig 字段面（snake_case、无派生指标），原样交给 store 作 diff 基线；
    // 表单行与边界由 contract/dispatcher.js 派生，不在此重复。
    async getDispatcherConfig() {
      const { payload } = await get('dispatcherConfig')
      return payload || {}
    },
    async updateDispatcherConfig(patch) {
      const { payload } = await call(httpClient, 'dispatcherConfigUpdate', { body: patch })
      return payload || {}
    },
    async graphOverview() {
      const { payload } = await get('graphOverview')
      return normGraph(payload)
    },
    async graphStats() {
      const { payload } = await get('graphStats')
      return normGraphStats(payload)
    },
    /** 邻域：节点不存在时后端返回 404 node not found，由 store 呈现 */
    async graphNeighbors(nodeId) {
      const { payload } = await get('graphNeighbors', { params: { id: nodeId } })
      return normNeighbors(payload)
    },
    /** limit 省略即后端缺省 10；total_collaborators 是截断前全量 */
    async graphCollaborators(expertId, limit) {
      const { payload } = await get('graphCollaborators', {
        params: { id: expertId },
        query: collaboratorQuery(limit)
      })
      return normCollaborators(payload)
    },
    /** 不可达时后端仍返回 200 + found:false，不抛错，界面按 found 呈现 */
    async graphPath(source, target) {
      const { payload } = await get('graphPath', { params: { source, target } })
      return normGraphPath(payload)
    },
    async graphCommunities() {
      const { payload } = await get('graphCommunities')
      return normCommunities(payload)
    },
    /** 重建是写操作：图版本号 +1 并落盘 */
    async rebuildGraph() {
      const { payload } = await call(httpClient, 'graphRebuild')
      return normGraphRebuild(payload)
    },

    // ── 图谱节点级 CRUD（N4，管理写面；返回 affected + stats，供画布即时刷新）──
    /** 新增节点：{ id, label, node_type, properties? }；id 重复 409、node_type 非法 400 */
    async createGraphNode(body) {
      const { payload } = await call(httpClient, 'graphNodeCreate', { body })
      return payload || {}
    },
    /** 更新节点：{ label?, node_type?, properties? } 合并式；id 不存在 404 */
    async updateGraphNode(id, body) {
      const { payload } = await call(httpClient, 'graphNodeUpdate', { params: { id }, body })
      return payload || {}
    },
    /** 删除节点：联动删除其所有关联边，响应带 removed_edges 计数 */
    async deleteGraphNode(id) {
      const { payload } = await call(httpClient, 'graphNodeDelete', { params: { id } })
      return payload || {}
    },
    /** 新增边：{ source, target, edge_type, weight?, properties? }；端点须存在、重复边 409 */
    async createGraphEdge(body) {
      const { payload } = await call(httpClient, 'graphEdgeCreate', { body })
      return payload || {}
    },
    /** 更新边：{ edge_type?, weight?, properties? } 合并式；seq 越界 404 */
    async updateGraphEdge(seq, body) {
      const { payload } = await call(httpClient, 'graphEdgeUpdate', { params: { seq }, body })
      return payload || {}
    },
    /** 删除边：按 seq；删除后剩余边下标前移重排 */
    async deleteGraphEdge(seq) {
      const { payload } = await call(httpClient, 'graphEdgeDelete', { params: { seq } })
      return payload || {}
    },
    /**
     * 最优团队组建。body 一律由 contract/graph.js 生成：字段写错不会报错，
     * 只会被 serde 静默丢弃（constraints 就是后端收了却从不读的那一类）。
     */
    async optimalTeam(input) {
      const { payload } = await call(httpClient, 'optimalTeam', { body: optimalTeamBody(input) })
      return normOptimalTeam(payload)
    },

    /**
     * 图 RAG 多跳邻域扩展（T2）。body 由 contract/graph.js 的 ragExpandBody 生成：
     * seeds 必发，max_depth 合法范围 1..=4（缺省 2），node_types 作用在结果侧。
     * 返回归一化行 + stats + rerank 标注（当前 graph_only，向量融合待 #27）。
     */
    async expandGraphNeighborhood(input) {
      const { payload } = await call(httpClient, 'graphRagExpand', { body: ragExpandBody(input) })
      return {
        query: payload?.query || {},
        results: ragExpandRows(payload),
        stats: payload?.stats || {},
        rerank: payload?.rerank || ''
      }
    },

    // ── 广场交互：预约 / 收藏 / 即时咨询 / 咨询室 / 团队 ─────────
    async listMyBookings() {
      const { payload } = await get('bookingsMine')
      return normBookingList(payload)
    },
    /**
     * 创建预约。后端 experts_ext.rs:168-171 对未注册/已停用专家返回 404，
     * scheduled_at 省略时默认 +24h，duration_minutes 默认 60。
     */
    async createBooking({ expertId, topic, scheduledAt, durationMinutes }) {
      const body = { expert_id: expertId, topic }
      if (scheduledAt) body.scheduled_at = scheduledAt
      if (durationMinutes) body.duration_minutes = durationMinutes
      const { payload } = await get('bookingCreate', { body })
      return normBooking(payload)
    },
    async cancelBooking(id) {
      const { payload } = await get('bookingCancel', { params: { id } })
      return normBookingCancel(payload)
    },
    /** 读取真实收藏快照；一个专家页最多 200 位，拆为至多两个 100 位批次。 */
    async readFavorites(expertIds) {
      if (!Array.isArray(expertIds) || expertIds.length > 200) throw new Error('收藏读取最多支持一个专家页（200 位）')
      const batches = [expertIds.slice(0, 100)]
      if (expertIds.length > 100) batches.push(expertIds.slice(100))
      const states = await Promise.all(batches.map(async ids => {
        const { payload } = await get('expertFavoritesQuery', { body: favoriteQueryBody(ids) })
        if (!Array.isArray(payload?.items) || payload.items.length !== ids.length || payload.items.some((item, i) => item?.expert_id !== ids[i] || typeof item.favorite !== 'boolean')) {
          throw new Error('收藏读取响应格式错误')
        }
        return payload.items.map(item => ({ expertId: item.expert_id, favorite: item.favorite }))
      }))
      return states.flat()
    },
    async toggleFavorite(expertId, { idempotencyKey = createFavoriteRequest(expertId).key } = {}) {
      const { payload } = await get('expertFavorite', {
        params: { id: expertId }, headers: favoriteRequestHeaders(idempotencyKey), retryOnPost: true
      })
      return normFavorite(payload)
    },
    async consultNow(expertId, { topic = '即时咨询', question, channel = 'text' } = {}) {
      const { payload } = await get('consultNow', { params: { id: expertId }, body: { topic, question, channel } })
      return normConsultNow(payload)
    },
    async consultRoom(bookingId) {
      const { payload } = await get('consultRoom', { params: { id: bookingId } })
      return normConsultRoom(payload)
    },
    async joinTeam({ teamId, expertId, role = 'member' }) {
      const { payload } = await get('joinTeam', { body: { team_id: teamId, expert_id: expertId, role } })
      return normTeamApplication(payload)
    },

    // ── 智能协作：路由 / 单专家 / 多专家 / 辩论 / 智能咨询 / 算法分析 ──
    // 单一入口。body 由 contract/collab.js 的 collabBody 生成：字段名须与后端结构体逐字对应，
    // 传错名不会报错、只会被 serde 静默丢弃，所以拼 body 的活儿不留给调用方。
    async collaborate(mode, input = {}) {
      const def = collabMode(mode)
      if (!def) throw new Error(`未知协作模式: ${mode}`)
      const norm = NORM_BY_ENDPOINT[def.endpoint]
      if (!norm) throw new Error(`协作模式 ${mode} 缺少归一化器`)
      const opts = { body: collabBody(def, input) }
      if (def.endpoint === 'expertConsult') {
        // 单专家咨询的专家走路径参数；后端 404 表示专家不存在或已停用
        const id = (input.expertIds || [])[0]
        if (!id) throw new Error('单专家咨询需要选择 1 位专家')
        opts.params = { id }
      }
      const { payload } = await get(def.endpoint, opts)
      return norm(payload)
    },

    /** 列表信封兜底取数组，供只关心列表的调用方使用 */
    listFrom: unwrapList
  }
}

export const allianceApi = createAllianceApi()
