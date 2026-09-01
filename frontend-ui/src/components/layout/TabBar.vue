<template>
  <div class="tabs" v-if="!isAIFullscreen">
    <div
      v-for="t in tabs"
      :key="t.path"
      class="tab"
      :class="{ active: t.path === currentPath }"
      @click="$router.push(t.path)"
    >
      {{ t.label }}
      <el-icon v-if="t.closable" class="tab-close" @click.stop="closeTab(t.path)">
        <Close />
      </el-icon>
    </div>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Close } from '@element-plus/icons-vue'
import { NAV_MODULES, SUB_MODULES, HIDDEN_MODULES } from '@/constants'

const props = defineProps({
  isAIFullscreen: { type: Boolean, default: false }
})

const route = useRoute()
const router = useRouter()

const currentPath = computed(() => route.path)

// 构建完整的模块索引
const ALL_MODULES = computed(() => {
  const list = [...NAV_MODULES]
  Object.values(SUB_MODULES).forEach(subs => {
    subs.forEach(s => {
      if (!list.find(m => m.path === s.path)) {
        list.push({ key: s.key, label: s.label, path: s.path, color: '#6366f1', bg: '#eef2ff' })
      }
    })
  })
  HIDDEN_MODULES.forEach(m => {
    if (!list.find(x => x.path === m.path)) list.push(m)
  })
  return list
})

const tabs = computed(() => {
  const list = [{ path: '/dashboard', label: '工作台', closable: false }]
  const m = ALL_MODULES.value.find((x) => x.path === route.path)
  if (m && m.path !== '/dashboard') list.push({ path: m.path, label: m.label, closable: true })
  return list
})

function closeTab(path) {
  if (route.path === path) router.push('/dashboard')
}
</script>

<style scoped>
.tabs {
  height: 42px; flex-shrink: 0; background: var(--bg-card); border-bottom: 1px solid var(--border);
  display: flex; align-items: center; gap: 6px; padding: 0 16px; overflow-x: auto;
}
.tab {
  display: flex; align-items: center; gap: 6px; height: 28px; padding: 0 12px;
  border-radius: 8px; font-size: 13px; color: var(--text-2); cursor: pointer;
  white-space: nowrap; transition: all var(--transition);
}
.tab:hover { background: var(--bg-page); }
.tab.active { background: var(--brand-soft); color: var(--brand-dark); font-weight: 600; }
.tab-close { font-size: 12px; border-radius: 50%; }
.tab-close:hover { background: var(--bg-tertiary); color: #fff; }

@media (max-width: 768px) {
  .tabs { display: none; }
}
</style>
