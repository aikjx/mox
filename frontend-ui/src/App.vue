<template>
  <el-config-provider :locale="zhCn">
    <div class="app-shell" :class="{ 'sidebar-collapsed': collapsed, 'ai-fullscreen': isAIFullscreen }">
      <!-- 侧边栏 -->
      <TheSidebar
        ref="sidebarRef"
        :collapsed="collapsed"
        :is-a-i-fullscreen="isAIFullscreen"
        @toggle-collapse="appStore.toggleSidebar()"
      />

      <!-- 主区域 -->
      <div class="app-main">
        <!-- 顶栏 -->
        <TheTopbar
          ref="topbarRef"
          :collapsed="collapsed"
          :is-a-i-fullscreen="isAIFullscreen"
          @toggle-collapse="appStore.toggleSidebar()"
          @toggle-help="appStore.openHelpDrawer()"
          @refresh-health="refreshHealth"
        />

        <!-- 页面内容 -->
        <main class="app-content">
          <router-view v-slot="{ Component }">
            <transition name="page-fade" mode="out-in">
              <component :is="Component" />
            </transition>
          </router-view>
        </main>
      </div>

      <!-- 全局帮助 Drawer -->
      <el-drawer
        v-model="helpDrawerOpen"
        title="键盘快捷键 & 快捷操作"
        direction="rtl"
        size="420px"
      >
        <div v-for="hg in HOTKEY_GROUPS" :key="hg.group" class="help-group">
          <h4 class="help-group-title">{{ hg.group }}</h4>
          <ul class="help-list">
            <li v-for="(it, idx) in hg.items" :key="idx">
              <span class="help-keys">
                <kbd v-for="k in it.keys" :key="k" class="kbd">{{ k }}</kbd>
              </span>
              <span class="help-desc">{{ it.desc }}</span>
            </li>
          </ul>
        </div>
        <div class="help-group">
          <h4 class="help-group-title">顶栏快捷新建</h4>
          <ul class="help-list">
            <li v-for="q in QUICK_CREATE_COMMANDS" :key="q.key">
              <span class="help-keys">
                <el-icon class="qc-icon-help"><component :is="q.icon" /></el-icon>
              </span>
              <span class="help-desc">{{ q.label }}<span v-if="q.tip" class="help-desc-sub">（{{ q.tip }}）</span></span>
            </li>
          </ul>
        </div>
        <div class="help-footer">
          提示：在输入框/表格内编辑时，全局快捷键自动暂停（避免误触）；按 Esc 取消当前编辑并返回全局模式。
        </div>
      </el-drawer>

      <!-- 新手引导 -->
      <OnboardingGuide v-model="onboardingVisible" />
    </div>
  </el-config-provider>
</template>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import { QUICK_CREATE_COMMANDS, HOTKEY_GROUPS, NAV_MODULES } from '@/constants'
import { getHealth } from '@/api'
import { useGlobalShortcuts } from '@/globalShortcuts'
import { provideProjectContext, useProject } from '@/composables/projectContext.js'
import { useAppStore } from '@/stores/app.store'
import { useUiStore } from '@/stores/ui.store'
import TheSidebar from '@/components/layout/TheSidebar.vue'
import TheTopbar from '@/components/layout/TheTopbar.vue'
import OnboardingGuide from '@/components/OnboardingGuide.vue'

// 全局项目上下文注入
provideProjectContext()
const { onChange: onProjectChange } = useProject()

const appStore = useAppStore()
const uiStore = useUiStore()

const route = useRoute()
const router = useRouter()

const sidebarRef = ref(null)
const topbarRef = ref(null)
const onboardingVisible = ref(false)
let disposeShortcuts = null

// 从 store 获取响应式状态
const collapsed = computed(() => appStore.sidebarCollapsed)
const helpDrawerOpen = computed({
  get: () => appStore.helpDrawerOpen,
  set: (v) => { appStore.helpDrawerOpen = v }
})
const isAIFullscreen = computed(() => uiStore.aiFullscreen)

// 健康检查
function refreshHealth() {
  sidebarRef.value?.refreshHealth?.()
}

