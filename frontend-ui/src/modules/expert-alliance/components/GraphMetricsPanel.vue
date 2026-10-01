<template>
  <section class="agm">
    <h2 class="agm-title">图谱指标</h2>
    <el-alert v-if="store.error.metrics" type="error" show-icon :closable="false" title="统计获取失败"
      :description="store.error.metrics" />
    <p v-else-if="store.loading.metrics" class="agm-dim">统计加载中…</p>
    <dl v-else-if="metrics" class="agm-grid">
      <div v-for="cell in cells" :key="cell.label" class="agm-cell">
        <dt class="agm-cell-label">{{ cell.label }}</dt>
        <dd class="agm-cell-value">{{ cell.value }}</dd>
        <dd class="agm-dim">{{ cell.hint }}</dd>
      </div>
    </dl>

    <div class="agm-block">
      <h3 class="agm-subtitle">中心性前 {{ centrality.length }}</h3>
      <p class="agm-dim">degree_centrality = 度数 / (节点数 − 1)；betweenness 为 Brandes 介数（无向图已除以 2）</p>
      <ul v-if="centrality.length" class="agm-rank">
        <li v-for="row in centrality" :key="row.id" class="agm-rank-item">
          <button class="agm-jump" type="button" @click="store.selectNode(row.id)">{{ row.name || row.id }}</button>
          <span>度 {{ row.degree }}</span>
          <span>中心性 {{ row.degreeCentrality.toFixed(3) }}</span>
          <span>介数 {{ row.betweenness.toFixed(1) }}</span>
        </li>
      </ul>
      <p v-else class="agm-dim">图里还没有专家节点。</p>
    </div>

    <div class="agm-block">
      <h3 class="agm-subtitle">
        协作社区
        <small v-if="communities" class="agm-dim">{{ communities.algorithm }} · {{ communities.iterations }} 轮{{ communities.converged ? '已收敛' : '未收敛' }}</small>
      </h3>
      <el-alert v-if="store.error.communities" type="error" show-icon :closable="false" title="社区划分获取失败"
        :description="store.error.communities" />
      <p v-else-if="store.loading.communities" class="agm-dim">标签传播计算中…</p>
      <p v-else-if="!communities" class="agm-dim">尚未加载。</p>
      <template v-else>
        <p class="agm-dim">
          模块度 Q = {{ communities.modularity.toFixed(3) }}，共 {{ communities.totalCommunities }} 簇。
          社区由标签传播自动划出，不是人工分组。
        </p>
        <ul class="agm-communities">
          <li v-for="c in communities.communities" :key="c.communityId" class="agm-community">
            <header class="agm-community-head">
              <span class="agm-community-id">{{ c.communityId }}</span>
              <span class="agm-dim">{{ c.size }} 名 · 内边 {{ c.internalEdges }} · 外边 {{ c.externalEdges }}</span>
            </header>
            <p class="agm-community-members">{{ c.memberLabels.join('、') || '（成员标签均为空）' }}</p>
          </li>
        </ul>
      </template>
    </div>

    <div class="agm-block">
      <h3 class="agm-subtitle">最短路径</h3>
      <p class="agm-dim">BFS 无权最短路；不可达时后端仍返回 200 与 found=false，不会报错。</p>
      <div class="agm-path">
        <el-select v-model="pathSource" filterable placeholder="起点专家" size="small" class="agm-select"
          @change="(v) => store.setPathDraft('source', v)">
          <el-option v-for="o in expertOptions" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <span class="agm-dim">→</span>
        <el-select v-model="pathTarget" filterable placeholder="终点专家" size="small" class="agm-select"
          @change="(v) => store.setPathDraft('target', v)">
          <el-option v-for="o in expertOptions" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-button size="small" :loading="store.loading.path" :disabled="!canQueryPath" @click="store.findPath()">查询</el-button>
      </div>
      <el-alert v-if="store.error.path" type="error" show-icon :closable="false" title="路径查询失败" :description="store.error.path" />
      <template v-else-if="path">
        <p v-if="!path.found" class="agm-warn">两节点间不连通（{{ pathChainText(path.path, false) }}）。</p>
        <template v-else>
          <p class="agm-path-chain">{{ pathChainText(path.path, true) }}</p>
          <p class="agm-dim">跳数 {{ path.pathLength }} · 边权和 {{ path.totalWeight.toFixed(3) }}（同一对节点必得同一路径，BFS 前驱表按邻接序回溯）</p>
        </template>
      </template>
    </div>
  </section>
