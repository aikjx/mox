// 调度器配置契约：唯一权威是网关 experts_dispatcher.rs 的 update_config 校验分支
// 与 experts_common.rs 的 DispatcherConfig 默认值函数。字段名、边界、默认值一律从 Rust 解析，
// 前端不得凭想象补范围（见 contract.test.js 的「调度配置」组断言）。
//
// 后端语义：PUT 为**合并式**更新 —— body 里没出现的键保持原值，响应是合并后的完整配置。
// 因此前端只发改动过的键，而不是整份回写。

/** update_config 的 `let valid = [...]` 清单，顺序即后端声明顺序 */
export const DISPATCH_STRATEGY = Object.freeze([
  { value: 'round_robin', label: '轮询' },
  { value: 'least_load', label: '最小负载' },
  { value: 'best_match', label: '最佳匹配（后端默认）' },
  { value: 'weighted_random', label: '加权随机' }
])

/**
 * 表单字段规格。
 * checked=true 表示 update_config 里有对应校验分支（越界即 400），边界取该分支的字面量；
 * checked=false 表示后端照收不验，前端只作输入约束，不得声称"后端会拒绝"。
 */
export const DISPATCH_CONFIG_FIELDS = Object.freeze([
  {
    key: 'strategy',
    label: '调度策略',
    kind: 'select',
    default: 'best_match',
    checked: true,
    hint: '4 个取值之外后端返回 400'
  },
  {
    key: 'intelligent_matching',
    label: '智能匹配',
    kind: 'switch',
    default: true,
    checked: false,
    hint: '后端不校验，按布尔照收'
  },
  {
    key: 'match_threshold',
    label: '匹配阈值',
    kind: 'number',
    min: 0,
    max: 1,
    step: 0.05,
    default: 0.3,
    checked: true,
    hint: '区间 0–1（含端点），越界 400'
  },
  {
    key: 'max_retries',
    label: '最大重试次数',
    kind: 'number',
    min: 0,
    max: 10,
    step: 1,
    default: 3,
    checked: true,
    hint: '上限 10，超过即 400'
  },
  {
    key: 'timeout_seconds',
    label: '超时（秒）',
    kind: 'number',
    min: 1,
    max: 3600,
    step: 30,
    default: 120,
    checked: true,
    hint: '区间 1–3600，越界 400'
  },
  {
    key: 'circuit_breaker_threshold',
    label: '熔断阈值（连续失败次数）',
    kind: 'number',
    min: 0,
    max: 100,
    step: 1,
    default: 5,
    checked: false,
    hint: '后端不校验；上限只是本表单的输入约束'
  },
  {
    key: 'concurrency_control',
    label: '并发控制',
    kind: 'switch',
    default: true,
    checked: false,
    hint: '后端不校验，按布尔照收'
  }
])

// DispatcherConfig 还有 weights: HashMap<String, f64>（专家 ID → 权重）。
// 后端接受但本模块暂不挂载编辑入口：没有专家选择器就无法填 key，
// 硬塞一个 JSON 文本框只会造出"能填但没人会用"的控件。登记在此，契约测试保证它仍是后端字段。
export const DISPATCH_UNMOUNTED_FIELDS = Object.freeze([
  { key: 'weights', reason: 'Map<专家ID, 权重>：缺专家多选控件，挂载即无效输入面' }
])

export function dispatchStrategyLabel(value) {
  return DISPATCH_STRATEGY.find((s) => s.value === value)?.label || value
}

export function dispatchField(key) {
  return DISPATCH_CONFIG_FIELDS.find((f) => f.key === key) || null
}

/**
 * 后端配置 → 表单行：值缺失时回落默认并标记 `missing`，
 * 不假装后端给了没给的东西（GET 返回的是内存态，字段齐全，但换实现时要能看出差异）。
 */
