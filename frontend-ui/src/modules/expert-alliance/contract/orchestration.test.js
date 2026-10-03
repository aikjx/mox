// 编排面契约门禁：断言全部指向网关 experts_orchestration.rs / experts_common.rs 的真实文本。
// 三条设计约束：
// ① 键集等集靠「按签名找块 + 命中次数确定」，不靠行号切片——大括号计数取块、剥掉嵌套层再取键，
//    否则 orchestrate 那种内联嵌套对象会把子键混进顶层；
// ② 行号只用来钉 ORCH_SIMULATED：每个"写死的常量"必须仍然出现在它被声明的那一行，
//    后端换成真实实现时本文件先红，界面文案随之收；
// ③ 归一化产物的键与 wire 键一一对应：界面读不到后端没给的东西，也不漏读后端给的东西。
import { describe, it, expect } from 'vitest'
import { readFileSync, existsSync } from 'node:fs'
import path from 'node:path'

import {
  ORCH_DEFAULT_FUSION_STRATEGY, ORCH_DEFAULT_TASK_TYPE, ORCH_EXPERT_SUMMARY_KEYS, ORCH_EXECUTE_KEYS,
  ORCH_EXECUTE_FAILED_KEYS, ORCH_EXECUTED_STEP_KEYS, ORCH_EXECUTION_KEYS, ORCH_FIELD_PROVENANCE,
  ORCH_FUSION_KEYS, ORCH_HISTORY_KEYS, ORCH_HISTORY_PAGE_SIZE_MAX, ORCH_MATCH_SCORE_FLOOR,
  ORCH_MAX_EXPERTS_DEFAULT, ORCH_ORCHESTRATE_KEYS, ORCH_PLAN_GENERATE_KEYS, ORCH_PLAN_STEP_KEYS,
  ORCH_PLAN_STEP_SUMMARY_KEYS, ORCH_PROVENANCE, ORCH_RECORD_KEYS, ORCH_SIMULATED, ORCH_STATS_KEYS,
  ORCH_STEP_COUNT_BY_TASK_TYPE, ORCH_STEP_RESULT_KEYS, ORCH_STEP_TYPES, ORCH_TASK_TYPE_TABLES,
  ORCH_TOP_EXPERT_KEYS, ORCH_ZERO_COUNTERS, orchEmptyExpertsNote, orchExecuteOutcome, orchFallbackNote,
  orchHistoryAnomaly, orchHistoryPages, orchHistoryQuery, orchProvenanceOf, orchRunDisclaimer, orchStatusSplitNote,
  orchStepExpertNote, orchTopologyNote, orchUsesFallbackTable, orchVolatileNote, orchZeroCounterNote,
  orchestrateBody, orchestrateProblem, planExecuteBody, planExecuteProblem, planGenerateBody
} from './orchestration.js'
import { ENDPOINTS, UNMOUNTED_ROUTES } from './endpoints.js'
import { normOrchExecution, normOrchHistory, normOrchPlan, normOrchStats, normOrchestration } from '../model/normalize.js'

function findRoot(from) {
  let dir = from
  for (let i = 0; i < 8; i++) {
    if (existsSync(path.join(dir, 'docs/API-REGISTRY.md')) && existsSync(path.join(dir, 'platform'))) return dir
    const parent = path.dirname(dir)
    if (parent === dir) break
    dir = parent
  }
  throw new Error(`未找到仓库根（自 ${from} 向上）`)
}

const ROOT = findRoot(process.cwd())
const read = (rel) => readFileSync(path.join(ROOT, rel), 'utf8').replace(/\r\n/g, '\n')
const ORCH_RS = read('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_orchestration.rs')
const COMMON_RS = read('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs')
const DB_RS = read('platform/gateway/mox-platform-gateway-svc/src/alliance/experts_db.rs')
const REGISTRY_MD = read('docs/API-REGISTRY.md')
const AOV = read('frontend-ui/src/modules/expert-alliance/views/AllianceOrchestrationView.vue')
const ORS = read('frontend-ui/src/modules/expert-alliance/store/alliance-orch.store.js')
const ORCH_LINES = ORCH_RS.split('\n')

const snakeToCamel = (s) => s.replace(/_+([a-z0-9])/g, (_, c) => c.toUpperCase())
const jsonKeys = (block) => [...block.matchAll(/"([a-z_]+)"\s*:/g)].map((m) => m[1])

/** 剥掉嵌套的 {} 与 [] 内容，只留本层的 `"key":` —— 内联子对象不得污染顶层键集 */
function ownLevel(text) {
  let out = ''
  let depth = 0
  for (const ch of text) {
    if (ch === '{' || ch === '[') depth += 1
    else if (ch === '}' || ch === ']') depth -= 1
    else if (depth === 0) out += ch
    else if (depth > 0) out += ' '
  }
  return out
}

