<template>
  <div class="graph-page">
    <!-- 第一层：顶部操作栏（固定） -->
    <header class="graph-topbar">
      <div class="gt-left">
        <div class="gt-title-wrap">
          <div class="gt-icon">🕸️</div>
          <div>
            <h1 class="gt-title">知识图谱</h1>
            <div class="gt-sub">
              <span class="gt-badge" v-if="stats">
                <span class="gt-dot ok"></span>
                {{ stats.node_count || 0 }} 节点 · {{ stats.edge_count || 0 }} 关系
              </span>
              <span class="gt-stage" v-if="loadStage !== 'physics'">{{ stageLabel }}</span>
              <span class="gt-stage ok" v-else>● 已就绪</span>
            </div>
          </div>
        </div>
      </div>
      <div class="gt-right">
        <div class="gt-search">
          <el-icon><Search /></el-icon>
          <input
            v-model="searchQ"
            class="gt-search-input"
            placeholder="搜索节点…"
            @keyup.enter="doSearch"
            @keyup.esc="clearSearch"
          />
          <kbd v-if="!searchQ" class="gt-kbd">/</kbd>
        </div>
        <el-button type="primary" class="gt-primary-btn" @click="goAIAnalysis">
          <el-icon><Promotion /></el-icon>
          <span>AI 深度分析</span>
        </el-button>
        <el-button class="gt-icon-btn" @click="reload" title="刷新">
          <el-icon><Refresh /></el-icon>
        </el-button>
        <el-button class="gt-icon-btn" @click="toggleSidePanel" title="工具面板">
          <el-icon><Grid /></el-icon>
        </el-button>
      </div>
    </header>

    <!-- 第二层：主工作区（左侧工具面板 + 中央画布） -->
    <div class="graph-main">
      <!-- 左侧工具面板（分层展开） -->
      <aside class="side-panel" :class="{ collapsed: sidePanelCollapsed }">
        <!-- 布局模式 -->
        <div class="sp-section">
          <div class="sp-section-header" @click="toggleSection('layout')">
            <span class="sp-section-icon">🎯</span>
            <span class="sp-section-title">布局模式</span>
            <el-icon class="sp-section-arrow">
              <component :is="expandedSections.has('layout') ? 'ArrowUp' : 'ArrowDown'" />
            </el-icon>
          </div>
          <transition name="sp-expand">
            <div v-show="expandedSections.has('layout')" class="sp-section-body">
              <div class="layout-grid">
                <div
                  v-for="l in layoutOptions"
                  :key="l.key"
                  class="layout-option"
                  :class="{ active: layoutMode === l.key }"
                  @click="applyLayout(l.key)"
                >
                  <div class="lo-icon">{{ l.icon }}</div>
                  <div class="lo-name">{{ l.name }}</div>
                </div>
              </div>
            </div>
          </transition>
        </div>

        <!-- 样式调节 -->
        <div class="sp-section">
          <div class="sp-section-header" @click="toggleSection('style')">
            <span class="sp-section-icon">🎨</span>
            <span class="sp-section-title">样式调节</span>
            <el-icon class="sp-section-arrow">
              <component :is="expandedSections.has('style') ? 'ArrowUp' : 'ArrowDown'" />
            </el-icon>
          </div>
          <transition name="sp-expand">
            <div v-show="expandedSections.has('style')" class="sp-section-body">
              <div class="style-row">
                <label>节点大小</label>
                <el-slider v-model="layoutConfig.nodeSize" :min="5" :max="40" :step="1" @change="updateLayout" />
                <span class="style-val">{{ layoutConfig.nodeSize }}px</span>
              </div>
              <div class="style-row">
                <label>连线长度</label>
                <el-slider v-model="layoutConfig.linkDistance" :min="20" :max="200" :step="5" @change="updateLayout" />
                <span class="style-val">{{ layoutConfig.linkDistance }}px</span>
              </div>
              <div class="style-row">
                <label>斥力强度</label>
                <el-slider v-model="layoutConfig.repulsion" :min="50" :max="500" :step="10" @change="updateLayout" />
                <span class="style-val">{{ layoutConfig.repulsion }}</span>
              </div>
              <div class="style-row">
                <label>引力强度</label>
                <el-slider v-model="layoutConfig.gravity" :min="0" :max="1" :step="0.01" @change="updateLayout" />
                <span class="style-val">{{ layoutConfig.gravity.toFixed(2) }}</span>
              </div>
              <div class="style-row">
                <label>显示标签</label>
                <el-switch v-model="layoutConfig.showLabels" @change="updateLayout" size="small" />
              </div>
              <div class="style-row" v-if="layoutConfig.showLabels">
                <label>标签大小</label>
                <el-slider v-model="layoutConfig.labelSize" :min="8" :max="20" :step="1" @change="updateLayout" />
                <span class="style-val">{{ layoutConfig.labelSize }}px</span>
              </div>
              <div class="style-row">
                <label>连线透明度</label>
                <el-slider v-model="layoutConfig.linkOpacity" :min="0.1" :max="1" :step="0.05" @change="updateLayout" />
                <span class="style-val">{{ layoutConfig.linkOpacity.toFixed(2) }}</span>
              </div>
              <div class="style-row">
                <label>连线粗细</label>
                <el-slider v-model="layoutConfig.linkWidth" :min="0.3" :max="3" :step="0.1" @change="updateLayout" />
                <span class="style-val">{{ layoutConfig.linkWidth.toFixed(1) }}px</span>
              </div>
              <div class="sp-actions">
                <el-button size="small" @click="resetLayoutConfig">重置默认</el-button>
                <el-button size="small" type="primary" @click="screenshotGraph">导出截图</el-button>
              </div>
            </div>
          </transition>
        </div>

        <!-- 快捷分析 -->
        <div class="sp-section">
          <div class="sp-section-header" @click="toggleSection('analysis')">
            <span class="sp-section-icon">⚡</span>
            <span class="sp-section-title">快捷分析</span>
            <el-icon class="sp-section-arrow">
              <component :is="expandedSections.has('analysis') ? 'ArrowUp' : 'ArrowDown'" />
            </el-icon>
          </div>
          <transition name="sp-expand">
            <div v-show="expandedSections.has('analysis')" class="sp-section-body">
              <div class="qa-list">
                <div class="qa-item" @click="runQuickAnalysis('centrality')">
                  <div class="qa-icon" style="background:var(--accent-dim);color:var(--accent-light)">🎯</div>
                  <div class="qa-info">
                    <div class="qa-title">中心性分析</div>
                    <div class="qa-desc">识别关键节点</div>
                  </div>
                </div>
                <div class="qa-item" @click="runQuickAnalysis('community')">
                  <div class="qa-icon" style="background:#ecfeff;color:#06b6d4">🧩</div>
                  <div class="qa-info">
                    <div class="qa-title">社区发现</div>
                    <div class="qa-desc">自动聚类分组</div>
                  </div>
                </div>
                <div class="qa-item" @click="runQuickAnalysis('path')">
                  <div class="qa-icon" style="background:#ecfdf5;color:#10b981">🛤️</div>
                  <div class="qa-info">
                    <div class="qa-title">最短路径</div>
                    <div class="qa-desc">两节点关联分析</div>
                  </div>
                </div>
                <div class="qa-item" @click="runQuickAnalysis('pagerank')">
                  <div class="qa-icon" style="background:#fef3c7;color:#d97706">📊</div>
                  <div class="qa-info">
                    <div class="qa-title">PageRank</div>
                    <div class="qa-desc">节点重要性排名</div>
                  </div>
                </div>
                <div class="qa-item" @click="runQuickAnalysis('activation')">
                  <div class="qa-icon" style="background:#fce7f3;color:#ec4899">🔥</div>
                  <div class="qa-info">
                    <div class="qa-title">激活传播</div>
                    <div class="qa-desc">影响扩散模拟</div>
                  </div>
                </div>
              </div>
            </div>
          </transition>
        </div>

        <!-- 节点类型图例 -->
        <div class="sp-section">
          <div class="sp-section-header" @click="toggleSection('legend')">
            <span class="sp-section-icon">🏷️</span>
            <span class="sp-section-title">节点类型</span>
            <el-icon class="sp-section-arrow">
              <component :is="expandedSections.has('legend') ? 'ArrowUp' : 'ArrowDown'" />
            </el-icon>
          </div>
          <transition name="sp-expand">
            <div v-show="expandedSections.has('legend')" class="sp-section-body">
              <div class="legend-list">
                <div class="legend-item" v-for="(color, type) in nodeTypeLegend" :key="type">
                  <span class="legend-dot" :style="{ background: color }"></span>
                  <span class="legend-label">{{ typeLabels[type] || type }}</span>
                </div>
              </div>
            </div>
          </transition>
        </div>
      </aside>

      <!-- 中央画布区 -->
      <main class="graph-canvas-wrap">
        <svg v-if="showSkeleton" class="skeleton-svg" viewBox="0 0 800 520" preserveAspectRatio="xMidYMid meet">
          <defs>
            <radialGradient id="gvGlow" cx="50%" cy="50%" r="50%">
              <stop offset="0%" stop-color="#6366f1" stop-opacity="0.25" />
              <stop offset="100%" stop-color="#0b1020" stop-opacity="0" />
            </radialGradient>
          </defs>
          <rect width="800" height="520" fill="#0b1020" rx="12" />
          <circle cx="400" cy="260" r="220" fill="url(#gvGlow)" />
          <g stroke="rgba(148,163,184,0.18)" stroke-width="1" fill="none">
            <line v-for="(_, i) in 20" :key="'sk-e'+i"
              :x1="400 + 140*Math.cos(i*Math.PI/10)" :y1="260 + 100*Math.sin(i*Math.PI/10)"
              :x2="400 + 240*Math.cos((i+3)*Math.PI/10)" :y2="260 + 180*Math.sin((i+3)*Math.PI/10)" />
          </g>
          <g v-for="(n, i) in skelNodes" :key="'sk-n'+i">
            <circle :cx="400 + 180*Math.cos(i*Math.PI/6 + 0.2)" :cy="260 + 120*Math.sin(i*Math.PI/6 + 0.2)"
              :r="n.r" :fill="n.c" opacity="0.92" />
          </g>
          <text x="400" y="500" text-anchor="middle" fill="#94a3b8" font-size="13" letter-spacing="2">
            {{ stageLabel }} · {{ stageProgress }}%
          </text>
        </svg>
        <div ref="graphEl" class="graph-canvas" :class="{ covered: showSkeleton }"></div>

        <!-- 右下角统计条 -->
        <div class="graph-statbar" v-if="stats && !showSkeleton">
          <div class="gs-item">
            <div class="gs-value">{{ stats.node_count || 0 }}</div>
            <div class="gs-label">节点</div>
          </div>
          <div class="gs-divider"></div>
          <div class="gs-item">
            <div class="gs-value">{{ stats.edge_count || 0 }}</div>
            <div class="gs-label">关系</div>
          </div>
          <div class="gs-divider"></div>
          <div class="gs-item">
            <div class="gs-value">{{ stats.type_count || 0 }}</div>
            <div class="gs-label">类型</div>
          </div>
          <div class="gs-divider"></div>
          <div class="gs-item">
            <div class="gs-value">{{ layoutModeName }}</div>
            <div class="gs-label">布局</div>
          </div>
        </div>
      </main>
    </div>

    <!-- 分析结果抽屉（底部上滑） -->
    <transition name="drawer-up">
      <div v-if="analysisResult" class="analysis-drawer">
        <div class="ad-header">
          <div class="ad-title">
            <el-icon><DataAnalysis /></el-icon>
            <span>{{ analysisResult.title }}</span>
          </div>
          <el-button text @click="analysisResult = null">
            <el-icon><Close /></el-icon>
          </el-button>
        </div>
        <div class="ad-body">
          <pre class="ad-content">{{ JSON.stringify(analysisResult.data, null, 2) }}</pre>
        </div>
      </div>
    </transition>
  </div>
