<!--
  多专家咨询对话框
  职责：问题输入、专家选择、咨询模式、结果展示（对比/列表）
-->
<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="emit('close')"
    title="多专家咨询"
    width="560px"
    :close-on-click-modal="!submitting"
    class="multi-consult-dialog"
  >
    <el-form label-width="88px" label-position="right">
      <el-form-item label="咨询问题" required>
        <el-input
          :model-value="question"
          type="textarea"
          :rows="3"
          placeholder="请输入您想咨询的问题…"
          maxlength="500"
          show-word-limit
          resize="none"
          @update:model-value="emit('update:question', $event)"
        />
      </el-form-item>

      <el-form-item label="选择专家" required>
        <div class="consult-expert-picker">
          <div class="consult-expert-list">
            <div
              v-for="exp in activeExperts"
              :key="exp.id"
              class="consult-expert-chip"
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
          <div class="consult-expert-count">
            已选 <b>{{ selectedExpertIds.length }}</b> 位专家
          </div>
        </div>
      </el-form-item>

      <el-form-item label="咨询模式">
        <el-radio-group :model-value="mode" class="consult-mode-group" @update:model-value="emit('update:mode', $event)">
          <el-radio-button value="parallel">
            <span class="mode-icon-inline">⚡</span>
            并行模式
            <span class="mode-hint">（同时回答）</span>
          </el-radio-button>
          <el-radio-button value="serial">
            <span class="mode-icon-inline">🔄</span>
            串行模式
            <span class="mode-hint">（依次回答）</span>
          </el-radio-button>
        </el-radio-group>
      </el-form-item>

      <el-form-item label="结果展示">
        <el-switch
          :model-value="compareView"
          active-text="对比视图"
          inactive-text="列表视图"
          @update:model-value="emit('update:compareView', $event)"
        />
      </el-form-item>
    </el-form>

    <!-- 咨询结果展示 -->
    <div v-if="results.length > 0" class="consult-results-section">
      <div class="results-section-head">
        <span class="results-section-title">
          <el-icon><DocumentCopy /></el-icon>
          咨询结果
        </span>
        <el-tag size="small" type="success" effect="light">
          {{ results.length }} 位专家已回答
        </el-tag>
      </div>

      <!-- 对比视图 -->
      <div v-if="compareView" class="compare-view">
        <div class="compare-grid">
          <div
            v-for="(result, idx) in results"
            :key="idx"
            class="compare-card"
          >
            <div class="compare-card-head" :style="{ borderTopColor: expertColor(result.expert?.type) }">
              <div class="compare-expert">
                <span class="compare-avatar" :style="{ background: expertColor(result.expert?.type) }">
                  {{ expertEmoji(result.expert?.type) }}
                </span>
                <span class="compare-name">{{ result.expert?.name || '专家' }}</span>
              </div>
              <el-tag size="small" type="primary" effect="light" v-if="result.confidence">
                置信度 {{ (result.confidence * 100).toFixed(0) }}%
              </el-tag>
            </div>
            <div class="compare-card-body">
              <div class="compare-content">{{ result.response }}</div>
            </div>
          </div>
        </div>
      </div>

      <!-- 列表视图 -->
      <div v-else class="list-view">
        <div
          v-for="(result, idx) in results"
          :key="idx"
          class="result-item-card"
        >
          <div class="result-item-head">
            <span class="result-avatar" :style="{ background: expertColor(result.expert?.type) }">
              {{ expertEmoji(result.expert?.type) }}
            </span>
            <span class="result-name">{{ result.expert?.name || '专家' }}</span>
            <el-tag v-if="result.confidence" size="small" type="primary" effect="light">
              置信度 {{ (result.confidence * 100).toFixed(0) }}%
            </el-tag>
            <span v-if="result.duration_ms" class="result-duration">{{ (result.duration_ms / 1000).toFixed(1) }}s</span>
          </div>
          <div class="result-item-body">{{ result.response }}</div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="dialog-footer">
        <el-button @click="$emit('close')" :disabled="submitting">关闭</el-button>
        <el-button
          type="primary"
          :loading="submitting"
          :disabled="!canStart"
          @click="$emit('start')"
        >
          <el-icon><Connection /></el-icon>
          <span>开始咨询</span>
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup>
import { computed } from 'vue'
import { CircleCheckFilled, DocumentCopy, Connection } from '@element-plus/icons-vue'

const props = defineProps({
  visible: { type: Boolean, default: false },
  question: { type: String, default: '' },
  selectedExpertIds: { type: Array, default: () => [] },
  mode: { type: String, default: 'parallel' },
  compareView: { type: Boolean, default: false },
  results: { type: Array, default: () => [] },
  submitting: { type: Boolean, default: false },
  experts: { type: Array, default: () => [] }
})

const emit = defineEmits(['close', 'start', 'toggle-expert', 'update:question', 'update:mode', 'update:compareView'])

const activeExperts = computed(() => props.experts.filter(e => e.status === 'active'))

const canStart = computed(() =>
  props.question.trim() && props.selectedExpertIds.length >= 1
)

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
