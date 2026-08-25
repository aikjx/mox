<template>
  <div class="project-chip" :class="{ 'is-empty': !currentProject }">
    <div class="pc-icon">
      <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
        <path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6z"
              :fill="currentProject ? '#6366f1' : '#94a3b8'" opacity="0.15"/>
        <path d="M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6z"
              fill="none" :stroke="currentProject ? '#6366f1' : '#94a3b8'" stroke-width="1.6" stroke-linejoin="round"/>
      </svg>
    </div>
    <div class="pc-body">
      <div class="pc-label">跟进项目</div>
      <div class="pc-name">
        <template v-if="currentProject">{{ currentProject.name }}</template>
        <template v-else>未选择 · 点击顶栏 <b>选择项目</b> 创建/切换</template>
      </div>
    </div>
    <span v-if="currentProject?.status" class="pc-status" :class="statusCls">{{ statusLabel }}</span>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useProject } from '@/composables/projectContext.js'

const { currentProject, onChange, ensureProjectContext } = useProject()

const tick = ref(0)
const bump = () => tick.value++
onMounted(async () => {
  const off = onChange(bump)
  onBeforeUnmount(off)
  try { await ensureProjectContext() } catch {}
  bump()
})

const statusCls = computed(() => {
  const s = String(currentProject.value?.status || '').toLowerCase()
  if (['active', '进行中', 'in_progress'].includes(s)) return 'active'
  if (['done', 'completed', '已完成'].includes(s)) return 'done'
  if (['paused', 'blocked'].includes(s)) return 'warn'
  return ''
})
const statusLabel = computed(() => {
  const s = String(currentProject.value?.status || '').toLowerCase()
  return { active: '进行中', in_progress: '进行中', done: '已完成', completed: '已完成',
    paused: '已暂停', blocked: '阻塞', planning: '规划中', pending: '待启动' }[s]
    || (currentProject.value?.status || '规划中')
})
</script>

<style scoped>
.project-chip {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px 8px 10px;
  border-radius: 10px;
  background: linear-gradient(180deg, #ffffff 0%, #f8fafc 100%);
  border: 1px solid #e2e8f0;
  box-shadow: 0 1px 2px -1px rgba(15,23,42,0.05);
  margin-bottom: 12px;
}
.project-chip.is-empty { opacity: 0.88; }
.pc-icon {
  width: 28px; height: 28px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 8px;
  background: #fff;
  border: 1px solid #e2e8f0;
  flex: 0 0 auto;
}
.pc-body { flex: 1 1 auto; min-width: 0; }
.pc-label { font-size: 11px; color: #94a3b8; letter-spacing: 0.4px; line-height: 1; margin-bottom: 3px; }
.pc-name {
  font-size: 13px; color: #0f172a; font-weight: 600;
  line-height: 1.25;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.pc-name b { color: #6366f1; }
.pc-status {
  font-size: 11px;
  padding: 3px 8px;
  border-radius: 999px;
  background: #e2e8f0; color: #334155;
  flex: 0 0 auto;
}
.pc-status.active { background: #eef2ff; color: #4338ca; }
.pc-status.done   { background: #ecfdf5; color: #047857; }
.pc-status.warn   { background: #fff7ed; color: #c2410c; }
</style>
