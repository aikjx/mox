<!--
  通用加载态组件
  骨架屏或 spinner
-->
<template>
  <div class="common-loading-state" :class="type">
    <!-- Spinner 模式 -->
    <template v-if="type === 'spinner'">
      <el-icon class="loading-spinner" :class="{ pulse: pulse }"><Loading /></el-icon>
      <span v-if="text" class="loading-text">{{ text }}</span>
    </template>

    <!-- 骨架屏模式 -->
    <template v-else-if="type === 'skeleton'">
      <div v-for="i in rows" :key="i" class="skeleton-row">
        <div class="skeleton-block" :style="{ width: widths[i % widths.length] }"></div>
      </div>
    </template>

    <!-- 全屏遮罩模式 -->
    <template v-else-if="type === 'fullscreen'">
      <div class="fullscreen-overlay">
        <el-icon class="loading-spinner large"><Loading /></el-icon>
        <span v-if="text" class="loading-text large">{{ text }}</span>
      </div>
    </template>
  </div>
</template>

<script setup>
import { Loading } from '@element-plus/icons-vue'

defineProps({
  type: { type: String, default: 'spinner', validator: (v) => ['spinner', 'skeleton', 'fullscreen'].includes(v) },
  text: { type: String, default: '加载中...' },
  rows: { type: Number, default: 4 },
  pulse: { type: Boolean, default: true },
  widths: { type: Array, default: () => ['100%', '60%', '80%', '40%'] }
})
</script>

<style scoped>
.common-loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
.loading-spinner {
  font-size: 24px;
  color: var(--el-color-primary);
  animation: rotate 1.5s linear infinite;
}
.loading-spinner.large { font-size: 40px; }
.loading-spinner.pulse { animation: rotate 1.5s linear infinite, pulse 1.5s ease-in-out infinite; }
.loading-text { margin-top: 12px; font-size: 14px; color: #909399; }
.loading-text.large { font-size: 16px; }

/* 骨架屏 */
.skeleton-row { margin-bottom: 12px; width: 100%; }
.skeleton-block {
  height: 16px;
  background: linear-gradient(90deg, #f0f0f0 25%, #e0e0e0 50%, #f0f0f0 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite;
  border-radius: 4px;
}

/* 全屏遮罩 */
.fullscreen-overlay {
  position: fixed;
  top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(255, 255, 255, 0.9);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

@keyframes rotate { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
@keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
@keyframes shimmer { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }
</style>
