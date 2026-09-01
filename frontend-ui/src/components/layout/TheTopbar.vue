<template>
  <header class="topbar" v-if="!isAIFullscreen">
    <!-- 左侧：折叠 + 面包屑 -->
    <div class="topbar-left">
      <button class="topbar-icon-btn" @click="$emit('toggle-collapse')" :title="collapsed ? '展开侧栏' : '收起侧栏'">
        <el-icon>
          <component :is="collapsed ? 'Expand' : 'Fold'" />
        </el-icon>
      </button>

      <!-- 自定义面包屑 -->
      <div class="crumb-custom">
        <span class="crumb-module" @click="goDashboard">{{ moduleLabel }}</span>
        <span class="crumb-sep">/</span>
        <span class="crumb-current">{{ currentLabel }}</span>
      </div>
    </div>

    <!-- 右侧：搜索 + 新建 + 通知 + 主题 + 用户 -->
    <div class="topbar-right">
      <!-- 全局搜索 -->
      <div class="global-search" :class="{ focused: searchFocused }">
        <el-icon class="search-icon"><Search /></el-icon>
        <input
          ref="searchInputRef"
          v-model="searchText"
          class="search-input"
          placeholder="搜索… (Ctrl + K)"
          @focus="onSearchFocus"
          @blur="onSearchBlur"
          @input="onSearchInput"
          @keydown.enter="doGlobalSearch"
          @keydown.esc="closeSearchPanel"
          @keydown.down.prevent="selectNextCmd"
          @keydown.up.prevent="selectPrevCmd"
        />
        <kbd v-if="!searchFocused" class="kbd">⌘K</kbd>

        <!-- 命令面板下拉 -->
        <div v-if="showCmdPanel" class="cmd-panel" @mousedown.prevent>
          <!-- 快速操作 -->
          <div v-if="!searchText" class="cmd-section">
            <div class="cmd-section-title">快速操作</div>
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
            </div>
          </div>

          <!-- 模块跳转 -->
          <div v-if="filteredModules.length" class="cmd-section">
            <div class="cmd-section-title">模块</div>
            <div
              v-for="(mod, i) in filteredModules.slice(0, 6)"
              :key="'m-'+mod.key"
              class="cmd-item"
              :class="{ active: cmdIdx === (searchText ? 0 : quickCommands.length) + i }"
              @click="goModule(mod)"
              @mouseenter="cmdIdx = (searchText ? 0 : quickCommands.length) + i"
            >
              <div class="cmd-body">
                <div class="cmd-title">{{ mod.label }}</div>
                <div class="cmd-desc">跳转到 {{ mod.label }} 页面</div>
              </div>
              <span class="cmd-action">跳转 →</span>
            </div>
          </div>

          <div v-if="searchText && !filteredModules.length" class="cmd-empty">
            <el-icon><Search /></el-icon>
            <span>按 Enter 在任务中搜索「{{ searchText }}」</span>
          </div>
        </div>
      </div>

      <!-- 快捷新建 -->
      <el-dropdown trigger="click" @command="onQuickCreate" class="quick-create">
        <el-button type="primary" :icon="Plus" class="quick-create-btn">新建</el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item v-for="q in QUICK_CREATE_COMMANDS" :key="q.key" :command="q">
              <div class="qc-item">
                <el-icon><component :is="q.icon" /></el-icon>
                <span class="qc-label">{{ q.label }}</span>
                <span v-if="q.tip" class="qc-tip">{{ q.tip }}</span>
              </div>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>

      <!-- 通知 -->
      <NotificationCenter />

      <!-- 主题切换 -->
      <ThemeSwitcher />

      <!-- 用户菜单 -->
      <el-dropdown placement="bottom-end">
        <div class="user-menu">
          <el-avatar :size="30" class="user-avatar">A</el-avatar>
        </div>
        <template #dropdown>
          <el-dropdown-menu>
            <div class="user-dropdown-header">
              <div class="ud-name">管理员</div>
              <div class="ud-role">系统架构师</div>
            </div>
            <el-dropdown-item @click="$router.push('/admin')">
              <el-icon><Setting /></el-icon> 系统设置
            </el-dropdown-item>
            <el-dropdown-item @click="$emit('toggle-help')">
              <el-icon><QuestionFilled /></el-icon> 快捷键帮助
            </el-dropdown-item>
            <el-dropdown-item divided>
              <el-icon><SwitchButton /></el-icon> 退出登录
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </header>
</template>

<script setup>
import { ref, computed, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Fold, Expand, Plus, Search, ArrowDown, QuestionFilled, Setting, SwitchButton
} from '@element-plus/icons-vue'
import { NAV_MODULES, NAV_GROUPS, QUICK_CREATE_COMMANDS, SUB_MODULES, HIDDEN_MODULES, MODULE_SIDEBAR_CONFIG } from '@/constants'
import NotificationCenter from '@/components/NotificationCenter.vue'
import ThemeSwitcher from '@/components/ThemeSwitcher.vue'

defineProps({
  collapsed: { type: Boolean, default: false },
  isAIFullscreen: { type: Boolean, default: false }
})

