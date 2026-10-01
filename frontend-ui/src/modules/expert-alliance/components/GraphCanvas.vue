<template>
  <div class="agc">
    <svg ref="svgEl" class="agc-svg" :viewBox="`0 0 ${viewport.width} ${viewport.height}`" role="img"
      :aria-label="`协作图谱，${nodeCount} 个节点 ${edgeCount} 条边`">
      <g class="agc-edges">
        <line v-for="(e, i) in drawableEdges" :key="`e${i}`"
          :class="['agc-line', e.edgeType === GRAPH_EDGE_TYPE.has_domain ? 'is-domain' : 'is-collab']"
          :x1="e.x1" :y1="e.y1" :x2="e.x2" :y2="e.y2" :stroke-width="e.width" />
      </g>
      <g class="agc-nodes">
        <g v-for="n in nodes" :key="n.id" class="agc-node"
          :class="{ 'is-selected': n.id === selectedId, 'is-link-source': n.id === linkSourceId }"
          tabindex="0" role="button" :aria-label="`${nodeTypeLabel(n)} ${graphNodeLabel(n)}`"
          @click="onNodeClick(n.id)" @keydown.enter.prevent="onNodeClick(n.id)"
          @pointerdown="onNodePointerDown($event, n)">
          <title>{{ graphNodeLabel(n) }}</title>
          <circle class="agc-dot" :class="nodeDotClass(n)"
            :cx="n.x" :cy="n.y" :r="n.radius" />
          <text class="agc-label" :class="{ 'is-domain': n.nodeType === GRAPH_NODE_TYPE.domain, 'is-lost': isLostGraphLabel(n.label) }" :x="n.x" :y="n.y + n.radius + 11"
            text-anchor="middle">{{ clipped(graphNodeLabel(n)) }}</text>
        </g>
      </g>
    </svg>

    <div class="agc-legend">
      <span class="agc-key"><i class="agc-swatch is-expert" />专家（{{ expertCount }}）</span>
      <span class="agc-key"><i class="agc-swatch is-domain" />能力域（{{ domainCount }}）</span>
      <span v-if="capabilityCount" class="agc-key"><i class="agc-swatch is-capability" />能力点（{{ capabilityCount }}）</span>
      <span class="agc-key"><i class="agc-rule is-collab" />协作（相似度 &gt;0.1）</span>
      <span class="agc-key"><i class="agc-rule is-domain" />能力域归属</span>
      <span class="agc-hint">{{ editHint }}</span>
      <span v-if="lostLabelCount" class="agc-lost">{{ lostLabelCount }} 个节点名称在写入后端时已编码丢失，图上改标 id 短码</span>
    </div>
  </div>
</template>

<script setup>
// 图谱画布：只做"坐标 → SVG"的呈现与意图上抛，不碰接口、不算布局（布局在 model/layout.js）。
// U1 增量：编辑模式下节点可拖拽（坐标经 drag 事件上交，本组件不落库），并高亮连线起点。
import { computed, ref } from 'vue'
import { GRAPH_EDGE_TYPE, GRAPH_NODE_TYPE, graphNodeLabel, isLostGraphLabel } from '@/modules/expert-alliance/contract'
import { GRAPH_VIEWPORT } from '@/modules/expert-alliance/model'

const props = defineProps({
  layout: { type: Object, required: true },
  selectedId: { type: String, default: '' },
  viewport: { type: Object, default: () => ({ ...GRAPH_VIEWPORT }) },
  editMode: { type: Boolean, default: false },
  linkSourceId: { type: String, default: '' }
})
// select：普通点选；drag：拖拽落位（id + SVG 坐标），由 store 记视觉覆盖
const emit = defineEmits(['select', 'drag'])

const viewport = computed(() => props.viewport || { ...GRAPH_VIEWPORT })
const nodes = computed(() => props.layout.nodes || [])
const nodeCount = computed(() => nodes.value.length)
const positions = computed(() => props.layout.positions || {})
const edgeCount = computed(() => (props.layout.edges || []).length)

const svgEl = ref(null)

const expertCount = computed(() => nodes.value.filter((n) => n.nodeType === GRAPH_NODE_TYPE.expert).length)
const domainCount = computed(() => nodes.value.filter((n) => n.nodeType === GRAPH_NODE_TYPE.domain).length)
const capabilityCount = computed(() => nodes.value.filter((n) => n.nodeType === 'capability').length)
const lostLabelCount = computed(() => nodes.value.filter((n) => isLostGraphLabel(n.label)).length)

const editHint = computed(() =>
  props.editMode ? '编辑模式：拖节点挪位置（不入库），点两个节点即连线' : '节点大小按无向度数放大；点击节点看邻域与协作者'
)

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

// capability 是手动 CRUD 才会出现的节点（后端 VALID_NODE_TYPES 含它），派生图里不会有；
// 给它第三色兜底，但不进 GRAPH_NODE_TYPE（那个被 builder 双向守卫钉死）。
function nodeDotClass(n) {
  if (n.nodeType === GRAPH_NODE_TYPE.domain) return 'is-domain'
  if (n.nodeType === 'capability') return 'is-capability'
  return 'is-expert'
}

