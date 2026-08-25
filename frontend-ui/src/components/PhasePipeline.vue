<template>
  <div class="pipeline" :class="{ compact }">
    <div class="pipeline-inner">
      <div
        v-for="(p, idx) in PHASES"
        :key="p.key"
        class="pp-step"
        :class="{ active: currentIndex === idx, done: idx < currentIndex, locked: locked && idx > currentIndex }"
        @click="!locked && onStepClick(idx)"
      >
        <span class="pp-orb">
          <el-icon v-if="idx < currentIndex" :size="14"><Check /></el-icon>
          <span v-else class="pp-orb-num">{{ idx + 1 }}</span>
        </span>
        <span class="pp-meta" v-if="!compact">
          <span class="pp-name">{{ p.label }}</span>
          <span class="pp-desc">{{ p.desc }}</span>
        </span>
        <span v-if="idx < PHASES.length - 1" class="pp-connector">
          <span class="pp-connector-fill" :style="{ width: connectorWidth(idx) }"></span>
        </span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { Check } from '@element-plus/icons-vue'

const props = defineProps({
  modelValue: { type: String, default: 'requirement' },
  progress: { type: Array, default: null },
  compact: { type: Boolean, default: false },
  locked: { type: Boolean, default: false }
})
const emit = defineEmits(['update:modelValue', 'change'])

const PHASES = [
  { key: 'requirement', label: '需求架构', desc: '编译 · 建模 · 拆解问题', color: '#6366f1', route: '/caomei' },
  { key: 'graph',       label: '知识图谱', desc: '璇玑 · 关系 · 全维发现',   color: '#06b6d4', route: '/graph' },
  { key: 'design',      label: '方案设计', desc: '架构 · 编排 · 资源绑定',   color: '#8b5cf6', route: '/workflow' },
  { key: 'develop',     label: '开发运行', desc: '算子 · 代码 · 执行',       color: '#10b981', route: '/algolab' },
  { key: 'release',     label: '运行发布', desc: '发布 · 监控 · 交付',       color: '#f59e0b', route: '/monitor' }
]

const currentIndex = computed(() => {
  const i = PHASES.findIndex((x) => x.key === props.modelValue)
  return Math.max(0, Math.min(PHASES.length - 1, i < 0 ? 0 : i))
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
  const k = PHASES[idx].key
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
  flex: 0 0 20%;
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

.pipeline.compact { padding: 6px 12px; border-radius: 10px; }
.pipeline.compact .pp-orb { width: 22px; height: 22px; border-radius: 7px; font-size: 11px; }
.pipeline.compact .pp-connector { top: 12px; }
</style>
