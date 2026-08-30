<!--
  专家联盟统一工作台 · Expert Alliance Unified Workspace
  ======================================================
  架构原则：前端融合 · 后端模块化
  三栏布局：左(专家联盟) | 中(图谱画布) | 右(知识库云盘)
-->

<template>
  <div class="expert-workspace">
    <!-- ========== 顶部全局工具栏 ========== -->
    <header class="ws-header">
      <div class="ws-header-left">
        <div class="ws-logo">
          <span class="ws-logo-icon">🕸️</span>
          <span class="ws-logo-text">专家联盟工作台</span>
        </div>
        <div class="ws-project-selector">
          <el-select v-model="currentProject" size="small" class="ws-project-select">
            <el-option label="璇玑知识工程" value="xuanji" />
            <el-option label="MOX 平台架构" value="mox" />
            <el-option label="AI 算法实验室" value="ailab" />
          </el-select>
        </div>
      </div>

      <div class="ws-header-center">
        <div class="ws-mode-tabs">
          <button
            v-for="mode in workModes"
            :key="mode.key"
            class="ws-mode-tab"
            :class="{ active: activeMode === mode.key }"
            @click="activeMode = mode.key"
          >
            <span class="ws-mode-icon">{{ mode.icon }}</span>
            <span class="ws-mode-label">{{ mode.label }}</span>
          </button>
        </div>
      </div>

      <div class="ws-header-right">
        <div class="ws-global-search">
          <el-icon><Search /></el-icon>
          <input
            v-model="globalSearch"
            class="ws-search-input"
            placeholder="全局搜索：专家 / 文档 / 节点…"
            @keyup.enter="doGlobalSearch"
          />
          <kbd class="ws-search-kbd">⌘K</kbd>
        </div>
        <el-button type="primary" size="small" class="ws-ai-btn" @click="openAIAssistant">
          <el-icon><MagicStick /></el-icon>
          <span>AI 协作</span>
        </el-button>
        <el-button size="small" text class="ws-icon-btn" title="通知">
          <el-icon><Bell /></el-icon>
          <span class="ws-badge-dot"></span>
        </el-button>
        <el-avatar :size="32" class="ws-avatar">U</el-avatar>
      </div>
    </header>

    <!-- ========== 主工作区 · 三栏布局 ========== -->
    <div class="ws-main">
      <!-- ---- 左栏：专家联盟面板 ---- -->
      <aside
        class="ws-panel ws-panel-left"
        :class="{ collapsed: leftCollapsed }"
        :style="{ width: leftCollapsed ? '48px' : leftPanelWidth + 'px' }"
      >
        <div class="ws-panel-header">
          <span v-if="!leftCollapsed" class="ws-panel-title">
            <span class="ws-panel-icon">👥</span>
            专家联盟
          </span>
          <button class="ws-panel-toggle" @click="leftCollapsed = !leftCollapsed" :title="leftCollapsed ? '展开' : '收起'">
            <el-icon v-if="!leftCollapsed"><ArrowLeft /></el-icon>
            <el-icon v-else><ArrowRight /></el-icon>
          </button>
        </div>

        <div v-if="!leftCollapsed" class="ws-panel-body">
          <!-- 专家列表 -->
          <div class="ws-expert-section">
            <div class="ws-section-label">在线专家 ({{ onlineExperts.length }})</div>
            <div class="ws-expert-list">
              <div
                v-for="expert in onlineExperts"
                :key="expert.id"
                class="ws-expert-item"
                :class="{ active: activeExpert?.id === expert.id }"
                @click="selectExpert(expert)"
              >
                <el-avatar :size="36" :style="{ background: expert.color }">{{ expert.avatar }}</el-avatar>
                <div class="ws-expert-info">
                  <div class="ws-expert-name">
                    {{ expert.name }}
                    <span class="ws-online-dot"></span>
                  </div>
                  <div class="ws-expert-role">{{ expert.role }}</div>
                </div>
                <div class="ws-expert-status">{{ expert.statusText }}</div>
              </div>
            </div>
          </div>

          <!-- 协作会话 -->
          <div class="ws-expert-section">
            <div class="ws-section-label">
              协作会话
              <el-button size="small" text class="ws-add-btn" @click="newCollaboration">
                <el-icon><Plus /></el-icon>
              </el-button>
            </div>
            <div class="ws-session-list">
              <div
                v-for="session in sessions"
                :key="session.id"
                class="ws-session-item"
                :class="{ active: activeSession?.id === session.id }"
                @click="selectSession(session)"
              >
                <div class="ws-session-title">{{ session.title }}</div>
                <div class="ws-session-meta">
                  <span class="ws-session-experts">{{ session.experts.join('、') }}</span>
                  <span class="ws-session-time">{{ session.time }}</span>
                </div>
              </div>
            </div>
          </div>

          <!-- 快捷工具 -->
          <div class="ws-expert-section">
            <div class="ws-section-label">快捷工具</div>
            <div class="ws-tool-grid">
              <button class="ws-tool-btn" @click="triggerDebate">
                <span class="ws-tool-icon">⚔️</span>
                <span>专家辩论</span>
              </button>
              <button class="ws-tool-btn" @click="triggerOrchestration">
                <span class="ws-tool-icon">🎯</span>
                <span>任务编排</span>
              </button>
              <button class="ws-tool-btn" @click="triggerVoting">
                <span class="ws-tool-icon">🗳️</span>
                <span>融合投票</span>
              </button>
              <button class="ws-tool-btn" @click="triggerConsult">
                <span class="ws-tool-icon">💬</span>
                <span>多轮咨询</span>
              </button>
            </div>
          </div>
        </div>

        <!-- 折叠状态图标列表 -->
        <div v-else class="ws-collapsed-icons">
          <button
            v-for="expert in onlineExperts.slice(0, 6)"
            :key="expert.id"
            class="ws-collapsed-avatar"
            :title="expert.name"
            @click="leftCollapsed = false; selectExpert(expert)"
          >
            <el-avatar :size="32" :style="{ background: expert.color }">{{ expert.avatar }}</el-avatar>
          </button>
        </div>
      </aside>

      <!-- ---- 中栏：图谱画布 ---- -->
      <main class="ws-center">
        <!-- 画布工具栏 -->
        <div class="ws-canvas-toolbar">
          <div class="ws-canvas-tools-left">
            <button
              v-for="tool in canvasTools"
              :key="tool.key"
              class="ws-canvas-tool"
              :class="{ active: activeCanvasTool === tool.key }"
              :title="tool.label"
              @click="activeCanvasTool = tool.key"
            >
              <span>{{ tool.icon }}</span>
            </button>
            <div class="ws-tool-divider"></div>
            <button class="ws-canvas-tool" title="放大" @click="zoomIn">
              <el-icon><ZoomIn /></el-icon>
            </button>
            <button class="ws-canvas-tool" title="缩小" @click="zoomOut">
              <el-icon><ZoomOut /></el-icon>
            </button>
            <button class="ws-canvas-tool" title="适应视图" @click="fitView">
              <el-icon><FullScreen /></el-icon>
            </button>
          </div>

          <div class="ws-canvas-tools-center">
            <div class="ws-layout-switcher">
              <button
                v-for="layout in layouts"
                :key="layout.key"
                class="ws-layout-btn"
                :class="{ active: currentLayout === layout.key }"
                @click="currentLayout = layout.key"
              >
                {{ layout.icon }} {{ layout.label }}
              </button>
            </div>
          </div>

          <div class="ws-canvas-tools-right">
            <div class="ws-graph-stats">
              <span class="ws-stat-item"><strong>{{ mockStats.nodes }}</strong> 节点</span>
              <span class="ws-stat-divider">·</span>
              <span class="ws-stat-item"><strong>{{ mockStats.edges }}</strong> 关系</span>
              <span class="ws-stat-divider">·</span>
              <span class="ws-stat-item"><strong>{{ mockStats.types }}</strong> 类型</span>
            </div>
            <el-button size="small" type="primary" plain @click="runGraphAlgo">
              <el-icon><DataAnalysis /></el-icon>
              图谱分析
            </el-button>
          </div>
        </div>

        <!-- 图谱画布区（SVG 模拟） -->
        <div class="ws-graph-canvas" ref="canvasRef">
          <svg class="ws-graph-svg" viewBox="0 0 800 500" preserveAspectRatio="xMidYMid meet">
            <!-- 背景网格 -->
            <defs>
              <pattern id="grid" width="40" height="40" patternUnits="userSpaceOnUse">
                <path d="M 40 0 L 0 0 0 40" fill="none" stroke="rgba(99,102,241,0.06)" stroke-width="1"/>
              </pattern>
              <radialGradient id="nodeGlow" cx="50%" cy="50%" r="50%">
                <stop offset="0%" style="stop-color:#6366f1;stop-opacity:0.3" />
                <stop offset="100%" style="stop-color:#6366f1;stop-opacity:0" />
              </radialGradient>
            </defs>
            <rect width="100%" height="100%" fill="url(#grid)" />

            <!-- 边（关系） -->
            <g class="ws-edges">
              <line v-for="edge in mockEdges" :key="edge.id"
                :x1="edge.sourceX" :y1="edge.sourceY"
                :x2="edge.targetX" :y2="edge.targetY"
                :stroke="edge.color || '#94a3b8'"
                :stroke-width="edge.width || 1.5"
                :stroke-opacity="0.6"
                class="ws-edge"
              />
            </g>

            <!-- 节点 -->
            <g class="ws-nodes">
              <g v-for="node in mockNodes" :key="node.id"
                :transform="`translate(${node.x}, ${node.y})`"
                class="ws-node"
                :class="{ selected: selectedNode?.id === node.id, highlight: node.highlight }"
                @click="selectNode(node)"
              >
                <circle :r="node.size || 18" :fill="node.color || '#6366f1'" :opacity="0.85" />
                <circle :r="(node.size || 18) + 8" fill="url(#nodeGlow)" v-if="node.highlight" />
                <text text-anchor="middle" dy="4" fill="white" :font-size="node.size > 20 ? 11 : 10" font-weight="600">
                  {{ node.label }}
                </text>
              </g>
            </g>
          </svg>

          <!-- 选中节点信息浮层 -->
          <div v-if="selectedNode" class="ws-node-info-card" :style="nodeCardStyle">
            <div class="ws-node-info-header">
              <span class="ws-node-info-icon" :style="{ background: selectedNode.color }">{{ selectedNode.label }}</span>
              <div>
                <div class="ws-node-info-title">{{ selectedNode.fullName }}</div>
                <div class="ws-node-info-type">{{ selectedNode.type }}</div>
              </div>
              <button class="ws-node-close" @click="selectedNode = null">
                <el-icon><Close /></el-icon>
              </button>
            </div>
            <div class="ws-node-info-body">
              <div class="ws-node-info-row">
                <span class="ws-node-info-label">关联文档</span>
                <span class="ws-node-info-value">{{ selectedNode.docs }} 篇</span>
              </div>
              <div class="ws-node-info-row">
                <span class="ws-node-info-label">关联专家</span>
                <span class="ws-node-info-value">{{ selectedNode.experts }} 位</span>
              </div>
              <div class="ws-node-info-row">
                <span class="ws-node-info-label">中心性排名</span>
                <span class="ws-node-info-value">#{{ selectedNode.rank }}</span>
              </div>
            </div>
            <div class="ws-node-info-actions">
              <el-button size="small" @click="viewNodeDocs(selectedNode)">
                <el-icon><Document /></el-icon>
                查看文档
              </el-button>
              <el-button size="small" type="primary" @click="askExpertsAbout(selectedNode)">
                <el-icon><ChatDotRound /></el-icon>
                咨询专家
              </el-button>
            </div>
          </div>
        </div>

        <!-- 底部协作对话栏 -->
        <div class="ws-collab-bar" :class="{ expanded: collabExpanded }">
          <div class="ws-collab-header" @click="collabExpanded = !collabExpanded">
            <div class="ws-collab-title">
              <el-icon><ChatLineSquare /></el-icon>
              <span>协作讨论 · {{ activeSession?.title || '未开始' }}</span>
              <span class="ws-collab-count">{{ collabMessages.length }} 条消息</span>
            </div>
            <div class="ws-collab-toggle">
              <el-icon v-if="collabExpanded"><ArrowDown /></el-icon>
              <el-icon v-else><ArrowUp /></el-icon>
            </div>
          </div>
          <div v-if="collabExpanded" class="ws-collab-body">
            <div class="ws-collab-messages">
              <div v-for="msg in collabMessages" :key="msg.id" class="ws-collab-msg" :class="msg.role">
                <el-avatar :size="28" :style="{ background: msg.color }">{{ msg.avatar }}</el-avatar>
                <div class="ws-collab-msg-content">
                  <div class="ws-collab-msg-meta">
                    <span class="ws-collab-msg-name">{{ msg.name }}</span>
                    <span class="ws-collab-msg-time">{{ msg.time }}</span>
                  </div>
                  <div class="ws-collab-msg-text">{{ msg.text }}</div>
                </div>
              </div>
            </div>
            <div class="ws-collab-input">
              <input v-model="collabInput" class="ws-collab-input-field" placeholder="输入问题或指令… (Enter 发送)" @keyup.enter="sendCollabMsg" />
              <el-button type="primary" size="small" @click="sendCollabMsg">发送</el-button>
            </div>
          </div>
        </div>
      </main>

      <!-- ---- 右栏：知识库云盘面板 ---- -->
      <aside
        class="ws-panel ws-panel-right"
        :class="{ collapsed: rightCollapsed }"
        :style="{ width: rightCollapsed ? '48px' : rightPanelWidth + 'px' }"
      >
        <div class="ws-panel-header">
          <button class="ws-panel-toggle" @click="rightCollapsed = !rightCollapsed" :title="rightCollapsed ? '展开' : '收起'">
            <el-icon v-if="!rightCollapsed"><ArrowRight /></el-icon>
            <el-icon v-else><ArrowLeft /></el-icon>
          </button>
          <span v-if="!rightCollapsed" class="ws-panel-title">
            知识库云盘
            <span class="ws-panel-icon">📚</span>
          </span>
        </div>

        <div v-if="!rightCollapsed" class="ws-panel-body">
          <!-- Tab 切换 -->
          <div class="ws-kb-tabs">
            <button
              v-for="tab in kbTabs"
              :key="tab.key"
              class="ws-kb-tab"
              :class="{ active: activeKbTab === tab.key }"
              @click="activeKbTab = tab.key"
            >
              {{ tab.icon }} {{ tab.label }}
            </button>
          </div>

          <!-- 搜索 -->
          <div class="ws-kb-search">
            <el-icon><Search /></el-icon>
            <input v-model="kbSearch" class="ws-kb-search-input" placeholder="搜索文档…" @keyup.enter="searchKb" />
          </div>

          <!-- 文档列表 -->
          <div v-if="activeKbTab === 'docs'" class="ws-doc-list">
            <div class="ws-doc-category">
              <div class="ws-doc-category-title">
                <el-icon><FolderOpened /></el-icon>
                架构设计文档
              </div>
              <div
                v-for="doc in archDocs"
                :key="doc.id"
                class="ws-doc-item"
                :class="{ active: activeDoc?.id === doc.id }"
                @click="openDoc(doc)"
              >
                <span class="ws-doc-icon">{{ doc.icon }}</span>
                <div class="ws-doc-info">
                  <div class="ws-doc-name">{{ doc.name }}</div>
                  <div class="ws-doc-meta">{{ doc.size }} · {{ doc.updated }}</div>
                </div>
                <span class="ws-doc-badge" v-if="doc.linked">🔗</span>
              </div>
            </div>

            <div class="ws-doc-category">
              <div class="ws-doc-category-title">
                <el-icon><Folder /></el-icon>
                算法研究
              </div>
              <div
                v-for="doc in algoDocs"
                :key="doc.id"
                class="ws-doc-item"
                :class="{ active: activeDoc?.id === doc.id }"
                @click="openDoc(doc)"
              >
                <span class="ws-doc-icon">{{ doc.icon }}</span>
                <div class="ws-doc-info">
                  <div class="ws-doc-name">{{ doc.name }}</div>
                  <div class="ws-doc-meta">{{ doc.size }} · {{ doc.updated }}</div>
                </div>
                <span class="ws-doc-badge" v-if="doc.linked">🔗</span>
              </div>
            </div>
          </div>

          <!-- 标签云 -->
          <div v-if="activeKbTab === 'tags'" class="ws-tag-cloud">
            <span
              v-for="tag in popularTags"
              :key="tag.name"
              class="ws-tag-cloud-item"
              :style="{ fontSize: tag.size + 'px', opacity: tag.opacity }"
              @click="filterByTag(tag)"
            >
              {{ tag.name }}
            </span>
          </div>

          <!-- 版本历史 -->
          <div v-if="activeKbTab === 'versions'" class="ws-version-list">
            <div
              v-for="(ver, idx) in recentVersions"
              :key="ver.id"
              class="ws-version-item"
              :class="{ latest: idx === 0 }"
            >
              <div class="ws-version-header">
                <span class="ws-version-badge">{{ idx === 0 ? '当前版本' : '历史版本' }}</span>
                <span class="ws-version-time">{{ ver.time }}</span>
              </div>
              <div class="ws-version-doc">{{ ver.docName }}</div>
              <div class="ws-version-author">{{ ver.author }} · {{ ver.action }}</div>
            </div>
          </div>

          <!-- 快捷操作 -->
          <div class="ws-kb-actions">
            <el-button size="small" class="ws-kb-action-btn" @click="uploadDoc">
              <el-icon><Upload /></el-icon>
              上传文档
            </el-button>
            <el-button size="small" type="primary" class="ws-kb-action-btn" @click="createDoc">
              <el-icon><Edit /></el-icon>
              新建
            </el-button>
          </div>
        </div>

        <!-- 折叠状态图标 -->
        <div v-else class="ws-collapsed-icons ws-collapsed-right">
          <button class="ws-collapsed-icon-btn" title="知识库" @click="rightCollapsed = false">
            <span>📚</span>
          </button>
          <button class="ws-collapsed-icon-btn" title="标签" @click="rightCollapsed = false; activeKbTab = 'tags'">
            <span>🏷️</span>
          </button>
          <button class="ws-collapsed-icon-btn" title="版本" @click="rightCollapsed = false; activeKbTab = 'versions'">
            <span>📋</span>
          </button>
        </div>
      </aside>
    </div>

    <!-- ========== AI 助手浮窗 ========== -->
    <div v-if="aiAssistantOpen" class="ws-ai-assistant">
      <div class="ws-ai-header">
        <span class="ws-ai-icon">🤖</span>
        <span class="ws-ai-title">AI 协作助手</span>
        <button class="ws-ai-close" @click="aiAssistantOpen = false">
          <el-icon><Close /></el-icon>
        </button>
      </div>
      <div class="ws-ai-body">
        <div class="ws-ai-suggestions">
          <div class="ws-ai-suggest-title">快捷指令</div>
          <div class="ws-ai-suggest-grid">
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('分析图谱')">🔍 分析当前图谱</button>
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('推荐专家')">👥 推荐相关专家</button>
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('生成报告')">📝 生成研究报告</button>
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('知识问答')">❓ 知识问答</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import {
  Search, MagicStick, Bell, ArrowLeft, ArrowRight, Plus,
  ZoomIn, ZoomOut, FullScreen, DataAnalysis, Close,
  Document, ChatDotRound, ChatLineSquare, ArrowDown, ArrowUp,
  Folder, FolderOpened, Upload, Edit
} from '@element-plus/icons-vue'

