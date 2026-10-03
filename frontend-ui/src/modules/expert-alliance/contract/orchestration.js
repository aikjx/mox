// 编排面契约：唯一权威是网关 alliance/experts_orchestration.rs 的六个 handler
// 与 experts_common.rs 的 OrchestrationRecord / ExpertsSharedState。
//
// 步骤模板仍来自固定表；执行必须调用真实模型，终态取决于响应与治理判定。
// 模型分析不等于执行外部动作。字段来源由 ORCH_PROVENANCE 唯一标注。

/** POST /api/experts/orchestrate 响应键（:649-666，顺序即 Rust 声明顺序） */
export const ORCH_ORCHESTRATE_KEYS = Object.freeze([
  'orchestration_id', 'task', 'task_type', 'experts', 'plan', 'execution', 'result', 'created_at'
])

/** orchestrate 响应里的 experts[] 键（:636-640）——只有身份三件套，没有分数 */
export const ORCH_EXPERT_SUMMARY_KEYS = Object.freeze(['id', 'name', 'title'])

/** orchestrate 响应里的 plan.steps[] 键（:642-647） */
export const ORCH_PLAN_STEP_SUMMARY_KEYS = Object.freeze(['step_id', 'name', 'status', 'depends_on'])

/** orchestrate 响应里的 execution 键（:658-663） */
export const ORCH_EXECUTION_KEYS = Object.freeze(['status', 'steps_completed', 'steps_total', 'duration_ms'])

/** 融合结果键（fuse_results 出参 :432-438）——orchestrate 的 result 与 plan/execute 的 final_result 同一形状 */
export const ORCH_FUSION_KEYS = Object.freeze([
  'summary', 'key_findings', 'step_summaries', 'recommendations', 'confidence', 'fusion_strategy'
])

/** 单步模拟产出键（simulate_step_execution :265-273） */
export const ORCH_STEP_RESULT_KEYS = Object.freeze([
  'step_id', 'step_type', 'summary', 'key_findings', 'expert', 'confidence', 'executed_at'
])

/** POST /api/experts/plan/generate 响应键（:720-729） */
export const ORCH_PLAN_GENERATE_KEYS = Object.freeze([
  'plan_id', 'task', 'task_type', 'experts', 'steps', 'fusion_strategy', 'status', 'created_at'
])

/** plan/generate 的 steps[] 键（:710-718）：比 orchestrate 多 description/expert_id/step_type，少一层包装 */
export const ORCH_PLAN_STEP_KEYS = Object.freeze([
  'step_id', 'name', 'description', 'expert_id', 'step_type', 'depends_on', 'status'
])

/** POST /api/experts/plan/execute 的成功形状（execute_plan 末尾 :518-528） */
export const ORCH_EXECUTE_KEYS = Object.freeze([
  'plan_id', 'execution_id', 'status', 'steps_executed', 'steps_total', 'overall_status', 'completed_at', 'duration_ms', 'final_result'
])

/**
 * 拓扑排序失败时的形状（:453-462）：多一个 error，少 completed_at 与 final_result。
 * 关键陷阱：这个分支由 handler 原样 `ok(result)` 返回（:766），所以**HTTP 200、code 0**，
 * 失败只写在 body 里。按状态码判成败会把一次成环的计划读成成功。
 */
export const ORCH_EXECUTE_FAILED_KEYS = Object.freeze([
  'plan_id', 'execution_id', 'status', 'error', 'steps_executed', 'steps_total', 'overall_status', 'duration_ms'
])

/** execute 的 steps_executed[] 键（:494-500） */
export const ORCH_EXECUTED_STEP_KEYS = Object.freeze(['step_id', 'name', 'status', 'duration_ms', 'result'])

/** GET /api/experts/orchestration/stats 响应键（:820-835） */
export const ORCH_STATS_KEYS = Object.freeze([
  'total_plans', 'plans_draft', 'plans_ready', 'plans_running', 'plans_completed', 'plans_failed',
  'total_executions', 'success_rate', 'avg_duration_ms', 'avg_steps_per_plan', 'top_used_experts',
  'fusion_strategy_distribution', 'task_type_distribution', 'ts'
])