</template>
<script setup>
import { ref, onMounted, onBeforeUnmount, computed, nextTick, shallowRef, markRaw, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
// [P1-1 渐进加载 · 先画布后力学] 静态仅依赖轻量类型/API；3D 重库 ForceGraph3D 改为动态 import 后按需拆分异步 chunk（≈1.2MB 单独下载，不阻塞首帧）
import { NODE_TYPE_COLORS } from '@/types'
import { useProject } from '@/composables/projectContext.js'
import {
  getGraph,
  getGraphStats,
  getShortestPath,
  getNeighbors,
  recommendNodes,
  graphSearch,
  getCentrality,
  getCommunities,
  getPagerank,
  propagateActivation
} from '@/api'
import {
  Share, Search, Refresh, ArrowRight, Promotion,
  Grid, Setting, Close, DataAnalysis, ArrowUp, ArrowDown
} from '@element-plus/icons-vue'

const graphEl = ref(null)
const stats = ref(null)
const nodeIds = ref([])
// 用 shallowRef/markRaw 防止 Vue 递归代理 three.js/FG 对象（大对象深代理 = 严重卡顿 + 内存翻倍）
const router = useRouter()

/* ===== 布局系统 ===== */
const layoutMode = ref('force')
const showLayoutPanel = ref(false)
const currentGraphData = ref({ nodes: [], edges: [] })

/* ===== 侧边面板（分层展开） ===== */
const sidePanelCollapsed = ref(false)
// 默认展开布局和样式，折叠分析和图例（用户按需展开）
const expandedSections = ref(new Set(['layout', 'style']))

const layoutOptions = [
  { key: 'force', name: '力导向', icon: '🧲' },
  { key: 'circular', name: '同心环', icon: '🔵' },
  { key: 'radial', name: '径向', icon: '🌟' },
  { key: 'hierarchical', name: '分层', icon: '📊' },
  { key: 'grid', name: '网格', icon: '🔲' },
  { key: 'fruchterman', name: 'FR稳定', icon: '⚡' }
]

const typeLabels = {
  core: '核心算子',
  activation: '激活函数',
  math: '数学算子',
  signal: '信号算子',
  data: '数据算子',
  ai: 'AI 算子',
  graph: '图算子',
  optimizer: '优化器',
  loss: '损失函数',
  regularization: '正则化',
  normalization: '归一化',
  custom: '自定义'
}

const nodeTypeLegend = computed(() => {
  const colors = NODE_TYPE_COLORS
  // 只显示有数据的类型
  const types = new Set()
  currentGraphData.value.nodes.forEach(n => {
    if (n.node_type) types.add(n.node_type)
  })
  const result = {}
  types.forEach(t => {
    if (colors[t]) result[t] = colors[t]
  })
  // 如果没有数据，显示所有类型
  if (Object.keys(result).length === 0) {
    return colors
  }
  return result
})

const layoutModeName = computed(() => {
  const opt = layoutOptions.find(l => l.key === layoutMode.value)
  return opt ? opt.name : layoutMode.value
})

function toggleSidePanel() {
  sidePanelCollapsed.value = !sidePanelCollapsed.value
}

function toggleSection(sectionKey) {
  const next = new Set(expandedSections.value)
  if (next.has(sectionKey)) {
    next.delete(sectionKey)
  } else {
    next.add(sectionKey)
  }
  expandedSections.value = next
}

/* ===== 分析结果 ===== */
const analysisResult = ref(null)

const DEFAULT_LAYOUT_CONFIG = {
  nodeSize: 16,
  linkDistance: 80,
  repulsion: 200,
  gravity: 0.1,
  showLabels: true,
  labelSize: 11,
  linkOpacity: 0.35,
  linkWidth: 0.8
}

const layoutConfig = reactive({ ...DEFAULT_LAYOUT_CONFIG })

function resetLayoutConfig() {
  Object.assign(layoutConfig, DEFAULT_LAYOUT_CONFIG)
  updateLayout()
}

// AI分析图谱：跳转到AI助手，带上图谱上下文
function goAIAnalysis() {
  router.push({ path: '/ai', query: { source: 'graph', action: 'analyze' } })
}

// 快捷分析入口
function runQuickAnalysis(type) {
  const actions = {
    centrality: '请对当前知识图谱进行中心性分析，包括度中心性、介数中心性、紧密中心性，并识别Top 10关键节点。',
    community: '请对当前知识图谱进行社区发现分析，识别主要社群、各社区的核心节点和特征，并给出可视化建议。',
    path: '请帮我分析图谱中两个节点之间的最短路径和关联关系。（请先选择起始节点和目标节点）'
  }
  router.push({ path: '/ai', query: { source: 'graph', action: type, prompt: encodeURIComponent(actions[type]) } })
}

let fg = null
let fgModule = null

// ---------- [P1-1] 渐进加载 Stage 状态机 ----------
// skeleton → fetch → module → render → physics  5 段式，每段 20% 进度
const LOAD_WEIGHT = Object.freeze({ skeleton: 0, fetch: 20, module: 45, render: 80, physics: 100 })
const loadStage = ref(/** @type {'skeleton'|'fetch'|'module'|'render'|'physics'} */ ('skeleton'))
function setStage(s) { loadStage.value = s }
const stageProgress = computed(() => LOAD_WEIGHT[loadStage.value] ?? 0)
const stageLabel = computed(() => ({
  skeleton: '① 初始化布局',
  fetch: '② 加载图谱数据',
  module: '③ 加载 3D 渲染引擎',
  render: '④ 渲染首帧',
  physics: '⑤ 力学收敛',
}[loadStage.value] || ''))
const showSkeleton = computed(() => ['skeleton', 'fetch', 'module'].includes(loadStage.value))

// 骨架 12 节点（按 NODE_TYPE_COLORS 调色，纯视觉占位）
const _ntColors = Object.values(NODE_TYPE_COLORS)
const skelNodes = Array.from({ length: 12 }, (_, i) => ({
  r: 6 + ((i * 7) % 10),
  c: _ntColors[i % _ntColors.length] || '#60a5fa',
}))

// 缓存 Promise（模块单例，避免重复 import()）
let _fgLoaderPromise = null
function loadForceGraph3DModule() {
  if (_fgLoaderPromise) return _fgLoaderPromise
  setStage('module')
  _fgLoaderPromise = import(
    /* webpackChunkName: "3d-force-graph" */
    /* @vite-ignore */
    '3d-force-graph'
  ).then(m => { fgModule = markRaw(m.default || m); return fgModule })
    .then((m) => { setStage('render'); return m })
    .catch((err) => {
      // [P1-1 鲁棒性修复] 失败后清除缓存，下一次 reload() 允许重试（否则缓存 rejected Promise → 永久失败直到整页刷新）
      _fgLoaderPromise = null
      throw err
    })
  return _fgLoaderPromise
}

/* ===== 布局算法 ===== */

// 同心圆环布局（按节点类型分组，每个类型一个圆环）
function applyCircularLayout(graphData, centerX = 0, centerY = 0, centerZ = 0) {
  const nodes = [...graphData.nodes]
  const edges = [...graphData.edges]

  // 按类型分组
  const groups = {}
  nodes.forEach(n => {
    const type = n.node_type || 'default'
    if (!groups[type]) groups[type] = []
    groups[type].push(n)
  })

  const typeList = Object.keys(groups)
  const ringCount = typeList.length
  const baseRadius = 60 + layoutConfig.nodeSize * 2

  typeList.forEach((type, ringIdx) => {
    const groupNodes = groups[type]
    const n = groupNodes.length
    const radius = baseRadius + ringIdx * (layoutConfig.linkDistance * 0.8)

    groupNodes.forEach((node, i) => {
      const angle = (i / n) * Math.PI * 2 - Math.PI / 2
      node.x = centerX + radius * Math.cos(angle)
      node.y = centerY + radius * Math.sin(angle)
      node.z = centerZ + (i % 3 - 1) * 10
      node.fx = node.x
      node.fy = node.y
      node.fz = node.z
    })
  })

  return { nodes, links: edges }
}

// 径向布局（从中心节点向外扩散，按度数分层）
function applyRadialLayout(graphData, centerX = 0, centerY = 0) {
  const nodes = [...graphData.nodes]
  const edges = [...graphData.edges]

  // 计算每个节点的度数
  const degreeMap = {}
  nodes.forEach(n => { degreeMap[n.id] = 0 })
  edges.forEach(e => {
    if (degreeMap[e.source] != null) degreeMap[e.source]++
    if (degreeMap[e.target] != null) degreeMap[e.target]++
  })

  // 按度数排序，度数最高的作为中心
  const sorted = [...nodes].sort((a, b) => (degreeMap[b.id] || 0) - (degreeMap[a.id] || 0))
  const centerNode = sorted[0]
  const ringSize = 8 // 每层节点数

  // 分层
  const layers = []
  let idx = 1 // 跳过中心节点
  while (idx < sorted.length) {
    const layer = sorted.slice(idx, idx + ringSize + layers.length * 2)
    layers.push(layer)
    idx += layer.length
  }

  // 中心节点
  if (centerNode) {
    centerNode.x = centerX
    centerNode.y = centerY
    centerNode.z = 0
    centerNode.fx = centerX
    centerNode.fy = centerY
    centerNode.fz = 0
  }

  // 各层节点按圆环分布
  const layerDistance = layoutConfig.linkDistance * 0.9
  layers.forEach((layer, layerIdx) => {
    const radius = (layerIdx + 1) * layerDistance
    const n = layer.length
    layer.forEach((node, i) => {
      const angle = (i / n) * Math.PI * 2 - Math.PI / 2 + layerIdx * 0.3
      node.x = centerX + radius * Math.cos(angle)
      node.y = centerY + radius * Math.sin(angle)
      node.z = (i % 2 === 0 ? 1 : -1) * (layerIdx * 8)
      node.fx = node.x
      node.fy = node.y
      node.fz = node.z
    })
  })

  return { nodes, links: edges }
}

// 分层布局（按层级从上到下，基于拓扑排序）
function applyHierarchicalLayout(graphData, centerX = 0) {
  const nodes = [...graphData.nodes]
  const edges = [...graphData.edges]

  // 构建邻接表
  const adj = {}
  const inDegree = {}
  nodes.forEach(n => {
    adj[n.id] = []
    inDegree[n.id] = 0
  })
  edges.forEach(e => {
    if (adj[e.source]) adj[e.source].push(e.target)
    if (inDegree[e.target] != null) inDegree[e.target]++
  })

  // 拓扑分层（BFS 从入度为0的节点开始）
  const layers = []
  const visited = new Set()
  const layerHeight = layoutConfig.linkDistance * 1.2
  const nodeWidth = layoutConfig.nodeSize * 2.5

  // 第一层：入度为0的节点
  let currentLayer = nodes.filter(n => (inDegree[n.id] || 0) === 0).map(n => n.id)
  if (currentLayer.length === 0 && nodes.length > 0) {
    // 没有根节点，取度数最高的作为根
    const degreeMap = {}
    nodes.forEach(n => { degreeMap[n.id] = 0 })
    edges.forEach(e => {
      if (degreeMap[e.source] != null) degreeMap[e.source]++
      if (degreeMap[e.target] != null) degreeMap[e.target]++
    })
    const root = nodes.reduce((a, b) => (degreeMap[a.id] || 0) > (degreeMap[b.id] || 0) ? a : b)
    currentLayer = [root.id]
  }

  while (currentLayer.length > 0) {
    layers.push(currentLayer)
    currentLayer.forEach(id => visited.add(id))

    const nextLayer = new Set()
    currentLayer.forEach(id => {
      (adj[id] || []).forEach(nextId => {
        if (!visited.has(nextId)) {
          nextLayer.add(nextId)
        }
      })
    })
    currentLayer = [...nextLayer]
  }

  // 把未访问的节点加到最后一层
  const unvisited = nodes.filter(n => !visited.has(n.id)).map(n => n.id)
  if (unvisited.length > 0) {
    layers.push(unvisited)
  }

  // 计算位置
  const totalHeight = (layers.length - 1) * layerHeight
  const startY = -totalHeight / 2

  layers.forEach((layer, layerIdx) => {
    const y = startY + layerIdx * layerHeight
    const totalWidth = (layer.length - 1) * nodeWidth
    const startX = centerX - totalWidth / 2

    layer.forEach((nodeId, i) => {
      const node = nodes.find(n => n.id === nodeId)
      if (node) {
        node.x = startX + i * nodeWidth
        node.y = y
        node.z = (i % 2 === 0 ? 1 : -1) * 5
        node.fx = node.x
        node.fy = node.y
        node.fz = node.z
      }
    })
  })

  return { nodes, links: edges }
}

// 网格布局（按类型分类，每类一个网格区域）
function applyGridLayout(graphData) {
  const nodes = [...graphData.nodes]
  const edges = [...graphData.edges]

  // 按类型分组
  const groups = {}
  nodes.forEach(n => {
    const type = n.node_type || 'default'
    if (!groups[type]) groups[type] = []
    groups[type].push(n)
  })

  const typeList = Object.keys(groups)
  const cols = Math.ceil(Math.sqrt(typeList.length))
  const rows = Math.ceil(typeList.length / cols)

  const regionWidth = 180
  const regionHeight = 140
  const nodeGap = 28 + layoutConfig.nodeSize

  typeList.forEach((type, idx) => {
    const col = idx % cols
    const row = Math.floor(idx / cols)
    const regionX = (col - (cols - 1) / 2) * regionWidth
    const regionY = (row - (rows - 1) / 2) * regionHeight

    const groupNodes = groups[type]
    const n = groupNodes.length
    const ncols = Math.ceil(Math.sqrt(n))
    const nrows = Math.ceil(n / ncols)

    groupNodes.forEach((node, i) => {
      const nc = i % ncols
      const nr = Math.floor(i / ncols)
      node.x = regionX + (nc - (ncols - 1) / 2) * nodeGap
      node.y = regionY + (nr - (nrows - 1) / 2) * nodeGap
      node.z = 0
      node.fx = node.x
      node.fy = node.y
      node.fz = node.z
    })
  })

  return { nodes, links: edges }
}

// Fruchterman-Reingold 力导向布局（更稳定的力导向算法）
function applyFruchtermanLayout(graphData) {
  const nodes = [...graphData.nodes]
  const edges = [...graphData.edges]

  // 先用圆形布局初始化，避免随机位置导致的混乱
  const n = nodes.length
  const radius = 100 + n * 2
  nodes.forEach((node, i) => {
    const angle = (i / n) * Math.PI * 2
    node.x = radius * Math.cos(angle)
    node.y = radius * Math.sin(angle)
    node.z = (i % 5 - 2) * 15
    node.fx = undefined
    node.fy = undefined
    node.fz = undefined
  })

  return { nodes, links: edges }
}

// 应用指定布局
function applyLayout(mode) {
  layoutMode.value = mode
  if (!fg || !currentGraphData.value.nodes.length) return

  const data = {
    nodes: JSON.parse(JSON.stringify(currentGraphData.value.nodes)),
    edges: currentGraphData.value.edges
  }

  let layouted
  switch (mode) {
    case 'circular':
      layouted = applyCircularLayout(data)
      break
    case 'radial':
      layouted = applyRadialLayout(data)
      break
    case 'hierarchical':
      layouted = applyHierarchicalLayout(data)
      break
    case 'grid':
      layouted = applyGridLayout(data)
      break
    case 'fruchterman':
      layouted = applyFruchtermanLayout(data)
      // FR 布局需要力学迭代，设置更强的参数
      if (typeof fg.d3Force === 'function') {
        const charge = fg.d3Force('charge')
        if (charge && typeof charge.strength === 'function') {
          charge.strength(-layoutConfig.repulsion * 1.5)
        }
        const link = fg.d3Force('link')
        if (link && typeof link.distance === 'function') {
          link.distance(layoutConfig.linkDistance * 0.8)
        }
      }
      break
    case 'force':
    default:
      // 标准力导向：释放所有固定位置
      data.nodes.forEach(n => {
        n.fx = undefined
        n.fy = undefined
        n.fz = undefined
      })
      layouted = { nodes: data.nodes, links: data.edges }
      break
  }

  // 应用节点大小和标签配置
  layouted.nodes.forEach(n => {
    n.val = layoutConfig.nodeSize
    n.size = layoutConfig.nodeSize
  })

  fg.graphData({ nodes: layouted.nodes, links: layouted.links })

  // 如果是固定布局，需要重新加热力导向引擎让它稳定
  if (mode === 'force' || mode === 'fruchterman') {
    if (typeof fg.d3ReheatSimulation === 'function') {
      try { fg.d3ReheatSimulation() } catch (_) {}
    } else if (typeof fg.refresh === 'function') {
      try { fg.refresh() } catch (_) {}
    }
  }

  updateVisualConfig()
  ElMessage.success(`已切换到${getLayoutName(mode)}`)
}

// 获取布局名称
function getLayoutName(mode) {
  const names = {
    force: '力导向布局',
    circular: '同心圆环布局',
    radial: '径向布局',
    hierarchical: '分层布局',
    grid: '网格布局',
    fruchterman: 'FR 力导向布局'
  }
  return names[mode] || mode
}

// 更新可视化配置（节点大小、标签、连线样式等）
function updateVisualConfig() {
  if (!fg) return

  fg.nodeVal(layoutConfig.nodeSize)
    .nodeOpacity(0.95)
    .linkWidth(layoutConfig.linkWidth)
    .linkOpacity(layoutConfig.linkOpacity)

  if (layoutConfig.showLabels) {
    fg.nodeLabel((n) => n.label || n.id)
    if (typeof fg.linkDirectionalParticleWidth === 'function') {
      // 标签通过 HTML overlay 或 3D text 实现，这里简化处理
    }
  } else {
    fg.nodeLabel('')
  }
}

// 更新布局（配置变化时调用）
function updateLayout() {
  if (!fg) return

  // 更新力学参数
  if (typeof fg.d3Force === 'function') {
    const charge = fg.d3Force('charge')
    if (charge && typeof charge.strength === 'function') {
      charge.strength(-layoutConfig.repulsion)
    }
    const link = fg.d3Force('link')
    if (link && typeof link.distance === 'function') {
      link.distance(layoutConfig.linkDistance)
    }
    const center = fg.d3Force('center')
    if (center && typeof center.strength === 'function') {
      center.strength(layoutConfig.gravity)
    }
  }

  updateVisualConfig()

  // 重新加热模拟
  if (layoutMode.value === 'force' || layoutMode.value === 'fruchterman') {
    if (typeof fg.d3ReheatSimulation === 'function') {
      try { fg.d3ReheatSimulation() } catch (_) {}
    }
  }
}

// 导出截图
function screenshotGraph() {
  if (!fg || !graphEl.value) return
  try {
    const canvas = graphEl.value.querySelector('canvas')
    if (canvas) {
      const link = document.createElement('a')
      link.download = `knowledge-graph-${layoutMode.value}-${Date.now()}.png`
      link.href = canvas.toDataURL('image/png')
      link.click()
      ElMessage.success('截图已导出')
    }
  } catch (e) {
    ElMessage.error('导出失败：' + e.message)
  }
}

// 静态圆形布局（用于"力学前先出首帧"，让用户"先看到结构"再等待力学收敛 2-3s）
function applyStaticCircularLayout(graphData, radius = 180) {
  const n = Math.max(1, graphData.nodes.length)
  graphData.nodes.forEach((node, i) => {
    const theta = (i / n) * Math.PI * 2
    // 轻微随机 Z 轴，避免所有点重合导致"画面扁平"
    node.x = node.x ?? (radius * Math.cos(theta))
    node.y = node.y ?? (radius * Math.sin(theta))
    node.z = node.z ?? ((i % 5 - 2) * 22)
    node.fx = node.fy = node.fz = undefined // 允许后续力学接管
  })
  return graphData
}

// ---------- 其余原有状态（搜索 / 路径 / 邻居 / 推荐 / 中心性 / 社区 / 激活） ----------
// 统一搜索（对话 + 图谱节点）
const searchQ = ref('')
const searchResult = ref(null)
async function doSearch() {
  const q = searchQ.value.trim()
  if (!q) return
  try {
    const res = await graphSearch(q, 30)
    // 归一化后端返回，兼容 {dialogues, graph_nodes} 与 {results} 两种契约，避免 undefined.length 崩溃
    const src = (res && typeof res === 'object') ? res : {}
    const results = Array.isArray(src.results) ? src.results : []
    searchResult.value = {
      dialogues: Array.isArray(src.dialogues) ? src.dialogues
        : results.filter(r => String(r.type || r.kind || '').includes('dialogue')),
      graph_nodes: Array.isArray(src.graph_nodes) ? src.graph_nodes
        : results.filter(r => !String(r.type || r.kind || '').includes('dialogue')),
    }
  } catch (e) {
    ElMessage.error('搜索失败：' + e.message)
  }
}
function clearSearch() {
  searchQ.value = ''
  searchResult.value = null
}

const tab = ref('path')
const pathSrc = ref('')
const pathDst = ref('')
const pathResult = ref(null)
const loadingPath = ref(false)

const nbId = ref('')
const neighbors = ref([])
const loadingNb = ref(false)

const recCtx = ref([])
const recs = ref([])
const loadingRec = ref(false)

// 中心性分析：pagerank / degree / betweenness
const centType = ref('pagerank')
const centrality = ref([])
const loadingCent = ref(false)
async function loadCentrality() {
  loadingCent.value = true
  try {
    if (centType.value === 'pagerank') {
      const map = await getPagerank()
      centrality.value = Object.entries(map.pagerank || {})
        .map(([id, value]) => ({ id, value: Number(value) || 0 }))
        .sort((a, b) => b.value - a.value)
    } else if (centType.value === 'degree') {
      const metrics = await getCentrality()
      centrality.value = Object.entries(metrics.degree || {})
        .map(([id, info]) => ({ id, value: Number(info?.normalized) || Number(info?.degree) || 0 }))
        .sort((a, b) => b.value - a.value)
    } else {
      const metrics = await getCentrality()
      centrality.value = Object.entries(metrics.betweenness || {})
        .map(([id, value]) => ({ id, value: Number(value) || 0 }))
        .sort((a, b) => b.value - a.value)
    }
  } catch (e) {
    ElMessage.error('中心性计算失败：' + e.message)
  } finally {
    loadingCent.value = false
  }
}

// 社区发现
const communities = ref([])
const loadingComm = ref(false)
async function loadCommunities() {
  loadingComm.value = true
  try {
    const map = await getCommunities()
    communities.value = (map.communities || []).map(c => ({
      id: c.id,
      nodes: Array.isArray(c.members) ? c.members : (c.members || '').split(/\s+/).filter(Boolean),
      size: c.size || 0
    }))
  } catch (e) {
    ElMessage.error('社区检测失败：' + e.message)
  } finally {
    loadingComm.value = false
  }
}

// 激活传播：从种子节点沿边扩散激活能量，识别影响力节点
const actSeeds = ref([])
const actIter = ref(10)
const activation = ref([])
const loadingAct = ref(false)
async function doPropagate() {
  if (!actSeeds.value.length) {
    ElMessage.warning('请选择至少一个种子节点')
    return
  }
  loadingAct.value = true
  try {
    const map = await propagateActivation(actSeeds.value, actIter.value)
    activation.value = Object.entries(map.energy || {})
      .map(([id, value]) => ({ id, value: Number(value) || 0 }))
      .filter(a => a.value > 0)
      .sort((a, b) => b.value - a.value)
  } catch (e) {
    ElMessage.error('激活传播失败：' + e.message)
  } finally {
    loadingAct.value = false
  }
}

const statCards = computed(() => {
  const s = stats.value || {}
  return [
    { label: '节点数', value: s.nodes ?? 0 },
    { label: '边数', value: s.edges ?? 0 },
    { label: '密度', value: (s.density ?? 0).toFixed(3) },
    { label: '社区数', value: s.communities ?? 0 }
  ]
})

async function reload() {
  setStage('fetch')
  try {
    // [P1-1 真正并行化修复] 启动两条任务同时并发：
    //   task A = 后端取图数据 & stats（API IO-bound）
    //   task B = 动态 import 3D 重库 chunk（1.3MB，network-bound）
    //   两条并行跑，最差情况 = 串行（两者共用带宽），最优情况节省 min(Ta, Tb) ≈ 60% 首屏等待
    const fetchTask = (async () => {
      const [g, st] = await Promise.all([getGraph(), getGraphStats()])
      stats.value = st
      nodeIds.value = g.nodes.map((n) => n.id)
      // 保存当前图谱数据供布局切换使用
      currentGraphData.value = { nodes: g.nodes, edges: g.edges }
      return { g, st }
    })()
    const load3dTask = loadForceGraph3DModule()
    // 允许取数先返回 → 立刻把 stats 卡片点亮（用户"先看到数据再等 3D canvas"）
    const [{ g }, ForceGraph3D] = await Promise.all([fetchTask, load3dTask])
    if (!graphEl.value) await nextTick()
    if (fg) {
      applyStaticCircularLayout(g)
      fg.graphData({ nodes: g.nodes, links: g.edges })
    } else {
      applyStaticCircularLayout(g)
      initGraph(ForceGraph3D, g)
    }
    // 应用初始可视化配置
    updateVisualConfig()
    // [P1-1 力学后置] 首帧先显示静态布局，260ms 后再启动力导向引擎，避免"白屏等力学收敛 2-3s"
    setTimeout(() => {
      if (!fg) return
      // 启用全部力学力（charge/collide/link/center），ForceGraph3D 默认引擎是 d3-force，这里显式 warm up
      if (typeof fg.d3Force === 'function') {
        const charge = fg.d3Force('charge')
        if (charge && typeof charge.strength === 'function') charge.strength(-layoutConfig.repulsion)
        const link = fg.d3Force('link')
        if (link && typeof link.distance === 'function') link.distance(layoutConfig.linkDistance)
        const center = fg.d3Force('center')
        if (center && typeof center.strength === 'function') center.strength(layoutConfig.gravity)
        const collide = fg.d3Force('collision')
        if (collide && typeof collide.radius === 'function') collide.radius(18)
        // 冷启动后让 d3-force 重新"热起来"：用 d3Reheat => fg 内部暴露 d3ReheatSimulation?
        if (typeof fg.d3ReheatSimulation === 'function') {
          try { fg.d3ReheatSimulation() } catch (_) { /* ignore */ }
        } else if (typeof fg.refresh === 'function') {
          try { fg.refresh() } catch (_) { /* ignore */ }
        }
      }
      setStage('physics')
    }, 260)
  } catch (e) {
    setStage('skeleton')
    ElMessage.error('图谱加载失败：' + e.message)
  }
}

function initGraph(ForceGraph3D, g) {
  fg = ForceGraph3D()(graphEl.value)
    .backgroundColor('#0b1020')
    .graphData({ nodes: g.nodes, links: g.edges })
    .nodeLabel((n) => `${n.label} (${n.node_type})`)
    .nodeColor((n) => n.color)
    .nodeVal((n) => n.size)
    .linkColor(() => 'rgba(148,163,184,0.35)')
    .linkWidth(0.5)
    .nodeOpacity(0.95)
    .enableNodeDrag(false)
    // [P1-1] 首帧静态布局 + 力学冷却阈值更"宽松"（温度降得更快）
    .warmupTicks(0)
    .cooldownTicks(180)
    .cooldownTime(2500)
  fg.cameraPosition({ z: 320 })
}

async function findPath() {
  if (!pathSrc.value || !pathDst.value) {
    ElMessage.warning('请选择起点和终点')
    return
  }
  loadingPath.value = true
  try {
    pathResult.value = await getShortestPath(pathSrc.value, pathDst.value)
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    loadingPath.value = false
  }
}

async function findNb() {
  if (!nbId.value) return
  loadingNb.value = true
  try {
    neighbors.value = await getNeighbors(nbId.value)
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    loadingNb.value = false
  }
}

async function findRec() {
  if (!recCtx.value.length) {
    ElMessage.warning('请选择上下文节点')
    return
  }
  loadingRec.value = true
  try {
    recs.value = await recommendNodes({ context_nodes: recCtx.value, limit: 8 })
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    loadingRec.value = false
  }
}

onMounted(reload)
onBeforeUnmount(() => {
  if (fg && fg._destructor) fg._destructor()
})

// ===== 璇玑：以项目为核心的联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  let _loaded = false
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { reload() })
    await _ensureProject().catch(() => {})
    if (!_loaded) {
      _loaded = true
      reload()
    }
  })
  const _ob$ = onBeforeUnmount == null ? null : onBeforeUnmount(() => { _offPj && _offPj() })
  // 若脚本未引入 onBeforeUnmount，退化为 window beforeunload 兜底（页面关闭）
  if (typeof onBeforeUnmount === 'undefined') {
    // 不操作：Vue 路由离开时组件 destroy，本作用域已销毁
  }
}
</script>

