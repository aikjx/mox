// 联盟任务 DAG 的分层与依赖计数。刻意放在 model 层而不是视图里：页脚要说「多少条依赖参与了
// 分层」，这个数字必须由画分层用的同一份节点集算出来，否则报的是另一件事。
// 后端 edges 与 nodes[].dependencies 是 1:1 生成的（alliance.rs:1565-1569），但依赖可以指向
// 不存在的节点、也可以成环——DAG 侧没有 Kahn 检查（拓扑校验在 experts_orchestration.rs:443 起，
// 那是计划执行路径，不是这份出参），所以这两种形状在这里显式记账而不是静默丢掉。
import { NODE_STATUS } from '@/modules/expert-alliance/contract'

const depsOf = (node) => (Array.isArray(node?.dependencies) ? node.dependencies : []).filter((d) => d !== '' && d !== null && d !== undefined)

/**
 * @param {Array} nodes normDag().nodes（每项含 id / status / dependencies）
 * @param {Array} edges normDag().edges，仅用于核对后端边数与依赖清单是否仍然 1:1
 */
export function layoutDag(nodes = [], edges = []) {
  const list = (Array.isArray(nodes) ? nodes : []).filter((n) => n && n.id)
  const byId = new Map(list.map((n) => [n.id, n]))

  let declared = 0
  let drawn = 0
  let backEdges = 0
  const tally = {}
  for (const n of list) {
    const deps = depsOf(n)
    declared += deps.length
    drawn += deps.filter((d) => byId.has(d)).length
    const status = n.status || NODE_STATUS.PENDING
    tally[status] = (tally[status] ?? 0) + 1
  }

  const depth = new Map()
  const visiting = new Set()
  const resolve = (id) => {
    if (depth.has(id)) return depth.get(id)
    if (visiting.has(id)) {
      backEdges += 1
      return 0
    }
    visiting.add(id)
    let d = 0
    for (const dep of depsOf(byId.get(id))) {
      if (byId.has(dep)) d = Math.max(d, resolve(dep) + 1)
    }
    visiting.delete(id)
    depth.set(id, d)
    return d
  }
  for (const n of list) resolve(n.id)

  const buckets = new Map()
  for (const n of list) {
    const d = depth.get(n.id) ?? 0
    if (!buckets.has(d)) buckets.set(d, [])
    buckets.get(d).push(n)
  }
  const layers = [...buckets.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([d, group]) => ({ depth: d, nodes: group }))

  return {
    layers,
    tally,
    declared,
    drawn,
    dangling: declared - drawn,
    backEdges,
    edgeTotal: (Array.isArray(edges) ? edges : []).length,
    edgeDelta: (Array.isArray(edges) ? edges : []).length - declared
  }
}