function nodeTypeLabel(n) {
  if (n.nodeType === GRAPH_NODE_TYPE.domain) return '能力域'
  if (n.nodeType === 'capability') return '能力点'
  return '专家'
}

// ── 拖拽（原生 pointer events，不引第三方库）──────────────────────
// 拖过之后的 click 要吞掉，否则松手瞬间又触发一次 select。
const drag = ref({ active: false, id: '', moved: false, startClientX: 0, startClientY: 0 })
let suppressNextClick = false

function onNodeClick(id) {
  if (suppressNextClick) {
    suppressNextClick = false
    return
  }
  emit('select', id)
}

/** client → SVG viewBox 坐标：用 getScreenCTM 容忍画布被 CSS 缩放；jsdom 无此 API 时按比例兜底 */
function toSvgPoint(clientX, clientY) {
  const svg = svgEl.value
  if (!svg) return { x: 0, y: 0 }
  if (typeof svg.createSVGPoint === 'function' && typeof svg.getScreenCTM === 'function') {
    const ctm = svg.getScreenCTM()
    if (ctm) {
      const pt = svg.createSVGPoint()
      pt.x = clientX
      pt.y = clientY
      const mapped = pt.matrixTransform(ctm.inverse())
      return { x: mapped.x, y: mapped.y }
    }
  }
  const rect = svg.getBoundingClientRect()
  if (!rect.width || !rect.height) return { x: 0, y: 0 }
  return {
    x: ((clientX - rect.left) / rect.width) * viewport.value.width,
    y: ((clientY - rect.top) / rect.height) * viewport.value.height
  }
}

function onNodePointerDown(e, n) {
  if (!props.editMode) return
  // 左键才拖；右键/中键不拦，保持浏览器默认
  if (e.button !== 0) return
  drag.value = { active: true, id: n.id, moved: false, startClientX: e.clientX, startClientY: e.clientY }
  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp, { once: true })
}

function onPointerMove(e) {
  if (!drag.value.active) return
  const dx = e.clientX - drag.value.startClientX
  const dy = e.clientY - drag.value.startClientY
  if (!drag.value.moved && Math.hypot(dx, dy) < 3) return
  drag.value.moved = true
  const { x, y } = toSvgPoint(e.clientX, e.clientY)
  // 夹进画布内，避免拖丢到 viewBox 外看不见
  const clamped = {
    x: Math.max(8, Math.min(viewport.value.width - 8, x)),
    y: Math.max(8, Math.min(viewport.value.height - 8, y))
  }
  emit('drag', drag.value.id, clamped.x, clamped.y)
}

function onPointerUp() {
  if (drag.value.moved) suppressNextClick = true
  drag.value.active = false
  window.removeEventListener('pointermove', onPointerMove)
}

/** 按码点裁剪：String#slice 会把代理对切成一半，留下一个读不出的孤立字符 */
function clipped(label) {
  const chars = Array.from(String(label ?? ''))
  return chars.length > 10 ? `${chars.slice(0, 9).join('')}…` : chars.join('')
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
  touch-action: none;
}
.agc-line { stroke: var(--border); stroke-opacity: .85; }
.agc-line.is-collab { stroke: var(--accent-dim); }
.agc-line.is-domain { stroke: var(--border); stroke-dasharray: 3 3; }
.agc-node { cursor: pointer; }
.agc-node:focus-visible circle { outline: 2px solid var(--accent); }
.agc-dot.is-expert { fill: var(--accent); }
.agc-dot.is-domain { fill: var(--warning); }
.agc-dot.is-capability { fill: var(--success); }
.agc-node.is-selected .agc-dot { stroke: var(--accent-light); stroke-width: 3; }
.agc-node.is-link-source .agc-dot { stroke: var(--success); stroke-width: 3; stroke-dasharray: 2 2; }
.agc-label { font-size: 10px; fill: var(--text-secondary); pointer-events: none; }
.agc-label.is-domain { fill: var(--text-primary); font-weight: 600; }
.agc-legend { display: flex; flex-wrap: wrap; align-items: center; gap: 10px; font-size: 12px; color: var(--text-secondary); }
.agc-key { display: inline-flex; align-items: center; gap: 5px; }
.agc-swatch { width: 10px; height: 10px; border-radius: var(--radius); display: inline-block; }
.agc-swatch.is-expert { background: var(--accent-fill); color: var(--on-accent); }
.agc-swatch.is-domain { background: var(--warning); }
.agc-swatch.is-capability { background: var(--success); }
.agc-rule { width: 16px; height: 2px; display: inline-block; background: var(--accent-dim); }
.agc-rule.is-domain { background: var(--border); }
.agc-hint { color: var(--text-muted); }
.agc-lost { color: var(--warning); }
</style>
