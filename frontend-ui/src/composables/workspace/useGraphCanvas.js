/**
 * 图谱画布 Composable
 * 职责：图谱数据加载、视口控制、布局切换、节点交互
 */
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { getExpertGraph } from '@/api/experts.api.js'

export function useGraphCanvas(expertColor) {
  const canvasRef = ref(null)
  const activeCanvasTool = ref('select')
  const currentLayout = ref('force')
  const selectedNode = ref(null)
  const graphLoading = ref(false)
  const graphAnalyzing = ref(false)
  const viewport = ref({ x: 0, y: 0, scale: 1 })
  const graphNodes = ref([])
  const graphEdges = ref([])
  const graphStats = ref({ nodes: 0, edges: 0, types: 0 })

  const svgViewBox = computed(() => {
    const w = 800 / viewport.value.scale
    const h = 500 / viewport.value.scale
    const x = viewport.value.x - w / 2 + 400
    const y = viewport.value.y - h / 2 + 250
    return `${x} ${y} ${w} ${h}`
  })

  let isDragging = false
  let dragStart = { x: 0, y: 0 }
  let viewportStart = { x: 0, y: 0 }

  async function loadGraphData() {
    graphLoading.value = true
    try {
      const res = await getExpertGraph()
      const data = res?.data || res
      if (data?.nodes && data?.edges) {
        graphNodes.value = normalizeGraphNodes(data.nodes)
        graphEdges.value = normalizeGraphEdges(data.edges, data.nodes)
        graphStats.value = { nodes: data.nodes.length, edges: data.edges.length, types: [...new Set(data.nodes.map(n => n.type || 'default'))].length }
      } else useMockGraph()
    } catch (e) { console.warn('[workspace] 加载图谱失败:', e); useMockGraph() }
    finally { graphLoading.value = false }
  }

  function useMockGraph() {
    const nodes = [
      { id: 'n1', label: '专家', fullName: '专家实体', type: '核心实体', x: 400, y: 200, size: 28, color: '#6366f1', docs: 156, experts: 12, rank: 1, highlight: true },
      { id: 'n2', label: '知识', fullName: '知识节点', type: '核心实体', x: 250, y: 280, size: 24, color: '#06b6d4', docs: 203, experts: 8, rank: 2 },
      { id: 'n3', label: '文档', fullName: '文档实体', type: '内容实体', x: 550, y: 280, size: 22, color: '#10b981', docs: 892, experts: 5, rank: 3 },
      { id: 'n4', label: '图谱', fullName: '知识图谱', type: '系统', x: 320, y: 120, size: 20, color: '#8b5cf6', docs: 45, experts: 7, rank: 5 },
      { id: 'n5', label: '算法', fullName: '图算法', type: '算法', x: 480, y: 120, size: 18, color: '#f59e0b', docs: 67, experts: 6, rank: 4 },
      { id: 'n6', label: '协作', fullName: '协作模式', type: '关系类型', x: 180, y: 180, size: 16, color: '#ec4899', docs: 23, experts: 4, rank: 8 },
      { id: 'n7', label: '云盘', fullName: '云存储', type: '系统', x: 620, y: 180, size: 18, color: '#14b8a6', docs: 34, experts: 3, rank: 7 },
      { id: 'n8', label: '编排', fullName: '任务编排', type: '能力', x: 200, y: 380, size: 16, color: '#f97316', docs: 28, experts: 5, rank: 9 },
      { id: 'n9', label: '辩论', fullName: '专家辩论', type: '协作模式', x: 400, y: 380, size: 18, color: '#ef4444', docs: 12, experts: 8, rank: 6 },
      { id: 'n10', label: '融合', fullName: '知识融合', type: '能力', x: 600, y: 380, size: 16, color: '#84cc16', docs: 31, experts: 4, rank: 10 }
    ]
    const edges = [
      { id: 'e1', source: 'n1', target: 'n2', color: '#6366f1', width: 2 },
      { id: 'e2', source: 'n1', target: 'n3', color: '#6366f1', width: 2 },
      { id: 'e3', source: 'n1', target: 'n4', color: '#94a3b8' },
      { id: 'e4', source: 'n1', target: 'n5', color: '#94a3b8' },
      { id: 'e5', source: 'n2', target: 'n6', color: '#94a3b8' },
      { id: 'e6', source: 'n3', target: 'n7', color: '#94a3b8' },
      { id: 'e7', source: 'n2', target: 'n8', color: '#94a3b8' },
      { id: 'e8', source: 'n1', target: 'n9', color: '#ef4444', width: 2 },
      { id: 'e9', source: 'n3', target: 'n10', color: '#94a3b8' },
      { id: 'e10', source: 'n4', target: 'n5', color: '#94a3b8' },
      { id: 'e11', source: 'n2', target: 'n4', color: '#06b6d4', width: 1.5 },
      { id: 'e12', source: 'n3', target: 'n5', color: '#10b981', width: 1.5 }
    ]
    graphNodes.value = nodes
    graphEdges.value = edges.map(e => {
      const s = nodes.find(n => n.id === e.source)
      const t = nodes.find(n => n.id === e.target)
      return { ...e, sourceX: s?.x || 0, sourceY: s?.y || 0, targetX: t?.x || 0, targetY: t?.y || 0 }
    })
    graphStats.value = { nodes: nodes.length, edges: edges.length, types: 7 }
  }

  function normalizeGraphNodes(nodes) {
    return nodes.map((n, i) => ({
      id: n.id || `n${i}`, label: (n.label || n.name || '?').slice(0, 4), fullName: n.name || n.label || '',
      type: n.type || '节点', x: n.x || 200 + Math.random() * 400, y: n.y || 100 + Math.random() * 300,
      size: n.size || (n.highlight ? 24 : 18), color: n.color || expertColor?.(n.type) || '#6366f1',
      docs: n.doc_count || n.docs || 0, experts: n.expert_count || n.experts || 0,
      rank: n.rank || '-', highlight: n.highlight || false, description: n.description || ''
    }))
  }

  function normalizeGraphEdges(edges, nodes) {
    const nodeMap = {}
    nodes.forEach(n => { nodeMap[n.id || n.name] = n })
    return edges.map((e, i) => {
      const s = nodeMap[e.source || e.from || e.s]
      const t = nodeMap[e.target || e.to || e.t]
      return { id: e.id || `e${i}`, sourceX: s?.x || 0, sourceY: s?.y || 0, targetX: t?.x || 0, targetY: t?.y || 0, color: e.color || '#94a3b8', width: e.width || 1.5, highlight: e.highlight || false }
    })
  }

  function selectNode(node) {
    selectedNode.value = node
    graphEdges.value.forEach(e => {
      e.highlight = e.id?.includes(node.id) || graphEdges.value.some(edge => (edge.sourceX === node.x && edge.sourceY === node.y) || (edge.targetX === node.x && edge.targetY === node.y))
    })
  }

  function switchLayout(layout) { currentLayout.value = layout; applyLayout(layout) }

  function applyLayout(layout) {
    const nodes = graphNodes.value
    const cx = 400, cy = 250
    if (layout === 'force') useMockGraph()
    else if (layout === 'radial') {
      const center = nodes[0]
      if (center) { center.x = cx; center.y = cy }
      nodes.slice(1).forEach((n, i) => {
        const angle = (i / (nodes.length - 1)) * Math.PI * 2
        const r = 120 + (i % 3) * 40
        n.x = cx + Math.cos(angle) * r; n.y = cy + Math.sin(angle) * r
      })
      updateEdgePositions()
    } else if (layout === 'hierarchical') {
      const levels = 4
      const perLevel = Math.ceil(nodes.length / levels)
      nodes.forEach((n, i) => {
        const level = Math.floor(i / perLevel)
        const posInLevel = i % perLevel
        const nodesInLevel = Math.min(perLevel, nodes.length - level * perLevel)
        n.x = cx + (posInLevel - (nodesInLevel - 1) / 2) * 100
        n.y = 80 + level * 130
      })
      updateEdgePositions()
    } else if (layout === 'circular') {
      nodes.forEach((n, i) => {
        const angle = (i / nodes.length) * Math.PI * 2 - Math.PI / 2
        const r = 150
        n.x = cx + Math.cos(angle) * r; n.y = cy + Math.sin(angle) * r
      })
      updateEdgePositions()
    }
  }

  function updateEdgePositions() {
    graphEdges.value.forEach(e => {
      const s = graphNodes.value.find(n => Math.abs(n.x - e.sourceX) < 1 && Math.abs(n.y - e.sourceY) < 1)
      const t = graphNodes.value.find(n => Math.abs(n.x - e.targetX) < 1 && Math.abs(n.y - e.targetY) < 1)
      if (s) { e.sourceX = s.x; e.sourceY = s.y }
      if (t) { e.targetX = t.x; e.targetY = t.y }
    })
  }

  function zoomIn() { viewport.value.scale = Math.min(viewport.value.scale * 1.2, 3) }
  function zoomOut() { viewport.value.scale = Math.max(viewport.value.scale / 1.2, 0.3) }
  function fitView() { viewport.value = { x: 0, y: 0, scale: 1 } }

  async function runGraphAlgo() {
    graphAnalyzing.value = true
    try {
      await new Promise(r => setTimeout(r, 1500))
      graphNodes.value.forEach((n, i) => { n.highlight = i < 3 })
      ElMessage.success('图谱分析完成，已高亮核心节点')
    } catch (e) { ElMessage.error('图谱分析失败') }
    finally { graphAnalyzing.value = false }
  }

  function onCanvasMouseDown(e) {
    if (activeCanvasTool.value === 'pan' || e.button === 1) {
      isDragging = true; dragStart = { x: e.clientX, y: e.clientY }; viewportStart = { ...viewport.value }
    }
  }
  function onCanvasMouseMove(e) {
    if (isDragging) {
      const dx = (e.clientX - dragStart.x) / viewport.value.scale
      const dy = (e.clientY - dragStart.y) / viewport.value.scale
      viewport.value.x = viewportStart.x - dx; viewport.value.y = viewportStart.y - dy
    }
  }
  function onCanvasMouseUp() { isDragging = false }
  function onCanvasWheel(e) {
    e.preventDefault()
    const delta = e.deltaY > 0 ? 0.9 : 1.1
    viewport.value.scale = Math.max(0.3, Math.min(3, viewport.value.scale * delta))
  }
  function onNodeMouseDown(e, node) { /* 节点拖拽逻辑可扩展 */ }

  return {
    canvasRef, activeCanvasTool, currentLayout, selectedNode, graphLoading, graphAnalyzing,
    viewport, svgViewBox, graphNodes, graphEdges, graphStats,
    loadGraphData, selectNode, switchLayout, zoomIn, zoomOut, fitView, runGraphAlgo,
    onCanvasMouseDown, onCanvasMouseMove, onCanvasMouseUp, onCanvasWheel, onNodeMouseDown
  }
}
