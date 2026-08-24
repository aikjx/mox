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
            <template v-for="g in NAV_GROUPS" :key="g.key">
              <div class="nav-group" v-show="!collapsed">{{ g.label }}</div>
              <router-link
                v-for="m in modulesByGroup(g.key)"
                :key="m.key"
                :to="m.path"
                class="nav-item"
                :class="{ active: isActive(m.path) }"
              >
                <span class="nav-bar"></span>
                <el-icon class="nav-icon"><component :is="m.icon" /></el-icon>
                <span v-show="!collapsed" class="nav-label">{{ m.label }}</span>
              </router-link>
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
        <header class="topbar">
          <div class="topbar-left">
            <el-icon class="collapse-btn" @click="collapsed = !collapsed">
              <Fold v-if="!collapsed" />
              <Expand v-else />
            </el-icon>
            <el-breadcrumb separator="/">
              <el-breadcrumb-item :to="{ path: '/dashboard' }">首页</el-breadcrumb-item>
              <el-breadcrumb-item v-for="b in crumbs" :key="b.path">{{ b.label }}</el-breadcrumb-item>
              <el-breadcrumb-item v-if="crumbsExtra" class="crumb-extra">{{ crumbsExtra }}</el-breadcrumb-item>
            </el-breadcrumb>
          </div>

          <div class="topbar-right">
            <!-- 全局搜索输入框（企业级「命令面板」入口，Ctrl+K / Ctrl+Shift+P 聚焦） -->
            <div class="global-search" :class="{ focused: searchFocused }" @click="focusSearch">
              <el-icon class="search-icon"><Search /></el-icon>
              <input
                ref="searchInputRef"
                v-model="searchText"
                class="search-input"
                placeholder="搜索 36 模块 / 任务 / 算子…（Ctrl/⌘ + K）"
                @focus="searchFocused = true"
                @blur="searchFocused = false"
                @keyup.enter="doGlobalSearch"
                @keydown.esc="searchFocused = false; searchText = ''; $event.target.blur()"
              />
              <kbd v-if="!searchFocused" class="kbd">Ctrl + K</kbd>
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
    </div>
  </el-config-provider>
</template>

<script setup>
import { ref, computed, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import {
  Fold, Expand, Refresh, Document, ArrowDown, Close, Search, QuestionFilled, Lightning
} from '@element-plus/icons-vue'
import {
  NAV_MODULES, APP_VERSION, NAV_GROUPS, QUICK_CREATE_COMMANDS, HOTKEY_GROUPS
} from '@/types'
import { getHealth } from '@/api'
import { useGlobalShortcuts } from '@/globalShortcuts'

const route = useRoute()
const router = useRouter()

const collapsed = ref(false)
const health = ref({ status: 'pending', label: '连接中…' })
const searchInputRef = ref(null)
const searchText = ref('')
const searchFocused = ref(false)
const helpDrawerOpen = ref(false)
let disposeShortcuts = null

// 二级面包屑（路由 meta.subLabel 或监听子 Dialog 打开后的全局 crumb 事件）
const crumbsExtra = computed(() => route.meta?.subLabel || null)

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
function modulesByGroup(gKey) {
  const g = NAV_GROUPS.find((x) => x.key === gKey)
  if (!g) return []
  const set = new Set(g.items)
  return NAV_MODULES.filter((m) => set.has(m.key))
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
    const ok = r.status === 'ok' || r.status === 'running'
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
function doGlobalSearch() {
  const t = String(searchText.value || '').trim()
  if (!t) return
  // 命中 1：NAV_MODULES 精确匹配 label/key → 直接跳
  const mod = NAV_MODULES.find((m) => m.label.includes(t) || m.key.toLowerCase().includes(t.toLowerCase()))
  if (mod) {
    router.push(mod.path)
    return
  }
  // 命中 2：搜索任务 → 跳 /tasks?q=xxx
  router.push({ path: '/tasks', query: { q: t } })
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

// === 全局快捷键绑定 ===
onMounted(() => {
  refreshHealth()
  disposeShortcuts = useGlobalShortcuts({
    focusSearch,
    toggleHelpDrawer: () => { helpDrawerOpen.value = !helpDrawerOpen.value },
    navModules: NAV_MODULES.slice(0, 9),
    pushRoute: (p) => router.push(p)
  })
  // /market?action=upload → 自动触发 MarketView 打开上传对话框（Query 驱动无状态化）
  if (route.path === '/market' && route.query?.action === 'upload') {
    window.dispatchEvent(new CustomEvent('xuanji:open-market-upload'))
  }
})
watch(() => route.path, () => {
  refreshHealth()
  // 如果跳转到 /market?action=upload 或 /tasks?action=create
  if (route.path === '/market' && route.query?.action === 'upload') {
    window.dispatchEvent(new CustomEvent('xuanji:open-market-upload'))
  }
  if (route.path === '/tasks' && route.query?.action === 'create') {
    window.dispatchEvent(new CustomEvent('xuanji:open-create-task'))
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
.topbar-left { display: flex; align-items: center; gap: 16px; }
.collapse-btn { font-size: 20px; cursor: pointer; color: var(--text-2); }
.collapse-btn:hover { color: var(--brand); }
.crumb-extra { color: var(--brand-dark); font-weight: 600; }
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
.content {
  flex: 1; overflow-y: auto; padding: 22px;
  background:
    radial-gradient(1200px 400px at 100% 0%, rgba(99, 102, 241, 0.05), transparent),
    radial-gradient(900px 360px at 0% 100%, rgba(6, 182, 212, 0.05), transparent),
    var(--bg-page);
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
</style>
