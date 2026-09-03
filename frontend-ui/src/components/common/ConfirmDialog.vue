<!--
  通用确认对话框组件
  封装 el-dialog，用于删除/危险操作确认
-->
<template>
  <el-dialog
    v-model="visible"
    :title="title"
    :width="width"
    :close-on-click-modal="false"
    :close-on-press-escape="!loading"
    :show-close="!loading"
    class="common-confirm-dialog"
    @closed="handleClosed"
  >
    <div class="confirm-content">
      <div class="confirm-icon" :class="type">
        <el-icon v-if="type === 'danger'"><WarningFilled /></el-icon>
        <el-icon v-else-if="type === 'warning'"><Warning /></el-icon>
        <el-icon v-else-if="type === 'success'"><CircleCheckFilled /></el-icon>
        <el-icon v-else><InfoFilled /></el-icon>
      </div>
      <div class="confirm-text">
        <div class="confirm-message" v-html="message"></div>
        <div v-if="description" class="confirm-description">{{ description }}</div>
        <div v-if="showInput" class="confirm-input">
          <el-input v-model="inputValue" :placeholder="inputPlaceholder" @keyup.enter="handleConfirm" />
        </div>
      </div>
    </div>

    <template #footer>
      <div class="confirm-footer">
        <el-button :disabled="loading" @click="handleCancel">{{ cancelText }}</el-button>
        <el-button
          :type="confirmType"
          :loading="loading"
          @click="handleConfirm"
        >
          {{ loading ? loadingText : confirmText }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup>
import { ref, watch, computed } from 'vue'
import { WarningFilled, Warning, CircleCheckFilled, InfoFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'

const props = defineProps({
  modelValue: { type: Boolean, default: false },
  title: { type: String, default: '确认操作' },
  message: { type: String, default: '确定要执行此操作吗？' },
  description: { type: String, default: '' },
  type: { type: String, default: 'warning', validator: (v) => ['danger', 'warning', 'success', 'info'].includes(v) },
  confirmText: { type: String, default: '确定' },
  cancelText: { type: String, default: '取消' },
  loadingText: { type: String, default: '处理中...' },
  width: { type: String, default: '420px' },
  showInput: { type: Boolean, default: false },
  inputPlaceholder: { type: String, default: '请输入确认内容' },
  requireInputMatch: { type: String, default: '' }
})

const emit = defineEmits(['update:modelValue', 'confirm', 'cancel', 'closed'])

const visible = ref(props.modelValue)
const loading = ref(false)
const inputValue = ref('')

watch(() => props.modelValue, (v) => { visible.value = v; if (v) inputValue.value = '' })
watch(visible, (v) => { emit('update:modelValue', v) })

const confirmType = computed(() => {
  const map = { danger: 'danger', warning: 'warning', success: 'success', info: 'primary' }
  return map[props.type] || 'primary'
})

function handleConfirm() {
  if (props.showInput) {
    if (!inputValue.value.trim()) { ElMessage.warning('请输入确认内容'); return }
    if (props.requireInputMatch && inputValue.value.trim() !== props.requireInputMatch) {
      ElMessage.error(`请输入 "${props.requireInputMatch}" 以确认`); return
    }
  }
  loading.value = true
  emit('confirm', { inputValue: inputValue.value, done: () => { loading.value = false; visible.value = false } })
}

function handleCancel() {
  if (loading.value) return
  emit('cancel')
  visible.value = false
}

function handleClosed() {
  loading.value = false
  emit('closed')
}
</script>

<style scoped>
.confirm-content { display: flex; gap: 16px; padding: 8px 0; }
.confirm-icon { font-size: 24px; flex-shrink: 0; margin-top: 2px; }
.confirm-icon.danger { color: #f56c6c; }
.confirm-icon.warning { color: #e6a23c; }
.confirm-icon.success { color: #67c23a; }
.confirm-icon.info { color: #409eff; }
.confirm-text { flex: 1; }
.confirm-message { font-size: 14px; color: #303133; line-height: 1.6; }
.confirm-description { font-size: 12px; color: #909399; margin-top: 8px; }
.confirm-input { margin-top: 16px; }
.confirm-footer { display: flex; justify-content: flex-end; gap: 8px; }
</style>