const emit = defineEmits(['toggle-collapse', 'toggle-help', 'refresh-health'])

const route = useRoute()
const router = useRouter()

const searchInputRef = ref(null)
const searchText = ref('')
const searchFocused = ref(false)
const showCmdPanel = ref(false)
const cmdIdx = ref(0)

// 模块索引
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

// 当前模块 key
const currentModuleKey = computed(() => {
  const p = route.path
  if (p.startsWith('/dashboard')) return 'dashboard'
  if (p.startsWith('/projects')) return 'projects'
  if (p.startsWith('/tasks')) return 'tasks'
  if (p.startsWith('/expert-workspace') || p.startsWith('/expert-center') || p.startsWith('/expert-plaza')) return 'expert'
  if (p.startsWith('/ai')) return 'ai'
  if (p.startsWith('/graph')) return 'graph'
  if (p.startsWith('/operators')) return 'operators'
  if (p.startsWith('/workflow')) return 'workflow'
  if (p.startsWith('/market')) return 'market'
  if (p.startsWith('/admin')) return 'admin'
  return 'dashboard'
})

const moduleLabel = computed(() => {
  const cfg = MODULE_SIDEBAR_CONFIG[currentModuleKey.value]
  return cfg ? cfg.title : '工作台'
})

const currentLabel = computed(() => {
  // 从 route.meta 或 SUB_MODULES 匹配当前页
  if (route.meta?.title) return route.meta.title
  if (route.meta?.subLabel) return route.meta.subLabel
  const m = ALL_MODULES.value.find((x) => route.path.startsWith(x.path) && x.path !== '/')
  return m ? m.label : '概览'
})

function goDashboard() {
  router.push('/dashboard')
}

function modulesByGroup(gKey) {
  const g = NAV_GROUPS.find((x) => x.key === gKey)
  if (!g) return []
  const set = new Set(g.items)
  return NAV_MODULES.filter((m) => set.has(m.key))
}

// 快速操作
const quickCommands = computed(() => [
  { key: 'new-project', label: '新建项目', desc: '创建一个新项目', icon: 'FolderAdd', color: '#818cf8', bg: 'rgba(99,102,241,.15)', action: 'event', event: 'mox:open-create-project' },
  { key: 'new-chat', label: '新建对话', desc: '打开 AI 助手新对话', icon: 'ChatDotRound', color: '#ec4899', bg: 'rgba(236,72,153,.15)', action: 'route', route: '/ai' },
  { key: 'new-flow', label: '新建工作流', desc: '创建新的工作流编排', icon: 'Operation', color: '#f59e0b', bg: 'rgba(245,158,11,.15)', action: 'event', event: 'open-create-flow' },
  { key: 'projects', label: '项目列表', desc: '查看所有项目', icon: 'List', color: '#06b6d4', bg: 'rgba(6,182,212,.15)', action: 'route', route: '/projects' },
  { key: 'settings', label: '系统设置', desc: '打开系统设置', icon: 'Setting', color: '#9aa0b4', bg: 'rgba(148,163,184,.15)', action: 'route', route: '/admin' }
])

const filteredModules = computed(() => {
  const t = String(searchText.value || '').trim().toLowerCase()
  if (!t) return []
  return ALL_MODULES.value.filter(m =>
    m.label.toLowerCase().includes(t) ||
    (m.key && m.key.toLowerCase().includes(t))
  )
})

// 搜索
function focusSearch() {
  nextTick(() => {
    const native = document.querySelector('input.search-input')
    if (native && native !== document.activeElement) native.focus?.()
  })
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

function onSearchInput() { cmdIdx.value = 0 }

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
  return Math.min(filteredModules.value.length, 6) || 1
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
  }
  closeSearchPanel()
}

function doGlobalSearch() {
  const t = String(searchText.value || '').trim()
  if (!t) {
    if (quickCommands.value[cmdIdx.value]) executeCmd(quickCommands.value[cmdIdx.value])
    return
  }
  if (filteredModules.value.length && cmdIdx.value < Math.min(filteredModules.value.length, 6)) {
    goModule(filteredModules.value[cmdIdx.value])
    return
  }
  router.push({ path: '/tasks', query: { q: t } })
  closeSearchPanel()
}

// 快捷新建
function onQuickCreate(q) {
  if (!q) return
  if (q.action === 'event') {
    window.dispatchEvent(new CustomEvent(q.event, { detail: { from: 'quick-create' } }))
    return
  }
  if (q.action === 'route') {
    router.push({ path: q.route, query: q.query || {} })
  }
}

defineExpose({ focusSearch })
</script>

