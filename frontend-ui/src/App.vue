<template>
  <el-config-provider :locale="zhCn">
    <div class="app-shell" :class="{ 'sidebar-collapsed': collapsed, 'ai-fullscreen': isAIFullscreen }">
      <!-- 侧边栏 -->
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

        <!-- 侧边栏导航（精简后约 10 项 · 3 大域） -->


        <el-scrollbar class="nav-scroll">
          <nav class="nav">
            <template v-for="g in NAV_GROUPS" :key="g.key">
              <div class="nav-group" v-show="!collapsed">{{ g.label }}</div>
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
                  <el-icon class="nav-icon"><component :is="m.icon" /></el-icon>
                  <span v-show="!collapsed" class="nav-label">{{ m.label }}</span>
                </router-link>
              </el-tooltip>
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

      <!-- 主区域 -->
      <div class="main">
        <!-- 顶栏 -->
        <header class="topbar" v-if="!isAIFullscreen">
          <div class="topbar-left">
            <el-icon class="collapse-btn" @click="collapsed = !collapsed">
              <Fold v-if="!collapsed" />
              <Expand v-else />
            </el-icon>

            <!-- 项目切换器（顶栏核心位置 · 以项目为根） -->
            <ProjectPicker variant="top" class="topbar-project-picker" />

            <el-breadcrumb separator="/" class="crumb-nav">
              <el-breadcrumb-item :to="{ path: '/dashboard' }">
                <el-icon style="margin-right:4px"><HomeFilled /></el-icon>工作台
              </el-breadcrumb-item>
              <el-breadcrumb-item v-for="b in crumbs" :key="b.path">
                <el-dropdown trigger="click" @command="(p) => router.push(p)" class="crumb-dropdown">
                  <span class="crumb-clickable">{{ b.label }}<el-icon style="margin-left:2px"><ArrowDown /></el-icon></span>
                  <template #dropdown>
                    <el-dropdown-menu>
                      <el-dropdown-item
                        v-for="sib in siblingModules(b.key)"
                        :key="sib.path"
                        :command="sib.path"
                        :class="{ 'is-active': sib.path === b.path }"
                      >
                        <el-icon style="margin-right:6px;color:var(--brand-dark)"><component :is="sib.icon" /></el-icon>
                        {{ sib.label }}
                      </el-dropdown-item>
                    </el-dropdown-menu>
                  </template>
                </el-dropdown>
              </el-breadcrumb-item>
              <el-breadcrumb-item v-if="crumbsExtra" class="crumb-extra">{{ crumbsExtra }}</el-breadcrumb-item>
            </el-breadcrumb>
          </div>

          <div class="topbar-right">
            <!-- 全局搜索输入框（企业级「命令面板」入口，Ctrl+K / Ctrl+Shift+P 聚焦） -->
            <div class="global-search" :class="{ focused: searchFocused }">
              <el-icon class="search-icon"><Search /></el-icon>
              <input
                ref="searchInputRef"
                v-model="searchText"
                class="search-input"
                placeholder="搜索模块 / 任务 / 算子…（Ctrl/⌘ + K）"
                @focus="onSearchFocus"
                @blur="onSearchBlur"
                @input="onSearchInput"
                @keydown.enter="doGlobalSearch"
                @keydown.esc="closeSearchPanel"
                @keydown.down.prevent="selectNextCmd"
                @keydown.up.prevent="selectPrevCmd"
              />
              <kbd v-if="!searchFocused" class="kbd">Ctrl + K</kbd>

              <!-- 命令面板下拉 -->
              <div v-if="showCmdPanel" class="cmd-panel" @mousedown.prevent>
                <!-- 分组：快速操作（无输入时显示） -->
                <div v-if="!searchText" class="cmd-section">
                  <div class="cmd-section-title">⚡ 快速操作</div>
                  <div
                    v-for="(cmd, i) in quickCommands"
                    :key="'q-'+i"
                    class="cmd-item"
                    :class="{ active: cmdIdx === i }"
                    @click="executeCmd(cmd)"
                    @mouseenter="cmdIdx = i"
                  >
                    <div class="cmd-icon" :style="{ background: cmd.bg, color: cmd.color }">
                      <el-icon><component :is="cmd.icon" /></el-icon>
                    </div>
                    <div class="cmd-body">
                      <div class="cmd-title">{{ cmd.label }}</div>
                      <div class="cmd-desc">{{ cmd.desc }}</div>
                    </div>
                    <kbd v-if="cmd.shortcut" class="cmd-kbd">{{ cmd.shortcut }}</kbd>
                  </div>
                </div>

                <!-- 分组：模块跳转 -->
                <div v-if="filteredModules.length" class="cmd-section">
                  <div class="cmd-section-title">📦 模块</div>
                  <div
                    v-for="(mod, i) in filteredModules.slice(0, 6)"
                    :key="'m-'+mod.key"
                    class="cmd-item"
                    :class="{ active: cmdIdx === (searchText ? 0 : quickCommands.length) + i }"
                    @click="goModule(mod)"
                    @mouseenter="cmdIdx = (searchText ? 0 : quickCommands.length) + i"
                  >
                    <div class="cmd-icon" :style="{ background: mod.bg, color: mod.color }">
                      <el-icon><component :is="mod.icon" /></el-icon>
                    </div>
                    <div class="cmd-body">
                      <div class="cmd-title">{{ mod.label }}</div>
                      <div class="cmd-desc">跳转到 {{ mod.label }} 页面</div>
                    </div>
                    <span class="cmd-action">跳转 →</span>
                  </div>
                </div>

                <!-- 搜索提示 -->
                <div v-if="searchText && !filteredModules.length" class="cmd-empty">
                  <el-icon><Search /></el-icon>
                  <span>按 Enter 在任务中搜索「{{ searchText }}」</span>
                </div>
              </div>
            </div>

            <!-- 快捷新建下拉（4 项最常用入口） -->
            <el-dropdown trigger="click" @command="onQuickCreate" class="quick-create">
              <el-button type="primary" plain :icon="Lightning" class="quick-create-btn">⚡ 新建</el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item
                    v-for="q in QUICK_CREATE_COMMANDS"
                    :key="q.key"
                    :command="q"
                  >
                    <div class="qc-item">
                      <el-icon><component :is="q.icon" /></el-icon>
                      <span class="qc-label">{{ q.label }}</span>
                      <span v-if="q.tip" class="qc-tip">{{ q.tip }}</span>
                    </div>
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>

            <!-- 通知中心 -->
            <NotificationCenter />

            <el-tooltip content="快捷键帮助（Shift + ?）" placement="bottom">
              <div class="top-action" @click="helpDrawerOpen = true">
                <el-icon><QuestionFilled /></el-icon>
              </div>
            </el-tooltip>
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
                  <el-dropdown-item divided @click="helpDrawerOpen = true">
                    <el-icon><QuestionFilled /></el-icon> 快捷键与帮助
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </header>

        <!-- 多页签 -->
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

        <!-- 5 阶段全维流程条（仅项目化视图展示 · φ 布局定位元素） -->
        <div v-if="showPipeline" class="phasebar-wrap">
          <PhasePipeline v-model="currentPhase" compact @change="onPhaseChange" />
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

      <!-- 全局帮助 Drawer（Shift + ? 切换，列出所有快捷键 + 顶部快捷操作） -->
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

      <!-- 新手引导（首次访问自动弹出） -->
      <OnboardingGuide v-model="onboardingVisible" />
    </div>
  </el-config-provider>