/** stats 的 top_used_experts[] 键（:799），按 usage_count 降序并截断到 10（:801-806） */
export const ORCH_TOP_EXPERT_KEYS = Object.freeze(['expert_id', 'usage_count'])

/** GET /api/experts/orchestration/history 响应键（:954-959）——没有 total_pages */
export const ORCH_HISTORY_KEYS = Object.freeze(['records', 'total', 'page', 'page_size'])

/** 历史行键（OrchestrationRecord，experts_common.rs:434-448；无 skip_serializing_if，12 键恒在） */
export const ORCH_RECORD_KEYS = Object.freeze([
  'execution_id', 'plan_id', 'task_type', 'status', 'expert_ids', 'steps_completed', 'steps_total',
  'result_summary', 'result', 'created_at', 'completed_at', 'duration_ms'
])

/**
 * 后端写死、不是算出来的常量。每条给出位置与源文件里的字面量，
 * 契约测试逐条断言字面量仍在原处——后端哪天换实现，本表会先红，界面文案随之收。
 * 2026-10-02 去幻影化后：步骤内容改为真实模型咨询（不再是 simulate_step_execution 的写死文案，
 * confidence 也不再恒 0.85）；本表只保留当前源码里仍真实存在的硬编码字面量。
 */
export const ORCH_SIMULATED = Object.freeze([
  {
    id: 'step_initial_pending',
    at: 'experts_orchestration.rs:187',
    literal: 'status: "pending".to_string(),',
    field: 'plan.steps[].status 初值',
    text: '新生成的步骤一律 pending，与任务内容无关'
  },
  {
    id: 'plan_init_draft',
    at: 'experts_orchestration.rs:204',
    literal: 'status: "draft".to_string(),',
    field: 'plan.status 初值',
    text: '新生成的计划一律 draft'
  },
  {
    id: 'plan_generate_status',
    at: 'experts_orchestration.rs:729',
    literal: '"status": "draft",',
    field: 'plan/generate 响应 status',
    text: 'plan/generate 响应里 status 字面 draft'
  },
  {
    id: 'step_list',
    at: 'experts_orchestration.rs:102',
    literal: 'match task_type {',
    field: 'plan.steps[].name / description / step_type',
    text: '步骤清单本身也是查表：四张专用表 + 一张兜底表（research/consulting/development/analysis），任务写什么都不改变步数与顺序'
  },
  {
    id: 'evidence_kind',
    at: 'experts_orchestration.rs:471',
    literal: '"evidence_kind": "model_response"',
    field: 'execute.evidence_kind',
    text: '标记产出来自真实模型咨询——去幻影化后步骤正文是真的，不是写死文案表'
  },
  {
    id: 'default_task_type',
    at: 'experts_orchestration.rs:533',
    literal: 'unwrap_or_else(|| "general".into())',
    field: 'task_type 缺省',
    text: 'orchestrate 未带 task_type 时后端填 general'
  },
  {
    id: 'default_fusion',
    at: 'experts_orchestration.rs:534',
    literal: 'unwrap_or_else(|| "weighted".into())',
    field: 'fusion_strategy 缺省',
    text: 'orchestrate 未带 fusion_strategy 时后端填 weighted'
  },
  {
    id: 'match_score_floor',
    at: 'experts_orchestration.rs:544',
    literal: '*s > 0.2',
    field: '候选专家过滤门槛',
    text: '匹配分低于 0.2 的候选被 filter 掉'
  }
])

/**
 * 恒为 0 的统计项：不是"暂时没有"，而是**没有任何代码路径能写出这个值**。
 * 去幻影化后 plan.status 写入路径为：draft（:204 生成）、running（:409/:740 执行开始）、
 * 终态 failed/completed/partial（:457-464 按 error 与是否全完成三选一）。
 * 因此 plans_failed 已有写入路径（stats :821 真计数），不再恒 0；只剩 plans_ready 无人写。
 */
export const ORCH_ZERO_COUNTERS = Object.freeze(['plans_ready'])