export function dispatchRows(config = {}) {
  return DISPATCH_CONFIG_FIELDS.map((f) => {
    const raw = config?.[f.key]
    const present = raw !== undefined && raw !== null
    let value = present ? raw : f.default
    if (f.kind === 'number' && present) {
      const n = Number(raw)
      value = Number.isFinite(n) ? n : f.default
    }
    if (f.kind === 'switch') value = !!raw
    return { ...f, value, present, dirty: present && value !== f.default }
  })
}

/** 提交前校验：复刻 update_config 的四条 400 分支，返回中文原因（空串表示可发） */
export function dispatchProblem(key, value) {
  const f = dispatchField(key)
  if (!f) return `未知配置项 ${key}`
  if (f.kind === 'select') {
    return DISPATCH_STRATEGY.some((s) => s.value === value) ? '' : `策略必须是 ${DISPATCH_STRATEGY.map((s) => s.value).join(' / ')}`
  }
  if (f.kind === 'switch') return typeof value === 'boolean' ? '' : `${f.label}必须是布尔值`
  const n = Number(value)
  if (!Number.isFinite(n)) return `${f.label}必须是数字`
  if (!f.checked) return ''
  if (n < f.min || n > f.max) return `${f.label}需在 ${f.min}–${f.max} 之间`
  return ''
}

/**
 * 表单草稿 → PUT 请求体：只保留与后端当前值不同的键（合并式更新）。
 * 改动项若校验不过，返回 problem 而不是发出一个必 400 的请求。
 */
export function dispatchPatch(current = {}, draft = {}) {
  const patch = {}
  const problems = []
  for (const f of DISPATCH_CONFIG_FIELDS) {
    if (!(f.key in draft)) continue
    const value = f.kind === 'number' ? Number(draft[f.key]) : draft[f.key]
    if (value === current?.[f.key]) continue
    const problem = dispatchProblem(f.key, value)
    if (problem) {
      problems.push(problem)
      continue
    }
    patch[f.key] = value
  }
  return { patch, problem: problems[0] || '' }
}

/** 整份配置的越界项（进入表单即检查，便于把后端会拒绝的值先标红） */
export function dispatchInvalid(draft = {}) {
  const invalid = {}
  for (const f of DISPATCH_CONFIG_FIELDS) {
    const problem = dispatchProblem(f.key, draft[f.key])
    if (problem) invalid[f.key] = problem
  }
  return invalid
}

// ── 分发实跑（POST /api/experts/dispatcher/dispatch）──────────────────
// 请求体规格取 DispatchBody（experts_dispatcher.rs:85-92），发哪些键由 handler 实际读取决定：
// dispatch 只把 task_type/input/expert_ids 交给 dispatch_task（:559-560），constraints 收下即丢。
// task_type 虽被后端显式弃用（:219 的"已知缺口"注释与 :225 的 `_task_type` 形参），仍**必须发**——该键无 serde default，
// 缺了整个 JSON 提取就被 axum 拒掉，而拒绝体不是 {code,msg,data} 信封，页面拿不到可读原因。

/** @param {Record<string, unknown>} [form] 实跑表单（camelCase，与视图绑定一致） */
export function dispatchRunBody(form = {}) {
  const body = {
    task_type: String(form.taskType ?? '').trim(),
    input: String(form.input ?? '').trim()
  }
  // null/undefined 要先丢掉再 String()：否则 String(null) === 'null' 会变成一个看着像 id 的假专家
  const ids = Array.isArray(form.expertIds)
    ? form.expertIds.filter((v) => v !== undefined && v !== null && String(v).trim()).map((v) => String(v).trim())
    : []
  if (ids.length) body.expert_ids = ids
  return body
}

/**
 * 实跑前的表单校验。空 input 不发是后端算出来的结论，不是体验偏好：
 * compute_match_score 按空白/逗号/、// 切词且**不过滤空 token**（experts_common.rs:774,781），
 * 空串对任何领域都 `contains("") === true`，于是「领域匹配」这一项（权重 0.30）对每位候选都满分，
 * 这份分数与需求无关，摆在界面上就是一条假匹配度。
 */