// ========== 布局状态 ==========
const leftCollapsed = ref(false)
const rightCollapsed = ref(false)
const leftPanelWidth = ref(280)
const rightPanelWidth = ref(320)
const collabExpanded = ref(true)
const aiAssistantOpen = ref(false)

// ========== 工作模式 ==========
const activeMode = ref('collaboration')
const workModes = [
  { key: 'exploration', icon: '🔍', label: '知识探索' },
  { key: 'collaboration', icon: '🤝', label: '专家协作' },
  { key: 'orchestration', icon: '🎯', label: '任务编排' },
  { key: 'analysis', icon: '📊', label: '深度分析' }
]

// ========== 项目 ==========
const currentProject = ref('xuanji')
const globalSearch = ref('')

// ========== 专家数据（模拟） ==========
const onlineExperts = ref([
  { id: 1, name: '璇玑算法', avatar: '璇', role: '图算法专家', color: '#6366f1', statusText: '空闲' },
  { id: 2, name: '架构师', avatar: '架', role: '系统架构专家', color: '#06b6d4', statusText: '思考中' },
  { id: 3, name: '知识库管家', avatar: '知', role: '知识管理专家', color: '#10b981', statusText: '索引中' },
  { id: 4, name: '数据分析师', avatar: '数', role: '数据分析专家', color: '#f59e0b', statusText: '忙碌' },
  { id: 5, name: '产品规划师', avatar: '产', role: '产品设计专家', color: '#ec4899', statusText: '空闲' }
])
const activeExpert = ref(null)
const activeSession = ref(null)

