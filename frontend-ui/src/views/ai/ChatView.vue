<!-- 全局 AI 助手页面 - 带会话侧栏的完整布局 -->
<template>
  <div class="ai-chat-page">
    <!-- 左侧会话侧栏 -->
    <div class="ai-sidebar" :class="{ collapsed: sidebarCollapsed }">
      <div class="sidebar-header">
        <div class="brand">
          <div class="brand-icon">
            <el-icon><Cpu /></el-icon>
          </div>
          <span v-if="!sidebarCollapsed" class="brand-text">AI 助手</span>
        </div>
        <el-button text class="toggle-btn" @click="sidebarCollapsed = !sidebarCollapsed">
          <el-icon><component :is="sidebarCollapsed ? 'Expand' : 'Fold'" /></el-icon>
        </el-button>
      </div>

      <div class="new-chat-wrap">
        <el-button type="primary" class="new-chat-btn" @click="handleNewSession">
          <el-icon><Plus /></el-icon>
          <span v-if="!sidebarCollapsed">新对话</span>
        </el-button>
      </div>

      <el-scrollbar class="session-scroll">
        <div v-if="!sidebarCollapsed" class="session-list">
          <div class="list-label">最近对话</div>
          <div
            v-for="s in aiStore.sortedSessions"
            :key="s.id"
            class="session-item"
            :class="{ active: s.id === aiStore.currentSessionId }"
            @click="handleSelectSession(s.id)"
          >
            <el-icon class="session-icon"><ChatDotRound /></el-icon>
            <div class="session-text">
              <span class="session-title">{{ s.title }}</span>
              <span class="session-time">{{ formatTime(s.updatedAt) }}</span>
            </div>
            <el-button
              v-if="sidebarCollapsed"
              text
              size="small"
              class="session-del"
              @click.stop="handleDeleteSession(s.id)"
            >
              <el-icon><Delete /></el-icon>
            </el-button>
          </div>
          <el-empty v-if="aiStore.sortedSessions.length === 0" description="暂无对话" :image-size="50" />
        </div>
      </el-scrollbar>

      <div class="sidebar-footer">
        <div class="assistant-switcher" @click="showAssistantPanel = true">
          <div class="switcher-avatar" :style="{ background: aiStore.currentAssistantObj.gradient }">
            {{ aiStore.currentAssistantObj.emoji }}
          </div>
          <div v-if="!sidebarCollapsed" class="switcher-info">
            <div class="switcher-name">{{ aiStore.currentAssistantObj.name }}</div>
            <div class="switcher-tip">点击切换助手</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 主对话区 -->
    <div class="ai-main">
      <AIChatPanel
        mode="full"
        :suggestions="currentSuggestions"
        @new-session="handleNewSession"
      />
    </div>

    <!-- 助手选择抽屉 -->
    <el-drawer
      v-model="showAssistantPanel"
      title="选择 AI 助手"
      direction="rtl"
      size="300px"
    >
      <div class="assistant-list">
        <div
          v-for="(a, key) in assistantsList"
          :key="key"
          class="assistant-item"
          :class="{ active: aiStore.currentAssistant === key }"
          @click="handleSelectAssistant(key)"
        >
          <div class="ai-avatar" :style="{ background: a.gradient }">{{ a.emoji }}</div>
          <div class="ai-info">
            <div class="ai-name">{{ a.name }}</div>
            <div class="ai-desc">{{ a.description }}</div>
          </div>
          <el-icon v-if="aiStore.currentAssistant === key" class="ai-check"><CircleCheckFilled /></el-icon>
        </div>
      </div>
    </el-drawer>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import {
  Cpu, Expand, Fold, Plus, ChatDotRound, Delete, CircleCheckFilled
} from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useAIStore, ASSISTANTS } from '@/stores/ai.store'
import AIChatPanel from '@/components/ai/AIChatPanel.vue'

const aiStore = useAIStore()

const sidebarCollapsed = ref(false)
const showAssistantPanel = ref(false)

const assistantsList = computed(() => ASSISTANTS)