<style scoped>
.gv {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.stat-row {
  margin: 0;
}
.stat {
  padding: 16px 18px;
}
.stat-value {
  font-size: 22px;
  font-weight: 700;
}

/* 快捷分析入口 */
.quick-actions {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}
@media (max-width: 900px) {
  .quick-actions { grid-template-columns: repeat(2, 1fr); }
}
.qa-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border-1);
  border-radius: 12px;
  cursor: pointer;
  transition: all var(--transition);
}
.qa-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow);
  border-color: var(--brand);
}
.qa-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 20px;
  flex-shrink: 0;
}
.qa-info {
  flex: 1;
  min-width: 0;
}
.qa-title {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-1);
}
.qa-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}
.qa-arrow {
  color: var(--text-3);
  font-size: 14px;
  flex-shrink: 0;
  transition: transform var(--transition);
}
.qa-card:hover .qa-arrow {
  color: var(--brand);
  transform: translateX(3px);
}
.stat-label {
  font-size: 13px;
  color: var(--text-3);
}
.graph-grid {
  grid-template-columns: 1fr 360px;
}
@media (max-width: 1100px) {
  .graph-grid {
    grid-template-columns: 1fr;
  }
}
.graph-box {
  padding: 18px;
  position: relative;
}
.graph-canvas {
  width: 100%;
  height: 520px;
  background: #0b1020;
  border-radius: 12px;
  overflow: hidden;
}
/* P1-1 渐进加载：骨架显示期间把 canvas 设为 opacity 0，避免 WebGL 清屏闪白 */
.graph-canvas.covered { opacity: 0; pointer-events: none; }
.canvas-wrap { position: relative; width: 100%; }
.skeleton-svg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 520px;
  border-radius: 12px;
  z-index: 2;
  display: block;
}
.stage-chip {
  margin-left: 10px;
  padding: 2px 9px;
  font-weight: 500;
  border-radius: 999px;
  background: rgba(99, 102, 241, 0.10);
  color: #818cf8;
  font-size: 12px;
  letter-spacing: 0.3px;
}
.ok-chip {
  margin-left: 10px;
  padding: 2px 9px;
  font-weight: 600;
  border-radius: 999px;
  background: rgba(34, 197, 94, 0.10);
  color: #22c55e;
  font-size: 12px;
  letter-spacing: 0.3px;
}
.stage-bar-wrap {
  position: absolute;
  z-index: 3;
  right: 18px;
  bottom: 18px;
  width: 280px;
  background: rgba(15, 23, 42, 0.65);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  padding: 10px 12px;
  border-radius: 10px;
  border: 1px solid rgba(148, 163, 184, 0.12);
}
.stage-bar {
  height: 6px;
  background: rgba(148, 163, 184, 0.15);
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 8px;
}
.stage-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, #6366f1 0%, #22d3ee 100%);
  border-radius: 4px;
  transition: width 420ms cubic-bezier(0.22, 1, 0.36, 1);
}
.stage-hints {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 4px;
  font-size: 11px;
  color: #64748b;
}
.stage-hints span {
  opacity: 0.55;
  transition: opacity 0.3s ease, color 0.3s ease;
}
.stage-hints span.active {
  opacity: 1;
  color: #818cf8;
  font-weight: 600;
}
.legend {
  position: absolute;
  bottom: 26px;
  left: 26px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  max-width: 60%;
}
.lg {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: #cbd5e1;
  background: rgba(15, 23, 42, 0.6);
  padding: 2px 7px;
  border-radius: 6px;
}
.lg i {
  width: 9px;
  height: 9px;
  border-radius: 50%;
}
.side {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.card-pad {
  padding: 18px 20px;
}
.path-result {
  margin-top: 12px;
  font-size: 13px;
  color: var(--text-1);
  background: var(--bg-page);
  padding: 10px 12px;
  border-radius: 8px;
}
.nb-list {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 200px;
  overflow: auto;
}
.nb {
  font-size: 13px;
  padding: 6px 10px;
  background: var(--bg-page);
  border-radius: 7px;
}

/* 布局设置面板 */
.layout-panel {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 260px;
  background: rgba(15, 23, 42, 0.95);
  border: 1px solid rgba(148, 163, 184, 0.15);
  border-radius: 12px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  z-index: 10;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}
.lp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.1);
}
.lp-title {
  font-size: 14px;
  font-weight: 700;
  color: #e2e8f0;
}
.lp-body {
  padding: 12px 14px;
  max-height: 460px;
  overflow-y: auto;
}
.lp-section {
  margin-bottom: 14px;
}
.lp-section:last-child {
  margin-bottom: 0;
}
.lp-label {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  font-weight: 600;
  color: #cbd5e1;
  margin-bottom: 6px;
}
.lp-value {
  position: absolute;
  right: 0;
  top: 0;
  font-size: 11px;
  color: #6366f1;
  font-weight: 700;
}
.lp-section .el-slider {
  position: relative;
  padding-right: 36px;
}
.lp-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid rgba(148, 163, 184, 0.1);
}
.lp-actions .el-button {
  flex: 1;
}