// 全局快捷键
onMounted(() => {
  appStore.initTheme()
  disposeShortcuts = useGlobalShortcuts({
    focusSearch: () => topbarRef.value?.focusSearch?.(),
    toggleHelpDrawer: () => { helpDrawerOpen.value = !helpDrawerOpen.value },
    navModules: NAV_MODULES.slice(0, 9),
    pushRoute: (p) => router.push(p)
  })

  // Query 驱动无状态化
  if (route.path === '/market' && route.query?.action === 'upload') {
    window.dispatchEvent(new CustomEvent('mox:open-market-upload'))
  }
})

watch(() => route.path, () => {
  refreshHealth()
  if (route.path === '/market' && route.query?.action === 'upload') {
    window.dispatchEvent(new CustomEvent('mox:open-market-upload'))
  }
  if (route.path === '/tasks' && route.query?.action === 'create') {
    window.dispatchEvent(new CustomEvent('mox:open-create-task'))
  }
})

let healthTimer = window.setInterval(refreshHealth, 30000)

onBeforeUnmount(() => {
  if (disposeShortcuts) disposeShortcuts()
  if (healthTimer) window.clearInterval(healthTimer)
})
</script>

<style>
/* ===== CSS 变量 · 设计令牌 ===== */
:root {
  --sidebar-width: 240px;
  --header-h: 56px;
  --content-pad: 20px;
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.06), 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.08), 0 2px 4px rgba(0, 0, 0, 0.04);
  --shadow-lg: 0 12px 32px rgba(0, 0, 0, 0.12), 0 4px 8px rgba(0, 0, 0, 0.06);
  --transition: 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}
</style>

<style scoped>
.app-shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: var(--bg-page, #f8fafc);
}

/* ===== Main Area ===== */
.app-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  position: relative;
}

.app-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--content-pad, 20px);
  -webkit-overflow-scrolling: touch;
  scrollbar-width: thin;
  scrollbar-color: var(--border, #e2e8f0) transparent;
}

.app-content::-webkit-scrollbar {
  width: 6px;
}

.app-content::-webkit-scrollbar-track {
  background: transparent;
}

.app-content::-webkit-scrollbar-thumb {
  background: var(--border, #e2e8f0);
  border-radius: 3px;
}

.app-content::-webkit-scrollbar-thumb:hover {
  background: var(--text-3, #94a3b8);
}

/* AI 全屏模式 */
.ai-fullscreen .app-main {
  margin-left: 0 !important;
}

.ai-fullscreen .app-content {
  padding: 0;
}

.ai-fullscreen .app-content > * {
  height: 100%;
}

/* ===== 页面过渡 ===== */
.page-fade-enter-active,
.page-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.25s cubic-bezier(0.22, 1, 0.36, 1);
}

.page-fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}

.page-fade-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.page-fade-leave-active {
  position: absolute;
  width: 100%;
}

/* ===== 帮助 Drawer ===== */
.help-group {
  margin-bottom: 18px;
}

.help-group-title {
  font-size: 11px;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--text-3, #94a3b8);
  margin: 0 0 8px;
  font-weight: 600;
}

.help-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.help-list li {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--bg-page, #f8fafc);
}

.help-keys {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.help-desc {
  color: var(--text-1, #0f172a);
  font-size: 13px;
  flex: 1;
}

.help-desc-sub {
  color: var(--text-3, #94a3b8);
  font-size: 12px;
  margin-left: 6px;
}

.qc-icon-help {
  color: var(--brand-dark, #4338ca);
  font-size: 14px;
}

.help-footer {
  margin-top: 20px;
  padding: 12px 14px;
  border-radius: 10px;
  background: var(--brand-soft, #eef2ff);
  color: var(--brand-dark, #4338ca);
  font-size: 12px;
  line-height: 1.7;
}

.kbd {
  display: inline-block;
  padding: 2px 8px;
  font-size: 11px;
  background: #fff;
  color: var(--text-2, #64748b);
  border: 1px solid var(--border, #e2e8f0);
  border-bottom-width: 2px;
  border-radius: 5px;
  line-height: 1.4;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-weight: 500;
}

/* ===== 侧边栏折叠时的主区域 ===== */
.sidebar-collapsed .app-main {
  /* 侧边栏宽度由组件自身控制，这里不需要额外调整 */
}

/* ===== 响应式 ===== */
@media (max-width: 1024px) {
  :root {
    --content-pad: 16px;
  }
}

@media (max-width: 768px) {
  :root {
    --content-pad: 12px;
  }

  .app-shell {
    flex-direction: column;
  }
}
</style>
