// 模式拓扑契约门禁：断言全部指向 alliance.rs 的 build_dag_for_task、planner.rs 的动态选型、
// dag_engine.rs 的分支消费三处真实文本。三条设计约束：
// ① 每模式的节点清单按「大括号计数取 match 分支 + 按序取 n(...) 调用」解析，不靠行号切片；
// ② 首末节点的契约位（恒 Completed / 恒融合输出）与 Running 个数都参与比对——
//    后端把 Running 挪一个位置，本文件就要红，因为界面那句"N 个恒记 Running"是照本表说的；
// ③ dynamic 的"分支不在响应里"是**否定式**结论，所以它必须有正面对手：
//    planner 里确有 routes.push、执行器里确有消费点，而网关那份出参文本里零命中；
// ④ 引用里的行号一律按大括号配对重算后再比对，不做"字符串里含 alliance.rs:4xx"的装饰性检查——
//    写下 planner.rs:332-401 时被这一条抓出真实结束行是 411，装饰性检查永远发现不了这种错。
import { describe, it, expect } from 'vitest'
import { readFileSync, existsSync } from 'node:fs'
import path from 'node:path'

import {
  DYNAMIC_BRANCH, MODE_AXIS_NOTE, MODE_CONTRACT_NODES, MODE_TOPOLOGY,
  modePendingCount, modeTopologyNote, modeTopologyOf, modeWireOf
} from './mode.js'
import { MODE_DISPLAY, MODE_WIRE, modeLabel } from './enums.js'

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
const ALLIANCE_RS = read('platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs')
const PLANNER_RS = read('platform/domains/alliance/core/mox-alliance-scheduler-core/src/planner.rs')
const DAG_ENGINE_RS = read('platform/domains/alliance/core/mox-alliance-executor-core/src/dag_engine.rs')
const TYPES_RS = read('platform/domains/alliance/proto/mox-alliance-common-proto/src/types.rs')
const ACV = read('frontend-ui/src/modules/expert-alliance/views/AllianceConsoleView.vue')

/** 大括号计数取块：返回 start 处那个 `{` 配对的闭括号下标（含） */
function blockEnd(src, openIdx) {
  let depth = 0
  for (let i = openIdx; i < src.length; i++) {
    if (src[i] === '{') depth++
    else if (src[i] === '}') { depth--; if (depth === 0) return i }
  }
  throw new Error('未配对的左花括号')
}

const FN_HEAD = 'fn build_dag_for_task'
const fnStart = ALLIANCE_RS.indexOf(FN_HEAD)
const fnBody = ALLIANCE_RS.slice(fnStart, blockEnd(ALLIANCE_RS, ALLIANCE_RS.indexOf('{', fnStart)) + 1)

