<template>
  <div class="agv">
    <header class="agv-head">
      <div>
        <h1 class="agv-title">专家协作图谱</h1>
        <p class="agv-sub">
          由专家注册表派生的协作网络：{{ stats.nodeCount }} 节点 / {{ stats.edgeCount }} 边，
          版本 v{{ stats.version }}，构建于 {{ stats.builtAt || '未知' }}
        </p>
      </div>
      <div class="agv-actions">
        <el-tag size="small" type="info" effect="plain">内存态派生图</el-tag>
        <el-button :icon="Refresh" :loading="store.loading.graph" @click="reload">刷新</el-button>
        <el-button v-role-any="['super_admin', 'tenant_admin']" :icon="MagicStick" :loading="store.loading.rebuild" @click="rebuild">重建图谱</el-button>
        <el-button v-role-any="['super_admin', 'tenant_admin']" :type="store.editMode ? 'primary' : 'default'" @click="toggleEdit">
          {{ store.editMode ? '退出编辑' : '编辑模式' }}
        </el-button>
        <el-button v-role-any="['super_admin', 'tenant_admin']" :icon="Plus" @click="openCreateNode">新增节点</el-button>
      </div>
    </header>

    <el-alert v-if="store.error.graph" class="agv-alert" type="error" show-icon :closable="false"
      title="图谱获取失败" :description="`${store.error.graph}；统计、社区与团队面板仍单独重试。`" />
    <el-alert v-if="store.error.rebuild" class="agv-alert" type="error" show-icon :closable="false"
      title="图谱重建失败" :description="store.error.rebuild" />
    <p v-else-if="rebuildNote" class="agv-note">{{ rebuildNote }}</p>

    <div class="agv-body">
      <div class="agv-main">
        <div class="agv-card">
          <GraphCanvas :layout="store.layout" :selected-id="store.selectedId" :edit-mode="store.editMode"
            :link-source-id="store.linkSourceId" @select="onCanvasSelect" @drag="store.setNodePosition" />
          <p v-if="!store.layout.nodes.length && !store.loading.graph" class="agv-dim">
            图为空：注册表里还没有专家，或图谱尚未构建。可先在专家广场注册专家，再点「重建图谱」。
          </p>
        </div>
        <div class="agv-card">
          <GraphTeamPanel :store="store" :skill-options="skillOptions" />
        </div>
      </div>

      <aside class="agv-side">
        <div class="agv-card">
          <GraphNodeInspector :store="store" />
        </div>
        <div class="agv-card agv-side-notes">
          <h2 class="agv-notes-title">这份图谱不告诉你什么</h2>
          <ul class="agv-notes">
            <li>节点属性只有头衔、能力域、评分、可用状态四项——<strong>没有技能清单</strong>，技能要从注册表接口取，组建面板的候选项正来自那里。</li>
            <li>协作边的权重是「共享能力域的 Jaccard 相似度」，<strong>不是真实合作次数</strong>；相似度 ≤0.1 的专家对根本不建边。</li>
            <li>图是注册表的投影：注册了新专家不会自动进图，需要点「重建图谱」（版本号 +1 并落盘）。</li>
            <li>
              <code>constraints</code> 字段后端收了但从不读取，所以表单里没有它——
              填了也不影响结果，与其摆个假控件，不如不摆。
            </li>
            <li>路径查询不可达时后端返回 200 + <code>found:false</code>，界面按「不连通」呈现而不是报错。</li>
          </ul>
        </div>
      </aside>
    </div>

    <div class="agv-card agv-metrics">
      <GraphMetricsPanel :store="store" />
    </div>

    <el-dialog v-model="createVisible" title="新增节点" width="420px" append-to-body>
      <el-form label-width="64px" size="small">
        <el-form-item label="ID">
          <el-input v-model="store.nodeDraft.id" placeholder="如 exp-new-1 / capability-ocr" />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="store.nodeDraft.label" placeholder="节点显示名" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="store.nodeDraft.nodeType" style="width: 100%">
            <el-option v-for="t in WRITE_TYPES" :key="t.value" :label="t.label" :value="t.value" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="createVisible = false">取消</el-button>
        <el-button size="small" type="primary" :loading="store.loading.graph" @click="submitCreate">创建</el-button>
      </template>
    </el-dialog>

    <el-dialog :model-value="!!store.pendingEdge.source" title="选择边类型" width="380px" append-to-body @update:model-value="onEdgeDialogClose">
      <p class="agv-dim">
        连接 <code>{{ store.pendingEdge.source }}</code> → <code>{{ store.pendingEdge.target }}</code>
      </p>
      <el-radio-group v-model="pendingEdgeType" class="agv-edge-types">
        <el-radio v-for="t in EDGE_TYPES" :key="t.value" :value="t.value">
          {{ t.label }}<span class="agv-dim">（{{ t.weightHint }}）</span>
        </el-radio>
      </el-radio-group>
      <template #footer>
        <el-button size="small" @click="onEdgeDialogClose(false)">取消</el-button>
        <el-button size="small" type="primary" :loading="store.loading.graph" @click="submitEdge">连线</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