export function orchZeroCounterNote(key) {
  if (!ORCH_ZERO_COUNTERS.includes(key)) return ''
  return `${key} 恒为 0：后端从未把 plan.status 写成该值（只有 draft/running/failed/completed/partial 五个写入路径），这条 0 不是"没有"，是"没有能记录它的代码"`
}

/** 历史侧的反证：plans_failed 恒 0，但经 plan/execute 的历史行 status 可以是 failed（:750 读 overall_status） */
export function orchStatusSplitNote(stats, records = []) {
  const failedHistory = records.filter((r) => r?.status === 'failed').length
  if (!failedHistory) return ''
  return `同一份进程内状态有两个口径：stats 说 failed 计划 ${stats?.plans_failed ?? 0} 个，而本页历史里有 ${failedHistory} 行 status=failed —— plan/execute 的失败写进了历史（读 overall_status，:750），但没有写回 plan.status，两处都别单独引用`
}

// ── 请求体 ────────────────────────────────────────────────────────────
// 三个写面的 task 都带 #[serde(default, alias = "question")]（:538/:553）。
// 前端一律发正名 task：发 question 能通，但那是后端为兼容旧前端留的口子，
// 契约测试盯的是 task 这个字段名，走别名等于把契约挂在注释上。

/** orchestrate 的 max_experts 后端默认值（:585 unwrap_or(3)） */
export const ORCH_MAX_EXPERTS_DEFAULT = 3
/** task_type / fusion_strategy 缺省时后端自己填的字面量（:583-584/:677-678） */
export const ORCH_DEFAULT_TASK_TYPE = 'general'
export const ORCH_DEFAULT_FUSION_STRATEGY = 'weighted'
/** 匹配分数门槛：低于它的候选被 filter 掉（:593） */
export const ORCH_MATCH_SCORE_FLOOR = 0.2

/**
 * select_steps_for_task_type 的 match 分支（:101-139）。只有这 4 个 task_type 有专用步骤表，
 * 其余（含后端默认值 general）全部落到 :131 的兜底表——所以界面不得写"已按 X 类型定制流程"，
 * 除这 4 个之外 everyone 拿到的是同一套 6 步通用文案。
 */
export const ORCH_TASK_TYPE_TABLES = Object.freeze(['research', 'consulting', 'development', 'analysis'])

/** 各表的步数：development 与兜底表都是 6 步（前者多一记 validate），其余三张 5 步 */
export const ORCH_STEP_COUNT_BY_TASK_TYPE = Object.freeze({ research: 5, consulting: 5, development: 6, analysis: 5, __fallback: 6 })

/** 兜底表文案：步数只从上面的表取，界面不得另写一份数字 */
export function orchFallbackNote() {
  return `该类型没有专用步骤表，后端落到兜底表（${ORCH_STEP_COUNT_BY_TASK_TYPE.__fallback} 步通用文案）——不会因为写了它就得到定制流程`
}

/** 兜底表服务的 task_type（后端默认值即落在这里，:583/:677） */
export function orchUsesFallbackTable(taskType) {
  return !ORCH_TASK_TYPE_TABLES.includes(String(taskType || ''))
}

/** simulate_step_execution 的文案表键（:230-263）：七类 + 兜底，八条分支都是写死中文 */
export const ORCH_STEP_TYPES = Object.freeze(['intake', 'research', 'analysis', 'consult', 'review', 'synthesize', 'validate'])

/**
 * 拓扑形状：generate_plan 让每一步只依赖上一步（:174-177 的 prev_step_id），
 * 所以本面产出的"依赖图"**恒为一条链**——没有并行分叉、没有汇聚，也不可能成环。
 * 界面据此说明形状来源，不得暗示这是任务需求决定的拓扑。
 */
export function orchTopologyNote(steps = []) {
  const chain = steps.every((s, i) => (i === 0 ? (s?.dependsOn?.length ?? 0) === 0 : s?.dependsOn?.length === 1 && s.dependsOn[0] === steps[i - 1].stepId))
  if (!steps.length) return '本次没有生成任何步骤'
  if (chain) return `依赖关系是 ${steps.length} 步单链：后端固定"每步只依赖上一步"（experts_orchestration.rs:174-177），与任务内容无关，也不存在并行分支`
  return '步骤依赖不是单链：本面的生成器只会产出链，出现别的形状说明 plan 来自别处（例如 plan/execute 传了 step_ids 过滤）'
}