const sessions = ref([
  { id: 1, title: '架构优化方案讨论', experts: ['架构师', '璇玑算法'], time: '10分钟前' },
  { id: 2, title: '知识图谱融合策略', experts: ['璇玑算法', '知识库管家'], time: '1小时前' },
  { id: 3, title: '性能瓶颈分析', experts: ['架构师', '数据分析师'], time: '昨天' }
])

// ========== 图谱画布 ==========
const activeCanvasTool = ref('select')
const currentLayout = ref('force')
const selectedNode = ref(null)
const canvasTools = [
  { key: 'select', icon: '🖱️', label: '选择' },
  { key: 'pan', icon: '✋', label: '平移' },
  { key: 'add-node', icon: '➕', label: '添加节点' },
  { key: 'add-edge', icon: '🔗', label: '添加关系' },
  { key: 'delete', icon: '🗑️', label: '删除' }
]
const layouts = [
  { key: 'force', icon: '🔄', label: '力导向' },
  { key: 'radial', icon: '☀️', label: '辐射' },
  { key: 'hierarchical', icon: '🏛️', label: '层次' },
  { key: 'circular', icon: '⭕', label: '环形' }
]

// 模拟图谱数据
const mockStats = { nodes: 1247, edges: 3582, types: 23 }

const mockNodes = ref([
  { id: 1, label: '专家', fullName: '专家实体', type: '核心实体', x: 400, y: 200, size: 28, color: '#6366f1', docs: 156, experts: 12, rank: 1, highlight: true },
  { id: 2, label: '知识', fullName: '知识节点', type: '核心实体', x: 250, y: 280, size: 24, color: '#06b6d4', docs: 203, experts: 8, rank: 2 },
  { id: 3, label: '文档', fullName: '文档实体', type: '内容实体', x: 550, y: 280, size: 22, color: '#10b981', docs: 892, experts: 5, rank: 3 },
  { id: 4, label: '图谱', fullName: '知识图谱', type: '系统', x: 320, y: 120, size: 20, color: '#8b5cf6', docs: 45, experts: 7, rank: 5 },
  { id: 5, label: '算法', fullName: '图算法', type: '算法', x: 480, y: 120, size: 18, color: '#f59e0b', docs: 67, experts: 6, rank: 4 },
  { id: 6, label: '协作', fullName: '协作模式', type: '关系类型', x: 180, y: 180, size: 16, color: '#ec4899', docs: 23, experts: 4, rank: 8 },
  { id: 7, label: '云盘', fullName: '云存储', type: '系统', x: 620, y: 180, size: 18, color: '#14b8a6', docs: 34, experts: 3, rank: 7 },
  { id: 8, label: '编排', fullName: '任务编排', type: '能力', x: 200, y: 380, size: 16, color: '#f97316', docs: 28, experts: 5, rank: 9 },
  { id: 9, label: '辩论', fullName: '专家辩论', type: '协作模式', x: 400, y: 380, size: 18, color: '#ef4444', docs: 12, experts: 8, rank: 6 },
  { id: 10, label: '融合', fullName: '知识融合', type: '能力', x: 600, y: 380, size: 16, color: '#84cc16', docs: 31, experts: 4, rank: 10 }
])

