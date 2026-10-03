<template>
  <section class="event-connection" aria-label="实时更新状态">
    <div role="status" aria-live="polite" aria-atomic="true">
      <strong>{{ label }}</strong>
      <span v-if="connection.message"> · {{ connection.message }}</span>
    </div>
    <button type="button" :disabled="busy" @click="$emit('reconnect')">重新连接</button>
  </section>
</template>
<script setup>
import { computed } from 'vue'
import { EVENT_CONNECTION } from '@/modules/expert-alliance/contract'
const props = defineProps({ connection: { type: Object, required: true } })
defineEmits(['reconnect'])
const labels = {
  [EVENT_CONNECTION.IDLE]: '实时更新未连接',
  [EVENT_CONNECTION.CONNECTING]: '正在连接实时更新',
  [EVENT_CONNECTION.REFRESHING]: '正在刷新页面数据',
  [EVENT_CONNECTION.LIVE]: '实时更新已连接',
  [EVENT_CONNECTION.DISCONNECTED]: '实时更新已断开',
  [EVENT_CONNECTION.ERROR]: '实时更新需要处理'
}
const label = computed(() => labels[props.connection.state] || '实时更新状态未知')
const busy = computed(() => [EVENT_CONNECTION.CONNECTING, EVENT_CONNECTION.REFRESHING].includes(props.connection.state))
</script>
<style scoped>
.event-connection { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; padding: 12px; margin-bottom: 12px; border: 1px solid var(--border); border-radius: var(--radius-md); color: var(--text-secondary); background: var(--bg-card); }
.event-connection strong { color: var(--text-primary); }
.event-connection button { padding: 6px 12px; border: 1px solid var(--border); border-radius: var(--radius-md); color: var(--text-primary); background: var(--bg-card); cursor: pointer; }
.event-connection button:disabled { cursor: wait; opacity: .6; }
.event-connection button:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