</template>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import {
  Fold, Expand, Refresh, Document, ArrowDown, Close, Search, QuestionFilled, Lightning, Moon, Sunny
} from '@element-plus/icons-vue'
import {
  NAV_MODULES, APP_VERSION, NAV_GROUPS, QUICK_CREATE_COMMANDS, HOTKEY_GROUPS, PROJECT_PHASES, SUB_MODULES, HIDDEN_MODULES
} from '@/types'
import { getHealth } from '@/api'
import { useGlobalShortcuts } from '@/globalShortcuts'
import { provideProjectContext, useProject } from '@/composables/projectContext.js'
import ProjectPicker from '@/components/ProjectPicker.vue'
import OnboardingGuide from '@/components/OnboardingGuide.vue'
import NotificationCenter from '@/components/NotificationCenter.vue'
import PhasePipeline from '@/components/PhasePipeline.vue'

// 全局项目上下文注入（ provide + 单例共享，任意视图可用 useProject() 读取）
provideProjectContext()
const { onChange: onProjectChange } = useProject()

const route = useRoute()
const router = useRouter()

const collapsed = ref(false)
const health = ref({ status: 'pending', label: '连接中…' })
const searchInputRef = ref(null)
const searchText = ref('')
const searchFocused = ref(false)
const showCmdPanel = ref(false)
const cmdIdx = ref(0)