/** 步骤专家指派：在候选池内按 compute_match_score(task + 步骤名) 取 argmax（:159-172），候选池只有 top N */
export function orchStepExpertNote(steps = []) {
  const ids = new Set(steps.map((s) => s?.expertId).filter(Boolean))
  if (!steps.length || ids.size) return ''
  return '所有步骤的 expert_id 都是 null：候选池为空时后端直接给 None（:160-161），计划依然生成，步骤名与文案照旧'
}

/** POST /api/experts/orchestrate 请求体（OrchestrateBody :535-548） */
export function orchestrateBody(form = {}) {
  const body = { task: String(form.task ?? '').trim() }
  const taskType = String(form.taskType ?? '').trim()
  if (taskType) body.task_type = taskType
  const fusion = String(form.fusionStrategy ?? '').trim()
  if (fusion) body.fusion_strategy = fusion
  // 0 是合法值（take(0) → 零位专家），不能用 || 判缺省，否则用户设置的 0 会被静默换成 3
  if (form.maxExperts !== undefined && form.maxExperts !== null && form.maxExperts !== '') {
    body.max_experts = Number(form.maxExperts)
  }
  const ids = (Array.isArray(form.expertIds) ? form.expertIds : []).filter((v) => v !== undefined && v !== null && String(v).trim()).map((v) => String(v).trim())
  // 空数组要跳过而非发出：retain 会把候选清空（:597-599），"不指定"与"指定 0 位"在 wire 上必须区分开
  if (ids.length) body.expert_ids = ids
  return body
}

/** POST /api/experts/plan/generate 请求体（GeneratePlanBody :550-561：没有 max_experts，硬上限 top 5 :692） */
export function planGenerateBody(form = {}) {
  const body = { task: String(form.task ?? '').trim() }
  const taskType = String(form.taskType ?? '').trim()
  if (taskType) body.task_type = taskType
  const fusion = String(form.fusionStrategy ?? '').trim()
  if (fusion) body.fusion_strategy = fusion
  const ids = (Array.isArray(form.expertIds) ? form.expertIds : []).filter((v) => v !== undefined && v !== null && String(v).trim()).map((v) => String(v).trim())
  if (ids.length) body.expert_ids = ids
  return body
}

/** POST /api/experts/plan/execute 请求体（ExecutePlanBody :563-568）：plan_id 无 serde default，缺整个 body 就被 axum 拒 */
export function planExecuteBody(form = {}) {
  const body = { plan_id: String(form.planId ?? '').trim() }
  const ids = (Array.isArray(form.stepIds) ? form.stepIds : []).filter((v) => v !== undefined && v !== null && String(v).trim()).map((v) => String(v).trim())
  // 空数组**必须发**：execute_set 是 Some(空集)，should_execute 全 false（:477）→ 零步执行但仍返回 completed_at 形状；
  // 不发才是 None（全量执行）。所以这里不能像 expert_ids 那样"空则省略"。
  if (Array.isArray(form.stepIds)) body.step_ids = ids
  return body
}

/** 编排动作的前置校验：返回中文原因，空串表示可以发 */
export function orchestrateProblem(form = {}) {
  if (!String(form.task ?? '').trim()) return '任务描述不能为空：后端对空白 task 直接 400（:580-582）'
  const n = form.maxExperts
  if (n !== undefined && n !== null && n !== '') {
    const v = Number(n)
    if (!Number.isInteger(v) || v < 0) return '参与人数只能是非负整数：0 表示不选任何专家'
  }
  return ''
}

export function planExecuteProblem(form = {}) {
  if (!String(form.planId ?? '').trim()) return '缺少 plan_id：该字段没有 serde 默认值，缺了会被请求层拒掉，页面拿到的不是 {code,msg} 信封（:565）'
  return ''
}

