<!--
  专家联盟面板（左栏）
  职责：专家筛选/列表、协作会话列表、快捷工具入口
-->
<template>
  <aside
    class="ws-panel ws-panel-left"
    :class="{ collapsed: collapsed }"
  >
    <div class="ws-panel-header">
      <span v-if="!collapsed" class="ws-panel-title">
        <span class="ws-panel-icon">👥</span>
        专家联盟
        <el-tag size="small" type="success" effect="light" class="ws-online-tag">
          {{ onlineExpertCount }} 在线
        </el-tag>
      </span>
      <button class="ws-panel-toggle" @click="$emit('toggle-collapse')" :title="collapsed ? '展开' : '收起'">
        <el-icon v-if="!collapsed"><ArrowLeft /></el-icon>
        <el-icon v-else><ArrowRight /></el-icon>
      </button>
    </div>

    <div v-if="!collapsed" class="ws-panel-body">
      <!-- 专家筛选搜索 -->
      <div class="ws-expert-filter">
        <el-select v-model="filterType" placeholder="类型" clearable size="small" class="ws-filter-select">
          <el-option v-for="(label, key) in EXPERT_TYPES" :key="key" :label="label" :value="key" />
        </el-select>
        <el-input v-model="searchKeyword" placeholder="搜索专家…" clearable size="small" class="ws-filter-search">
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
      </div>

      <!-- 专家列表 -->
      <div class="ws-expert-section">
        <div class="ws-section-label">
          <span>专家列表</span>
          <div class="ws-section-actions">
            <el-button size="small" text class="ws-smart-match-btn" @click="$emit('open-smart-route')">
              <el-icon><Compass /></el-icon>
              智能匹配
            </el-button>
            <span class="ws-section-count">{{ filteredExperts.length }} 位</span>
          </div>
        </div>
        <el-scrollbar class="ws-expert-scroll">
          <div
            v-for="expert in filteredExperts"
            :key="expert.id"
            class="ws-expert-item expert-card"
            :class="{ active: activeExpert?.id === expert.id, selected: isExpertSelected(expert.id) }"
            @click="$emit('expert-click', expert)"
          >
            <div class="ws-expert-avatar gradient-avatar" :style="{ background: expertGradient(expert.type) }">
              {{ expertEmoji(expert.type) }}
              <span class="ws-expert-status-dot" :class="'dot-' + expert.status" :title="expertStatusText(expert.status)"></span>
            </div>
            <div class="ws-expert-info">
              <div class="ws-expert-name-row">
                <span class="ws-expert-name">{{ expert.name }}</span>
                <span v-if="expert.metrics?.success_rate" class="ws-expert-rate" :style="{ color: expertColor(expert.type) }">
                  {{ (expert.metrics.success_rate * 100).toFixed(0) }}%
                </span>
              </div>
              <div class="ws-expert-role">{{ EXPERT_TYPES[expert.type] || expert.type }}</div>
              <div v-if="expert.capabilities?.length" class="ws-expert-tags">
                <span v-for="cap in expert.capabilities.slice(0, 2)" :key="cap" class="ws-cap-tag" :style="{ borderColor: expertColor(expert.type) + '40', color: expertColor(expert.type) }">{{ cap }}</span>
              </div>
            </div>
            <div v-if="isExpertSelected(expert.id)" class="ws-expert-check">
              <el-icon><CircleCheckFilled /></el-icon>
            </div>
            <div v-else class="ws-expert-status-badge" :class="'badge-' + expert.status">
              {{ expertStatusText(expert.status) }}
            </div>
          </div>
          <el-empty v-if="filteredExperts.length === 0 && expertsLoading" description="加载中…" :image-size="40" />
          <el-empty v-else-if="filteredExperts.length === 0" description="暂无匹配专家" :image-size="40" />
        </el-scrollbar>
      </div>

      <!-- 协作会话 -->
      <div class="ws-expert-section">
        <div class="ws-section-label">
          <span>协作会话</span>
          <el-button size="small" text class="ws-add-btn" @click="$emit('new-collaboration')">
            <el-icon><Plus /></el-icon>
            新建
          </el-button>
        </div>
        <el-scrollbar class="ws-session-scroll">
          <div
            v-for="session in sessions"
            :key="session.id"
            class="ws-session-item"
            :class="{ active: activeSession?.id === session.id }"
            @click="$emit('select-session', session)"
          >
            <div class="ws-session-title">{{ session.title }}</div>
            <div class="ws-session-meta">
              <span class="ws-session-experts">
                {{ session.expert_count || 0 }} 位专家
              </span>
              <span class="ws-session-time">{{ formatTime(session.updated_at || session.created_at) }}</span>
            </div>
            <div v-if="session.mode" class="ws-session-mode">
              <el-tag size="small" :type="sessionModeType(session.mode)" effect="light">
                {{ sessionModeLabel(session.mode) }}
              </el-tag>
            </div>
          </div>
          <el-empty v-if="sessions.length === 0 && sessionsLoading" description="加载中…" :image-size="30" />
          <el-empty v-else-if="sessions.length === 0" description="暂无会话" :image-size="30" />
        </el-scrollbar>
      </div>

      <!-- 快捷工具 -->
      <div class="ws-expert-section">
        <div class="ws-section-label">快捷工具</div>
        <div class="ws-tool-grid">
          <button class="ws-tool-btn tool-card" :class="{ active: activeMode === 'debate' }" @click="$emit('open-debate')">
            <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #ef4444, #f97316)">
              <span class="ws-tool-icon">⚔️</span>
            </div>
            <span>专家辩论</span>
          </button>
          <button class="ws-tool-btn tool-card" :class="{ active: activeMode === 'orchestration' }" @click="$emit('trigger-orchestration')">
            <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #7c3aed, #06b6d4)">
              <span class="ws-tool-icon">🎯</span>
            </div>
            <span>任务编排</span>
          </button>
          <button class="ws-tool-btn tool-card" @click="$emit('trigger-voting')">
            <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #10b981, #14b8a6)">
              <span class="ws-tool-icon">🗳️</span>
            </div>
            <span>融合投票</span>
          </button>
          <button class="ws-tool-btn tool-card" :class="{ active: activeMode === 'collaboration' }" @click="$emit('open-multi-consult')">
            <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #8b5cf6, #ec4899)">
              <span class="ws-tool-icon">💬</span>
            </div>
            <span>多专家咨询</span>
          </button>
          <button class="ws-tool-btn tool-card" @click="$emit('open-register')">
            <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #f59e0b, #ef4444)">
              <span class="ws-tool-icon">➕</span>
            </div>
            <span>注册专家</span>
          </button>
          <button class="ws-tool-btn tool-card" @click="$emit('open-smart-route')">
            <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #06b6d4, #3b82f6)">
              <span class="ws-tool-icon">🧭</span>
            </div>
            <span>智能匹配</span>
          </button>
        </div>
      </div>
    </div>

    <!-- 折叠状态图标列表 -->
    <div v-else class="ws-collapsed-icons">
      <button
        v-for="expert in filteredExperts.slice(0, 6)"
        :key="expert.id"
        class="ws-collapsed-avatar"
        :title="expert.name"
        @click="$emit('expand-and-select', expert)"
      >
        <div class="ws-collapsed-avatar-inner" :style="{ background: expertColor(expert.type) }">
          {{ expertEmoji(expert.type) }}
        </div>
      </button>
      <el-divider class="ws-collapsed-divider" />
      <button class="ws-collapsed-icon-btn" title="新建会话" @click="$emit('expand-and-new-session')">
        <el-icon><Plus /></el-icon>
      </button>
    </div>
  </aside>
