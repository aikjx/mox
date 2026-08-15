<template>
  <div class="graph-wrap">
    <div class="graph-head">
      <div class="title">
        <el-icon><Share /></el-icon>
        <span>业务流程图 · 知识图谱</span>
      </div>
      <div class="legend">
        <span v-for="(c, t) in legendTypes" :key="t" class="lg">
          <i :style="{ background: c }"></i>{{ t }}
        </span>
      </div>
      <div class="head-actions">
        <el-button size="small" @click="reload" :loading="loading">
          <el-icon><Refresh /></el-icon>刷新
        </el-button>
        <el-switch v-model="showRelation" active-text="关系连线" inline-prompt />
      </div>
    </div>

    <div class="graph-body">
      <div ref="chart" class="chart"></div>

      <!-- 实时处理流程轨迹 -->
      <div class="flow-trace" v-if="trace.length">
        <div class="trace-title">
          <el-icon><Connection /></el-icon> 实时处理流程
        </div>
        <div class="trace-list">
          <div
            v-for="(n, i) in trace"
            :key="i"
            class="trace-node"
            :class="n.status"
          >
            <span class="step">{{ i + 1 }}</span>
            <span class="t-name">{{ n.name }}</span>
            <span class="t-type">{{ n.type }}</span>
            <span class="t-ms">{{ n.duration }}ms</span>
          </div>
        </div>
      </div>

      <div v-if="!loading && !hasData" class="placeholder">
        暂无图谱数据，请在左侧对话中触发“展示知识图谱”或询问算子。
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import * as echarts from 'echarts'
import { Share, Refresh, Connection } from '@element-plus/icons-vue'
import { getGraph } from '@/api'
import { NODE_TYPE_COLORS } from '@/types'

const chart = ref(null)
const loading = ref(false)
const hasData = ref(false)
const showRelation = ref(true)
const trace = ref([])
let inst = null

const legendTypes = NODE_TYPE_COLORS

async function reload() {
  loading.value = true
  try {
    const data = await getGraph()
    render(data)
    hasData.value = !!(data.nodes && data.nodes.length)
  } catch (e) {
    // 静默：可能尚未触发图谱
  } finally {
    loading.value = false
  }
}

function buildOption(data) {
  const nodes = (data.nodes || []).map((n) => ({
    id: n.id,
    name: n.label,
    value: n.size || 20,
    symbolSize: Math.max(14, Math.min(60, n.size || 20)),
    category: n.node_type,
    itemStyle: { color: n.color || NODE_TYPE_COLORS[n.node_type] || '#64748b' },
    _activation: n.activation,
    label: { show: (n.size || 20) > 30 },
  }))

  const links = (data.edges || []).map((e) => ({
    source: e.source,
    target: e.target,
    value: e.weight,
    lineStyle: {
      width: Math.max(0.6, Math.min(4, e.weight * 2)),
      opacity: 0.35,
      color: '#3a4a66',
      curveness: 0.12,
    },
  }))

  const categories = Object.keys(NODE_TYPE_COLORS).map((t) => ({ name: t }))

  return {
    backgroundColor: 'transparent',
    tooltip: {
      backgroundColor: '#0a0f1e',
      borderColor: '#243049',
      textStyle: { color: '#e6ecf5' },
      formatter: (p) => {
        if (p.dataType === 'node') {
          return `<b>${p.data.name}</b><br/>类型：${p.data.category}<br/>激活度：${(
            p.data._activation || 0
          ).toFixed(3)}<br/>PageRank：${(p.value || 0).toFixed(4)}`
        }
        return `${p.data.source} → ${p.data.target}<br/>权重：${p.data.value}`
      },
    },
    legend: { show: false },
    animationDuration: 800,
    series: [
      {
        type: 'graph',
        layout: 'force',
        roam: true,
        draggable: true,
        categories,
        data: nodes,
        links: showRelation.value ? links : [],
        label: { color: '#e6ecf5', fontSize: 12 },
        emphasis: {
          focus: 'adjacency',
          lineStyle: { width: 4, opacity: 0.8 },
        },
        force: {
          repulsion: 220,
          edgeLength: [60, 160],
          gravity: 0.08,
          friction: 0.18,
        },
        lineStyle: { color: 'source', curveness: 0.12 },
      },
    ],
  }
}

function render(data) {
  if (!inst) return
  inst.setOption(buildOption(data), true)
}

/** 外部调用：展示某次流程执行的实时轨迹 */
function showTrace(nodeResults) {
  trace.value = (nodeResults || []).map((n) => ({
    name: n.node_name || n.node_id,
    type: n.node_type,
    status: n.status || (n.error ? 'failed' : 'success'),
    duration: n.duration_ms || 0,
  }))
  // 高亮图谱中对应节点
  if (inst) {
    const opt = inst.getOption()
    const series = opt.series && opt.series[0]
    if (series && series.data) {
      const map = new Map(
        (nodeResults || []).map((n) => [n.node_id, n.status || (n.error ? 'failed' : 'success')])
      )
      series.data.forEach((d) => {
        if (map.has(d.id)) {
          d.itemStyle = {
            ...(d.itemStyle || {}),
            borderColor: map.get(d.id) === 'failed' ? '#ef4444' : '#10b981',
            borderWidth: 4,
            shadowBlur: 20,
            shadowColor: map.get(d.id) === 'failed' ? '#ef4444' : '#10b981',
          }
        }
      })
      inst.setOption({ series: [{ data: series.data }] }, false)
    }
  }
}

function resize() {
  inst && inst.resize()
}

onMounted(async () => {
  await nextTick()
  inst = echarts.init(chart.value, null, { renderer: 'canvas' })
  window.addEventListener('resize', resize)
  await reload()
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', resize)
  inst && inst.dispose()
})

watch(showRelation, () => reload())

defineExpose({ reload, showTrace })
</script>

<style scoped>
.graph-wrap {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-width: 0;
}
.graph-head {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  flex-wrap: wrap;
}
.title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
}
.legend {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.lg {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--text-dim);
}
.lg i {
  width: 9px;
  height: 9px;
  border-radius: 3px;
  display: inline-block;
}
.head-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 10px;
}
.graph-body {
  position: relative;
  flex: 1;
  min-height: 0;
}
.chart {
  width: 100%;
  height: 100%;
}
.flow-trace {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 240px;
  max-height: calc(100% - 24px);
  overflow-y: auto;
  padding: 10px 12px;
  background: rgba(18, 26, 46, 0.92);
  border: 1px solid var(--border);
  border-radius: 10px;
  backdrop-filter: blur(6px);
}
.trace-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--accent);
}
.trace-node {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 8px;
  margin-bottom: 6px;
  background: var(--bg-panel-2);
  border-left: 3px solid var(--text-dim);
  font-size: 12px;
}
.trace-node.success { border-left-color: var(--success); }
.trace-node.failed { border-left-color: var(--danger); }
.step {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--primary);
  color: #fff;
  font-size: 11px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.t-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.t-type { color: var(--text-dim); }
.t-ms { color: var(--text-dim); }
.placeholder {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-dim);
  font-size: 13px;
  pointer-events: none;
}
</style>