/* 下拉菜单项激活态 */
.el-dropdown-menu__item.active {
  color: var(--brand-primary, #6366f1);
  font-weight: 600;
  background: var(--brand-soft, rgba(99, 102, 241, 0.08));
}
.muted {
  color: var(--text-3);
  font-size: 12px;
}

/* ===== 分层式知识图谱页面 ===== */
.graph-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* 顶部操作栏 */
.graph-topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  background: var(--bg-card);
  border-bottom: 1px solid var(--border-1);
  flex-shrink: 0;
  z-index: 5;
}
.gt-left { display: flex; align-items: center; gap: 16px; }
.gt-title-wrap { display: flex; align-items: center; gap: 12px; }
.gt-icon {
  width: 40px; height: 40px; border-radius: 12px;
  background: linear-gradient(135deg, #6366f1, #06b6d4);
  display: grid; place-items: center;
  font-size: 20px;
  flex-shrink: 0;
}
.gt-title {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-1);
  margin: 0;
}
.gt-sub {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-top: 2px;
}
.gt-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-2);
  font-weight: 500;
}
.gt-dot {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--text-3);
}
.gt-dot.ok {
  background: var(--success);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2);
}
.gt-stage {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  background: var(--bg-page);
  color: var(--text-3);
  font-weight: 500;
}
.gt-stage.ok {
  background: rgba(16, 185, 129, 0.1);
  color: var(--success);
}
.gt-right { display: flex; align-items: center; gap: 10px; }

