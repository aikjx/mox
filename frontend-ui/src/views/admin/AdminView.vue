<template>
  <div class="adm">
    <div class="head">
      <div>
        <h2 class="page-title">系统管理</h2>
        <p class="page-subtitle">菜单管理 · 字典管理 · 参数配置 · 访问凭证 · 审计日志 · 存储与模块 · HITL 人机协同审批</p>
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

    <router-view v-slot="{ Component }">
      <transition name="fade" mode="out-in">
        <component :is="Component" />
      </transition>
    </router-view>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'

const TABS = [
  { key: 'overview', label: '管理总览', icon: 'Odometer' },
  { key: 'user', label: '用户管理', icon: 'User' },
  { key: 'role', label: '角色管理', icon: 'UserFilled' },
  { key: 'department', label: '部门管理', icon: 'OfficeBuilding' },
  { key: 'menu', label: '菜单管理', icon: 'Menu' },
  { key: 'dict', label: '字典管理', icon: 'Collection' },
  { key: 'config', label: '参数配置', icon: 'Tools' },
  { key: 'access', label: '访问凭证', icon: 'Key' },
  { key: 'audit', label: '审计日志', icon: 'List' },
  { key: 'storage', label: '存储与模块', icon: 'Coin' },
  { key: 'hitl', label: 'HITL 审批', icon: 'Clock' },
  { key: 'monitor', label: '系统监控', icon: 'DataAnalysis' },
  { key: 'api', label: '接口管理', icon: 'Connection' },
  { key: 'logs', label: '在线日志', icon: 'Tickets' },
  { key: 'llm', label: '大模型配置', icon: 'Cpu' },
  { key: 'docs', label: 'API 文档', icon: 'Document' }
]

const TAB_KEYS = TABS.map(t => t.key)

const route = useRoute()
const router = useRouter()

const activeTab = computed(() => {
  // 优先从子路由名获取，其次从 query.tab 兼容旧链接
  const name = route.name?.toString().replace('Admin', '').toLowerCase()
  if (name && TAB_KEYS.includes(name)) return name
  const q = String(route.query.tab || 'overview')
  return TAB_KEYS.includes(q) ? q : 'overview'
})

function switchTab(key) {
  const target = key === 'overview' ? '/admin' : `/admin/${key}`
  router.push(target)
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
