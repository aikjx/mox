<!--
  专家辩论对话框
  职责：辩题输入、专家选择、辩论模式配置、发起辩论
-->
<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="emit('close')"
    title="发起专家辩论"
    width="520px"
    :close-on-click-modal="!submitting"
    class="debate-dialog"
  >
    <el-form label-width="88px" label-position="right">
      <el-form-item label="辩题" required>
        <el-input
          :model-value="topic"
          type="textarea"
          :rows="2"
          placeholder="请输入辩论主题…"
          maxlength="200"
          show-word-limit
          resize="none"
          @update:model-value="emit('update:topic', $event)"
        />
      </el-form-item>

      <el-form-item label="参与专家" required>
        <div class="debate-expert-picker">
          <div class="debate-expert-list">
            <div
              v-for="exp in activeExperts"
              :key="exp.id"
              class="debate-expert-chip"
              :class="{ selected: selectedExpertIds.includes(exp.id) }"
              @click="$emit('toggle-expert', exp.id)"
            >
              <span class="chip-avatar" :style="{ background: expertColor(exp.type) }">
                {{ expertEmoji(exp.type) }}
              </span>
              <span class="chip-name">{{ exp.name }}</span>
              <el-icon v-if="selectedExpertIds.includes(exp.id)" class="chip-check"><CircleCheckFilled /></el-icon>
            </div>
          </div>
          <div class="debate-expert-count">
            已选 <b>{{ selectedExpertIds.length }}</b> 位专家（至少 2 位）
          </div>
        </div>
      </el-form-item>

      <el-form-item label="辩论模式">
        <div class="debate-mode-picker">
          <div
            v-for="opt in modeOptions"
            :key="opt.value"
            class="debate-mode-card"
            :class="{ active: mode === opt.value }"
            @click="$emit('update:mode', opt.value)"
          >
            <div class="mode-icon">{{ opt.icon }}</div>
            <div class="mode-name">{{ opt.label }}</div>
            <div class="mode-desc">{{ opt.desc }}</div>
          </div>
        </div>
      </el-form-item>

      <el-form-item v-if="mode === 'adversarial'" label="辩论轮次">
        <el-input-number :model-value="rounds" :min="1" :max="10" size="small" @update:model-value="emit('update:rounds', $event)" />
        <span class="form-hint">轮</span>
      </el-form-item>

      <el-form-item label="辩论状态">
        <el-tag :type="statusTagType" effect="light" size="small">
          {{ statusLabel }}
        </el-tag>
      </el-form-item>
    </el-form>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="$emit('close')" :disabled="submitting">取消</el-button>
        <el-button
          type="primary"
          :loading="submitting"
          :disabled="!canStart"
          @click="$emit('start')"
        >
          <el-icon><Flag /></el-icon>
          <span>开始辩论</span>
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed } from 'vue'
import { CircleCheckFilled, Flag } from '@element-plus/icons-vue'

const props = defineProps({
  visible: { type: Boolean, default: false },
  topic: { type: String, default: '' },
  selectedExpertIds: { type: Array, default: () => [] },
  mode: { type: String, default: 'adversarial' },
  rounds: { type: Number, default: 3 },
  status: { type: String, default: 'preparing' },
  submitting: { type: Boolean, default: false },
  experts: { type: Array, default: () => [] }
})

const emit = defineEmits(['close', 'start', 'toggle-expert', 'update:topic', 'update:mode', 'update:rounds'])

const activeExperts = computed(() => props.experts.filter(e => e.status === 'active'))

const canStart = computed(() =>
  props.topic.trim() && props.selectedExpertIds.length >= 2
)

const statusLabel = computed(() => {
  const map = { preparing: '准备中', ongoing: '进行中', summarized: '已总结' }
  return map[props.status] || '准备中'
})

const statusTagType = computed(() => {
  const map = { preparing: 'info', ongoing: 'warning', summarized: 'success' }
  return map[props.status] || 'info'
})

const modeOptions = [
  { value: 'adversarial', label: '对抗式辩论', icon: '⚔️', desc: '专家分正反两方，针锋相对' },
  { value: 'roundtable', label: '圆桌式讨论', icon: '圆桌', desc: '多位专家平等交流，各抒己见' }
]

function expertColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#6366f1', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', graph: '#06b6d4',
    security: '#ef4444', performance: '#f97316', monitor: '#14b8a6',
    market: '#8b5cf6', mcp: '#0ea5e9', automation: '#84cc16',
    requirement: '#f43f5e', fusion: '#a855f7', operator: '#64748b',
    custom: '#64748b'
  }
  return colors[type] || '#6366f1'
}

function expertEmoji(type) {
  const emojis = {
    algorithm: '🧮', architecture: '🏗️', data: '🔗',
    ai: '🤖', workflow: '⚡', graph: '🕸️',
    security: '🔒', performance: '🚀', monitor: '📊',
    market: '📈', mcp: '🔌', automation: '🤖',
    requirement: '📋', fusion: '🔀', operator: '⚙️',
    custom: '👤'
  }
  return emojis[type] || '👤'
}
</script>