/** 抓出源文件里每个 json!({...}) 的顶层键列表 */
function jsonBlocks(src) {
  const out = []
  const re = /json!\s*\(\s*\{/g
  let m
  while ((m = re.exec(src))) {
    const open = src.indexOf('{', m.index)
    let depth = 0
    let i = open
    for (; i < src.length; i++) {
      if (src[i] === '{') depth += 1
      else if (src[i] === '}') {
        depth -= 1
        if (depth === 0) break
      }
    }
    out.push(jsonKeys(ownLevel(src.slice(open + 1, i))))
  }
  return out
}

const ORCH_BLOCKS = jsonBlocks(ORCH_RS)

/** 按键集签名找块，并核对命中处数（同一形状被两处复用要如实计入，不是错误） */
function blockWith(keys, label, times = 1) {
  const sig = JSON.stringify(keys)
  const hits = ORCH_BLOCKS.filter((b) => JSON.stringify(b) === sig)
  expect(hits.length, `${label}：键集 ${keys.join(',')} 命中 ${hits.length} 处，期望 ${times} 处`).toBe(times)
}

function structFields(structName, src = COMMON_RS) {
  const m = src.match(new RegExp(`struct ${structName} \\{([\\s\\S]*?)\\n\\}`))
  if (!m) throw new Error(`Rust 源缺少 struct ${structName}`)
  const out = []
  for (const line of m[1].split('\n')) {
    const t = line.trim()
    if (!t || t.startsWith('//') || t.startsWith('#[')) continue
    const f = t.match(/^(?:pub\s+)?([a-z_]+):/)
    if (f) out.push(f[1])
  }
  return out
}

describe('编排面路由 ↔ 网关 router 与注册表', () => {
  const router = ORCH_RS.match(/pub fn build_experts_orchestration_router[\s\S]*?\n}/)[0]
  const rows = [...router.matchAll(/\.route\("([^"]+)",\s*(get|post)\((\w+)\)/g)].map((m) => ({ path: m[1], verb: m[2].toUpperCase(), handler: m[3] }))

  it('router 恰好装配 6 条，本面挂载其中 5 条', () => {
    expect(rows.length).toBe(6)
    const mounted = Object.values(ENDPOINTS).filter((e) => e.registry.startsWith('experts.orch.'))
    expect(mounted.length).toBe(5)
    for (const ep of mounted) {
      expect(rows.find((r) => r.path === ep.path && r.verb === ep.method), `${ep.path} ${ep.method} 不在网关 router 里`).toBeTruthy()
    }
  })

  it('plugins 那条仍未挂载，且台账里的定性是 rejected 而非 backlog', () => {
    const pluginRow = rows.find((r) => r.path === '/api/experts/orchestration/plugins')
    expect(pluginRow).toBeTruthy()
    expect(Object.values(ENDPOINTS).some((e) => e.path === pluginRow.path), 'plugins 被挂载了：它是纯硬编码数组').toBe(false)
    expect(UNMOUNTED_ROUTES.find((x) => x.registry === 'experts.orch.plugins')?.verdict).toBe('rejected')
  })

  it('五条注册表行的路径与方法逐字对齐 docs/API-REGISTRY.md', () => {
    for (const id of ['experts.orch.orchestrate', 'experts.orch.plan_generate', 'experts.orch.plan_execute', 'experts.orch.stats', 'experts.orch.history']) {
      const ep = Object.values(ENDPOINTS).find((e) => e.registry === id)
      expect(ep, `${id} 未挂载`).toBeTruthy()
      expect(REGISTRY_MD, `${id} 的登记行与方法/路径不符`).toContain(`| \`${id}\` | ${ep.method} | \`${ep.path}\` |`)
    }
  })

  it('五个 handler 都返回 ApiResponse<Value>，故信封一律 flat（不是 SDK 那套 nested）', () => {
    for (const h of ['orchestrate', 'generate_plan_handler', 'execute_plan_handler', 'orchestration_stats', 'orchestration_history']) {
      expect(ORCH_RS, `${h} 的返回类型不是 ApiResponse<Value>`).toMatch(new RegExp(`async fn ${h}\\([\\s\\S]*?\\) -> ApiResponse<Value>`))
    }
  })

  it('本面 5 行已从台账移出，backlog 归零', () => {
    for (const id of ['experts.orch.orchestrate', 'experts.orch.plan_generate', 'experts.orch.plan_execute', 'experts.orch.stats', 'experts.orch.history']) {
      expect(UNMOUNTED_ROUTES.some((x) => x.registry === id), `${id} 仍留在台账`).toBe(false)
    }
    expect(UNMOUNTED_ROUTES.filter((x) => x.verdict === 'backlog').length, '台账仍有 backlog：先挂载或举证拒绝').toBe(0)
  })
})

describe('编排面出参键集 ↔ Rust json! 字面量', () => {
  it('orchestrate 的 8 键与它的三个子形状', () => {
    blockWith(ORCH_ORCHESTRATE_KEYS, 'orchestrate 响应')
    // 去幻影化后 experts 摘要只在 orchestrate(:610) 与 plan/generate(:695) 两处拼 json!；execute 不再回 experts 摘要
    blockWith(ORCH_EXPERT_SUMMARY_KEYS, 'experts 摘要', 2)
    blockWith(ORCH_PLAN_STEP_SUMMARY_KEYS, 'plan.steps[]')
    expect(ORCH_ORCHESTRATE_KEYS).toEqual(['orchestration_id', 'task', 'task_type', 'experts', 'plan', 'execution', 'result', 'created_at'])
  })

  it('execution 与 plan 两个内联包装的键集（不是独立 json!，只能按源码文本核）', () => {
    expect(ORCH_RS).toMatch(/"plan": \{\s*"plan_id": plan\.plan_id,\s*"steps": plan_steps_summary,\s*\}/)
    // 去幻影化后 execution.status 不再是字面量 "completed"，而是读真实的 plan.status（:633）
    expect(ORCH_RS).toMatch(new RegExp(
      `"execution": \\{${ORCH_EXECUTION_KEYS.map((k, i) => `\\s*"${k}": ${i === 0 ? 'plan\\.status' : '[^,\\n]+'},`).join('')}\\s*\\}`
    ))
  })

  it('plan/generate 的 8 键与 7 键步骤', () => {
    blockWith(ORCH_PLAN_GENERATE_KEYS, 'plan/generate 响应')
    blockWith(ORCH_PLAN_STEP_KEYS, 'plan/generate steps[]')
  })

  it('plan/execute 出参：单一形状，error/final_result 同为可空 Option（成环失败不再单独成块）', () => {
    // 去幻影化后 execute_plan 末尾只吐一个 json! 块（:468-471）：error 与 final_result 都是 Option——
    // 成功 error=null / 失败 final_result=null，不再是"成功 9 键 / 成环失败 8 键"两个不同键集的块。
    blockWith(['plan_id', 'execution_id', 'status', 'overall_status', 'steps_executed', 'steps_total', 'error', 'duration_ms', 'final_result', 'evidence_kind'], 'execute 出参形状')
    blockWith(ORCH_EXECUTED_STEP_KEYS, 'steps_executed[]')
  })

  it('stats 的 14 键与 top_used_experts 的 2 键', () => {
    blockWith(ORCH_STATS_KEYS, 'stats 响应')
    blockWith(ORCH_TOP_EXPERT_KEYS, 'top_used_experts[]')
    expect(ORCH_STATS_KEYS.length).toBe(14)
  })

  it('history 的 4 键没有 total_pages：页码总数只能前端算', () => {
    blockWith(ORCH_HISTORY_KEYS, 'history 响应')
    expect(ORCH_HISTORY_KEYS).not.toContain('total_pages')
  })

  it('融合结果 6 键与单步融合输入 5 键', () => {
    blockWith(ORCH_FUSION_KEYS, 'fuse_results 出参')
    // 去幻影化后单步产出改为真实咨询答复；喂给融合的中间块是 5 键（:452-453），不再是 simulate_step_execution 的 7 键
    blockWith(['summary', 'key_findings', 'confidence', 'source', 'evidence_kind'], '单步融合输入')
  })

  it('历史行的键集与 OrchestrationRecord 字段逐字等集，且没有 skip_serializing_if', () => {
    expect(structFields('OrchestrationRecord')).toEqual([...ORCH_RECORD_KEYS])
    expect(COMMON_RS.match(/pub struct OrchestrationRecord \{[\s\S]*?\n\}/)[0]).not.toMatch(/skip_serializing_if/)
  })
})

describe('ORCH_SIMULATED：每个常量仍钉在原来那一行', () => {
  it('清单非空且每条都给出位置、字面量、字段与界面话术', () => {
    expect(ORCH_SIMULATED.length).toBeGreaterThanOrEqual(8)
    for (const s of ORCH_SIMULATED) {
      expect(s.at, `${s.id} 缺位置`).toMatch(/experts_orchestration\.rs:\d+$/)
      expect(s.literal.length, `${s.id} 缺字面量`).toBeGreaterThan(4)
      expect(s.text.length, `${s.id} 缺说明`).toBeGreaterThan(10)
      expect(s.field.length, `${s.id} 缺字段名`).toBeGreaterThan(0)
    }
  })

  it('字面量逐条命中所声明的行号（行号漂了说明这一面又变了）', () => {
    for (const s of ORCH_SIMULATED) {
      const line = Number(s.at.split(':')[1])
      expect(ORCH_LINES[line - 1] || '', `${s.id}：${s.at} 这一行不再包含 ${s.literal}`).toContain(s.literal)
    }
  })

  it('档位取自唯一来源表，真实终态、计时与模型产出不能标成常量', () => {
    for (const [path, tier] of Object.entries(ORCH_FIELD_PROVENANCE)) {
      expect(ORCH_PROVENANCE[tier], `${path} 的档位 ${tier} 未定义`).toBeTruthy()
    }
    expect(ORCH_FIELD_PROVENANCE['plan.status']).toBe('literal')
    expect(ORCH_FIELD_PROVENANCE['stats.plans_ready']).toBe('literal')
    for (const p of ['orchestrate.execution.status', 'execute.steps_executed[].status', 'stats.plans_failed']) expect(ORCH_FIELD_PROVENANCE[p]).toBe('real')
    expect(ORCH_FIELD_PROVENANCE['execute.steps_executed[].duration_ms']).toBe('wallclock')
    expect(ORCH_FIELD_PROVENANCE['execute.steps_executed[].result.confidence']).toBe('model')
    expect(ORCH_RS).toContain('"status": plan.status')
    expect(ORCH_RS).toContain('step_start.elapsed().as_millis()')
    expect(ORCH_RS).toContain('generate_expert_answer(expert, &question).await')
  })
})

describe('恒为 0 的统计项与两个 status 口径', () => {
  it('plan.status 的直赋只有 running；终态 failed/completed/partial 走条件三选一，ready 无写入路径', () => {
    // 去幻影化后赋值语法由 .to_string() 改为 .into()；终态（:457-464）按 error 与是否全完成三选一。
    const assigns = [...new Set([...ORCH_RS.matchAll(/plan\.status = "(\w+)"\.into\(\)/g)].map((m) => m[1]))]
    expect(assigns.sort()).toEqual(['running'])
    expect(ORCH_RS).toContain('status: "draft".to_string()')
    expect(ORCH_RS).toMatch(/plan\.status = if error\.is_some\(\) \{\s*"failed"/)
    expect(ORCH_RS).not.toMatch(/plan\.status = "ready"/)
    expect(ORCH_ZERO_COUNTERS).toEqual(['plans_ready'])
    for (const k of ORCH_ZERO_COUNTERS) expect(ORCH_STATS_KEYS).toContain(k)
  })

  it('成环错误折叠进统一结果：topological_sort 出错写入 error，终态条件把 plan.status 置 failed', () => {
    // 去幻影化后没有独立的 `Err(e) => { return json!({...}) }` 分支；环检测错误先写 error（:398），
    // 再由 :457-464 的终态条件把 plan.status 置 failed——失败会被 stats 与历史如实记录。
    expect(ORCH_RS).toContain('let mut error = order.as_ref().err().cloned();')
    expect(ORCH_RS).toMatch(/plan\.status = if error\.is_some\(\) \{\s*"failed"/)
  })

  it('orchestrate 走真实咨询 DAG，历史行 status 读真实 plan.status（不再写死 completed）', () => {
    const handler = ORCH_RS.match(/async fn orchestrate\([\s\S]*?\n\}/)[0]
    expect(handler).toContain('let exec_result = execute_plan(&mut plan, None, &matched_experts).await;')
    // 去幻影化后历史行 status 读 plan.status.clone()（:593），不再是字面量 "completed"。
    expect(handler).toContain('status: plan.status.clone()')
    expect(handler).not.toContain('status: "completed".to_string()')
  })

  it('zero 计数有专属说明；两口径对照只在历史真出现 failed 时给', () => {
    expect(orchZeroCounterNote('plans_ready')).toMatch(/恒为 0/)
    expect(orchZeroCounterNote('plans_failed')).toBe('')
    expect(orchZeroCounterNote('plans_completed')).toBe('')
    expect(orchStatusSplitNote({ plans_failed: 0 }, [{ status: 'failed' }])).toMatch(/两个口径/)
    expect(orchStatusSplitNote({ plans_failed: 0 }, [{ status: 'completed' }])).toBe('')
  })
})

describe('请求体：发哪些键由 handler 读什么决定', () => {
  it('三个请求体 struct 的字段清单与契约拼装一致', () => {
    expect(structFields('OrchestrateBody', ORCH_RS, false)).toEqual(['task', 'task_type', 'expert_ids', 'max_experts', 'fusion_strategy'])
    expect(structFields('GeneratePlanBody', ORCH_RS, false)).toEqual(['task', 'task_type', 'expert_ids', 'fusion_strategy'])
    expect(structFields('ExecutePlanBody', ORCH_RS, false)).toEqual(['plan_id', 'step_ids'])
    expect(orchestrateBody({ task: ' 画个架构图 ', taskType: 'development', fusionStrategy: 'weighted', maxExperts: 2, expertIds: ['a', '', null] }))
      .toEqual({ task: '画个架构图', task_type: 'development', fusion_strategy: 'weighted', max_experts: 2, expert_ids: ['a'] })
    expect(orchestrateBody({ task: 'x' })).toEqual({ task: 'x' })
    expect(planGenerateBody({ task: 'x', expertIds: [] })).toEqual({ task: 'x' })
  })

  it('缺省常量与后端 unwrap_or 的字面量同源', () => {
    expect(ORCH_MAX_EXPERTS_DEFAULT).toBe(3)
    expect(ORCH_RS).toContain('body.max_experts.unwrap_or(3)')
    expect(ORCH_DEFAULT_TASK_TYPE).toBe('general')
    expect(ORCH_DEFAULT_FUSION_STRATEGY).toBe('weighted')
    expect(ORCH_RS).toContain('unwrap_or_else(|| "general".into())')
    expect(ORCH_RS).toContain('unwrap_or_else(|| "weighted".into())')
  })

  it('max_experts=0 必须发出去（0 与"不发"在后端是两种结果）', () => {
    expect(orchestrateBody({ task: 'x', maxExperts: 0 })).toEqual({ task: 'x', max_experts: 0 })
    expect(planGenerateBody({ task: 'x' })).not.toHaveProperty('max_experts')
  })

  it('step_ids 空数组与不发是两种语义，不能像 expert_ids 那样省略', () => {
    expect(planExecuteBody({ planId: 'p1', stepIds: [] })).toEqual({ plan_id: 'p1', step_ids: [] })
    expect(planExecuteBody({ planId: 'p1' })).toEqual({ plan_id: 'p1' })
    expect(planExecuteBody({ planId: 'p1', stepIds: ['s1', ' s2 '] })).toEqual({ plan_id: 'p1', step_ids: ['s1', 's2'] })
    // 去幻影化后变量改名 execute_set→selected，仍是 step_ids.map(...) 收集成 HashSet（:397）
    expect(ORCH_RS).toContain('let selected = step_ids.map(|ids| ids.into_iter().collect::<std::collections::HashSet<_>>());')
  })

  it('plan_id 没有 serde default：缺了会在请求层被拒，页面拿不到 {code,msg} 信封', () => {
    const execBody = ORCH_RS.match(/struct ExecutePlanBody \{([\s\S]*?)\n\}/)[1]
    expect(execBody).toMatch(/\n\s{4}plan_id: String,/)
    expect(execBody).not.toMatch(/#\[serde\(default\)\]\s*\n\s*plan_id/)
    expect(planExecuteProblem({ planId: '  ' })).toMatch(/plan_id/)
    expect(planExecuteProblem({ planId: 'p1' })).toBe('')
  })

  it('空 task 前置拦下：后端对空白 task 直接 400', () => {
    expect(ORCH_RS).toContain('if body.task.trim().is_empty()')
    expect(orchestrateProblem({ task: '   ' })).toMatch(/400/)
    expect(orchestrateProblem({ task: 'x', maxExperts: 1.5 })).toMatch(/非负整数/)
    expect(orchestrateProblem({ task: 'x' })).toBe('')
  })

  it('一律发正名 task，不用后端的 question 别名', () => {
    expect(ORCH_RS.match(/#\[serde\(default, alias = "question"\)\]/g).length).toBe(2)
    const keys = [...Object.keys(orchestrateBody({ task: 'x' })), ...Object.keys(planGenerateBody({ task: 'x' }))]
    expect(keys).not.toContain('question')
    expect(keys).toContain('task')
  })
})

describe('步骤表与拓扑形状', () => {
  it('只有 4 个 task_type 有专用表，其余（含后端默认 general）落到兜底', () => {
    const fn = ORCH_RS.match(/fn select_steps_for_task_type[\s\S]*?\n\}\n/)[0]
    expect([...fn.matchAll(/^\s{8}"(\w+)" =>/gm)].map((m) => m[1])).toEqual(['research', 'consulting', 'development', 'analysis'])
    expect(ORCH_TASK_TYPE_TABLES).toEqual(['research', 'consulting', 'development', 'analysis'])
    expect(orchUsesFallbackTable('general')).toBe(true)
    expect(orchUsesFallbackTable('research')).toBe(false)
  })

  it('各表的步数与契约记法一致（development 与兜底各 6 步，其余三张 5 步）', () => {
    const fn = ORCH_RS.match(/fn select_steps_for_task_type([\s\S]*?)\n\}\n/)[1]
    const counts = {}
    let current = null
    for (const line of fn.split('\n')) {
      const arm = line.match(/^\s{8}"(\w+)" =>/)
      if (arm) current = arm[1]
      else if (/^\s{8}_ =>/.test(line)) current = '__fallback'
      else if (current && /\("[a-z_]+", "/.test(line)) counts[current] = (counts[current] || 0) + 1
    }
    expect(counts).toEqual({ ...ORCH_STEP_COUNT_BY_TASK_TYPE })
    // 界面文案里的步数取自同一张表：改表不改文案会在这里红
    expect(orchFallbackNote()).toContain(`${ORCH_STEP_COUNT_BY_TASK_TYPE.__fallback} 步通用文案`)
  })

  it('step_type 文案表就是契约列出的 7 类', () => {
    // 去幻影化后 simulate_step_execution 已删；step_type 改为 select_steps_for_task_type 元组表的首元素（:100-141）
    const fn = ORCH_RS.match(/fn select_steps_for_task_type\([\s\S]*?\n\}/)[0]
    const types = [...new Set([...fn.matchAll(/\("(\w+)", "[^"]*", "[^"]*"\)/g)].map((m) => m[1]))].sort()
    expect(types).toEqual([...ORCH_STEP_TYPES].sort())
  })

  it('生成的依赖恒为单链：每步只依赖上一步，所以本面的图不可能成环', () => {
    expect(ORCH_RS).toMatch(/let depends_on = match prev_step_id \{\s*Some\(ref prev\) => vec!\[prev\.clone\(\)\],\s*None => vec!\[\],/)
    const chain = [{ stepId: 'a', dependsOn: [] }, { stepId: 'b', dependsOn: ['a'] }, { stepId: 'c', dependsOn: ['b'] }]
    expect(orchTopologyNote(chain)).toMatch(/单链/)
    expect(orchTopologyNote([{ stepId: 'a', dependsOn: ['b'] }, { stepId: 'b', dependsOn: ['a'] }])).toMatch(/不是单链/)
    expect(orchTopologyNote([])).toMatch(/没有生成任何步骤/)
  })

  it('步骤专家可以全为 null：候选池为空时后端直接给 None', () => {
    expect(ORCH_RS).toContain('let assigned_expert = if experts.is_empty()')
    expect(orchStepExpertNote([{ stepId: 'a', expertId: '' }])).toMatch(/expert_id 都是 null/)
    expect(orchStepExpertNote([{ stepId: 'a', expertId: 'exp-1' }])).toBe('')
  })

  it('零位专家的响应有专门话术，非空时没有；门槛 0.2 与源码同值', () => {
    expect(orchEmptyExpertsNote({ experts: [] })).toMatch(/没有选出任何专家/)
    expect(orchEmptyExpertsNote({ experts: [{ id: 'a' }] })).toBe('')
    expect(orchEmptyExpertsNote({})).toBe('')
    expect(ORCH_MATCH_SCORE_FLOOR).toBe(0.2)
    expect(ORCH_RS).toContain('*s > 0.2')
  })
})

describe('分页与状态判定', () => {
  it('历史面不走 parse_pagination，page_size 后端不夹取', () => {
    const handler = ORCH_RS.match(/async fn orchestration_history\([\s\S]*?\n\}/)[0]
    expect(handler).toContain('params.get("page_size")')
    expect(handler).not.toContain('parse_pagination')
    expect(ORCH_RS).toMatch(/let page_size: usize = params\.get\("page_size"\)\.and_then\(\|v\| v\.parse\(\)\.ok\(\)\)\.unwrap_or\(20\);/)
    expect(ORCH_RS).toMatch(/\.skip\(offset\)\.take\(page_size\)/)
  })

  it('请求侧夹取到 1..=200，缺省回落后端默认的 20', () => {
    // 200 与 20 都从 experts_common.rs 现取：本面不走 parse_pagination，
    // 但前端自设的上限必须与网关其它分页同口径，比对契约常量本身不算证据。
    const gatewayMax = Number(COMMON_RS.match(/let page_size = page_size\.clamp\(1, (\d+)\);/)[1])
    const gatewayDefault = Number(ORCH_RS.match(/let page_size: usize = params\.get\("page_size"\)\.and_then\(\|v\| v\.parse\(\)\.ok\(\)\)\.unwrap_or\((\d+)\);/)[1])
    expect(gatewayMax).toBe(200)
    expect(ORCH_HISTORY_PAGE_SIZE_MAX).toBe(gatewayMax)
    expect(orchHistoryQuery({})).toEqual({ page: 1, page_size: gatewayDefault })
    expect(orchHistoryQuery({ page: 0, pageSize: 9999 })).toEqual({ page: 1, page_size: gatewayMax })
    expect(orchHistoryQuery({ page: '3', pageSize: '50', status: ' failed ', taskType: 'research' }))
      .toEqual({ page: 3, page_size: 50, status: 'failed', task_type: 'research' })
  })

  it('page_size=0 是真实可达的空页陷阱', () => {
    expect(orchHistoryAnomaly({ page_size: 0, total: 7, records: [] })).toMatch(/take\(0\)/)
    expect(orchHistoryAnomaly({ page_size: 20, total: 7, records: [1, 2] })).toBe('')
    expect(orchHistoryPages({ total: 7, page_size: 0 }).pages).toBe(0)
    expect(orchHistoryPages({ total: 45, page_size: 20 })).toEqual({ total: 45, pages: 3, computed: true })
  })

  it('成环失败是 200 + body 里的 failed：判定只看 body', () => {
    expect(ORCH_RS.match(/async fn execute_plan_handler\([\s\S]*?\n\}/)[0]).toContain('ok(result)')
    const failed = { status: 'failed', error: 'topological sort failed: cycle detected' }
    expect(orchExecuteOutcome(failed).failed).toBe(true)
    expect(orchExecuteOutcome(failed).note).toMatch(/HTTP 200/)
    expect(orchExecuteOutcome({ status: 'completed' }).failed).toBe(false)
    expect(orchExecuteOutcome({ status: 'completed' }).note).toBe('')
    expect(orchExecuteOutcome({}).failed).toBe(false)
  })

  it('内存态仍是权威，D4 已把 plans/history 投影 SQLite；演练提示固定话术', () => {
    expect(orchVolatileNote()).toMatch(/SQLite/)
    expect(orchRunDisclaimer()).toMatch(/Kahn/)
    expect(COMMON_RS).toContain('pub plans: Arc<Mutex<HashMap<String, CollaborationPlan>>>')
    expect(COMMON_RS).toContain('pub orchestration_history: Arc<Mutex<Vec<OrchestrationRecord>>>')
    // D4 落盘：DB 层已有 plans/history 的写读投影（upsert_plan / insert_history_record）
    expect(DB_RS).toContain('fn upsert_plan')
    expect(DB_RS).toContain('fn insert_history_record')
  })
})

describe('归一化产物与 wire 键一一对应', () => {
  it('orchestrate：包装层级被摊平，但一个 wire 键都不丢', () => {
    const raw = {
      orchestration_id: 'exec-1', task: 't', task_type: 'general',
      experts: [{ id: 'e1', name: 'n', title: 'p' }],
      plan: { plan_id: 'plan-1', steps: [{ step_id: 's1', name: '需求分析', status: 'completed', depends_on: [] }] },
      execution: { status: 'completed', steps_completed: 1, steps_total: 1, duration_ms: 12 },
      result: { summary: 's', key_findings: ['a'], step_summaries: ['b'], recommendations: ['c'], confidence: 0.85, fusion_strategy: 'weighted' },
      created_at: '2026-09-24T00:00:00Z'
    }
    const out = normOrchestration(raw)
    expect(Object.keys(out).sort()).toEqual(['orchestrationId', 'task', 'taskType', 'experts', 'planId', 'steps', 'execution', 'result', 'createdAt'].sort())
    expect(out.steps[0]).toEqual({ stepId: 's1', name: '需求分析', description: '', expertId: '', stepType: '', dependsOn: [], status: 'completed' })
    expect(out.experts[0]).toEqual({ id: 'e1', name: 'n', title: 'p' })
    expect(out.execution).toEqual({ status: 'completed', stepsCompleted: 1, stepsTotal: 1, durationMs: 12 })
    expect(out.result.confidence).toBe(0.85)
    expect(normOrchestration(null).experts).toEqual([])
    expect(normOrchestration(null).result).toBe(null)
  })

  it('plan/generate 的键与 wire 等集', () => {
    const plan = normOrchPlan({ plan_id: 'p', task: 't', task_type: 'research', experts: [], steps: [], fusion_strategy: 'weighted', status: 'draft', created_at: 'x' })
    expect(Object.keys(plan).sort()).toEqual(ORCH_PLAN_GENERATE_KEYS.map(snakeToCamel).sort())
    expect(plan.status).toBe('draft')
  })

  it('execute 的两种形状归一化后仍可辨：failed 时 finalResult 为 null、error 有值', () => {
    const expected = [...ORCH_EXECUTE_KEYS.map(snakeToCamel), 'error'].sort()
    const ok = normOrchExecution({ plan_id: 'p', execution_id: 'e', status: 'completed', overall_status: 'completed', steps_executed: [], steps_total: 2, completed_at: 't', duration_ms: 3, final_result: null })
    expect(Object.keys(ok).sort()).toEqual(expected)
    expect(ok.completedAt).toBe('t')
    expect(ok.error).toBe('')
    const failed = normOrchExecution({ plan_id: 'p', execution_id: 'e', status: 'failed', error: 'cycle', steps_executed: [], steps_total: 2, overall_status: 'failed', duration_ms: 0 })
    expect(Object.keys(failed).sort()).toEqual(expected)
    expect(failed.finalResult).toBe(null)
    expect(failed.completedAt).toBe('')
    expect(failed.error).toBe('cycle')
    const withSteps = normOrchExecution({ steps_executed: [{ step_id: 's1', name: 'n', status: 'completed', duration_ms: 15, result: { step_id: 's1', step_type: 'intake', summary: 's', key_findings: [], expert: null, confidence: 0.85, executed_at: 't' } }] })
    expect(Object.keys(withSteps.stepsExecuted[0]).sort()).toEqual(ORCH_EXECUTED_STEP_KEYS.map(snakeToCamel).sort())
    expect(withSteps.stepsExecuted[0].result.expert).toBe(null)
  })

  it('stats：13 个直译 + ts→serverTs，两个分布原样透传', () => {
    // 每个数值都给互不相同的非零值：只比键集的话，读错 wire 键会得到 undefined→0，
    // 一个恰好为 0 的字段就把这种错完全遮住（变异体 Q9 正是这么活下来的）。
    const raw = { total_plans: 11, plans_draft: 12, plans_ready: 13, plans_running: 14, plans_completed: 15, plans_failed: 16, total_executions: 17, success_rate: 0.6667, avg_duration_ms: 18, avg_steps_per_plan: 19, top_used_experts: [{ expert_id: 'e1', usage_count: 2 }], fusion_strategy_distribution: { weighted: 2 }, task_type_distribution: { research: 3 }, ts: 't' }
    const out = normOrchStats(raw)
    expect(Object.keys(out).sort()).toEqual([...ORCH_STATS_KEYS.filter((k) => k !== 'ts').map(snakeToCamel), 'serverTs'].sort())
    for (const k of ORCH_STATS_KEYS) {
      if (k === 'ts' || k === 'top_used_experts' || k.endsWith('_distribution')) continue
      expect(out[snakeToCamel(k)], `${k} 没有从自己的 wire 键取值`).toEqual(raw[k])
    }
    expect(out.topUsedExperts[0]).toEqual({ expertId: 'e1', usageCount: 2 })
    expect(out.serverTs).toBe('t')
    expect(normOrchStats(null).fusionStrategyDistribution).toEqual({})
  })

  it('history：12 个行字段一个不丢，result 为 null 时不编造对象', () => {
    const out = normOrchHistory({
      records: [{
        execution_id: 'e', plan_id: 'p', task_type: 'research', status: 'failed', expert_ids: ['a'],
        steps_completed: 0, steps_total: 3, result_summary: '', result: null, created_at: 'c', completed_at: null, duration_ms: 0
      }],
      total: 1, page: 1, page_size: 20
    })
    expect(Object.keys(out).sort()).toEqual(ORCH_HISTORY_KEYS.map(snakeToCamel).sort())
    expect(Object.keys(out.records[0]).sort()).toEqual(ORCH_RECORD_KEYS.map(snakeToCamel).sort())
    expect(out.records[0].result).toBe(null)
    expect(out.records[0].completedAt).toBe('')
  })

  it('来源角标：literal 与 simulated 各自成档，未登记的字段返回 null 由界面按真实计算渲染', () => {
    expect(orchProvenanceOf('orchestrate.execution.status').tier).toBe('real')
    expect(orchProvenanceOf('orchestrate.result.summary').tier).toBe('model')
    expect(orchProvenanceOf('orchestrate.experts').tier).toBe('real')
    expect(orchProvenanceOf('stats.total_plans').tier).toBe('real')
    expect(orchProvenanceOf('未登记字段')).toBe(null)
    expect(ORCH_PROVENANCE.literal.tone).toBe('danger')
    expect(ORCH_PROVENANCE.simulated.tone).toBe('warning')
  })
})

describe('视图与 store 只做装配：口径全部取自契约', () => {
  const template = AOV.slice(0, AOV.indexOf('<script setup>'))

  it('界面每个带角标的字段路径都在来源清单里登记过', () => {
    const used = [...AOV.matchAll(/tier(?:Label|Of)\('([^']+)'\)/g)].map((m) => m[1])
    expect([...new Set(used)].length, '解析不出界面用的路径，这条断言就是空转').toBeGreaterThan(5)
    for (const p of new Set(used)) expect(ORCH_FIELD_PROVENANCE[p], `${p} 未登记：角标会静默降级成"真实计算"`).toBeTruthy()
  })

  it('兜底表话术由契约给出，界面不另写一份步数', () => {
    expect(template).toContain('store.fallbackNote')
    expect(ORS).toMatch(/const fallbackNote = computed\(\(\) => \(fallbackTable\.value \? orchFallbackNote\(\) : ''\)\)/)
    expect(template).not.toMatch(/兜底表（\d+ 步/)
  })

  it('常量图例转发整张 ORCH_SIMULATED，不筛不减', () => {
    expect(ORS).toMatch(/const simulatedLegend = computed\(\(\) => ORCH_SIMULATED\.map\(\(x\) => \(\{ id: x\.id, field: x\.field, text: x\.text, at: x\.at \}\)\)\)/)
    expect(template).toContain('store.simulatedLegend')
  })

  it('界面静态说明里引的每个源码坐标都真实存在', () => {
    const cited = [...template.matchAll(/:(\d{1,4})(?:-(\d{1,4}))?/g)].flatMap((m) => [m[1], m[2]].filter(Boolean).map(Number))
    expect(cited.length).toBeGreaterThan(8)
    for (const n of new Set(cited)) {
      expect(n, `界面引了 :${n}，但 Rust 只有 ${ORCH_LINES.length} 行`).toBeLessThanOrEqual(ORCH_LINES.length)
      expect(ORCH_LINES[n - 1].trim().length, `:${n} 指向空行`).toBeGreaterThan(0)
    }
  })
})
