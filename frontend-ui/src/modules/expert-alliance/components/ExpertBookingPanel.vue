<template>
  <section class="ab">
    <div class="ab-summary">
      <div v-for="s in summary" :key="s.key" class="ab-cell" :class="{ 'is-active': statusFilter === s.key }">
        <button class="ab-cell-btn" @click="toggle(s.key)">
          <span class="ab-cell-value">{{ s.value }}</span>
          <span class="ab-cell-label">{{ s.label }}</span>
        </button>
      </div>
    </div>

    <p class="ab-caption">后端 /api/experts/bookings/mine 未按登录用户过滤（建单写死 user_id），此处实为全量预约。</p>

    <el-alert v-if="error" type="error" show-icon :closable="false" title="预约列表加载失败" :description="error" />

    <div v-else-if="loading" class="ab-loading"><el-skeleton :rows="4" animated /></div>

    <el-empty v-else-if="!visible.length" :image-size="70" :description="emptyText" />

    <el-table v-else :data="visible" size="small" class="ab-table">
      <el-table-column label="专家" min-width="120">
        <template #default="{ row }">
          <div class="ab-expert">
            <span class="ab-expert-name">{{ row.expertName || row.expertId }}</span>
            <span class="ab-expert-id">{{ row.expertId }}</span>
          </div>
        </template>
      </el-table-column>
      <el-table-column prop="topic" label="主题" min-width="180" show-overflow-tooltip />
      <el-table-column label="计划时间" min-width="150">
        <template #default="{ row }">{{ formatTime(row.scheduledAt) }}</template>
      </el-table-column>
      <el-table-column label="时长" width="80">
        <template #default="{ row }">{{ row.durationMinutes }} 分钟</template>
      </el-table-column>
      <el-table-column label="状态" width="92">
        <template #default="{ row }">
          <el-tag :type="statusTone(row.status)" size="small" effect="light">{{ bookingStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="170" align="right">
        <template #default="{ row }">
          <el-button
            size="small"
            text
            :icon="VideoPlay"
            :disabled="!cancellable(row.status)"
            @click="emit('room', row)"
          >咨询室</el-button>
          <el-popconfirm v-if="cancellable(row.status)" title="确认取消该预约？" @confirm="emit('cancel', row)">
            <template #reference>
              <el-button size="small" type="danger" plain :icon="CircleClose" :loading="pendingId === row.id">取消</el-button>
            </template>
          </el-popconfirm>
        </template>
      </el-table-column>
    </el-table>
  </section>
</template>

<script setup>
import { formatDateTimeLocale } from '@/utils'
import { computed, ref } from 'vue'
import { CircleClose, VideoPlay } from '@element-plus/icons-vue'
import { BOOKING_STATUS, bookingStatusLabel, canCancelBooking } from '@/modules/expert-alliance/contract'

const props = defineProps({
  bookings: { type: Array, default: () => [] },
  counts: { type: Object, default: () => ({ pending: 0, confirmed: 0, completed: 0, cancelled: 0 }) },
  loading: { type: Boolean, default: false },
  error: { type: String, default: '' },
  pendingId: { type: String, default: '' }
})
const emit = defineEmits(['cancel', 'room'])

const statusFilter = ref('')
const cancellable = canCancelBooking

const STATUS_TONE = {
  [BOOKING_STATUS.PENDING]: 'warning',
  [BOOKING_STATUS.CONFIRMED]: 'success',
  [BOOKING_STATUS.COMPLETED]: 'info',
  [BOOKING_STATUS.CANCELLED]: 'danger'
}

const summary = computed(() => [
  { key: '', label: '全部', value: props.bookings.length },
  { key: BOOKING_STATUS.PENDING, label: '待确认', value: props.counts.pending },
  { key: BOOKING_STATUS.CONFIRMED, label: '已确认', value: props.counts.confirmed },
  { key: BOOKING_STATUS.COMPLETED, label: '已完成', value: props.counts.completed },
  { key: BOOKING_STATUS.CANCELLED, label: '已取消', value: props.counts.cancelled }
])

const visible = computed(() => (
  statusFilter.value ? props.bookings.filter((b) => b.status === statusFilter.value) : props.bookings
))
const emptyText = computed(() => (statusFilter.value ? '该状态下没有预约' : '还没有预约，去专家发现页挑选并下单'))

function toggle(key) {
  statusFilter.value = statusFilter.value === key ? '' : key
}

function statusTone(status) {
  return STATUS_TONE[status] ?? 'info'
}

function formatTime(iso) {
  if (!iso) return '—'
  const d = new Date(iso)
  return Number.isNaN(d.getTime()) ? iso : (formatDateTimeLocale(d) ?? iso)
}
</script>

<style scoped>
.ab { display: flex; flex-direction: column; gap: 12px; }
.ab-summary {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 8px;
}
.ab-cell {
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-secondary);
}
.ab-cell.is-active { border-color: var(--accent); }
.ab-cell-btn {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  padding: 8px 10px;
  border: 0;
  background: transparent;
  cursor: pointer;
  color: inherit;
}
.ab-cell-value { font-size: 17px; font-weight: 600; color: var(--text-primary); }
.ab-cell-label { font-size: 11px; color: var(--text-muted); }
.ab-caption { margin: 0; font-size: 11px; color: var(--text-muted); }
.ab-loading { padding: 10px; }
.ab-expert { display: flex; flex-direction: column; }
.ab-expert-name { font-size: 12px; color: var(--text-primary); }
.ab-expert-id { font-size: 11px; color: var(--text-muted); }
</style>
