<template>
  <aside class="module-sidebar" :class="{ collapsed: collapsed }">
    <div class="module-sidebar-inner">
      <!-- Header -->
      <div class="ms-header">
        <div class="ms-header-text">
          <div class="ms-title">{{ currentModule.title }}</div>
          <div class="ms-subtitle">{{ currentModule.subtitle }}</div>
        </div>
        <button class="ms-collapse-btn" @click="$emit('toggle-collapse')" :title="collapsed ? '展开' : '收起'">
          <span class="ms-collapse-icon">{{ collapsed ? '›' : '‹' }}</span>
        </button>
      </div>

      <!-- Search -->
      <div class="ms-search" @click="focusTopbarSearch">
        <span class="ms-search-icon">🔍</span>
        <span class="ms-search-placeholder">搜索...</span>
        <kbd class="ms-search-kbd">⌘K</kbd>
      </div>

      <!-- Nav sections -->
      <div class="ms-nav">
        <template v-for="section in currentModule.sections" :key="section.title">
          <div class="ms-section-title">{{ section.title }}</div>
          <div class="ms-section-items">
            <div
              v-for="item in section.items"
              :key="item.key"
              class="ms-sidebar-item"
              :class="{ active: activeItemKey === item.key, navigable: !!item.path }"
              @click="onItemActivate(item)"
            >
              <el-icon v-if="iconOf(item.icon)" class="ms-item-icon"><component :is="iconOf(item.icon)" /></el-icon>
              <span v-else class="ms-item-icon">{{ item.icon }}</span>
              <span class="ms-item-label">{{ item.label }}</span>
              <span v-if="item.count != null" class="ms-item-count">{{ item.count }}</span>
            </div>
          </div>
        </template>
      </div>
    </div>
  </aside>
</template>

<script setup>
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useActiveModule } from '@/composables'
import { navIcon } from '@/modules'

defineProps({
  collapsed: { type: Boolean, default: false },
  isAIFullscreen: { type: Boolean, default: false }
})

defineEmits(['toggle-collapse'])

const router = useRouter()
const { route, config: currentModule } = useActiveModule()

// 高亮态由路由派生，不再依赖点击本地状态（点过就消失的问题即此）
const activeItemKey = computed(() => {
  const items = currentModule.value.sections.flatMap(s => s.items || [])
  return items.find(i => i.path && i.path === route.path)?.key ?? ''
})

function onItemActivate(item) {
  if (!item.path || item.path === route.path) return
  router.push(item.path)
}

// 显式走登记表而不是靠全局注册：本处除了 Element Plus 图标名还要兼容旧配置的 emoji，
// 未登记的名称返回 null ⇒ 落到 v-else 分支原样显示文字，比空白可诊断。
const iconOf = (name) => navIcon(name)

function focusTopbarSearch() {
  // 触发全局搜索快捷键
  window.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, bubbles: true }))
}

// 保留 expose 接口（App.vue 可能调用）
defineExpose({ refreshHealth: () => {} })
</script>

<style scoped>
.module-sidebar {
  width: 240px;
  flex-shrink: 0;
  background: var(--bg-tertiary);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  z-index: 9;
  overflow: hidden;
}

.module-sidebar.collapsed {
  width: 0;
  border-right: none;
}

.module-sidebar-inner {
  width: 240px;
  height: 100%;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

/* Header */
.ms-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.ms-header-text {
  min-width: 0;
  flex: 1;
}

.ms-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.ms-subtitle {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
}

.ms-collapse-btn {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-secondary, var(--text-tertiary));
  cursor: pointer;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  transition: all 0.15s ease;
}
.ms-collapse-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.ms-collapse-icon {
  font-size: 16px;
  font-weight: 600;
  line-height: 1;
}

/* Search */
.ms-search {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin: 12px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}
.ms-search:hover {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(99,102,241,.1);
}

.ms-search-icon {
  font-size: 14px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.ms-search-placeholder {
  font-size: 12px;
  color: var(--text-muted);
  flex: 1;
}

.ms-search-kbd {
  font-size: 10px;
  padding: 2px 5px;
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-muted);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  flex-shrink: 0;
}

/* Nav */
.ms-nav {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 8px;
}
.ms-nav::-webkit-scrollbar {
  width: 4px;
}
.ms-nav::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 2px;
}

.ms-section-title {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 8px 12px 6px;
}

.ms-section-items {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.ms-sidebar-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  color: var(--text-secondary, var(--text-tertiary));
  cursor: default;
  transition: all 0.15s ease;
}
/* 只有带 path 的条目可跳转；无 path 的旧配置项此前伪装成可点，是 FE-05 缺陷 */
.ms-sidebar-item.navigable {
  cursor: pointer;
}
.ms-sidebar-item:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}
.ms-sidebar-item.active {
  background: var(--accent-dim, rgba(99,102,241,.15));
  color: var(--accent-light);
  font-weight: 500;
}

.ms-item-icon {
  font-size: 16px;
  width: 20px;
  text-align: center;
  flex-shrink: 0;
}

.ms-item-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ms-item-count {
  margin-left: auto;
  font-size: 11px;
  padding: 1px 6px;
  background: var(--bg-card);
  border-radius: 10px;
  color: var(--text-muted);
  flex-shrink: 0;
}
.ms-sidebar-item.active .ms-item-count {
  /* 徽标是"字压在色块上"，走填充档位那一对；--accent 在多数皮肤里是亮档，压白字只有 1.8~4:1 */
  background: var(--brand-fill);
  color: var(--on-brand);
}
</style>
