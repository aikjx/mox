<template>
  <section class="ssa">
    <h2 class="ssa-title">会话统计</h2>
    <p class="ssa-dim">
      口径是<strong>全平台</strong>：后端遍历内存里的全部会话（session_stats），不受左侧过滤条件影响，也不按 user_id 切分。
    </p>
    <el-alert v-if="store.error.stats" type="error" show-icon :closable="false" title="统计获取失败"
      :description="store.error.stats" />
    <p v-else-if="store.loading.stats" class="ssa-dim">统计计算中…</p>
    <p v-else-if="!stats" class="ssa-dim">尚未加载。</p>

    <template v-else>
      <dl class="ssa-grid">
        <div v-for="cell in cells" :key="cell.label" class="ssa-cell">
          <dt class="ssa-cell-label">{{ cell.label }}</dt>
          <dd class="ssa-cell-value">{{ cell.value }}</dd>
          <dd class="ssa-dim">{{ cell.hint }}</dd>
        </div>
      </dl>

      <div class="ssa-block">
        <h3 class="ssa-subtitle">状态分布</h3>
        <p class="ssa-dim">
          三档计数之和 {{ counted }} / 总会话 {{ stats.totalSessions }}{{ gapText }}。
          后端只统计 active / archived / closed，写成别的状态不报错，但会从这里消失。
        </p>
        <div class="ssa-bars">
          <div v-for="bar in statusBars" :key="bar.label" class="ssa-bar-row">
            <span class="ssa-bar-label">{{ bar.label }}</span>
            <span class="ssa-bar-track"><i class="ssa-bar-fill" :style="{ width: `${bar.pct}%` }" /></span>
            <span class="ssa-bar-value">{{ bar.count }}</span>
          </div>
        </div>
      </div>

      <div class="ssa-block">
        <h3 class="ssa-subtitle">按会话类型</h3>
        <p class="ssa-dim">来自 <code>session_type_distribution</code>：统计的是会话条数，不是消息条数。</p>
        <ul v-if="typeRows.length" class="ssa-list">
          <li v-for="t in typeRows" :key="t.type" class="ssa-list-item">
            <button class="ssa-jump" type="button" :title="`按类型 ${t.type} 过滤列表`" @click="filterByType(t.type)">
              {{ t.label }}
            </button>
            <span class="ssa-dim">{{ t.type }}</span>
            <span>{{ t.count }} 个</span>
            <span class="ssa-dim">{{ t.share }}%</span>
          </li>
        </ul>
        <p v-else class="ssa-dim">还没有会话。</p>
      </div>

      <div class="ssa-block">
        <h3 class="ssa-subtitle">会话数最多的专家（前 {{ stats.topExpertsBySessions.length }}）</h3>
        <p class="ssa-dim">
          排序按计数降序，后端 <code>take(10)</code> 截断；相同时的先后取决于 HashMap 迭代序，不是名称。
        </p>
        <ul v-if="stats.topExpertsBySessions.length" class="ssa-list">
          <li v-for="(e, i) in stats.topExpertsBySessions" :key="e.expertId" class="ssa-list-item">
            <span class="ssa-rank">{{ i + 1 }}</span>
            <button class="ssa-jump" type="button" :title="`按专家 ${e.expertId} 过滤列表`"
              @click="filterByExpert(e.expertId)">{{ expertName(e.expertId) }}</button>
            <span class="ssa-dim">{{ e.expertId }}</span>
            <span>{{ e.count }} 个会话</span>
          </li>
        </ul>
        <p v-else class="ssa-dim">没有任何会话关联专家——expert_ids 为空的会话不进这张表。</p>
      </div>
    </template>
  </section>
</template>

<script setup>
// 统计面板：只读 session_stats 的响应键，一个前端派生量都不假装是后端给的。
// 唯一在本组件算的是"占比"与"三档之和与总数的差"，两者都写明是自己算的。
import { computed } from 'vue'
import { SESSION_STATUSES, SESSION_TYPES, sessionTypeLabel } from '@/modules/expert-alliance/contract'
import { minutesText } from '@/modules/expert-alliance/model'

const props = defineProps({
  store: { type: Object, required: true },
  expertNames: { type: Object, default: () => ({}) }
})
const store = props.store

const stats = computed(() => store.stats)
const counted = computed(() =>
  stats.value ? stats.value.activeSessions + stats.value.archivedSessions + stats.value.closedSessions : 0
)
const gapText = computed(() => {
  const s = stats.value
  if (!s) return ''
  const gap = s.totalSessions - counted.value
  return gap > 0 ? `，差 ${gap} 个会话处于三档之外的状态` : ''
})