// ── 历史分页 ──────────────────────────────────────────────────────────
// 本面是 gateway 里**唯一不走 parse_pagination 的分页**：page/page_size 直接 parse 后使用
// （:936-937），既没有 1..=200 的上限，也没有 page>=1 的下限。
// page_size=0 会让 skip(0).take(0) 返回空页而 total 非零——一个"有数据但永远翻不出内容"的状态。

export const ORCH_HISTORY_PAGE_SIZE_MAX = 200

/** 与网关其它分页保持一致的夹取；越界时带出提示，不假装后端做了这件事 */
export function orchHistoryQuery(filters = {}) {
  const page = Math.max(1, Math.trunc(Number(filters.page) || 1))
  const raw = Number(filters.pageSize)
  const clamped = Number.isFinite(raw) && raw > 0 ? Math.min(Math.trunc(raw), ORCH_HISTORY_PAGE_SIZE_MAX) : 20
  const query = { page, page_size: clamped }
  const status = String(filters.status ?? '').trim()
  if (status) query.status = status
  const taskType = String(filters.taskType ?? '').trim()
  if (taskType) query.task_type = taskType
  return query
}

/** 后端回了 page_size=0 之类时给界面的话：这是真实行为，不是前端 bug */
export function orchHistoryAnomaly(res) {
  const size = Number(res?.page_size)
  const total = Number(res?.total) || 0
  const rows = Array.isArray(res?.records) ? res.records.length : 0
  if (size === 0 && total > 0) return '后端按 page_size=0 处理，take(0) 使本页必然为空而 total 非零（:937/:952）：本页没有内容不是历史被清空'
  if (rows && size > 0 && rows > size) return `单页行数 ${rows} 超过请求的 ${size}：与 take(page_size) 矛盾，需回查后端`
  return ''
}

/** total_pages 由后端缺席 → 前端计算，界面须标明"由 total/page_size 算得" */
export function orchHistoryPages(res) {
  const total = Number(res?.total) || 0
  const size = Number(res?.page_size) || 0
  return { total, pages: size > 0 ? Math.ceil(total / size) : 0, computed: true }
}

// ── 逐字段来源（界面文案单源）────────────────────────────────────────

export const ORCH_PROVENANCE = Object.freeze({
  real: { tone: 'success', label: '真实计算', text: '由注册表与请求算出，可在后端源码找到算式' },
  simulated: { tone: 'warning', label: '固定模板', text: '步骤模板来自任务类型表，尚未按任务动态生成' },
  model: { tone: 'warning', label: '模型分析', text: '来自真实模型响应及融合，不构成外部操作已经完成的证据' },
  literal: { tone: 'danger', label: '常量', text: '后端写死的字面量，不随输入变化，不含判断信息' },
  wallclock: { tone: 'success', label: '真实计时', text: 'std::time::Instant 测得的本次进程内耗时' }
})

/** 字段 → 来源档位。界面按此渲染角标；未列出的字段默认 real。 */
export const ORCH_FIELD_PROVENANCE = Object.freeze({
  'orchestrate.experts': 'real',
  'orchestrate.plan.steps': 'real',
  'orchestrate.execution.status': 'real',
  'orchestrate.execution.steps_completed': 'real',
  'orchestrate.execution.duration_ms': 'wallclock',
  'orchestrate.result.summary': 'model',
  'orchestrate.result.key_findings': 'model',
  'orchestrate.result.recommendations': 'model',
  'orchestrate.result.confidence': 'model',
  'orchestrate.result.fusion_strategy': 'real',
  'orchestrate.result.step_summaries': 'model',
  'execute.steps_executed[].status': 'real',
  'execute.steps_executed[].duration_ms': 'wallclock',
  'execute.steps_executed[].result.expert': 'real',
  'execute.steps_executed[].result.confidence': 'model',
  'execute.final_result.confidence': 'model',
  'execute.error': 'real',
  'plan.status': 'literal',
  'plan.steps[].name': 'simulated',
  'plan.steps[].description': 'simulated',
  'plan.steps[].step_type': 'simulated',
  'plan.steps[].depends_on': 'literal',
  'plan.steps[].expert_id': 'real',
  'stats.total_plans': 'real',
  'stats.success_rate': 'real',
  'stats.avg_duration_ms': 'real',
  'stats.avg_steps_per_plan': 'real',
  'stats.top_used_experts': 'real',
  'stats.plans_ready': 'literal',
  'stats.plans_failed': 'real'
})