// 快速操作命令
const quickCommands = computed(() => [
  { key: 'new-project', label: '新建项目', desc: '创建一个新项目', icon: 'FolderAdd', color: '#4f46e5', bg: '#eef2ff', action: 'event', event: 'open-create-project', shortcut: 'N' },
  { key: 'new-chat', label: '新建对话', desc: '打开 AI 助手新对话', icon: 'ChatDotRound', color: '#ec4899', bg: '#fce7f3', action: 'route', route: '/ai', shortcut: 'C' },
  { key: 'new-flow', label: '新建工作流', desc: '创建新的工作流编排', icon: 'Operation', color: '#f59e0b', bg: '#fffbeb', action: 'event', event: 'open-create-flow', shortcut: 'F' },
  { key: 'toggle-theme', label: '切换主题', desc: '切换明暗主题', icon: 'Moon', color: '#6366f1', bg: '#eef2ff', action: 'toggle-theme', shortcut: 'T' },
  { key: 'projects', label: '项目列表', desc: '查看所有项目', icon: 'List', color: '#0ea5e9', bg: '#e0f2fe', action: 'route', route: '/projects', shortcut: 'P' },
  { key: 'settings', label: '系统设置', desc: '打开系统设置', icon: 'Setting', color: '#475569', bg: '#f1f5f9', action: 'route', route: '/admin', shortcut: 'S' }
])

// 过滤后的模块
const filteredModules = computed(() => {
  const t = String(searchText.value || '').trim().toLowerCase()
  if (!t) return []
  return ALL_MODULES.value.filter(m =>
    m.label.toLowerCase().includes(t) ||
    (m.key && m.key.toLowerCase().includes(t))
  )
})
const helpDrawerOpen = ref(false)
const onboardingVisible = ref(false)
let disposeShortcuts = null

// 当前项目阶段：按路由自动推断；用户点击流程条可切换
const PIPELINE_ROUTES = new Set([
  '/dashboard', '/projects', '/workbench', '/tasks', '/resources',
  '/expert-center', '/expert-enterprise', '/expert-orchestrator',
  '/ai', '/graph', '/mox-fusion', '/caomei', '/knowledge-base', '/llm-config',
  '/workflow', '/automation', '/plugins', '/mcp',
  '/algolab', '/infinite-optimizer', '/botCenter', '/browser',
  '/monitor'
])
const showPipeline = computed(() => PIPELINE_ROUTES.has(route.path) || route.path.startsWith('/market'))

// 根据路由推断阶段（也可由视图自定义事件 mox:set-phase 覆盖）
const inferredPhase = computed(() => {
  const mapping = {
    // 需求阶段
    '/caomei': 'requirement',
    '/ai': 'requirement',
    '/knowledge-base': 'requirement',
    '/llm-config': 'requirement',
    // 架构阶段
    '/graph': 'architecture',
    '/mox-fusion': 'architecture',
    '/expert-center': 'architecture',
    '/expert-enterprise': 'architecture',
    '/expert-orchestrator': 'architecture',
    // 开发阶段
    '/operators': 'develop',
    '/workflow': 'develop',
    '/plugins': 'develop',
    '/mcp': 'develop',
    '/automation': 'develop',
    '/browser': 'develop',
    '/algolab': 'develop',
    '/infinite-optimizer': 'develop',
    '/botCenter': 'develop',
    '/market': 'develop',
    // 发布阶段
    '/monitor': 'release',
    '/docs': 'release',
    '/admin': 'release'
  }
  return mapping[route.path] || 'requirement'
})
const currentPhase = ref(inferredPhase.value)
watch(inferredPhase, (p) => { currentPhase.value = p })
// 允许子视图通过 window event 覆盖阶段（联盟内 5 段流程切换）
window.addEventListener?.('mox:set-phase', (e) => {
  const key = e?.detail?.key
  if (key && PROJECT_PHASES.some((p) => p.key === key)) currentPhase.value = key
})
function onPhaseChange(p) {
  // 跳阶段 → 跳对应默认路由
  const ph = PROJECT_PHASES.find((x) => x.key === p.key)
  if (!ph) return
  // 路由规则：按阶段跳对应默认模块
  const defaults = {
    requirement: '/dashboard',
    architecture: '/graph',
    develop: '/operators',
    release: '/monitor'
  }
  const target = defaults[ph.key]
  if (target && target !== route.path) router.push(target)
}

