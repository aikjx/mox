<!--
  工作台顶部全局工具栏
  职责：Logo、项目选择器、工作模式切换、全局搜索、AI助手入口、通知、用户头像
-->
<template>
  <header class="ws-header glass-header">
    <div class="ws-header-gradient-bar"></div>

    <div class="ws-header-left">
      <div class="ws-logo">
        <div class="ws-logo-icon-wrap">
          <span class="ws-logo-icon">🕸️</span>
        </div>
        <span class="ws-logo-text">专家联盟工作台</span>
      </div>
      <el-divider direction="vertical" class="ws-header-divider" />
      <div class="ws-project-selector">
        <el-select :model-value="currentProject" size="small" class="ws-project-select" @update:model-value="$emit('update:currentProject', $event)" @change="$emit('project-change')">
          <el-option v-for="p in projectOptions" :key="p.id" :label="p.name" :value="p.id" />
        </el-select>
      </div>
    </div>

    <div class="ws-header-center">
      <div class="ws-mode-tabs glass-tabs">
        <button
          v-for="(mode, idx) in workModes"
          :key="mode.key"
          class="ws-mode-tab"
          :class="{ active: activeMode === mode.key, 'mode-enter': modeTransitioning }"
          @click="$emit('switch-mode', mode.key)"
        >
          <div class="ws-mode-icon-wrap" :style="{ background: mode.gradient }">
            <el-icon class="ws-mode-icon"><component :is="mode.iconComp" /></el-icon>
          </div>
          <span class="ws-mode-label">{{ mode.label }}</span>
          <span class="ws-mode-shortcut">Ctrl+{{ idx + 1 }}</span>
        </button>
      </div>
    </div>

    <div class="ws-header-right">
      <div class="ws-global-search glass-search">
        <el-icon class="search-icon"><Search /></el-icon>
        <el-input
          :model-value="globalSearch"
          class="ws-search-input"
          placeholder="全局搜索：专家 / 文档 / 节点…"
          clearable
          @update:model-value="$emit('update:globalSearch', $event)"
          @keyup.enter="$emit('global-search')"
          @clear="$emit('global-search')"
        >
          <template #append>
            <span class="ws-search-kbd">⌘K</span>
          </template>
        </el-input>
      </div>
      <el-button size="small" class="ws-ai-btn gradient-btn" @click="$emit('open-ai')">
        <el-icon><MagicStick /></el-icon>
        <span>AI 协作</span>
      </el-button>
      <el-badge :value="notifCount" :hidden="!hasNotifications" class="ws-notif-badge">
        <el-button size="small" text class="ws-icon-btn" title="通知">
          <el-icon><Bell /></el-icon>
        </el-button>
      </el-badge>
      <div class="ws-user-avatar-wrap">
        <el-avatar :size="36" class="ws-avatar gradient-avatar">U</el-avatar>
        <span class="ws-avatar-online-dot"></span>
      </div>
    </div>
  </header>
</template>

<script setup>
import { Search, MagicStick, Bell } from '@element-plus/icons-vue'

defineProps({
  currentProject: { type: String, default: '' },
  projectOptions: { type: Array, default: () => [] },
  activeMode: { type: String, default: 'collaboration' },
  modeTransitioning: { type: Boolean, default: false },
  workModes: { type: Array, default: () => [] },
  globalSearch: { type: String, default: '' },
  notifCount: { type: Number, default: 0 },
  hasNotifications: { type: Boolean, default: true }
})

defineEmits(['update:currentProject', 'update:globalSearch', 'project-change', 'switch-mode', 'global-search', 'open-ai'])
</script>
