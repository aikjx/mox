<template>
  <el-config-provider :locale="zhCn">
    <div class="app-shell" :class="{ 'sidebar-collapsed': collapsed }">
      <!-- 侧边栏 -->
      <aside class="sidebar">
        <div class="logo" @click="goHome">
          <div class="logo-mark">玄</div>
          <transition name="fade">
            <div v-show="!collapsed" class="logo-text">
              <div class="logo-title">璇玑系统</div>
              <div class="logo-sub">Xuanji Graph System</div>
            </div>
          </transition>
        </div>

        <el-scrollbar class="nav-scroll">
          <nav class="nav">
            <router-link
              v-for="m in NAV_MODULES"
              :key="m.key"
              :to="m.path"
              class="nav-item"
              :class="{ active: isActive(m.path) }"
            >
              <span class="nav-bar"></span>
              <el-icon class="nav-icon"><component :is="m.icon" /></el-icon>
              <span v-show="!collapsed" class="nav-label">{{ m.label }}</span>
            </router-link>
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

      <!-- 主区域 -->
      <div class="main">
        <!-- 顶栏 -->
        <header class="topbar">
          <div class="topbar-left">
            <el-icon class="collapse-btn" @click="collapsed = !collapsed">
              <Fold v-if="!collapsed" />
              <Expand v-else />
            </el-icon>
            <el-breadcrumb separator="/">
              <el-breadcrumb-item :to="{ path: '/dashboard' }">首页</el-breadcrumb-item>
              <el-breadcrumb-item v-for="b in crumbs" :key="b.path">{{ b.label }}</el-breadcrumb-item>
            </el-breadcrumb>
          </div>

          <div class="topbar-right">
            <el-tooltip content="刷新系统状态" placement="bottom">
              <div class="top-action" @click="refreshHealth">
                <el-icon><Refresh /></el-icon>
              </div>
            </el-tooltip>
            <el-tooltip content="API 文档" placement="bottom">
              <div class="top-action" @click="$router.push('/docs')">
                <el-icon><Document /></el-icon>
              </div>
            </el-tooltip>
            <el-dropdown>
              <div class="user">
                <el-avatar :size="32" style="background: linear-gradient(135deg,#6366f1,#06b6d4)">A</el-avatar>
                <span class="user-name">管理员</span>
                <el-icon><ArrowDown /></el-icon>
              </div>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item @click="$router.push('/monitor')">
                    <el-icon><Setting /></el-icon> 系统设置
                  </el-dropdown-item>
                  <el-dropdown-item divided @click="$router.push('/docs')">
                    <el-icon><Document /></el-icon> API 文档
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </header>

        <!-- 多页签 -->
        <div class="tabs">
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

        <!-- 内容 -->
        <main class="content">
          <router-view v-slot="{ Component }">
            <transition name="fade" mode="out-in">
              <component :is="Component" />
            </transition>
          </router-view>
        </main>
      </div>
    </div>
  </el-config-provider>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import { NAV_MODULES, APP_VERSION } from '@/types'
import { getHealth } from '@/api'

const route = useRoute()
const router = useRouter()

const collapsed = ref(false)
const health = ref({ status: 'pending', label: '连接中…' })

const currentPath = computed(() => route.path)
const crumbs = computed(() => {
  const m = NAV_MODULES.find((x) => route.path.startsWith(x.path))
  return m ? [{ path: m.path, label: m.label }] : []
})
const tabs = computed(() => {
  const list = [{ path: '/dashboard', label: '工作台', closable: false }]
  const m = NAV_MODULES.find((x) => x.path === route.path)
  if (m && m.path !== '/dashboard') list.push({ path: m.path, label: m.label, closable: true })
  return list
})

function isActive(path) {
  return route.path === path || (path !== '/dashboard' && route.path.startsWith(path))
}
function closeTab(path) {
  if (route.path === path) router.push('/dashboard')
}
function goHome() {
  router.push('/dashboard')
}
async function refreshHealth() {
  try {
    const r = await getHealth()
    const ok = r.status === 'ok' || r.status === 'running'
    health.value = { status: ok ? 'ok' : 'down', label: ok ? '服务正常' : '服务异常' }
  } catch {
    health.value = { status: 'down', label: '连接失败' }
  }
}