.gt-search {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: 0 10px;
  border-radius: 10px;
  width: 220px;
  background: var(--bg-page);
  border: 1px solid var(--border-1);
  transition: all 0.2s;
  position: relative;
}
.gt-search:focus-within {
  width: 280px;
  border-color: var(--brand);
  box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.12);
  background: var(--bg-input-focus);
}
.gt-search .el-icon { color: var(--text-3); font-size: 16px; flex-shrink: 0; }
.gt-search-input {
  all: unset;
  flex: 1;
  font-size: 13px;
  color: var(--text-1);
}
.gt-search-input::placeholder { color: var(--text-3); }
.gt-kbd {
  font-size: 10px;
  padding: 1px 6px;
  background: rgba(148, 163, 184, 0.15);
  color: var(--text-3);
  border-radius: 4px;
  flex-shrink: 0;
  font-family: ui-monospace, monospace;
}

.gt-primary-btn {
  height: 36px;
  border-radius: 10px;
  padding: 0 16px;
  font-weight: 600;
}
.gt-icon-btn {
  width: 36px; height: 36px;
  border-radius: 10px;
  padding: 0;
  display: grid;
  place-items: center;
}

/* 主工作区 */
.graph-main {
  flex: 1;
  display: flex;
  min-height: 0;
  position: relative;
}