// 二级面包屑（路由 meta.subLabel 或监听子 Dialog 打开后的全局 crumb 事件）
const crumbsExtra = computed(() => route.meta?.subLabel || null)

const currentPath = computed(() => route.path)

// AI 页面全屏模式（隐藏侧边栏、顶栏、Tabs）
const isAIFullscreen = computed(() => route.path.startsWith('/ai'))
// 构建完整的模块索引（一级 + 二级子模块 + 三级隐藏模块）
const ALL_MODULES = computed(() => {
  const list = [...NAV_MODULES]
  // 添加二级子模块
  Object.values(SUB_MODULES).forEach(subs => {
    subs.forEach(s => {
      if (!list.find(m => m.path === s.path)) {
        list.push({ key: s.key, label: s.label, path: s.path, color: '#6366f1', bg: '#eef2ff' })
      }
    })
  })
  // 添加三级隐藏模块
  HIDDEN_MODULES.forEach(m => {
    if (!list.find(x => x.path === m.path)) list.push(m)
  })
  return list
})

const crumbs = computed(() => {
  const m = ALL_MODULES.value.find((x) => route.path.startsWith(x.path) && x.path !== '/')
  return m ? [{ path: m.path, label: m.label, key: m.key, icon: m.icon }] : []
})
const tabs = computed(() => {
  const list = [{ path: '/dashboard', label: '工作台', closable: false }]
  const m = ALL_MODULES.value.find((x) => x.path === route.path)
  if (m && m.path !== '/dashboard') list.push({ path: m.path, label: m.label, closable: true })
  return list
})
function modulesByGroup(gKey) {
  const g = NAV_GROUPS.find((x) => x.key === gKey)
  if (!g) return []
  const set = new Set(g.items)
  return NAV_MODULES.filter((m) => set.has(m.key))
}
// 获取同组模块（面包屑下拉快速切换）
function siblingModules(modKey) {
  const mod = NAV_MODULES.find(m => m.key === modKey)
  if (!mod) return NAV_MODULES
  for (const g of NAV_GROUPS) {
    if (g.items.includes(modKey)) {
      return modulesByGroup(g.key)
    }
  }
  return NAV_MODULES
}
function isActive(path) {
  return route.path === path || (path !== '/dashboard' && route.path.startsWith(path))
}
function closeTab(path) {
  if (route.path === path) router.push('/dashboard')
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

// === 顶栏全局搜索 ===
function focusSearch() {
  // 优先同步直接聚焦（避免快捷键场景下异步 nextTick 被浏览器失焦窃取焦点）
  const el = searchInputRef.value
  if (el && typeof el.focus === 'function') {
    try { el.focus() } catch {}
    return
  }
  // 兜底：ref 尚未挂载时，等一帧后重试（如刷新后首帧立即触发 Ctrl+K）
  nextTick(() => {
    const e2 = searchInputRef.value
    if (e2 && typeof e2.focus === 'function') {
      try { e2.focus() } catch {}
    }
  })
  // 最后兜底：原生 DOM 查询（确保即使 ref 链出问题，Ctrl+K 也不会失效）
  setTimeout(() => {
    const native = document.querySelector('input.search-input, input[placeholder*=\"Ctrl\"]')
    if (native && native !== document.activeElement && typeof native.focus === 'function') {
      try { native.focus() } catch {}
    }
  }, 0)
}
function onSearchFocus() {
  searchFocused.value = true
  showCmdPanel.value = true
  cmdIdx.value = 0
}
function onSearchBlur() {
  searchFocused.value = false
  setTimeout(() => { showCmdPanel.value = false }, 150)
}
function onSearchInput() {
  cmdIdx.value = 0
}
function closeSearchPanel() {
  showCmdPanel.value = false
  searchText.value = ''
  searchInputRef.value?.blur()
}
function selectNextCmd() {
  const total = getCmdTotal()
  cmdIdx.value = (cmdIdx.value + 1) % total
}
function selectPrevCmd() {
  const total = getCmdTotal()
  cmdIdx.value = (cmdIdx.value - 1 + total) % total
}
function getCmdTotal() {
  const t = String(searchText.value || '').trim()
  if (!t) return quickCommands.value.length + Math.min(filteredModules.value.length, 6)
  return Math.min(filteredModules.value.length, 6) || 1 // 1 = 搜索提示
}
function goModule(mod) {
  router.push(mod.path)
  closeSearchPanel()
}
function executeCmd(cmd) {
  if (cmd.action === 'route') {
    router.push(cmd.route)
  } else if (cmd.action === 'event') {
    window.dispatchEvent(new CustomEvent(cmd.event, { detail: { from: 'command-palette' } }))
  } else if (cmd.action === 'toggle-theme') {
    toggleTheme()
  }
  closeSearchPanel()
}
function doGlobalSearch() {
  const t = String(searchText.value || '').trim()
  // 有选中项时执行选中项
  if (!t) {
    if (quickCommands.value[cmdIdx.value]) {
      executeCmd(quickCommands.value[cmdIdx.value])
      return
    }
  } else {
    // 输入了内容：如果有匹配模块且选中了，跳模块
    if (filteredModules.value.length && cmdIdx.value < Math.min(filteredModules.value.length, 6)) {
      goModule(filteredModules.value[cmdIdx.value])
      return
    }
    // 否则跳任务搜索
    router.push({ path: '/tasks', query: { q: t } })
    closeSearchPanel()
  }
}

// === 顶栏「⚡ 新建」下拉 ===
function onQuickCreate(q) {
  if (!q) return
  if (q.action === 'event') {
    // 全局派发事件（对应视图监听后打开自己的 Dialog）
    window.dispatchEvent(new CustomEvent(q.event, { detail: { from: 'quick-create' } }))
    return
  }
  if (q.action === 'route') {
    router.push({ path: q.route, query: q.query || {} })
  }
}

// === 主题切换 ===
function toggleTheme() {
  const isDark = document.documentElement.classList.toggle('dark')
  localStorage.setItem('theme', isDark ? 'dark' : 'light')
  // 同步命令面板图标
  const cmd = quickCommands.value.find(c => c.key === 'toggle-theme')
  if (cmd) {
    cmd.icon = isDark ? 'Sunny' : 'Moon'
    cmd.label = isDark ? '浅色模式' : '深色模式'
  }
}
function initTheme() {
  const saved = localStorage.getItem('theme')
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
  const isDark = saved ? saved === 'dark' : prefersDark
  if (isDark) {
    document.documentElement.classList.add('dark')
  }
  // 同步命令面板图标
  setTimeout(() => {
    const cmd = quickCommands.value.find(c => c.key === 'toggle-theme')
    if (cmd) {
      cmd.icon = isDark ? 'Sunny' : 'Moon'
      cmd.label = isDark ? '浅色模式' : '深色模式'
    }
  }, 0)
}

// === 全局快捷键绑定 ===
onMounted(() => {
  // initTheme() // 已迁移到 useTheme composable 统一管理
  refreshHealth()
  disposeShortcuts = useGlobalShortcuts({
    focusSearch,
    toggleHelpDrawer: () => { helpDrawerOpen.value = !helpDrawerOpen.value },
    navModules: NAV_MODULES.slice(0, 9),
    pushRoute: (p) => router.push(p)
  })
  // /market?action=upload → 自动触发 MarketView 打开上传对话框（Query 驱动无状态化）
  if (route.path === '/market' && route.query?.action === 'upload') {
    window.dispatchEvent(new CustomEvent('mox:open-market-upload'))
  }
})
watch(() => route.path, () => {
  refreshHealth()
  // 如果跳转到 /market?action=upload 或 /tasks?action=create
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
.app-shell.sidebar-collapsed .sidebar { width: 68px; }
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
.nav { display: flex; flex-direction: column; gap: 2px; }
.nav-group {
  font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em;
  color: #64748b; padding: 10px 14px 4px;
}
.nav-item {
  display: flex; align-items: center; gap: 12px; height: 42px; padding: 0 14px;
  border-radius: 11px; color: #94a3b8; font-size: 14px; font-weight: 500;
  transition: all var(--transition); position: relative; white-space: nowrap;
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
  transition: transform var(--transition);
}
.nav-item.active .nav-bar { transform: translateY(-50%) scaleY(1); }
.nav-icon { font-size: 18px; flex-shrink: 0; }
.sidebar-footer {
  height: 50px; display: flex; align-items: center; justify-content: space-between;
  padding: 0 18px; border-top: 1px solid rgba(255, 255, 255, 0.06);
  font-size: 12px; color: #64748b;
}
.health { display: flex; align-items: center; gap: 7px; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--warning); }
.dot.ok { background: var(--success); box-shadow: 0 0 0 3px rgba(16, 185, 129, 0.2); }
.dot.down { background: var(--danger); box-shadow: 0 0 0 3px rgba(239, 68, 68, 0.2); }

.main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.topbar {
  height: var(--header-h); flex-shrink: 0; background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(12px); border-bottom: 1px solid var(--border);
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 20px; box-shadow: var(--shadow-sm); z-index: 5;
}
.topbar-left { display: flex; align-items: center; gap: 16px; min-width: 0; }
.collapse-btn { font-size: 20px; cursor: pointer; color: var(--text-2); flex-shrink: 0; }
.collapse-btn:hover { color: var(--brand); }

/* 顶栏项目选择器 */
.topbar-project-picker {
  margin-left: 4px;
}
:deep(.topbar-project-picker .pp-select) {
  width: 260px;
  height: 36px;
}
:deep(.topbar-project-picker .pp-select .el-select__wrapper) {
  background: var(--brand-soft);
  border-color: transparent;
  height: 36px;
  min-height: 36px;
  border-radius: 10px;
  font-weight: 600;
  color: var(--brand-dark);
}
:deep(.topbar-project-picker .pp-select:hover .el-select__wrapper),
:deep(.topbar-project-picker .pp-select.is-focused .el-select__wrapper) {
  background: #fff;
  border-color: var(--brand);
  box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.12);
}
.crumb-extra { color: var(--brand-dark); font-weight: 600; }
.crumb-clickable {
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  padding: 2px 6px;
  border-radius: 6px;
  transition: all 0.15s;
}
.crumb-clickable:hover {
  background: var(--brand-soft);
  color: var(--brand-dark);
}
.crumb-dropdown :deep(.el-dropdown-menu__item.is-active) {
  color: var(--brand);
  font-weight: 600;
  background: var(--brand-soft);
}
/* 面包屑防截断：不换行、不压缩、文字溢出时省略 */
.topbar-left :deep(.el-breadcrumb) {
  flex-shrink: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 320px;
}
.topbar-left :deep(.el-breadcrumb__item) {
  white-space: nowrap;
}
.topbar-right { display: flex; align-items: center; gap: 10px; }
.top-action {
  width: 36px; height: 36px; border-radius: 10px; display: grid; place-items: center;
  cursor: pointer; color: var(--text-2); font-size: 18px; transition: all var(--transition);
}
.top-action:hover { background: var(--brand-soft); color: var(--brand); }
.user {
  display: flex; align-items: center; gap: 8px; padding: 4px 10px 4px 4px;
  border-radius: 999px; cursor: pointer; transition: background var(--transition);
}
.user:hover { background: var(--bg-page); }
.user-name { font-size: 14px; font-weight: 600; }

/* 全局命令面板输入框 */
.global-search {
  display: flex; align-items: center; gap: 8px;
  height: 36px; padding: 0 10px; border-radius: 10px;
  width: 240px; background: var(--bg-page);
  border: 1px solid var(--border); transition: all var(--transition);
  cursor: text;
}
.global-search.focused, .global-search:hover {
  width: 420px; border-color: var(--brand); box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.12);
  background: #fff;
}
.search-icon { color: var(--text-3); font-size: 16px; }
.search-input {
  all: unset; flex: 1; border: 0; background: transparent;
  font-size: 13px; color: var(--text-1);
}
.search-input::placeholder { color: var(--text-3); }