/** 解析出的每个 match 分支：{ variant, nodes: [{ name, expert, status }] } */
const ARMS = (() => {
  const out = []
  const re = /AllianceMode::(\w+)\s*=>\s*\{/g
  let m
  while ((m = re.exec(fnBody))) {
    const end = blockEnd(fnBody, m.index + m[0].length - 1)
    const arm = fnBody.slice(m.index + m[0].length, end)
    const calls = [...arm.matchAll(/\bn\(\s*"([^"]+)",\s*"([^"]+)",\s*NodeExecStatus::(\w+)/g)]
    out.push({ variant: m[1], nodes: calls.map((c) => ({ name: c[1], expert: c[2], status: c[3].toLowerCase() })) })
  }
  return out
})()

const byVariant = new Map(ARMS.map((a) => [a.variant, a]))
/** 契约表的键是 wire 值（snake_case），Rust 分支名是其 PascalCase 变体 */
const variantOfWire = (wire) => wire.split('_').map((s) => s[0].toUpperCase() + s.slice(1)).join('')
const armForWire = (wire) => byVariant.get(variantOfWire(wire))
const wireOf = (variant) => variant.replace(/([a-z])([A-Z])/g, '$1_$2').toLowerCase()

describe('模式拓扑表与网关查表一一对上', () => {
  it('解析到了 build_dag_for_task 的七个分支，且每分支至少三个节点', () => {
    expect(ARMS.length, '分支数变了：契约表与界面措辞都要跟着改').toBe(7)
    expect(new Set(ARMS.map((a) => a.variant)).size).toBe(7)
    for (const a of ARMS) {
      expect(a.nodes.length, `${a.variant} 一个节点都没解析到（写法变了？）`).toBeGreaterThan(2)
    }
  })

  it('MODE_TOPOLOGY 的键集恰是 MODE_WIRE 的七个值（既不缺也不多）', () => {
    expect(Object.keys(MODE_TOPOLOGY).sort()).toEqual(Object.values(MODE_WIRE).sort())
    for (const a of ARMS) {
      expect(MODE_TOPOLOGY, `${a.variant} → ${wireOf(a.variant)} 不在契约表里`).toHaveProperty(wireOf(a.variant))
    }
  })

  it('每模式的节点数、Running 个数与按序节点名逐字对齐 Rust', () => {
    for (const [wire, top] of Object.entries(MODE_TOPOLOGY)) {
      const arm = armForWire(wire)
      expect(arm, `${wire} 在 Rust 里没有对应分支`).toBeTruthy()
      expect(arm.nodes.length, `${wire} 节点数与 Rust 不符`).toBe(top.nodes)
      expect(arm.nodes.map((n) => n.name), `${wire} 节点清单与 Rust 不符`).toEqual(top.names)
      expect(arm.nodes.filter((n) => n.status === 'running').length, `${wire} 的 Running 个数变了`).toBe(top.running)
    }
  })

  it('首末节点契约位成立：首恒 Completed、末恒融合输出', () => {
    for (const [wire, top] of Object.entries(MODE_TOPOLOGY)) {
      const arm = armForWire(wire)
      expect(arm.nodes[0].name, `${wire} 首节点不是契约里的需求分析`).toBe(MODE_CONTRACT_NODES.first)
      expect(arm.nodes[0].status, `${wire} 首节点状态不再是 completed——那句"恒 Completed"要收`).toBe('completed')
      expect(arm.nodes[top.nodes - 1].name, `${wire} 末节点不再是融合出口`).toBe(MODE_CONTRACT_NODES.last)
      expect(arm.nodes[top.nodes - 1].expert).toBe('expert-fusion')
    }
  })

  it('Pending 个数由契约算出且与 Running+Completed 加起来等于节点数', () => {
    for (const [wire, top] of Object.entries(MODE_TOPOLOGY)) {
      expect(modePendingCount(wire)).toBe(top.nodes - 1 - top.running)
      expect(top.nodes - 1 - top.running, `${wire} 的 Pending 数为负（模板写错了）`).toBeGreaterThanOrEqual(0)
    }
    expect(modePendingCount('nope')).toBe(0)
    expect(modeTopologyOf('nope')).toBe(null)
  })

  it('Rust 文档注释只列了六种模式拓扑，而分支有七个——前端不得跟着说"六"', () => {
    const doc = ALLIANCE_RS.slice(0, fnStart)
    const claimed = (doc.match(/^\/\/\/ - \w+/gm) || []).length
    expect(claimed, '那条注释补上了第七种？连这段说明一起重看').toBe(6)
    expect(ARMS.length).toBeGreaterThan(claimed)
    for (const wire of Object.values(MODE_WIRE)) {
      expect(modeTopologyNote(wire), `${wire} 的措辞里出现了"六种"`).not.toMatch(/六\s*[种个]/)
    }
  })
})

describe('dynamic 模式：网关画的是一个节点，分支不在响应里', () => {
  it('网关侧确有「动态路由」这个节点，且它是普通 expert 节点', () => {
    const arm = byVariant.get('Dynamic')
    expect(arm.nodes.map((n) => n.name)).toContain(DYNAMIC_BRANCH.gatewayNode)
    expect(arm.nodes.find((n) => n.name === DYNAMIC_BRANCH.gatewayNode).expert).toBe(DYNAMIC_BRANCH.gatewayExpert)
    expect(arm.nodes.filter((n) => n.name === DYNAMIC_BRANCH.gatewayNode).length, '分支节点不止一个？措辞要改').toBe(1)
  })

  it('真实选型在 planner 的 generate_dynamic_plan，且确实写了路由', () => {
    expect(PLANNER_RS).toMatch(/fn generate_dynamic_plan\(/)
    expect(PLANNER_RS).toMatch(/routes\.push\(PlanDynamicRoute\s*\{/)
    expect(PLANNER_RS).toMatch(/decision_node: "node-decision"/)
    expect(TYPES_RS).toMatch(/pub struct PlanDynamicRoute \{[\s\S]*?pub true_branch: Vec<String>,[\s\S]*?pub false_branch: Vec<String>,/)
  })

  it('路由只在执行器内存里被消费，never 出现在网关出参里', () => {
    expect(DYNAMIC_BRANCH.onWire).toBe(false)
    expect(DAG_ENGINE_RS).toMatch(/fn build_dynamic_routes\(plan: &CollaborationPlan\) -> Option</)
    expect(DAG_ENGINE_RS).toMatch(/if plan\.dynamic_routes\.is_empty\(\) \{\s*return None/)
    expect(ALLIANCE_RS, '网关开始把 dynamic_routes 发出去了：这条否定结论要翻案').not.toMatch(/dynamic_routes/)
  })

  it('dynamic 的说明同时交代"节点"与"分支缺席"', () => {
    const note = modeTopologyNote(MODE_WIRE.DYNAMIC)
    expect(note).toContain(DYNAMIC_BRANCH.gatewayNode)
    expect(note).toContain(DYNAMIC_BRANCH.plannerAt)
    expect(note).toContain(DYNAMIC_BRANCH.routeStruct)
    expect(note).toMatch(/图上没有分支/)
    expect(note).toContain('不在任何响应里')
  })
})

describe('措辞与界面接线', () => {
  it('每个模式的说明都点出"展示态/模板"与源码坐标，且两条线名都出现', () => {
    for (const wire of Object.values(MODE_WIRE)) {
      const note = modeTopologyNote(wire)
      expect(note, `${wire} 没说这是模板`).toMatch(/展示态拓扑/)
      expect(note, `${wire} 没说这不是进度`).toMatch(/不是执行读数/)
      expect(note, `${wire} 没给后端坐标`).toMatch(/alliance\.rs:4(39|41)/)
      expect(note, `${wire} 没交代 mode_serde 线名`).toContain(wire)
      expect(note, `${wire} 没交代 mode_display 线名`).toContain(MODE_DISPLAY[Object.keys(MODE_WIRE).find((k) => MODE_WIRE[k] === wire)])
      expect(note).toContain('恒记 Running')
    }
  })

  it('拓扑表按两条线都能查：出参给的是 mode_display，请求发的是 mode_serde', () => {
    expect(modeWireOf('expert_alliance')).toBe(MODE_WIRE.PARALLEL)
    expect(modeWireOf('single_expert')).toBe(MODE_WIRE.SEQUENTIAL)
    expect(modeWireOf('dynamic')).toBe(MODE_WIRE.DYNAMIC)
    expect(modeWireOf('autonomous')).toBe(MODE_WIRE.HIERARCHICAL)
    expect(modeTopologyOf('expert_alliance').nodes).toBe(MODE_TOPOLOGY[MODE_WIRE.PARALLEL].nodes)
    expect(modeWireOf('nonsense')).toBe('')
    for (const wire of Object.values(MODE_WIRE)) {
      const display = MODE_DISPLAY[Object.keys(MODE_WIRE).find((k) => MODE_WIRE[k] === wire)]
      expect(modeTopologyNote(display), `${display} 查不到拓扑`).not.toMatch(/按未知处理/)
    }
  })

  it('未知模式不编造拓扑，而是指名那个值', () => {
    const note = modeTopologyNote('octopus')
    expect(note).toContain('octopus')
    expect(note).toMatch(/按未知处理/)
    expect(modeTopologyNote(undefined)).toMatch(/模式值：未知/)
  })

  it('任务模式与工作台六模式两根轴各自有名分，不互相顶替', () => {
    expect(MODE_AXIS_NOTE).toMatch(/七个/)
    expect(MODE_AXIS_NOTE).toMatch(/六个不同的请求端点/)
    expect(Object.keys(MODE_WIRE).length).toBe(7)
    expect(Object.keys(MODE_DISPLAY).length).toBe(7)
    expect(TYPES_RS).toMatch(/pub enum AllianceMode \{[\s\S]*?Sequential[\s\S]*?Parallel[\s\S]*?Debate[\s\S]*?Hierarchical[\s\S]*?Iterative[\s\S]*?Voting[\s\S]*?Dynamic,/)
    for (const wire of Object.values(MODE_WIRE)) expect(modeLabel(wire), `${wire} 没有中文名`).not.toBe(wire)
  })

  it('控制台把这段说明接在 DAG 页签上，且字符串来自契约', () => {
    expect(ACV).toMatch(/modeTopologyNote\(/)
    const template = ACV.slice(0, ACV.indexOf('<script setup>'))
    expect(template).toMatch(/dagModeNote|modeTopologyNote/)
    // 只证明"调了这个函数"不够：读错字段（runtime.mode 是本地预览开关，不是任务模式）照样绿
    expect(ACV, '说明条取的不是任务的 mode 字段').toMatch(/modeTopologyNote\(store\.detail\.task\?\.mode\)/)
  })

  it('模式下拉由 MODE_WIRE 派生，界面没有另写一份短清单', () => {
    expect(ACV).toMatch(/Object\.entries\(MODE_WIRE\)\.map/)
    const hardcoded = ACV.match(/\[\s*'sequential'[\s\S]*?\]/) || ACV.match(/\[\s*'parallel'[\s\S]*?\]/)
    expect(hardcoded, '界面写死了一份模式清单：会漏掉 dynamic').toBeFalsy()
  })
})

describe('引用坐标指向真文本', () => {
  const line = (src, n) => src.split('\n')[n - 1] || ''
  /** 从 fn 头往后按大括号配对算出真实结束行（行号只有这么算才不会是装饰） */
  const fnEndLine = (src, head) => {
    const start = src.indexOf(head)
    expect(start, `找不到 ${head}`).toBeGreaterThan(-1)
    return src.slice(0, blockEnd(src, src.indexOf('{', start)) + 1).split('\n').length
  }

  it('模式说明里的 alliance.rs 两截坐标逐行命中契约注释与函数本体', () => {
    for (const wire of Object.values(MODE_WIRE)) {
      const spec = modeTopologyNote(wire).match(/alliance\.rs:(\d+)-(\d+)\/:(\d+)-(\d+)/)
      expect(spec, `${wire} 的说明没写成 alliance.rs:起-止/:起-止 两截`).toBeTruthy()
      expect(line(ALLIANCE_RS, +spec[1]), '第一截起点不再是契约注释').toContain('契约保持')
      expect(line(ALLIANCE_RS, +spec[2]), '第一截终点不再是末节点契约').toContain('末节点恒')
      expect(line(ALLIANCE_RS, +spec[3]), '第二截起点不再是 build_dag_for_task').toContain(FN_HEAD)
      expect(+spec[4], `${wire} 说明里的函数结束行与大括号配对不符`).toBe(fnEndLine(ALLIANCE_RS, FN_HEAD))
    }
  })

  it('plannerAt 与两根轴那句话的 types.rs 坐标都框住被引用的那段', () => {
    const dyn = DYNAMIC_BRANCH.plannerAt.match(/^planner\.rs:(\d+)-(\d+)$/)
    expect(dyn, 'plannerAt 得写成 planner.rs:起-止').toBeTruthy()
    expect(line(PLANNER_RS, +dyn[1])).toContain('fn generate_dynamic_plan')
    expect(+dyn[2], 'plannerAt 的结束行与大括号配对不符').toBe(fnEndLine(PLANNER_RS, 'fn generate_dynamic_plan'))

    const axis = MODE_AXIS_NOTE.match(/types\.rs:(\d+)-(\d+)/)
    expect(axis, '那句"两根轴"没给 AllianceMode 的坐标').toBeTruthy()
    expect(line(TYPES_RS, +axis[1])).toContain('pub enum AllianceMode')
    const span = TYPES_RS.split('\n').slice(+axis[1] - 1, +axis[2]).join('\n')
    for (const a of ARMS) {
      expect(span, `${a.variant} 不在所引用的枚举段里（清单漂了或段长漂了）`).toContain(`${a.variant},`)
    }
  })
})
