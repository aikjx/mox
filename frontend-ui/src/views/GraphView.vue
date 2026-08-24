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
        <h3 class="section-title">
          关系网络
          <span class="muted stage-chip" v-if="loadStage !== 'physics'">{{ stageLabel }}</span>
          <span class="ok-chip" v-else>● 已就绪</span>
        </h3>
        <!-- 主画布：优先显示占位骨架（SVG）→ 3D 模块加载完再渲染 WebGL 画布 -->
        <div class="canvas-wrap">
          <!-- Stage A: 骨架占位（数据/模块未就绪时，SVG 轻量占位） -->
          <svg v-if="showSkeleton" class="skeleton-svg" viewBox="0 0 800 520" preserveAspectRatio="xMidYMid meet" aria-label="图谱骨架（占位）">
            <defs>
              <radialGradient id="gvGlow" cx="50%" cy="50%" r="50%">
                <stop offset="0%" stop-color="#6366f1" stop-opacity="0.25" />
                <stop offset="100%" stop-color="#0b1020" stop-opacity="0" />
              </radialGradient>
            </defs>
            <rect width="800" height="520" fill="#0b1020" rx="12" />
            <circle cx="400" cy="260" r="220" fill="url(#gvGlow)" />
            <!-- 占位边：20 条随机环形骨架（纯装饰） -->
            <g stroke="rgba(148,163,184,0.18)" stroke-width="1" fill="none">
              <line v-for="(_, i) in 20" :key="'sk-e'+i"
                :x1="400 + 140*Math.cos(i*Math.PI/10)" :y1="260 + 100*Math.sin(i*Math.PI/10)"
                :x2="400 + 240*Math.cos((i+3)*Math.PI/10)" :y2="260 + 180*Math.sin((i+3)*Math.PI/10)" />
            </g>
            <!-- 占位节点：12 个同心圆分布的彩色 dot（按 node type 调色） -->
            <g v-for="(n, i) in skelNodes" :key="'sk-n'+i">
              <circle :cx="400 + 180*Math.cos(i*Math.PI/6 + 0.2)" :cy="260 + 120*Math.sin(i*Math.PI/6 + 0.2)"
                :r="n.r" :fill="n.c" opacity="0.92" />
            </g>
            <text x="400" y="500" text-anchor="middle" fill="#94a3b8" font-size="13" letter-spacing="2">
              {{ stageLabel }} · {{ stageProgress }}%
            </text>
          </svg>
          <!-- Stage B: WebGL 画布（3D ForceGraph3D 实际挂载点） -->
          <div ref="graphEl" class="graph-canvas" :class="{ covered: showSkeleton }"></div>
          <!-- 阶段进度条 -->
          <div class="stage-bar-wrap" v-if="loadStage !== 'physics'">
            <div class="stage-bar">
              <div class="stage-bar-fill" :style="{ width: stageProgress + '%' }"></div>
            </div>
            <div class="stage-hints">
              <span :class="{ active: loadStage !== 'skeleton' }">① 取数</span>
              <span :class="{ active: ['module','render','physics'].includes(loadStage) }">② 3D 库</span>
              <span :class="{ active: ['render','physics'].includes(loadStage) }">③ 绘帧</span>
              <span :class="{ active: loadStage === 'physics' }">④ 力学</span>
            </div>
          </div>
        </div>
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
                  {{ c.id }} <span class="muted">{{ (c.value ?? 0).toFixed(4) }}</span>
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
                  <span class="muted">{{ c.nodes.length }} 节点</span>
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
                  {{ a.id }} <span class="muted">激活值 {{ (a.value ?? 0).toFixed(4) }}</span>
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
import { ref, onMounted, onBeforeUnmount, computed, nextTick, shallowRef, markRaw } from 'vue'
import { ElMessage } from 'element-plus'
// [P1-1 渐进加载 · 先画布后力学] 静态仅依赖轻量类型/API；3D 重库 ForceGraph3D 改为动态 import 后按需拆分异步 chunk（≈1.2MB 单独下载，不阻塞首帧）
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
// 用 shallowRef/markRaw 防止 Vue 递归代理 three.js/FG 对象（大对象深代理 = 严重卡顿 + 内存翻倍）
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
    // [P1-1 力学后置] 首帧先显示静态布局，260ms 后再启动力导向引擎，避免"白屏等力学收敛 2-3s"
    setTimeout(() => {
      if (!fg) return
      // 启用全部力学力（charge/collide/link/center），ForceGraph3D 默认引擎是 d3-force，这里显式 warm up
      if (typeof fg.d3Force === 'function') {
        const charge = fg.d3Force('charge')
        if (charge && typeof charge.strength === 'function') charge.strength(-120)
        const link = fg.d3Force('link')
        if (link && typeof link.distance === 'function') link.distance(42)
        const center = fg.d3Force('center')
        if (center && typeof center.strength === 'function') center.strength(0.15)
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
.muted {
  color: var(--text-3);
  font-size: 12px;
}
</style>
