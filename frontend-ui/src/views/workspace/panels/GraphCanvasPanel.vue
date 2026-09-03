<!--
  图谱画布面板
  职责：画布工具栏、SVG图谱渲染、节点选中信息浮层、加载态
-->
<template>
  <div class="ws-graph-section">
    <!-- 画布工具栏 -->
    <div class="ws-canvas-toolbar">
      <div class="ws-canvas-tools-left">
        <button
          v-for="tool in canvasTools"
          :key="tool.key"
          class="ws-canvas-tool"
          :class="{ active: activeCanvasTool === tool.key }"
          :title="tool.label"
          @click="$emit('update:activeCanvasTool', tool.key)"
        >
          <el-icon><component :is="tool.icon" /></el-icon>
        </button>
        <div class="ws-tool-divider"></div>
        <button class="ws-canvas-tool" title="放大" @click="$emit('zoom-in')">
          <el-icon><ZoomIn /></el-icon>
        </button>
        <button class="ws-canvas-tool" title="缩小" @click="$emit('zoom-out')">
          <el-icon><ZoomOut /></el-icon>
        </button>
        <button class="ws-canvas-tool" title="适应视图" @click="$emit('fit-view')">
          <el-icon><FullScreen /></el-icon>
        </button>
      </div>

      <div class="ws-canvas-tools-center">
        <div class="ws-layout-switcher">
          <button
            v-for="layout in layouts"
            :key="layout.key"
            class="ws-layout-btn"
            :class="{ active: currentLayout === layout.key }"
            @click="$emit('switch-layout', layout.key)"
          >
            <span>{{ layout.icon }}</span>
            <span class="ws-layout-label">{{ layout.label }}</span>
          </button>
        </div>
      </div>

      <div class="ws-canvas-tools-right">
        <div class="ws-graph-stats">
          <span class="ws-stat-item"><strong>{{ graphStats.nodes }}</strong> 节点</span>
          <span class="ws-stat-divider">·</span>
          <span class="ws-stat-item"><strong>{{ graphStats.edges }}</strong> 关系</span>
          <span class="ws-stat-divider">·</span>
          <span class="ws-stat-item"><strong>{{ graphStats.types }}</strong> 类型</span>
        </div>
        <el-button size="small" type="primary" plain @click="$emit('run-graph-algo')" :loading="graphAnalyzing">
          <el-icon><DataAnalysis /></el-icon>
          图谱分析
        </el-button>
      </div>
    </div>

    <!-- 图谱画布区 -->
    <div
      class="ws-graph-canvas"
      ref="canvasRef"
      @mousedown="$emit('canvas-mousedown', $event)"
      @mousemove="$emit('canvas-mousemove', $event)"
      @mouseup="$emit('canvas-mouseup', $event)"
      @wheel="$emit('canvas-wheel', $event)"
    >
      <svg class="ws-graph-svg" :viewBox="svgViewBox" preserveAspectRatio="xMidYMid meet">
        <defs>
          <pattern id="wsGrid" width="40" height="40" patternUnits="userSpaceOnUse" patternTransform="translate(0,0)">
            <path d="M 40 0 L 0 0 0 40" fill="none" stroke="rgba(99,102,241,0.06)" stroke-width="1"/>
          </pattern>
          <radialGradient id="nodeGlow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" style="stop-color:#6366f1;stop-opacity:0.35" />
            <stop offset="100%" style="stop-color:#6366f1;stop-opacity:0" />
          </radialGradient>
          <radialGradient id="nodeGlowCyan" cx="50%" cy="50%" r="50%">
            <stop offset="0%" style="stop-color:#06b6d4;stop-opacity:0.35" />
            <stop offset="100%" style="stop-color:#06b6d4;stop-opacity:0" />
          </radialGradient>
          <filter id="nodeShadow" x="-50%" y="-50%" width="200%" height="200%">
            <feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#0f172a" flood-opacity="0.15"/>
          </filter>
          <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
            <polygon points="0 0, 10 3.5, 0 7" fill="#94a3b8" opacity="0.6"/>
          </marker>
        </defs>
        <rect width="100%" height="100%" fill="url(#wsGrid)" />

        <!-- 边（关系） -->
        <g class="ws-edges">
          <line v-for="edge in graphEdges" :key="edge.id"
            :x1="edge.sourceX" :y1="edge.sourceY"
            :x2="edge.targetX" :y2="edge.targetY"
            :stroke="edge.color || '#94a3b8'"
            :stroke-width="edge.width || 1.5"
            :stroke-opacity="edge.highlight ? 0.9 : 0.5"
            class="ws-edge"
            :class="{ highlight: edge.highlight }"
          />
        </g>

        <!-- 节点 -->
        <g class="ws-nodes">
          <g v-for="node in graphNodes" :key="node.id"
            :transform="`translate(${node.x}, ${node.y})`"
            class="ws-node"
            :class="{ selected: selectedNode?.id === node.id, highlight: node.highlight }"
            @click.stop="$emit('select-node', node)"
            @mousedown.stop="$emit('node-mousedown', $event, node)"
          >
            <circle v-if="node.highlight || selectedNode?.id === node.id" :r="(node.size || 18) + 10" :fill="node.id === selectedNode?.id ? 'url(#nodeGlow)' : 'url(#nodeGlowCyan)'" />
            <circle :r="node.size || 18" :fill="nodeColor(node)" :opacity="0.9" filter="url(#nodeShadow)" />
            <text text-anchor="middle" :dy="(node.size || 18) > 20 ? 4 : 3" fill="white" :font-size="(node.size || 18) > 20 ? 11 : 10" font-weight="600">
              {{ node.label }}
            </text>
          </g>
        </g>
      </svg>

      <!-- 选中节点信息浮层 -->
      <div v-if="selectedNode" class="ws-node-info-card" :style="nodeCardStyle">
        <div class="ws-node-info-header">
          <span class="ws-node-info-icon" :style="{ background: nodeColor(selectedNode) }">{{ selectedNode.label }}</span>
          <div class="ws-node-info-head-text">
            <div class="ws-node-info-title">{{ selectedNode.fullName || selectedNode.label }}</div>
            <div class="ws-node-info-type">{{ selectedNode.type }}</div>
          </div>
          <button class="ws-node-close" @click.stop="$emit('clear-selected-node')">
            <el-icon><Close /></el-icon>
          </button>
        </div>
        <div class="ws-node-info-body">
          <div class="ws-node-info-row">
            <span class="ws-node-info-label">关联文档</span>
            <span class="ws-node-info-value">{{ selectedNode.docs || 0 }} 篇</span>
          </div>
          <div class="ws-node-info-row">
            <span class="ws-node-info-label">关联专家</span>
            <span class="ws-node-info-value">{{ selectedNode.experts || 0 }} 位</span>
          </div>
          <div class="ws-node-info-row">
            <span class="ws-node-info-label">中心性排名</span>
            <span class="ws-node-info-value">#{{ selectedNode.rank || '-' }}</span>
          </div>
          <div v-if="selectedNode.description" class="ws-node-info-desc">
            {{ selectedNode.description }}
          </div>
        </div>
        <div class="ws-node-info-actions">
          <el-button size="small" @click="$emit('view-node-docs', selectedNode)">
            <el-icon><Document /></el-icon>
            查看文档
          </el-button>
          <el-button size="small" type="primary" @click="$emit('ask-experts-about', selectedNode)">
            <el-icon><ChatDotRound /></el-icon>
            咨询专家
          </el-button>
        </div>
      </div>

      <!-- 加载遮罩 -->
      <div v-if="graphLoading" class="ws-graph-loading">
        <el-icon class="is-loading ws-loading-icon"><Loading /></el-icon>
        <span>加载图谱数据…</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import {
  ZoomIn, ZoomOut, FullScreen, DataAnalysis, Close,
  Document, ChatDotRound, Loading
} from '@element-plus/icons-vue'

