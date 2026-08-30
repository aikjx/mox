<template>
  <header class="topbar" v-if="!isAIFullscreen">
    <div class="topbar-left">
      <el-icon class="collapse-btn" @click="$emit('toggle-collapse')">
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
          <el-dropdown trigger="click" @command="(p) => $router.push(p)" class="crumb-dropdown">
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
      <!-- 全局搜索输入框 -->
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

      <!-- 快捷新建下拉 -->
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
        <div class="top-action" @click="$emit('toggle-help')">
          <el-icon><QuestionFilled /></el-icon>
        </div>
      </el-tooltip>
      <el-tooltip content="刷新系统状态" placement="bottom">
        <div class="top-action" @click="$emit('refresh-health')">
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
            <el-dropdown-item divided @click="$emit('toggle-help')">
              <el-icon><QuestionFilled /></el-icon> 快捷键与帮助
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
  Fold, Expand, Refresh, Document, ArrowDown, Search, QuestionFilled, Lightning, HomeFilled
} from '@element-plus/icons-vue'
import {
  NAV_MODULES, NAV_GROUPS, QUICK_CREATE_COMMANDS, SUB_MODULES, HIDDEN_MODULES
} from '@/constants'
import ProjectPicker from '@/components/ProjectPicker.vue'
import NotificationCenter from '@/components/NotificationCenter.vue'

const props = defineProps({
  collapsed: { type: Boolean, default: false },
  isAIFullscreen: { type: Boolean, default: false }
})

const emit = defineEmits(['toggle-collapse', 'toggle-help', 'refresh-health', 'toggle-theme'])

const route = useRoute()
const router = useRouter()

const searchInputRef = ref(null)
const searchText = ref('')
const searchFocused = ref(false)
const showCmdPanel = ref(false)
const cmdIdx = ref(0)

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

const crumbsExtra = computed(() => route.meta?.subLabel || null)

const crumbs = computed(() => {
  const m = ALL_MODULES.value.find((x) => route.path.startsWith(x.path) && x.path !== '/')
  return m ? [{ path: m.path, label: m.label, key: m.key, icon: m.icon }] : []
})

function modulesByGroup(gKey) {
  const g = NAV_GROUPS.find((x) => x.key === gKey)
  if (!g) return []
  const set = new Set(g.items)
  return NAV_MODULES.filter((m) => set.has(m.key))
}

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

