<template>
  <div class="pipeline" :class="{ compact }">
    <!-- 标题栏（compact 模式显示，提供上下文说明） -->
    <div v-if="compact && title !== false" class="pipeline-header">
      <div class="ph-left">
        <span class="ph-dot" :style="{ background: currentPhaseColor }"></span>
        <span class="ph-title">{{ title || '项目全维流程' }}</span>
        <span v-if="showProgress && currentPhaseLabel" class="ph-current">
          · {{ currentPhaseLabel }}
        </span>
      </div>
      <div v-if="showProgress" class="ph-right">
        <span class="ph-progress-text">{{ overallProgress }}%</span>
        <span class="ph-progress-bar">
          <span class="ph-progress-fill" :style="{ width: overallProgress + '%' }"></span>
        </span>
      </div>
    </div>

    <div class="pipeline-inner">
      <div
        v-for="(p, idx) in phases"
        :key="p.key"
        class="pp-step"
        :class="{ active: currentIndex === idx, done: idx < currentIndex, locked: locked && idx > currentIndex }"
        @click="!locked && onStepClick(idx)"
      >
        <span class="pp-orb">
          <el-icon v-if="idx < currentIndex" :size="compact ? 12 : 14"><Check /></el-icon>
          <span v-else class="pp-orb-num">{{ idx + 1 }}</span>
        </span>

        <!-- 步骤文字：非紧凑模式显示名称+描述；紧凑模式显示短名称 -->
        <span class="pp-meta" :class="{ 'pp-meta-compact': compact }">
          <span class="pp-name">{{ p.label }}</span>
          <span v-if="!compact && p.desc" class="pp-desc">{{ p.desc }}</span>
          <span v-else-if="compact && p.short" class="pp-short">{{ p.short }}</span>
        </span>

        <span v-if="idx < phases.length - 1" class="pp-connector">
          <span class="pp-connector-fill" :style="{ width: connectorWidth(idx) }"></span>
        </span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { Check } from '@element-plus/icons-vue'
import { PROJECT_PHASES } from '@/types'

const props = defineProps({
  modelValue: { type: String, default: 'requirement' },
  progress: { type: Array, default: null },
  compact: { type: Boolean, default: false },
  locked: { type: Boolean, default: false },
  phases: { type: Array, default: null },
  title: { type: [String, Boolean], default: null },
  showProgress: { type: Boolean, default: false }
})
const emit = defineEmits(['update:modelValue', 'change'])

// 优先使用外部传入的 phases，否则使用全局 PROJECT_PHASES
const phases = computed(() => {
  if (props.phases && props.phases.length) return props.phases
  // 兼容旧版：从全局 PROJECT_PHASES 映射，添加 short 短标签
  return PROJECT_PHASES.map(p => ({
    ...p,
    short: p.desc?.split('·')[0]?.trim() || p.desc || ''
  }))
})

const currentIndex = computed(() => {
  const i = phases.value.findIndex((x) => x.key === props.modelValue)
  return Math.max(0, Math.min(phases.value.length - 1, i < 0 ? 0 : i))
})

const currentPhaseLabel = computed(() => phases.value[currentIndex.value]?.label || '')
const currentPhaseColor = computed(() => phases.value[currentIndex.value]?.color || '#6366f1')

const overallProgress = computed(() => {
  if (props.progress && Array.isArray(props.progress)) {
    const vals = props.progress.map(v => Math.max(0, Math.min(100, Number(v || 0))))
    if (!vals.length) return 0
    return Math.round(vals.reduce((a, b) => a + b, 0) / vals.length)
  }
  // 简化进度：已完成阶段 × 100 + 当前阶段 50%
  const done = currentIndex.value
  const total = phases.value.length
  if (done >= total) return 100
  return Math.round((done / total) * 100 + (50 / total))
})

function connectorWidth(idx) {
  if (props.progress && Array.isArray(props.progress)) {
    return Math.max(0, Math.min(100, Number(props.progress[idx] ?? 0))) + '%'
  }
  if (idx < currentIndex.value) return '100%'
  if (idx === currentIndex.value) return '50%'
  return '0%'
}
function onStepClick(idx) {
  const k = phases.value[idx].key
  emit('update:modelValue', k)
  emit('change', { key: k, index: idx })
}
</script>