const props = defineProps({
  activeCanvasTool: { type: String, default: 'select' },
  currentLayout: { type: String, default: 'force' },
  selectedNode: { type: Object, default: null },
  graphLoading: { type: Boolean, default: false },
  graphAnalyzing: { type: Boolean, default: false },
  graphNodes: { type: Array, default: () => [] },
  graphEdges: { type: Array, default: () => [] },
  graphStats: { type: Object, default: () => ({ nodes: 0, edges: 0, types: 0 }) },
  svgViewBox: { type: String, default: '0 0 800 500' },
  viewport: { type: Object, default: () => ({ x: 0, y: 0, scale: 1 }) }
})

defineEmits([
  'update:activeCanvasTool', 'zoom-in', 'zoom-out', 'fit-view',
  'switch-layout', 'run-graph-algo', 'canvas-mousedown', 'canvas-mousemove',
  'canvas-mouseup', 'canvas-wheel', 'select-node', 'node-mousedown',
  'clear-selected-node', 'view-node-docs', 'ask-experts-about'
])

const canvasTools = [
  { key: 'select', icon: 'Pointer', label: '选择' },
  { key: 'pan', icon: 'Rank', label: '平移' },
  { key: 'add-node', icon: 'Plus', label: '添加节点' },
  { key: 'add-edge', icon: 'Link', label: '添加关系' },
  { key: 'delete', icon: 'Delete', label: '删除' }
]

const layouts = [
  { key: 'force', icon: '🔄', label: '力导向' },
  { key: 'radial', icon: '☀️', label: '辐射' },
  { key: 'hierarchical', icon: '🏛️', label: '层次' },
  { key: 'circular', icon: '⭕', label: '环形' }
]

const nodeCardStyle = computed(() => {
  if (!props.selectedNode) return {}
  const scale = props.viewport.scale
  const nodeX = props.selectedNode.x * scale
  const nodeY = props.selectedNode.y * scale
  return {
    left: Math.min(nodeX + 30, 500) + 'px',
    top: Math.max(nodeY - 60, 20) + 'px'
  }
})

function nodeColor(node) {
  return node.color || '#6366f1'
}
</script>