// 快速操作命令
const quickCommands = computed(() => [
  { key: 'new-project', label: '新建项目', desc: '创建一个新项目', icon: 'FolderAdd', color: '#4f46e5', bg: '#eef2ff', action: 'event', event: 'mox:open-create-project', shortcut: 'N' },
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

// === 顶栏全局搜索 ===
function focusSearch() {
  const el = searchInputRef.value
  if (el && typeof el.focus === 'function') {
    try { el.focus() } catch {}
    return
  }
  nextTick(() => {
    const e2 = searchInputRef.value
    if (e2 && typeof e2.focus === 'function') {
      try { e2.focus() } catch {}
    }
  })
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
  } else if (cmd.action === 'toggle-theme') {
    emit('toggle-theme')
  }
  closeSearchPanel()
}
function doGlobalSearch() {
  const t = String(searchText.value || '').trim()
  if (!t) {
    if (quickCommands.value[cmdIdx.value]) {
      executeCmd(quickCommands.value[cmdIdx.value])
      return
    }
  } else {
    if (filteredModules.value.length && cmdIdx.value < Math.min(filteredModules.value.length, 6)) {
      goModule(filteredModules.value[cmdIdx.value])
      return
    }
    router.push({ path: '/tasks', query: { q: t } })
    closeSearchPanel()
  }
}

// === 顶栏「⚡ 新建」下拉 ===
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

// 暴露方法给父组件
defineExpose({ focusSearch })
</script>

<style scoped>
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
.topbar-project-picker { margin-left: 4px; }
:deep(.topbar-project-picker .pp-select) {
  width: 260px; height: 36px;
}
:deep(.topbar-project-picker .pp-select .el-select__wrapper) {
  background: var(--brand-soft); border-color: transparent;
  height: 36px; min-height: 36px; border-radius: 10px;
  font-weight: 600; color: var(--brand-dark);
}
:deep(.topbar-project-picker .pp-select:hover .el-select__wrapper),
:deep(.topbar-project-picker .pp-select.is-focused .el-select__wrapper) {
  background: #fff; border-color: var(--brand);
  box-shadow: 0 0 0 3px rgba(79, 70, 229, 0.12);
}
.crumb-extra { color: var(--brand-dark); font-weight: 600; }
.crumb-clickable {
  cursor: pointer; display: inline-flex; align-items: center;
  padding: 2px 6px; border-radius: 6px; transition: all 0.15s;
}
.crumb-clickable:hover { background: var(--brand-soft); color: var(--brand-dark); }
.crumb-dropdown :deep(.el-dropdown-menu__item.is-active) {
  color: var(--brand); font-weight: 600; background: var(--brand-soft);
}
/* 面包屑防截断 */
.topbar-left :deep(.el-breadcrumb) {
  flex-shrink: 0; white-space: nowrap; overflow: hidden;
  text-overflow: ellipsis; max-width: 320px;
}
.topbar-left :deep(.el-breadcrumb__item) { white-space: nowrap; }
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
  cursor: text; position: relative;
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
.cmd-panel {
  position: absolute; top: calc(100% + 8px); left: 0; right: 0;
  background: #fff; border: 1px solid var(--border-1);
  border-radius: 12px; box-shadow: 0 12px 40px rgba(0, 0, 0, 0.15);
  max-height: 420px; overflow-y: auto; z-index: 1000; padding: 6px;
}
.cmd-section { margin-bottom: 4px; }
.cmd-section:last-child { margin-bottom: 0; }
.cmd-section-title {
  font-size: 11px; font-weight: 700; color: var(--text-3);
  padding: 8px 10px 6px; letter-spacing: 0.5px; text-transform: uppercase;
}
.cmd-item {
  display: flex; align-items: center; gap: 10px;
  padding: 8px 10px; border-radius: 8px;
  cursor: pointer; transition: all 0.15s;
}
.cmd-item:hover, .cmd-item.active { background: var(--brand-soft); }
.cmd-item.active {
  background: linear-gradient(90deg, rgba(79, 70, 229, 0.12), rgba(99, 102, 241, 0.08));
}
.cmd-icon {
  width: 32px; height: 32px; border-radius: 8px;
  display: grid; place-items: center; font-size: 15px; flex-shrink: 0;
}
.cmd-body { flex: 1; min-width: 0; }
.cmd-title { font-weight: 600; font-size: 13px; color: var(--text-1); }
.cmd-desc { font-size: 11px; color: var(--text-3); margin-top: 1px; }
.cmd-action { font-size: 11px; color: var(--brand); font-weight: 600; flex-shrink: 0; }
.cmd-kbd {
  font-size: 10px; padding: 2px 6px;
  background: rgba(148, 163, 184, 0.15); color: var(--text-2);
  border-radius: 4px; flex-shrink: 0;
}
.cmd-empty {
  display: flex; align-items: center; gap: 8px;
  padding: 12px 10px; color: var(--text-3); font-size: 12px;
}
.cmd-empty .el-icon { color: var(--brand); }

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

/* 响应式 */
@media (max-width: 1024px) {
  .global-search { width: 180px; }
  .global-search.focused, .global-search:hover { width: 280px; }
  .topbar-project-picker :deep(.pp-select) { width: 200px; }
  .user-name { display: none; }
}
@media (max-width: 768px) {
  .topbar-left { gap: 8px; }
  .topbar-project-picker :deep(.pp-select) { width: 140px; height: 32px; }
  .topbar-project-picker :deep(.pp-select .el-select__wrapper) { height: 32px; min-height: 32px; border-radius: 8px; }
  .global-search { display: none; }
  .quick-create-btn { padding: 0 10px; font-size: 12px; }
  .quick-create-btn .el-icon { margin-right: 4px; }
  .top-action { width: 32px; height: 32px; }
}
@media (max-width: 480px) {
  .topbar-project-picker :deep(.pp-select) { width: 110px; }
  .quick-create-btn span:not(.el-icon) { display: none; }
  .quick-create-btn { padding: 0 8px; }
  .notification-center { display: none; }
}
</style>
