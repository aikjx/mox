<template>
  <div class="adm">
    <div class="head">
      <div>
        <h2 class="page-title">系统管理</h2>
        <p class="page-subtitle">管理总览 · 访问凭证 · 审计日志 · 存储与模块 · HITL 人机协同审批</p>
      </div>
    </div>

    <div class="adm-tabs">
      <div
        v-for="t in TABS"
        :key="t.key"
        class="adm-tab"
        :class="{ active: t.key === activeTab }"
        @click="switchTab(t.key)"
      >
        <el-icon :size="16"><component :is="t.icon" /></el-icon>
        <span>{{ t.label }}</span>
      </div>
    </div>

    <component :is="activeComp" />
  </div>
</template>

<script setup>
import { computed, defineAsyncComponent } from 'vue'
import { useRoute, useRouter } from 'vue-router'

// 管理区面板（懒加载，tab 切换时才载入）
const PANELS = {
  overview: defineAsyncComponent(() => import('./panels/AdminOverview.vue')),
  access: defineAsyncComponent(() => import('./panels/AdminAccess.vue')),
  audit: defineAsyncComponent(() => import('./panels/AdminAudit.vue')),
  storage: defineAsyncComponent(() => import('./panels/AdminStorage.vue')),
  hitl: defineAsyncComponent(() => import('./panels/AdminHitl.vue')),
  monitor: defineAsyncComponent(() => import('./panels/AdminMonitor.vue')),
  docs: defineAsyncComponent(() => import('./panels/AdminDocs.vue')),
  llm: defineAsyncComponent(() => import('./panels/AdminLlm.vue'))
}

const TABS = [
  { key: 'overview', label: '管理总览', icon: 'Odometer' },
  { key: 'access', label: '访问凭证', icon: 'Key' },
  { key: 'audit', label: '审计日志', icon: 'List' },
  { key: 'storage', label: '存储与模块', icon: 'Coin' },
  { key: 'hitl', label: 'HITL 审批', icon: 'Clock' },
  { key: 'monitor', label: '系统监控', icon: 'DataAnalysis' },
  { key: 'llm', label: '大模型配置', icon: 'Cpu' },
  { key: 'docs', label: 'API 文档', icon: 'Document' }
]

const route = useRoute()
const router = useRouter()

const activeTab = computed(() => {
  const t = String(route.query.tab || 'overview')
  return PANELS[t] ? t : 'overview'
})

const activeComp = computed(() => PANELS[activeTab.value])

function switchTab(key) {
  router.replace({ query: { ...route.query, tab: key === 'overview' ? undefined : key } })
}
</script>

<style scoped>
.head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 16px;
}
.adm-tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.adm-tab {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 9px 16px;
  border-radius: 10px;
  background: var(--bg-panel);
  border: 1px solid var(--border-light);
  box-shadow: var(--shadow-sm);
  font-size: 13px;
  font-weight: 500;
  color: var(--text-2);
  cursor: pointer;
  transition: all var(--transition);
}
.adm-tab:hover { color: var(--brand); border-color: var(--brand); }
.adm-tab.active {
  background: linear-gradient(135deg, var(--brand-light), var(--brand-dark));
  color: #fff;
  border-color: transparent;
  box-shadow: 0 8px 22px rgba(79, 70, 229, 0.35);
}
.adm-tabs + :deep(.panel) { margin-top: 0; }
</style>
