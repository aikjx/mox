<!--
  通用空态组件
  图标 + 文案 + 操作按钮
-->
<template>
  <div class="common-empty-state">
    <div class="empty-icon" :style="{ fontSize: iconSize + 'px' }">
      <slot name="icon">{{ icon }}</slot>
    </div>
    <div class="empty-text" :style="{ color: textColor }">
      <slot>{{ text }}</slot>
    </div>
    <div v-if="description" class="empty-description">
      <slot name="description">{{ description }}</slot>
    </div>
    <div v-if="actionText || $slots.action" class="empty-action">
      <slot name="action">
        <el-button type="primary" :icon="Plus" @click="$emit('action')">{{ actionText }}</el-button>
      </slot>
    </div>
  </div>
</template>

<script setup>
import { Plus } from '@element-plus/icons-vue'

defineProps({
  icon: { type: String, default: '📭' },
  iconSize: { type: Number, default: 48 },
  text: { type: String, default: '暂无数据' },
  description: { type: String, default: '' },
  actionText: { type: String, default: '' },
  textColor: { type: String, default: '#909399' }
})

defineEmits(['action'])
</script>

<style scoped>
.common-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  text-align: center;
}
.empty-icon { margin-bottom: 16px; opacity: 0.6; }
.empty-text { font-size: 14px; margin-bottom: 8px; }
.empty-description { font-size: 12px; color: #c0c4cc; margin-bottom: 16px; }
.empty-action { margin-top: 8px; }
</style>