const cells = computed(() => {
  const s = stats.value
  if (!s) return []
  return [
    { label: '会话总数', value: `${s.totalSessions}`, hint: `其中今日新建 ${s.sessionsToday} 个` },
    { label: '消息总数', value: `${s.totalMessages}`, hint: `平均 ${s.avgMessagesPerSession.toFixed(2)} 条 / 会话（含空会话）` },
    { label: '平均活跃时长', value: minutesText(s.avgSessionDurationMinutes), hint: 'created_at 到 last_active_at 的分钟差，负值不计数' },
    {
      label: '进行中',
      value: `${s.activeSessions}`,
      hint: `归档 ${s.archivedSessions} · 关闭 ${s.closedSessions}`
    }
  ]
})

const statusBars = computed(() => {
  const s = stats.value
  if (!s) return []
  const total = s.totalSessions || 1
  const counts = { active: s.activeSessions, archived: s.archivedSessions, closed: s.closedSessions }
  return SESSION_STATUSES.map((st) => ({ label: st.label, count: counts[st.value], pct: pct(counts[st.value], total) }))
})

const typeRows = computed(() => {
  const s = stats.value
  if (!s) return []
  const total = s.totalSessions || 1
  // 后端给的是 map，顺序不稳定：这里按类型声明序排，同类型数多的在前
  const known = SESSION_TYPES.map((t) => t.value)
  const rows = [...s.typeDistribution]
  rows.sort((a, b) => {
    const ka = known.indexOf(a.type)
    const kb = known.indexOf(b.type)
    if (ka !== kb) return (ka < 0 ? known.length : ka) - (kb < 0 ? known.length : kb)
    return b.count - a.count
  })
  return rows.map((r) => ({
    type: r.type,
    label: sessionTypeLabel(r.type),
    count: r.count,
    share: pct(r.count, total).toFixed(0)
  }))
})

function pct(part, total) {
  return total > 0 ? (part / total) * 100 : 0
}

// 只有已加载的那页注册表名单能翻译出名字；查不到就显示 id，
// 因为后端从不校验 expert_ids，这里既不能断定专家已删除，也不能断定它不存在。
function expertName(id) {
  return props.expertNames[id] || `（不在已加载的专家名单中）`
}

function filterByType(type) {
  store.setFilter('sessionType', type)
  store.loadList()
}

function filterByExpert(expertId) {
  store.setFilter('expertId', expertId)
  store.loadList()
}
</script>

<style scoped>
.ssa { display: flex; flex-direction: column; gap: 10px; }
.ssa-title { margin: 0; font-size: 15px; color: var(--text-primary); }
.ssa-subtitle { margin: 0; font-size: 13px; color: var(--text-primary); }
.ssa-dim { margin: 0; font-size: 12px; color: var(--text-muted); line-height: 1.6; }
.ssa-dim code { font-size: 11px; color: var(--accent-light); }
.ssa-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 8px; margin: 0; }
.ssa-cell { padding: 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); }
.ssa-cell-label { margin: 0; font-size: 12px; color: var(--text-muted); }
.ssa-cell-value { margin: 2px 0; font-size: 18px; color: var(--text-primary); }
.ssa-cell dd { font-size: 12px; }
.ssa-block { display: flex; flex-direction: column; gap: 6px; padding-top: 8px; border-top: 1px solid var(--border-light); }
.ssa-bars { display: flex; flex-direction: column; gap: 4px; }
.ssa-bar-row { display: grid; grid-template-columns: 72px 1fr 48px; align-items: center; gap: 6px; font-size: 12px; color: var(--text-secondary); }
.ssa-bar-track { height: 6px; border-radius: var(--radius-sm); background: var(--bg-tertiary); overflow: hidden; }
.ssa-bar-fill { display: block; height: 100%; background: var(--accent-fill); color: var(--on-accent); }
.ssa-bar-value { text-align: right; font-variant-numeric: tabular-nums; }
.ssa-list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.ssa-list-item { display: flex; align-items: baseline; gap: 8px; flex-wrap: wrap; font-size: 12px; color: var(--text-secondary); }
.ssa-jump { background: none; border: 0; padding: 0; font: inherit; color: var(--accent-light); cursor: pointer; }
.ssa-jump:hover { text-decoration: underline; }
.ssa-rank { color: var(--text-muted); font-variant-numeric: tabular-nums; }
</style>