</template>

<script setup>
import { ref, computed } from 'vue'
import { Search, ArrowLeft, ArrowRight, Plus, Compass, CircleCheckFilled } from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'

const props = defineProps({
  collapsed: { type: Boolean, default: false },
  experts: { type: Array, default: () => [] },
  expertsLoading: { type: Boolean, default: false },
  activeExpert: { type: Object, default: null },
  selectedExpertIds: { type: Array, default: () => [] },
  sessions: { type: Array, default: () => [] },
  sessionsLoading: { type: Boolean, default: false },
  activeSession: { type: Object, default: null },
  activeMode: { type: String, default: 'collaboration' }
})

defineEmits([
  'toggle-collapse', 'expert-click', 'select-session', 'new-collaboration',
  'open-debate', 'trigger-orchestration', 'trigger-voting', 'open-multi-consult',
  'open-register', 'open-smart-route', 'expand-and-select', 'expand-and-new-session'
])

const filterType = ref('')
const searchKeyword = ref('')

const onlineExpertCount = computed(() =>
  props.experts.filter(e => e.status === 'active').length
)

const filteredExperts = computed(() => {
  let list = props.experts
  if (filterType.value) {
    list = list.filter(e => e.type === filterType.value)
  }
  if (searchKeyword.value) {
    const kw = searchKeyword.value.toLowerCase()
    list = list.filter(e =>
      (e.name || '').toLowerCase().includes(kw) ||
      (e.type || '').toLowerCase().includes(kw) ||
      (e.capabilities || []).some(c => (c || '').toLowerCase().includes(kw))
    )
  }
  return list
})

