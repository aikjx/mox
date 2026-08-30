<template>
  <aside class="sidebar" v-if="!isAIFullscreen">
    <div class="logo" @click="goHome">
      <div class="logo-mark">玄</div>
      <transition name="fade">
        <div v-show="!collapsed" class="logo-text">
          <div class="logo-title">璇玑系统</div>
          <div class="logo-sub">Mox Graph System</div>
        </div>
      </transition>
    </div>

    <!-- 侧边栏导航（分层展开式 · 3 大域 · 核心固定 + 展开折叠） -->
    <el-scrollbar class="nav-scroll">
      <nav class="nav">
        <template v-for="g in NAV_GROUPS" :key="g.key">
          <!-- 分组标题 + 展开/折叠按钮 -->
          <div
            class="nav-group-header"
            :class="{ collapsed: isGroupCollapsed(g.key) }"
            @click="toggleGroup(g.key)"
          >
            <span class="nav-group-label">{{ g.label }}</span>
            <el-icon class="nav-group-arrow">
              <component :is="isGroupCollapsed(g.key) ? 'ArrowDown' : 'ArrowUp'" />
            </el-icon>
          </div>

          <!-- 分组内模块：核心组（project/core）默认展开，扩展组可折叠 -->
          <transition name="nav-expand">
            <div v-show="!isGroupCollapsed(g.key)" class="nav-group-items">
              <el-tooltip
                v-for="m in modulesByGroup(g.key)"
                :key="m.key"
                :content="m.label"
                placement="right"
                :disabled="!collapsed"
                :show-after="300"
              >
                <router-link
                  :to="m.path"
                  class="nav-item"
                  :class="{ active: isActive(m.path) }"
                >
                  <span class="nav-bar"></span>
                  <div class="nav-icon-wrap" :style="{ background: m.bg, color: m.color }">
                    <el-icon class="nav-icon"><component :is="m.icon" /></el-icon>
                  </div>
                  <span v-show="!collapsed" class="nav-label">{{ m.label }}</span>
                  <!-- 有二级子模块时显示展开箭头 -->
                  <el-icon
                    v-if="!collapsed && hasSubModules(m.key)"
                    class="nav-sub-arrow"
                    @click.stop="toggleSubModule(m.key, $event)"
                  >
                    <component :is="expandedSubs.has(m.key) ? 'ArrowDown' : 'ArrowRight'" />
                  </el-icon>
                </router-link>
              </el-tooltip>

              <!-- 二级子模块（展开时显示） -->
              <transition name="nav-sub-expand">
                <div v-if="!collapsed && expandedSubs.has(g.key) && expandedSubs.get(g.key)?.length" class="nav-sub-items">
                  <router-link
                    v-for="s in getExpandedSubs(g.key)"
                    :key="s.key"
                    :to="s.path"
                    class="nav-sub-item"
                    :class="{ active: route.path === s.path }"
                  >
                    <span class="nav-sub-dot"></span>
                    <span class="nav-sub-label">{{ s.label }}</span>
                  </router-link>
                </div>
              </transition>
            </div>
          </transition>
        </template>
      </nav>
    </el-scrollbar>

    <div class="sidebar-footer" v-show="!collapsed">
      <div class="ver">v{{ APP_VERSION }}</div>
      <div class="health">
        <div class="dot" :class="health.status"></div>
        <span>{{ health.label }}</span>
      </div>
    </div>
  </aside>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowUp, ArrowRight } from '@element-plus/icons-vue'
import {
  NAV_MODULES, APP_VERSION, NAV_GROUPS, SUB_MODULES, HIDDEN_MODULES
} from '@/constants'
import { getHealth } from '@/api'

const props = defineProps({
  collapsed: { type: Boolean, default: false },
  isAIFullscreen: { type: Boolean, default: false }
})

const emit = defineEmits(['update:collapsed', 'toggle-collapse'])

const route = useRoute()
const router = useRouter()

const health = ref({ status: 'pending', label: '连接中…' })

