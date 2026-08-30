<template>
  <aside class="sidebar" :class="{ collapsed: collapsed }">
    <!-- Logo 区 -->
    <div class="sidebar-brand" @click="goHome">
      <div class="brand-mark">
        <span class="brand-mark-inner">玄</span>
      </div>
      <transition name="fade-slide">
        <div v-show="!collapsed" class="brand-text">
          <div class="brand-title">璇玑系统</div>
          <div class="brand-sub">Mox Graph System</div>
        </div>
      </transition>
    </div>

    <!-- 导航 -->
    <el-scrollbar class="sidebar-nav-scroll">
      <nav class="sidebar-nav">
        <template v-for="g in NAV_GROUPS" :key="g.key">
          <!-- 分组标题 -->
          <div class="nav-section" v-show="!isGroupCollapsed(g.key) || collapsed">
            <span
              v-if="!collapsed"
              class="nav-section-label"
            >{{ g.label }}</span>
          </div>

          <!-- 分组内导航项 -->
          <div class="nav-items" :class="{ 'is-collapsed-group': isGroupCollapsed(g.key) && !collapsed }">
            <template v-for="m in modulesByGroup(g.key)" :key="m.key">
              <el-tooltip
                :content="m.label"
                placement="right"
                :disabled="!collapsed"
                :show-after="400"
              >
                <router-link
                  :to="m.path"
                  class="nav-item"
                  :class="{ active: isActive(m.path) }"
                >
                  <span class="nav-item-indicator" :class="{ active: isActive(m.path) }"></span>
                  <el-icon class="nav-item-icon"><component :is="m.icon" /></el-icon>
                  <span v-show="!collapsed" class="nav-item-label">{{ m.label }}</span>
                </router-link>
              </el-tooltip>
            </template>
          </div>
        </template>
      </nav>
    </el-scrollbar>

    <!-- 底部：健康状态 + 折叠 -->
    <div class="sidebar-footer">
      <div v-show="!collapsed" class="footer-health">
        <span class="health-dot" :class="health.status"></span>
        <span class="health-text">{{ health.label }}</span>
      </div>
      <button class="footer-collapse" @click="$emit('toggle-collapse')" :title="collapsed ? '展开侧边栏' : '收起侧边栏'">
        <el-icon>
          <component :is="collapsed ? 'ArrowRight' : 'ArrowLeft'" />
        </el-icon>
      </button>
    </div>
  </aside>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowLeft, ArrowRight } from '@element-plus/icons-vue'
import { NAV_MODULES, APP_VERSION, NAV_GROUPS } from '@/constants'
import { getHealth } from '@/api'

defineProps({
  collapsed: { type: Boolean, default: false },
  isAIFullscreen: { type: Boolean, default: false }
})

defineEmits(['toggle-collapse'])

const route = useRoute()
const router = useRouter()

const health = ref({ status: 'pending', label: '连接中…' })

/* ===== 导航分组折叠 ===== */
const collapsedGroups = ref(new Set())

function isGroupCollapsed(key) {
  return collapsedGroups.value.has(key)
}

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

defineExpose({ refreshHealth })

onMounted(() => {
  refreshHealth()
})
</script>

<style scoped>
.sidebar {
  width: var(--sidebar-width, 240px);
  flex-shrink: 0;
  background: var(--sidebar-bg, #0f172a);
  border-right: 1px solid var(--sidebar-border, rgba(255, 255, 255, 0.06));
  display: flex;
  flex-direction: column;
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  z-index: 10;
}

.sidebar.collapsed {
  width: 68px;
}

/* ===== Brand / Logo ===== */
.sidebar-brand {
  height: var(--header-h, 56px);
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 20px;
  cursor: pointer;
  border-bottom: 1px solid var(--sidebar-border, rgba(255, 255, 255, 0.06));
  flex-shrink: 0;
}

.brand-mark {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: linear-gradient(135deg, var(--brand, #6366f1), var(--accent, #06b6d4));
  display: grid;
  place-items: center;
  flex-shrink: 0;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.4);
}

.brand-mark-inner {
  color: #fff;
  font-weight: 800;
  font-size: 18px;
  letter-spacing: -0.5px;
}

.brand-text {
  min-width: 0;
  overflow: hidden;
}

.brand-title {
  font-size: 15px;
  font-weight: 700;
  color: #f1f5f9;
  line-height: 1.2;
}

.brand-sub {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
  letter-spacing: 0.3px;
}

/* ===== Nav Scroll ===== */
.sidebar-nav-scroll {
  flex: 1;
  padding: 8px 10px;
}

.sidebar-nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* 分组标题 */
.nav-section {
  padding: 16px 14px 6px;
}

.nav-section-label {
  font-size: 10px;
  font-weight: 600;
  color: #475569;
  text-transform: uppercase;
  letter-spacing: 0.12em;
}

.nav-items {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-items.is-collapsed-group {
  display: none;
}

/* 导航项 */
.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 40px;
  padding: 0 12px;
  border-radius: 9px;
  color: #94a3b8;
  font-size: 13.5px;
  font-weight: 500;
  transition: all 0.15s ease;
  position: relative;
  white-space: nowrap;
  text-decoration: none;
}

.nav-item:hover {
  background: rgba(255, 255, 255, 0.05);
  color: #e2e8f0;
}

.nav-item.active {
  background: rgba(99, 102, 241, 0.12);
  color: #a5b4fc;
}

.nav-item-indicator {
  position: absolute;
  left: 0;
  top: 50%;
  transform: translateY(-50%) scaleY(0);
  width: 3px;
  height: 18px;
  border-radius: 0 3px 3px 0;
  background: var(--brand, #6366f1);
  transition: transform 0.2s ease;
}

.nav-item-indicator.active {
  transform: translateY(-50%) scaleY(1);
}

.nav-item-icon {
  font-size: 18px;
  flex-shrink: 0;
  width: 20px;
  height: 20px;
}

.nav-item.active .nav-item-icon {
  color: #818cf8;
}

.nav-item-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ===== Footer ===== */
.sidebar-footer {
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  border-top: 1px solid var(--sidebar-border, rgba(255, 255, 255, 0.06));
  flex-shrink: 0;
}

.footer-health {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #64748b;
}

.health-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f59e0b;
}

.health-dot.ok {
  background: #10b981;
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2);
}

.health-dot.down {
  background: #ef4444;
  box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.2);
}

.footer-collapse {
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: #64748b;
  cursor: pointer;
  display: grid;
  place-items: center;
  transition: all 0.15s;
  margin-left: auto;
}

.footer-collapse:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #e2e8f0;
}

.collapsed .footer-collapse {
  margin: 0 auto;
}

/* ===== Transitions ===== */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.2s ease;
  opacity: 1;
  transform: translateX(0);
}

.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-8px);
}

/* ===== Responsive ===== */
@media (max-width: 768px) {
  .sidebar {
    position: fixed;
    z-index: 200;
    transform: translateX(-100%);
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }
  .sidebar.mobile-open {
    transform: translateX(0);
  }
}
</style>