/* 命令面板下拉 */
.global-search {
  position: relative;
}
.cmd-panel {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  right: 0;
  background: #fff;
  border: 1px solid var(--border-1);
  border-radius: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.15);
  max-height: 420px;
  overflow-y: auto;
  z-index: 1000;
  padding: 6px;
}
.cmd-section {
  margin-bottom: 4px;
}
.cmd-section:last-child {
  margin-bottom: 0;
}
.cmd-section-title {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-3);
  padding: 8px 10px 6px;
  letter-spacing: 0.5px;
  text-transform: uppercase;
}
.cmd-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.cmd-item:hover, .cmd-item.active {
  background: var(--brand-soft);
}
.cmd-item.active {
  background: linear-gradient(90deg, rgba(79, 70, 229, 0.12), rgba(99, 102, 241, 0.08));
}
.cmd-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  font-size: 15px;
  flex-shrink: 0;
}
.cmd-body {
  flex: 1;
  min-width: 0;
}
.cmd-title {
  font-weight: 600;
  font-size: 13px;
  color: var(--text-1);
}
.cmd-desc {
  font-size: 11px;
  color: var(--text-3);
  margin-top: 1px;
}
.cmd-action {
  font-size: 11px;
  color: var(--brand);
  font-weight: 600;
  flex-shrink: 0;
}
.cmd-kbd {
  font-size: 10px;
  padding: 2px 6px;
  background: rgba(148, 163, 184, 0.15);
  color: var(--text-2);
  border-radius: 4px;
  flex-shrink: 0;
}
.cmd-empty {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 10px;
  color: var(--text-3);
  font-size: 12px;
}
.cmd-empty .el-icon {
  color: var(--brand);
}

