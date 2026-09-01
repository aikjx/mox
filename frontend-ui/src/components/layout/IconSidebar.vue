<template>
  <aside class="icon-sidebar">
    <!-- Logo -->
    <div class="icon-logo" @click="goHome" title="璇玑系统">
      <span class="icon-logo-text">智</span>
    </div>

    <!-- 导航分组 -->
    <div class="icon-nav-scroll">
      <template v-for="group in ICON_NAV_GROUPS" :key="group.key">
        <!-- 分组标签（竖排） -->
        <div class="icon-group-label">{{ group.label }}</div>

        <!-- 组内图标 -->
        <div class="icon-group-items">
          <template v-for="item in group.items" :key="item.key">
            <!-- 非底部项正常渲染 -->
            <el-tooltip
              v-if="!item.bottom"
              :content="item.label"
              placement="right"
              :show-after="300"
            >
              <router-link
                :to="item.path"
                class="icon-nav-item"
                :class="{ active: isActive(item.path) }"
              >
                <span class="icon-nav-emoji">{{ item.icon }}</span>
                <span v-if="item.badge" class="icon-nav-badge">{{ item.badge }}</span>
              </router-link>
            </el-tooltip>
          </template>
        </div>
      </template>
    </div>

    <!-- 底部：健康状态 + 系统设置 -->
    <div class="icon-sidebar-bottom">
      <div class="icon-health-dot" :class="health.status" :title="health.label"></div>
      <el-tooltip content="系统设置" placement="right" :show-after="300">
        <router-link
          to="/admin"
          class="icon-nav-item"
          :class="{ active: isActive('/admin') }"
        >
          <span class="icon-nav-emoji">⚡</span>
        </router-link>
      </el-tooltip>
    </div>
  </aside>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ICON_NAV_GROUPS } from '@/constants'
import { getHealth } from '@/api'

const route = useRoute()
const router = useRouter()

const health = ref({ status: 'pending', label: '连接中…' })

function isActive(path) {
  if (path === '/dashboard') return route.path === '/dashboard'
  return route.path.startsWith(path)
}

function goHome() {
  router.push('/dashboard')
}

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
.icon-sidebar {
  width: 64px;
  flex-shrink: 0;
  background: var(--bg-secondary, #161821);
  border-right: 1px solid var(--border, #2d3148);
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
  z-index: 10;
}

/* Logo */
.icon-logo {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: linear-gradient(135deg, #6366f1, #a855f7);
  display: grid;
  place-items: center;
  margin-top: 12px;
  margin-bottom: 8px;
  cursor: pointer;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  flex-shrink: 0;
  box-shadow: 0 4px 12px rgba(99,102,241,.35);
}
.icon-logo:hover {
  transform: scale(1.05);
  box-shadow: 0 6px 16px rgba(99,102,241,.5);
}
.icon-logo-text {
  color: #fff;
  font-weight: 700;
  font-size: 18px;
  line-height: 1;
}

/* 导航滚动区 */
.icon-nav-scroll {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  overflow-y: auto;
  overflow-x: hidden;
  width: 100%;
  padding: 4px 0;
}
.icon-nav-scroll::-webkit-scrollbar {
  display: none;
}

/* 分组标签（竖排） */
.icon-group-label {
  font-size: 9px;
  color: var(--text-muted, #6b7280);
  writing-mode: vertical-rl;
  transform: rotate(180deg);
  letter-spacing: 2px;
  margin: 10px 0 4px;
  font-weight: 500;
  user-select: none;
}

.icon-group-items {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

/* 导航图标项 */
.icon-nav-item {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  cursor: pointer;
  color: var(--text-secondary, #9aa0b4);
  transition: all 0.15s ease;
  position: relative;
  text-decoration: none;
}
.icon-nav-item:hover {
  background: var(--bg-hover, #2a2f45);
  color: var(--text-primary, #e8eaed);
}
.icon-nav-item.active {
  background: var(--accent-dim, rgba(99,102,241,.15));
  color: var(--accent-light, #818cf8);
}
.icon-nav-item.active::before {
  content: '';
  position: absolute;
  left: -10px;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 24px;
  border-radius: 0 3px 3px 0;
  background: var(--accent, #6366f1);
}

.icon-nav-emoji {
  font-size: 20px;
  line-height: 1;
  display: block;
}

/* 角标 */
.icon-nav-badge {
  position: absolute;
  top: 4px;
  right: 2px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: 8px;
  background: var(--danger, #ef4444);
  color: #fff;
  font-size: 10px;
  font-weight: 600;
  display: grid;
  place-items: center;
  line-height: 1;
}

/* 底部 */
.icon-sidebar-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 0 12px;
  flex-shrink: 0;
  border-top: 1px solid var(--border, #2d3148);
  width: 100%;
  margin-top: auto;
}

.icon-health-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--warning, #f59e0b);
  margin-bottom: 2px;
}
.icon-health-dot.ok {
  background: var(--success, #10b981);
  box-shadow: 0 0 0 3px rgba(16,185,129,.2);
}
.icon-health-dot.down {
  background: var(--danger, #ef4444);
  box-shadow: 0 0 0 3px rgba(239,68,68,.2);
}
.icon-health-dot.pending {
  background: var(--warning, #f59e0b);
}
</style>
