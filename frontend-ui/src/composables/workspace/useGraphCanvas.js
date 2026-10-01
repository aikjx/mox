/**
 * 工作台图谱区外壳：图数据、字段口径与布局全部来自联盟模块
 * （store/alliance-graph.store.js + model/layout.js + components/GraphCanvas.vue）。
 * 本文件只剩工作台特有的两件事：视口缩放/平移，以及把选中态交给 store。
 *
 * 为什么不允许再在这里自己拉数归一化（2026-09-27 实测的整条静默失真链路）：
 * - payload 给 node_type，旧代码读 n.type ⇒ 每个节点都掉成"节点"、类型统计恒 1、按类型上色失效；
 * - doc_count / expert_count / rank 后端从不产出 ⇒ 信息卡三行永远显示 0 / 0 / '-'；
 * - 坐标来自 Math.random() ⇒ 同一张图每次刷新都在换位置，无法比对也无法截图复验；
 * - 标签 .slice(0, 4) ⇒ architecture 画成 clou、data 与 database 撞成同一个 'data'，
 *   而后端写入侧就丢了编码的那两个专家（label='???????'）画成 '????'；
 * - 边端点按坐标反查 ⇒ 一布局就接错线；
 * - 空态分支写的是 nodes.value / edges.value，这两个 ref 在本作用域不存在 ⇒ 真跑到就是 ReferenceError；
 * - "图谱分析"是 setTimeout(1500) 后高亮前三个节点再宣称"已高亮核心节点"，与任何计算无关。
 * 模块侧那一份把这些逐条做掉了（确定性分层布局、真实 stats、按码点裁剪、id 解析边端点、分区错误隔离），
 * 完整分析面（中心性榜、社区、路径、最优团队）在 /alliance/graph，工作台不再养第二份。
 */
import { computed, ref } from 'vue'
import { GRAPH_NODE_TYPE } from '@/modules/expert-alliance/contract'
import { useAllianceGraphStore } from '@/modules/expert-alliance/store'

const SCALE_MIN = 0.5
const SCALE_MAX = 2.5
const SCALE_STEP = 1.25
const clamp = (v, min, max) => Math.max(min, Math.min(max, v))

export function useGraphCanvas() {
  const store = useAllianceGraphStore()

  const canvasRef = ref(null)
  const viewport = ref({ x: 0, y: 0, scale: 1 })

  const graphLoading = computed(() => store.loading.graph)
  const graphError = computed(() => store.error.graph)
  const selectedId = computed(() => store.selectedId)
  const selectedNode = computed(() => store.selectedNode)
  const layout = computed(() => store.layout)

  /** 统计条只报后端真给过的数：节点/边数取当前布局（与画布所见一致），拆分与密度取 stats */
  const graphStats = computed(() => {
    const stats = store.graph?.stats || {}
    const nodes = layout.value?.nodes || []
    const countByType = (type) => nodes.filter((n) => n.nodeType === type).length
    return {
      nodes: nodes.length,
      edges: (layout.value?.edges || []).length,
      expertNodes: Number(stats.expertCount) || countByType(GRAPH_NODE_TYPE.expert),
      domainNodes: Number(stats.domainCount) || countByType(GRAPH_NODE_TYPE.domain),
      density: Number(stats.density) || 0,
      version: Number(store.graph?.version) || 0,
      builtAt: store.graph?.builtAt || ''
    }
  })

  const viewportStyle = computed(() => {
    const v = viewport.value
    return {
      transform: `translate(${v.x}px, ${v.y}px) scale(${v.scale})`,
      transformOrigin: 'center center'
    }
  })

  async function loadGraphData() {
    await store.loadGraph()
    return !!store.graph
  }

  function selectNode(id) {
    store.selectNode(id)
  }

  function clearSelectedNode() {
    store.selectedId = ''
  }

  function zoomIn() { viewport.value.scale = clamp(viewport.value.scale * SCALE_STEP, SCALE_MIN, SCALE_MAX) }
  function zoomOut() { viewport.value.scale = clamp(viewport.value.scale / SCALE_STEP, SCALE_MIN, SCALE_MAX) }
  function fitView() { viewport.value = { x: 0, y: 0, scale: 1 } }

  let isDragging = false
  let dragStart = { x: 0, y: 0 }
  let viewportStart = { x: 0, y: 0 }

  function onCanvasMouseDown(e) {
    if (e.button !== 0 && e.button !== 1) return
    isDragging = true
    dragStart = { x: e.clientX, y: e.clientY }
    viewportStart = { ...viewport.value }
  }
  function onCanvasMouseMove(e) {
    if (!isDragging) return
    viewport.value.x = viewportStart.x + (e.clientX - dragStart.x)
    viewport.value.y = viewportStart.y + (e.clientY - dragStart.y)
  }
  function onCanvasMouseUp() { isDragging = false }
  function onCanvasWheel(e) {
    e.preventDefault()
    const next = viewport.value.scale * (e.deltaY > 0 ? 0.9 : 1.1)
    viewport.value.scale = clamp(next, SCALE_MIN, SCALE_MAX)
  }

  return {
    // 检视器组件按 AllianceGraphView 的既有约定收 store 本身（它要读 neighbors/collaborators/loading）
    graphStore: store,
    canvasRef, viewport, viewportStyle,
    graphLoading, graphError, graphStats, layout,
    selectedId, selectedNode,
    loadGraphData, selectNode, clearSelectedNode,
    zoomIn, zoomOut, fitView,
    onCanvasMouseDown, onCanvasMouseMove, onCanvasMouseUp, onCanvasWheel
  }
}