const mockEdges = ref([
  { id: 'e1', sourceX: 400, sourceY: 200, targetX: 250, targetY: 280, color: '#6366f1', width: 2 },
  { id: 'e2', sourceX: 400, sourceY: 200, targetX: 550, targetY: 280, color: '#6366f1', width: 2 },
  { id: 'e3', sourceX: 400, sourceY: 200, targetX: 320, targetY: 120, color: '#94a3b8' },
  { id: 'e4', sourceX: 400, sourceY: 200, targetX: 480, targetY: 120, color: '#94a3b8' },
  { id: 'e5', sourceX: 250, sourceY: 280, targetX: 180, targetY: 180, color: '#94a3b8' },
  { id: 'e6', sourceX: 550, sourceY: 280, targetX: 620, targetY: 180, color: '#94a3b8' },
  { id: 'e7', sourceX: 250, sourceY: 280, targetX: 200, targetY: 380, color: '#94a3b8' },
  { id: 'e8', sourceX: 400, sourceY: 200, targetX: 400, targetY: 380, color: '#ef4444', width: 2 },
  { id: 'e9', sourceX: 550, sourceY: 280, targetX: 600, targetY: 380, color: '#94a3b8' },
  { id: 'e10', sourceX: 320, sourceY: 120, targetX: 480, targetY: 120, color: '#94a3b8' },
  { id: 'e11', sourceX: 250, sourceY: 280, targetX: 320, targetY: 120, color: '#06b6d4', width: 1.5 },
  { id: 'e12', sourceX: 550, sourceY: 280, targetX: 480, targetY: 120, color: '#10b981', width: 1.5 }
])