<style scoped>
.pipeline {
  width: 100%;
  border-radius: 12px;
  padding: 12px 16px;
  background: linear-gradient(180deg, #ffffff 0%, #fafbfc 100%);
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 3px -1px rgba(15, 23, 42, 0.06);
}
.pipeline-inner {
  position: relative;
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
}
.pp-step {
  position: relative;
  flex: 1;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 2px 0;
  cursor: pointer;
  user-select: none;
}
.pp-step.locked { cursor: not-allowed; }
.pp-orb {
  flex: 0 0 auto;
  width: 28px; height: 28px;
  border-radius: 9px;
  display: grid;
  place-items: center;
  background: #fff;
  color: #94a3b8;
  border: 1.5px solid #cbd5e1;
  font-weight: 700;
  font-size: 12px;
  transition: all 160ms ease;
  position: relative;
  z-index: 2;
}
.pp-step.active .pp-orb {
  background: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
  color: #fff;
  border-color: transparent;
  box-shadow: 0 8px 18px -6px rgba(99, 102, 241, 0.50), 0 0 0 4px rgba(99, 102, 241, 0.10);
  transform: translateY(-1px);
}
.pp-step.done .pp-orb {
  background: #ecfdf5;
  color: #059669;
  border-color: #10b981;
}
.pp-orb-num { line-height: 1; }
.pp-meta { display: flex; flex-direction: column; min-width: 0; padding-top: 3px; flex: 1; }
.pp-name { font-size: 13.5px; font-weight: 600; color: #334155; line-height: 1.2; white-space: nowrap; }
.pp-step.active .pp-name { color: #4f46e5; }
.pp-step.done .pp-name { color: #047857; }
.pp-desc { margin-top: 3px; font-size: 11.5px; color: #94a3b8; line-height: 1.3; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 240px; }

/* 连接线：相邻 step 从 orb 中心到 orb 中心 */
.pp-connector {
  position: absolute;
  top: 16px;
  left: 100%;
  width: 100%;
  margin-left: -14px;
  height: 3px;
  border-radius: 3px;
  background: #e2e8f0;
  overflow: hidden;
  z-index: 1;
}
.pp-connector-fill {
  display: block;
  height: 100%;
  width: 0;
  background: linear-gradient(90deg, #6366f1 0%, #06b6d4 100%);
  transition: width 240ms cubic-bezier(.4,0,.2,1);
}
.pp-step.done .pp-connector-fill { background: linear-gradient(90deg, #10b981 0%, #6366f1 100%); }

/* ============ 紧凑模式改进 ============ */
.pipeline.compact {
  padding: 10px 16px;
  border-radius: 10px;
}

/* 紧凑模式标题栏 */
.pipeline-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-bottom: 8px;
  margin-bottom: 8px;
  border-bottom: 1px solid #e2e8f0;
}
.ph-left {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  color: #475569;
}
.ph-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}
.ph-title {
  font-weight: 600;
  color: #334155;
  font-size: 12.5px;
}
.ph-current {
  color: #64748b;
  font-size: 12px;
  font-weight: 400;
}
.ph-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ph-progress-text {
  font-size: 12px;
  font-weight: 600;
  color: #6366f1;
  font-variant-numeric: tabular-nums;
}
.ph-progress-bar {
  width: 60px;
  height: 4px;
  background: #e2e8f0;
  border-radius: 4px;
  overflow: hidden;
}
.ph-progress-fill {
  display: block;
  height: 100%;
  background: linear-gradient(90deg, #6366f1 0%, #06b6d4 100%);
  border-radius: 4px;
  transition: width 300ms ease;
}

/* 紧凑模式步骤：竖排，圆点在上，文字在下 */
.pipeline.compact .pipeline-inner {
  align-items: flex-start;
}
.pipeline.compact .pp-step {
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 0;
}
.pipeline.compact .pp-orb {
  width: 24px;
  height: 24px;
  border-radius: 7px;
  font-size: 11px;
}
.pipeline.compact .pp-step.active .pp-orb {
  transform: scale(1.06);
}
.pipeline.compact .pp-meta {
  padding-top: 0;
  text-align: center;
  align-items: center;
  width: 100%;
}
.pipeline.compact .pp-name {
  font-size: 11.5px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}
.pipeline.compact .pp-short {
  margin-top: 2px;
  font-size: 10px;
  color: #94a3b8;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}
.pipeline.compact .pp-step.done .pp-short { color: #10b981; }
.pipeline.compact .pp-step.active .pp-short { color: #6366f1; }

/* 紧凑模式连接线位置调整（竖排后，线在圆点右侧水平方向） */
.pipeline.compact .pp-connector {
  top: 12px;
  left: 50%;
  width: 100%;
  margin-left: 0;
}
</style>