onMounted(refreshHealth)
watch(() => route.path, refreshHealth)
window.setInterval(refreshHealth, 30000)
</script>

<style scoped>
.app-shell {
  display: flex;
  height: 100%;
  overflow: hidden;
}
.sidebar {
  width: var(--sidebar-w);
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
.app-shell.sidebar-collapsed .sidebar {
  width: 68px;
}
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
  width: 34px;
  height: 34px;
  flex-shrink: 0;
  border-radius: 10px;
  background: linear-gradient(135deg, var(--brand-light), var(--accent));
  color: #fff;
  font-weight: 800;
  font-size: 18px;
  display: grid;
  place-items: center;
  box-shadow: 0 4px 14px rgba(99, 102, 241, 0.6);
}
.logo-title {
  font-size: 15px;
  font-weight: 700;
  color: #fff;
  white-space: nowrap;
}
.logo-sub {
  font-size: 10px;
  color: #64748b;
  letter-spacing: 0.4px;
  white-space: nowrap;
}
.nav-scroll {
  flex: 1;
  padding: 14px 12px;
}
.nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 46px;
  padding: 0 14px;
  border-radius: 11px;
  color: #94a3b8;
  font-size: 14px;
  font-weight: 500;
  transition: all var(--transition);
  position: relative;
  white-space: nowrap;
}
.nav-item:hover {
  background: rgba(255, 255, 255, 0.06);
  color: #e2e8f0;
}
.nav-item.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.95), rgba(67, 56, 202, 0.95));
  color: #fff;
  box-shadow: 0 8px 22px rgba(79, 70, 229, 0.45);
}
.nav-bar {
  position: absolute;
  left: -12px;
  top: 50%;
  transform: translateY(-50%) scaleY(0);
  width: 4px;
  height: 22px;
  border-radius: 0 4px 4px 0;
  background: var(--accent);
  transition: transform var(--transition);
}
.nav-item.active .nav-bar {
  transform: translateY(-50%) scaleY(1);
}
.nav-icon {
  font-size: 19px;
  flex-shrink: 0;
}
.sidebar-footer {
  height: 50px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 18px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 12px;
  color: #64748b;
}
.health {
  display: flex;
  align-items: center;
  gap: 7px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--warning);
}
.dot.ok {
  background: var(--success);
  box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2);
}
.dot.down {
  background: var(--danger);
  box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.2);
}

.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.topbar {
  height: var(--header-h);
  flex-shrink: 0;
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  box-shadow: var(--shadow-sm);
  z-index: 5;
}
.topbar-left {
  display: flex;
  align-items: center;
  gap: 16px;
}
.collapse-btn {
  font-size: 20px;
  cursor: pointer;
  color: var(--text-2);
}
.collapse-btn:hover {
  color: var(--brand);
}
.topbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
.top-action {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  cursor: pointer;
  color: var(--text-2);
  font-size: 18px;
  transition: all var(--transition);
}
.top-action:hover {
  background: var(--brand-soft);
  color: var(--brand);
}
.user {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 10px 4px 4px;
  border-radius: 999px;
  cursor: pointer;
  transition: background var(--transition);
}
.user:hover {
  background: var(--bg-page);
}
.user-name {
  font-size: 14px;
  font-weight: 600;
}
.tabs {
  height: 42px;
  flex-shrink: 0;
  background: #fff;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 16px;
  overflow-x: auto;
}
.tab {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 12px;
  border-radius: 8px;
  font-size: 13px;
  color: var(--text-2);
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition);
}
.tab:hover {
  background: var(--bg-page);
}
.tab.active {
  background: var(--brand-soft);
  color: var(--brand-dark);
  font-weight: 600;
}
.tab-close {
  font-size: 12px;
  border-radius: 50%;
}
.tab-close:hover {
  background: #cbd5e1;
  color: #fff;
}
.content {
  flex: 1;
  overflow-y: auto;
  padding: 22px;
  background:
    radial-gradient(1200px 400px at 100% 0%, rgba(99, 102, 241, 0.05), transparent),
    radial-gradient(900px 360px at 0% 100%, rgba(6, 182, 212, 0.05), transparent),
    var(--bg-page);
}
</style>