const nodeCardStyle = computed(() => {
  if (!selectedNode.value) return {}
  return {
    left: Math.min(selectedNode.value.x + 30, 600) + 'px',
    top: Math.max(selectedNode.value.y - 60, 20) + 'px'
  }
})

// ========== 知识库 ==========
const activeKbTab = ref('docs')
const kbSearch = ref('')
const activeDoc = ref(null)

const kbTabs = [
  { key: 'docs', icon: '📄', label: '文档' },
  { key: 'tags', icon: '🏷️', label: '标签' },
  { key: 'versions', icon: '📋', label: '版本' }
]

const archDocs = ref([
  { id: 1, name: '专家联盟架构设计 V3.0', icon: '📐', size: '2.4 MB', updated: '10分钟前', linked: true },
  { id: 2, name: '知识图谱域架构规范', icon: '🕸️', size: '1.8 MB', updated: '2小时前', linked: true },
  { id: 3, name: '算法归一化设计方案', icon: '🧮', size: '956 KB', updated: '昨天', linked: false },
  { id: 4, name: '云存储域接口定义', icon: '☁️', size: '620 KB', updated: '3天前', linked: true }
])

const algoDocs = ref([
  { id: 5, name: '中心性算法对比研究', icon: '📊', size: '1.2 MB', updated: '昨天', linked: true },
  { id: 6, name: '社区发现算法优化', icon: '🔬', size: '890 KB', updated: '3天前', linked: false }
])

const popularTags = ref([
  { name: '架构设计', size: 18, opacity: 1 },
  { name: '知识图谱', size: 16, opacity: 0.9 },
  { name: '图算法', size: 15, opacity: 0.85 },
  { name: '专家系统', size: 17, opacity: 0.95 },
  { name: '微服务', size: 14, opacity: 0.8 },
  { name: 'RAG', size: 13, opacity: 0.75 },
  { name: '向量检索', size: 12, opacity: 0.7 },
  { name: '性能优化', size: 15, opacity: 0.85 },
  { name: '模块化', size: 14, opacity: 0.8 },
  { name: '归一化', size: 13, opacity: 0.75 },
  { name: '协作模式', size: 12, opacity: 0.7 },
  { name: '知识融合', size: 14, opacity: 0.8 }
])

const recentVersions = ref([
  { id: 1, docName: '专家联盟架构设计 V3.0', time: '2026-08-30 14:30', author: '架构师', action: '更新了第6章' },
  { id: 2, docName: '知识图谱域架构规范', time: '2026-08-30 10:15', author: '璇玑算法', action: '新增算法接口' },
  { id: 3, docName: '算法归一化设计方案', time: '2026-08-29 16:45', author: '数据分析师', action: '创建文档' }
])

// ========== 协作对话 ==========
const collabMessages = ref([
  { id: 1, role: 'expert', name: '架构师', avatar: '架', color: '#06b6d4', time: '14:25', text: '根据当前图谱分析，建议采用混合架构：前端统一工作台，后端模块化服务。' },
  { id: 2, role: 'expert', name: '璇玑算法', avatar: '璇', color: '#6366f1', time: '14:26', text: '同意。算法层可以抽成统一核心库，这样图谱分析和专家匹配都能复用。' },
  { id: 3, role: 'user', name: '我', avatar: 'U', color: '#64748b', time: '14:27', text: '那性能方面会有损失吗？' },
  { id: 4, role: 'expert', name: '数据分析师', avatar: '数', color: '#f59e0b', time: '14:28', text: 'gRPC 调用开销约 0.1-0.5ms，对于 AI 协作场景（单任务 30s+）完全可接受。' }
])
const collabInput = ref('')