/* 左侧工具面板 */
.side-panel {
  width: 260px;
  flex-shrink: 0;
  background: var(--bg-card);
  border-right: 1px solid var(--border-1);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  transition: width 0.25s ease;
}
.side-panel.collapsed {
  width: 0;
  overflow: hidden;
  border-right: none;
}

.sp-section {
  border-bottom: 1px solid var(--border-1);
}
.sp-section:last-child { border-bottom: none; }

.sp-section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 14px;
  cursor: pointer;
  user-select: none;
  transition: background 0.15s;
}
.sp-section-header:hover {
  background: var(--bg-page);
}
.sp-section-icon {
  font-size: 16px;
  width: 22px;
  text-align: center;
  flex-shrink: 0;
}
.sp-section-title {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-1);
}
.sp-section-arrow {
  font-size: 12px;
  color: var(--text-3);
  transition: transform 0.2s;
}

.sp-section-body {
  padding: 4px 14px 14px;
}

.sp-expand-enter-active,
.sp-expand-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}
.sp-expand-enter-from,
.sp-expand-leave-to {
  opacity: 0;
  max-height: 0;
  padding-top: 0;
  padding-bottom: 0;
}
.sp-expand-enter-to,
.sp-expand-leave-from {
  opacity: 1;
  max-height: 400px;
}

