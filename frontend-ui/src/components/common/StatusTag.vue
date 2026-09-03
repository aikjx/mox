<!--
  通用状态标签组件
  根据状态值显示不同颜色的 el-tag
-->
<template>
  <el-tag
    :type="tagType"
    :effect="effect"
    :size="size"
    :round="round"
    :closable="closable"
    :disable-transitions="disableTransitions"
    @close="$emit('close')"
  >
    <span v-if="showDot" class="status-dot" :style="{ background: dotColor }"></span>
    <slot>{{ label }}</slot>
  </el-tag>
</template>

<script setup>
import { computed } from 'vue'

const props = defineProps({
  status: { type: [String, Number], required: true },
  statusMap: {
    type: Object,
    default: () => ({
      pending: { label: '待处理', type: 'info', color: '#909399' },
      processing: { label: '进行中', type: 'primary', color: '#409eff' },
      success: { label: '已完成', type: 'success', color: '#67c23a' },
      warning: { label: '警告', type: 'warning', color: '#e6a23c' },
      error: { label: '失败', type: 'danger', color: '#f56c6c' },
      active: { label: '活跃', type: 'success', color: '#67c23a' },
      inactive: { label: '未激活', type: 'info', color: '#909399' },
      archived: { label: '已归档', type: 'info', color: '#909399' }
    })
  },
  effect: { type: String, default: 'light' },
  size: { type: String, default: 'default' },
  round: { type: Boolean, default: false },
  closable: { type: Boolean, default: false },
  disableTransitions: { type: Boolean, default: false },
  showDot: { type: Boolean, default: false }
})

defineEmits(['close'])

const config = computed(() => props.statusMap[props.status] || { label: String(props.status), type: 'info', color: '#909399' })
const label = computed(() => config.value.label)
const tagType = computed(() => config.value.type || 'info')
const dotColor = computed(() => config.value.color || '#909399')
</script>

<style scoped>
.status-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  margin-right: 6px;
  vertical-align: middle;
}
</style>
