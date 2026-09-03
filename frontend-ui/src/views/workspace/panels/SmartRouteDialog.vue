<!--
  智能匹配专家对话框
  职责：问题描述输入、AI智能路由匹配、推荐结果展示、专家选择
-->
<template>
  <el-dialog
    :model-value="visible"
    @update:model-value="emit('close')"
    title="智能匹配专家"
    width="500px"
    :close-on-click-modal="!loading"
    class="smart-route-dialog"
  >
    <div class="smart-route-intro">
      <div class="intro-icon">🧠</div>
      <div class="intro-text">
        <div class="intro-title">AI 智能路由</div>
        <div class="intro-desc">输入问题描述，系统将自动推荐最匹配的专家</div>
      </div>
    </div>

    <el-form label-width="88px" label-position="right">
      <el-form-item label="问题描述" required>
        <el-input
          :model-value="question"
          type="textarea"
          :rows="3"
          placeholder="请描述您的问题或需求…"
          maxlength="300"
          show-word-limit
          resize="none"
          @update:model-value="emit('update:question', $event)"
          @keyup.enter.ctrl="$emit('do-route')"
        />
      </el-form-item>

      <el-form-item label="推荐数量">
        <el-input-number :model-value="maxExperts" :min="1" :max="6" size="small" @update:model-value="emit('update:maxExperts', $event)" />
        <span class="form-hint">位专家</span>
      </el-form-item>
    </el-form>

    <div class="smart-route-action">
      <el-button
        type="primary"
        :loading="loading"
        :disabled="!question.trim()"
        @click="$emit('do-route')"
        class="smart-route-btn"
      >
        <el-icon><Compass /></el-icon>
        <span>{{ loading ? '匹配中…' : '开始智能匹配' }}</span>
      </el-button>
    </div>

    <!-- 匹配结果 -->
    <div v-if="result" class="smart-route-results">
      <div class="route-result-head">
        <span class="route-result-title">匹配结果</span>
        <el-tag size="small" type="success" effect="light">
          {{ result.selected?.length || 0 }} 位推荐
        </el-tag>
      </div>

      <div class="route-expert-list">
        <div
          v-for="(item, idx) in result.selected || []"
          :key="item.id || idx"
          class="route-expert-item"
        >
          <div class="route-rank">{{ idx + 1 }}</div>
          <div class="route-avatar" :style="{ background: expertColor(item.type || item.expert_type) }">
            {{ expertEmoji(item.type || item.expert_type) }}
          </div>
          <div class="route-info">
            <div class="route-name">{{ item.name || item.expert_name }}</div>
            <div class="route-type">{{ EXPERT_TYPES[item.type || item.expert_type] || item.type || '专家' }}</div>
            <div v-if="item.reason" class="route-reason">{{ item.reason }}</div>
          </div>
          <div class="route-score">
            <div class="score-ring" :style="{ '--score': item.score || item.confidence || 0 }">
              <span>{{ ((item.score || item.confidence || 0) * 100).toFixed(0) }}%</span>
            </div>
            <span class="score-label">匹配度</span>
          </div>
          <el-button
            size="small"
            type="primary"
            plain
            class="route-select-btn"
            @click="$emit('select-expert', item)"
          >选择</el-button>
        </div>
      </div>

      <div class="route-actions-footer">
        <el-button size="small" @click="$emit('select-all')">
          <el-icon><CircleCheckFilled /></el-icon>
          一键选择全部推荐
        </el-button>
      </div>
    </div>

    <div v-else-if="loading" class="smart-route-loading">
      <el-icon class="is-loading loading-spinner"><Loading /></el-icon>
      <span>正在分析您的问题并匹配专家…</span>
    </div>
  </el-dialog>
</template>

<script setup>
import { Compass, CircleCheckFilled, Loading } from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'

defineProps({
  visible: { type: Boolean, default: false },
  question: { type: String, default: '' },
  maxExperts: { type: Number, default: 3 },
  loading: { type: Boolean, default: false },
  result: { type: Object, default: null }
})

const emit = defineEmits(['close', 'do-route', 'select-expert', 'select-all', 'update:question', 'update:maxExperts'])

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
