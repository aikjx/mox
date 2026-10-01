<template>
  <div class="agc">
    <svg class="agc-svg" :viewBox="`0 0 ${viewport.width} ${viewport.height}`" role="img"
      :aria-label="`协作图谱，${nodeCount} 个节点 ${edgeCount} 条边`">
      <g class="agc-edges">
        <line v-for="(e, i) in drawableEdges" :key="`e${i}`"
          :class="['agc-line', e.edgeType === GRAPH_EDGE_TYPE.has_domain ? 'is-domain' : 'is-collab']"
          :x1="e.x1" :y1="e.y1" :x2="e.x2" :y2="e.y2" :stroke-width="e.width" />
      </g>
      <g class="agc-nodes">
        <g v-for="n in nodes" :key="n.id" class="agc-node" :class="{ 'is-selected': n.id === selectedId }"
          tabindex="0" role="button" :aria-label="`${nodeTypeLabel(n)} ${n.label}`"
          @click="emit('select', n.id)" @keydown.enter.prevent="emit('select', n.id)">
          <circle class="agc-dot" :class="n.nodeType === GRAPH_NODE_TYPE.domain ? 'is-domain' : 'is-expert'"
            :cx="n.x" :cy="n.y" :r="n.radius" />
          <text class="agc-label" :class="{ 'is-domain': n.nodeType === GRAPH_NODE_TYPE.domain }" :x="n.x" :y="n.y + n.radius + 11"
            text-anchor="middle">{{ clipped(n.label) }}</text>
        </g>
      </g>
    </svg>

    <div class="agc-legend">
      <span class="agc-key"><i class="agc-swatch is-expert" />专家（{{ expertCount }}）</span>
      <span class="agc-key"><i class="agc-swatch is-domain" />能力域（{{ domainCount }}）</span>
      <span class="agc-key"><i class="agc-rule is-collab" />协作（相似度 &gt;0.1）</span>
      <span class="agc-key"><i class="agc-rule is-domain" />能力域归属</span>
      <span class="agc-hint">节点大小按无向度数放大；点击节点看邻域与协作者</span>
    </div>
  </div>
</template>

<script setup>
// 图谱画布：只做"坐标 → SVG"的呈现与点击上报，不碰接口、不算布局（布局在 model/layout.js）。
import { computed } from 'vue'
import { GRAPH_EDGE_TYPE, GRAPH_NODE_TYPE } from '@/modules/expert-alliance/contract'
import { GRAPH_VIEWPORT } from '@/modules/expert-alliance/model'

const props = defineProps({
  layout: { type: Object, required: true },
  selectedId: { type: String, default: '' },
  viewport: { type: Object, default: () => ({ ...GRAPH_VIEWPORT }) }
})
const emit = defineEmits(['select'])

const viewport = computed(() => props.viewport || { ...GRAPH_VIEWPORT })
const nodes = computed(() => props.layout.nodes || [])
const nodeCount = computed(() => nodes.value.length)
const positions = computed(() => props.layout.positions || {})
const edgeCount = computed(() => (props.layout.edges || []).length)

const nodesById = computed(() => {
  const map = {}
  for (const n of nodes.value) map[n.id] = n
  return map
})

const expertCount = computed(() => nodes.value.filter((n) => n.nodeType === GRAPH_NODE_TYPE.expert).length)
const domainCount = computed(() => nodes.value.filter((n) => n.nodeType === GRAPH_NODE_TYPE.domain).length)

// 端点缺失的边不画：图可能刚 rebuild 过而某侧节点尚未同步，画出 NaN 会让整幅 SVG 失真
const drawableEdges = computed(() => {
  const out = []
  for (const e of props.layout.edges || []) {
    const a = positions.value[e.source]
    const b = positions.value[e.target]
    if (!a || !b) continue
    out.push({
      x1: a.x, y1: a.y, x2: b.x, y2: b.y,
      edgeType: e.edgeType,
      width: e.edgeType === GRAPH_EDGE_TYPE.has_domain ? 1 : Math.max(1.2, Math.min(4, (Number(e.weight) || 0) * 4))
    })
  }
  return out
})

function nodeTypeLabel(n) {
  return n.nodeType === GRAPH_NODE_TYPE.domain ? '能力域' : '专家'
}

function clipped(label) {
  const s = String(label ?? '')
  return s.length > 10 ? `${s.slice(0, 9)}…` : s
}
</script>

<style scoped>
.agc { display: flex; flex-direction: column; gap: 8px; }
.agc-svg {
  width: 100%;
  height: auto;
  aspect-ratio: 720 / 520;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}
.agc-line { stroke: var(--border); stroke-opacity: .85; }
.agc-line.is-collab { stroke: var(--accent-dim); }
.agc-line.is-domain { stroke: var(--border); stroke-dasharray: 3 3; }
.agc-node { cursor: pointer; }
.agc-node:focus-visible circle { outline: 2px solid var(--accent); }
.agc-dot.is-expert { fill: var(--accent); }
.agc-dot.is-domain { fill: var(--warning); }
.agc-node.is-selected .agc-dot { stroke: var(--accent-light); stroke-width: 3; }
.agc-label { font-size: 10px; fill: var(--text-secondary); pointer-events: none; }
.agc-label.is-domain { fill: var(--text-primary); font-weight: 600; }
.agc-legend { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; font-size: 12px; color: var(--text-secondary); }
.agc-key { display: inline-flex; align-items: center; gap: 5px; }
.agc-swatch { width: 10px; height: 10px; border-radius: var(--radius); display: inline-block; }
.agc-swatch.is-expert { background: var(--accent-fill); color: var(--on-accent); }
.agc-swatch.is-domain { background: var(--warning); }
.agc-rule { width: 16px; height: 2px; display: inline-block; background: var(--accent-dim); }
.agc-rule.is-domain { background: var(--border); }
.agc-hint { color: var(--text-muted); }
</style>
