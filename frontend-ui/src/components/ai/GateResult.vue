<template>
  <div class="gate-result" :class="`gate-result--${gradeClass}`">
    <!-- 总分展示 -->
    <div class="gate-result__header">
      <div class="gate-result__score-ring">
        <svg viewBox="0 0 100 100" class="gate-result__ring">
          <circle cx="50" cy="50" r="42" fill="none" stroke="#e4e3dd" stroke-width="8" />
          <circle
            cx="50" cy="50" r="42" fill="none"
            :stroke="gradeColor" stroke-width="8"
            stroke-linecap="round"
            :stroke-dasharray="`${score * 263.9} 263.9`"
            transform="rotate(-90 50 50)"
          />
        </svg>
        <div class="gate-result__score-text">
          <span class="gate-result__score-value">{{ (score * 100).toFixed(0) }}</span>
          <span class="gate-result__score-unit">分</span>
        </div>
      </div>
      <div class="gate-result__summary">
        <div class="gate-result__grade" :style="{ color: gradeColor }">
          {{ gradeLabel }}
        </div>
        <div class="gate-result__status">
          <el-tag :type="passed ? 'success' : 'danger'" size="small">
            {{ passed ? '通过门禁' : '未通过门禁' }}
          </el-tag>
          <el-tag v-if="retryable" type="warning" size="small">可重试优化</el-tag>
        </div>
        <div v-if="latency" class="gate-result__latency">
          评估耗时 {{ formatLatency(latency) }}
        </div>
      </div>
    </div>

    <!-- 各维度得分 -->
    <div v-if="dimensions && Object.keys(dimensions).length > 0" class="gate-result__dimensions">
      <div class="gate-result__dimensions-title">各维度得分</div>
      <div class="gate-result__dimension-list">
        <div v-for="(value, key) in dimensions" :key="key" class="gate-result__dimension">
          <div class="gate-result__dimension-header">
            <span class="gate-result__dimension-name">{{ formatDimensionName(key) }}</span>
            <span class="gate-result__dimension-value">{{ (value * 100).toFixed(0) }}%</span>
          </div>
          <el-progress
            :percentage="Math.round(value * 100)"
            :stroke-width="6"
            :color="getDimensionColor(value)"
          />
        </div>
      </div>
    </div>

    <!-- 阻断原因 -->
    <div v-if="blockReason" class="gate-result__block">
      <el-alert
        :title="blockReason"
        type="error"
        :closable="false"
        show-icon
      />
    </div>

    <!-- 改进建议 -->
    <div v-if="suggestions && suggestions.length > 0" class="gate-result__suggestions">
      <div class="gate-result__suggestions-title">改进建议</div>
      <ul class="gate-result__suggestion-list">
        <li v-for="(suggestion, index) in suggestions" :key="index" class="gate-result__suggestion">
          <span class="gate-result__suggestion-number">{{ index + 1 }}</span>
          <span>{{ suggestion }}</span>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  result: {
    type: Object,
    required: true
  }
})

const score = computed(() => props.result.score || 0)
const grade = computed(() => props.result.grade || 'D')
const passed = computed(() => props.result.passed || false)
const retryable = computed(() => grade.value === 'C')
const dimensions = computed(() => props.result.dimensions || {})
const blockReason = computed(() => props.result.block_reason || null)
const suggestions = computed(() => props.result.suggestions || [])
const latency = computed(() => props.result.latency_ms || 0)

const gradeColor = computed(() => {
  const colors = { A: '#10b981', B: '#06b6d4', C: '#f59e0b', D: '#ef4444' }
  return colors[grade.value] || '#9ca3af'
})

const gradeClass = computed(() => grade.value.toLowerCase())

const gradeLabel = computed(() => {
  const labels = { A: '优秀', B: '良好', C: '合格', D: '不合格' }
  return labels[grade.value] || grade.value
})

function formatDimensionName(key) {
  const names = {
    quality: '质量', coverage: '覆盖度', timeliness: '时效',
    security: '安全', performance: '性能', maintainability: '可维护性',
    correctness: '正确性', completeness: '完整性', consistency: '一致性'
  }
  return names[key] || key
}

function getDimensionColor(value) {
  if (value >= 0.85) return '#10b981'
  if (value >= 0.70) return '#06b6d4'
  if (value >= 0.50) return '#f59e0b'
  return '#ef4444'
}

function formatLatency(ms) {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}
</script>

<style scoped>
.gate-result {
  padding: 16px;
  background: #fff;
  border-radius: 8px;
}

.gate-result--a { border-left: 4px solid #10b981; }
.gate-result--b { border-left: 4px solid #06b6d4; }
.gate-result--c { border-left: 4px solid #f59e0b; }
.gate-result--d { border-left: 4px solid #ef4444; }

.gate-result__header {
  display: flex;
  align-items: center;
  gap: 24px;
  margin-bottom: 16px;
}

.gate-result__score-ring {
  position: relative;
  width: 80px;
  height: 80px;
  flex-shrink: 0;
}

.gate-result__ring {
  width: 100%;
  height: 100%;
}

.gate-result__score-text {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
}

.gate-result__score-value {
  font-size: 22px;
  font-weight: 700;
  color: #1a1b1c;
}

.gate-result__score-unit {
  font-size: 11px;
  color: #9ca3af;
  margin-left: 2px;
}

.gate-result__summary {
  flex: 1;
}

.gate-result__grade {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 6px;
}

.gate-result__status {
  display: flex;
  gap: 8px;
  margin-bottom: 4px;
}

.gate-result__latency {
  font-size: 12px;
  color: #9ca3af;
}

.gate-result__dimensions {
  margin-bottom: 16px;
}

.gate-result__dimensions-title,
.gate-result__suggestions-title {
  font-size: 13px;
  font-weight: 600;
  color: #1a1b1c;
  margin-bottom: 10px;
}

.gate-result__dimension-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.gate-result__dimension-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 4px;
}

.gate-result__dimension-name {
  font-size: 12px;
  color: #6b7280;
}

.gate-result__dimension-value {
  font-size: 12px;
  font-weight: 600;
  color: #1a1b1c;
}

.gate-result__block {
  margin-bottom: 12px;
}

.gate-result__suggestion-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.gate-result__suggestion {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 0;
  font-size: 13px;
  color: #374151;
  line-height: 1.5;
}

.gate-result__suggestion-number {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #fef3c7;
  color: #d97706;
  font-size: 11px;
  font-weight: 600;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  margin-top: 1px;
}
</style>
