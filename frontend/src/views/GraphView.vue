<template>
  <div class="gv">
    <div class="head">
      <div>
        <h2 class="page-title">知识图谱</h2>
        <p class="page-subtitle">算子关系网络可视化 · 中心性 / 社区发现 / 最短路径分析</p>
      </div>
      <div class="head-actions">
        <el-button @click="reload"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <div class="grid grid-4 stat-row" v-if="stats">
      <div class="stat panel" v-for="s in statCards" :key="s.label">
        <div class="stat-value">{{ s.value }}</div>
        <div class="stat-label">{{ s.label }}</div>
      </div>
    </div>

    <div class="grid graph-grid">
      <div class="panel graph-box">
        <h3 class="section-title">关系网络</h3>
        <div ref="graphEl" class="graph-canvas"></div>
        <div class="legend">
          <span v-for="(c, t) in NODE_TYPE_COLORS" :key="t" class="lg">
            <i :style="{ background: c }"></i>{{ t }}
          </span>
        </div>
      </div>

      <div class="side">
        <div class="panel card-pad">
          <h3 class="section-title">图谱分析</h3>
          <el-tabs v-model="tab">
            <el-tab-pane label="最短路径" name="path">
              <el-form label-width="56px">
                <el-form-item label="起点">
                  <el-select v-model="pathSrc" filterable placeholder="选择节点" style="width: 100%">
                    <el-option v-for="n in nodeIds" :key="n" :label="n" :value="n" />
                  </el-select>
                </el-form-item>
                <el-form-item label="终点">
                  <el-select v-model="pathDst" filterable placeholder="选择节点" style="width: 100%">
                    <el-option v-for="n in nodeIds" :key="n" :label="n" :value="n" />
                  </el-select>
                </el-form-item>
                <el-button type="primary" :loading="loadingPath" @click="findPath" style="width: 100%">
                  计算路径
                </el-button>
              </el-form>
              <div v-if="pathResult" class="path-result">
                <template v-if="pathResult.path?.length">
                  {{ pathResult.path.join(' → ') }}
                  <div class="muted">距离：{{ pathResult.distance }}</div>
                </template>
                <el-empty v-else description="无可达路径" :image-size="50" />
              </div>
            </el-tab-pane>

            <el-tab-pane label="节点邻居" name="nb">
              <el-select v-model="nbId" filterable placeholder="选择节点" style="width: 100%; margin-bottom: 10px">
                <el-option v-for="n in nodeIds" :key="n" :label="n" :value="n" />
              </el-select>
              <el-button :loading="loadingNb" @click="findNb" style="width: 100%">查询邻居</el-button>
              <div v-if="neighbors.length" class="nb-list">
                <div v-for="(nb, i) in neighbors" :key="i" class="nb">
                  {{ nb[0] }} <span class="muted">权重 {{ nb[1]?.toFixed?.(2) }}</span>
                </div>
              </div>
            </el-tab-pane>

            <el-tab-pane label="推荐" name="rec">
              <el-select v-model="recCtx" multiple filterable placeholder="选择上下文节点" style="width: 100%; margin-bottom: 10px">
                <el-option v-for="n in nodeIds" :key="n" :label="n" :value="n" />
              </el-select>
              <el-button type="primary" :loading="loadingRec" @click="findRec" style="width: 100%">
                智能推荐
              </el-button>
              <div v-if="recs.length" class="nb-list">
                <div v-for="(r, i) in recs" :key="i" class="nb">
                  {{ r.node_id || r.id }} <span class="muted">评分 {{ (r.score ?? 0).toFixed(3) }}</span>
                </div>
              </div>
            </el-tab-pane>
          </el-tabs>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount, computed, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import ForceGraph3D from '3d-force-graph'
import { NODE_TYPE_COLORS } from '@/types'
import {
  getGraph,
  getGraphStats,
  getShortestPath,
  getNeighbors,
  recommendNodes
} from '@/api'

const graphEl = ref(null)
const stats = ref(null)
const nodeIds = ref([])
let fg = null

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
  try {
    const [g, st] = await Promise.all([getGraph(), getGraphStats()])
    stats.value = st
    nodeIds.value = g.nodes.map((n) => n.id)
    if (fg) {
      fg.graphData({ nodes: g.nodes, links: g.edges })
    } else {
      await nextTick()
      initGraph(g)
    }
  } catch (e) {
    ElMessage.error('图谱加载失败：' + e.message)
  }
}

function initGraph(g) {
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
.muted {
  color: var(--text-3);
  font-size: 12px;
}
</style>