export function dispatchRunProblem(form = {}) {
  if (!String(form.input ?? '').trim()) return '需求描述不能为空：空串会让后端把「领域匹配」对全员判满分，分数不再反映需求'
  return ''
}

/**
 * 实跑结果 → "配置到底生效没有"的判据。每条结论都对应后端一处可核对的字面量，
 * 界面不得把"请求成功"说成"按你所选策略选中"。
 * @param {{strategyUsed?:string, assigned?:Array<{matchScore:number}>}|null} result
 * @param {Record<string, any>|null} [config] 后端当前配置（GET /dispatcher/config 的原始响应）
 */
export function dispatchRunFindings(result, config) {
  const strategy = String(result?.strategyUsed ?? '')
  const out = []
  if (!strategy) return out
  const configured = config?.strategy
  if (strategy === 'specified') {
    out.push({ tone: 'info', text: '本次走"指定专家"分支（strategy_used=specified），策略配置没有参与选人' })
  } else if (strategy.endsWith('(fallback)')) {
    out.push({ tone: 'danger', text: `后端不认识配置里的策略，已回退（strategy_used=${strategy}）` })
  } else if (!configured) {
    out.push({ tone: 'info', text: `strategy_used=${strategy}；配置未取到，无法比对是否按你所选策略` })
  } else if (strategy !== configured) {
    out.push({ tone: 'danger', text: `strategy_used=${strategy} 与当前配置 ${configured} 不一致：要么实跑后配置又改过，要么走的不是这条分支` })
  } else {
    out.push({ tone: 'success', text: `strategy_used=${strategy}，与当前配置一致` })
  }
  const scores = (result?.assigned ?? []).map((a) => a?.matchScore)
  if (scores.length && scores.every((v) => v === 0.5)) {
    out.push({ tone: 'warning', text: '所有 match_score 恰为 0.5：intelligent_matching=false 时后端对每位候选硬编码 0.5（experts_dispatcher.rs:171-175），本次排序不反映匹配度' })
  }
  if (strategy === 'weighted_random' && !Object.keys(config?.weights || {}).length) {
    out.push({ tone: 'info', text: '权重表为空：后端按 match_score（下限 0.01）加权抽取（experts_dispatcher.rs:193-196），伪随机种子取自时间戳纳秒（:204-205），所以同一份输入两次实跑可能选到不同专家，这不是缓存或网络故障' })
  }
  return out
}

// ── 调度状态与负载重置 ────────────────────────────────────────────────
// GET /dispatcher/status（experts_dispatcher.rs:452-545）是 reset 的读数侧：
// 没有它，「重置成功」就只剩后端自报的一个布尔，界面无法自证。

/** 后端把 engine_status 写死成字面量（:536），它不是探活结果 */
export const DISPATCH_ENGINE_STATUS_LITERAL = 'running'

/** status 的真实出参键集（:536-545）。界面读了这个集合之外的键就是凭空造字段 */
export const DISPATCH_STATUS_KEYS = Object.freeze([
  'engine_status', 'current_strategy', 'active_dispatches', 'total_dispatches',
  'success_rate', 'avg_dispatch_ms', 'circuit_breakers', 'expert_loads', 'last_dispatch_at', 'ts'
])

/**
 * 熔断列表为空时的原话。FAILURE_COUNTS（:42）在全 gateway 只有读侧（:47/:132/:494/:776）
 * 与 reset 自己的 remove/clear（:794/:828），没有任何生产路径 insert ——
 * 所以空列表是"没有数据"，不是"数据良好"，两者必须在界面上分开。
 */
export function breakerEmptyNote(list = []) {
  if ((list || []).length) return ''
  return '后端当前没有累计失败次数的代码路径，熔断计数永远为空，因此这份空列表不代表"所有专家服务正常"（experts_dispatcher.rs:42-51 只有读侧与清除）'
}