/* 快捷新建按钮 */
.quick-create-btn { height: 36px; border-radius: 10px; padding: 0 14px; font-weight: 600; }
.qc-item { display: flex; align-items: center; gap: 10px; min-width: 240px; }
.qc-item .el-icon { color: var(--brand-dark); }
.qc-label { flex: 1; font-weight: 500; }
.qc-tip { font-size: 11px; color: var(--text-3); }

.kbd {
  display: inline-block; padding: 1px 6px; font-size: 11px;
  background: rgba(148, 163, 184, 0.14); color: var(--text-2);
  border: 1px solid var(--border); border-radius: 5px; line-height: 1.6;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.tabs {
  height: 42px; flex-shrink: 0; background: #fff; border-bottom: 1px solid var(--border);
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
.tab-close:hover { background: #cbd5e1; color: #fff; }

/* 阶段流程条（在 tabs 与 content 之间） */
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
/* AI 全屏模式：去掉 padding 和背景 */
.ai-fullscreen .main { margin-left: 0 !important; }
.ai-fullscreen .content {
  padding: 0;
  background: var(--bg-deep-sky, #f8fafc);
}
.ai-fullscreen .content > * {
  height: 100%;
}

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

/* ===== 响应式适配 ===== */
/* 平板：≤1024px */
@media (max-width: 1024px) {
  .sidebar {
    width: 60px;
    transform: translateX(0);
  }
  .main-wrap {
    margin-left: 60px;
  }
  .collapse-btn { display: none; }
  .nav-group { display: none; }
  .global-search { width: 180px; }
  .global-search.focused, .global-search:hover { width: 280px; }
  .topbar-project-picker :deep(.pp-select) { width: 200px; }
  .user-name { display: none; }
  .content { padding: 16px; }
}

/* 手机：≤768px */
@media (max-width: 768px) {
  .sidebar {
    position: fixed;
    z-index: 200;
    transform: translateX(-100%);
    transition: transform 0.3s ease;
  }
  .sidebar.mobile-open {
    transform: translateX(0);
  }
  .main-wrap {
    margin-left: 0;
  }
  .collapse-btn { display: block; }
  .topbar-left { gap: 8px; }
  .topbar-project-picker :deep(.pp-select) { width: 140px; height: 32px; }
  .topbar-project-picker :deep(.pp-select .el-select__wrapper) { height: 32px; min-height: 32px; border-radius: 8px; }
  .global-search { display: none; }
  .quick-create-btn { padding: 0 10px; font-size: 12px; }
  .quick-create-btn .el-icon { margin-right: 4px; }
  .top-action { width: 32px; height: 32px; }
  .content { padding: 12px; }
  .page-container { padding: 16px; }
  .tabs { display: none; }
}

/* 小屏手机：≤480px */
@media (max-width: 480px) {
  .topbar-project-picker :deep(.pp-select) { width: 110px; }
  .quick-create-btn span:not(.el-icon) { display: none; }
  .quick-create-btn { padding: 0 8px; }
  .notification-center { display: none; }
}
</style>