</template>

<script setup>
// 指标面板：统计 + 社区 + 路径查询。三份接口各自独立加载，任一失败不影响其余呈现。
import { computed, ref } from 'vue'
import { pathChainText } from '@/modules/expert-alliance/model'

const props = defineProps({ store: { type: Object, required: true } })
const store = props.store

const metrics = computed(() => store.metrics)
const communities = computed(() => store.communities)
const centrality = computed(() => metrics.value?.topCentralityExperts || [])
const expertOptions = computed(() => store.expertOptions)
const path = computed(() => store.path)

const pathSource = ref(store.pathDraft.source)
const pathTarget = ref(store.pathDraft.target)
const canQueryPath = computed(() => !!(pathSource.value && pathTarget.value))

const pct = (v) => `${(Number(v || 0) * 100).toFixed(1)}%`
const cells = computed(() => {
  const m = metrics.value
  if (!m) return []
  return [
    { label: '节点', value: `${m.totalNodes}`, hint: `专家 ${m.expertNodes} · 能力域 ${m.domainNodes}` },
    { label: '边', value: `${m.totalEdges}`, hint: `协作 ${m.collaborationEdges} · 归属 ${m.domainEdges}` },
    { label: '密度', value: pct(m.density), hint: '实际边数 / 完全图边数' },
    { label: '平均聚类系数', value: m.avgClusteringCoefficient.toFixed(3), hint: '邻居互连程度，越高越抱团' },
    {
      label: '连通分量',
      value: `${m.connectedComponents}`,
      hint: `最大簇 ${m.largestComponentSize} 名 —— 分量 >1 说明联盟被切成孤岛`
    }
  ]
})
</script>

<style scoped>
.agm { display: flex; flex-direction: column; gap: 10px; }
.agm-title { margin: 0; font-size: 15px; color: var(--text-primary); }
.agm-subtitle { margin: 0; font-size: 13px; color: var(--text-primary); }
.agm-dim { font-size: 12px; color: var(--text-muted); }
.agm-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 8px; margin: 0; }
.agm-cell { padding: 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); }
.agm-cell-label { margin: 0; font-size: 12px; color: var(--text-muted); }
.agm-cell-value { margin: 2px 0; font-size: 18px; color: var(--text-primary); }
.agm-cell dd { font-size: 12px; }
.agm-block { display: flex; flex-direction: column; gap: 6px; padding-top: 8px; border-top: 1px solid var(--border-light); }
.agm-rank, .agm-communities { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.agm-rank-item { display: flex; flex-wrap: wrap; gap: 8px; font-size: 12px; color: var(--text-secondary); }
.agm-jump { background: none; border: 0; padding: 0; font: inherit; color: var(--accent-light); cursor: pointer; }
.agm-jump:hover { text-decoration: underline; }
.agm-community { padding: 6px 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); }
.agm-community-head { display: flex; flex-wrap: wrap; align-items: baseline; gap: 8px; font-size: 12px; }
.agm-community-id { color: var(--text-primary); font-weight: 600; }
.agm-community-members { margin: 2px 0 0; font-size: 12px; color: var(--text-secondary); }
.agm-path { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
.agm-select { width: 180px; }
.agm-warn { margin: 0; font-size: 12px; color: var(--warning); }
.agm-path-chain { margin: 0; font-size: 13px; color: var(--text-primary); word-break: break-word; }
</style>