// ========== 方法 ==========
function selectExpert(expert) {
  activeExpert.value = expert
}
function selectSession(session) {
  activeSession.value = session
}
function selectNode(node) {
  selectedNode.value = node
}
function openDoc(doc) {
  activeDoc.value = doc
}
function newCollaboration() {
  // TODO: 新建协作会话
}
function triggerDebate() {
  // TODO: 触发专家辩论
}
function triggerOrchestration() {
  // TODO: 触发任务编排
}
function triggerVoting() {
  // TODO: 触发融合投票
}
function triggerConsult() {
  // TODO: 触发多轮咨询
}
function zoomIn() {}
function zoomOut() {}
function fitView() {}
function runGraphAlgo() {}
function viewNodeDocs(node) {
  rightCollapsed.value = false
  activeKbTab.value = 'docs'
}
function askExpertsAbout(node) {
  collabExpanded.value = true
  collabInput.value = `请专家们分析一下「${node.fullName}」的相关情况`
}
function sendCollabMsg() {
  if (!collabInput.value.trim()) return
  collabMessages.value.push({
    id: Date.now(),
    role: 'user',
    name: '我',
    avatar: 'U',
    color: '#64748b',
    time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
    text: collabInput.value
  })
  collabInput.value = ''
}
function doGlobalSearch() {}
function searchKb() {}
function filterByTag(tag) {}
function uploadDoc() {}
function createDoc() {}
function openAIAssistant() {
  aiAssistantOpen.value = !aiAssistantOpen.value
}
function aiSuggestion(type) {
  collabInput.value = `请执行：${type}`
  aiAssistantOpen.value = false
  collabExpanded.value = true
}
</script>

<style scoped>
/* ========== 全局布局 ========== */
.expert-workspace {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f8fafc;
  font-family: 'Instrument Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
  color: #0f172a;
  overflow: hidden;
}

/* ========== 顶部 Header ========== */
.ws-header {
  height: 56px;
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(12px);
  border-bottom: 1px solid rgba(99, 102, 241, 0.12);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  flex-shrink: 0;
  z-index: 100;
}
.ws-header-left,
.ws-header-right {
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 300px;
}
.ws-header-right {
  justify-content: flex-end;
}

.ws-logo {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 16px;
  background: linear-gradient(135deg, #6366f1, #06b6d4);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}
.ws-logo-icon { font-size: 20px; }

.ws-project-select { width: 160px; }

/* 工作模式 Tabs */
.ws-mode-tabs {
  display: flex;
  background: rgba(99, 102, 241, 0.06);
  border-radius: 10px;
  padding: 4px;
  gap: 2px;
}
.ws-mode-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border: none;
  background: transparent;
  border-radius: 7px;
  font-size: 13px;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
}
.ws-mode-tab:hover { color: #6366f1; }
.ws-mode-tab.active {
  background: white;
  color: #6366f1;
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(0,0,0,0.08);
}

/* 全局搜索 */
.ws-global-search {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(99, 102, 241, 0.06);
  border: 1px solid rgba(99, 102, 241, 0.12);
  border-radius: 8px;
  padding: 6px 12px;
  width: 280px;
  color: #64748b;
}
.ws-search-input {
  flex: 1;
  border: none;
  background: transparent;
  outline: none;
  font-size: 13px;
  color: #0f172a;
}
.ws-search-input::placeholder { color: #94a3b8; }
.ws-search-kbd {
  font-size: 11px;
  color: #94a3b8;
  background: white;
  padding: 2px 6px;
  border-radius: 4px;
  border: 1px solid rgba(99, 102, 241, 0.15);
}

.ws-ai-btn { gap: 6px; }
.ws-icon-btn {
  position: relative;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
}
.ws-icon-btn:hover { background: rgba(99, 102, 241, 0.06); color: #6366f1; }
.ws-badge-dot {
  position: absolute;
  top: 8px;
  right: 8px;
  width: 8px;
  height: 8px;
  background: #ef4444;
  border-radius: 50%;
  border: 2px solid white;
}
.ws-avatar { cursor: pointer; }

/* ========== 主工作区三栏 ========== */
.ws-main {
  flex: 1;
  display: flex;
  overflow: hidden;
  position: relative;
}

/* ========== 侧边面板通用 ========== */
.ws-panel {
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(99, 102, 241, 0.1);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  transition: width 0.3s ease;
  overflow: hidden;
}
.ws-panel-left {
  border-right: 1px solid rgba(99, 102, 241, 0.1);
  border-top: none;
  border-left: none;
  border-bottom: none;
}
.ws-panel-right {
  border-left: 1px solid rgba(99, 102, 241, 0.1);
  border-top: none;
  border-right: none;
  border-bottom: none;
}

.ws-panel-header {
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  border-bottom: 1px solid rgba(99, 102, 241, 0.08);
  flex-shrink: 0;
}
.ws-panel-left .ws-panel-header { flex-direction: row; }
.ws-panel-right .ws-panel-header { flex-direction: row; }
.ws-panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}
.ws-panel-icon { font-size: 16px; }
.ws-panel-toggle {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: 6px;
  color: #64748b;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}
.ws-panel-toggle:hover {
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
}

.ws-panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

/* 折叠状态图标列表 */
.ws-collapsed-icons {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 12px 0;
  gap: 8px;
}
.ws-collapsed-avatar,
.ws-collapsed-icon-btn {
  width: 36px;
  height: 36px;
  border: none;
  background: transparent;
  border-radius: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
  font-size: 18px;
}
.ws-collapsed-avatar:hover,
.ws-collapsed-icon-btn:hover {
  background: rgba(99, 102, 241, 0.1);
  transform: scale(1.05);
}

/* ========== 左栏：专家联盟 ========== */
.ws-expert-section {
  margin-bottom: 20px;
}
.ws-section-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  font-weight: 600;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 10px;
}
.ws-add-btn {
  padding: 0 !important;
  width: 20px;
  height: 20px;
  color: #6366f1 !important;
}

/* 专家列表 */
.ws-expert-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ws-expert-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
}
.ws-expert-item:hover {
  background: rgba(99, 102, 241, 0.06);
}
.ws-expert-item.active {
  background: rgba(99, 102, 241, 0.12);
  border: 1px solid rgba(99, 102, 241, 0.2);
}
.ws-expert-info { flex: 1; min-width: 0; }
.ws-expert-name {
  font-size: 13px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 6px;
}
.ws-online-dot {
  width: 8px;
  height: 8px;
  background: #10b981;
  border-radius: 50%;
  flex-shrink: 0;
}
.ws-expert-role {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}
.ws-expert-status {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(16, 185, 129, 0.1);
  color: #10b981;
  flex-shrink: 0;
}

/* 会话列表 */
.ws-session-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.ws-session-item {
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}
.ws-session-item:hover { background: rgba(99, 102, 241, 0.06); }
.ws-session-item.active {
  background: rgba(99, 102, 241, 0.1);
  border: 1px solid rgba(99, 102, 241, 0.2);
}
.ws-session-title {
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ws-session-meta {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: #94a3b8;
}

/* 快捷工具 */
.ws-tool-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}
.ws-tool-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px 8px;
  border: 1px solid rgba(99, 102, 241, 0.12);
  background: rgba(255, 255, 255, 0.5);
  border-radius: 10px;
  cursor: pointer;
  font-size: 12px;
  color: #64748b;
  transition: all 0.2s;
}
.ws-tool-btn:hover {
  border-color: #6366f1;
  color: #6366f1;
  background: rgba(99, 102, 241, 0.04);
  transform: translateY(-1px);
}
.ws-tool-icon { font-size: 20px; }

/* ========== 中栏：图谱画布 ========== */
.ws-center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  position: relative;
}

