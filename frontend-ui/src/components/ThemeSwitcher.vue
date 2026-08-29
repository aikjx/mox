<template>
  <div class="theme-switcher-group" role="group" aria-label="主题切换">
    <button
      v-for="t in availableThemes"
      :key="t.key"
      class="theme-switcher"
      :class="{ active: theme === t.key }"
      :title="t.label"
      @click="setTheme(t.key)"
    >
      <span class="theme-swatch" :style="{ background: t.swatch }"></span>
    </button>
  </div>
</template>

<script setup>
import { useTheme } from '@/composables/useTheme'

const { theme, setTheme, availableThemes } = useTheme()
</script>

<style scoped>
/* 样式继承自 themes/index.css 中的全局 .theme-switcher-group */
</style>