// 协作图谱页面：只提供外壳与三个子面板的装配，接口调用与状态都在 alliance-graph.store。
import { computed, onMounted, ref } from 'vue'
import { MagicStick, Plus, Refresh } from '@element-plus/icons-vue'
import { GRAPH_EDGE_TYPES, GRAPH_WRITE_NODE_TYPES } from '@/modules/expert-alliance/contract'
import { useAllianceGraphStore } from '@/modules/expert-alliance/store'
import { useAllianceExpertsStore } from '@/modules/expert-alliance/store'
import { GraphCanvas } from '@/modules/expert-alliance/components'
import { GraphNodeInspector } from '@/modules/expert-alliance/components'
import { GraphMetricsPanel } from '@/modules/expert-alliance/components'
import { GraphTeamPanel } from '@/modules/expert-alliance/components'

const store = useAllianceGraphStore()
const expertStore = useAllianceExpertsStore()

const WRITE_TYPES = GRAPH_WRITE_NODE_TYPES
const EDGE_TYPES = GRAPH_EDGE_TYPES
const createVisible = ref(false)
const pendingEdgeType = ref(EDGE_TYPES[0].value)

// 画布点选交给 store 分发：编辑模式下可能是"选连线起点/弹边类型"，普通模式才是选中查邻域
function onCanvasSelect(id) {
  const intent = store.canvasClickNode(id)
  if (intent === 'select') store.selectNode(id)
  else if (intent === 'pending-edge') pendingEdgeType.value = EDGE_TYPES[0].value
}

function toggleEdit() {
  store.setEditMode(!store.editMode)
}

function openCreateNode() {
  store.resetNodeDraft()
  createVisible.value = true
}

async function submitCreate() {
  const { id, label, nodeType } = store.nodeDraft
  if (!id || !label) return
  const res = await store.createGraphNode({ id, label, node_type: nodeType })
  if (res) createVisible.value = false
}

function onEdgeDialogClose(clear = true) {
  if (clear) store.cancelLink()
}

async function submitEdge() {
  const { source, target } = store.pendingEdge
  if (!source || !target) return
  await store.createGraphEdge({ source, target, edge_type: pendingEdgeType.value })
  store.cancelLink()
}

const stats = computed(() => store.graph?.stats || { nodeCount: 0, edgeCount: 0, version: 0 })
const rebuildNote = computed(() => {
  const r = store.rebuildResult
  if (!r) return ''
  return `已重建：v${r.previousVersion} → v${r.newVersion}，${r.nodeCount} 节点 / ${r.edgeCount} 边（${r.durationMs} ms）`
})

// 图谱节点不带 skills，组建面板的技能候选只能取注册表；名单为空时候选框仍可自由输入
const skillOptions = computed(() => {
  const seen = new Set()
  for (const e of expertStore.experts) for (const s of e.skills || []) seen.add(s)
  return [...seen].sort().map((s) => ({ value: s, label: s }))
})

function reload() {
  store.loadGraph()
  store.loadMetrics()
  store.loadCommunities()
}

function rebuild() {
  store.rebuild()
}

onMounted(() => {
  reload()
  if (!expertStore.experts.length) expertStore.loadExperts()
})
</script>

<style scoped>
.agv { display: flex; flex-direction: column; gap: 12px; }
.agv-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.agv-title { margin: 0; font-size: 20px; color: var(--text-primary); }
.agv-sub { margin: 4px 0 0; font-size: 12px; color: var(--text-secondary); }
.agv-actions { display: flex; align-items: center; gap: 8px; }
.agv-alert, .agv-note { margin: 0; }
.agv-note { font-size: 12px; color: var(--success); }
.agv-dim { margin: 0; font-size: 12px; color: var(--text-muted); }
.agv-body { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 12px; align-items: start; }
.agv-main { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
.agv-side { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
.agv-card {
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
}
.agv-metrics { display: block; }
.agv-side-notes { display: flex; flex-direction: column; gap: 6px; }
.agv-notes-title { margin: 0; font-size: 13px; color: var(--text-primary); }
.agv-notes { margin: 0; padding-left: 16px; display: flex; flex-direction: column; gap: 6px; font-size: 12px; line-height: 1.6; color: var(--text-secondary); }
.agv-notes code { font-size: 11px; color: var(--accent-light); }
.agv-edge-types { display: flex; flex-direction: column; gap: 8px; }
@media (max-width: 1180px) {
  .agv-body { grid-template-columns: minmax(0, 1fr); }
}
</style>
