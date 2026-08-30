<template>
  <el-config-provider :locale="zhCn">
    <div class="app-shell" :class="{ 'sidebar-collapsed': collapsed, 'ai-fullscreen': isAIFullscreen }">
      <!-- 侧边栏 -->
      <TheSidebar
        ref="sidebarRef"
        :collapsed="collapsed"
        :is-a-i-fullscreen="isAIFullscreen"
      />

      <!-- 主区域 -->
      <div class="main">
        <!-- 顶栏 -->
        <TheTopbar
          ref="topbarRef"
          :collapsed="collapsed"
          :is-a-i-fullscreen="isAIFullscreen"
          @toggle-collapse="appStore.toggleSidebar()"
          @toggle-help="appStore.openHelpDrawer()"
          @refresh-health="refreshHealth"
          @toggle-theme="toggleTheme"
        />

        <!-- 多页签 -->
        <TabBar :is-a-i-fullscreen="isAIFullscreen" />

        <!-- 5 阶段全维流程条 -->
        <div v-if="showPipeline" class="phasebar-wrap">
          <PhasePipeline
            v-model="currentPhase"
            compact
            :show-progress="true"
            title="项目全维流程"
            @change="onPhaseChange"
          />
        </div>

        <!-- 内容 -->
        <main class="content" :class="{ 'content--withphase': showPipeline }">
          <router-view v-slot="{ Component }">
            <transition name="page-slide" mode="out-in">
              <component :is="Component" />
            </transition>
          </router-view>
        </main>
      </div>

      <!-- 全局帮助 Drawer -->
      <el-drawer v-model="helpDrawerOpen" title="键盘快捷键 & 快捷操作" direction="rtl" size="420px">
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
        <div class="help-footer">提示：在输入框/表格内编辑时，Ctrl+K / Ctrl+⇧+N 等全局快捷键自动暂停（避免误触）；按 Esc 取消当前编辑并返回全局模式。</div>
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
import { PROJECT_PHASES, QUICK_CREATE_COMMANDS, HOTKEY_GROUPS, NAV_MODULES } from '@/constants'
import { getHealth } from '@/api'
import { useGlobalShortcuts } from '@/globalShortcuts'
import { provideProjectContext, useProject } from '@/composables/projectContext.js'
import { useAppStore } from '@/stores/app.store'
import { useUiStore } from '@/stores/ui.store'
import TheSidebar from '@/components/layout/TheSidebar.vue'
import TheTopbar from '@/components/layout/TheTopbar.vue'
import TabBar from '@/components/layout/TabBar.vue'
import OnboardingGuide from '@/components/OnboardingGuide.vue'
import PhasePipeline from '@/components/PhasePipeline.vue'

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

// 5 阶段流程条显示判断
const PIPELINE_ROUTES = new Set([
  '/dashboard', '/projects', '/workbench', '/tasks', '/resources',
  '/expert-center', '/expert-enterprise', '/expert-orchestrator',
  '/ai', '/graph', '/mox-fusion', '/caomei', '/knowledge-base', '/llm-config',
  '/workflow', '/automation', '/plugins', '/mcp',
  '/algolab', '/infinite-optimizer', '/botCenter', '/browser',
  '/monitor'
])
const showPipeline = computed(() => PIPELINE_ROUTES.has(route.path) || route.path.startsWith('/market'))

// 根据路由推断阶段
const inferredPhase = computed(() => {
  const mapping = {
    '/caomei': 'requirement', '/ai': 'requirement',
    '/knowledge-base': 'requirement', '/llm-config': 'requirement',
    '/graph': 'architecture', '/mox-fusion': 'architecture',
    '/expert-center': 'architecture', '/expert-enterprise': 'architecture',
    '/expert-orchestrator': 'architecture',
    '/operators': 'develop', '/workflow': 'develop', '/plugins': 'develop',
    '/mcp': 'develop', '/automation': 'develop', '/browser': 'develop',
    '/algolab': 'develop', '/infinite-optimizer': 'develop',
    '/botCenter': 'develop', '/market': 'develop',
    '/monitor': 'release', '/docs': 'release', '/admin': 'release'
  }
  return mapping[route.path] || 'requirement'
})
const currentPhase = computed({
  get: () => uiStore.currentPhase,
  set: (v) => { uiStore.setPhase(v) }
})
watch(inferredPhase, (p) => { currentPhase.value = p })

