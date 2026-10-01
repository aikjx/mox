<template>
  <div v-if="scores" class="mxe-root">
    <div class="mxe-head">
      <span class="mxe-title">为什么匹配</span>
      <span class="mxe-total">总分 {{ fmt(matchScore) }} = Σ 维得分 × 权重</span>
    </div>

    <div v-for="row in rows" :key="row.key" class="mxe-row">
      <span class="mxe-label">{{ row.label }}</span>
      <i class="mxe-track"><i class="mxe-fill" :style="{ width: pct(row.value) }" /></i>
      <span class="mxe-val">{{ fmt(row.value) }}</span>
      <span class="mxe-weight">权重 {{ fmt(row.weight) }}</span>
      <span class="mxe-contrib">贡献 +{{ fmt(row.contrib) }}</span>
    </div>

    <p v-if="matchReason" class="mxe-reason">{{ matchReason }}</p>
    <p class="mxe-note">
      健康度按 <b>0.05</b> 加权（非过滤，仅状态分 is_healthy?1.0:0.2；文档旧口径「0.15」系 bio 权重误植，2026-10-01 纠错）。
      优先级维的权重取「评分权重」（Expert 无独立 rating 字段，priority 归一化后映射）。
    </p>
  </div>
</template>

<script setup>
import { computed } from 'vue'

// U2 匹配透明化：把后端逐维演算 {value, weight, contrib} 画成条。
// 后端未带 scores（旧上游/降级路径）时整个面板不渲染，卡片维持原总分展示。
const props = defineProps({
  scores: { type: Object, default: null },
  matchScore: { type: Number, default: 0 },
  matchReason: { type: String, default: '' }
})

const fmt = (v) => (Number.isFinite(Number(v)) ? Number(v).toFixed(3) : '—')
const pct = (v) => `${Math.min(100, Math.max(0, (Number(v) || 0) * 100)).toFixed(1)}%`

const rows = computed(() => {
  const s = props.scores
  if (!s) return []
  return [
    { key: 'domain', label: '领域', ...s.domain },
    { key: 'capability', label: '能力', ...s.capability },
    { key: 'priority', label: '优先级', ...s.priority },
    { key: 'performance', label: '历史表现', ...s.performance },
    { key: 'health', label: '健康度', ...s.health }
  ]
})
</script>

<style scoped>
.mxe-root {
  margin-top: 8px;
  padding: 8px 10px;
  border: 1px dashed var(--border-soft);
  border-radius: var(--radius-xs);
  background: var(--bg-secondary);
  font-size: 12px;
}
.mxe-head {
  display: flex;
  justify-content: space-between;
  margin-bottom: 6px;
}
.mxe-title { font-weight: 600; color: var(--text-primary); }
.mxe-total { color: var(--text-secondary); }
.mxe-row {
  display: grid;
  grid-template-columns: 3.5em 1fr 3.2em 5.5em 5.5em;
  align-items: center;
  gap: 6px;
  line-height: 1.6;
}
.mxe-label { color: var(--text-primary); }
.mxe-track {
  display: inline-block;
  height: 6px;
  background: var(--bg-tertiary);
  border-radius: var(--radius-xs);
  overflow: hidden;
}
.mxe-fill {
  display: block;
  height: 100%;
  background: var(--accent);
}
.mxe-val, .mxe-weight, .mxe-contrib { color: var(--text-secondary); text-align: right; font-variant-numeric: tabular-nums; }
.mxe-weight { color: var(--text-tertiary); }
.mxe-contrib { color: var(--success); }
.mxe-reason { margin: 6px 0 0; color: var(--text-tertiary); }
.mxe-note { margin: 6px 0 0; color: var(--warning); line-height: 1.5; }
</style>
