<template>
  <section class="ecm">
    <div class="ecm-head">
      <el-input v-model="keyword" class="ecm-search" size="small" clearable
        placeholder="按能力名 / 领域过滤目录（仅本页过滤，不发请求）" :prefix-icon="Search" />
      <span class="ecm-count">{{ countText }}</span>
    </div>

    <el-alert v-if="error" type="error" show-icon :closable="false" title="能力目录加载失败" :description="error" />
    <div v-else-if="loading" class="ecm-loading"><el-skeleton :rows="5" animated /></div>
    <el-empty v-else-if="!groups.length" :image-size="70" :description="emptyText" />

    <div v-else class="ecm-groups">
      <div v-for="g in groups" :key="g.domain" class="ecm-group" :class="{ 'is-active': g.domain === activeDomain }">
        <button class="ecm-group-btn" @click="emit('pick', g.domain)">
          <span class="ecm-group-name">{{ g.domain || '未标领域' }}</span>
          <small class="ecm-group-meta">{{ g.items.length }} 项能力 · {{ g.slots }} 人次</small>
        </button>
        <ul class="ecm-list">
          <li v-for="c in g.items" :key="c.id" class="ecm-item">
            <span class="ecm-item-name" :title="c.id">{{ c.name || c.id }}</span>
            <el-progress class="ecm-item-bar" :percentage="averageProficiency(c)" :stroke-width="6" :show-text="false" />
            <span class="ecm-item-count">{{ c.expertCount }} 人 · {{ averageProficiency(c) }}%</span>
          </li>
        </ul>
      </div>
    </div>

    <p class="ecm-caption">
      目录来自 {{ ENDPOINTS.expertCapabilities.path }}：同一能力被多位已启用专家提及时合并为一行，
      百分比是这些专家对该项的<b>平均熟练度</b>，人数是具备该项的专家数。点领域名会按「领域」去筛专家列表——
      后端列表接口不接收 capability_id，因此筛选只能落到领域这一层，不会按具体能力项过滤。
    </p>
  </section>
</template>

<script setup>
import { computed, ref } from 'vue'
import { Search } from '@element-plus/icons-vue'
import { ENDPOINTS } from '@/modules/expert-alliance/contract'

const props = defineProps({
  data: { type: Object, default: null },
  loading: { type: Boolean, default: false },
  error: { type: String, default: '' },
  activeDomain: { type: String, default: '' }
})
const emit = defineEmits(['pick'])

const keyword = ref('')

// 后端按 capability id 升序返回（experts_registry.rs:467-469），分组时保持这个次序，不重排
const rows = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  const items = props.data?.items ?? []
  if (!kw) return items
  return items.filter((c) => `${c.name} ${c.domain} ${c.id}`.toLowerCase().includes(kw))
})

const groups = computed(() => {
  const order = props.data?.domains?.length ? props.data.domains : [...new Set(rows.value.map((c) => c.domain))]
  const byDomain = new Map(order.map((d) => [d, []]))
  for (const c of rows.value) {
    if (!byDomain.has(c.domain)) byDomain.set(c.domain, [])
    byDomain.get(c.domain).push(c)
  }
  return [...byDomain.entries()]
    .filter(([, items]) => items.length)
    .map(([domain, items]) => ({ domain, items, slots: items.reduce((s, c) => s + c.expertCount, 0) }))
})

const countText = computed(() => {
  const total = props.data?.total ?? 0
  if (!total) return '目录为空'
  return keyword.value.trim()
    ? `筛出 ${rows.value.length} / ${total} 项能力 · ${groups.value.length} 个领域`
    : `${total} 项能力 · ${groups.value.length} 个领域`
})
const emptyText = computed(() => {
  if (!props.data) return '能力目录尚未加载'
  return props.data.items.length ? '没有匹配这个关键词的能力项' : '注册中心里还没有已启用专家登记结构化能力项'
})

// avg_proficiency 是 0–100 的均值（后端把 u8 熟练度求和后除以人数），越界值夹回进度条量程
function averageProficiency(cap) {
  const v = Number(cap?.avgProficiency)
  return Number.isFinite(v) ? Math.max(0, Math.min(100, Math.round(v))) : 0
}
</script>

<style scoped>
.ecm { display: flex; flex-direction: column; gap: 10px; }
.ecm-head { display: flex; align-items: center; gap: 10px; }
.ecm-search { width: 300px; }
.ecm-count { margin-left: auto; font-size: 12px; color: var(--text-muted); }
.ecm-loading { padding: 8px; }
.ecm-groups {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 10px;
}
.ecm-group {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  overflow: hidden;
}
.ecm-group.is-active { border-color: var(--accent); }
.ecm-group-btn {
  display: flex;
  align-items: baseline;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: 0;
  border-bottom: 1px solid var(--border-light);
  background: var(--bg-secondary);
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.ecm-group-btn:hover { background: var(--bg-hover); }
.ecm-group-name { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.ecm-group-meta { font-size: 11px; color: var(--text-muted); }
.ecm-list { margin: 0; padding: 6px 10px; list-style: none; display: flex; flex-direction: column; gap: 6px; }
.ecm-item { display: grid; grid-template-columns: minmax(0, 1fr) 70px 78px; align-items: center; gap: 8px; }
.ecm-item-name { font-size: 12px; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.ecm-item-count { font-size: 11px; color: var(--text-muted); text-align: right; }
.ecm-caption { margin: 0; font-size: 11px; line-height: 1.6; color: var(--text-muted); }
</style>
