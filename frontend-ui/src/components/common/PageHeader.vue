<!--
  通用页面头部组件
  标题 + 描述 + 操作按钮区
-->
<template>
  <div class="common-page-header" :class="{ bordered: showBorder }">
    <div class="page-header-left">
      <!-- 返回按钮 -->
      <el-button v-if="showBack" text class="back-btn" @click="$emit('back')">
        <el-icon><ArrowLeft /></el-icon>
      </el-button>

      <div class="page-header-content">
        <div class="page-title-row">
          <h2 class="page-title">
            <slot name="title">{{ title }}</slot>
          </h2>
          <!-- 标题右侧标签 -->
          <div v-if="tags.length || $slots.tags" class="page-tags">
            <slot name="tags">
              <el-tag v-for="tag in tags" :key="tag" size="small" effect="light" class="page-tag">{{ tag }}</el-tag>
            </slot>
          </div>
        </div>

        <!-- 描述 -->
        <div v-if="description || $slots.description" class="page-description">
          <slot name="description">{{ description }}</slot>
        </div>

        <!-- 面包屑 -->
        <el-breadcrumb v-if="breadcrumb.length" separator="/" class="page-breadcrumb">
          <el-breadcrumb-item v-for="(item, idx) in breadcrumb" :key="idx" :to="item.to">
            {{ item.label }}
          </el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <!-- 操作按钮区 -->
    <div v-if="$slots.default || actions.length" class="page-header-actions">
      <slot>
        <el-button
          v-for="action in actions"
          :key="action.key"
          :type="action.type || 'primary'"
          :icon="action.icon"
          :loading="action.loading"
          :disabled="action.disabled"
          @click="action.handler?.()"
        >
          {{ action.label }}
        </el-button>
      </slot>
    </div>
  </div>
</template>

<script setup>
import { ArrowLeft } from '@element-plus/icons-vue'

defineProps({
  title: { type: String, default: '' },
  description: { type: String, default: '' },
  tags: { type: Array, default: () => [] },
  breadcrumb: { type: Array, default: () => [] },
  actions: { type: Array, default: () => [] },
  showBack: { type: Boolean, default: false },
  showBorder: { type: Boolean, default: true }
})

defineEmits(['back'])
</script>

<style scoped>
.common-page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 20px 24px;
  background: var(--el-bg-color);
  margin-bottom: 16px;
  border-radius: 8px;
}
.common-page-header.bordered {
  border-bottom: 1px solid var(--el-border-color-lighter);
  border-radius: 0;
  margin-bottom: 0;
}
.page-header-left { display: flex; align-items: flex-start; gap: 12px; flex: 1; min-width: 0; }
.back-btn { padding: 4px; font-size: 18px; margin-top: 2px; }
.page-header-content { flex: 1; min-width: 0; }
.page-title-row { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
.page-title {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  color: var(--el-text-color-primary);
  line-height: 1.4;
}
.page-tags { display: flex; gap: 6px; flex-wrap: wrap; }
.page-tag { font-weight: normal; }
.page-description {
  margin-top: 6px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  line-height: 1.6;
}
.page-breadcrumb { margin-top: 8px; }
.page-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  margin-left: 16px;
}
</style>