/* 布局选择网格 */
.layout-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 6px;
}
.layout-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 10px 4px;
  border-radius: 8px;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s;
}
.layout-option:hover {
  background: var(--bg-page);
  border-color: var(--border-1);
}
.layout-option.active {
  background: var(--brand-soft);
  border-color: var(--brand);
}
.lo-icon { font-size: 20px; }
.lo-name { font-size: 11px; color: var(--text-2); font-weight: 500; }
.layout-option.active .lo-name { color: var(--brand-dark); font-weight: 600; }

/* 样式调节行 */
.style-row {
  margin-bottom: 12px;
}
.style-row:last-child { margin-bottom: 0; }
.style-row label {
  display: block;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-2);
  margin-bottom: 4px;
}
.style-row .el-slider { margin: 0; }
.style-val {
  position: absolute;
  right: 0;
  top: 0;
  font-size: 11px;
  color: var(--brand);
  font-weight: 600;
}
.style-row .el-slider { position: relative; padding-right: 36px; }

.sp-actions {
  display: flex;
  gap: 8px;
  margin-top: 14px;
  padding-top: 12px;
  border-top: 1px solid var(--border-1);
}
.sp-actions .el-button { flex: 1; }

/* 快捷分析列表 */
.qa-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.qa-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.qa-item:hover {
  background: var(--bg-page);
}
.qa-icon {
  width: 32px; height: 32px;
  border-radius: 8px;
  display: grid; place-items: center;
  font-size: 14px;
  flex-shrink: 0;
}
.qa-info { flex: 1; min-width: 0; }
.qa-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-1);
}
.qa-desc {
  font-size: 11px;
  color: var(--text-3);
  margin-top: 1px;
}