/* ===== 导航分组折叠状态 ===== */
const collapsedGroups = ref(new Set(['extend']))

function isGroupCollapsed(groupKey) {
  return collapsedGroups.value.has(groupKey)
}

function toggleGroup(groupKey) {
  const next = new Set(collapsedGroups.value)
  if (next.has(groupKey)) {
    next.delete(groupKey)
  } else {
    next.add(groupKey)
  }
  collapsedGroups.value = next
  localStorage.setItem('nav_collapsed_groups', JSON.stringify([...next]))
}

function restoreNavState() {
  try {
    const saved = localStorage.getItem('nav_collapsed_groups')
    if (saved) {
      collapsedGroups.value = new Set(JSON.parse(saved))
    }
  } catch (_) { /* ignore */ }
}

/* ===== 二级子模块展开状态 ===== */
const expandedSubs = ref(new Map())

function hasSubModules(moduleKey) {
  return !!(SUB_MODULES[moduleKey] && SUB_MODULES[moduleKey].length > 1)
}

function toggleSubModule(moduleKey, event) {
  if (event) event.stopPropagation()
  const next = new Map(expandedSubs.value)
  if (next.has(moduleKey)) {
    next.delete(moduleKey)
  } else {
    next.set(moduleKey, SUB_MODULES[moduleKey] || [])
  }
  expandedSubs.value = next
}

function getExpandedSubs(groupKey) {
  const mods = modulesByGroup(groupKey)
  const all = []
  mods.forEach(m => {
    if (expandedSubs.value.has(m.key)) {
      all.push(...(expandedSubs.value.get(m.key) || []))
    }
  })
  return all
}

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

function modulesByGroup(gKey) {
  const g = NAV_GROUPS.find((x) => x.key === gKey)
  if (!g) return []
  const set = new Set(g.items)
  return NAV_MODULES.filter((m) => set.has(m.key))
}

function isActive(path) {
  return route.path === path || (path !== '/dashboard' && route.path.startsWith(path))
}

function goHome() { router.push('/dashboard') }

async function refreshHealth() {
  try {
    const r = await getHealth()
    const ok = r.status === 'ok' || r.status === 'running' || r.status === 'healthy'
    health.value = { status: ok ? 'ok' : 'down', label: ok ? '服务正常' : '服务异常' }
  } catch {
    health.value = { status: 'down', label: '连接失败' }
  }
}

// 暴露方法给父组件
defineExpose({ refreshHealth })

onMounted(() => {
  restoreNavState()
  refreshHealth()
})
</script>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  flex-shrink: 0;
  background: linear-gradient(180deg, #0f172a 0%, #111c34 100%);
  color: #cbd5e1;
  display: flex;
  flex-direction: column;
  transition: width var(--transition);
  position: relative;
  z-index: 10;
  box-shadow: 4px 0 24px rgba(15, 23, 42, 0.4);
}
.sidebar-collapsed .sidebar { width: 68px; }
.logo {
  height: var(--header-h);
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 0 18px;
  cursor: pointer;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.logo-mark {
  width: 34px; height: 34px; flex-shrink: 0; border-radius: 10px;
  background: linear-gradient(135deg, var(--brand-light), var(--accent));
  color: #fff; font-weight: 800; font-size: 18px; display: grid; place-items: center;
  box-shadow: 0 4px 14px rgba(99, 102, 241, 0.6);
}
.logo-title { font-size: 15px; font-weight: 700; color: #fff; white-space: nowrap; }
.logo-sub { font-size: 10px; color: #64748b; letter-spacing: 0.4px; white-space: nowrap; }
.nav-scroll { flex: 1; padding: 10px 12px; }
.nav { display: flex; flex-direction: column; gap: 4px; }

/* 分组标题栏 */
.nav-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px 6px;
  cursor: pointer;
  user-select: none;
  transition: opacity 0.2s;
}
.nav-group-header:hover { opacity: 0.9; }
.nav-group-header.collapsed .nav-group-label { opacity: 0.7; }
.nav-group-label {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: #64748b;
  font-weight: 600;
}
.nav-group-arrow {
  font-size: 12px;
  color: #475569;
  transition: transform 0.2s;
}

/* 分组内容过渡动画 */
.nav-expand-enter-active,
.nav-expand-leave-active {
  transition: all 0.25s ease;
  overflow: hidden;
}
.nav-expand-enter-from,
.nav-expand-leave-to {
  opacity: 0;
  max-height: 0;
}
.nav-expand-enter-to,
.nav-expand-leave-from {
  opacity: 1;
  max-height: 500px;
}

.nav-group-items {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex; align-items: center; gap: 12px; height: 44px; padding: 0 12px;
  border-radius: 11px; color: #94a3b8; font-size: 14px; font-weight: 500;
  transition: all 0.2s; position: relative; white-space: nowrap;
}
.nav-item:hover { background: rgba(255, 255, 255, 0.06); color: #e2e8f0; }
.nav-item.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.95), rgba(67, 56, 202, 0.95));
  color: #fff;
  box-shadow: 0 8px 22px rgba(79, 70, 229, 0.45);
}
.nav-bar {
  position: absolute; left: -12px; top: 50%; transform: translateY(-50%) scaleY(0);
  width: 4px; height: 22px; border-radius: 0 4px 4px 0; background: var(--accent);
  transition: transform 0.2s;
}
.nav-item.active .nav-bar { transform: translateY(-50%) scaleY(1); }

