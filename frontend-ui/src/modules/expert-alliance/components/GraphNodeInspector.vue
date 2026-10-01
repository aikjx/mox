<template>
  <section class="agn">
    <header class="agn-head">
      <h2 class="agn-title">{{ node ? node.label || node.id : '未选择节点' }}</h2>
      <el-tag v-if="node" size="small" :type="isDomain ? 'warning' : 'primary'" effect="plain">
        {{ isDomain ? '能力域' : '专家' }}
      </el-tag>
    </header>

    <p v-if="!node" class="agn-empty">在左侧图上点一个节点，这里给出它的邻域与协作者。</p>

    <el-alert v-else-if="store.error.node" type="error" show-icon :closable="false"
      :title="isDomain ? '邻域获取失败' : '邻域或协作者获取失败'" :description="store.error.node" />

    <template v-else>
      <dl class="agn-meta">
        <template v-if="!isDomain">
          <dt>头衔</dt>
          <dd>{{ node.title || '后端未填' }}</dd>
          <dt>评分</dt>
          <dd>{{ node.avgRating === null ? '后端未返回' : node.avgRating.toFixed(2) }}</dd>
          <dt>可用状态</dt>
          <dd>{{ availabilityLabel(node.availability) }}<span class="agn-dim">（{{ node.availability || '未知' }}）</span></dd>
        </template>
        <dt>度数</dt>
        <dd>{{ node.degree }}（本图内直接相连的边数）</dd>
        <dt>节点 ID</dt>
        <dd><code>{{ node.id }}</code></dd>
      </dl>

      <div class="agn-block">
        <h3 class="agn-block-title">
          邻域 {{ neighbors ? neighbors.neighborCount : 0 }}
          <small class="agn-dim">GET /api/expert-graph/neighbors/:id</small>
        </h3>
        <p v-if="store.loading.node" class="agn-dim">加载中…</p>
        <ul v-else-if="neighborRows.length" class="agn-list">
          <li v-for="row in neighborRows" :key="row.key" class="agn-item">
            <button class="agn-jump" type="button" @click="store.selectNode(row.id)">{{ row.label }}</button>
            <span class="agn-tag">{{ row.edgeLabel }}</span>
            <span class="agn-dim">{{ row.direction === 'out' ? '出边' : '入边' }} · 权重 {{ row.weight.toFixed(2) }}</span>
            <span v-if="row.sharedDomains.length" class="agn-dim">共享 {{ row.sharedDomains.join('、') }}</span>
          </li>
        </ul>
        <p v-else class="agn-dim">该节点没有任何相连的边（图由注册表现算，孤点说明它没有能力域记录）。</p>
      </div>

      <div v-if="!isDomain" class="agn-block">
        <h3 class="agn-block-title">
          协作者 {{ collaborators ? `${shown} / ${collaborators.totalCollaborators}` : 0 }}
          <small class="agn-dim">GET /api/expert-graph/collaborators/:id</small>
        </h3>
        <div class="agn-limit">
          <label class="agn-dim" for="agn-limit">返回条数</label>
          <el-select id="agn-limit" :model-value="store.collaboratorLimit" size="small" style="width: 96px"
            @change="(v) => store.changeCollaboratorLimit(v)">
            <el-option v-for="n in COLLABORATOR_LIMITS" :key="n" :label="String(n)" :value="n" />
          </el-select>
        </div>
        <p v-if="store.collaboratorsTruncated" class="agn-note">
          仅显示前 {{ shown }} 位（按协作权重降序），后端还有 {{ store.collaborators.totalCollaborators - shown }} 位被 limit 截断。
        </p>
        <ul v-if="collaboratorRows.length" class="agn-list">
          <li v-for="c in collaboratorRows" :key="c.id" class="agn-item">
            <span class="agn-rank">{{ c.rank }}</span>
            <button class="agn-jump" type="button" @click="store.selectNode(c.id)">{{ c.name || c.id }}</button>
            <span class="agn-dim">权重 {{ c.collaborationWeight.toFixed(3) }}</span>
            <span v-if="c.sharedDomains.length" class="agn-tag">{{ c.sharedDomains.join('、') }}</span>
          </li>
        </ul>
        <p v-else-if="!store.loading.node" class="agn-dim">
          没有协作边：该专家与其他专家无共享能力域（或相似度未超过 0.1 建边阈值）。
        </p>
      </div>
    </template>
  </section>
</template>

<script setup>
// 节点详情：只读 store 的 neighbors/collaborators，跳转仍走 store.selectNode。
// 注意 collaborators 与 neighbors 是两份不同口径——后者含能力域边，前者只算 collaborates_with。
import { computed } from 'vue'
import { COLLABORATOR_LIMITS, GRAPH_NODE_TYPE, edgeTypeMeta } from '@/modules/expert-alliance/contract'
import { availabilityLabel } from '@/modules/expert-alliance/contract'

const props = defineProps({ store: { type: Object, required: true } })
const store = props.store

const node = computed(() => store.selectedNode)
const isDomain = computed(() => node.value?.nodeType === GRAPH_NODE_TYPE.domain)
const neighbors = computed(() => store.neighbors)
const collaborators = computed(() => store.collaborators)

const neighborRows = computed(() =>
  (neighbors.value?.neighbors || []).map((n) => ({
    key: `${n.id}-${n.edgeType}-${n.direction}`,
    id: n.id,
    label: n.label || n.id,
    edgeLabel: edgeTypeMeta(n.edgeType).label,
    direction: n.direction,
    weight: Number(n.weight) || 0,
    sharedDomains: n.sharedDomains || []
  }))
)

const collaboratorRows = computed(() => collaborators.value?.collaborators || [])
const shown = computed(() => collaboratorRows.value.length)
</script>

<style scoped>
.agn { display: flex; flex-direction: column; gap: 10px; }
.agn-head { display: flex; align-items: center; gap: 8px; }
.agn-title { margin: 0; font-size: 15px; color: var(--text-primary); }
.agn-empty, .agn-dim { font-size: 12px; color: var(--text-muted); }
.agn-meta { display: grid; grid-template-columns: 72px minmax(0, 1fr); gap: 4px 8px; margin: 0; font-size: 12px; }
.agn-meta dt { color: var(--text-muted); }
.agn-meta dd { margin: 0; color: var(--text-primary); word-break: break-all; }
.agn-meta code { font-size: 11px; color: var(--accent-light); }
.agn-block { display: flex; flex-direction: column; gap: 6px; padding-top: 8px; border-top: 1px solid var(--border-light); }
.agn-block-title { margin: 0; font-size: 13px; color: var(--text-primary); display: flex; align-items: baseline; gap: 6px; }
.agn-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.agn-item { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; font-size: 12px; color: var(--text-secondary); }
.agn-jump { background: none; border: 0; padding: 0; font: inherit; color: var(--accent-light); cursor: pointer; text-align: left; }
.agn-jump:hover { text-decoration: underline; }
.agn-tag {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  background: var(--bg-hover);
  color: var(--text-secondary);
}
.agn-limit { display: flex; align-items: center; gap: 6px; }
.agn-rank {
  min-width: 18px;
  text-align: right;
  font-size: 11px;
  color: var(--text-muted);
}
.agn-note {
  margin: 0;
  padding: 4px 8px;
  font-size: 12px;
  color: var(--warning);
  background: var(--bg-hover);
  border-radius: var(--radius-sm);
}
</style>