/* 画布工具栏 */
.ws-canvas-toolbar {
  height: 44px;
  background: rgba(255, 255, 255, 0.6);
  backdrop-filter: blur(8px);
  border-bottom: 1px solid rgba(99, 102, 241, 0.08);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  flex-shrink: 0;
}
.ws-canvas-tools-left,
.ws-canvas-tools-right {
  display: flex;
  align-items: center;
  gap: 4px;
}
.ws-canvas-tool {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  font-size: 16px;
  transition: all 0.2s;
}
.ws-canvas-tool:hover {
  background: rgba(99, 102, 241, 0.08);
  color: #6366f1;
}
.ws-canvas-tool.active {
  background: rgba(99, 102, 241, 0.15);
  color: #6366f1;
}
.ws-tool-divider {
  width: 1px;
  height: 20px;
  background: rgba(99, 102, 241, 0.15);
  margin: 0 4px;
}

.ws-layout-switcher {
  display: flex;
  background: rgba(99, 102, 241, 0.06);
  border-radius: 8px;
  padding: 3px;
  gap: 2px;
}
.ws-layout-btn {
  padding: 4px 12px;
  border: none;
  background: transparent;
  border-radius: 6px;
  font-size: 12px;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
}
.ws-layout-btn:hover { color: #6366f1; }
.ws-layout-btn.active {
  background: white;
  color: #6366f1;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(0,0,0,0.06);
}

.ws-graph-stats {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #64748b;
  margin-right: 12px;
}
.ws-stat-item strong {
  color: #0f172a;
  font-weight: 700;
}
.ws-stat-divider { color: #cbd5e1; }

/* 图谱画布 */
.ws-graph-canvas {
  flex: 1;
  position: relative;
  overflow: hidden;
  background:
    radial-gradient(ellipse at 30% 20%, rgba(99, 102, 241, 0.04) 0%, transparent 50%),
    radial-gradient(ellipse at 70% 80%, rgba(6, 182, 212, 0.04) 0%, transparent 50%),
    #f8fafc;
}
.ws-graph-svg {
  width: 100%;
  height: 100%;
  cursor: grab;
}
.ws-graph-svg:active { cursor: grabbing; }

.ws-node {
  cursor: pointer;
  transition: transform 0.2s;
}
.ws-node:hover {
  transform: scale(1.1);
}
.ws-node.selected circle:first-child {
  stroke: #6366f1;
  stroke-width: 3;
  stroke-dasharray: 4 2;
}
.ws-edge {
  transition: stroke-opacity 0.2s;
}

/* 节点信息卡片 */
.ws-node-info-card {
  position: absolute;
  width: 260px;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(99, 102, 241, 0.15);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(15, 23, 42, 0.1);
  z-index: 10;
  overflow: hidden;
  animation: slideIn 0.2s ease;
}
@keyframes slideIn {
  from { opacity: 0; transform: translateY(-8px); }
  to { opacity: 1; transform: translateY(0); }
}
.ws-node-info-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(99, 102, 241, 0.08);
  position: relative;
}
.ws-node-info-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  color: white;
  font-size: 12px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ws-node-info-title {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}
.ws-node-info-type {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}
.ws-node-close {
  position: absolute;
  right: 10px;
  top: 10px;
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  border-radius: 4px;
  color: #94a3b8;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ws-node-close:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}
.ws-node-info-body {
  padding: 12px 14px;
}
.ws-node-info-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 0;
  font-size: 12px;
}
.ws-node-info-label { color: #64748b; }
.ws-node-info-value { font-weight: 600; color: #0f172a; }
.ws-node-info-actions {
  display: flex;
  gap: 8px;
  padding: 0 14px 12px;
}
.ws-node-info-actions .el-button { flex: 1; }

/* 底部协作栏 */
.ws-collab-bar {
  border-top: 1px solid rgba(99, 102, 241, 0.1);
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(10px);
  flex-shrink: 0;
  transition: all 0.3s ease;
}
.ws-collab-header {
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  cursor: pointer;
  transition: background 0.2s;
}
.ws-collab-header:hover { background: rgba(99, 102, 241, 0.03); }
.ws-collab-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}
.ws-collab-count {
  font-size: 11px;
  color: #64748b;
  font-weight: 400;
  background: rgba(99, 102, 241, 0.08);
  padding: 2px 8px;
  border-radius: 10px;
}
.ws-collab-toggle {
  color: #64748b;
  display: flex;
  align-items: center;
}

.ws-collab-body {
  height: 180px;
  display: flex;
  flex-direction: column;
  padding: 0 16px 12px;
}
.ws-collab-messages {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 8px 0;
}
.ws-collab-msg {
  display: flex;
  gap: 8px;
}
.ws-collab-msg.user { flex-direction: row-reverse; }
.ws-collab-msg-content {
  max-width: 70%;
}
.ws-collab-msg-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: #64748b;
  margin-bottom: 4px;
}
.ws-collab-msg.user .ws-collab-msg-meta { justify-content: flex-end; }
.ws-collab-msg-name { font-weight: 600; }
.ws-collab-msg-text {
  background: rgba(99, 102, 241, 0.06);
  padding: 8px 12px;
  border-radius: 10px;
  font-size: 13px;
  line-height: 1.5;
}
.ws-collab-msg.user .ws-collab-msg-text {
  background: linear-gradient(135deg, #6366f1, #06b6d4);
  color: white;
}

.ws-collab-input {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
.ws-collab-input-field {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid rgba(99, 102, 241, 0.15);
  border-radius: 8px;
  font-size: 13px;
  outline: none;
  background: white;
  transition: border-color 0.2s;
}
.ws-collab-input-field:focus {
  border-color: #6366f1;
}

/* ========== 右栏：知识库 ========== */
.ws-kb-tabs {
  display: flex;
  background: rgba(99, 102, 241, 0.06);
  border-radius: 8px;
  padding: 3px;
  margin-bottom: 12px;
  gap: 2px;
}
.ws-kb-tab {
  flex: 1;
  padding: 5px 0;
  border: none;
  background: transparent;
  border-radius: 6px;
  font-size: 12px;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
}
.ws-kb-tab:hover { color: #6366f1; }
.ws-kb-tab.active {
  background: white;
  color: #6366f1;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(0,0,0,0.06);
}

.ws-kb-search {
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(99, 102, 241, 0.04);
  border: 1px solid rgba(99, 102, 241, 0.1);
  border-radius: 8px;
  padding: 6px 10px;
  margin-bottom: 12px;
  color: #64748b;
}
.ws-kb-search-input {
  flex: 1;
  border: none;
  background: transparent;
  outline: none;
  font-size: 12px;
  color: #0f172a;
}

/* 文档列表 */
.ws-doc-category { margin-bottom: 16px; }
.ws-doc-category-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: #64748b;
  margin-bottom: 8px;
  padding: 0 4px;
}

.ws-doc-list { display: flex; flex-direction: column; gap: 2px; }
.ws-doc-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}
.ws-doc-item:hover { background: rgba(99, 102, 241, 0.06); }
.ws-doc-item.active {
  background: rgba(99, 102, 241, 0.1);
  border: 1px solid rgba(99, 102, 241, 0.2);
}
.ws-doc-icon { font-size: 20px; flex-shrink: 0; }
.ws-doc-info { flex: 1; min-width: 0; }
.ws-doc-name {
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ws-doc-meta {
  font-size: 11px;
  color: #94a3b8;
  margin-top: 2px;
}
.ws-doc-badge { font-size: 12px; flex-shrink: 0; }

/* 标签云 */
.ws-tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 8px 4px;
  justify-content: center;
}
.ws-tag-cloud-item {
  padding: 4px 10px;
  background: rgba(99, 102, 241, 0.06);
  border: 1px solid rgba(99, 102, 241, 0.1);
  border-radius: 20px;
  cursor: pointer;
  transition: all 0.2s;
  color: #6366f1;
}
.ws-tag-cloud-item:hover {
  background: #6366f1;
  color: white;
  transform: translateY(-1px);
}

/* 版本列表 */
.ws-version-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.ws-version-item {
  padding: 10px 12px;
  background: rgba(255, 255, 255, 0.5);
  border: 1px solid rgba(99, 102, 241, 0.08);
  border-radius: 8px;
  border-left: 3px solid #94a3b8;
}
.ws-version-item.latest {
  border-left-color: #10b981;
  background: rgba(16, 185, 129, 0.04);
}
.ws-version-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}
.ws-version-badge {
  font-size: 10px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
}
.ws-version-item.latest .ws-version-badge {
  background: rgba(16, 185, 129, 0.12);
  color: #10b981;
}
.ws-version-time { font-size: 11px; color: #94a3b8; }
.ws-version-doc {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 4px;
}
.ws-version-author { font-size: 11px; color: #64748b; }

/* 知识库操作按钮 */
.ws-kb-actions {
  display: flex;
  gap: 8px;
  margin-top: auto;
  padding-top: 12px;
  border-top: 1px solid rgba(99, 102, 241, 0.08);
}
.ws-kb-action-btn { flex: 1; gap: 4px; }

/* ========== AI 助手浮窗 ========== */
.ws-ai-assistant {
  position: absolute;
  right: 20px;
  bottom: 20px;
  width: 320px;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(12px);
  border: 1px solid rgba(99, 102, 241, 0.15);
  border-radius: 16px;
  box-shadow: 0 12px 40px rgba(15, 23, 42, 0.15);
  z-index: 50;
  overflow: hidden;
  animation: floatUp 0.3s ease;
}
@keyframes floatUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}
.ws-ai-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 16px;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.08), rgba(6, 182, 212, 0.08));
  border-bottom: 1px solid rgba(99, 102, 241, 0.08);
}
.ws-ai-icon { font-size: 22px; }
.ws-ai-title {
  flex: 1;
  font-weight: 700;
  font-size: 14px;
}
.ws-ai-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: 6px;
  color: #64748b;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ws-ai-close:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}