<style scoped>
.topbar {
  height: 52px;
  flex-shrink: 0;
  background: var(--bg-secondary, #161821);
  border-bottom: 1px solid var(--border, #2d3148);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  z-index: 5;
}

/* ===== Left ===== */
.topbar-left {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  flex: 1;
}

.topbar-icon-btn {
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary, #9aa0b4);
  cursor: pointer;
  display: grid;
  place-items: center;
  font-size: 16px;
  transition: all 0.15s;
  flex-shrink: 0;
}

.topbar-icon-btn:hover {
  background: var(--bg-hover, #2a2f45);
  color: var(--text-primary, #e8eaed);
}

/* 自定义面包屑 */
.crumb-custom {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.crumb-module {
  color: var(--text-secondary, #9aa0b4);
  cursor: pointer;
  transition: color 0.15s;
}
.crumb-module:hover {
  color: var(--accent-light, #818cf8);
}

.crumb-sep {
  color: var(--text-muted, #6b7280);
  font-size: 12px;
}

.crumb-current {
  color: var(--text-primary, #e8eaed);
  font-weight: 500;
}

/* ===== Right ===== */
.topbar-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

/* 全局搜索 */
.global-search {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 10px;
  border-radius: 8px;
  width: 200px;
  background: var(--bg-card, #242838);
  border: 1px solid var(--border, #2d3148);
  transition: all 0.2s ease;
  cursor: text;
  position: relative;
}

.global-search.focused,
.global-search:hover {
  width: 320px;
  border-color: var(--accent, #6366f1);
  box-shadow: 0 0 0 3px rgba(99,102,241,.1);
}

.search-icon {
  color: var(--text-muted, #6b7280);
  font-size: 15px;
}

.search-input {
  all: unset;
  flex: 1;
  border: 0;
  background: transparent;
  font-size: 13px;
  color: var(--text-primary, #e8eaed);
}

.search-input::placeholder {
  color: var(--text-muted, #6b7280);
}

.kbd {
  font-size: 10px;
  padding: 2px 6px;
  background: var(--bg-tertiary, #1e2130);
  color: var(--text-muted, #6b7280);
  border: 1px solid var(--border, #2d3148);
  border-radius: 4px;
  flex-shrink: 0;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

/* 命令面板 */
.cmd-panel {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  right: 0;
  background: var(--bg-secondary, #161821);
  border: 1px solid var(--border, #2d3148);
  border-radius: 10px;
  box-shadow: var(--shadow-lg, 0 10px 40px rgba(0,0,0,.4));
  max-height: 380px;
  overflow-y: auto;
  z-index: 1000;
  padding: 6px;
}

.cmd-section { margin-bottom: 4px; }
.cmd-section:last-child { margin-bottom: 0; }

.cmd-section-title {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-muted, #6b7280);
  padding: 8px 10px 4px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.cmd-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 7px;
  cursor: pointer;
  transition: all 0.15s;
}

.cmd-item:hover,
.cmd-item.active {
  background: var(--accent-dim, rgba(99,102,241,.15));
}

.cmd-icon {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  display: grid;
  place-items: center;
  font-size: 14px;
  flex-shrink: 0;
}

.cmd-body { flex: 1; min-width: 0; }
.cmd-title { font-weight: 600; font-size: 13px; color: var(--text-primary, #e8eaed); }
.cmd-desc { font-size: 11px; color: var(--text-muted, #6b7280); margin-top: 1px; }
.cmd-action { font-size: 11px; color: var(--accent-light, #818cf8); font-weight: 600; flex-shrink: 0; }

.cmd-empty {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 10px;
  color: var(--text-muted, #6b7280);
  font-size: 12px;
}

.cmd-empty .el-icon { color: var(--accent, #6366f1); }

/* 快捷新建按钮 */
.quick-create-btn {
  height: 34px;
  border-radius: 8px;
  padding: 0 14px;
  font-weight: 500;
  font-size: 13px;
  background: var(--accent, #6366f1) !important;
  border-color: var(--accent, #6366f1) !important;
}
.quick-create-btn:hover {
  background: #5558e3 !important;
  border-color: #5558e3 !important;
}

.qc-item {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 200px;
}

.qc-item .el-icon { color: var(--accent-light, #818cf8); }
.qc-label { flex: 1; font-weight: 500; font-size: 13px; color: var(--text-primary, #e8eaed); }
.qc-tip { font-size: 11px; color: var(--text-muted, #6b7280); }

/* 用户菜单 */
.user-menu {
  display: flex;
  align-items: center;
  padding: 2px;
  border-radius: 999px;
  cursor: pointer;
  transition: background 0.15s;
}

.user-menu:hover {
  background: var(--bg-hover, #2a2f45);
}

.user-avatar {
  background: linear-gradient(135deg, #6366f1, #ec4899) !important;
  font-weight: 700;
  font-size: 13px;
}

.user-dropdown-header {
  padding: 10px 14px;
  border-bottom: 1px solid var(--border, #2d3148);
  margin-bottom: 4px;
}

.ud-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary, #e8eaed);
}

.ud-role {
  font-size: 12px;
  color: var(--text-muted, #6b7280);
  margin-top: 2px;
}

/* ===== Responsive ===== */
@media (max-width: 1024px) {
  .global-search { width: 160px; }
  .global-search.focused, .global-search:hover { width: 240px; }
}

@media (max-width: 768px) {
  .topbar { padding: 0 12px; }
  .topbar-left { gap: 8px; }
  .crumb-custom { display: none; }
  .global-search { display: none; }
  .quick-create-btn { padding: 0 10px; font-size: 12px; }
}
</style>
