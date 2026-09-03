<!--
  KPI 指标卡面板
  职责：展示待办/进行中/已完成/逾期等统计卡片，点击上报事件
-->
<template>
  <div class="ws-kpi-row">
    <div
      v-for="kpi in kpiCards"
      :key="kpi.key"
      class="ws-kpi-card glass-card"
      @click="$emit('kpi-click', kpi.key)"
    >
      <div class="ws-kpi-icon" :style="{ background: kpi.gradient }">
        <span>{{ kpi.icon }}</span>
      </div>
      <div class="ws-kpi-info">
        <div class="ws-kpi-value">{{ kpi.value }}</div>
        <div class="ws-kpi-label">{{ kpi.label }}</div>
      </div>
      <div class="ws-kpi-trend" :class="kpi.trend > 0 ? 'up' : 'down'">
        <el-icon><component :is="kpi.trend > 0 ? 'Top' : 'Bottom'" /></el-icon>
        <span>{{ Math.abs(kpi.trend) }}%</span>
      </div>
      <div class="ws-kpi-gradient-bar" :style="{ background: kpi.gradient }"></div>
    </div>
  </div>
</template>

<script setup>
defineProps({
  kpiCards: { type: Array, default: () => [] }
})

defineEmits(['kpi-click'])
</script>
