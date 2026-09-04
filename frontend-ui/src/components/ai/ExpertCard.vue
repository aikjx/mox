<template>
  <div class="expert-card" :class="{ 'expert-card--compact': compact }">
    <div class="expert-card__header">
      <div class="expert-card__avatar" :style="{ background: getAvatarColor(expert.expert_id) }">
        {{ getInitials(expert.expert_id) }}
      </div>
      <div class="expert-card__info">
        <div class="expert-card__name">{{ expert.description || expert.expert_id }}</div>
        <div class="expert-card__dimension">{{ formatDimension(expert.dimension) }}</div>
      </div>
      <div v-if="!compact" class="expert-card__priority">
        <el-tag size="small" :type="getPriorityType(expert.priority)">P{{ expert.priority }}</el-tag>
      </div>
    </div>

    <div v-if="!compact" class="expert-card__body">
      <div class="expert-card__stats">
        <div class="expert-card__stat">
          <span class="expert-card__stat-label">通过率</span>
          <span class="expert-card__stat-value">{{ ((expert.gate_a_rate_30d || 0) * 100).toFixed(0) }}%</span>
        </div>
        <div class="expert-card__stat">
          <span class="expert-card__stat-label">平均耗时</span>
          <span class="expert-card__stat-value">{{ expert.avg_latency_ms || 0 }}ms</span>
        </div>
      </div>
      <div v-if="expert.supported_classes && expert.supported_classes.size > 0" class="expert-card__classes">
        <el-tag
          v-for="cls in Array.from(expert.supported_classes).slice(0, 3)"
          :key="cls"
          size="small"
          type="info"
          effect="plain"
        >
          {{ cls }}
        </el-tag>
        <el-tag v-if="expert.supported_classes.size > 3" size="small" type="info" effect="plain">
          +{{ expert.supported_classes.size - 3 }}
        </el-tag>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  expert: {
    type: Object,
    required: true
  },
  compact: {
    type: Boolean,
    default: false
  }
})

const colorPalette = [
  '#6366f1', '#06b6d4', '#10b981', '#f59e0b', '#ef4444',
  '#8b5cf6', '#ec4899', '#14b8a6', '#f97316', '#3b82f6'
]

function getAvatarColor(id) {
  if (!id) return colorPalette[0]
  let hash = 0
  for (let i = 0; i < id.length; i++) {
    hash = id.charCodeAt(i) + ((hash << 5) - hash)
  }
  return colorPalette[Math.abs(hash) % colorPalette.length]
}

function getInitials(name) {
  if (!name) return '?'
  const parts = name.split(/[-_]/)
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase()
  }
  return name.slice(0, 2).toUpperCase()
}

function formatDimension(dim) {
  if (!dim) return ''
  const dimMap = {
    'code': '代码', 'security': '安全', 'performance': '性能',
    'architecture': '架构', 'data': '数据', 'ai': 'AI',
    'product': '产品', 'business': '业务', 'legal': '法务',
    'ux': '体验', 'devops': '运维', 'test': '测试'
  }
  return dimMap[dim] || dim
}

function getPriorityType(priority) {
  if (priority >= 8) return 'danger'
  if (priority >= 5) return 'warning'
  return 'info'
}
</script>

<style scoped>
.expert-card {
  padding: 12px;
  background: #fff;
  border-radius: 8px;
  border: 1px solid #e4e3dd;
  transition: all 0.2s ease;
}

.expert-card:hover {
  border-color: #6366f1;
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.1);
}

.expert-card--compact {
  padding: 8px 10px;
}

.expert-card__header {
  display: flex;
  align-items: center;
  gap: 10px;
}

.expert-card__avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 12px;
  font-weight: 600;
  flex-shrink: 0;
}

.expert-card--compact .expert-card__avatar {
  width: 24px;
  height: 24px;
  font-size: 10px;
}

.expert-card__info {
  flex: 1;
  min-width: 0;
}

.expert-card__name {
  font-size: 13px;
  font-weight: 600;
  color: #1a1b1c;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.expert-card--compact .expert-card__name {
  font-size: 12px;
}

.expert-card__dimension {
  font-size: 11px;
  color: #9ca3af;
  margin-top: 1px;
}

.expert-card__body {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid #f4f3ee;
}

.expert-card__stats {
  display: flex;
  gap: 16px;
  margin-bottom: 8px;
}

.expert-card__stat {
  display: flex;
  flex-direction: column;
}

.expert-card__stat-label {
  font-size: 10px;
  color: #9ca3af;
}

.expert-card__stat-value {
  font-size: 13px;
  font-weight: 600;
  color: #1a1b1c;
}

.expert-card__classes {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}
</style>