/** 成功率的可信度说明：分母只有终态记录，无终态样本时后端直接给 1.0（:462-469） */
export function successRateNote(status) {
  if (!Number(status?.totalDispatches)) return '本次进程还没有调度记录：后端在无终态样本时把 success_rate 返回为 1.0（experts_dispatcher.rs:466-469），这不是"100% 成功"'
  return '成功率 = 完成 /（完成 + 失败），不含进行中的记录；计数读的是进程内 dispatch_records（experts_common.rs:464、:506 初始为空 Vec，无落盘），重启即归零'
}

/**
 * 二次确认弹窗的后果清单：每条都对应后端一处实现，不写"可能导致…"这类无法核对的话。
 * @param {{ all?: boolean, name?: string, id?: string }} target
 */
export function dispatchResetLines(target = {}) {
  const all = !!target.all
  const who = all ? '注册表里的全部专家' : `「${target.name || target.id || '该专家'}」`
  return [
    all
      ? '把整张注册表的 availability.current_load 逐个写成 0（:817-823 走 registry.values_mut()，含停用与软删除者）。注意状态表只列 enabled 专家（:515-517），所以表内人数不是被重置的人数，回执里的 reset_count 才是'
      : `把 ${who} 的 availability.current_load 写成 0；只有注册表里存在这个 id 才会写（:786-790 是 if let Some）`,
    all
      ? '熔断计数整表 map.clear()（:828-831）—— 但后端没有写入失败计数的路径，这一步当前无数据可清'
      : '删除该专家的熔断计数（:793-797）—— 后端没有写入失败计数的路径，这一步当前无数据可清',
    '两处都只改内存、不落库：这两个 handler 全程不调 save_registry，而注册表本身是 SQLite 支撑，所以网关重启后负载按最后一次落库的快照恢复；重置之后的第一次咨询又会把"已归零"顺带写进库（experts_registry.rs:806-810）',
    all
      ? '响应只有 reset_count / reset_expert_ids / reset_at，没有逐项旧值（:836-840）——想知道"重置前是多少"只能靠重置前的这份状态读数'
      : "后端对查无此 id 也返回 reset:true（无 404，previous_load 走 unwrap_or(0)，:769-783），所以这条回执不证明该专家存在",
    '可观察的效果只在并发控制打开时：current_load >= max_concurrent 才会把专家挡在候选之外（:141-147），归零不改变任何人的匹配分数'
  ]
}

/** 全量重置没有角色判定可依赖（后端只认证不授权），前端唯一的中继就是这道手动门槛 */
export const DISPATCH_RESET_ALL_CONFIRM = 'RESET-ALL'

/**
 * 单专家重置的请求体。`reason` 可留空，但**必须发一个 JSON 对象**：
 * handler 签名是 `Json<ResetBody>`（experts_dispatcher.rs:763-766），空体会被 axum
 * 拒成非 {code,msg,data} 信封的错误体，页面拿不到可读原因。
 */
export function dispatchResetBody(reason) {
  const text = String(reason ?? '').trim()
  return text ? { reason: text } : {}
}

/** 重置回执 → 成功通知文案。不得把 reset:true 说成"该专家已被重置"这类存在性断言 */
export function dispatchResetNotice(res = {}) {
  const id = res.expertId || '该 id'
  return `已请求重置 ${id} 的调度状态，后端回读到的重置前负载为 ${res.previousLoad ?? 0}。注意：后端对不存在的 id 同样返回 reset:true，这条回执不代表该专家存在，也不代表已落库。`
}

/** 全量重置回执 → 通知文案（计数来自后端，不由前端按选中数猜测） */
export function dispatchResetAllNotice(res = {}) {
  return `已请求全量重置，后端报告本次覆盖 ${res.resetCount ?? 0} 位专家（reset_expert_ids 来自注册表当前键集）。该动作同样不落库，重启后按最后一次落库快照恢复。`
}