.ws-ai-body { padding: 16px; }
.ws-ai-suggest-title {
  font-size: 12px;
  font-weight: 600;
  color: #64748b;
  margin-bottom: 10px;
}
.ws-ai-suggest-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}
.ws-ai-suggest-btn {
  padding: 10px 12px;
  background: rgba(99, 102, 241, 0.04);
  border: 1px solid rgba(99, 102, 241, 0.1);
  border-radius: 8px;
  font-size: 12px;
  color: #0f172a;
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
}
.ws-ai-suggest-btn:hover {
  border-color: #6366f1;
  background: rgba(99, 102, 241, 0.08);
  transform: translateY(-1px);
}

/* ========== 滚动条 ========== */
.ws-panel-body::-webkit-scrollbar,
.ws-collab-messages::-webkit-scrollbar {
  width: 6px;
}
.ws-panel-body::-webkit-scrollbar-track,
.ws-collab-messages::-webkit-scrollbar-track {
  background: transparent;
}
.ws-panel-body::-webkit-scrollbar-thumb,
.ws-collab-messages::-webkit-scrollbar-thumb {
  background: rgba(99, 102, 241, 0.2);
  border-radius: 3px;
}
.ws-panel-body::-webkit-scrollbar-thumb:hover,
.ws-collab-messages::-webkit-scrollbar-thumb:hover {
  background: rgba(99, 102, 241, 0.4);
}
</style>
