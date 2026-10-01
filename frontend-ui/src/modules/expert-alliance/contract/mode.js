/**
 * 协作模式（AllianceMode）的**拓扑口径**单源。
 *
 * 权威源：`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs` 的
 * `build_dag_for_task`（:441-515）。这张表是网关**创建任务时**按模式查表写出的展示态拓扑，
 * 不是执行读数：首节点恒 Completed、中间某个节点恒 Running、其余恒 Pending（:439-440 的契约注释
 * 自己就是这么说的）。所以"DAG 页签"里看到的进度是模式模板，与任务真实进展无关。
 *
 * `contract.test.js` / `mode.test.js` 按行解析该函数并与本表逐节点比对：
 * 后端换掉某个节点名、多挂一个节点、把 Running 挪个位置，这里先红。
 */

import { MODE_DISPLAY, MODE_WIRE } from './enums.js'

/** 网关展示态拓扑：每模式的节点数、Running 个数（其余按 Pending 计）、按序节点名 */
export const MODE_TOPOLOGY = Object.freeze({
  [MODE_WIRE.SEQUENTIAL]: { nodes: 4, running: 1, names: ['需求分析', '方案设计', '方案评审', '融合输出'] },
  [MODE_WIRE.PARALLEL]: { nodes: 5, running: 1, names: ['需求分析', '架构设计', '数据建模', '方案评审', '融合输出'] },
  [MODE_WIRE.ITERATIVE]: { nodes: 6, running: 1, names: ['需求分析', '初稿生成', '人工评审', '修订迭代', '二次确认', '融合输出'] },
  [MODE_WIRE.HIERARCHICAL]: { nodes: 7, running: 1, names: ['需求分析', '协调规划', '子任务·架构', '子任务·数据', '子任务·实现', '专家汇总', '融合输出'] },
  [MODE_WIRE.VOTING]: { nodes: 6, running: 3, names: ['需求分析', '专家意见·甲', '专家意见·乙', '专家意见·丙', '投票裁决', '融合输出'] },
  [MODE_WIRE.DEBATE]: { nodes: 5, running: 2, names: ['需求分析', '正方陈述', '反方陈述', '仲裁裁决', '融合输出'] },
  [MODE_WIRE.DYNAMIC]: { nodes: 4, running: 1, names: ['需求分析', '动态路由', '深度执行', '融合输出'] }
})

/** 首末节点的契约位（:439-440 明文）：首节点恒完成、末节点恒融合 */
export const MODE_CONTRACT_NODES = Object.freeze({ first: '需求分析', last: '融合输出' })

/**
 * dynamic 模式的两条轴：网关展示态里**没有分支**（只有一个叫「动态路由」的节点，:509），
 * 真实选型在调度器规划期（`planner.rs::generate_dynamic_plan` :332-411），其产物
 * `PlanDynamicRoute`（types.rs:355-370）只在执行器内存里被消费
 * （`dag_engine.rs:593-600`，`plan.dynamic_routes` 为空时直接返回 None），
 * **不出现在任何响应里** —— 所以界面无法回答"这一次实际走了哪条分支"。
 */
export const DYNAMIC_BRANCH = Object.freeze({
  gatewayNode: '动态路由',
  gatewayExpert: 'expert-router',
  plannerAt: 'planner.rs:332-411',
  routeStruct: 'PlanDynamicRoute',
  onWire: false
})

/** 任务模式（AllianceMode 七变体）与协作工作台的"六模式"是两根轴，不可互相指代 */
export const MODE_AXIS_NOTE = Object.freeze(
  '这里的七个模式是任务的协作拓扑（`AllianceMode`，types.rs:133-148），'
  + '与智能协作工作台那六项不是一回事——后者是六个不同的请求端点，选它等于选接口'
)

export function modeWireOf(mode) {
  if (!mode) return ''
  if (MODE_TOPOLOGY[mode]) return mode
  const entry = Object.entries(MODE_DISPLAY).find(([, v]) => v === mode)
  return entry ? MODE_WIRE[entry[0]] || '' : ''
}

/** 后端两条线都收：任务出参给的是 mode_display（`expert_alliance` 这类），请求发的是 mode_serde */
export function modeTopologyOf(mode) {
  return MODE_TOPOLOGY[modeWireOf(mode)] || null
}

/** 该模式在建任务时会被写成 Pending 的节点数（首节点 Completed、其余按模板分 Running/Pending） */
export function modePendingCount(mode) {
  const t = modeTopologyOf(mode)
  return t ? t.nodes - 1 - t.running : 0
}

/**
 * DAG 页签顶部的模式说明。措辞硬约束：
 * 必须点出"建任务时按模式查表"（否则用户把图读成进度），
 * dynamic 还要点出"分支不在响应里"（否则用户追问走了哪条分支时界面在装）。
 */
export function modeTopologyNote(mode) {
  const wire = modeWireOf(mode)
  const t = MODE_TOPOLOGY[wire]
  if (!t) {
    return `该任务的协作模式不在拓扑表里（模式值：${mode || '未知'}）——后端 `
      + 'build_dag_for_task 只有七个分支，多出来的值意味着这张图和模式对不上，'
      + '节点数与状态分布都无从解释，按未知处理'
  }
  const display = MODE_DISPLAY[Object.keys(MODE_WIRE).find((k) => MODE_WIRE[k] === wire)]
  const names = [wire, display].filter((x, i, a) => x && a.indexOf(x) === i).join(' / ')
  const base = `本图是网关建任务时按模式查表写出的展示态拓扑（${names}）：`
    + `${t.nodes} 个节点里 ${t.running} 个恒记 Running、${modePendingCount(wire)} 个恒记 Pending，`
    + `首节点「${MODE_CONTRACT_NODES.first}」恒 Completed、末节点「${MODE_CONTRACT_NODES.last}」恒为融合出口`
    + '（alliance.rs:439-440/:441-515）——这些状态来自模板，不是执行读数，进度请看节点状态列而不必看这张图'
  if (wire !== MODE_WIRE.DYNAMIC) return base
  return `${base}。另外 ${DYNAMIC_BRANCH.gatewayNode} 只是**一个节点**（expert-router，:509），`
    + `图上没有分支：真实的动态选型在调度器规划期完成（${DYNAMIC_BRANCH.plannerAt}），`
    + `其 ${DYNAMIC_BRANCH.routeStruct} 只在执行器内存里被消费、不在任何响应里，`
    + '所以界面答不了"这一次实际走了哪条分支"——能说的只有"这条模式允许分支"'
}
