// 协作图谱布局：纯函数、完全确定性（同一份 nodes/edges 必得同一组坐标）。
// 刻意不用 force-directed 弹簧布局或随机初始化：那类布局每次刷新都在动，既无法快照测试，
// 也让"专家—能力域"这种天然分层结构失去可读性。
//
// 布局规则对齐图的真实构成（见 contract/graph.js 顶部说明）：
// 能力域节点等角分布在中环，专家节点围在自己主域的外侧弧上，
// 没有任何 has_domain 边的专家落在外环，不与其他簇重叠。
import { GRAPH_EDGE_TYPE, GRAPH_NODE_TYPE } from '@/modules/expert-alliance/contract'

export const GRAPH_VIEWPORT = Object.freeze({ width: 720, height: 520 })

const round = (v) => Math.round(v * 100) / 100

/** 无向度数表：协作边与归属边一并计入，与后端 degree_centrality 的口径一致 */
export function graphDegrees(edges = []) {
  const degree = {}
  for (const e of edges) {
    if (!e) continue
    if (e.source) degree[e.source] = (degree[e.source] || 0) + 1
    if (e.target) degree[e.target] = (degree[e.target] || 0) + 1
  }
  return degree
}

/** 节点半径：度数越大越显眼，封顶避免个别枢纽节点吃掉画布 */
export function nodeRadius(degree = 0, cap = 18) {
  return round(9 + Math.min(cap, (Number(degree) || 0) * 1.6))
}

function firstDomainOf(nodeId, edges) {
  for (const e of edges) {
    if (!e || e.edgeType !== GRAPH_EDGE_TYPE.has_domain) continue
    if (e.source === nodeId) return e.target
    if (e.target === nodeId) return e.source
  }
  return ''
}

/**
 * @param {Array<{id:string,nodeType:string}>} nodes normGraph().nodes
 * @param {Array<{source:string,target:string,edgeType:string}>} edges normGraph().edges
 * @returns {{positions: Record<string,{x:number,y:number}>, nodes: Array, ringRadius: number[]}}
 */
export function graphLayout(nodes = [], edges = [], viewport = GRAPH_VIEWPORT) {
  const { width, height } = viewport
  const cx = width / 2
  const cy = height / 2
  const domains = nodes.filter((n) => n?.nodeType === GRAPH_NODE_TYPE.domain)
  const experts = nodes.filter((n) => n && n.nodeType !== GRAPH_NODE_TYPE.domain)
  const ringRadius = [round(Math.min(width, height) * 0.22), round(Math.min(width, height) * 0.44)]

  const positions = {}
  const domainAngle = {}
  domains.forEach((d, i) => {
    const angle = (Math.PI * 2 * i) / Math.max(domains.length, 1) - Math.PI / 2
    domainAngle[d.id] = angle
    positions[d.id] = {
      x: round(cx + Math.cos(angle) * ringRadius[0]),
      y: round(cy + Math.sin(angle) * ringRadius[0])
    }
  })

  // 按"第一条归属边"给专家归簇：与边数组顺序有关，因此稳定可复算
  const clusters = new Map()
  const orphans = []
  for (const n of experts) {
    const home = firstDomainOf(n.id, edges)
    if (home && positions[home]) {
      if (!clusters.has(home)) clusters.set(home, [])
      clusters.get(home).push(n.id)
    } else {
      orphans.push(n.id)
    }
  }

  for (const [domainId, members] of clusters) {
    const base = domainAngle[domainId]
    const home = positions[domainId]
    // 弧长随簇规模张开，成员多时绕得更宽，避免叠成一团
    const spread = Math.min(Math.PI * 1.1, (Math.PI / 5) * Math.max(members.length, 1))
    const orbit = ringRadius[0] * (members.length > 4 ? 0.62 : 0.45)
    members.forEach((id, i) => {
      const t = members.length === 1 ? 0 : i / (members.length - 1) - 0.5
      const angle = base + t * spread
      positions[id] = {
        x: round(home.x + Math.cos(angle) * orbit),
        y: round(home.y + Math.sin(angle) * orbit)
      }
    })
  }

  orphans.forEach((id, i) => {
    const angle = (Math.PI * 2 * i) / Math.max(orphans.length, 1) - Math.PI / 2
    positions[id] = {
      x: round(cx + Math.cos(angle) * ringRadius[1]),
      y: round(cy + Math.sin(angle) * ringRadius[1])
    }
  })

  const degree = graphDegrees(edges)
  return {
    positions,
    degree,
    edges,
    // 放置对每个有 id 的节点都是全覆盖的（域在中环，专家在簇上或外环），故只挡掉无 id 条目：
    // 它们进画布就是 cx/cy 为 undefined 的 SVG 圆，浏览器什么都不画也不报错。
    nodes: nodes.filter((n) => n?.id).map((n) => ({
      ...n,
      ...positions[n.id],
      degree: degree[n.id] || 0,
      radius: nodeRadius(degree[n.id] || 0)
    })),
    center: { x: round(cx), y: round(cy) },
    ringRadius
  }
}

/** 路径序列 → 可读链路文本，不可达时明确说"不连通"而不是空串 */
export function pathChainText(path = [], found = true) {
  const labels = (Array.isArray(path) ? path : []).map((p) => p.label || p.nodeId).filter(Boolean)
  if (!found || !labels.length) return '两节点间不存在连通路径'
  return labels.join(' → ')
}