// 允许子视图覆盖阶段
window.addEventListener?.('mox:set-phase', (e) => {
  const key = e?.detail?.key
  if (key && PROJECT_PHASES.some((p) => p.key === key)) currentPhase.value = key
})

function onPhaseChange(p) {
  const ph = PROJECT_PHASES.find((x) => x.key === p.key)
  if (!ph) return
  const defaults = {
    requirement: '/dashboard',
    architecture: '/graph',
    develop: '/operators',
    release: '/monitor'
  }
  const target = defaults[ph.key]
  if (target && target !== route.path) router.push(target)
}

// 健康检查
async function refreshHealth() {
  sidebarRef.value?.refreshHealth?.()
}

// 主题切换
function toggleTheme() {
  appStore.toggleTheme()
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

<style scoped>
.app-shell {
  display: flex;
  height: 100%;
  overflow: hidden;
}
.sidebar-collapsed .sidebar { width: 68px; }
.main { flex: 1; display: flex; flex-direction: column; min-width: 0; }

/* 阶段流程条 */
.phasebar-wrap {
  padding: 0 22px 0;
  background: transparent;
  flex-shrink: 0;
}
.content--withphase { padding-top: 10px; }
.content {
  flex: 1; overflow-y: auto; padding: 22px;
  background:
    radial-gradient(1200px 400px at 100% 0%, rgba(99, 102, 241, 0.05), transparent),
    radial-gradient(900px 360px at 0% 100%, rgba(6, 182, 212, 0.05), transparent),
    var(--bg-page);
}
/* AI 全屏模式 */
.ai-fullscreen .main { margin-left: 0 !important; }
.ai-fullscreen .content {
  padding: 0;
  background: var(--bg-deep-sky, #f8fafc);
}
.ai-fullscreen .content > * { height: 100%; }

/* 页面切换过渡动画 */
.page-slide-enter-active,
.page-slide-leave-active {
  transition: all 0.25s cubic-bezier(0.22, 1, 0.36, 1);
}
.page-slide-enter-from {
  opacity: 0;
  transform: translateY(12px);
}
.page-slide-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
.page-slide-leave-active {
  position: absolute;
  width: 100%;
}

/* 全局帮助 Drawer 样式 */
.help-group { margin-bottom: 18px; }
.help-group-title {
  font-size: 12px; letter-spacing: 0.1em; text-transform: uppercase;
  color: var(--text-3); margin: 0 0 8px;
}
.help-list { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 8px; }
.help-list li {
  display: flex; align-items: center; gap: 12px;
  padding: 8px 10px; border-radius: 8px; background: var(--bg-page);
}
.help-keys { display: inline-flex; align-items: center; gap: 4px; flex-shrink: 0; }
.help-desc { color: var(--text-1); font-size: 13px; flex: 1; }
.help-desc-sub { color: var(--text-3); font-size: 12px; margin-left: 6px; }
.qc-icon-help { color: var(--brand-dark); font-size: 14px; }
.help-footer {
  margin-top: 20px; padding: 12px 14px; border-radius: 10px;
  background: var(--brand-soft); color: var(--brand-dark);
  font-size: 12px; line-height: 1.7;
}

.kbd {
  display: inline-block; padding: 1px 6px; font-size: 11px;
  background: rgba(148, 163, 184, 0.14); color: var(--text-2);
  border: 1px solid var(--border); border-radius: 5px; line-height: 1.6;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}
</style>