export function orchProvenanceOf(fieldPath) {
  const tier = ORCH_FIELD_PROVENANCE[fieldPath]
  return tier ? { tier, ...ORCH_PROVENANCE[tier] } : null
}

/** 界面上方那条自陈：一次编排之后必须让用户知道"跑的是真拓扑、假步骤" */
export function orchRunDisclaimer() {
  return '依赖排序与选人来自真实注册表（Kahn 拓扑 + compute_match_score）；步骤模板按任务类型固定，正文来自真实模型咨询与融合，耗时由进程实测。模型分析不能证明外部操作已执行；请按执行终态与实际产物核对结果。'
}

/** D4 落盘后：plans/orchestration_history 仍以进程内内存态为权威，但写后立即投影 SQLite（best-effort），重启可从库里恢复 */
export function orchVolatileNote() {
  return '计划表与执行历史以网关进程内内存态为权威（plans / orchestration_history），D4 起写后立即投影 SQLite（upsert_plan / insert_history_record），崩溃可恢复；持久化失败仅记日志不阻断业务'
}

/**
 * 空选人的解释：三个 filter 叠加可以让 experts 为 []，而响应仍是成功。
 * 界面若把"0 位专家"渲染成空白面板，就是替后端把失败藏起来。
 */
export function orchEmptyExpertsNote(res) {
  const n = Array.isArray(res?.experts) ? res.experts.length : null
  if (n === null || n > 0) return ''
  return '后端没有选出任何专家：候选须启用且在线，通过匹配门槛 0.2 与指定 expert_ids 筛选。执行需要真实可用专家；空候选不会产生成功的模型结果。'
}

/** orchestrate/plan 响应里的步骤列表：两个面的包装层级不同，取值口径在此收口 */
export function orchStepsOf(res) {
  if (Array.isArray(res?.steps)) return res.steps
  if (Array.isArray(res?.plan?.steps)) return res.plan.steps
  return []
}

/** plan/execute 响应 → 成败判定：只看 body，不看状态码（200 + status:"failed" 是真实形状） */
export function orchExecuteOutcome(res) {
  const status = String(res?.status ?? '')
  const failed = status === 'failed' || !!res?.error
  return {
    failed,
    status,
    error: res?.error || '',
    note: failed
      ? '请求成功返回（HTTP 200 / code 0），失败只写在 body 里：拓扑排序成环时后端直接 return 这个形状（:452-463），所以判定成败只能读 status'
      : ''
  }
}

/**
 * 编排面状态档（计划 / 步骤 / 总体共用一张表）。wire 权威：
 * platform/gateway/mox-platform-gateway-svc/src/alliance/experts_orchestration.rs
 *   :186 pending（计划步骤初值）  :203 draft（计划）  :469/:484 running
 *   :490/:497/:511 completed      :456 failed         :508 completed|partial
 *   :750 unknown（读不到 overall_status 时的兜底串）
 * 与任务域 TASK_STATUS 只共享四个措辞（待执行/执行中/已完成/失败），
 * 本表另有 draft/partial/unknown 三档，任务域没有 planning 之外的这组值，故不并表。
 */
export const ORCH_STATUS = Object.freeze({
  PENDING: 'pending',
  DRAFT: 'draft',
  RUNNING: 'running',
  COMPLETED: 'completed',
  FAILED: 'failed',
  PARTIAL: 'partial',
  UNKNOWN: 'unknown'
})

export const ORCH_STATUS_LABELS = Object.freeze({
  pending: '待执行',
  draft: '草案',
  running: '执行中',
  completed: '已完成',
  failed: '失败',
  partial: '部分完成',
  unknown: '未知'
})

export function orchStatusLabel(status) {
  return ORCH_STATUS_LABELS[status] ?? status ?? ''
}
