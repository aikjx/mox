<template>
  <el-dropdown trigger="click" placement="bottom-end" @command="setTheme">
    <div class="theme-switcher-topbar" :title="`主题：${currentThemeLabel}`">
      <span class="theme-swatch" :style="{ background: currentSwatch }"></span>
      <el-icon class="theme-arrow"><ArrowDown /></el-icon>
    </div>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item
          v-for="t in availableThemes"
          :key="t.key"
          :command="t.key"
          :class="{ 'is-active': theme === t.key }"
        >
          <span class="theme-option">
            <span class="theme-swatch-sm" :style="{ background: t.swatch }"></span>
            <span class="theme-label">{{ t.label }}</span>
            <el-icon v-if="theme === t.key" class="theme-check"><Check /></el-icon>
          </span>
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup>
import { computed } from 'vue'
import { ArrowDown, Check } from '@element-plus/icons-vue'
import { useTheme } from '@/composables/useTheme'

const { theme, setTheme, availableThemes } = useTheme()

const currentThemeLabel = computed(() => {
  const t = availableThemes.find(x => x.key === theme.value)
  return t?.label || '主题'
})

const currentSwatch = computed(() => {
  const t = availableThemes.find(x => x.key === theme.value)
  return t?.swatch || 'linear-gradient(135deg, #f6f8fc, #e0e7ff)'
})
</script>

<style scoped>
.theme-switcher-topbar {
  display: flex; align-items: center; gap: 6px;
  height: 36px; padding: 0 10px;
  border-radius: 10px; cursor: pointer;
  transition: all 0.2s;
  color: var(--text-2);
}
.theme-switcher-topbar:hover {
  background: var(--brand-soft);
  color: var(--brand);
}
.theme-swatch {
  width: 18px; height: 18px; border-radius: 50%;
  border: 2px solid var(--border);
  box-shadow: 0 1px 3px rgba(0,0,0,0.1);
}
.theme-arrow {
  font-size: 12px; opacity: 0.7;
}
.theme-option {
  display: flex; align-items: center; gap: 10px; min-width: 160px;
}
.theme-swatch-sm {
  width: 20px; height: 20px; border-radius: 6px;
  border: 1px solid var(--border);
  flex-shrink: 0;
}
.theme-label {
  flex: 1; font-size: 13px; font-weight: 500;
}
.theme-check {
  color: var(--brand); font-size: 14px;
}
:deep(.el-dropdown-menu__item.is-active) {
  color: var(--brand); font-weight: 600;
  background: var(--brand-soft);
}
</style>