/* 图例列表 */
.legend-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.legend-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-2);
}
.legend-dot {
  width: 10px; height: 10px;
  border-radius: 3px;
  flex-shrink: 0;
}
.legend-label { flex: 1; }

/* 中央画布区 */
.graph-canvas-wrap {
  flex: 1;
  position: relative;
  min-width: 0;
  background: #0b1020;
}

.skeleton-svg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  z-index: 2;
}

.graph-canvas {
  width: 100%;
  height: 100%;
}
.graph-canvas.covered {
  opacity: 0;
}

/* 右下角统计条 */
.graph-statbar {
  position: absolute;
  right: 16px;
  bottom: 16px;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 10px 16px;
  background: rgba(15, 23, 42, 0.9);
  border: 1px solid rgba(148, 163, 184, 0.15);
  border-radius: 12px;
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  z-index: 10;
}
.gs-item {
  text-align: center;
}
.gs-value {
  font-size: 16px;
  font-weight: 700;
  color: #e2e8f0;
}
.gs-label {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}
.gs-divider {
  width: 1px;
  height: 28px;
  background: rgba(148, 163, 184, 0.2);
}

/* 底部分析抽屉 */
.drawer-up-enter-active,
.drawer-up-leave-active {
  transition: all 0.3s ease;
}
.drawer-up-enter-from,
.drawer-up-leave-to {
  transform: translateY(100%);
  opacity: 0;
}

.analysis-drawer {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  max-height: 45%;
  background: var(--bg-card);
  border-top: 1px solid var(--border-1);
  border-radius: 16px 16px 0 0;
  display: flex;
  flex-direction: column;
  z-index: 20;
  box-shadow: 0 -8px 32px rgba(0, 0, 0, 0.1);
}
.ad-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border-1);
  flex-shrink: 0;
}
.ad-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1);
}
.ad-title .el-icon { color: var(--brand); }
.ad-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}
.ad-content {
  margin: 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-2);
  background: var(--bg-page);
  padding: 12px;
  border-radius: 8px;
  overflow-x: auto;
}
</style>