function isExpertSelected(id) {
  return props.selectedExpertIds.includes(id)
}

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

function expertGradient(type) {
  const gradients = {
    algorithm: 'linear-gradient(135deg, #6366f1, #8b5cf6)',
    architecture: 'linear-gradient(135deg, #6366f1, #06b6d4)',
    data: 'linear-gradient(135deg, #10b981, #14b8a6)',
    ai: 'linear-gradient(135deg, #ec4899, #8b5cf6)',
    workflow: 'linear-gradient(135deg, #f59e0b, #ef4444)',
    graph: 'linear-gradient(135deg, #06b6d4, #3b82f6)',
    security: 'linear-gradient(135deg, #ef4444, #f97316)',
    performance: 'linear-gradient(135deg, #f97316, #f59e0b)',
    monitor: 'linear-gradient(135deg, #14b8a6, #10b981)',
    market: 'linear-gradient(135deg, #8b5cf6, #ec4899)',
    mcp: 'linear-gradient(135deg, #0ea5e9, #06b6d4)',
    automation: 'linear-gradient(135deg, #84cc16, #10b981)',
    requirement: 'linear-gradient(135deg, #f43f5e, #ec4899)',
    fusion: 'linear-gradient(135deg, #a855f7, #7c3aed)',
    operator: 'linear-gradient(135deg, #64748b, #475569)',
    custom: 'linear-gradient(135deg, #64748b, #475569)'
  }
  return gradients[type] || 'linear-gradient(135deg, #7c3aed, #06b6d4)'
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

function expertStatusText(status) {
  const map = { active: '在线', busy: '忙碌', offline: '离线', idle: '空闲' }
  return map[status] || '在线'
}

function formatTime(ts) {
  if (!ts) return ''
  const now = Date.now()
  const diff = now - ts
  if (diff < 60000) return '刚刚'
  if (diff < 3600000) return Math.floor(diff / 60000) + '分钟前'
  if (diff < 86400000) return Math.floor(diff / 3600000) + '小时前'
  if (diff < 604800000) return Math.floor(diff / 86400000) + '天前'
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()}`
}

function sessionModeLabel(mode) {
  const map = { smart: '智能路由', single: '单专家', multi: '多专家', debate: '辩论', algorithm: '算法分析' }
  return map[mode] || '协作'
}

function sessionModeType(mode) {
  const map = { smart: 'info', single: 'primary', multi: 'success', debate: 'warning', algorithm: 'danger' }
  return map[mode] || 'info'
}
</script>