const currentSuggestions = computed(() => {
  const suggestions = {
    general: [
      '帮我设计一个企业级知识图谱系统的技术架构',
      '分析知识图谱领域的技术趋势',
      '做一份项目需求mox 模块化系统架构分析报告',
      '评审微服务架构设计方案'
    ],
    architect: [
      '设计高可用微服务架构',
      '技术选型对比分析',
      '数据库架构设计方案',
      '系统性能瓶颈分析与优化'
    ],
    analyst: [
      '生成竞品分析报告',
      '数据分析方法论建议',
      '项目可行性分析',
      '技术趋势洞察'
    ],
    data: [
      '设计知识图谱 Schema',
      '数据治理方案',
      'ETL 流水线设计',
      '数据建模最佳实践'
    ],
    product: [
      '需求文档模板',
      '产品路线图规划',
      '用户故事拆分',
      'MVP 功能定义'
    ],
    devops: [
      'Kubernetes 部署方案',
      'CI/CD 流水线设计',
      '监控告警体系搭建',
      '灾备与高可用方案'
    ]
  }
  return suggestions[aiStore.currentAssistant] || suggestions.general
})

function formatTime(ts) {
  if (!ts) return ''
  const now = Date.now()
  const diff = now - ts
  if (diff < 60000) return '刚刚'
  if (diff < 3600000) return Math.floor(diff / 60000) + '分钟前'
  if (diff < 86400000) return Math.floor(diff / 3600000) + '小时前'
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()}`
}

function handleNewSession() {
  aiStore.newSession()
}

function handleSelectSession(id) {
  aiStore.selectSession(id)
}

function handleDeleteSession(id) {
  ElMessageBox.confirm('删除这个对话？', '确认', {
    type: 'warning',
    confirmButtonText: '删除',
    cancelButtonText: '取消'
  }).then(() => {
    aiStore.deleteSession(id)
    ElMessage.success('已删除')
  }).catch(() => {})
}

function handleSelectAssistant(key) {
  aiStore.setAssistant(key)
  showAssistantPanel.value = false
  ElMessage.success(`已切换到 ${ASSISTANTS[key].name}`)
}

onMounted(() => {
  aiStore.setScope('global')
  aiStore.ensureSession()
})
</script>

<style scoped>
.ai-chat-page {
  display: flex;
  height: 100%;
  width: 100%;
  background: var(--bg-tertiary);
}

/* 侧栏 */
.ai-sidebar {
  width: 240px;
  flex-shrink: 0;
  background: var(--bg-card);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  transition: width 0.25s ease;
}

.ai-sidebar.collapsed {
  width: 60px;
}

.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border-ghost);
  height: 52px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 8px;
}

.brand-icon {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 14px;
}

.brand-text {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-primary);
}

.toggle-btn {
  color: #94a3b8;
  padding: 4px;
}

.new-chat-wrap {
  padding: 10px;
}

.new-chat-btn {
  width: 100%;
  justify-content: center;
  font-size: 13px;
}

.session-scroll {
  flex: 1;
  overflow: hidden;
}

.session-list {
  padding: 0 6px;
}

.list-label {
  font-size: 10px;
  font-weight: 600;
  color: #94a3b8;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  padding: 10px 8px 6px;
}

.session-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.12s;
  margin-bottom: 2px;
}

.session-item:hover {
  background: var(--bg-tertiary);
}

.session-item.active {
  background: var(--accent-dim);
}

.session-icon {
  font-size: 14px;
  color: #64748b;
  flex-shrink: 0;
}

.session-item.active .session-icon {
  color: #6366f1;
}

.session-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.session-title {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-weight: 500;
}

.session-item.active .session-title {
  color: #4f46e5;
  font-weight: 600;
}

.session-time {
  font-size: 10px;
  color: #94a3b8;
}

.sidebar-footer {
  padding: 10px;
  border-top: 1px solid var(--border-ghost);
}

.assistant-switcher {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.12s;
}

.assistant-switcher:hover {
  background: var(--bg-tertiary);
}

.switcher-avatar {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 15px;
  flex-shrink: 0;
}

.switcher-info {
  flex: 1;
  min-width: 0;
}

.switcher-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.switcher-tip {
  font-size: 10px;
  color: #94a3b8;
  margin-top: 1px;
}

/* 主区 */
.ai-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

/* 助手列表 */
.assistant-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.assistant-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px;
  border-radius: 8px;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.15s;
}

.assistant-item:hover {
  background: var(--bg-tertiary);
}

.assistant-item.active {
  background: var(--accent-dim);
  border-color: #c7d2fe;
}

.ai-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 18px;
  flex-shrink: 0;
}

.ai-info {
  flex: 1;
  min-width: 0;
}

.ai-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 2px;
}

.ai-desc {
  font-size: 11px;
  color: #64748b;
}

.ai-check {
  color: #6366f1;
  font-size: 18px;
}

@media (max-width: 768px) {
  .ai-sidebar {
    position: fixed;
    z-index: 100;
    transform: translateX(-100%);
  }
  .ai-sidebar:not(.collapsed) {
    transform: translateX(0);
  }
}
</style>
