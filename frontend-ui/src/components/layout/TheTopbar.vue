<template>
  <header class="topbar" v-if="!isAIFullscreen">
    <!-- 左侧：折叠 + 项目 + 面包屑 -->
    <div class="topbar-left">
      <button class="topbar-icon-btn" @click="$emit('toggle-collapse')" :title="collapsed ? '展开侧边栏' : '收起侧边栏'">
        <el-icon>
          <component :is="collapsed ? 'Expand' : 'Fold'" />
        </el-icon>
      </button>

      <div class="topbar-divider"></div>

      <!-- 项目选择器 -->
      <ProjectPicker variant="top" class="topbar-project-picker" />

      <!-- 面包屑 -->
      <el-breadcrumb separator="/" class="crumb-nav">
        <el-breadcrumb-item :to="{ path: '/dashboard' }">
          工作台
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
                  {{ sib.label }}
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </el-breadcrumb-item>
        <el-breadcrumb-item v-if="crumbsExtra" class="crumb-extra">{{ crumbsExtra }}</el-breadcrumb-item>
      </el-breadcrumb>
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
import { NAV_MODULES, NAV_GROUPS, QUICK_CREATE_COMMANDS, SUB_MODULES, HIDDEN_MODULES } from '@/constants'
import ProjectPicker from '@/components/ProjectPicker.vue'
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
  for (const g of NAV_GROUPS) {
    if (g.items.includes(modKey)) return modulesByGroup(g.key)
  }
  return NAV_MODULES
}

// 快速操作
const quickCommands = computed(() => [
  { key: 'new-project', label: '新建项目', desc: '创建一个新项目', icon: 'FolderAdd', color: '#4f46e5', bg: '#eef2ff', action: 'event', event: 'mox:open-create-project' },
  { key: 'new-chat', label: '新建对话', desc: '打开 AI 助手新对话', icon: 'ChatDotRound', color: '#ec4899', bg: '#fce7f3', action: 'route', route: '/ai' },
  { key: 'new-flow', label: '新建工作流', desc: '创建新的工作流编排', icon: 'Operation', color: '#f59e0b', bg: '#fffbeb', action: 'event', event: 'open-create-flow' },
  { key: 'projects', label: '项目列表', desc: '查看所有项目', icon: 'List', color: '#0ea5e9', bg: '#e0f2fe', action: 'route', route: '/projects' },
  { key: 'settings', label: '系统设置', desc: '打开系统设置', icon: 'Setting', color: '#475569', bg: '#f1f5f9', action: 'route', route: '/admin' }
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
  height: var(--header-h, 56px);
  flex-shrink: 0;
  background: var(--topbar-bg, rgba(255, 255, 255, 0.8));
  backdrop-filter: saturate(180%) blur(12px);
  -webkit-backdrop-filter: saturate(180%) blur(12px);
  border-bottom: 1px solid var(--border, #e2e8f0);
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
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text-2, #64748b);
  cursor: pointer;
  display: grid;
  place-items: center;
  font-size: 18px;
  transition: all 0.15s;
  flex-shrink: 0;
}

.topbar-icon-btn:hover {
  background: var(--bg-page, #f8fafc);
  color: var(--brand, #6366f1);
}

.topbar-divider {
  width: 1px;
  height: 20px;
  background: var(--border, #e2e8f0);
  flex-shrink: 0;
}

/* 项目选择器 */
.topbar-project-picker {
  flex-shrink: 0;
}

:deep(.topbar-project-picker .pp-select) {
  width: 240px;
  height: 34px;
}

:deep(.topbar-project-picker .pp-select .el-select__wrapper) {
  background: var(--brand-soft, #eef2ff);
  border-color: transparent;
  height: 34px;
  min-height: 34px;
  border-radius: 8px;
  font-weight: 600;
  color: var(--brand-dark, #4338ca);
}

:deep(.topbar-project-picker .pp-select:hover .el-select__wrapper) {
  background: #fff;
  border-color: var(--brand, #6366f1);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}

/* 面包屑 */
.crumb-nav {
  flex-shrink: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 300px;
}

.crumb-extra {
  color: var(--brand-dark);
  font-weight: 600;
}

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
  background: var(--bg-page, #f8fafc);
  border: 1px solid transparent;
  transition: all 0.2s ease;
  cursor: text;
  position: relative;
}

.global-search.focused,
.global-search:hover {
  width: 320px;
  border-color: var(--brand, #6366f1);
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
  background: #fff;
}

.search-icon {
  color: var(--text-3, #94a3b8);
  font-size: 15px;
}

.search-input {
  all: unset;
  flex: 1;
  border: 0;
  background: transparent;
  font-size: 13px;
  color: var(--text-1, #0f172a);
}

.search-input::placeholder {
  color: var(--text-3, #94a3b8);
}

.kbd {
  font-size: 10px;
  padding: 2px 6px;
  background: rgba(148, 163, 184, 0.15);
  color: var(--text-2, #64748b);
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
  background: #fff;
  border: 1px solid var(--border, #e2e8f0);
  border-radius: 10px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.12);
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
  color: var(--text-3, #94a3b8);
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
  background: var(--brand-soft, #eef2ff);
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
.cmd-title { font-weight: 600; font-size: 13px; color: var(--text-1, #0f172a); }
.cmd-desc { font-size: 11px; color: var(--text-3, #94a3b8); margin-top: 1px; }
.cmd-action { font-size: 11px; color: var(--brand, #6366f1); font-weight: 600; flex-shrink: 0; }

.cmd-empty {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 10px;
  color: var(--text-3, #94a3b8);
  font-size: 12px;
}

.cmd-empty .el-icon { color: var(--brand, #6366f1); }

/* 快捷新建按钮 */
.quick-create-btn {
  height: 34px;
  border-radius: 8px;
  padding: 0 14px;
  font-weight: 600;
  font-size: 13px;
}

.qc-item {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 200px;
}

.qc-item .el-icon { color: var(--brand-dark, #4338ca); }
.qc-label { flex: 1; font-weight: 500; font-size: 13px; }
.qc-tip { font-size: 11px; color: var(--text-3, #94a3b8); }

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
  background: var(--bg-page, #f8fafc);
}

.user-avatar {
  background: linear-gradient(135deg, #6366f1, #06b6d4) !important;
  font-weight: 700;
  font-size: 13px;
}

.user-dropdown-header {
  padding: 10px 14px;
  border-bottom: 1px solid var(--border, #e2e8f0);
  margin-bottom: 4px;
}

.ud-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1, #0f172a);
}

.ud-role {
  font-size: 12px;
  color: var(--text-3, #94a3b8);
  margin-top: 2px;
}

/* ===== Responsive ===== */
@media (max-width: 1024px) {
  .global-search { width: 160px; }
  .global-search.focused, .global-search:hover { width: 240px; }
  :deep(.topbar-project-picker .pp-select) { width: 180px; }
  .crumb-nav { max-width: 200px; }
}

@media (max-width: 768px) {
  .topbar { padding: 0 12px; }
  .topbar-left { gap: 8px; }
  .crumb-nav { display: none; }
  .global-search { display: none; }
  .quick-create-btn { padding: 0 10px; font-size: 12px; }
  :deep(.topbar-project-picker .pp-select) { width: 140px; height: 32px; }
  :deep(.topbar-project-picker .pp-select .el-select__wrapper) { height: 32px; min-height: 32px; border-radius: 8px; }
}
</style>
