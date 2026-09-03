<!--
  历史记录面板
  职责：展示协作历史事件时间线，支持点击跳转
-->
<template>
  <transition name="slide-fade">
    <div v-if="visible" class="ws-history-panel">
      <div class="ws-history-header">
        <span class="ws-history-title">
          <el-icon><RefreshRight /></el-icon>
          协作历史
        </span>
        <button class="ws-history-close" @click="$emit('close')">
          <el-icon><Close /></el-icon>
        </button>
      </div>
      <el-scrollbar class="ws-history-scroll">
        <div class="ws-history-timeline">
          <div
            v-for="(item, idx) in historyEvents"
            :key="item.id"
            class="ws-history-item"
            :class="'event-' + item.type"
            @click="$emit('jump-to-history', item)"
          >
            <div class="ws-history-dot"></div>
            <div class="ws-history-content">
              <div class="ws-history-title-row">
                <span class="ws-history-icon">{{ historyIcon(item.type) }}</span>
                <span class="ws-history-event-title">{{ item.title }}</span>
              </div>
              <div class="ws-history-desc">{{ item.description }}</div>
              <div class="ws-history-time">{{ item.time }}</div>
            </div>
            <div v-if="idx < historyEvents.length - 1" class="ws-history-line"></div>
          </div>
          <el-empty v-if="historyEvents.length === 0" description="暂无历史记录" :image-size="40" />
        </div>
      </el-scrollbar>
    </div>
  </transition>
</template>

<script setup>
import { RefreshRight, Close } from '@element-plus/icons-vue'

defineProps({
  visible: { type: Boolean, default: false },
  historyEvents: { type: Array, default: () => [] }
})

defineEmits(['close', 'jump-to-history'])

function historyIcon(type) {
  const icons = {
    message: '💬', file: '📎', phase: '📊',
    whiteboard: '🎨', mode: '🔄', member: '👥', task: '🎯'
  }
  return icons[type] || '📌'
}
</script>