/* 一级图标背景 */
.nav-icon-wrap {
  width: 28px; height: 28px; border-radius: 8px;
  display: grid; place-items: center;
  flex-shrink: 0;
  transition: all 0.2s;
}
.nav-item.active .nav-icon-wrap {
  background: rgba(255, 255, 255, 0.2) !important;
  color: #fff !important;
}
.nav-icon { font-size: 16px; flex-shrink: 0; }

/* 二级展开箭头 */
.nav-sub-arrow {
  margin-left: auto;
  font-size: 12px;
  color: #64748b;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.15s;
}
.nav-sub-arrow:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #e2e8f0;
}

/* 二级子菜单 */
.nav-sub-expand-enter-active,
.nav-sub-expand-leave-active {
  transition: all 0.2s ease;
  overflow: hidden;
}
.nav-sub-expand-enter-from,
.nav-sub-expand-leave-to {
  opacity: 0;
  max-height: 0;
}
.nav-sub-expand-enter-to,
.nav-sub-expand-leave-from {
  opacity: 1;
  max-height: 200px;
}

.nav-sub-items {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding-left: 40px;
  margin: 2px 0 4px;
}
.nav-sub-item {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 12px;
  border-radius: 7px;
  font-size: 13px;
  color: #64748b;
  font-weight: 500;
  transition: all 0.15s;
  white-space: nowrap;
}
.nav-sub-item:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #cbd5e1;
}
.nav-sub-item.active {
  color: #a5b4fc;
  background: rgba(99, 102, 241, 0.12);
}
.nav-sub-dot {
  width: 5px; height: 5px; border-radius: 50%;
  background: #475569;
  flex-shrink: 0;
}
.nav-sub-item.active .nav-sub-dot {
  background: #6366f1;
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.2);
}
.nav-sub-label { flex: 1; }
.sidebar-footer {
  height: 50px; display: flex; align-items: center; justify-content: space-between;
  padding: 0 18px; border-top: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 12px; color: #64748b;
}
.health { display: flex; align-items: center; gap: 7px; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--warning); }
.dot.ok { background: var(--success); box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2); }
.dot.down { background: var(--danger); box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.2); }

/* 响应式 */
@media (max-width: 1024px) {
  .sidebar { width: 60px; }
  .nav-group { display: none; }
}
@media (max-width: 768px) {
  .sidebar {
    position: fixed;
    z-index: 200;
    transform: translateX(-100%);
    transition: transform 0.3s ease;
  }
  .sidebar.mobile-open { transform: translateX(0); }
}
</style>
