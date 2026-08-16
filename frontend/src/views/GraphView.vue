<template>
  <div class="gv">
    <div class="head">
      <div>
        <h2 class="page-title">知识图谱</h2>
        <p class="page-subtitle">算子关系网络可视化 · 中心性 / 社区发现 / 最短路径分析</p>
      </div>
      <div class="head-actions">
        <el-input
          v-model="searchQ"
          placeholder="搜索对话/图谱节点"
          clearable
          style="width: 240px"
          @keyup.enter="doSearch"
          @clear="clearSearch"
        >
          <template #append>
            <el-button @click="doSearch"><el-icon><Search /></el-icon></el-button>
          </template>
        </el-input>
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
        <div class="panel card-pad" v-if="searchResult">
          <h3 class="section-title">
            搜索结果
            <el-button text size="small" @click="clearSearch">清空</el-button>
          </h3>
          <div v-if="searchResult.dialogues.length" class="nb-list">
            <div class="nb" v-for="(d, i) in searchResult.dialogues" :key="'d'+i">
              <span class="comm-tag">对话</span> {{ d.snippet }}
            </div>
          </div>
          <div v-if="searchResult.graph_nodes.length" class="nb-list">
            <div class="nb" v-for="(n, i) in searchResult.graph_nodes" :key="'n'+i">
              <span class="comm-tag">节点</span> {{ n.title }}
              <span class="muted">{{ n.snippet }}</span>
            </div>
          </div>
          <el-empty v-if="!searchResult.dialogues.length && !searchResult.graph_nodes.length"
                    description="无匹配" :image-size="50" />
        </div>
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
                  <div class="muted">权重：{{ pathResult.total_weight?.toFixed(3) }} · 跳数：{{ pathResult.length }}</div>
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

            <el-tab-pane label="中心性" name="cent">
              <el-radio-group v-model="centType" size="small" style="margin-bottom: 10px">
                <el-radio-button value="pagerank">PageRank</el-radio-button>
                <el-radio-button value="degree">度中心性</el-radio-button>
                <el-radio-button value="betweenness">中介中心性</el-radio-button>
              </el-radio-group>
              <el-button :loading="loadingCent" @click="loadCentrality" size="small" style="width: 100%; margin-bottom: 10px">
                计算中心性
              </el-button>
              <div v-if="centrality.length" class="nb-list">
                <div v-for="(c, i) in centrality.slice(0, 15)" :key="i" class="nb">
                  <span class="rank">#{{ i + 1 }}</span>
                  {{ c.id }} <span class="muted">{{ c.value.toFixed(4) }}</span>
                </div>
              </div>
            </el-tab-pane>

            <el-tab-pane label="社区发现" name="comm">
              <el-button :loading="loadingComm" @click="loadCommunities" size="small" style="width: 100%; margin-bottom: 10px">
                检测社区
              </el-button>
              <div v-if="communities.length" class="nb-list">
                <div v-for="(c, i) in communities" :key="i" class="nb comm">
                  <span class="comm-tag">社区{{ c.id }}</span>
                  <span class="muted">{{ c.nodes.length }} 节点 · 密度 {{ c.density.toFixed(3) }}</span>
                  <div class="comm-nodes">{{ c.nodes.slice(0, 8).join('、') }}{{ c.nodes.length > 8 ? '…' : '' }}</div>
                </div>
              </div>
            </el-tab-pane>

            <el-tab-pane label="激活传播" name="act">
              <el-select v-model="actSeeds" multiple filterable placeholder="选择种子节点（可多选）" style="width: 100%; margin-bottom: 10px">
                <el-option v-for="n in nodeIds" :key="n" :label="n" :value="n" />
              </el-select>
              <div class="act-opt">
                <span class="muted">迭代轮数</span>
                <el-input-number v-model="actIter" :min="1" :max="50" size="small" style="width: 100px" />
              </div>
              <el-button type="primary" :loading="loadingAct" @click="doPropagate" size="small" style="width: 100%; margin-bottom: 10px">
                开始传播
              </el-button>
              <div v-if="activation.length" class="nb-list">
                <div v-for="(a, i) in activation.slice(0, 20)" :key="i" class="nb">
                  <span class="rank">#{{ i + 1 }}</span>
                  {{ a.id }} <span class="muted">激活值 {{ a.value.toFixed(4) }}</span>
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
  recommendNodes,
  graphSearch,
  getCentrality,
  getCommunities,
  getPagerank,
  propagateActivation
} from '@/api'

const graphEl = ref(null)
const stats = ref(null)
const nodeIds = ref([])
let fg = null

// 统一搜索（对话 + 图谱节点）
const searchQ = ref('')
const searchResult = ref(null)
async function doSearch() {
  const q = searchQ.value.trim()
  if (!q) return
  try {
    const res = await graphSearch(q, 30)
    searchResult.value = res
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
      centrality.value = Object.entries(map)
        .map(([id, value]) => ({ id, value }))
        .sort((a, b) => b.value - a.value)
    } else {
      const metrics = await getCentrality()
      const src = centType.value === 'degree'
        ? metrics.degree_centrality
        : metrics.betweenness_centrality
      centrality.value = Object.entries(src || {})
        .map(([id, value]) => ({ id, value }))
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
    communities.value = await getCommunities()
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
    activation.value = Object.entries(map)
      .map(([id, value]) => ({ id, value }))
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
