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
            <div class="ws-expert-avatar gradient-avatar" :style="{ background: expertGradient(visualKey(expert)) }">
              {{ expertEmoji(visualKey(expert)) }}
              <span class="ws-expert-status-dot" :class="'dot-' + statusClass(expert)" :title="statusText(expert)"></span>
            </div>
            <div class="ws-expert-info">
              <div class="ws-expert-name-row">
                <span class="ws-expert-name">{{ expert.name }}</span>
                <span v-if="expert.metrics?.resolutionRate" class="ws-expert-rate" :style="{ color: expertColor(visualKey(expert)) }">
                  {{ (expert.metrics.resolutionRate * 100).toFixed(0) }}%
                </span>
              </div>
              <div class="ws-expert-role">{{ expertTitleText(expert) }}</div>
              <div v-if="capabilityTags(expert).length" class="ws-expert-tags">
                <span v-for="cap in capabilityTags(expert)" :key="cap" class="ws-cap-tag" :style="{ borderColor: expertColor(visualKey(expert)) + '40', color: expertColor(visualKey(expert)) }">{{ cap }}</span>
              </div>
            </div>
            <div v-if="isExpertSelected(expert.id)" class="ws-expert-check">
              <el-icon><CircleCheckFilled /></el-icon>
            </div>
            <div v-else class="ws-expert-status-badge" :class="'badge-' + statusClass(expert)">
              {{ statusText(expert) }}
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
            v-for="row in sessionRows"
            :key="row.session.id"
            class="ws-session-item"
            :class="{ active: activeSession?.id === row.session.id }"
            @click="$emit('select-session', row.session)"
          >
            <div class="ws-session-title">{{ row.title }}</div>
            <div class="ws-session-meta">
              <span class="ws-session-experts">
                {{ row.expertCount }} 位专家
              </span>
              <span class="ws-session-time">{{ row.timeText }}</span>
            </div>
            <div class="ws-session-mode">
              <el-tag size="small" :type="row.typeTag" effect="light">
                {{ row.typeLabel }}
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
        <div class="ws-collapsed-avatar-inner" :style="{ background: expertColor(visualKey(expert)) }">
          {{ expertEmoji(visualKey(expert)) }}
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
import { EXPERT_TYPES, expertColor, expertGradient, expertEmoji } from '@/constants'
import { availabilityLabel, sessionActivityAt, sessionListTitle, sessionTypeLabel, sessionTypeTagType } from '@/modules/expert-alliance/contract'
import { expertStatusClass, expertVisualKey, sessionTimeText } from '@/modules/expert-alliance/model'

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

// 行来自 model/normalize 的 normExpert：配色按领域、状态按可用性词表，二者都不再由视图猜字段名
const visualKey = (expert) => expertVisualKey(expert)
const statusClass = (expert) => expertStatusClass(expert)
const statusText = (expert) => availabilityLabel(expert?.status)
const expertTitleText = (expert) =>
  expert?.title || EXPERT_TYPES[expertVisualKey(expert)] || expert?.expertType || '专家'
const capabilityTags = (expert) => {
  const caps = (expert?.capabilities || []).map((c) => c?.name).filter(Boolean)
  return (caps.length ? caps : expert?.domains || []).slice(0, 2)
}

const onlineExpertCount = computed(() =>
  props.experts.filter((e) => e.online).length
)

const filteredExperts = computed(() => {
  let list = props.experts
  if (filterType.value) {
    list = list.filter(e => expertVisualKey(e) === filterType.value)
  }
  if (searchKeyword.value) {
    const kw = searchKeyword.value.toLowerCase()
    list = list.filter(e =>
      (e.name || '').toLowerCase().includes(kw) ||
      (e.expertType || '').toLowerCase().includes(kw) ||
      (e.domains || []).some(d => (d || '').toLowerCase().includes(kw)) ||
      (e.skills || []).some(s => (s || '').toLowerCase().includes(kw))
    )
  }
  return list
})

function isExpertSelected(id) {
  return props.selectedExpertIds.includes(id)
}

// 会话行的一切口径来自模块：字段名取自 normSession 的投影（后端从不发 updated_at/mode/expert_count），
// 标签与配色取自 contract/sessions 的 session_type 词表（旧版这里有一张含 smart/algorithm 的私表，
// 与后端词表不相交，于是服务端行永远没有标签、本地草稿行 reload 后标签就消失）。
const sessionRows = computed(() =>
  props.sessions.map((s) => ({
    session: s,
    title: sessionListTitle(s),
    expertCount: s.expertIds.length,
    timeText: sessionTimeText(sessionActivityAt(s)),
    typeLabel: sessionTypeLabel(s.sessionType),
    typeTag: sessionTypeTagType(s.sessionType)
  }))
)
</script>
