<!--
  专家联盟统一工作台 · Expert Alliance Unified Workspace
  ======================================================
  架构原则：前端融合 · 后端模块化
  三栏布局：左(专家联盟) | 中(图谱画布+协作) | 右(知识库云盘)
  P0 体验整合：真实 API 集成 · 三栏联动 · 折叠展开
-->

<template>
  <div class="expert-workspace">
    <!-- ========== 顶部全局工具栏（玻璃拟态升级版） ========== -->
    <header class="ws-header glass-header">
      <!-- 渐变装饰条 -->
      <div class="ws-header-gradient-bar"></div>

      <div class="ws-header-left">
        <div class="ws-logo">
          <div class="ws-logo-icon-wrap">
            <span class="ws-logo-icon">🕸️</span>
          </div>
          <span class="ws-logo-text">专家联盟工作台</span>
        </div>
        <el-divider direction="vertical" class="ws-header-divider" />
        <div class="ws-project-selector">
          <el-select v-model="currentProject" size="small" class="ws-project-select" @change="onProjectChange">
            <el-option label="璇玑知识工程" value="xuanji" />
            <el-option label="MOX 平台架构" value="mox" />
            <el-option label="AI 算法实验室" value="ailab" />
          </el-select>
        </div>
      </div>

      <div class="ws-header-center">
        <div class="ws-mode-tabs glass-tabs">
          <button
            v-for="(mode, idx) in workModes"
            :key="mode.key"
            class="ws-mode-tab"
            :class="{ active: activeMode === mode.key, 'mode-enter': modeTransitioning }"
            @click="switchWorkMode(mode.key)"
          >
            <div class="ws-mode-icon-wrap" :style="{ background: mode.gradient }">
              <el-icon class="ws-mode-icon"><component :is="mode.iconComp" /></el-icon>
            </div>
            <span class="ws-mode-label">{{ mode.label }}</span>
            <span class="ws-mode-shortcut">Ctrl+{{ idx + 1 }}</span>
          </button>
        </div>
      </div>

      <div class="ws-header-right">
        <div class="ws-global-search glass-search">
          <el-icon class="search-icon"><Search /></el-icon>
          <el-input
            v-model="globalSearch"
            class="ws-search-input"
            placeholder="全局搜索：专家 / 文档 / 节点…"
            clearable
            @keyup.enter="doGlobalSearch"
            @clear="doGlobalSearch"
          >
            <template #append>
              <span class="ws-search-kbd">⌘K</span>
            </template>
          </el-input>
        </div>
        <el-button size="small" class="ws-ai-btn gradient-btn" @click="openAIAssistant">
          <el-icon><MagicStick /></el-icon>
          <span>AI 协作</span>
        </el-button>
        <el-badge :value="notifCount" :hidden="!hasNotifications" class="ws-notif-badge">
          <el-button size="small" text class="ws-icon-btn" title="通知">
            <el-icon><Bell /></el-icon>
          </el-button>
        </el-badge>
        <div class="ws-user-avatar-wrap">
          <el-avatar :size="36" class="ws-avatar gradient-avatar">U</el-avatar>
          <span class="ws-avatar-online-dot"></span>
        </div>
      </div>
    </header>

    <!-- KPI 指标卡 -->
    <div class="ws-kpi-row">
      <div
        v-for="kpi in kpiCards"
        :key="kpi.key"
        class="ws-kpi-card glass-card"
        @click="onKpiClick(kpi.key)"
      >
        <div class="ws-kpi-icon" :style="{ background: kpi.gradient }">
          <span>{{ kpi.icon }}</span>
        </div>
        <div class="ws-kpi-info">
          <div class="ws-kpi-value">{{ kpi.value }}</div>
          <div class="ws-kpi-label">{{ kpi.label }}</div>
        </div>
        <div class="ws-kpi-trend" :class="kpi.trend > 0 ? 'up' : 'down'">
          <el-icon><component :is="kpi.trend > 0 ? 'Top' : 'Bottom'" /></el-icon>
          <span>{{ Math.abs(kpi.trend) }}%</span>
        </div>
        <div class="ws-kpi-gradient-bar" :style="{ background: kpi.gradient }"></div>
      </div>
    </div>

    <!-- ========== 主工作区 · 三栏布局 ========== -->
    <div class="ws-main">
      <!-- ---- 左栏：专家联盟面板 ---- -->
      <aside
        class="ws-panel ws-panel-left"
        :class="{ collapsed: leftCollapsed }"
      >
        <div class="ws-panel-header">
          <span v-if="!leftCollapsed" class="ws-panel-title">
            <span class="ws-panel-icon">👥</span>
            专家联盟
            <el-tag size="small" type="success" effect="light" class="ws-online-tag">
              {{ onlineExpertCount }} 在线
            </el-tag>
          </span>
          <button class="ws-panel-toggle" @click="leftCollapsed = !leftCollapsed" :title="leftCollapsed ? '展开' : '收起'">
            <el-icon v-if="!leftCollapsed"><ArrowLeft /></el-icon>
            <el-icon v-else><ArrowRight /></el-icon>
          </button>
        </div>

        <div v-if="!leftCollapsed" class="ws-panel-body">
          <!-- 专家筛选搜索 -->
          <div class="ws-expert-filter">
            <el-select v-model="expertFilterType" placeholder="类型" clearable size="small" class="ws-filter-select">
              <el-option v-for="(label, key) in EXPERT_TYPES" :key="key" :label="label" :value="key" />
            </el-select>
            <el-input v-model="expertSearch" placeholder="搜索专家…" clearable size="small" class="ws-filter-search">
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
          </div>

          <!-- 专家列表 -->
          <div class="ws-expert-section">
            <div class="ws-section-label">
              <span>专家列表</span>
              <div class="ws-section-actions">
                <el-button size="small" text class="ws-smart-match-btn" @click="openSmartRouteDialog">
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
                @click="handleExpertClick(expert)"
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
              <el-button size="small" text class="ws-add-btn" @click="newCollaboration">
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
                @click="selectSession(session)"
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
              <button class="ws-tool-btn tool-card" :class="{ active: activeMode === 'debate' }" @click="openDebateDialog">
                <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #ef4444, #f97316)">
                  <span class="ws-tool-icon">⚔️</span>
                </div>
                <span>专家辩论</span>
              </button>
              <button class="ws-tool-btn tool-card" :class="{ active: activeMode === 'orchestration' }" @click="triggerOrchestration">
                <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #7c3aed, #06b6d4)">
                  <span class="ws-tool-icon">🎯</span>
                </div>
                <span>任务编排</span>
              </button>
              <button class="ws-tool-btn tool-card" @click="triggerVoting">
                <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #10b981, #14b8a6)">
                  <span class="ws-tool-icon">🗳️</span>
                </div>
                <span>融合投票</span>
              </button>
              <button class="ws-tool-btn tool-card" :class="{ active: activeMode === 'collaboration' }" @click="openMultiConsultDialog">
                <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #8b5cf6, #ec4899)">
                  <span class="ws-tool-icon">💬</span>
                </div>
                <span>多专家咨询</span>
              </button>
              <button class="ws-tool-btn tool-card" @click="showRegisterDialog = true">
                <div class="ws-tool-icon-wrap" style="background: linear-gradient(135deg, #f59e0b, #ef4444)">
                  <span class="ws-tool-icon">➕</span>
                </div>
                <span>注册专家</span>
              </button>
              <button class="ws-tool-btn tool-card" @click="openSmartRouteDialog">
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
            @click="leftCollapsed = false; selectExpert(expert)"
          >
            <div class="ws-collapsed-avatar-inner" :style="{ background: expertColor(expert.type) }">
              {{ expertEmoji(expert.type) }}
            </div>
          </button>
          <el-divider class="ws-collapsed-divider" />
          <button class="ws-collapsed-icon-btn" title="新建会话" @click="leftCollapsed = false; newCollaboration()">
            <el-icon><Plus /></el-icon>
          </button>
        </div>
      </aside>

      <!-- ---- 中栏：图谱画布 + 协作讨论 ---- -->
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
              <el-icon><component :is="tool.icon" /></el-icon>
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
                @click="switchLayout(layout.key)"
              >
                <span>{{ layout.icon }}</span>
                <span class="ws-layout-label">{{ layout.label }}</span>
              </button>
            </div>
          </div>

          <div class="ws-canvas-tools-right">
            <div class="ws-graph-stats">
              <span class="ws-stat-item"><strong>{{ graphStats.nodes }}</strong> 节点</span>
              <span class="ws-stat-divider">·</span>
              <span class="ws-stat-item"><strong>{{ graphStats.edges }}</strong> 关系</span>
              <span class="ws-stat-divider">·</span>
              <span class="ws-stat-item"><strong>{{ graphStats.types }}</strong> 类型</span>
            </div>
            <el-button size="small" type="primary" plain @click="runGraphAlgo" :loading="graphAnalyzing">
              <el-icon><DataAnalysis /></el-icon>
              图谱分析
            </el-button>
          </div>
        </div>

        <!-- 图谱画布区 -->
        <div class="ws-graph-canvas" ref="canvasRef" @mousedown="onCanvasMouseDown" @mousemove="onCanvasMouseMove" @mouseup="onCanvasMouseUp" @wheel="onCanvasWheel">
          <svg class="ws-graph-svg" :viewBox="svgViewBox" preserveAspectRatio="xMidYMid meet">
            <defs>
              <pattern id="wsGrid" width="40" height="40" patternUnits="userSpaceOnUse" patternTransform="translate(0,0)">
                <path d="M 40 0 L 0 0 0 40" fill="none" stroke="rgba(99,102,241,0.06)" stroke-width="1"/>
              </pattern>
              <radialGradient id="nodeGlow" cx="50%" cy="50%" r="50%">
                <stop offset="0%" style="stop-color:#6366f1;stop-opacity:0.35" />
                <stop offset="100%" style="stop-color:#6366f1;stop-opacity:0" />
              </radialGradient>
              <radialGradient id="nodeGlowCyan" cx="50%" cy="50%" r="50%">
                <stop offset="0%" style="stop-color:#06b6d4;stop-opacity:0.35" />
                <stop offset="100%" style="stop-color:#06b6d4;stop-opacity:0" />
              </radialGradient>
              <filter id="nodeShadow" x="-50%" y="-50%" width="200%" height="200%">
                <feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#0f172a" flood-opacity="0.15"/>
              </filter>
              <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                <polygon points="0 0, 10 3.5, 0 7" fill="#94a3b8" opacity="0.6"/>
              </marker>
            </defs>
            <rect width="100%" height="100%" fill="url(#wsGrid)" />

            <!-- 边（关系） -->
            <g class="ws-edges">
              <line v-for="edge in graphEdges" :key="edge.id"
                :x1="edge.sourceX" :y1="edge.sourceY"
                :x2="edge.targetX" :y2="edge.targetY"
                :stroke="edge.color || '#94a3b8'"
                :stroke-width="edge.width || 1.5"
                :stroke-opacity="edge.highlight ? 0.9 : 0.5"
                class="ws-edge"
                :class="{ highlight: edge.highlight }"
              />
            </g>

            <!-- 节点 -->
            <g class="ws-nodes">
              <g v-for="node in graphNodes" :key="node.id"
                :transform="`translate(${node.x}, ${node.y})`"
                class="ws-node"
                :class="{ selected: selectedNode?.id === node.id, highlight: node.highlight }"
                @click.stop="selectNode(node)"
                @mousedown.stop="onNodeMouseDown($event, node)"
              >
                <circle v-if="node.highlight || selectedNode?.id === node.id" :r="(node.size || 18) + 10" :fill="node.id === selectedNode?.id ? 'url(#nodeGlow)' : 'url(#nodeGlowCyan)'" />
                <circle :r="node.size || 18" :fill="nodeColor(node)" :opacity="0.9" filter="url(#nodeShadow)" />
                <text text-anchor="middle" :dy="(node.size || 18) > 20 ? 4 : 3" fill="white" :font-size="(node.size || 18) > 20 ? 11 : 10" font-weight="600">
                  {{ node.label }}
                </text>
              </g>
            </g>
          </svg>

          <!-- 选中节点信息浮层 -->
          <div v-if="selectedNode" class="ws-node-info-card" :style="nodeCardStyle">
            <div class="ws-node-info-header">
              <span class="ws-node-info-icon" :style="{ background: nodeColor(selectedNode) }">{{ selectedNode.label }}</span>
              <div class="ws-node-info-head-text">
                <div class="ws-node-info-title">{{ selectedNode.fullName || selectedNode.label }}</div>
                <div class="ws-node-info-type">{{ selectedNode.type }}</div>
              </div>
              <button class="ws-node-close" @click.stop="selectedNode = null">
                <el-icon><Close /></el-icon>
              </button>
            </div>
            <div class="ws-node-info-body">
              <div class="ws-node-info-row">
                <span class="ws-node-info-label">关联文档</span>
                <span class="ws-node-info-value">{{ selectedNode.docs || 0 }} 篇</span>
              </div>
              <div class="ws-node-info-row">
                <span class="ws-node-info-label">关联专家</span>
                <span class="ws-node-info-value">{{ selectedNode.experts || 0 }} 位</span>
              </div>
              <div class="ws-node-info-row">
                <span class="ws-node-info-label">中心性排名</span>
                <span class="ws-node-info-value">#{{ selectedNode.rank || '-' }}</span>
              </div>
              <div v-if="selectedNode.description" class="ws-node-info-desc">
                {{ selectedNode.description }}
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

          <!-- 加载遮罩 -->
          <div v-if="graphLoading" class="ws-graph-loading">
            <el-icon class="is-loading ws-loading-icon"><Loading /></el-icon>
            <span>加载图谱数据…</span>
          </div>
        </div>

        <!-- ========== 任务编排模式视图 ========== -->
        <div v-show="activeMode === 'orchestration'" class="task-orch-view">
          <!-- 顶部：任务概览 + 控制栏 -->
          <div class="orch-top-bar glass-card">
            <div class="orch-progress-section">
              <div class="orch-progress-header">
                <span class="orch-progress-title">
                  <span class="orch-title-icon">🎯</span>
                  任务总览
                </span>
                <span class="orch-progress-stats">
                  <el-tag size="small" type="success" effect="light">{{ orchProgress.completed }} 已完成</el-tag>
                  <el-tag size="small" type="primary" effect="light">{{ orchProgress.inProgress }} 进行中</el-tag>
                  <el-tag size="small" type="info" effect="light">{{ orchProgress.total - orchProgress.completed - orchProgress.inProgress }} 待处理</el-tag>
                  <el-tag size="small" type="danger" effect="light" v-if="orchProgress.failed > 0">{{ orchProgress.failed }} 失败</el-tag>
                </span>
              </div>
              <div class="orch-progress-bar-wrap">
                <div class="orch-progress-bar">
                  <div class="orch-progress-fill" :style="{ width: orchProgress.percentage + '%' }"></div>
                  <div class="orch-progress-glow" :style="{ width: orchProgress.percentage + '%' }"></div>
                </div>
                <span class="orch-progress-text">{{ orchProgress.percentage }}%</span>
              </div>
            </div>
            <div class="orch-control-section">
              <el-select v-model="taskOrchestration.executionMode" size="small" class="orch-mode-select">
                <el-option label="自动执行" value="auto" />
                <el-option label="手动执行" value="manual" />
              </el-select>
              <el-button size="small" class="orch-btn-secondary" @click="resetAllTasks">
                <el-icon><RefreshLeft /></el-icon>
                重置
              </el-button>
              <el-button 
                size="small" 
                type="primary" 
                class="gradient-btn orch-btn-primary"
                @click="startTaskExecution"
                :disabled="taskOrchestration.subtasks.length === 0 || orchIsRunning"
                :loading="orchIsRunning"
              >
                <el-icon><Promotion /></el-icon>
                {{ orchIsRunning ? '执行中...' : '开始执行' }}
              </el-button>
            </div>
          </div>

          <!-- 三栏工作区 -->
          <div class="orch-main-area">
            <!-- ---- 左栏：任务拆解面板 ---- -->
            <div class="orch-panel orch-panel-left glass-card">
              <div class="orch-panel-header">
                <span class="orch-panel-title">
                  <span class="orch-panel-icon">📋</span>
                  任务拆解
                </span>
                <span class="orch-task-count">{{ taskOrchestration.subtasks.length }} 个子任务</span>
              </div>

              <!-- 原始任务输入 -->
              <div class="orch-task-input-section">
                <div class="orch-input-label">原始任务描述</div>
                <el-input
                  v-model="taskOrchestration.originalTask"
                  type="textarea"
                  :rows="3"
                  placeholder="请输入需要完成的复杂任务描述，AI 将自动拆解为子任务…"
                  resize="none"
                  class="orch-task-input"
                />
                <div class="orch-input-actions">
                  <el-button 
                    type="primary" 
                    class="gradient-btn orch-decompose-btn"
                    @click="decomposeTask"
                    :loading="decomposing"
                    :disabled="!taskOrchestration.originalTask.trim()"
                  >
                    <el-icon><MagicStick /></el-icon>
                    智能拆解
                  </el-button>
                  <el-button 
                    class="orch-add-manual-btn"
                    @click="addSubtaskManually"
                  >
                    <el-icon><Plus /></el-icon>
                    手动添加
                  </el-button>
                </div>
              </div>

              <!-- 子任务列表 -->
              <div class="orch-subtask-list">
                <div class="orch-list-header">
                  <span class="orch-list-title">子任务列表</span>
                  <div class="orch-list-actions">
                    <el-button size="small" text @click="collapseAllSubtasks">
                      <el-icon><Fold /></el-icon>
                      全部折叠
                    </el-button>
                  </div>
                </div>
                <el-scrollbar class="orch-subtask-scroll">
                  <div
                    v-for="(task, index) in taskOrchestration.subtasks"
                    :key="task.id"
                    class="orch-subtask-card"
                    :class="{ 
                      'is-selected': activeSubtaskId === task.id,
                      'is-dragging': draggingTaskId === task.id,
                      'drag-over': dragOverTaskId === task.id
                    }"
                    draggable="true"
                    @dragstart="onTaskDragStart($event, task)"
                    @dragend="onTaskDragEnd"
                    @dragover.prevent="onTaskDragOver($event, task)"
                    @drop="onTaskDrop($event, task)"
                    @click="selectSubtask(task)"
                  >
                    <div class="subtask-card-header">
                      <div class="subtask-index" :style="{ background: subtaskPriorityGradient(task.priority) }">
                        {{ index + 1 }}
                      </div>
                      <div class="subtask-title-row">
                        <span class="subtask-title">{{ task.title }}</span>
                        <div class="subtask-status-badge" :class="'status-' + task.status">
                          <span class="status-dot"></span>
                          {{ subtaskStatusText(task.status) }}
                        </div>
                      </div>
                    </div>
                    <div class="subtask-card-body">
                      <p class="subtask-desc">{{ task.description }}</p>
                      <div class="subtask-meta-row">
                        <el-tag 
                          size="small" 
                          effect="light"
                          :style="{ borderColor: expertColor(task.suggestedExpertType) + '50', color: expertColor(task.suggestedExpertType) }"
                        >
                          {{ expertEmoji(task.suggestedExpertType) }} {{ EXPERT_TYPES[task.suggestedExpertType] || task.suggestedExpertType }}
                        </el-tag>
                        <span class="subtask-time">
                          <el-icon><Clock /></el-icon>
                          {{ task.estimatedTime }}分钟
                        </span>
                      </div>
                      <div v-if="task.dependencies && task.dependencies.length > 0" class="subtask-deps">
                        <span class="deps-label">依赖:</span>
                        <span 
                          v-for="depId in task.dependencies" 
                          :key="depId"
                          class="dep-tag"
                        >
                          #{{ getSubtaskIndex(depId) + 1 }}
                        </span>
                      </div>
                    </div>
                    <div class="subtask-card-actions">
                      <button class="subtask-action-btn" @click.stop="editSubtask(task)" title="编辑">
                        <el-icon><Edit /></el-icon>
                      </button>
                      <button class="subtask-action-btn delete" @click.stop="deleteSubtask(task.id)" title="删除">
                        <el-icon><Delete /></el-icon>
                      </button>
                      <button class="subtask-action-btn" @click.stop="toggleSubtaskExpand(task)" title="展开详情">
                        <el-icon><component :is="task.expanded ? 'ArrowUp' : 'ArrowDown'" /></el-icon>
                      </button>
                    </div>

                    <!-- 展开的详情 -->
                    <div v-if="task.expanded" class="subtask-expanded-detail">
                      <div class="detail-section">
                        <div class="detail-label">分配专家</div>
                        <div class="assigned-experts">
                          <div
                            v-for="expId in task.expertIds"
                            :key="expId"
                            class="assigned-expert-avatar"
                            :style="{ background: expertGradient(getExpertById(expId)?.type) }"
                            :title="getExpertById(expId)?.name"
                          >
                            {{ expertEmoji(getExpertById(expId)?.type) }}
                          </div>
                          <button v-if="task.expertIds.length === 0" class="add-expert-btn" @click.stop="openAssignDialog(task)">
                            <el-icon><Plus /></el-icon>
                            分配专家
                          </button>
                        </div>
                      </div>
                      <div v-if="task.result" class="detail-section">
                        <div class="detail-label">执行结果</div>
                        <div class="task-result-text">{{ task.result }}</div>
                      </div>
                    </div>
                  </div>
                  <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无子任务，请输入任务描述并点击智能拆解" :image-size="60">
                    <template #description>
                      <div class="orch-empty-hint">
                        <p>输入任务描述后点击「智能拆解」</p>
                        <p class="hint-sub">AI 将自动分析并拆分为可执行的子任务</p>
                      </div>
                    </template>
                  </el-empty>
                </el-scrollbar>
              </div>
            </div>

            <!-- ---- 中栏：专家分配区域 ---- -->
            <div class="orch-panel orch-panel-center glass-card">
              <div class="orch-panel-header">
                <span class="orch-panel-title">
                  <span class="orch-panel-icon">👥</span>
                  专家分配
                </span>
                <el-button size="small" text class="orch-auto-assign-btn" @click="autoAssignExperts" :disabled="taskOrchestration.subtasks.length === 0">
                  <el-icon><MagicStick /></el-icon>
                  智能分配
                </el-button>
              </div>

              <!-- 专家池 -->
              <div class="orch-expert-pool">
                <div class="orch-pool-header">
                  <span class="orch-pool-title">可用专家池</span>
                  <span class="orch-pool-count">{{ availableExperts.length }} 位</span>
                </div>
                <div class="orch-expert-grid">
                  <div
                    v-for="expert in availableExperts"
                    :key="expert.id"
                    class="orch-expert-chip"
                    :class="{ 'is-busy': expert.status === 'busy' }"
                    draggable="true"
                    @dragstart="onExpertDragStart($event, expert)"
                    @dragend="onExpertDragEnd"
                    :title="expert.name + ' - ' + (expert.capabilities?.join('、') || '')"
                  >
                    <div class="chip-avatar gradient-avatar" :style="{ background: expertGradient(expert.type) }">
                      {{ expertEmoji(expert.type) }}
                      <span class="chip-status-dot" :class="'dot-' + expert.status"></span>
                    </div>
                    <div class="chip-info">
                      <span class="chip-name">{{ expert.name }}</span>
                      <span class="chip-role">{{ EXPERT_TYPES[expert.type] }}</span>
                    </div>
                    <div class="chip-load" :title="'当前负载: ' + expertLoad(expert.id) + ' 个任务'">
                      <div class="load-bar">
                        <div class="load-fill" :style="{ width: Math.min(expertLoad(expert.id) * 25, 100) + '%' }"></div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              <!-- 任务分配看板 -->
              <div class="orch-assignment-board">
                <div class="orch-board-header">
                  <span class="orch-board-title">任务分配看板</span>
                  <span class="orch-board-hint">拖拽专家到任务卡片上进行分配</span>
                </div>
                <el-scrollbar class="orch-board-scroll">
                  <div
                    v-for="(task, index) in taskOrchestration.subtasks"
                    :key="task.id"
                    class="orch-task-assign-card"
                    :class="{ 
                      'drag-over': expertDragOverTaskId === task.id,
                      'status-' + task.status
                    }"
                    @dragover.prevent="onExpertDragOverTask($event, task)"
                    @dragleave="onExpertDragLeaveTask"
                    @drop="onExpertDropOnTask($event, task)"
                    @click="selectSubtask(task)"
                  >
                    <div class="assign-card-left">
                      <div class="assign-task-index" :style="{ background: subtaskPriorityGradient(task.priority) }">
                        {{ index + 1 }}
                      </div>
                    </div>
                    <div class="assign-card-body">
                      <div class="assign-task-title-row">
                        <span class="assign-task-title">{{ task.title }}</span>
                        <el-tag size="small" :type="subtaskStatusTagType(task.status)" effect="light">
                          {{ subtaskStatusText(task.status) }}
                        </el-tag>
                      </div>
                      <p class="assign-task-desc">{{ task.description }}</p>
                      <div class="assign-experts-row">
                        <div class="assigned-experts-list">
                          <div
                            v-for="expId in task.expertIds"
                            :key="expId"
                            class="assigned-expert-chip"
                            :style="{ borderColor: expertColor(getExpertById(expId)?.type) }"
                          >
                            <span class="chip-avatar-sm" :style="{ background: expertGradient(getExpertById(expId)?.type) }">
                              {{ expertEmoji(getExpertById(expId)?.type) }}
                            </span>
                            <span class="chip-name-sm">{{ getExpertById(expId)?.name }}</span>
                            <button class="chip-remove" @click.stop="unassignExpert(task.id, expId)">
                              <el-icon><Close /></el-icon>
                            </button>
                          </div>
                          <button 
                            v-if="task.expertIds.length === 0" 
                            class="add-expert-inline-btn"
                            @click.stop="openAssignDialog(task)"
                          >
                            <el-icon><Plus /></el-icon>
                            分配专家
                          </button>
                        </div>
                      </div>
                    </div>
                    <div class="assign-card-right">
                      <div class="task-time-estimate">
                        <el-icon><Clock /></el-icon>
                        <span>{{ task.estimatedTime }}分钟</span>
                      </div>
                    </div>
                  </div>
                  <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无待分配任务" :image-size="50" />
                </el-scrollbar>
              </div>
            </div>

            <!-- ---- 右栏：任务执行时间线 ---- -->
            <div class="orch-panel orch-panel-right glass-card">
              <div class="orch-panel-header">
                <span class="orch-panel-title">
                  <span class="orch-panel-icon">📊</span>
                  执行时间线
                </span>
                <div class="orch-timeline-actions">
                  <el-button-group size="small">
                    <el-button :type="timelineView === 'gantt' ? 'primary' : ''" @click="timelineView = 'gantt'">甘特图</el-button>
                    <el-button :type="timelineView === 'list' ? 'primary' : ''" @click="timelineView = 'list'">列表</el-button>
                  </el-button-group>
                </div>
              </div>

              <!-- 甘特图视图 -->
              <div v-show="timelineView === 'gantt'" class="orch-gantt-container">
                <div class="gantt-header">
                  <div class="gantt-task-col">任务</div>
                  <div class="gantt-time-col">
                    <div class="gantt-time-scale">
                      <span v-for="i in ganttTimeSlots" :key="i" class="time-slot">{{ i * ganttSlotMinutes }}分</span>
                    </div>
                  </div>
                </div>
                <el-scrollbar class="gantt-body-scroll">
                  <div class="gantt-body">
                    <div
                      v-for="(task, index) in taskOrchestration.subtasks"
                      :key="task.id"
                      class="gantt-row"
                      :class="{ 'is-selected': activeSubtaskId === task.id }"
                      @click="selectSubtask(task)"
                    >
                      <div class="gantt-task-label">
                        <span class="gantt-task-idx">{{ index + 1 }}.</span>
                        <span class="gantt-task-name">{{ task.title }}</span>
                      </div>
                      <div class="gantt-bar-area">
                        <!-- 背景网格 -->
                        <div class="gantt-grid">
                          <div v-for="i in ganttTimeSlots" :key="i" class="grid-line"></div>
                        </div>
                        <!-- 任务条 -->
                        <div
                          class="gantt-task-bar"
                          :class="'status-' + task.status"
                          :style="{
                            left: (task.ganttOffset || 0) + '%',
                            width: Math.max(task.ganttWidth || 15, 8) + '%'
                          }"
                        >
                          <div class="gantt-bar-fill"></div>
                          <div class="gantt-bar-glow"></div>
                          <span class="gantt-bar-label">{{ task.estimatedTime }}分钟</span>
                        </div>
                        <!-- 依赖连线（SVG） -->
                      </div>
                    </div>
                    <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无任务时间线" :image-size="50" />
                  </div>
                </el-scrollbar>
              </div>

              <!-- 列表视图 -->
              <div v-show="timelineView === 'list'" class="orch-timeline-list">
                <el-scrollbar class="timeline-scroll">
                  <div class="timeline-list-inner">
                    <div
                      v-for="(task, index) in taskOrchestration.subtasks"
                      :key="task.id"
                      class="timeline-item"
                      :class="{ 'is-selected': activeSubtaskId === task.id }"
                      @click="selectSubtask(task)"
                    >
                      <div class="timeline-dot" :class="'status-' + task.status">
                        <el-icon v-if="task.status === 'completed'"><CircleCheckFilled /></el-icon>
                        <span v-else>{{ index + 1 }}</span>
                      </div>
                      <div class="timeline-line" v-if="index < taskOrchestration.subtasks.length - 1"></div>
                      <div class="timeline-content">
                        <div class="timeline-task-title">{{ task.title }}</div>
                        <div class="timeline-task-meta">
                          <span class="timeline-status" :class="'status-' + task.status">
                            {{ subtaskStatusText(task.status) }}
                          </span>
                          <span class="timeline-time">
                            <el-icon><Clock /></el-icon>
                            {{ task.estimatedTime }}分钟
                          </span>
                        </div>
                        <div v-if="task.expertIds && task.expertIds.length > 0" class="timeline-experts">
                          <div
                            v-for="expId in task.expertIds.slice(0, 3)"
                            :key="expId"
                            class="timeline-expert-avatar"
                            :style="{ background: expertGradient(getExpertById(expId)?.type) }"
                            :title="getExpertById(expId)?.name"
                          >
                            {{ expertEmoji(getExpertById(expId)?.type) }}
                          </div>
                          <span v-if="task.expertIds.length > 3" class="timeline-more-experts">
                            +{{ task.expertIds.length - 3 }}
                          </span>
                        </div>
                      </div>
                    </div>
                    <el-empty v-if="taskOrchestration.subtasks.length === 0" description="暂无任务" :image-size="50" />
                  </div>
                </el-scrollbar>
              </div>

              <!-- 风险预警区 -->
              <div v-if="riskTasks.length > 0" class="orch-risk-section">
                <div class="risk-section-header">
                  <span class="risk-icon">⚠️</span>
                  <span class="risk-title">风险预警</span>
                  <el-badge :value="riskTasks.length" class="risk-badge" />
                </div>
                <div class="risk-task-list">
                  <div v-for="task in riskTasks" :key="task.id" class="risk-task-item" @click="selectSubtask(task)">
                    <span class="risk-task-name">{{ task.title }}</span>
                    <el-tag size="small" type="warning" effect="light">有风险</el-tag>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- 底部协作对话栏（增强版） -->
        <div class="ws-collab-bar glass-card" :class="{ expanded: collabExpanded, 'is-running': allianceRunning, 'mode-transition': modeTransitioning }">
          <!-- 渐变装饰条 -->
          <div class="ws-collab-gradient-bar"></div>

          <div class="ws-collab-header" @click="collabExpanded = !collabExpanded">
            <div class="ws-collab-title">
              <div class="ws-collab-title-icon">
                <el-icon v-if="allianceRunning" class="ws-pulse-icon"><Promotion /></el-icon>
                <el-icon v-else><ChatLineSquare /></el-icon>
              </div>
              <span class="ws-collab-title-text">协作讨论 · {{ activeSession?.title || '未开始' }}</span>
              <el-tag v-if="allianceRunning" size="small" effect="light" class="ws-running-tag gradient-tag">
                {{ currentPhaseLabel }}
              </el-tag>
              <span class="ws-collab-count">{{ collabMessages.length }} 条消息</span>
              <!-- 正在输入提示 -->
              <span v-if="typingExperts.length > 0" class="ws-typing-indicator">
                <span class="typing-dots-mini"><i></i><i></i><i></i></span>
                {{ typingExperts.map(e => e.name).join('、') }} 正在思考
              </span>
            </div>
            <div class="ws-collab-header-actions" @click.stop>
              <!-- 历史记录按钮 -->
              <button class="ws-header-action-btn" :class="{ active: historyPanelOpen }" title="历史记录" @click="historyPanelOpen = !historyPanelOpen">
                <el-icon><RefreshRight /></el-icon>
              </button>
              <div class="ws-collab-toggle">
                <el-icon v-if="collabExpanded"><ArrowDown /></el-icon>
                <el-icon v-else><ArrowUp /></el-icon>
              </div>
            </div>
          </div>

          <div v-if="collabExpanded" class="ws-collab-body">
            <!-- 阶段进度可视化 -->
            <div class="ws-phase-progress-bar">
              <div
                v-for="(phase, idx) in projectPhases"
                :key="phase.key"
                class="ws-phase-item"
                :class="{ active: currentProjectPhase === idx, done: currentProjectPhase > idx, 'clickable': true }"
                @click="jumpToPhase(idx)"
              >
                <div class="ws-phase-dot-wrapper">
                  <div class="ws-phase-dot">
                    <el-icon v-if="currentProjectPhase > idx"><CircleCheckFilled /></el-icon>
                    <span v-else>{{ idx + 1 }}</span>
                  </div>
                  <div v-if="idx < projectPhases.length - 1" class="ws-phase-connector" :class="{ filled: currentProjectPhase > idx }"></div>
                </div>
                <span class="ws-phase-label">{{ phase.label }}</span>
              </div>
            </div>

            <!-- 协作内容 Tab -->
            <div class="ws-collab-tabs">
              <button
                v-for="tab in collabTabs"
                :key="tab.key"
                class="ws-collab-tab"
                :class="{ active: activeCollabTab === tab.key }"
                @click="activeCollabTab = tab.key"
              >
                <el-icon class="ws-collab-tab-icon"><component :is="tab.icon" /></el-icon>
                <span>{{ tab.label }}</span>
                <el-badge v-if="tab.badge" :value="tab.badge" class="ws-tab-badge" />
              </button>
              <div class="ws-collab-tabs-right">
                <!-- 协作成员 -->
                <div class="ws-collab-members">
                  <div
                    v-for="(member, idx) in collabMembers.slice(0, 4)"
                    :key="member.id"
                    class="ws-member-avatar"
                    :style="{ background: member.color, zIndex: 10 - idx }"
                    :title="member.name + ' - ' + memberStatusText(member.status)"
                  >
                    {{ member.avatar }}
                    <span class="ws-member-status-dot" :class="'status-' + member.status"></span>
                  </div>
                  <div v-if="collabMembers.length > 4" class="ws-member-avatar more-avatar" :title="`还有 ${collabMembers.length - 4} 位成员`">
                    +{{ collabMembers.length - 4 }}
                  </div>
                </div>
              </div>
            </div>

            <!-- ===== 讨论 Tab ===== -->
            <div v-show="activeCollabTab === 'discussion'" class="ws-tab-content ws-discussion-content">
              <!-- 文件栏 -->
              <div v-if="sharedFiles.length > 0" class="ws-files-bar">
                <div class="ws-files-bar-header">
                  <span class="ws-files-bar-title">
                    <el-icon><FolderOpened /></el-icon>
                    共享文件 ({{ sharedFiles.length }})
                  </span>
                  <el-button size="small" text class="ws-files-bar-toggle" @click="filesBarExpanded = !filesBarExpanded">
                    {{ filesBarExpanded ? '收起' : '展开' }}
                    <el-icon><component :is="filesBarExpanded ? 'ArrowUp' : 'ArrowDown'" /></el-icon>
                  </el-button>
                </div>
                <div v-show="filesBarExpanded" class="ws-files-list">
                  <div
                    v-for="file in sharedFiles"
                    :key="file.id"
                    class="ws-file-card"
                    @click="previewFile(file)"
                  >
                    <div class="ws-file-icon" :class="'file-' + file.type">
                      {{ fileIconEmoji(file.type) }}
                    </div>
                    <div class="ws-file-info">
                      <div class="ws-file-name">{{ file.name }}</div>
                      <div class="ws-file-meta">{{ file.size }} · {{ file.uploader }} · {{ file.time }}</div>
                    </div>
                    <el-button size="small" text class="ws-file-download" @click.stop="downloadFile(file)" title="下载">
                      <el-icon><Download /></el-icon>
                    </el-button>
                  </div>
                </div>
              </div>

              <el-scrollbar class="ws-collab-messages" ref="messagesScrollRef">
                <div v-for="msg in collabMessages" :key="msg.id" class="ws-collab-msg" :class="[msg.role, msg.phase ? `phase-${msg.phase}` : '']">
                  <div class="ws-collab-msg-avatar gradient-avatar" :style="{ background: msg.color || 'linear-gradient(135deg, #6366f1, #06b6d4)' }">
                    {{ msg.avatar || '?' }}
                  </div>
                  <div class="ws-collab-msg-content">
                    <div class="ws-collab-msg-meta">
                      <span class="ws-collab-msg-name">{{ msg.name }}</span>
                      <!-- 消息状态 -->
                      <span v-if="msg.status" class="ws-msg-status" :class="'status-' + msg.status" :title="msgStatusText(msg.status)">
                        <el-icon v-if="msg.status === 'sent'"><CircleCheckFilled /></el-icon>
                        <el-icon v-else-if="msg.status === 'thinking'" class="pulse-icon"><Loading /></el-icon>
                        <el-icon v-else-if="msg.status === 'done'"><CircleCheckFilled /></el-icon>
                        <el-icon v-else-if="msg.status === 'failed'"><Warning /></el-icon>
                      </span>
                      <span v-if="msg.phase" class="ws-collab-msg-phase">
                        <el-tag size="small" effect="plain" :type="phaseTagType(msg.phase)">{{ phaseLabel(msg.phase) }}</el-tag>
                      </span>
                      <span class="ws-collab-msg-time">{{ msg.time }}</span>
                    </div>
                    <div class="ws-collab-msg-text" v-html="formatMessageText(msg.text)"></div>
                    <!-- 消息附件 -->
                    <div v-if="msg.files && msg.files.length > 0" class="ws-msg-files">
                      <div
                        v-for="file in msg.files"
                        :key="file.id"
                        class="ws-msg-file-chip"
                        @click="previewFile(file)"
                      >
                        <span class="msg-file-icon">{{ fileIconEmoji(file.type) }}</span>
                        <span class="msg-file-name">{{ file.name }}</span>
                      </div>
                    </div>
                  </div>
                </div>
                <!-- 正在输入提示 -->
                <div v-for="expert in typingExperts" :key="expert.id" class="ws-collab-msg assistant ws-typing">
                  <div class="ws-collab-msg-avatar gradient-avatar" :style="{ background: expert.color }">
                    {{ expert.avatar }}
                  </div>
                  <div class="ws-collab-msg-content">
                    <div class="ws-collab-msg-meta">
                      <span class="ws-collab-msg-name">{{ expert.name }}</span>
                      <span class="ws-msg-status status-thinking" title="正在思考">
                        <el-icon class="pulse-icon"><Loading /></el-icon>
                      </span>
                    </div>
                    <div class="ws-typing-dots">
                      <span></span><span></span><span></span>
                    </div>
                  </div>
                </div>
                <div v-if="allianceRunning && typingExperts.length === 0" class="ws-collab-msg assistant ws-typing">
                  <div class="ws-collab-msg-avatar gradient-avatar" style="background: linear-gradient(135deg, #7c3aed, #06b6d4)">
                    🤖
                  </div>
                  <div class="ws-collab-msg-content">
                    <div class="ws-collab-msg-meta">
                      <span class="ws-collab-msg-name">AI 协作中</span>
                    </div>
                    <div class="ws-typing-dots">
                      <span></span><span></span><span></span>
                    </div>
                  </div>
                </div>
              </el-scrollbar>

              <div class="ws-collab-input-area">
                <!-- 文件上传拖拽区 -->
                <div
                  v-if="dragOver"
                  class="ws-drop-zone"
                  @dragover.prevent="dragOver = true"
                  @dragleave="dragOver = false"
                  @drop.prevent="handleFileDrop"
                >
                  <el-icon class="drop-zone-icon"><Upload /></el-icon>
                  <div class="drop-zone-text">释放文件以上传</div>
                </div>

                <div class="ws-collab-input-tools">
                  <el-upload
                    :show-file-list="false"
                    :before-upload="handleBeforeFileUpload"
                    multiple
                    class="ws-upload-trigger"
                  >
                    <el-button size="small" text class="ws-tool-mini-btn" title="上传文件">
                      <el-icon><Paperclip /></el-icon>
                    </el-button>
                  </el-upload>
                  <el-button size="small" text class="ws-tool-mini-btn" title="引用图谱节点" @click="insertNodeRef">
                    <el-icon><Share /></el-icon>
                  </el-button>
                  <el-button size="small" text class="ws-tool-mini-btn" title="添加到白板" @click="sendToWhiteboard">
                    <el-icon><CollectionTag /></el-icon>
                  </el-button>
                  <el-select v-model="collabMode" size="small" class="ws-mode-select" @change="onCollabModeChange">
                    <el-option label="智能路由" value="smart" />
                    <el-option label="单专家咨询" value="single" />
                    <el-option label="多专家协同" value="multi" />
                    <el-option label="专家辩论" value="debate" />
                    <el-option label="算法分析" value="algorithm" />
                  </el-select>
                </div>
                <div class="ws-collab-input-row">
                  <el-input
                    v-model="collabInput"
                    class="ws-collab-input-field"
                    type="textarea"
                    :rows="2"
                    placeholder="输入问题或指令… (Enter 发送，Shift+Enter 换行)"
                    resize="none"
                    @keydown.enter.exact.prevent="sendCollabMsg"
                  />
                  <el-button type="primary" class="ws-send-btn gradient-btn" @click="sendCollabMsg" :loading="allianceRunning">
                    <el-icon v-if="!allianceRunning"><Promotion /></el-icon>
                    <span>{{ allianceRunning ? '运行中' : '发送' }}</span>
                  </el-button>
                  <el-button v-if="allianceRunning" type="danger" plain class="ws-stop-btn" @click="stopAlliance">
                    <el-icon><Close /></el-icon>
                    停止
                  </el-button>
                </div>
              </div>
            </div>

            <!-- ===== 白板 Tab ===== -->
            <div v-show="activeCollabTab === 'whiteboard'" class="ws-tab-content ws-whiteboard-content">
              <div class="ws-whiteboard-toolbar">
                <button
                  v-for="tool in whiteboardTools"
                  :key="tool.key"
                  class="ws-wb-tool"
                  :class="{ active: activeWbTool === tool.key }"
                  :title="tool.label"
                  @click="selectWbTool(tool.key)"
                >
                  <span class="ws-wb-tool-icon">{{ tool.icon }}</span>
                  <span class="ws-wb-tool-label">{{ tool.label }}</span>
                </button>
                <div class="ws-wb-tool-divider"></div>
                <div class="ws-wb-color-picker">
                  <span class="wb-color-label">颜色</span>
                  <button
                    v-for="color in wbColors"
                    :key="color"
                    class="wb-color-dot"
                    :class="{ active: activeWbColor === color }"
                    :style="{ background: color }"
                    @click="activeWbColor = color"
                  ></button>
                </div>
                <div class="ws-wb-tool-divider"></div>
                <button class="ws-wb-tool" title="清空画布" @click="clearWhiteboard">
                  <el-icon><Delete /></el-icon>
                  <span class="ws-wb-tool-label">清空</span>
                </button>
              </div>
              <div
                class="ws-whiteboard-canvas"
                ref="whiteboardRef"
                @mousedown="onWbMouseDown"
                @mousemove="onWbMouseMove"
                @mouseup="onWbMouseUp"
                @mouseleave="onWbMouseUp"
              >
                <!-- 便签 -->
                <div
                  v-for="note in wbNotes"
                  :key="note.id"
                  class="wb-sticky-note"
                  :style="{ left: note.x + 'px', top: note.y + 'px', background: note.color }"
                  @mousedown.stop="startDragNote($event, note)"
                >
                  <div class="wb-note-header">
                    <span class="wb-note-title">{{ note.title || '便签' }}</span>
                    <button class="wb-note-delete" @click.stop="deleteWbNote(note.id)" title="删除">×</button>
                  </div>
                  <div class="wb-note-content" contenteditable="true" @blur="updateNoteContent($event, note)">{{ note.content }}</div>
                </div>
                <!-- 文本框 -->
                <div
                  v-for="text in wbTexts"
                  :key="text.id"
                  class="wb-text-box"
                  :style="{ left: text.x + 'px', top: text.y + 'px', color: text.color }"
                  @mousedown.stop="startDragText($event, text)"
                >
                  <div contenteditable="true" @blur="updateTextContent($event, text)">{{ text.content || '双击编辑文本' }}</div>
                  <button class="wb-text-delete" @click.stop="deleteWbText(text.id)" title="删除">×</button>
                </div>
                <!-- SVG 连线和画笔 -->
                <svg class="wb-draw-layer" :viewBox="wbViewBox">
                  <!-- 连线 -->
                  <line
                    v-for="line in wbLines"
                    :key="line.id"
                    :x1="line.x1" :y1="line.y1"
                    :x2="line.x2" :y2="line.y2"
                    :stroke="line.color"
                    stroke-width="2"
                    stroke-dasharray="5,5"
                  />
                  <!-- 自由画笔路径 -->
                  <path
                    v-for="(path, idx) in wbDrawPaths"
                    :key="'path-' + idx"
                    :d="path.d"
                    :stroke="path.color"
                    stroke-width="2"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                  <!-- 当前绘制的线 -->
                  <path
                    v-if="wbCurrentPath"
                    :d="wbCurrentPath"
                    :stroke="activeWbColor"
                    stroke-width="2"
                    fill="none"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    opacity="0.8"
                  />
                </svg>
                <!-- 空状态提示 -->
                <div v-if="wbNotes.length === 0 && wbTexts.length === 0 && wbDrawPaths.length === 0 && wbLines.length === 0" class="wb-empty-hint">
                  <div class="wb-empty-icon">🎨</div>
                  <div class="wb-empty-text">选择工具开始创作</div>
                  <div class="wb-empty-tips">便签 · 连线 · 画笔 · 文本</div>
                </div>
              </div>
              <div class="ws-whiteboard-footer">
                <span class="wb-stats">便签: {{ wbNotes.length }} | 文本: {{ wbTexts.length }} | 连线: {{ wbLines.length }}</span>
                <el-button size="small" type="primary" plain @click="saveWhiteboard">
                  <el-icon><CollectionTag /></el-icon>
                  保存白板
                </el-button>
              </div>
            </div>

            <!-- ===== 文件 Tab ===== -->
            <div v-show="activeCollabTab === 'files'" class="ws-tab-content ws-files-content">
              <div class="ws-files-upload-area"
                @dragover.prevent="fileDragOver = true"
                @dragleave="fileDragOver = false"
                @drop.prevent="handleFileDropToFiles"
                :class="{ 'drag-over': fileDragOver }"
              >
                <el-icon class="upload-area-icon"><Upload /></el-icon>
                <div class="upload-area-text">拖拽文件到此处上传</div>
                <div class="upload-area-hint">或</div>
                <el-upload
                  :show-file-list="false"
                  :before-upload="handleBeforeFileUpload"
                  multiple
                >
                  <el-button type="primary" plain size="small">点击选择文件</el-button>
                </el-upload>
              </div>
              <el-scrollbar class="ws-files-scroll">
                <div v-if="sharedFiles.length === 0" class="ws-files-empty">
                  <el-empty description="暂无共享文件" :image-size="60" />
                </div>
                <div v-else class="ws-files-grid">
                  <div
                    v-for="file in sharedFiles"
                    :key="file.id"
                    class="ws-file-card-large"
                    @click="previewFile(file)"
                  >
                    <div class="ws-file-preview" :class="'preview-' + file.type">
                      <span class="file-preview-icon">{{ fileIconEmoji(file.type) }}</span>
                    </div>
                    <div class="ws-file-card-body">
                      <div class="ws-file-name-row">
                        <span class="ws-file-name-large">{{ file.name }}</span>
                      </div>
                      <div class="ws-file-meta-row">
                        <span>{{ file.size }}</span>
                        <span>·</span>
                        <span>{{ file.uploader }}</span>
                      </div>
                      <div class="ws-file-actions-row">
                        <el-button size="small" text @click.stop="previewFile(file)">
                          <el-icon><Document /></el-icon>
                          预览
                        </el-button>
                        <el-button size="small" text @click.stop="downloadFile(file)">
                          <el-icon><Download /></el-icon>
                          下载
                        </el-button>
                      </div>
                    </div>
                  </div>
                </div>
              </el-scrollbar>
            </div>
          </div>

          <!-- 历史记录侧边栏 -->
          <transition name="slide-fade">
            <div v-if="historyPanelOpen" class="ws-history-panel">
              <div class="ws-history-header">
                <span class="ws-history-title">
                  <el-icon><RefreshRight /></el-icon>
                  协作历史
                </span>
                <button class="ws-history-close" @click="historyPanelOpen = false">
                  <el-icon><Close /></el-icon>
                </button>
              </div>
              <el-scrollbar class="ws-history-scroll">
                <div class="ws-history-timeline">
                  <div
                    v-for="(item, idx) in historyEvents"
                    :key="item.id"
                    class="ws-history-item"
                    :class="'event-' + item.type"
                    @click="jumpToHistory(item)"
                  >
                    <div class="ws-history-dot"></div>
                    <div class="ws-history-content">
                      <div class="ws-history-title-row">
                        <span class="ws-history-icon">{{ historyIcon(item.type) }}</span>
                        <span class="ws-history-event-title">{{ item.title }}</span>
                      </div>
                      <div class="ws-history-desc">{{ item.description }}</div>
                      <div class="ws-history-time">{{ item.time }}</div>
                    </div>
                    <div v-if="idx < historyEvents.length - 1" class="ws-history-line"></div>
                  </div>
                  <el-empty v-if="historyEvents.length === 0" description="暂无历史记录" :image-size="40" />
                </div>
              </el-scrollbar>
            </div>
          </transition>
        </div>
      </main>

      <!-- ---- 右栏：知识库云盘面板 ---- -->
      <aside
        class="ws-panel ws-panel-right"
        :class="{ collapsed: rightCollapsed }"
      >
        <div class="ws-panel-header">
          <button class="ws-panel-toggle" @click="rightCollapsed = !rightCollapsed" :title="rightCollapsed ? '展开' : '收起'">
            <el-icon v-if="!rightCollapsed"><ArrowRight /></el-icon>
            <el-icon v-else><ArrowLeft /></el-icon>
          </button>
          <span v-if="!rightCollapsed" class="ws-panel-title">
            <span class="ws-panel-icon">📚</span>
            知识库云盘
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
              @click="switchKbTab(tab.key)"
            >
              <el-icon><component :is="tab.icon" /></el-icon>
              <span>{{ tab.label }}</span>
            </button>
          </div>

          <!-- 搜索 -->
          <div class="ws-kb-search">
            <el-input v-model="kbSearchQuery" placeholder="搜索文档…" clearable size="small" @keyup.enter="searchKb" @clear="searchKb">
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
          </div>

          <!-- 文档列表 -->
          <div v-if="activeKbTab === 'docs'" class="ws-kb-docs">
            <!-- 分类树 -->
            <div class="ws-doc-categories">
              <div
                v-for="cat in categories"
                :key="cat.id"
                class="ws-doc-category"
                :class="{ active: activeCategory === cat.id }"
                @click="selectCategory(cat)"
              >
                <div class="ws-doc-category-header">
                  <el-icon v-if="expandedCategories.includes(cat.id)"><FolderOpened /></el-icon>
                  <el-icon v-else><Folder /></el-icon>
                  <span class="ws-cat-name">{{ cat.name }}</span>
                  <span class="ws-cat-count">{{ cat.count || 0 }}</span>
                </div>
              </div>
            </div>

            <el-divider class="ws-kb-divider" />

            <!-- 文档列表 -->
            <el-scrollbar class="ws-doc-scroll">
              <div
                v-for="doc in filteredDocs"
                :key="doc.id"
                class="ws-doc-item"
                :class="{ active: activeDoc?.id === doc.id, linked: doc.graph_linked }"
                @click="openDoc(doc)"
              >
                <span class="ws-doc-icon">{{ docIcon(doc.type) }}</span>
                <div class="ws-doc-info">
                  <div class="ws-doc-name">{{ doc.title || doc.name }}</div>
                  <div class="ws-doc-meta">
                    {{ formatFileSize(doc.size) }} · {{ formatTime(doc.updated_at || doc.created_at) }}
                  </div>
                </div>
                <span v-if="doc.graph_linked" class="ws-doc-badge" title="已关联图谱">
                  <el-icon><Link /></el-icon>
                </span>
              </div>
              <el-empty v-if="filteredDocs.length === 0 && docsLoading" description="加载中…" :image-size="40" />
              <el-empty v-else-if="filteredDocs.length === 0" description="暂无文档" :image-size="40" />
            </el-scrollbar>
          </div>

          <!-- 标签云 -->
          <div v-if="activeKbTab === 'tags'" class="ws-kb-tags">
            <div class="ws-tag-section-title">热门标签</div>
            <div class="ws-tag-cloud">
              <span
                v-for="tag in popularTags"
                :key="tag.name"
                class="ws-tag-cloud-item"
                :style="{ fontSize: tag.fontSize + 'px' }"
                @click="filterByTag(tag)"
              >
                {{ tag.name }}
                <span class="ws-tag-count">{{ tag.count }}</span>
              </span>
            </div>
          </div>

          <!-- 版本历史 -->
          <div v-if="activeKbTab === 'versions'" class="ws-kb-versions">
            <div class="ws-version-current-doc" v-if="activeDoc">
              <el-icon><Document /></el-icon>
              <span>{{ activeDoc.title || activeDoc.name }}</span>
            </div>
            <el-scrollbar class="ws-version-scroll">
              <div
                v-for="(ver, idx) in docVersions"
                :key="ver.id || ver.version"
                class="ws-version-item"
                :class="{ latest: idx === 0 }"
              >
                <div class="ws-version-header">
                  <span class="ws-version-badge">{{ idx === 0 ? '当前版本' : '历史版本' }}</span>
                  <span class="ws-version-time">{{ formatTime(ver.created_at) }}</span>
                </div>
                <div class="ws-version-label">v{{ ver.version || (docVersions.length - idx) }}</div>
                <div class="ws-version-author">{{ ver.author || '系统' }} · {{ ver.action || '更新' }}</div>
              </div>
              <el-empty v-if="docVersions.length === 0" description="暂无版本记录" :image-size="40" />
            </el-scrollbar>
          </div>

          <!-- 快捷操作 -->
          <div class="ws-kb-actions">
            <el-upload
              :show-file-list="false"
              :before-upload="handleBeforeUpload"
              class="ws-kb-upload"
            >
              <el-button size="small" class="ws-kb-action-btn">
                <el-icon><Upload /></el-icon>
                上传文档
              </el-button>
            </el-upload>
            <el-button size="small" type="primary" class="ws-kb-action-btn" @click="createDoc">
              <el-icon><Edit /></el-icon>
              新建
            </el-button>
          </div>
        </div>

        <!-- 折叠状态图标 -->
        <div v-else class="ws-collapsed-icons ws-collapsed-right">
          <button class="ws-collapsed-icon-btn" title="知识库" @click="rightCollapsed = false; activeKbTab = 'docs'">
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
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('分析图谱')">
              <span class="ws-suggest-icon">🔍</span>
              分析当前图谱
            </button>
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('推荐专家')">
              <span class="ws-suggest-icon">👥</span>
              推荐相关专家
            </button>
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('生成报告')">
              <span class="ws-suggest-icon">📝</span>
              生成研究报告
            </button>
            <button class="ws-ai-suggest-btn" @click="aiSuggestion('知识问答')">
              <span class="ws-suggest-icon">❓</span>
              知识问答
            </button>
          </div>
        </div>
        <div class="ws-ai-capabilities">
          <div class="ws-ai-suggest-title">联盟能力</div>
          <div class="ws-ai-cap-list">
            <div v-for="cap in allianceCapabilitiesList" :key="cap" class="ws-cap-item">
              <el-icon><CircleCheckFilled /></el-icon>
              <span>{{ cap }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ========== 注册专家对话框 ========== -->
    <RegisterExpertDialog
      v-model="showRegisterDialog"
      @registered="onExpertRegistered"
    />

    <!-- ========== 发起辩论对话框 ========== -->
    <el-dialog
      v-model="showDebateDialog"
      title="发起专家辩论"
      width="520px"
      :close-on-click-modal="!debateSubmitting"
      class="debate-dialog"
    >
      <el-form label-width="88px" label-position="right">
        <el-form-item label="辩题" required>
          <el-input
            v-model="debateConfig.topic"
            type="textarea"
            :rows="2"
            placeholder="请输入辩论主题…"
            maxlength="200"
            show-word-limit
            resize="none"
          />
        </el-form-item>

        <el-form-item label="参与专家" required>
          <div class="debate-expert-picker">
            <div class="debate-expert-list">
              <div
                v-for="exp in experts.filter(e => e.status === 'active')"
                :key="exp.id"
                class="debate-expert-chip"
                :class="{ selected: debateConfig.selectedExpertIds.includes(exp.id) }"
                @click="toggleDebateExpert(exp.id)"
              >
                <span class="chip-avatar" :style="{ background: expertColor(exp.type) }">
                  {{ expertEmoji(exp.type) }}
                </span>
                <span class="chip-name">{{ exp.name }}</span>
                <el-icon v-if="debateConfig.selectedExpertIds.includes(exp.id)" class="chip-check"><CircleCheckFilled /></el-icon>
              </div>
            </div>
            <div class="debate-expert-count">
              已选 <b>{{ debateConfig.selectedExpertIds.length }}</b> 位专家（至少 2 位）
            </div>
          </div>
        </el-form-item>

        <el-form-item label="辩论模式">
          <div class="debate-mode-picker">
            <div
              v-for="opt in debateModeOptions"
              :key="opt.value"
              class="debate-mode-card"
              :class="{ active: debateConfig.mode === opt.value }"
              @click="debateConfig.mode = opt.value"
            >
              <div class="mode-icon">{{ opt.icon }}</div>
              <div class="mode-name">{{ opt.label }}</div>
              <div class="mode-desc">{{ opt.desc }}</div>
            </div>
          </div>
        </el-form-item>

        <el-form-item v-if="debateConfig.mode === 'adversarial'" label="辩论轮次">
          <el-input-number v-model="debateConfig.rounds" :min="1" :max="10" size="small" />
          <span class="form-hint">轮</span>
        </el-form-item>

        <el-form-item label="辩论状态">
          <el-tag :type="debateStatusTagType" effect="light" size="small">
            {{ debateStatusLabel }}
          </el-tag>
        </el-form-item>
      </el-form>

      <template #footer>
        <div class="dialog-footer">
          <el-button @click="showDebateDialog = false" :disabled="debateSubmitting">取消</el-button>
          <el-button
            type="primary"
            :loading="debateSubmitting"
            :disabled="!canStartDebate"
            @click="startDebate"
          >
            <el-icon><Swords /></el-icon>
            <span>开始辩论</span>
          </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- ========== 多专家咨询对话框 ========== -->
    <el-dialog
      v-model="showMultiConsultDialog"
      title="多专家咨询"
      width="560px"
      :close-on-click-modal="!multiConsultSubmitting"
      class="multi-consult-dialog"
    >
      <el-form label-width="88px" label-position="right">
        <el-form-item label="咨询问题" required>
          <el-input
            v-model="multiConsultConfig.question"
            type="textarea"
            :rows="3"
            placeholder="请输入您想咨询的问题…"
            maxlength="500"
            show-word-limit
            resize="none"
          />
        </el-form-item>

        <el-form-item label="选择专家" required>
          <div class="consult-expert-picker">
            <div class="consult-expert-list">
              <div
                v-for="exp in experts.filter(e => e.status === 'active')"
                :key="exp.id"
                class="consult-expert-chip"
                :class="{ selected: multiConsultConfig.selectedExpertIds.includes(exp.id) }"
                @click="toggleMultiConsultExpert(exp.id)"
              >
                <span class="chip-avatar" :style="{ background: expertColor(exp.type) }">
                  {{ expertEmoji(exp.type) }}
                </span>
                <span class="chip-name">{{ exp.name }}</span>
                <el-icon v-if="multiConsultConfig.selectedExpertIds.includes(exp.id)" class="chip-check"><CircleCheckFilled /></el-icon>
              </div>
            </div>
            <div class="consult-expert-count">
              已选 <b>{{ multiConsultConfig.selectedExpertIds.length }}</b> 位专家
            </div>
          </div>
        </el-form-item>

        <el-form-item label="咨询模式">
          <el-radio-group v-model="multiConsultConfig.mode" class="consult-mode-group">
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
            v-model="multiConsultCompareView"
            active-text="对比视图"
            inactive-text="列表视图"
          />
        </el-form-item>
      </el-form>

      <!-- 咨询结果展示 -->
      <div v-if="multiConsultResults.length > 0" class="consult-results-section">
        <div class="results-section-head">
          <span class="results-section-title">
            <el-icon><DocumentCopy /></el-icon>
            咨询结果
          </span>
          <el-tag size="small" type="success" effect="light">
            {{ multiConsultResults.length }} 位专家已回答
          </el-tag>
        </div>

        <!-- 对比视图 -->
        <div v-if="multiConsultCompareView" class="compare-view">
          <div class="compare-grid">
            <div
              v-for="(result, idx) in multiConsultResults"
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
            v-for="(result, idx) in multiConsultResults"
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
          <el-button @click="showMultiConsultDialog = false" :disabled="multiConsultSubmitting">关闭</el-button>
          <el-button
            type="primary"
            :loading="multiConsultSubmitting"
            :disabled="!canStartMultiConsult"
            @click="startMultiConsult"
          >
            <el-icon><Connection /></el-icon>
            <span>开始咨询</span>
          </el-button>
        </div>
      </template>
    </el-dialog>

    <!-- ========== 智能匹配对话框 ========== -->
    <el-dialog
      v-model="showSmartRouteDialog"
      title="智能匹配专家"
      width="500px"
      :close-on-click-modal="!smartRoutingLoading"
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
            v-model="smartRouteQuestion"
            type="textarea"
            :rows="3"
            placeholder="请描述您的问题或需求…"
            maxlength="300"
            show-word-limit
            resize="none"
            @keyup.enter.ctrl="doSmartRoute"
          />
        </el-form-item>

        <el-form-item label="推荐数量">
          <el-input-number v-model="smartRouteMaxExperts" :min="1" :max="6" size="small" />
          <span class="form-hint">位专家</span>
        </el-form-item>
      </el-form>

      <div class="smart-route-action">
        <el-button
          type="primary"
          :loading="smartRoutingLoading"
          :disabled="!smartRouteQuestion.trim()"
          @click="doSmartRoute"
          class="smart-route-btn"
        >
          <el-icon><Compass /></el-icon>
          <span>{{ smartRoutingLoading ? '匹配中…' : '开始智能匹配' }}</span>
        </el-button>
      </div>

      <!-- 匹配结果 -->
      <div v-if="smartRouteResult" class="smart-route-results">
        <div class="route-result-head">
          <span class="route-result-title">匹配结果</span>
          <el-tag size="small" type="success" effect="light">
            {{ smartRouteResult.selected?.length || 0 }} 位推荐
          </el-tag>
        </div>

        <div class="route-expert-list">
          <div
            v-for="(item, idx) in smartRouteResult.selected || []"
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
              @click="selectRoutedExpert(item)"
            >选择</el-button>
          </div>
        </div>

        <div class="route-actions-footer">
          <el-button size="small" @click="selectAllRoutedExperts">
            <el-icon><CircleCheckFilled /></el-icon>
            一键选择全部推荐
          </el-button>
        </div>
      </div>

      <div v-else-if="smartRoutingLoading" class="smart-route-loading">
        <el-icon class="is-loading loading-spinner"><Loading /></el-icon>
        <span>正在分析您的问题并匹配专家…</span>
      </div>
    </el-dialog>

    <!-- 全局通知 -->
    <el-notification v-for="notif in notifications" :key="notif.id"
      :title="notif.title"
      :message="notif.message"
      :type="notif.type || 'info'"
      :duration="3000"
      @close="removeNotification(notif.id)"
    />
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted, onBeforeUnmount, nextTick, watch } from 'vue'
import { ElMessage, ElNotification } from 'element-plus'
import {
  Search, MagicStick, Bell, ArrowLeft, ArrowRight, Plus,
  ZoomIn, ZoomOut, FullScreen, DataAnalysis, Close,
  Document, ChatDotRound, ChatLineSquare, ArrowDown, ArrowUp,
  Folder, FolderOpened, Upload, Edit, CircleCheckFilled,
  Share, Link, Paperclip, Promotion, Loading, RefreshRight,
  UserFilled, SetUp, Pointer, Rank, Delete, CollectionTag,
  Swords, Connection, Compass, DocumentCopy, Warning, Download,
  Top, Bottom
} from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'
import { runAllianceFullSSE, getAllianceCapabilities } from '@/api/alliance'
import {
  getExperts, getExpertGraph, listExpertSessions,
  expertDebate, expertOrchestrate, multiExpertConsult,
  routeExperts, registerExpert
} from '@/api/experts.api.js'
import RegisterExpertDialog from '@/components/expert/RegisterExpertDialog.vue'
import {
  kbListDocuments, kbGetCategories, kbGetTags,
  kbSearch, kbGetVersions, kbGetStats
} from '@/api/kb.api.js'

// ========== 布局状态 ==========
const leftCollapsed = ref(false)
const rightCollapsed = ref(false)
const collabExpanded = ref(true)
const aiAssistantOpen = ref(false)
const hasNotifications = ref(true)
const notifCount = ref(3)
const historyPanelOpen = ref(false)

// ========== KPI 指标卡 ==========
const kpiCards = ref([
  { key: 'experts', icon: '👥', value: 12, label: '在线专家', trend: 8, gradient: 'linear-gradient(135deg, #7c3aed, #06b6d4)' },
  { key: 'sessions', icon: '💬', value: 28, label: '协作会话', trend: 15, gradient: 'linear-gradient(135deg, #ec4899, #8b5cf6)' },
  { key: 'docs', icon: '📄', value: 156, label: '知识文档', trend: 5, gradient: 'linear-gradient(135deg, #10b981, #14b8a6)' },
  { key: 'tasks', icon: '🎯', value: 7, label: '进行中任务', trend: -2, gradient: 'linear-gradient(135deg, #f59e0b, #ef4444)' }
])

function onKpiClick(key) {
  if (key === 'experts') {
    leftCollapsed.value = false
  } else if (key === 'docs') {
    rightCollapsed.value = false
    activeKbTab.value = 'docs'
  } else if (key === 'sessions') {
    collabExpanded.value = true
    activeCollabTab.value = 'discussion'
  } else if (key === 'tasks') {
    activeMode.value = 'orchestration'
  }
}

// ========== 工作模式 ==========
const savedMode = localStorage.getItem('expert_workspace_mode')
const activeMode = ref(savedMode || 'collaboration')
const modeTransitioning = ref(false)
const workModes = [
  { key: 'exploration', label: '知识探索', iconComp: 'Search', gradient: 'linear-gradient(135deg, #06b6d4, #3b82f6)' },
  { key: 'collaboration', label: '专家协作', iconComp: 'UserFilled', gradient: 'linear-gradient(135deg, #7c3aed, #06b6d4)' },
  { key: 'orchestration', label: '任务编排', iconComp: 'SetUp', gradient: 'linear-gradient(135deg, #f59e0b, #ef4444)' },
  { key: 'analysis', label: '深度分析', iconComp: 'DataAnalysis', gradient: 'linear-gradient(135deg, #10b981, #14b8a6)' }
]

function switchWorkMode(mode) {
  if (activeMode.value === mode) return
  modeTransitioning.value = true
  activeMode.value = mode
  // 保存模式记忆
  localStorage.setItem('expert_workspace_mode', mode)
  // 切换模式时调整面板状态
  if (mode === 'exploration') {
    leftCollapsed.value = true
    rightCollapsed.value = false
  } else if (mode === 'collaboration') {
    leftCollapsed.value = false
    collabExpanded.value = true
  } else if (mode === 'orchestration') {
    leftCollapsed.value = false
    rightCollapsed.value = false
  } else if (mode === 'analysis') {
    leftCollapsed.value = true
    rightCollapsed.value = true
  }
  setTimeout(() => {
    modeTransitioning.value = false
  }, 400)
  addHistoryEvent('mode', `切换到${workModes.find(m => m.key === mode)?.label || ''}模式`, '工作模式已切换')
}

// ========== 项目 ==========
const currentProject = ref('xuanji')
const globalSearch = ref('')

function onProjectChange() {
  loadExperts()
  loadSessions()
  loadGraphData()
  loadDocuments()
}

function doGlobalSearch() {
  if (!globalSearch.value.trim()) return
  // 同时搜索专家、文档、图谱节点
  expertSearch.value = globalSearch.value
  kbSearchQuery.value = globalSearch.value
  ElMessage.info(`正在全局搜索「${globalSearch.value}」…`)
}

// ========== 专家数据 ==========
const experts = ref([])
const expertsLoading = ref(false)
const expertFilterType = ref('')
const expertSearch = ref('')
const activeExpert = ref(null)
const selectedExpertIds = ref([])
const notifications = ref([])

// ========== 对话框状态 ==========
const showRegisterDialog = ref(false)
const showDebateDialog = ref(false)
const showMultiConsultDialog = ref(false)
const showSmartRouteDialog = ref(false)
const debateSubmitting = ref(false)
const multiConsultSubmitting = ref(false)
const smartRoutingLoading = ref(false)

// ========== 辩论配置 ==========
const debateConfig = reactive({
  topic: '',
  selectedExpertIds: [],
  mode: 'adversarial', // adversarial 对抗式 / roundtable 圆桌式
  rounds: 3
})
const debateStatus = ref('preparing') // preparing / ongoing / summarized
const debateMessages = ref([])
const debateSummary = ref('')
const debateModeOptions = [
  { value: 'adversarial', label: '对抗式辩论', icon: '⚔️', desc: '专家分正反两方，针锋相对' },
  { value: 'roundtable', label: '圆桌式讨论', icon: '圆桌', desc: '多位专家平等交流，各抒己见' }
]

// ========== 多专家咨询配置 ==========
const multiConsultConfig = reactive({
  question: '',
  selectedExpertIds: [],
  mode: 'parallel' // parallel 并行 / serial 串行
})
const multiConsultResults = ref([])
const multiConsultCompareView = ref(false)

// ========== 智能路由匹配 ==========
const smartRouteQuestion = ref('')
const smartRouteResult = ref(null)
const smartRouteMaxExperts = ref(3)

const onlineExpertCount = computed(() =>
  experts.value.filter(e => e.status === 'active').length
)

const filteredExperts = computed(() => {
  let list = experts.value
  if (expertFilterType.value) {
    list = list.filter(e => e.type === expertFilterType.value)
  }
  if (expertSearch.value) {
    const kw = expertSearch.value.toLowerCase()
    list = list.filter(e =>
      (e.name || '').toLowerCase().includes(kw) ||
      (e.type || '').toLowerCase().includes(kw) ||
      (e.capabilities || []).some(c => (c || '').toLowerCase().includes(kw))
    )
  }
  return list
})

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

function expertStatusClass(status) {
  return `status-${status || 'active'}`
}

function isExpertSelected(id) {
  return selectedExpertIds.value.includes(id)
}

function selectExpert(expert) {
  activeExpert.value = expert
}

function handleExpertClick(expert) {
  selectExpert(expert)
  // 点击同时切换到选中状态（多选）
  const idx = selectedExpertIds.value.indexOf(expert.id)
  if (idx >= 0) {
    selectedExpertIds.value.splice(idx, 1)
  } else {
    selectedExpertIds.value.push(expert.id)
  }
}

async function loadExperts() {
  expertsLoading.value = true
  try {
    const res = await getExperts({ project_id: currentProject.value, status: 'active' })
    if (res && Array.isArray(res.data)) {
      experts.value = res.data
    } else if (res && Array.isArray(res)) {
      experts.value = res
    } else {
      // 兜底模拟数据
      experts.value = getMockExperts()
    }
  } catch (e) {
    console.warn('[workspace] 加载专家列表失败，使用模拟数据:', e)
    experts.value = getMockExperts()
  } finally {
    expertsLoading.value = false
  }
}

function getMockExperts() {
  return [
    { id: 'exp-001', name: '林算法', type: 'algorithm', status: 'active',
      capabilities: ['动态规划', '图算法', '复杂度分析'],
      metrics: { total_consults: 1286, success_rate: 0.97 } },
    { id: 'exp-002', name: '陈架构', type: 'architecture', status: 'active',
      capabilities: ['微服务', 'DDD', '高可用设计'],
      metrics: { total_consults: 2103, success_rate: 0.95 } },
    { id: 'exp-003', name: '王数据', type: 'data', status: 'active',
      capabilities: ['数据建模', 'ETL', '数据治理'],
      metrics: { total_consults: 856, success_rate: 0.98 } },
    { id: 'exp-004', name: '张AI', type: 'ai', status: 'active',
      capabilities: ['LLM', 'RAG', 'Prompt工程'],
      metrics: { total_consults: 3241, success_rate: 0.94 } },
    { id: 'exp-005', name: '李工作流', type: 'workflow', status: 'busy',
      capabilities: ['流程编排', 'BPM', '自动化'],
      metrics: { total_consults: 678, success_rate: 0.96 } },
    { id: 'exp-006', name: '赵图谱', type: 'graph', status: 'active',
      capabilities: ['图数据库', 'Cypher', '图计算'],
      metrics: { total_consults: 945, success_rate: 0.93 } },
    { id: 'exp-007', name: '孙安全', type: 'security', status: 'active',
      capabilities: ['渗透测试', '安全审计', '合规'],
      metrics: { total_consults: 523, success_rate: 0.99 } },
    { id: 'exp-008', name: '周性能', type: 'performance', status: 'idle',
      capabilities: ['性能调优', '压测', '缓存策略'],
      metrics: { total_consults: 712, success_rate: 0.92 } }
  ]
}

// ========== 协作会话 ==========
const sessions = ref([])
const sessionsLoading = ref(false)
const activeSession = ref(null)

async function loadSessions() {
  sessionsLoading.value = true
  try {
    const res = await listExpertSessions({ project_id: currentProject.value, limit: 20 })
    if (res && Array.isArray(res.data)) {
      sessions.value = res.data
    } else if (res && Array.isArray(res)) {
      sessions.value = res
    } else {
      sessions.value = getMockSessions()
    }
  } catch (e) {
    console.warn('[workspace] 加载会话失败，使用模拟数据:', e)
    sessions.value = getMockSessions()
  } finally {
    sessionsLoading.value = false
  }
}

function getMockSessions() {
  return [
    { id: 'sess-001', title: '架构优化方案讨论', expert_count: 3, mode: 'debate',
      created_at: Date.now() - 600000, updated_at: Date.now() - 600000 },
    { id: 'sess-002', title: '知识图谱融合策略', expert_count: 2, mode: 'multi',
      created_at: Date.now() - 3600000, updated_at: Date.now() - 3600000 },
    { id: 'sess-003', title: '性能瓶颈分析', expert_count: 2, mode: 'single',
      created_at: Date.now() - 86400000, updated_at: Date.now() - 86400000 }
  ]
}

function selectSession(session) {
  activeSession.value = session
  // 加载会话消息（此处简化，实际应调用 API）
  collabMessages.value = [
    {
      id: Date.now(),
      role: 'system',
      name: '系统',
      avatar: '📢',
      color: '#64748b',
      time: formatTime(session.updated_at),
      text: `已进入「${session.title}」协作会话`
    }
  ]
}

function newCollaboration() {
  activeMode.value = 'collaboration'
  collabExpanded.value = true
  const newSess = {
    id: 'sess-' + Date.now(),
    title: '新协作会话',
    expert_count: selectedExpertIds.value.length || 0,
    mode: collabMode.value,
    created_at: Date.now(),
    updated_at: Date.now()
  }
  sessions.value.unshift(newSess)
  selectSession(newSess)
  ElMessage.success('已创建新的协作会话')
}

function sessionModeLabel(mode) {
  const map = { smart: '智能路由', single: '单专家', multi: '多专家', debate: '辩论', algorithm: '算法分析' }
  return map[mode] || '协作'
}

function sessionModeType(mode) {
  const map = { smart: 'info', single: 'primary', multi: 'success', debate: 'warning', algorithm: 'danger' }
  return map[mode] || 'info'
}

// ========== 协作 Tab 配置 ==========
const activeCollabTab = ref('discussion')
const collabTabs = computed(() => [
  { key: 'discussion', label: '讨论', icon: 'ChatLineSquare', badge: collabMessages.value.length },
  { key: 'whiteboard', label: '白板', icon: 'CollectionTag', badge: wbNotes.value.length + wbTexts.value.length || null },
  { key: 'files', label: '文件', icon: 'FolderOpened', badge: sharedFiles.value.length || null }
])

// ========== 协作成员 ==========
const collabMembers = ref([
  { id: 'user-1', name: '我', avatar: 'U', color: 'linear-gradient(135deg, #7c3aed, #06b6d4)', status: 'active', role: 'host' },
  { id: 'exp-002', name: '陈架构', avatar: '🏗️', color: 'linear-gradient(135deg, #6366f1, #06b6d4)', status: 'active', role: 'expert' },
  { id: 'exp-004', name: '张AI', avatar: '🤖', color: 'linear-gradient(135deg, #ec4899, #8b5cf6)', status: 'active', role: 'expert' },
  { id: 'exp-006', name: '赵图谱', avatar: '🕸️', color: 'linear-gradient(135deg, #06b6d4, #3b82f6)', status: 'busy', role: 'expert' },
  { id: 'exp-001', name: '林算法', avatar: '🧮', color: 'linear-gradient(135deg, #6366f1, #8b5cf6)', status: 'active', role: 'expert' }
])

function memberStatusText(status) {
  const map = { active: '在线', busy: '忙碌', offline: '离线', idle: '空闲' }
  return map[status] || '在线'
}

// ========== 正在输入的专家 ==========
const typingExperts = ref([])

// ========== 消息状态 ==========
function msgStatusText(status) {
  const map = { sent: '已发送', thinking: '正在思考', done: '已完成', failed: '失败' }
  return map[status] || ''
}

// ========== 项目阶段进度 ==========
const projectPhases = ref([
  { key: 'requirement', label: '需求分析' },
  { key: 'architecture', label: '架构设计' },
  { key: 'development', label: '开发实现' },
  { key: 'testing', label: '测试验证' },
  { key: 'release', label: '发布上线' }
])
const currentProjectPhase = ref(1)

function jumpToPhase(idx) {
  currentProjectPhase.value = idx
  const phase = projectPhases.value[idx]
  addHistoryEvent('phase', `进入「${phase.label}」阶段`, '项目阶段已切换')
  ElMessage.info(`已切换到「${phase.label}」阶段`)
}

// ========== 共享文件 ==========
const sharedFiles = ref([
  { id: 'f-001', name: '架构设计文档.pdf', type: 'pdf', size: '2.4 MB', uploader: '陈架构', time: '10:30' },
  { id: 'f-002', name: '需求规格说明书.docx', type: 'doc', size: '1.8 MB', uploader: '我', time: '09:15' },
  { id: 'f-003', name: '系统架构图.png', type: 'image', size: '856 KB', uploader: '张AI', time: '昨天' },
  { id: 'f-004', name: '接口定义.xlsx', type: 'excel', size: '342 KB', uploader: '赵图谱', time: '昨天' }
])
const filesBarExpanded = ref(true)
const dragOver = ref(false)
const fileDragOver = ref(false)

function fileIconEmoji(type) {
  const icons = { pdf: '📕', doc: '📘', image: '🖼️', excel: '📗', ppt: '📙', zip: '📦', code: '💻', other: '📄' }
  return icons[type] || '📄'
}

function handleBeforeFileUpload(file) {
  const type = getFileType(file.name)
  const newFile = {
    id: 'f-' + Date.now(),
    name: file.name,
    type: type,
    size: formatFileSize(file.size),
    uploader: '我',
    time: '刚刚'
  }
  sharedFiles.value.unshift(newFile)
  addHistoryEvent('file', `上传文件「${file.name}」`, '文件已共享到协作区')
  ElMessage.success(`文件「${file.name}」上传成功`)
  return false // 阻止自动上传，使用自定义逻辑
}

function handleFileDrop(e) {
  dragOver.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    Array.from(files).forEach(file => {
      handleBeforeFileUpload(file)
    })
  }
}

function handleFileDropToFiles(e) {
  fileDragOver.value = false
  const files = e.dataTransfer?.files
  if (files && files.length > 0) {
    Array.from(files).forEach(file => {
      handleBeforeFileUpload(file)
    })
  }
}

function getFileType(filename) {
  const ext = filename.split('.').pop()?.toLowerCase()
  if (['pdf'].includes(ext)) return 'pdf'
  if (['doc', 'docx', 'txt', 'md'].includes(ext)) return 'doc'
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'].includes(ext)) return 'image'
  if (['xls', 'xlsx', 'csv'].includes(ext)) return 'excel'
  if (['ppt', 'pptx'].includes(ext)) return 'ppt'
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'zip'
  if (['js', 'ts', 'py', 'java', 'go', 'cpp', 'html', 'css', 'vue', 'json'].includes(ext)) return 'code'
  return 'other'
}

function previewFile(file) {
  if (file.type === 'image') {
    ElMessage.info(`正在预览图片：${file.name}`)
  } else {
    ElMessage.info(`正在打开文档：${file.name}`)
  }
}

function downloadFile(file) {
  ElMessage.success(`开始下载：${file.name}`)
}

function sendToWhiteboard() {
  if (collabInput.value.trim()) {
    addWbNote(collabInput.value.substring(0, 20), collabInput.value)
    ElMessage.success('已添加到白板')
  } else {
    ElMessage.warning('请先输入内容')
  }
}

// ========== 白板功能 ==========
const whiteboardRef = ref(null)
const activeWbTool = ref('select')
const activeWbColor = ref('#7c3aed')
const wbNotes = ref([])
const wbTexts = ref([])
const wbLines = ref([])
const wbDrawPaths = ref([])
const wbCurrentPath = ref('')
const wbViewBox = ref('0 0 800 400')
let wbDrawing = false
let wbDragElement = null
let wbDragOffset = { x: 0, y: 0 }
let wbPathPoints = []

const whiteboardTools = [
  { key: 'select', label: '选择', icon: '👆' },
  { key: 'note', label: '便签', icon: '📝' },
  { key: 'line', label: '连线', icon: '➖' },
  { key: 'pen', label: '画笔', icon: '🖌️' },
  { key: 'text', label: '文本', icon: '🔤' },
  { key: 'eraser', label: '橡皮擦', icon: '🧹' }
]

const wbColors = ['#7c3aed', '#06b6d4', '#10b981', '#f59e0b', '#ef4444', '#ec4899', '#64748b']

function selectWbTool(tool) {
  activeWbTool.value = tool
}

function onWbMouseDown(e) {
  const rect = whiteboardRef.value?.getBoundingClientRect()
  if (!rect) return
  const x = e.clientX - rect.left
  const y = e.clientY - rect.top

  if (activeWbTool.value === 'note') {
    addWbNote('新便签', '', x, y)
    activeWbTool.value = 'select'
  } else if (activeWbTool.value === 'text') {
    addWbText('新文本', x, y)
    activeWbTool.value = 'select'
  } else if (activeWbTool.value === 'pen') {
    wbDrawing = true
    wbPathPoints = [{ x, y }]
    wbCurrentPath.value = `M ${x} ${y}`
  } else if (activeWbTool.value === 'line') {
    wbDrawing = true
    wbPathPoints = [{ x, y }]
  }
}

function onWbMouseMove(e) {
  const rect = whiteboardRef.value?.getBoundingClientRect()
  if (!rect) return
  const x = e.clientX - rect.left
  const y = e.clientY - rect.top

  if (wbDragElement) {
    wbDragElement.x = x - wbDragOffset.x
    wbDragElement.y = y - wbDragOffset.y
  }

  if (wbDrawing && activeWbTool.value === 'pen' && wbPathPoints.length > 0) {
    wbPathPoints.push({ x, y })
    wbCurrentPath.value = wbPathPoints.map((p, i) =>
      i === 0 ? `M ${p.x} ${p.y}` : `L ${p.x} ${p.y}`
    ).join(' ')
  }
}

function onWbMouseUp(e) {
  if (wbDrawing && activeWbTool.value === 'pen' && wbPathPoints.length > 1) {
    wbDrawPaths.value.push({
      d: wbCurrentPath.value,
      color: activeWbColor.value
    })
    addHistoryEvent('whiteboard', '添加画笔路径', '白板内容已更新')
  }
  wbDrawing = false
  wbDragElement = null
  wbCurrentPath.value = ''
  wbPathPoints = []
}

function addWbNote(title, content = '', x = 100, y = 80) {
  const note = {
    id: 'note-' + Date.now(),
    title: title,
    content: content,
    x: x,
    y: y,
    color: activeWbColor.value + '20'
  }
  wbNotes.value.push(note)
  addHistoryEvent('whiteboard', `添加便签「${title}」`, '白板内容已更新')
}

function startDragNote(e, note) {
  if (activeWbTool.value === 'eraser') {
    deleteWbNote(note.id)
    return
  }
  if (activeWbTool.value !== 'select') return
  const rect = whiteboardRef.value?.getBoundingClientRect()
  if (!rect) return
  wbDragOffset.x = e.clientX - rect.left - note.x
  wbDragOffset.y = e.clientY - rect.top - note.y
  wbDragElement = note
}

function deleteWbNote(id) {
  const idx = wbNotes.value.findIndex(n => n.id === id)
  if (idx >= 0) {
    wbNotes.value.splice(idx, 1)
    addHistoryEvent('whiteboard', '删除便签', '白板内容已更新')
  }
}

function updateNoteContent(e, note) {
  note.content = e.target.innerText
}

function addWbText(content, x = 100, y = 100) {
  const text = {
    id: 'text-' + Date.now(),
    content: content,
    x: x,
    y: y,
    color: activeWbColor.value
  }
  wbTexts.value.push(text)
  addHistoryEvent('whiteboard', `添加文本框`, '白板内容已更新')
}

function startDragText(e, text) {
  if (activeWbTool.value === 'eraser') {
    deleteWbText(text.id)
    return
  }
  if (activeWbTool.value !== 'select') return
  const rect = whiteboardRef.value?.getBoundingClientRect()
  if (!rect) return
  wbDragOffset.x = e.clientX - rect.left - text.x
  wbDragOffset.y = e.clientY - rect.top - text.y
  wbDragElement = text
}

function deleteWbText(id) {
  const idx = wbTexts.value.findIndex(t => t.id === id)
  if (idx >= 0) {
    wbTexts.value.splice(idx, 1)
    addHistoryEvent('whiteboard', '删除文本框', '白板内容已更新')
  }
}

function updateTextContent(e, text) {
  text.content = e.target.innerText
}

function clearWhiteboard() {
  wbNotes.value = []
  wbTexts.value = []
  wbLines.value = []
  wbDrawPaths.value = []
  addHistoryEvent('whiteboard', '清空画布', '白板已清空')
  ElMessage.success('白板已清空')
}

function saveWhiteboard() {
  const data = {
    notes: wbNotes.value,
    texts: wbTexts.value,
    lines: wbLines.value,
    drawPaths: wbDrawPaths.value
  }
  if (activeSession.value) {
    localStorage.setItem('wb_' + activeSession.value.id, JSON.stringify(data))
  }
  addHistoryEvent('whiteboard', '保存白板内容', '白板已保存')
  ElMessage.success('白板内容已保存')
}

// ========== 历史记录 ==========
const historyEvents = ref([
  { id: 'h-001', type: 'message', title: '陈架构 发送了消息', description: '关于微服务架构的建议...', time: '10:45' },
  { id: 'h-002', type: 'file', title: '上传文件', description: '架构设计文档.pdf', time: '10:30' },
  { id: 'h-003', type: 'phase', title: '进入架构设计阶段', description: '项目阶段已切换', time: '10:00' },
  { id: 'h-004', type: 'whiteboard', title: '添加便签', description: '核心架构思路', time: '09:45' },
  { id: 'h-005', type: 'mode', title: '切换到专家协作模式', description: '工作模式已切换', time: '09:30' }
])

function addHistoryEvent(type, title, description) {
  const now = new Date()
  const time = now.getHours().toString().padStart(2, '0') + ':' + now.getMinutes().toString().padStart(2, '0')
  historyEvents.value.unshift({
    id: 'h-' + Date.now(),
    type: type,
    title: title,
    description: description,
    time: time
  })
  // 最多保留 50 条
  if (historyEvents.value.length > 50) {
    historyEvents.value = historyEvents.value.slice(0, 50)
  }
}

function historyIcon(type) {
  const icons = {
    message: '💬', file: '📎', phase: '📊',
    whiteboard: '🎨', mode: '🔄', member: '👥', task: '🎯'
  }
  return icons[type] || '📌'
}

function jumpToHistory(item) {
  if (item.type === 'phase') {
    // 跳转到对应阶段
  } else if (item.type === 'file') {
    activeCollabTab.value = 'files'
  } else if (item.type === 'whiteboard') {
    activeCollabTab.value = 'whiteboard'
  }
  ElMessage.info(`跳转到：${item.title}`)
}

// ========== 快捷键 ==========
function handleKeydown(e) {
  // Ctrl+1~4 切换工作模式
  if (e.ctrlKey && ['1', '2', '3', '4'].includes(e.key)) {
    e.preventDefault()
    const idx = parseInt(e.key) - 1
    if (workModes[idx]) {
      switchWorkMode(workModes[idx].key)
    }
  }
  // Ctrl+K 全局搜索
  if (e.ctrlKey && e.key === 'k') {
    e.preventDefault()
    doGlobalSearch()
  }
}

// ========== 注册专家回调 ==========
function onExpertRegistered(expertData) {
  ElMessage.success(`专家「${expertData.name || '新专家'}」注册成功`)
  // 添加到专家列表
  if (expertData && !experts.value.find(e => e.id === expertData.id)) {
    experts.value.unshift({
      id: expertData.id,
      name: expertData.name,
      type: expertData.type,
      status: 'active',
      capabilities: expertData.capabilities || [],
      metrics: expertData.metrics || { total_consults: 0, success_rate: 0.95 }
    })
  }
  loadExperts()
}

// ========== 辩论相关 ==========
const canStartDebate = computed(() =>
  debateConfig.topic.trim() && debateConfig.selectedExpertIds.length >= 2
)

const debateStatusLabel = computed(() => {
  const map = { preparing: '准备中', ongoing: '进行中', summarized: '已总结' }
  return map[debateStatus.value] || '准备中'
})

const debateStatusTagType = computed(() => {
  const map = { preparing: 'info', ongoing: 'warning', summarized: 'success' }
  return map[debateStatus.value] || 'info'
})

function openDebateDialog() {
  debateConfig.topic = ''
  debateConfig.selectedExpertIds = [...selectedExpertIds.value]
  debateConfig.mode = 'adversarial'
  debateConfig.rounds = 3
  debateStatus.value = 'preparing'
  debateMessages.value = []
  debateSummary.value = ''
  showDebateDialog.value = true
}

function toggleDebateExpert(id) {
  const idx = debateConfig.selectedExpertIds.indexOf(id)
  if (idx >= 0) {
    debateConfig.selectedExpertIds.splice(idx, 1)
  } else {
    debateConfig.selectedExpertIds.push(id)
  }
}

async function startDebate() {
  if (!canStartDebate.value) return

  debateSubmitting.value = true
  debateStatus.value = 'ongoing'
  debateMessages.value = []
  debateSummary.value = ''

  try {
    const result = await expertDebate({
      question: debateConfig.topic,
      expert_ids: debateConfig.selectedExpertIds,
      rounds: debateConfig.rounds,
      mode: debateConfig.mode
    })

    // 处理辩论结果
    const history = result?.history || result?.data?.history || []
    history.forEach((round, roundIdx) => {
      const results = round.results || []
      results.forEach(r => {
        if (r.success) {
          debateMessages.value.push({
            id: Date.now() + roundIdx * 100 + Math.random(),
            expert: r.expert,
            response: r.response,
            round: roundIdx + 1,
            confidence: r.confidence
          })
        }
      })
    })
    debateSummary.value = result?.final_synthesis || result?.data?.final_synthesis || ''
    debateStatus.value = 'summarized'

    // 同步到协作消息区
    appendDebateToCollab()

    ElMessage.success(`辩论完成，共 ${debateConfig.rounds} 轮`)
  } catch (e) {
    console.warn('[debate] 辩论 API 调用失败，使用模拟数据:', e)
    await simulateDebate()
  } finally {
    debateSubmitting.value = false
  }
}

async function simulateDebate() {
  const selectedExperts = experts.value.filter(e => debateConfig.selectedExpertIds.includes(e.id))
  if (selectedExperts.length < 2) {
    ElMessage.error('请至少选择 2 位专家')
    debateStatus.value = 'preparing'
    return
  }

  debateMessages.value = []
  for (let round = 1; round <= debateConfig.rounds; round++) {
    for (const exp of selectedExperts) {
      await new Promise(r => setTimeout(r, 300 + Math.random() * 400))
      debateMessages.value.push({
        id: Date.now() + Math.random(),
        expert: { id: exp.id, name: exp.name, type: exp.type },
        response: `【第${round}轮 · ${exp.name}】从${EXPERT_TYPES[exp.type] || '专业'}角度来看，「${debateConfig.topic.slice(0, 20)}」这个问题的核心在于${round === 1 ? '明确定义和边界' : round === 2 ? '深入分析技术方案的优劣' : '综合评估可行性和风险'}。我认为应该采用${['渐进式迭代', '模块化设计', '数据驱动决策'][round % 3]}的方法来解决。`,
        round,
        confidence: 0.85 + Math.random() * 0.12
      })
    }
  }

  debateSummary.value = `## 辩论总结\n\n经过 ${debateConfig.rounds} 轮激烈讨论，${selectedExperts.map(e => e.name).join('、')} 等专家从不同角度对「${debateConfig.topic}」进行了深入分析。\n\n### 核心共识\n- 问题具有多维度复杂性，需要跨领域协作\n- 建议采用分阶段实施策略，降低风险\n- 数据驱动决策是关键成功因素\n\n### 分歧点\n- 技术路线选择上各有侧重\n- 实施优先级排序存在差异\n\n### 建议方案\n综合各方观点，建议采用「${debateConfig.mode === 'adversarial' ? '混合架构' : '协同推进'}」策略，充分发挥各领域专家优势，分阶段落地实施。`

  debateStatus.value = 'summarized'
  appendDebateToCollab()
  ElMessage.warning('辩论服务暂不可用，已生成模拟辩论结果')
}

function appendDebateToCollab() {
  if (!activeSession.value) {
    const newSess = {
      id: 'sess-' + Date.now(),
      title: debateConfig.topic.slice(0, 20) + '…',
      expert_count: debateConfig.selectedExpertIds.length,
      mode: 'debate',
      created_at: Date.now(),
      updated_at: Date.now()
    }
    sessions.value.unshift(newSess)
    selectSession(newSess)
  }

  collabMessages.value.push({
    id: Date.now(),
    role: 'system',
    name: '辩论系统',
    avatar: '⚔️',
    color: '#ef4444',
    time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
    text: `【辩论开始】主题：${debateConfig.topic}`
  })

  debateMessages.value.forEach(msg => {
    collabMessages.value.push({
      id: Date.now() + Math.random(),
      role: 'expert',
      name: msg.expert?.name || '专家',
      avatar: expertEmoji(msg.expert?.type),
      color: expertColor(msg.expert?.type),
      phase: 'debate',
      time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
      text: msg.response
    })
  })

  if (debateSummary.value) {
    collabMessages.value.push({
      id: Date.now() + 999,
      role: 'assistant',
      name: '辩论总结',
      avatar: '📝',
      color: '#10b981',
      phase: 'synthesize',
      time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
      text: debateSummary.value
    })
  }

  scrollMessagesToBottom()
}

// ========== 多专家咨询 ==========
const canStartMultiConsult = computed(() =>
  multiConsultConfig.question.trim() && multiConsultConfig.selectedExpertIds.length >= 1
)

function openMultiConsultDialog() {
  multiConsultConfig.question = ''
  multiConsultConfig.selectedExpertIds = [...selectedExpertIds.value]
  multiConsultConfig.mode = 'parallel'
  multiConsultResults.value = []
  multiConsultCompareView.value = false
  showMultiConsultDialog.value = true
}

function toggleMultiConsultExpert(id) {
  const idx = multiConsultConfig.selectedExpertIds.indexOf(id)
  if (idx >= 0) {
    multiConsultConfig.selectedExpertIds.splice(idx, 1)
  } else {
    multiConsultConfig.selectedExpertIds.push(id)
  }
}

async function startMultiConsult() {
  if (!canStartMultiConsult.value) return

  multiConsultSubmitting.value = true
  multiConsultResults.value = []

  try {
    const result = await multiExpertConsult({
      question: multiConsultConfig.question,
      expert_ids: multiConsultConfig.selectedExpertIds,
      mode: multiConsultConfig.mode
    })

    const results = result?.results || result?.data?.results || []
    multiConsultResults.value = results
      .filter(r => r.success)
      .map(r => ({
        expert: r.expert,
        response: r.response,
        confidence: r.confidence,
        duration_ms: r.duration_ms
      }))

    ElMessage.success(`咨询完成，共 ${multiConsultResults.value.length} 位专家参与`)
  } catch (e) {
    console.warn('[multiConsult] 多专家咨询 API 失败，使用模拟数据:', e)
    await simulateMultiConsult()
  } finally {
    multiConsultSubmitting.value = false
  }
}

async function simulateMultiConsult() {
  const selectedExperts = experts.value.filter(e => multiConsultConfig.selectedExpertIds.includes(e.id))
  multiConsultResults.value = []

  if (multiConsultConfig.mode === 'parallel') {
    // 并行：同时返回
    await new Promise(r => setTimeout(r, 1000))
    selectedExperts.forEach((exp, idx) => {
      multiConsultResults.value.push({
        expert: { id: exp.id, name: exp.name, type: exp.type },
        response: `【${exp.name}的回答】关于「${multiConsultConfig.question.slice(0, 20)}」的问题，从${EXPERT_TYPES[exp.type] || '专业'}角度分析：\n\n1. 核心要点：问题涉及多个层面，需要系统思考\n2. 建议方案：采用${['分治法', '迭代法', '模块化'][idx % 3]}策略逐步解决\n3. 注意事项：需要关注边界条件和异常处理\n\n以上是我的初步分析，供参考。`,
        confidence: 0.8 + Math.random() * 0.18,
        duration_ms: 800 + Math.random() * 1200
      })
    })
  } else {
    // 串行：依次返回
    for (const exp of selectedExperts) {
      await new Promise(r => setTimeout(r, 600 + Math.random() * 600))
      multiConsultResults.value.push({
        expert: { id: exp.id, name: exp.name, type: exp.type },
        response: `【${exp.name}的回答】针对「${multiConsultConfig.question.slice(0, 20)}」这个问题，我的分析如下：\n\n首先，明确问题的核心目标和约束条件。其次，基于${EXPERT_TYPES[exp.type] || '专业领域'}的知识，推荐以下方案：\n- 方案A：保守稳妥，风险低\n- 方案B：激进高效，收益高\n- 方案C：折中平衡，适用性广\n\n建议根据实际情况选择合适的方案。`,
        confidence: 0.78 + Math.random() * 0.2,
        duration_ms: 600 + Math.random() * 800
      })
    }
  }

  ElMessage.warning('咨询服务暂不可用，已生成模拟回答')
}

// ========== 智能路由匹配 ==========
function openSmartRouteDialog() {
  smartRouteQuestion.value = ''
  smartRouteResult.value = null
  smartRouteMaxExperts.value = 3
  showSmartRouteDialog.value = true
}

async function doSmartRoute() {
  if (!smartRouteQuestion.value.trim()) return

  smartRoutingLoading.value = true
  smartRouteResult.value = null

  try {
    const result = await routeExperts({
      question: smartRouteQuestion.value,
      maxExperts: smartRouteMaxExperts.value
    })

    smartRouteResult.value = result?.data || result
    ElMessage.success('智能匹配完成')
  } catch (e) {
    console.warn('[routeExperts] 智能路由 API 失败，使用模拟数据:', e)
    simulateSmartRoute()
  } finally {
    smartRoutingLoading.value = false
  }
}

function simulateSmartRoute() {
  const question = smartRouteQuestion.value.toLowerCase()
  const scoredExperts = experts.value
    .filter(e => e.status === 'active')
    .map(e => {
      let baseScore = 0.5 + Math.random() * 0.3
      // 根据问题关键词简单匹配
      const caps = (e.capabilities || []).join('').toLowerCase()
      const typeMatch = question.includes(e.type) ? 0.15 : 0
      const capMatch = e.capabilities?.some(c => question.includes(c.toLowerCase())) ? 0.1 : 0
      return {
        ...e,
        score: Math.min(0.98, baseScore + typeMatch + capMatch),
        reason: `基于「${EXPERT_TYPES[e.type]}」领域专长和${e.capabilities?.[0] || '相关'}技能匹配`
      }
    })
    .sort((a, b) => b.score - a.score)
    .slice(0, smartRouteMaxExperts.value)

  smartRouteResult.value = {
    selected: scoredExperts,
    question: smartRouteQuestion.value,
    mode: 'auto',
    reasoning: `根据问题描述中的关键词和领域特征，从 ${experts.value.length} 位专家中筛选出最佳匹配`
  }

  ElMessage.warning('智能路由服务暂不可用，已生成模拟匹配结果')
}

function selectRoutedExpert(item) {
  const id = item.id || item.expert_id
  if (!id) return
  if (!selectedExpertIds.value.includes(id)) {
    selectedExpertIds.value.push(id)
  }
  ElMessage.success(`已选择专家「${item.name || item.expert_name}」`)
}

function selectAllRoutedExperts() {
  const items = smartRouteResult.value?.selected || []
  let added = 0
  items.forEach(item => {
    const id = item.id || item.expert_id
    if (id && !selectedExpertIds.value.includes(id)) {
      selectedExpertIds.value.push(id)
      added++
    }
  })
  if (added > 0) {
    ElMessage.success(`已添加 ${added} 位推荐专家`)
  } else {
    ElMessage.info('推荐专家均已选中')
  }
  showSmartRouteDialog.value = false
}

// ========== 全局事件 ==========
function handleOpenRegisterExpert() {
  showRegisterDialog.value = true
}
function handleOpenExpertDebate() {
  openDebateDialog()
}
function handleOpenMultiConsult() {
  openMultiConsultDialog()
}
function handleSmartRouteExpert() {
  openSmartRouteDialog()
}

// ========== 快捷工具 ==========
function triggerDebate() {
  if (selectedExpertIds.value.length < 2) {
    ElMessage.warning('请至少选择 2 位专家进行辩论')
    return
  }
  activeMode.value = 'debate'
  collabExpanded.value = true
  collabMode.value = 'debate'
  newCollaboration()
  collabInput.value = `请以下专家就[主题]展开辩论：${selectedExpertNames()}`
}

function triggerOrchestration() {
  activeMode.value = 'orchestration'
  collabExpanded.value = true
  collabMode.value = 'multi'
  // 如果还没有任务，设置默认任务描述
  if (taskOrchestration.subtasks.length === 0) {
    taskOrchestration.originalTask = '设计并实现一个基于知识图谱的智能问答系统，要求支持多轮对话和上下文理解'
  }
}

function triggerVoting() {
  if (selectedExpertIds.value.length < 2) {
    ElMessage.warning('请至少选择 2 位专家参与投票')
    return
  }
  collabExpanded.value = true
  collabInput.value = `请以下专家就方案进行投票：${selectedExpertNames()}`
}

function triggerConsult() {
  activeMode.value = 'collaboration'
  collabExpanded.value = true
  collabMode.value = selectedExpertIds.value.length > 1 ? 'multi' : 'single'
  if (selectedExpertIds.value.length > 0) {
    collabInput.value = `请${selectedExpertNames()}专家分析：`
  }
}

function selectedExpertNames() {
  return experts.value
    .filter(e => selectedExpertIds.value.includes(e.id))
    .map(e => e.name)
    .join('、')
}

// ========== 图谱画布 ==========
const canvasRef = ref(null)
const activeCanvasTool = ref('select')
const currentLayout = ref('force')
const selectedNode = ref(null)
const graphLoading = ref(false)
const graphAnalyzing = ref(false)

const canvasTools = [
  { key: 'select', icon: 'Pointer', label: '选择' },
  { key: 'pan', icon: 'Rank', label: '平移' },
  { key: 'add-node', icon: 'Plus', label: '添加节点' },
  { key: 'add-edge', icon: 'Link', label: '添加关系' },
  { key: 'delete', icon: 'Delete', label: '删除' }
]

const layouts = [
  { key: 'force', icon: '🔄', label: '力导向' },
  { key: 'radial', icon: '☀️', label: '辐射' },
  { key: 'hierarchical', icon: '🏛️', label: '层次' },
  { key: 'circular', icon: '⭕', label: '环形' }
]

// 图谱视口控制
const viewport = ref({ x: 0, y: 0, scale: 1 })
const svgViewBox = computed(() => {
  const w = 800 / viewport.value.scale
  const h = 500 / viewport.value.scale
  const x = viewport.value.x - w / 2 + 400
  const y = viewport.value.y - h / 2 + 250
  return `${x} ${y} ${w} ${h}`
})

const graphNodes = ref([])
const graphEdges = ref([])
const graphStats = ref({ nodes: 0, edges: 0, types: 0 })

async function loadGraphData() {
  graphLoading.value = true
  try {
    const res = await getExpertGraph()
    const data = res?.data || res
    if (data?.nodes && data?.edges) {
      graphNodes.value = normalizeGraphNodes(data.nodes)
      graphEdges.value = normalizeGraphEdges(data.edges, data.nodes)
      graphStats.value = {
        nodes: data.nodes.length,
        edges: data.edges.length,
        types: [...new Set(data.nodes.map(n => n.type || 'default'))].length
      }
    } else {
      useMockGraph()
    }
  } catch (e) {
    console.warn('[workspace] 加载图谱失败，使用模拟数据:', e)
    useMockGraph()
  } finally {
    graphLoading.value = false
  }
}

function useMockGraph() {
  const nodes = [
    { id: 'n1', label: '专家', fullName: '专家实体', type: '核心实体', x: 400, y: 200, size: 28, color: '#6366f1', docs: 156, experts: 12, rank: 1, highlight: true },
    { id: 'n2', label: '知识', fullName: '知识节点', type: '核心实体', x: 250, y: 280, size: 24, color: '#06b6d4', docs: 203, experts: 8, rank: 2 },
    { id: 'n3', label: '文档', fullName: '文档实体', type: '内容实体', x: 550, y: 280, size: 22, color: '#10b981', docs: 892, experts: 5, rank: 3 },
    { id: 'n4', label: '图谱', fullName: '知识图谱', type: '系统', x: 320, y: 120, size: 20, color: '#8b5cf6', docs: 45, experts: 7, rank: 5 },
    { id: 'n5', label: '算法', fullName: '图算法', type: '算法', x: 480, y: 120, size: 18, color: '#f59e0b', docs: 67, experts: 6, rank: 4 },
    { id: 'n6', label: '协作', fullName: '协作模式', type: '关系类型', x: 180, y: 180, size: 16, color: '#ec4899', docs: 23, experts: 4, rank: 8 },
    { id: 'n7', label: '云盘', fullName: '云存储', type: '系统', x: 620, y: 180, size: 18, color: '#14b8a6', docs: 34, experts: 3, rank: 7 },
    { id: 'n8', label: '编排', fullName: '任务编排', type: '能力', x: 200, y: 380, size: 16, color: '#f97316', docs: 28, experts: 5, rank: 9 },
    { id: 'n9', label: '辩论', fullName: '专家辩论', type: '协作模式', x: 400, y: 380, size: 18, color: '#ef4444', docs: 12, experts: 8, rank: 6 },
    { id: 'n10', label: '融合', fullName: '知识融合', type: '能力', x: 600, y: 380, size: 16, color: '#84cc16', docs: 31, experts: 4, rank: 10 }
  ]
  const edges = [
    { id: 'e1', source: 'n1', target: 'n2', color: '#6366f1', width: 2 },
    { id: 'e2', source: 'n1', target: 'n3', color: '#6366f1', width: 2 },
    { id: 'e3', source: 'n1', target: 'n4', color: '#94a3b8' },
    { id: 'e4', source: 'n1', target: 'n5', color: '#94a3b8' },
    { id: 'e5', source: 'n2', target: 'n6', color: '#94a3b8' },
    { id: 'e6', source: 'n3', target: 'n7', color: '#94a3b8' },
    { id: 'e7', source: 'n2', target: 'n8', color: '#94a3b8' },
    { id: 'e8', source: 'n1', target: 'n9', color: '#ef4444', width: 2 },
    { id: 'e9', source: 'n3', target: 'n10', color: '#94a3b8' },
    { id: 'e10', source: 'n4', target: 'n5', color: '#94a3b8' },
    { id: 'e11', source: 'n2', target: 'n4', color: '#06b6d4', width: 1.5 },
    { id: 'e12', source: 'n3', target: 'n5', color: '#10b981', width: 1.5 }
  ]
  graphNodes.value = nodes
  graphEdges.value = edges.map(e => {
    const s = nodes.find(n => n.id === e.source)
    const t = nodes.find(n => n.id === e.target)
    return { ...e, sourceX: s?.x || 0, sourceY: s?.y || 0, targetX: t?.x || 0, targetY: t?.y || 0 }
  })
  graphStats.value = { nodes: nodes.length, edges: edges.length, types: 7 }
}

function normalizeGraphNodes(nodes) {
  return nodes.map((n, i) => ({
    id: n.id || `n${i}`,
    label: (n.label || n.name || '?').slice(0, 4),
    fullName: n.name || n.label || '',
    type: n.type || '节点',
    x: n.x || 200 + Math.random() * 400,
    y: n.y || 100 + Math.random() * 300,
    size: n.size || (n.highlight ? 24 : 18),
    color: n.color || expertColor(n.type),
    docs: n.doc_count || n.docs || 0,
    experts: n.expert_count || n.experts || 0,
    rank: n.rank || '-',
    highlight: n.highlight || false,
    description: n.description || ''
  }))
}

function normalizeGraphEdges(edges, nodes) {
  const nodeMap = {}
  nodes.forEach(n => { nodeMap[n.id || n.name] = n })
  return edges.map((e, i) => {
    const s = nodeMap[e.source || e.from || e.s]
    const t = nodeMap[e.target || e.to || e.t]
    return {
      id: e.id || `e${i}`,
      sourceX: s?.x || 0,
      sourceY: s?.y || 0,
      targetX: t?.x || 0,
      targetY: t?.y || 0,
      color: e.color || '#94a3b8',
      width: e.width || 1.5,
      highlight: e.highlight || false
    }
  })
}

function nodeColor(node) {
  return node.color || '#6366f1'
}

const nodeCardStyle = computed(() => {
  if (!selectedNode.value) return {}
  const scale = viewport.value.scale
  const nodeX = selectedNode.value.x * scale
  const nodeY = selectedNode.value.y * scale
  return {
    left: Math.min(nodeX + 30, 500) + 'px',
    top: Math.max(nodeY - 60, 20) + 'px'
  }
})

function selectNode(node) {
  selectedNode.value = node
  // 高亮关联边
  graphEdges.value.forEach(e => {
    e.highlight = e.id?.includes(node.id) ||
      graphEdges.value.some(edge =>
        (edge.sourceX === node.x && edge.sourceY === node.y) ||
        (edge.targetX === node.x && edge.targetY === node.y)
      )
  })
}

function viewNodeDocs(node) {
  rightCollapsed.value = false
  activeKbTab.value = 'docs'
  kbSearchQuery.value = node.fullName || node.label
  searchKb()
}

function askExpertsAbout(node) {
  collabExpanded.value = true
  collabInput.value = `请专家们分析一下「${node.fullName || node.label}」的相关情况，包括其定义、关联关系和应用场景。`
  if (!activeSession.value) {
    newCollaboration()
  }
}

function switchLayout(layout) {
  currentLayout.value = layout
  // 模拟布局切换动画
  applyLayout(layout)
}

function applyLayout(layout) {
  const nodes = graphNodes.value
  const cx = 400, cy = 250
  if (layout === 'force') {
    // 力导向 - 恢复原始位置
    useMockGraph()
  } else if (layout === 'radial') {
    // 辐射布局
    const center = nodes[0]
    if (center) {
      center.x = cx
      center.y = cy
    }
    nodes.slice(1).forEach((n, i) => {
      const angle = (i / (nodes.length - 1)) * Math.PI * 2
      const r = 120 + (i % 3) * 40
      n.x = cx + Math.cos(angle) * r
      n.y = cy + Math.sin(angle) * r
    })
    updateEdgePositions()
  } else if (layout === 'hierarchical') {
    // 层次布局
    const levels = 4
    const perLevel = Math.ceil(nodes.length / levels)
    nodes.forEach((n, i) => {
      const level = Math.floor(i / perLevel)
      const posInLevel = i % perLevel
      const nodesInLevel = Math.min(perLevel, nodes.length - level * perLevel)
      n.x = cx + (posInLevel - (nodesInLevel - 1) / 2) * 100
      n.y = 80 + level * 130
    })
    updateEdgePositions()
  } else if (layout === 'circular') {
    // 环形布局
    nodes.forEach((n, i) => {
      const angle = (i / nodes.length) * Math.PI * 2 - Math.PI / 2
      const r = 150
      n.x = cx + Math.cos(angle) * r
      n.y = cy + Math.sin(angle) * r
    })
    updateEdgePositions()
  }
}

function updateEdgePositions() {
  const nodeMap = {}
  graphNodes.value.forEach(n => { nodeMap[n.id] = n })
  // 注意：这里简单根据 source/target id 匹配，模拟数据用坐标匹配
  graphEdges.value.forEach(e => {
    // 尝试通过坐标近似匹配
    const s = graphNodes.value.find(n => Math.abs(n.x - e.sourceX) < 1 && Math.abs(n.y - e.sourceY) < 1)
    const t = graphNodes.value.find(n => Math.abs(n.x - e.targetX) < 1 && Math.abs(n.y - e.targetY) < 1)
    if (s) { e.sourceX = s.x; e.sourceY = s.y }
    if (t) { e.targetX = t.x; e.targetY = t.y }
  })
}

function zoomIn() {
  viewport.value.scale = Math.min(viewport.value.scale * 1.2, 3)
}
function zoomOut() {
  viewport.value.scale = Math.max(viewport.value.scale / 1.2, 0.3)
}
function fitView() {
  viewport.value = { x: 0, y: 0, scale: 1 }
}

async function runGraphAlgo() {
  graphAnalyzing.value = true
  try {
    // 模拟图谱分析
    await new Promise(r => setTimeout(r, 1500))
    // 高亮中心节点
    graphNodes.value.forEach((n, i) => {
      n.highlight = i < 3
    })
    ElMessage.success('图谱分析完成，已高亮核心节点')
  } catch (e) {
    ElMessage.error('图谱分析失败')
  } finally {
    graphAnalyzing.value = false
  }
}

// 画布拖拽
let isDragging = false
let dragStart = { x: 0, y: 0 }
let viewportStart = { x: 0, y: 0 }

function onCanvasMouseDown(e) {
  if (activeCanvasTool.value === 'pan' || e.button === 1) {
    isDragging = true
    dragStart = { x: e.clientX, y: e.clientY }
    viewportStart = { ...viewport.value }
  }
}

function onCanvasMouseMove(e) {
  if (isDragging) {
    const dx = (e.clientX - dragStart.x) / viewport.value.scale
    const dy = (e.clientY - dragStart.y) / viewport.value.scale
    viewport.value.x = viewportStart.x - dx
    viewport.value.y = viewportStart.y - dy
  }
}

function onCanvasMouseUp() {
  isDragging = false
}

function onCanvasWheel(e) {
  e.preventDefault()
  const delta = e.deltaY > 0 ? 0.9 : 1.1
  viewport.value.scale = Math.max(0.3, Math.min(3, viewport.value.scale * delta))
}

function onNodeMouseDown(e, node) {
  // 节点拖拽逻辑可扩展
}

// ========== 协作对话 ==========
const collabMessages = ref([])
const collabInput = ref('')
const collabMode = ref('smart')
const allianceRunning = ref(false)
const currentPhaseIndex = ref(-1)
const messagesScrollRef = ref(null)
let allianceAbortController = null

const alliancePhases = [
  { key: 'intent', label: '意图识别' },
  { key: 'team', label: '组队匹配' },
  { key: 'debate', label: '专家辩论' },
  { key: 'synthesize', label: '综合归纳' },
  { key: 'gate', label: '质量把关' },
  { key: 'learn', label: '知识学习' },
  { key: 'done', label: '完成' }
]

const currentPhaseLabel = computed(() => {
  if (currentPhaseIndex.value < 0) return '准备中'
  return alliancePhases[currentPhaseIndex.value]?.label || '处理中'
})

function phaseLabel(phase) {
  const p = alliancePhases.find(p => p.key === phase)
  return p?.label || phase
}

function phaseTagType(phase) {
  const types = {
    intent: 'info', team: 'primary', debate: 'warning',
    synthesize: 'success', gate: 'danger', learn: '', done: 'success'
  }
  return types[phase] || 'info'
}

function formatMessageText(text) {
  if (!text) return ''
  // 简单的格式化：转义HTML + 换行
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/\n/g, '<br/>')
}

function onCollabModeChange() {
  // 模式变化时的处理
}

async function sendCollabMsg() {
  if (!collabInput.value.trim() || allianceRunning.value) return
  const text = collabInput.value.trim()
  collabInput.value = ''

  // 添加用户消息
  collabMessages.value.push({
    id: Date.now(),
    role: 'user',
    name: '我',
    avatar: 'U',
    color: 'linear-gradient(135deg, #6366f1, #06b6d4)',
    time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
    text
  })

  scrollMessagesToBottom()

  // 如果没有活动会话，创建一个
  if (!activeSession.value) {
    newCollaboration()
  }

  // 启动联盟 SSE
  await runAlliance(text)
}

async function runAlliance(query) {
  allianceRunning.value = true
  currentPhaseIndex.value = 0

  try {
    await runAllianceFullSSE(
      {
        query,
        session_id: activeSession.value?.id,
        enable_llm_debate: collabMode.value === 'debate',
        team_size: selectedExpertIds.value.length || 3,
        context: {
          project_id: currentProject.value,
          mode: collabMode.value,
          selected_experts: JSON.stringify(selectedExpertIds.value)
        }
      },
      (frame) => {
        handleAllianceFrame(frame)
      }
    )
  } catch (e) {
    console.warn('[alliance] SSE 调用失败，使用模拟响应:', e)
    await simulateAllianceResponse(query)
  } finally {
    allianceRunning.value = false
    currentPhaseIndex.value = alliancePhases.length - 1
    setTimeout(() => { currentPhaseIndex.value = -1 }, 2000)
  }
}

function handleAllianceFrame(frame) {
  const phaseIdx = alliancePhases.findIndex(p => p.key === frame.phase)
  if (phaseIdx >= 0) {
    currentPhaseIndex.value = phaseIdx
  }

  // 根据阶段添加消息
  if (frame.payload) {
    let msg = null

    if (frame.phase === 'intent') {
      msg = {
        id: Date.now() + Math.random(),
        role: 'assistant',
        name: '意图分析',
        avatar: '🎯',
        color: '#6366f1',
        phase: 'intent',
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
        text: frame.payload.intent || frame.payload.summary || '正在分析您的问题意图…'
      }
    } else if (frame.phase === 'team') {
      const experts = frame.payload.experts || frame.payload.team || []
      msg = {
        id: Date.now() + Math.random(),
        role: 'assistant',
        name: '组队匹配',
        avatar: '👥',
        color: '#06b6d4',
        phase: 'team',
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
        text: `已匹配 ${experts.length} 位专家：${experts.map(e => e.name || e).join('、')}`
      }
    } else if (frame.phase === 'debate') {
      msg = {
        id: Date.now() + Math.random(),
        role: 'expert',
        name: frame.payload.expert_name || '专家发言',
        avatar: (frame.payload.expert_name || '专')[0],
        color: expertColor(frame.payload.expert_type),
        phase: 'debate',
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
        text: frame.payload.content || frame.payload.argument || ''
      }
    } else if (frame.phase === 'synthesize') {
      msg = {
        id: Date.now() + Math.random(),
        role: 'assistant',
        name: '综合归纳',
        avatar: '📝',
        color: '#10b981',
        phase: 'synthesize',
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
        text: frame.payload.summary || frame.payload.synthesis || '正在综合各方观点…'
      }
    } else if (frame.phase === 'done') {
      msg = {
        id: Date.now() + Math.random(),
        role: 'assistant',
        name: '协作完成',
        avatar: '✅',
        color: '#10b981',
        phase: 'done',
        time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
        text: frame.payload.final_answer || frame.payload.result || '协作完成，以上是综合结果。'
      }
    }

    if (msg && msg.text) {
      collabMessages.value.push(msg)
      scrollMessagesToBottom()
    }
  }
}

async function simulateAllianceResponse(query) {
  // 模拟联盟响应（当API不可用时）
  const phases = [
    { phase: 'intent', name: '意图分析', avatar: '🎯', color: '#6366f1',
      text: `已识别您的问题类型：${collabMode.value === 'debate' ? '辩题型' : '咨询型'}。正在匹配相关专家…` },
    { phase: 'team', name: '组队匹配', avatar: '👥', color: '#06b6d4',
      text: `已为您匹配 ${Math.min(selectedExpertIds.value.length || 3, 5)} 位专家：${selectedExpertNames() || '林算法、陈架构、张AI'}` },
    { phase: 'debate', name: '林算法', avatar: '璇', color: '#6366f1',
      text: `从算法角度分析，「${query.slice(0, 20)}」这个问题可以采用动态规划结合图论的方法来解决。时间复杂度为 O(n²)，空间复杂度为 O(n)。` },
    { phase: 'debate', name: '陈架构', avatar: '架', color: '#06b6d4',
      text: '从系统架构角度，我建议采用微服务架构，将算法能力封装为独立服务，通过 gRPC 调用。这样可以实现水平扩展和独立部署。' },
    { phase: 'synthesize', name: '综合归纳', avatar: '📝', color: '#10b981',
      text: '综合各位专家的观点：建议采用「微服务 + 算法核心库」的混合架构。算法层抽成统一核心库，服务层通过 gRPC 对外提供能力，既能保证性能又能实现灵活扩展。' },
    { phase: 'done', name: '协作完成', avatar: '✅', color: '#10b981',
      text: '专家联盟协作已完成！以上是综合分析结果。如需进一步讨论，可以继续提问或选择特定专家深入咨询。' }
  ]

  for (let i = 0; i < phases.length; i++) {
    if (!allianceRunning.value) break
    currentPhaseIndex.value = i
    await new Promise(r => setTimeout(r, 800 + Math.random() * 600))

    const p = phases[i]
    collabMessages.value.push({
      id: Date.now() + i,
      role: p.phase === 'debate' ? 'expert' : 'assistant',
      name: p.name,
      avatar: p.avatar,
      color: p.color,
      phase: p.phase,
      time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
      text: p.text
    })
    scrollMessagesToBottom()
  }
}

function stopAlliance() {
  allianceRunning.value = false
  if (allianceAbortController) {
    allianceAbortController.abort()
    allianceAbortController = null
  }
  collabMessages.value.push({
    id: Date.now(),
    role: 'system',
    name: '系统',
    avatar: '⚠️',
    color: '#f59e0b',
    time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
    text: '协作已被用户停止'
  })
}

function scrollMessagesToBottom() {
  nextTick(() => {
    if (messagesScrollRef.value) {
      messagesScrollRef.value.scrollTo?.({ top: 99999, behavior: 'smooth' })
    }
  })
}

function insertNodeRef() {
  if (selectedNode.value) {
    collabInput.value += `【节点：${selectedNode.value.fullName || selectedNode.value.label}】`
  }
}

// ========== 知识库 ==========
const activeKbTab = ref('docs')
const kbSearchQuery = ref('')
const activeDoc = ref(null)
const docsLoading = ref(false)
const categories = ref([])
const documents = ref([])
const popularTags = ref([])
const docVersions = ref([])
const activeCategory = ref(null)
const expandedCategories = ref([])

const kbTabs = [
  { key: 'docs', icon: 'Document', label: '文档' },
  { key: 'tags', icon: 'CollectionTag', label: '标签' },
  { key: 'versions', icon: 'RefreshRight', label: '版本' }
]

const filteredDocs = computed(() => {
  let list = documents.value
  if (activeCategory.value) {
    list = list.filter(d => d.category_id === activeCategory.value)
  }
  if (kbSearchQuery.value) {
    const kw = kbSearchQuery.value.toLowerCase()
    list = list.filter(d =>
      (d.title || d.name || '').toLowerCase().includes(kw) ||
      (d.tags || []).some(t => (t || '').toLowerCase().includes(kw))
    )
  }
  return list
})

async function switchKbTab(tab) {
  activeKbTab.value = tab
  if (tab === 'docs') {
    if (documents.value.length === 0) loadDocuments()
    if (categories.value.length === 0) loadCategories()
  } else if (tab === 'tags') {
    if (popularTags.value.length === 0) loadTags()
  } else if (tab === 'versions') {
    if (activeDoc.value) {
      loadVersions(activeDoc.value.id)
    }
  }
}

async function loadDocuments() {
  docsLoading.value = true
  try {
    const res = await kbListDocuments({ project_id: currentProject.value, limit: 50 })
    if (res && Array.isArray(res.data)) {
      documents.value = res.data
    } else if (res && Array.isArray(res)) {
      documents.value = res
    } else {
      documents.value = getMockDocs()
    }
  } catch (e) {
    console.warn('[workspace] 加载文档失败，使用模拟数据:', e)
    documents.value = getMockDocs()
  } finally {
    docsLoading.value = false
  }
}

async function loadCategories() {
  try {
    const res = await kbGetCategories()
    if (res && Array.isArray(res.data)) {
      categories.value = res.data
    } else if (res && Array.isArray(res)) {
      categories.value = res
    } else {
      categories.value = getMockCategories()
    }
    expandedCategories.value = categories.value.map(c => c.id)
  } catch (e) {
    categories.value = getMockCategories()
    expandedCategories.value = categories.value.map(c => c.id)
  }
}

async function loadTags() {
  try {
    const res = await kbGetTags()
    if (res && Array.isArray(res.data)) {
      popularTags.value = res.data.map(t => ({
        name: t.name || t.tag,
        count: t.count || 0,
        fontSize: 12 + Math.min(t.count || 0, 20) * 0.5
      }))
    } else {
      popularTags.value = getMockTags()
    }
  } catch (e) {
    popularTags.value = getMockTags()
  }
}

async function loadVersions(docId) {
  try {
    const res = await kbGetVersions(docId)
    if (res && Array.isArray(res.data)) {
      docVersions.value = res.data
    } else if (res && Array.isArray(res)) {
      docVersions.value = res
    } else {
      docVersions.value = getMockVersions()
    }
  } catch (e) {
    docVersions.value = getMockVersions()
  }
}

function getMockDocs() {
  return [
    { id: 'doc-001', title: '专家联盟架构设计 V3.0', type: 'pdf', size: 2516582,
      category_id: 'cat-arch', updated_at: Date.now() - 600000,
      tags: ['架构设计', '专家系统', '微服务'], graph_linked: true },
    { id: 'doc-002', title: '知识图谱域架构规范', type: 'doc', size: 1887436,
      category_id: 'cat-arch', updated_at: Date.now() - 7200000,
      tags: ['知识图谱', '架构规范'], graph_linked: true },
    { id: 'doc-003', title: '算法归一化设计方案', type: 'doc', size: 978944,
      category_id: 'cat-arch', updated_at: Date.now() - 86400000,
      tags: ['算法', '归一化'], graph_linked: false },
    { id: 'doc-004', title: '云存储域接口定义', type: 'api', size: 634880,
      category_id: 'cat-arch', updated_at: Date.now() - 259200000,
      tags: ['云存储', 'API'], graph_linked: true },
    { id: 'doc-005', title: '中心性算法对比研究', type: 'pdf', size: 1258291,
      category_id: 'cat-algo', updated_at: Date.now() - 86400000,
      tags: ['图算法', '中心性', '研究'], graph_linked: true },
    { id: 'doc-006', title: '社区发现算法优化', type: 'doc', size: 911360,
      category_id: 'cat-algo', updated_at: Date.now() - 259200000,
      tags: ['图算法', '社区发现'], graph_linked: false },
    { id: 'doc-007', title: 'RAG 检索增强生成实践', type: 'pdf', size: 2097152,
      category_id: 'cat-ai', updated_at: Date.now() - 172800000,
      tags: ['RAG', 'LLM', 'AI'], graph_linked: true },
    { id: 'doc-008', title: '向量检索性能调优指南', type: 'doc', size: 734003,
      category_id: 'cat-algo', updated_at: Date.now() - 432000000,
      tags: ['向量检索', '性能优化'], graph_linked: false }
  ]
}

function getMockCategories() {
  return [
    { id: 'cat-arch', name: '架构设计文档', count: 4 },
    { id: 'cat-algo', name: '算法研究', count: 3 },
    { id: 'cat-ai', name: 'AI 模型', count: 1 },
    { id: 'cat-data', name: '数据规范', count: 0 }
  ]
}

function getMockTags() {
  const tags = [
    { name: '架构设计', count: 15 },
    { name: '知识图谱', count: 12 },
    { name: '图算法', count: 10 },
    { name: '专家系统', count: 14 },
    { name: '微服务', count: 8 },
    { name: 'RAG', count: 6 },
    { name: '向量检索', count: 5 },
    { name: '性能优化', count: 9 },
    { name: '模块化', count: 7 },
    { name: '归一化', count: 4 },
    { name: '协作模式', count: 3 },
    { name: '知识融合', count: 6 }
  ]
  return tags.map(t => ({
    ...t,
    fontSize: 12 + Math.min(t.count, 20) * 0.4
  }))
}

function getMockVersions() {
  return [
    { id: 'v1', version: '3.0', created_at: Date.now() - 3600000, author: '陈架构', action: '重大版本更新' },
    { id: 'v2', version: '2.5', created_at: Date.now() - 86400000, author: '林算法', action: '新增算法章节' },
    { id: 'v3', version: '2.1', created_at: Date.now() - 172800000, author: '王数据', action: '修订数据模型' },
    { id: 'v4', version: '2.0', created_at: Date.now() - 604800000, author: '陈架构', action: '重构架构设计' }
  ]
}

function selectCategory(cat) {
  activeCategory.value = activeCategory.value === cat.id ? null : cat.id
}

function openDoc(doc) {
  activeDoc.value = doc
  // 如果在版本标签页，加载版本
  if (activeKbTab.value === 'versions') {
    loadVersions(doc.id)
  }
}

async function searchKb() {
  if (!kbSearchQuery.value.trim()) {
    // 清空搜索，恢复列表
    if (documents.value.length === 0) {
      loadDocuments()
    }
    return
  }
  docsLoading.value = true
  try {
    const res = await kbSearch({ query: kbSearchQuery.value, project_id: currentProject.value })
    if (res && Array.isArray(res.data)) {
      documents.value = res.data
    }
  } catch (e) {
    // 使用前端过滤
  } finally {
    docsLoading.value = false
  }
}

function filterByTag(tag) {
  kbSearchQuery.value = tag.name
  activeKbTab.value = 'docs'
  searchKb()
}

function docIcon(type) {
  const icons = { pdf: '📕', doc: '📄', api: '🔌', image: '🖼️', code: '💻', sheet: '📊' }
  return icons[type] || '📄'
}

function formatFileSize(bytes) {
  if (!bytes) return '未知'
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB'
  return (bytes / 1048576).toFixed(1) + ' MB'
}

function handleBeforeUpload(file) {
  ElMessage.info(`正在上传：${file.name}`)
  // 实际应调用上传 API
  return false // 阻止自动上传
}

function createDoc() {
  ElMessage.info('新建文档功能开发中…')
}

// ========== AI 助手 ==========
const allianceCapabilitiesList = ref([])

async function loadAllianceCapabilities() {
  try {
    const caps = await getAllianceCapabilities()
    if (caps?.intent_classes_7) {
      allianceCapabilitiesList.value = caps.intent_classes_7
    } else {
      allianceCapabilitiesList.value = [
        '7 类意图识别', '专家智能匹配', '多轮交叉辩论',
        '综合方案归纳', '质量闸门把关', '知识增量学习', '14 维度评估'
      ]
    }
  } catch (e) {
    allianceCapabilitiesList.value = [
      '7 类意图识别', '专家智能匹配', '多轮交叉辩论',
      '综合方案归纳', '质量闸门把关', '知识增量学习', '14 维度评估'
    ]
  }
}

function openAIAssistant() {
  aiAssistantOpen.value = !aiAssistantOpen.value
}

function aiSuggestion(type) {
  collabInput.value = `请执行：${type}`
  aiAssistantOpen.value = false
  collabExpanded.value = true
  if (!activeSession.value) {
    newCollaboration()
  }
}

// ========== 通知 ==========
function removeNotification(id) {
  const idx = notifications.value.findIndex(n => n.id === id)
  if (idx >= 0) notifications.value.splice(idx, 1)
}

// ========== 工具函数 ==========
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

// ========== 生命周期 ==========
onMounted(() => {
  loadExperts()
  loadSessions()
  loadGraphData()
  loadCategories()
  loadDocuments()
  loadTags()
  loadAllianceCapabilities()

  // 注册全局事件监听
  window.addEventListener('mox:open-register-expert', handleOpenRegisterExpert)
  window.addEventListener('mox:open-expert-debate', handleOpenExpertDebate)
  window.addEventListener('mox:open-multi-consult', handleOpenMultiConsult)
  window.addEventListener('mox:smart-route-expert', handleSmartRouteExpert)

  // 快捷键监听
  window.addEventListener('keydown', handleKeydown)
})

onBeforeUnmount(() => {
  // 清理全局事件监听
  window.removeEventListener('mox:open-register-expert', handleOpenRegisterExpert)
  window.removeEventListener('mox:open-expert-debate', handleOpenExpertDebate)
  window.removeEventListener('mox:open-multi-consult', handleOpenMultiConsult)
  window.removeEventListener('mox:smart-route-expert', handleSmartRouteExpert)

  // 清理快捷键监听
  window.removeEventListener('keydown', handleKeydown)
})

// 监听选中专家变化，更新会话中的专家数
watch(selectedExpertIds, () => {
  if (activeSession.value) {
    activeSession.value.expert_count = selectedExpertIds.value.length
  }
}, { deep: true })

// ============================================================================
// 任务编排模式 · Task Orchestration
// ============================================================================

// ---- 数据结构 ----
const taskOrchestration = reactive({
  originalTask: '',
  subtasks: [],
  executionMode: 'auto', // auto | manual
  progress: { total: 0, completed: 0, inProgress: 0, failed: 0, percentage: 0 }
})

const decomposing = ref(false)
const orchIsRunning = ref(false)
const activeSubtaskId = ref(null)
const timelineView = ref('gantt') // gantt | list
const draggingTaskId = ref(null)
const dragOverTaskId = ref(null)
const draggingExpert = ref(null)
const expertDragOverTaskId = ref(null)
const showAssignDialog = ref(false)
const assignTargetTask = ref(null)

// ---- 任务状态常量 ----
const SUBTASK_STATUS = {
  PENDING: 'pending',           // 待分配
  WAITING: 'waiting',           // 等待中（依赖未完成）
  IN_PROGRESS: 'inProgress',    // 进行中
  REVIEWING: 'reviewing',       // 审核中
  COMPLETED: 'completed',       // 已完成
  AT_RISK: 'atRisk',            // 有风险
  FAILED: 'failed',             // 失败
  ARCHIVED: 'archived'          // 已归档
}

const SUBTASK_STATUS_MAP = {
  pending: { label: '待分配', icon: '📋', color: '#64748b' },
  waiting: { label: '等待中', icon: '⏳', color: '#f59e0b' },
  inProgress: { label: '进行中', icon: '🚀', color: '#3b82f6' },
  reviewing: { label: '审核中', icon: '🔍', color: '#8b5cf6' },
  completed: { label: '已完成', icon: '✅', color: '#10b981' },
  atRisk: { label: '有风险', icon: '⚠️', color: '#f97316' },
  failed: { label: '失败', icon: '❌', color: '#ef4444' },
  archived: { label: '已归档', icon: '📦', color: '#64748b' }
}

function subtaskStatusText(status) {
  return SUBTASK_STATUS_MAP[status]?.label || status
}

function subtaskStatusTagType(status) {
  const map = {
    pending: 'info', waiting: 'warning', inProgress: 'primary',
    reviewing: 'warning', completed: 'success', atRisk: 'danger',
    failed: 'danger', archived: 'info'
  }
  return map[status] || 'info'
}

// ---- 优先级渐变 ----
function subtaskPriorityGradient(priority) {
  const gradients = {
    high: 'linear-gradient(135deg, #ef4444, #f97316)',
    medium: 'linear-gradient(135deg, #f59e0b, #eab308)',
    low: 'linear-gradient(135deg, #10b981, #14b8a6)'
  }
  return gradients[priority] || gradients.medium
}

// ---- 计算属性 ----
const orchProgress = computed(() => {
  const subtasks = taskOrchestration.subtasks
  const total = subtasks.length
  const completed = subtasks.filter(t => t.status === 'completed').length
  const inProgress = subtasks.filter(t => t.status === 'inProgress' || t.status === 'reviewing').length
  const failed = subtasks.filter(t => t.status === 'failed').length
  const percentage = total > 0 ? Math.round((completed / total) * 100) : 0
  return { total, completed, inProgress, failed, percentage }
})

const availableExperts = computed(() => experts.value.filter(e => e.status !== 'offline'))

const riskTasks = computed(() => taskOrchestration.subtasks.filter(t => t.status === 'atRisk'))

const ganttSlotMinutes = ref(15)
const ganttTimeSlots = computed(() => {
  const totalMinutes = taskOrchestration.subtasks.reduce((sum, t) => sum + (t.estimatedTime || 0), 0)
  return Math.max(Math.ceil(totalMinutes / ganttSlotMinutes.value), 4)
})

// ---- 辅助函数 ----
function getExpertById(id) {
  return experts.value.find(e => e.id === id)
}

function getSubtaskIndex(id) {
  return taskOrchestration.subtasks.findIndex(t => t.id === id)
}

function expertLoad(expertId) {
  return taskOrchestration.subtasks.filter(t => 
    t.expertIds?.includes(expertId) && 
    ['inProgress', 'reviewing', 'pending', 'waiting'].includes(t.status)
  ).length
}

function generateTaskId() {
  return 'task-' + Date.now() + '-' + Math.random().toString(36).substr(2, 6)
}

// ---- 协作消息辅助 ----
function addOrchMessage(msg) {
  collabMessages.value.push({
    id: Date.now() + Math.random(),
    time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
    ...msg
  })
  // 滚动到底部（如果消息滚动组件存在）
  if (messagesScrollRef.value) {
    nextTick(() => {
      messagesScrollRef.value.scrollTo?.({ top: 999999, behavior: 'smooth' })
    })
  }
}

// ---- 任务智能拆解 ----
async function decomposeTask() {
  if (!taskOrchestration.originalTask.trim()) {
    ElMessage.warning('请先输入任务描述')
    return
  }
  decomposing.value = true
  try {
    // 模拟 AI 拆解（实际应调用后端 API）
    await new Promise(resolve => setTimeout(resolve, 1500))
    
    const taskDesc = taskOrchestration.originalTask
    const subtasks = generateMockSubtasks(taskDesc)
    taskOrchestration.subtasks = subtasks
    updateGanttLayout()
    
    ElMessage.success(`已智能拆解为 ${subtasks.length} 个子任务`)
    addHistoryEvent('task', '任务智能拆解', `将「${taskDesc.substring(0, 20)}...」拆解为 ${subtasks.length} 个子任务`)
  } catch (e) {
    console.error('[orchestration] 任务拆解失败:', e)
    ElMessage.error('任务拆解失败，请重试')
  } finally {
    decomposing.value = false
  }
}

function generateMockSubtasks(taskDesc) {
  // 基于常见任务模式生成模拟子任务
  const templates = [
    { title: '需求分析与定义', desc: '深入分析任务需求，明确目标、边界和验收标准', type: 'requirement', time: 15, priority: 'high' },
    { title: '方案设计与架构规划', desc: '设计整体解决方案，规划技术架构和实现路径', type: 'architecture', time: 20, priority: 'high' },
    { title: '核心算法/模型研发', desc: '研发核心算法或AI模型，实现关键功能', type: 'algorithm', time: 30, priority: 'high' },
    { title: '数据处理与准备', desc: '数据采集、清洗、标注和预处理工作', type: 'data', time: 25, priority: 'medium' },
    { title: '系统集成与联调', desc: '各模块集成开发，接口联调测试', type: 'architecture', time: 20, priority: 'medium' },
    { title: '质量保障与测试', desc: '功能测试、性能测试、安全审计', type: 'security', time: 15, priority: 'medium' },
    { title: '文档编写与交付', desc: '编写技术文档、用户手册，准备交付物', type: 'requirement', time: 10, priority: 'low' }
  ]
  
  const numTasks = Math.min(Math.max(Math.floor(taskDesc.length / 30) + 3, 4), 7)
  const selected = templates.slice(0, numTasks)
  
  return selected.map((tpl, idx) => ({
    id: generateTaskId(),
    title: tpl.title,
    description: tpl.desc,
    priority: tpl.priority,
    status: idx === 0 ? 'pending' : 'waiting',
    suggestedExpertType: tpl.type,
    expertIds: [],
    dependencies: idx === 0 ? [] : [selected[0].id || ''],
    estimatedTime: tpl.time,
    startTime: null,
    endTime: null,
    result: '',
    messages: [],
    expanded: false,
    ganttOffset: 0,
    ganttWidth: 0
  })).map((task, idx, arr) => {
    // 设置正确的依赖关系
    if (idx > 0) {
      task.dependencies = [arr[idx - 1].id]
    }
    return task
  })
}

// ---- 甘特图布局计算 ----
function updateGanttLayout() {
  const subtasks = taskOrchestration.subtasks
  if (subtasks.length === 0) return
  
  // 计算总时间和每个任务的位置
  const totalMinutes = subtasks.reduce((sum, t) => sum + t.estimatedTime, 0)
  let cumulativeTime = 0
  
  subtasks.forEach(task => {
    task.ganttOffset = (cumulativeTime / totalMinutes) * 100
    task.ganttWidth = (task.estimatedTime / totalMinutes) * 100
    cumulativeTime += task.estimatedTime
  })
}

// ---- 子任务 CRUD ----
function addSubtaskManually() {
  const newTask = {
    id: generateTaskId(),
    title: '新子任务',
    description: '请编辑任务描述...',
    priority: 'medium',
    status: 'pending',
    suggestedExpertType: 'custom',
    expertIds: [],
    dependencies: [],
    estimatedTime: 15,
    startTime: null,
    endTime: null,
    result: '',
    messages: [],
    expanded: true,
    ganttOffset: 0,
    ganttWidth: 0
  }
  taskOrchestration.subtasks.push(newTask)
  updateGanttLayout()
  activeSubtaskId.value = newTask.id
}

function editSubtask(task) {
  activeSubtaskId.value = task.id
  task.expanded = true
  ElMessage.info('请在展开的详情中编辑任务信息')
}

function deleteSubtask(taskId) {
  const idx = getSubtaskIndex(taskId)
  if (idx >= 0) {
    const task = taskOrchestration.subtasks[idx]
    // 移除其他任务对该任务的依赖
    taskOrchestration.subtasks.forEach(t => {
      t.dependencies = t.dependencies.filter(d => d !== taskId)
    })
    taskOrchestration.subtasks.splice(idx, 1)
    updateGanttLayout()
    ElMessage.success('子任务已删除')
  }
}

function toggleSubtaskExpand(task) {
  task.expanded = !task.expanded
}

function collapseAllSubtasks() {
  taskOrchestration.subtasks.forEach(t => t.expanded = false)
}

function selectSubtask(task) {
  activeSubtaskId.value = task.id
}

// ---- 任务拖拽排序 ----
function onTaskDragStart(e, task) {
  draggingTaskId.value = task.id
  e.dataTransfer.effectAllowed = 'move'
  e.dataTransfer.setData('text/plain', task.id)
}

function onTaskDragEnd() {
  draggingTaskId.value = null
  dragOverTaskId.value = null
}

function onTaskDragOver(e, task) {
  if (draggingTaskId.value && draggingTaskId.value !== task.id) {
    dragOverTaskId.value = task.id
  }
}

function onTaskDrop(e, targetTask) {
  const draggedId = draggingTaskId.value
  if (!draggedId || draggedId === targetTask.id) return
  
  const draggedIdx = getSubtaskIndex(draggedId)
  const targetIdx = getSubtaskIndex(targetTask.id)
  
  if (draggedIdx >= 0 && targetIdx >= 0) {
    const [removed] = taskOrchestration.subtasks.splice(draggedIdx, 1)
    taskOrchestration.subtasks.splice(targetIdx, 0, removed)
    updateGanttLayout()
    ElMessage.success('任务顺序已调整')
  }
  
  draggingTaskId.value = null
  dragOverTaskId.value = null
}

// ---- 专家拖拽分配 ----
function onExpertDragStart(e, expert) {
  draggingExpert.value = expert
  e.dataTransfer.effectAllowed = 'copy'
  e.dataTransfer.setData('text/plain', expert.id)
}

function onExpertDragEnd() {
  draggingExpert.value = null
  expertDragOverTaskId.value = null
}

function onExpertDragOverTask(e, task) {
  expertDragOverTaskId.value = task.id
}

function onExpertDragLeaveTask() {
  expertDragOverTaskId.value = null
}

function onExpertDropOnTask(e, task) {
  const expert = draggingExpert.value
  if (!expert) return
  
  if (!task.expertIds.includes(expert.id)) {
    task.expertIds.push(expert.id)
    ElMessage.success(`已将 ${expert.name} 分配到「${task.title}」`)
  } else {
    ElMessage.info('该专家已分配到此任务')
  }
  
  draggingExpert.value = null
  expertDragOverTaskId.value = null
}

function unassignExpert(taskId, expertId) {
  const task = taskOrchestration.subtasks.find(t => t.id === taskId)
  if (task) {
    task.expertIds = task.expertIds.filter(id => id !== expertId)
    ElMessage.success('已取消专家分配')
  }
}

// ---- 智能分配专家 ----
async function autoAssignExperts() {
  if (taskOrchestration.subtasks.length === 0) {
    ElMessage.warning('请先创建子任务')
    return
  }
  
  try {
    await new Promise(resolve => setTimeout(resolve, 1000))
    
    let assignedCount = 0
    taskOrchestration.subtasks.forEach(task => {
      // 根据任务类型匹配最合适的专家
      const matchingExperts = experts.value.filter(e => 
        e.type === task.suggestedExpertType && e.status !== 'offline'
      )
      
      if (matchingExperts.length > 0) {
        // 选择负载最低的专家
        const bestExpert = matchingExperts.sort((a, b) => expertLoad(a.id) - expertLoad(b.id))[0]
        if (!task.expertIds.includes(bestExpert.id)) {
          task.expertIds = [bestExpert.id]
          assignedCount++
        }
      } else {
        // 没有完全匹配的，选负载最低的
        const available = experts.value.filter(e => e.status !== 'offline')
        if (available.length > 0) {
          const bestExpert = available.sort((a, b) => expertLoad(a.id) - expertLoad(b.id))[0]
          if (!task.expertIds.includes(bestExpert.id)) {
            task.expertIds = [bestExpert.id]
            assignedCount++
          }
        }
      }
    })
    
    ElMessage.success(`已智能分配 ${assignedCount} 个任务`)
    addHistoryEvent('task', '专家智能分配', `为 ${assignedCount} 个子任务自动匹配了专家`)
  } catch (e) {
    console.error('[orchestration] 智能分配失败:', e)
    ElMessage.error('智能分配失败，请重试')
  }
}

function openAssignDialog(task) {
  assignTargetTask.value = task
  showAssignDialog.value = true
}

// ---- 任务执行协调 ----
async function startTaskExecution() {
  if (taskOrchestration.subtasks.length === 0) {
    ElMessage.warning('请先创建子任务')
    return
  }
  
  // 检查是否所有任务都分配了专家
  const unassigned = taskOrchestration.subtasks.filter(t => t.expertIds.length === 0)
  if (unassigned.length > 0) {
    ElMessage.warning(`有 ${unassigned.length} 个任务未分配专家，是否使用智能分配？`)
    return
  }
  
  orchIsRunning.value = true
  ElMessage.success('任务执行已启动')
  addHistoryEvent('task', '开始任务执行', '任务编排流程已启动')
  
  // 模拟任务执行（按依赖关系串行执行）
  if (taskOrchestration.executionMode === 'auto') {
    executeTasksAuto()
  }
}

async function executeTasksAuto() {
  const tasks = [...taskOrchestration.subtasks]
  
  for (const task of tasks) {
    if (!orchIsRunning.value) break
    
    // 检查依赖是否已完成
    const depsCompleted = task.dependencies.every(depId => {
      const depTask = taskOrchestration.subtasks.find(t => t.id === depId)
      return depTask?.status === 'completed'
    })
    
    if (!depsCompleted) {
      task.status = 'waiting'
      continue
    }
    
    // 开始执行
    task.status = 'inProgress'
    task.startTime = Date.now()
    
    const expert = getExpertById(task.expertIds[0])
    addOrchMessage({
      role: 'assistant',
      name: expert?.name || 'AI专家',
      avatar: expertEmoji(expert?.type) || '🤖',
      color: expertColor(expert?.type) || '#6366f1',
      text: `开始执行「${task.title}」...`,
      status: 'thinking',
      phase: 'orchestration'
    })
    
    // 模拟执行时间
    await new Promise(resolve => setTimeout(resolve, Math.min(task.estimatedTime * 50, 2000)))
    
    // 模拟结果（90%成功率）
    const success = Math.random() > 0.1
    if (success) {
      task.status = 'completed'
      task.endTime = Date.now()
      task.result = `「${task.title}」执行完成，结果符合预期。\n核心产出：${task.description}的详细方案和实现代码。`
      
      addOrchMessage({
        role: 'assistant',
        name: expert?.name || 'AI专家',
        avatar: expertEmoji(expert?.type) || '🤖',
        color: expertColor(expert?.type) || '#6366f1',
        text: `✅ **${task.title}** 执行完成\n\n${task.result}`,
        status: 'done',
        phase: 'orchestration'
      })
    } else {
      task.status = 'failed'
      task.result = '执行过程中遇到问题，需要人工介入。'
      
      addOrchMessage({
        role: 'assistant',
        name: expert?.name || 'AI专家',
        avatar: expertEmoji(expert?.type) || '🤖',
        color: '#ef4444',
        text: `❌ **${task.title}** 执行失败\n\n执行过程中遇到异常，请检查任务配置或重新分配专家。`,
        status: 'failed',
        phase: 'orchestration'
      })
    }
    
    updateGanttLayout()
  }
  
  // 检查是否全部完成
  const allDone = taskOrchestration.subtasks.every(t => 
    ['completed', 'failed', 'archived'].includes(t.status)
  )
  
  if (allDone) {
    orchIsRunning.value = false
    const successCount = taskOrchestration.subtasks.filter(t => t.status === 'completed').length
    ElMessage.success(`任务执行完成：${successCount}/${taskOrchestration.subtasks.length} 成功`)
    
    addOrchMessage({
      role: 'system',
      name: '系统',
      avatar: '📊',
      color: '#10b981',
      text: `🎯 **任务编排完成**\n\n- 总任务数：${taskOrchestration.subtasks.length}\n- 成功完成：${successCount}\n- 失败：${taskOrchestration.subtasks.length - successCount}\n- 完成率：${orchProgress.value.percentage}%`,
      status: 'done',
      phase: 'orchestration'
    })
    
    addHistoryEvent('task', '任务编排完成', `完成率 ${orchProgress.value.percentage}%`)
  }
}

function resetAllTasks() {
  taskOrchestration.subtasks.forEach(task => {
    task.status = task.dependencies.length > 0 ? 'waiting' : 'pending'
    task.startTime = null
    task.endTime = null
    task.result = ''
  })
  orchIsRunning.value = false
  updateGanttLayout()
  ElMessage.success('所有任务已重置')
}
</script>

<style scoped>
/* ========== 全局布局 ========== */
.expert-workspace {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f1f5f9;
  font-family: 'Instrument Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
  color: #0f172a;
  overflow: hidden;
}

/* ========== 顶部 Header ========== */
.ws-header {
  height: 56px;
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(16px);
  border-bottom: 1px solid #e2e8f0;
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
  gap: 12px;
  min-width: 300px;
}
.ws-header-right {
  justify-content: flex-end;
}
.ws-header-divider {
  margin: 0 4px;
  border-color: #e2e8f0;
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
  background: #f1f5f9;
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
.ws-mode-icon { font-size: 14px; }

/* 全局搜索 */
.ws-global-search {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #64748b;
}
.ws-search-input {
  width: 260px;
}
.ws-search-kbd {
  font-size: 10px;
  color: #94a3b8;
  padding: 0 6px;
}

.ws-ai-btn { gap: 6px; }
.ws-icon-btn {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
}
.ws-icon-btn:hover { background: #f1f5f9; color: #6366f1; }
.ws-notif-badge { margin-right: 4px; }
.ws-avatar { cursor: pointer; }

/* ========== 主工作区三栏 ========== */
.ws-main {
  flex: 1;
  display: flex;
  overflow: hidden;
  position: relative;
  padding: 12px;
  gap: 12px;
  box-sizing: border-box;
}

/* ========== 侧边面板通用 ========== */
.ws-panel {
  background: white;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  transition: width 0.3s ease, all 0.3s ease;
  overflow: hidden;
  width: 280px;
}
.ws-panel.collapsed {
  width: 48px;
}

.ws-panel-header {
  height: 44px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px;
  border-bottom: 1px solid #f1f5f9;
  flex-shrink: 0;
}
.ws-panel-left .ws-panel-header { flex-direction: row; }
.ws-panel-right .ws-panel-header { flex-direction: row; }
.ws-panel-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
}
.ws-panel-icon { font-size: 15px; }
.ws-online-tag {
  margin-left: 6px;
  font-size: 10px;
  --el-tag-height: 18px;
}
.ws-panel-toggle {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: 6px;
  color: #94a3b8;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}
.ws-panel-toggle:hover {
  background: #f1f5f9;
  color: #6366f1;
}

.ws-panel-body {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  padding: 0;
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
.ws-collapsed-divider {
  margin: 8px 0;
  width: 28px;
  border-top: 1px solid #e2e8f0;
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
  color: #64748b;
}
.ws-collapsed-avatar:hover,
.ws-collapsed-icon-btn:hover {
  background: #f1f5f9;
  transform: scale(1.05);
}
.ws-collapsed-avatar-inner {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
}

/* ========== 左栏：专家联盟 ========== */
.ws-expert-filter {
  padding: 10px 12px;
  display: flex;
  gap: 8px;
  border-bottom: 1px solid #f1f5f9;
  flex-shrink: 0;
}
.ws-filter-select { width: 80px; flex-shrink: 0; }
.ws-filter-search { flex: 1; min-width: 0; }

.ws-expert-section {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-bottom: 1px solid #f1f5f9;
}
.ws-expert-section:last-child {
  border-bottom: none;
}
.ws-section-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  font-weight: 600;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 12px 12px 8px;
  flex-shrink: 0;
}
.ws-section-count {
  font-size: 11px;
  color: #94a3b8;
  text-transform: none;
  letter-spacing: 0;
}
.ws-add-btn {
  padding: 0 !important;
  color: #6366f1 !important;
  font-size: 12px;
  text-transform: none;
  letter-spacing: 0;
}

.ws-expert-scroll {
  flex: 1;
  min-height: 0;
  max-height: 280px;
}
.ws-session-scroll {
  flex: 1;
  min-height: 0;
  max-height: 180px;
}

/* 专家列表 */
.ws-expert-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  cursor: pointer;
  transition: all 0.15s;
  position: relative;
  border-bottom: 1px solid #f8fafc;
}
.ws-expert-item:hover {
  background: #f8fafc;
}
.ws-expert-item.active {
  background: #eef2ff;
}
.ws-expert-item.selected {
  background: #f0fdf4;
  border-left: 3px solid #10b981;
  padding-left: 9px;
}
.ws-expert-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}
.ws-expert-info { flex: 1; min-width: 0; }
.ws-expert-name {
  font-size: 13px;
  font-weight: 600;
  color: #1e293b;
  display: flex;
  align-items: center;
  gap: 6px;
  line-height: 1.3;
}
.ws-online-dot {
  width: 7px;
  height: 7px;
  background: #10b981;
  border-radius: 50%;
  flex-shrink: 0;
  box-shadow: 0 0 0 2px rgba(16, 185, 129, 0.2);
}
.ws-expert-role {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}
.ws-expert-tags {
  display: flex;
  gap: 4px;
  margin-top: 4px;
  flex-wrap: wrap;
}
.ws-cap-tag {
  font-size: 10px;
  padding: 1px 6px;
  background: #f1f5f9;
  color: #475569;
  border-radius: 4px;
  line-height: 1.4;
}
.ws-expert-check {
  color: #10b981;
  font-size: 16px;
}
.ws-expert-status {
  font-size: 10px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(16, 185, 129, 0.1);
  color: #10b981;
  flex-shrink: 0;
}
.ws-expert-status.status-busy {
  background: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
}
.ws-expert-status.status-offline {
  background: #f1f5f9;
  color: #94a3b8;
}
.ws-expert-status.status-idle {
  background: rgba(99, 102, 241, 0.1);
  color: #6366f1;
}

/* 会话列表 */
.ws-session-item {
  padding: 8px 12px;
  cursor: pointer;
  transition: all 0.15s;
  border-bottom: 1px solid #f8fafc;
}
.ws-session-item:hover { background: #f8fafc; }
.ws-session-item.active {
  background: #eef2ff;
  border-left: 3px solid #6366f1;
  padding-left: 9px;
}
.ws-session-title {
  font-size: 13px;
  font-weight: 500;
  color: #1e293b;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.3;
}
.ws-session-meta {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: #94a3b8;
  margin-bottom: 6px;
}
.ws-session-mode {
  display: flex;
}

/* 快捷工具 */
.ws-expert-section:last-child {
  padding-bottom: 12px;
}
.ws-tool-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
  padding: 0 12px 4px;
}
.ws-tool-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 12px 8px;
  border: 1px solid #e2e8f0;
  background: #fff;
  border-radius: 10px;
  cursor: pointer;
  font-size: 12px;
  color: #64748b;
  transition: all 0.2s;
}
.ws-tool-btn:hover {
  border-color: #6366f1;
  color: #6366f1;
  background: #faf5ff;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.12);
}
.ws-tool-btn.active {
  border-color: #6366f1;
  background: #eef2ff;
  color: #6366f1;
}
.ws-tool-icon { font-size: 20px; }

/* ========== 中栏：图谱画布 ========== */
.ws-center {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  position: relative;
  background: white;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  overflow: hidden;
}

/* 画布工具栏 */
.ws-canvas-toolbar {
  height: 44px;
  background: #fff;
  border-bottom: 1px solid #f1f5f9;
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
  transition: all 0.2s;
}
.ws-canvas-tool:hover {
  background: #f1f5f9;
  color: #6366f1;
}
.ws-canvas-tool.active {
  background: #eef2ff;
  color: #6366f1;
}
.ws-tool-divider {
  width: 1px;
  height: 20px;
  background: #e2e8f0;
  margin: 0 4px;
}

.ws-layout-switcher {
  display: flex;
  background: #f1f5f9;
  border-radius: 8px;
  padding: 3px;
  gap: 2px;
}
.ws-layout-btn {
  padding: 4px 10px;
  border: none;
  background: transparent;
  border-radius: 6px;
  font-size: 12px;
  color: #64748b;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  align-items: center;
  gap: 4px;
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
    radial-gradient(ellipse at 30% 20%, rgba(99, 102, 241, 0.03) 0%, transparent 50%),
    radial-gradient(ellipse at 70% 80%, rgba(6, 182, 212, 0.03) 0%, transparent 50%),
    #f8fafc;
}
.ws-graph-svg {
  width: 100%;
  height: 100%;
  cursor: grab;
  display: block;
}
.ws-graph-svg:active { cursor: grabbing; }

.ws-node {
  cursor: pointer;
  transition: transform 0.15s ease;
}
.ws-node:hover {
  transform: scale(1.15);
}
.ws-node.selected circle:nth-of-type(2) {
  stroke: #6366f1;
  stroke-width: 3;
  stroke-dasharray: 4 2;
}
.ws-edge {
  transition: stroke-opacity 0.2s;
}
.ws-edge.highlight {
  stroke-width: 2.5 !important;
}

/* 节点信息卡片 */
.ws-node-info-card {
  position: absolute;
  width: 260px;
  background: rgba(255, 255, 255, 0.98);
  backdrop-filter: blur(12px);
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(15, 23, 42, 0.12);
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
  border-bottom: 1px solid #f1f5f9;
  position: relative;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.04), rgba(6, 182, 212, 0.04));
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
  flex-shrink: 0;
}
.ws-node-info-head-text {
  flex: 1;
  min-width: 0;
}
.ws-node-info-title {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
  line-height: 1.3;
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
.ws-node-info-desc {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid #f1f5f9;
  font-size: 12px;
  color: #475569;
  line-height: 1.5;
}
.ws-node-info-actions {
  display: flex;
  gap: 8px;
  padding: 0 14px 12px;
}
.ws-node-info-actions .el-button { flex: 1; }

/* 加载遮罩 */
.ws-graph-loading {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  background: rgba(248, 250, 252, 0.9);
  color: #64748b;
  font-size: 13px;
  z-index: 5;
}
.ws-loading-icon {
  font-size: 24px;
  color: #6366f1;
}

/* 底部协作栏 */
.ws-collab-bar {
  border-top: 1px solid #e2e8f0;
  background: #fff;
  flex-shrink: 0;
  transition: all 0.3s ease;
  display: flex;
  flex-direction: column;
}
.ws-collab-bar.is-running {
  border-top-color: #f59e0b;
}
.ws-collab-header {
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  cursor: pointer;
  transition: background 0.2s;
  flex-shrink: 0;
}
.ws-collab-header:hover { background: #f8fafc; }
.ws-collab-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}
.ws-pulse-icon {
  color: #f59e0b;
  animation: pulse 1.5s infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
.ws-running-tag {
  margin-left: 4px;
  --el-tag-height: 18px;
}
.ws-collab-count {
  font-size: 11px;
  color: #64748b;
  font-weight: 400;
  background: #f1f5f9;
  padding: 2px 8px;
  border-radius: 10px;
  margin-left: 4px;
}
.ws-collab-toggle {
  color: #94a3b8;
  display: flex;
  align-items: center;
}

.ws-collab-body {
  display: flex;
  flex-direction: column;
  height: 260px;
  padding: 0;
}

/* 联盟阶段进度 */
.ws-alliance-phases {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 10px 16px;
  gap: 4px;
  border-bottom: 1px solid #f1f5f9;
  background: #fafbfc;
  overflow-x: auto;
}
.ws-phase-step {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border-radius: 6px;
  font-size: 11px;
  color: #94a3b8;
  white-space: nowrap;
  transition: all 0.3s;
}
.ws-phase-step.done {
  color: #10b981;
}
.ws-phase-step.active {
  color: #6366f1;
  background: #eef2ff;
  font-weight: 600;
}
.ws-phase-step-dot {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #e2e8f0;
  color: white;
  font-size: 10px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ws-phase-step.done .ws-phase-step-dot {
  background: #10b981;
}
.ws-phase-step.active .ws-phase-step-dot {
  background: #6366f1;
  animation: stepPulse 1s infinite;
}
@keyframes stepPulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(99, 102, 241, 0.4); }
  50% { box-shadow: 0 0 0 4px rgba(99, 102, 241, 0); }
}

/* 消息区 */
.ws-collab-messages {
  flex: 1;
  overflow: hidden;
  padding: 12px 16px;
}
.ws-collab-msg {
  display: flex;
  gap: 10px;
  margin-bottom: 12px;
  animation: msgIn 0.3s ease;
}
@keyframes msgIn {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: translateY(0); }
}
.ws-collab-msg.user {
  flex-direction: row-reverse;
}
.ws-collab-msg.system {
  justify-content: center;
}
.ws-collab-msg-avatar {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  flex-shrink: 0;
  color: white;
  font-weight: 600;
}
.ws-collab-msg-content {
  max-width: 70%;
  min-width: 0;
}
.ws-collab-msg.system .ws-collab-msg-content {
  max-width: 90%;
  text-align: center;
}
.ws-collab-msg-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: #64748b;
  margin-bottom: 4px;
  flex-wrap: wrap;
}
.ws-collab-msg.user .ws-collab-msg-meta { justify-content: flex-end; }
.ws-collab-msg-name { font-weight: 600; color: #475569; }
.ws-collab-msg-phase { line-height: 1; }
.ws-collab-msg-time { font-size: 10px; }
.ws-collab-msg-text {
  background: #f1f5f9;
  padding: 8px 12px;
  border-radius: 10px;
  font-size: 13px;
  line-height: 1.6;
  color: #1e293b;
  word-break: break-word;
}
.ws-collab-msg.user .ws-collab-msg-text {
  background: linear-gradient(135deg, #6366f1, #06b6d4);
  color: white;
}
.ws-collab-msg.system .ws-collab-msg-text {
  background: #f8fafc;
  color: #64748b;
  font-size: 12px;
  display: inline-block;
}

/* 打字动画 */
.ws-typing .ws-typing-dots {
  display: flex;
  gap: 4px;
  padding: 10px 12px;
  background: #f1f5f9;
  border-radius: 10px;
  width: fit-content;
}
.ws-typing-dots span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #94a3b8;
  animation: typingBounce 1.4s infinite ease-in-out;
}
.ws-typing-dots span:nth-child(1) { animation-delay: 0s; }
.ws-typing-dots span:nth-child(2) { animation-delay: 0.2s; }
.ws-typing-dots span:nth-child(3) { animation-delay: 0.4s; }
@keyframes typingBounce {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-6px); }
}

/* 输入区 */
.ws-collab-input-area {
  border-top: 1px solid #f1f5f9;
  padding: 8px 16px 12px;
  flex-shrink: 0;
}
.ws-collab-input-tools {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-bottom: 6px;
}
.ws-tool-mini-btn {
  width: 28px;
  height: 28px;
  padding: 0 !important;
  color: #94a3b8 !important;
}
.ws-tool-mini-btn:hover {
  color: #6366f1 !important;
  background: #f1f5f9 !important;
}
.ws-mode-select {
  margin-left: auto;
  width: 110px;
}
.ws-collab-input-row {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}
.ws-collab-input-field {
  flex: 1;
}
.ws-collab-input-field :deep(.el-textarea__inner) {
  font-size: 13px;
  padding: 8px 12px;
  resize: none;
}
.ws-send-btn {
  height: auto;
  padding: 10px 16px;
  gap: 6px;
}
.ws-stop-btn {
  height: auto;
  padding: 10px 12px;
}

/* ========== 右栏：知识库 ========== */
.ws-panel-right {
  width: 300px;
}

.ws-kb-tabs {
  display: flex;
  background: #f1f5f9;
  border-radius: 8px;
  padding: 3px;
  margin: 12px;
  gap: 2px;
  flex-shrink: 0;
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
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}
.ws-kb-tab:hover { color: #6366f1; }
.ws-kb-tab.active {
  background: white;
  color: #6366f1;
  font-weight: 600;
  box-shadow: 0 1px 2px rgba(0,0,0,0.06);
}

.ws-kb-search {
  padding: 0 12px 12px;
  flex-shrink: 0;
}

.ws-kb-docs {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

/* 分类列表 */
.ws-doc-categories {
  padding: 0 8px;
  flex-shrink: 0;
}
.ws-doc-category {
  margin-bottom: 2px;
}
.ws-doc-category-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  color: #475569;
  transition: all 0.15s;
}
.ws-doc-category-header:hover {
  background: #f1f5f9;
}
.ws-doc-category.active .ws-doc-category-header {
  background: #eef2ff;
  color: #6366f1;
  font-weight: 600;
}
.ws-cat-name {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ws-cat-count {
  font-size: 11px;
  color: #94a3b8;
  background: #f1f5f9;
  padding: 1px 6px;
  border-radius: 10px;
}
.ws-doc-category.active .ws-cat-count {
  background: rgba(99, 102, 241, 0.15);
  color: #6366f1;
}

.ws-kb-divider {
  margin: 8px 12px;
  border-color: #f1f5f9;
}

/* 文档列表 */
.ws-doc-scroll {
  flex: 1;
  min-height: 0;
  padding: 0 8px;
}
.ws-doc-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
  margin-bottom: 2px;
}
.ws-doc-item:hover { background: #f8fafc; }
.ws-doc-item.active {
  background: #eef2ff;
  border: 1px solid #c7d2fe;
}
.ws-doc-item.linked {
  position: relative;
}
.ws-doc-icon { font-size: 20px; flex-shrink: 0; }
.ws-doc-info { flex: 1; min-width: 0; }
.ws-doc-name {
  font-size: 13px;
  font-weight: 500;
  color: #1e293b;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.3;
}
.ws-doc-meta {
  font-size: 11px;
  color: #94a3b8;
  margin-top: 3px;
}
.ws-doc-badge {
  flex-shrink: 0;
  color: #06b6d4;
  font-size: 14px;
}

/* 标签云 */
.ws-kb-tags {
  flex: 1;
  overflow-y: auto;
  padding: 0 12px;
}
.ws-tag-section-title {
  font-size: 11px;
  font-weight: 600;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 12px;
}
.ws-tag-cloud {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-start;
}
.ws-tag-cloud-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  background: #f1f5f9;
  border: 1px solid #e2e8f0;
  border-radius: 20px;
  cursor: pointer;
  transition: all 0.2s;
  color: #6366f1;
}
.ws-tag-cloud-item:hover {
  background: #6366f1;
  border-color: #6366f1;
  color: white;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.2);
}
.ws-tag-count {
  font-size: 10px;
  opacity: 0.7;
}

/* 版本列表 */
.ws-kb-versions {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}
.ws-version-current-doc {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 12px;
  background: #f8fafc;
  font-size: 12px;
  font-weight: 600;
  color: #475569;
  border-bottom: 1px solid #f1f5f9;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.ws-version-scroll {
  flex: 1;
  min-height: 0;
  padding: 8px 12px;
}
.ws-version-item {
  padding: 10px 12px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  border-left: 3px solid #94a3b8;
  margin-bottom: 8px;
  transition: all 0.2s;
}
.ws-version-item:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}
.ws-version-item.latest {
  border-left-color: #10b981;
  background: #f0fdf4;
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
  background: #f1f5f9;
  color: #64748b;
}
.ws-version-item.latest .ws-version-badge {
  background: rgba(16, 185, 129, 0.12);
  color: #10b981;
}
.ws-version-time { font-size: 11px; color: #94a3b8; }
.ws-version-label {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 4px;
}
.ws-version-author { font-size: 11px; color: #64748b; }

/* 知识库操作按钮 */
.ws-kb-actions {
  display: flex;
  gap: 8px;
  padding: 12px;
  border-top: 1px solid #f1f5f9;
  flex-shrink: 0;
}
.ws-kb-action-btn { flex: 1; gap: 4px; }
.ws-kb-upload { flex: 1; }
.ws-kb-upload .el-button { width: 100%; gap: 4px; }

/* ========== AI 助手浮窗 ========== */
.ws-ai-assistant {
  position: absolute;
  right: 320px;
  bottom: 280px;
  width: 300px;
  background: rgba(255, 255, 255, 0.98);
  backdrop-filter: blur(12px);
  border: 1px solid #e2e8f0;
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
  border-bottom: 1px solid #f1f5f9;
}
.ws-ai-icon { font-size: 22px; }
.ws-ai-title {
  flex: 1;
  font-weight: 700;
  font-size: 14px;
  color: #0f172a;
}
.ws-ai-close {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  border-radius: 6px;
  color: #94a3b8;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.ws-ai-close:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}
.ws-ai-body { padding: 14px 16px; }
.ws-ai-suggestions { margin-bottom: 14px; }
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
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 12px;
  background: #f8fafc;
  border: 1px solid #e2e8f0;
  border-radius: 8px;
  font-size: 12px;
  color: #0f172a;
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
}
.ws-suggest-icon { font-size: 16px; }
.ws-ai-suggest-btn:hover {
  border-color: #6366f1;
  background: #faf5ff;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(99, 102, 241, 0.12);
}

.ws-ai-cap-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ws-cap-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #475569;
}
.ws-cap-item .el-icon {
  color: #10b981;
  font-size: 14px;
}

/* ========== 滚动条样式 ========== */
:deep(.el-scrollbar__thumb) {
  background: rgba(99, 102, 241, 0.2);
  border-radius: 3px;
}
:deep(.el-scrollbar__thumb:hover) {
  background: rgba(99, 102, 241, 0.4);
}
:deep(.el-scrollbar__bar.is-vertical) {
  width: 6px;
}

/* ========== 响应式适配 ========== */
@media (max-width: 1400px) {
  .ws-panel-right {
    width: 280px;
  }
  .ws-panel {
    width: 260px;
  }
}

@media (max-width: 1200px) {
  .ws-header-left {
    min-width: auto;
  }
  .ws-header-right {
    min-width: auto;
  }
  .ws-search-input {
    width: 180px;
  }
  .ws-mode-tab .ws-mode-label {
    display: none;
  }
  .ws-mode-tab {
    padding: 6px 10px;
  }
}

/* ========== 智能匹配按钮 ========== */
.ws-section-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ws-smart-match-btn {
  padding: 0 6px !important;
  font-size: 11px !important;
  color: #6366f1 !important;
}
.ws-smart-match-btn:hover {
  color: #4338ca !important;
}

/* ========== 通用对话框样式 ========== */
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.form-hint {
  margin-left: 8px;
  font-size: 12px;
  color: #94a3b8;
}

/* ========== 辩论对话框 ========== */
.debate-dialog :deep(.el-dialog__body) {
  padding-top: 8px;
}
.debate-expert-picker {
  width: 100%;
}
.debate-expert-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  max-height: 140px;
  overflow-y: auto;
  padding: 4px;
  background: #f8fafc;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}
.debate-expert-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px 5px 5px;
  border-radius: 999px;
  background: #fff;
  border: 1px solid #e2e8f0;
  cursor: pointer;
  transition: all 0.2s;
  user-select: none;
}
.debate-expert-chip:hover {
  border-color: #c7d2fe;
  background: #f5f3ff;
}
.debate-expert-chip.selected {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.1), rgba(14, 165, 233, 0.08));
  border-color: #6366f1;
}
.chip-avatar {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 12px;
}
.chip-name {
  font-size: 12px;
  font-weight: 500;
  color: #334155;
}
.chip-check {
  color: #6366f1;
  font-size: 14px;
}
.debate-expert-count {
  margin-top: 6px;
  font-size: 12px;
  color: #64748b;
}
.debate-expert-count b {
  color: #6366f1;
  font-weight: 700;
}

.debate-mode-picker {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  width: 100%;
}
.debate-mode-card {
  padding: 14px 12px;
  border-radius: 10px;
  border: 2px solid #e2e8f0;
  background: #f8fafc;
  cursor: pointer;
  transition: all 0.25s;
  text-align: center;
}
.debate-mode-card:hover {
  border-color: #c7d2fe;
  background: #f5f3ff;
}
.debate-mode-card.active {
  border-color: #6366f1;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.08), rgba(14, 165, 233, 0.05));
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.15);
}
.debate-mode-card .mode-icon {
  font-size: 24px;
  margin-bottom: 6px;
}
.debate-mode-card .mode-name {
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
  margin-bottom: 4px;
}
.debate-mode-card .mode-desc {
  font-size: 11px;
  color: #64748b;
  line-height: 1.4;
}

/* ========== 多专家咨询对话框 ========== */
.multi-consult-dialog :deep(.el-dialog__body) {
  padding-top: 8px;
  max-height: 70vh;
  overflow-y: auto;
}
.consult-expert-picker {
  width: 100%;
}
.consult-expert-list {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  max-height: 140px;
  overflow-y: auto;
  padding: 4px;
  background: #f8fafc;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
}
.consult-expert-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px 5px 5px;
  border-radius: 999px;
  background: #fff;
  border: 1px solid #e2e8f0;
  cursor: pointer;
  transition: all 0.2s;
  user-select: none;
}
.consult-expert-chip:hover {
  border-color: #c7d2fe;
  background: #f5f3ff;
}
.consult-expert-chip.selected {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.1), rgba(16, 185, 129, 0.08));
  border-color: #10b981;
}
.consult-expert-count {
  margin-top: 6px;
  font-size: 12px;
  color: #64748b;
}
.consult-expert-count b {
  color: #10b981;
  font-weight: 700;
}

.consult-mode-group :deep(.el-radio-button__inner) {
  padding: 8px 16px;
}
.mode-icon-inline {
  margin-right: 4px;
}
.mode-hint {
  font-size: 11px;
  color: #94a3b8;
  margin-left: 2px;
}

/* 咨询结果 */
.consult-results-section {
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid #e2e8f0;
}
.results-section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.results-section-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
}

/* 对比视图 */
.compare-view .compare-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 10px;
}
.compare-card {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.compare-card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  background: #f8fafc;
  border-top: 3px solid #6366f1;
}
.compare-expert {
  display: flex;
  align-items: center;
  gap: 8px;
}
.compare-avatar {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  font-size: 14px;
}
.compare-name {
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
}
.compare-card-body {
  padding: 12px;
  flex: 1;
  max-height: 200px;
  overflow-y: auto;
}
.compare-content {
  font-size: 12.5px;
  line-height: 1.7;
  color: #334155;
  white-space: pre-wrap;
  word-break: break-word;
}

/* 列表视图 */
.list-view {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.result-item-card {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  overflow: hidden;
}
.result-item-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: #f8fafc;
  border-bottom: 1px solid #e2e8f0;
}
.result-avatar {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  display: grid;
  place-items: center;
  font-size: 13px;
}
.result-name {
  font-size: 13px;
  font-weight: 700;
  color: #1e293b;
}
.result-duration {
  margin-left: auto;
  font-size: 11px;
  color: #94a3b8;
}
.result-item-body {
  padding: 12px;
  font-size: 13px;
  line-height: 1.75;
  color: #334155;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 180px;
  overflow-y: auto;
}

/* ========== 智能匹配对话框 ========== */
.smart-route-dialog :deep(.el-dialog__body) {
  padding-top: 8px;
}
.smart-route-intro {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.06), rgba(16, 185, 129, 0.05));
  border-radius: 10px;
  margin-bottom: 16px;
  border: 1px solid #e0e7ff;
}
.intro-icon {
  font-size: 32px;
  flex-shrink: 0;
}
.intro-title {
  font-size: 15px;
  font-weight: 700;
  color: #1e1b4b;
  margin-bottom: 2px;
}
.intro-desc {
  font-size: 12px;
  color: #6366f1;
}

.smart-route-action {
  display: flex;
  justify-content: center;
  margin: 8px 0 16px;
}
.smart-route-btn {
  width: 100%;
  font-weight: 600;
  padding: 12px;
}

/* 匹配结果 */
.smart-route-results {
  margin-top: 8px;
}
.route-result-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid #e2e8f0;
}
.route-result-title {
  font-size: 13.5px;
  font-weight: 700;
  color: #0f172a;
}
.route-expert-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 300px;
  overflow-y: auto;
}
.route-expert-item {
  display: grid;
  grid-template-columns: 24px 40px 1fr auto auto;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: #f8fafc;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  transition: all 0.2s;
}
.route-expert-item:hover {
  border-color: #c7d2fe;
  background: #f5f3ff;
}
.route-rank {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  font-size: 11px;
  font-weight: 800;
  display: grid;
  place-items: center;
}
.route-avatar {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 18px;
}
.route-info {
  min-width: 0;
}
.route-name {
  font-size: 13px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 2px;
}
.route-type {
  font-size: 11px;
  color: #64748b;
  margin-bottom: 2px;
}
.route-reason {
  font-size: 11px;
  color: #6366f1;
  line-height: 1.4;
}
.route-score {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}
.score-ring {
  width: 44px;
  height: 44px;
  border-radius: 50%;
  background: conic-gradient(
    #10b981 calc(var(--score) * 360deg),
    #e2e8f0 calc(var(--score) * 360deg)
  );
  display: grid;
  place-items: center;
  position: relative;
}
.score-ring::before {
  content: '';
  position: absolute;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #f8fafc;
}
.score-ring span {
  position: relative;
  z-index: 1;
  font-size: 10.5px;
  font-weight: 700;
  color: #1e293b;
}
.score-label {
  font-size: 10px;
  color: #64748b;
}
.route-select-btn {
  flex-shrink: 0;
}

.route-actions-footer {
  margin-top: 12px;
  padding-top: 10px;
  border-top: 1px dashed #e2e8f0;
  display: flex;
  justify-content: center;
}

.smart-route-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 30px 20px;
  color: #64748b;
  font-size: 13px;
}
.loading-spinner {
  font-size: 28px;
  color: #6366f1;
}

/* ========== 玻璃拟态基础样式 ========== */
.glass-card {
  background: rgba(255, 255, 255, 0.75);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.5);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.08);
}

.gradient-avatar {
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  color: white;
  font-weight: 600;
  position: relative;
  flex-shrink: 0;
}

.gradient-btn {
  background: linear-gradient(135deg, #7c3aed, #06b6d4) !important;
  border: none !important;
  color: white !important;
  transition: all 0.3s ease !important;
  box-shadow: 0 4px 15px rgba(124, 58, 237, 0.3) !important;
}
.gradient-btn:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 20px rgba(124, 58, 237, 0.4) !important;
}
.gradient-btn:active {
  transform: translateY(0);
}

.gradient-tag {
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.1), rgba(6, 182, 212, 0.1)) !important;
  border: 1px solid rgba(124, 58, 237, 0.2) !important;
  color: #7c3aed !important;
}

/* ========== 顶部 Header 玻璃拟态升级 ========== */
.glass-header {
  background: rgba(255, 255, 255, 0.85) !important;
  backdrop-filter: blur(24px) !important;
  -webkit-backdrop-filter: blur(24px) !important;
  border-bottom: 1px solid rgba(255, 255, 255, 0.6) !important;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.05) !important;
  position: relative;
  overflow: hidden;
}

.ws-header-gradient-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, #7c3aed, #06b6d4, #10b981, #f59e0b, #7c3aed);
  background-size: 200% 100%;
  animation: gradientMove 8s ease infinite;
}

@keyframes gradientMove {
  0%, 100% { background-position: 0% 50%; }
  50% { background-position: 100% 50%; }
}

.ws-logo-icon-wrap {
  width: 32px;
  height: 32px;
  border-radius: 10px;
  background: linear-gradient(135deg, #7c3aed, #06b6d4);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px rgba(124, 58, 237, 0.3);
}

.ws-project-selector {
  position: relative;
}

.glass-tabs {
  background: rgba(241, 245, 249, 0.8) !important;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.5);
}

.ws-mode-tab {
  position: relative;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
.ws-mode-tab .ws-mode-icon-wrap {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0.7;
  transition: all 0.3s ease;
}
.ws-mode-tab .ws-mode-icon {
  color: white;
  font-size: 13px;
}
.ws-mode-tab:hover {
  transform: translateY(-2px);
}
.ws-mode-tab:hover .ws-mode-icon-wrap {
  opacity: 1;
  transform: scale(1.1);
}
.ws-mode-tab.active {
  background: white !important;
  color: #7c3aed !important;
  box-shadow: 0 4px 12px rgba(124, 58, 237, 0.15) !important;
}
.ws-mode-tab.active .ws-mode-icon-wrap {
  opacity: 1;
}
.ws-mode-shortcut {
  font-size: 10px;
  color: #94a3b8;
  padding: 1px 4px;
  background: rgba(148, 163, 184, 0.1);
  border-radius: 4px;
  margin-left: 2px;
  opacity: 0;
  transition: opacity 0.2s;
}
.ws-mode-tab:hover .ws-mode-shortcut {
  opacity: 1;
}

.glass-search {
  background: rgba(241, 245, 249, 0.6);
  backdrop-filter: blur(10px);
  border-radius: 10px;
  padding: 0 8px;
  border: 1px solid rgba(255, 255, 255, 0.5);
  transition: all 0.3s ease;
}
.glass-search:focus-within {
  background: white;
  box-shadow: 0 0 0 3px rgba(124, 58, 237, 0.1);
  border-color: rgba(124, 58, 237, 0.3);
}
.search-icon {
  color: #94a3b8;
}

.ws-user-avatar-wrap {
  position: relative;
}
.ws-avatar-online-dot {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 10px;
  height: 10px;
  background: #10b981;
  border: 2px solid white;
  border-radius: 50%;
}

/* ========== KPI 指标卡 ========== */
.ws-kpi-row {
  display: flex;
  gap: 16px;
  padding: 12px 12px 0 12px;
  flex-shrink: 0;
}

.ws-kpi-card {
  flex: 1;
  border-radius: 16px;
  padding: 16px 20px;
  display: flex;
  align-items: center;
  gap: 14px;
  cursor: pointer;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
  overflow: hidden;
}
.ws-kpi-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.12);
}
.ws-kpi-icon {
  width: 48px;
  height: 48px;
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 22px;
  flex-shrink: 0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}
.ws-kpi-info {
  flex: 1;
  min-width: 0;
}
.ws-kpi-value {
  font-size: 24px;
  font-weight: 700;
  color: #0f172a;
  line-height: 1.2;
}
.ws-kpi-label {
  font-size: 13px;
  color: #64748b;
  margin-top: 2px;
}
.ws-kpi-trend {
  display: flex;
  align-items: center;
  gap: 2px;
  font-size: 12px;
  font-weight: 600;
  padding: 4px 8px;
  border-radius: 20px;
}
.ws-kpi-trend.up {
  color: #10b981;
  background: rgba(16, 185, 129, 0.1);
}
.ws-kpi-trend.down {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}
.ws-kpi-gradient-bar {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  height: 3px;
  opacity: 0.8;
}

/* ========== 专家卡片样式升级 ========== */
.expert-card {
  border-radius: 12px !important;
  padding: 12px !important;
  margin: 0 8px 8px 8px !important;
  background: linear-gradient(135deg, rgba(255,255,255,0.9), rgba(248,250,252,0.9)) !important;
  border: 1px solid rgba(226, 232, 240, 0.8) !important;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
  position: relative;
  overflow: hidden;
}
.expert-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, transparent, rgba(124, 58, 237, 0.3), transparent);
  opacity: 0;
  transition: opacity 0.3s;
}
.expert-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
  border-color: rgba(124, 58, 237, 0.2) !important;
}
.expert-card:hover::before {
  opacity: 1;
}
.expert-card.active {
  border-color: rgba(124, 58, 237, 0.4) !important;
  box-shadow: 0 4px 16px rgba(124, 58, 237, 0.15) !important;
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.05), rgba(6, 182, 212, 0.05)) !important;
}
.expert-card.selected {
  border-color: rgba(16, 185, 129, 0.4) !important;
  background: linear-gradient(135deg, rgba(16, 185, 129, 0.05), rgba(20, 184, 166, 0.05)) !important;
}

.ws-expert-avatar {
  width: 44px !important;
  height: 44px !important;
  font-size: 20px !important;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}

.ws-expert-status-dot {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  border: 2px solid white;
}
.ws-expert-status-dot.dot-active { background: #10b981; box-shadow: 0 0 6px rgba(16, 185, 129, 0.5); }
.ws-expert-status-dot.dot-busy { background: #f59e0b; }
.ws-expert-status-dot.dot-offline { background: #94a3b8; }
.ws-expert-status-dot.dot-idle { background: #06b6d4; }

.ws-expert-name-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.ws-expert-rate {
  font-size: 11px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 10px;
  background: rgba(124, 58, 237, 0.1);
}

.ws-expert-status-badge {
  font-size: 10px;
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 500;
}
.badge-active { background: rgba(16, 185, 129, 0.1); color: #10b981; }
.badge-busy { background: rgba(245, 158, 11, 0.1); color: #f59e0b; }
.badge-offline { background: rgba(148, 163, 184, 0.1); color: #64748b; }
.badge-idle { background: rgba(6, 182, 212, 0.1); color: #06b6d4; }

/* ========== 快捷工具卡片样式 ========== */
.tool-card {
  background: white !important;
  border: 1px solid #e2e8f0 !important;
  border-radius: 12px !important;
  padding: 12px 8px !important;
  flex-direction: column !important;
  gap: 8px !important;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1) !important;
}
.tool-card:hover {
  transform: translateY(-3px) !important;
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.1) !important;
  border-color: rgba(124, 58, 237, 0.3) !important;
}
.tool-card.active {
  border-color: rgba(124, 58, 237, 0.5) !important;
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.05), rgba(6, 182, 212, 0.05)) !important;
}
.ws-tool-icon-wrap {
  width: 40px;
  height: 40px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  box-shadow: 0 4px 10px rgba(0, 0, 0, 0.1);
  transition: transform 0.3s ease;
}
.tool-card:hover .ws-tool-icon-wrap {
  transform: scale(1.1) rotate(-5deg);
}

/* ========== 协作区增强样式 ========== */
.ws-collab-bar {
  position: relative;
  overflow: hidden;
}
.ws-collab-gradient-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, #7c3aed, #06b6d4, #10b981);
  opacity: 0.8;
}

.ws-collab-title-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.1), rgba(6, 182, 212, 0.1));
  display: flex;
  align-items: center;
  justify-content: center;
  color: #7c3aed;
}

.ws-collab-title-text {
  font-weight: 600;
  background: linear-gradient(135deg, #7c3aed, #06b6d4);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.ws-typing-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #7c3aed;
  padding: 2px 10px;
  background: rgba(124, 58, 237, 0.08);
  border-radius: 12px;
  margin-left: 8px;
}
.typing-dots-mini {
  display: inline-flex;
  gap: 2px;
}
.typing-dots-mini i {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #7c3aed;
  animation: typingBounce 1.4s infinite ease-in-out both;
}
.typing-dots-mini i:nth-child(1) { animation-delay: -0.32s; }
.typing-dots-mini i:nth-child(2) { animation-delay: -0.16s; }

@keyframes typingBounce {
  0%, 80%, 100% { transform: scale(0.6); opacity: 0.5; }
  40% { transform: scale(1); opacity: 1; }
}

.ws-header-action-btn {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  border-radius: 8px;
  color: #64748b;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s;
}
.ws-header-action-btn:hover {
  background: rgba(124, 58, 237, 0.1);
  color: #7c3aed;
}
.ws-header-action-btn.active {
  background: rgba(124, 58, 237, 0.15);
  color: #7c3aed;
}

.ws-collab-header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* ========== 阶段进度条 ========== */
.ws-phase-progress-bar {
  display: flex;
  justify-content: space-between;
  padding: 12px 20px 16px;
  background: linear-gradient(135deg, rgba(124, 58, 237, 0.03), rgba(6, 182, 212, 0.03));
  border-bottom: 1px solid rgba(226, 232, 240, 0.5);
}

.ws-phase-item {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  position: relative;
  cursor: pointer;
  transition: transform 0.2s;
}
.ws-phase-item:hover {
  transform: translateY(-2px);
}
.ws-phase-dot-wrapper {
  display: flex;
  align-items: center;
  width: 100%;
  justify-content: center;
  position: relative;
}
.ws-phase-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: #e2e8f0;
  color: #64748b;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  transition: all 0.3s ease;
  z-index: 2;
}
.ws-phase-item.active .ws-phase-dot {
  background: linear-gradient(135deg, #7c3aed, #06b6d4);
  color: white;
  box-shadow: 0 4px 12px rgba(124, 58, 237, 0.4);
  transform: scale(1.1);
}
.ws-phase-item.done .ws-phase-dot {
  background: linear-gradient(135deg, #10b981, #14b8a6);
  color: white;
}
.ws-phase-connector {
  position: absolute;
  top: 50%;
  left: 50%;
  width: calc(100% - 28px);
  height: 3px;
  background: #e2e8f0;
  transform: translateY(-50%);
  z-index: 1;
}
.ws-phase-connector.filled {
  background: linear-gradient(90deg, #10b981, #14b8a6);
}
.ws-phase-label {
  font-size: 12px;
  color: #64748b;
  font-weight: 500;
  transition: all 0.3s;
}
.ws-phase-item.active .ws-phase-label {
  color: #7c3aed;
  font-weight: 600;
}
.ws-phase-item.done .ws-phase-label {
  color: #10b981;
}

/* ========== 协作 Tab ========== */
.ws-collab-tabs {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  border-bottom: 1px solid #f1f5f9;
  background: rgba(248, 250, 252, 0.5);
}
.ws-collab-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 12px 16px;
  border: none;
  background: transparent;
  color: #64748b;
  font-size: 13px;
  cursor: pointer;
  position: relative;
  transition: all 0.3s ease;
}
.ws-collab-tab:hover {
  color: #7c3aed;
}
.ws-collab-tab.active {
  color: #7c3aed;
  font-weight: 600;
}
.ws-collab-tab.active::after {
  content: '';
  position: absolute;
  bottom: 0;
  left: 50%;
  transform: translateX(-50%);
  width: 24px;
  height: 3px;
  background: linear-gradient(90deg, #7c3aed, #06b6d4);
  border-radius: 2px;
}
.ws-collab-tab-icon {
  font-size: 14px;
}
.ws-tab-badge {
  margin-left: 4px;
}
.ws-collab-tabs-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* ========== 协作成员头像组 ========== */
.ws-collab-members {
  display: flex;
  align-items: center;
}
.ws-member-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  color: white;
  border: 2px solid white;
  margin-left: -6px;
  cursor: pointer;
  transition: transform 0.2s;
  position: relative;
}
.ws-member-avatar:first-child {
  margin-left: 0;
}
.ws-member-avatar:hover {
  transform: translateY(-2px) scale(1.1);
  z-index: 20 !important;
}
.more-avatar {
  background: #e2e8f0 !important;
  color: #64748b !important;
  font-size: 10px;
  font-weight: 600;
}
.ws-member-status-dot {
  position: absolute;
  bottom: -1px;
  right: -1px;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  border: 1.5px solid white;
}
.ws-member-status-dot.status-active { background: #10b981; }
.ws-member-status-dot.status-busy { background: #f59e0b; }
.ws-member-status-dot.status-offline { background: #94a3b8; }

/* ========== Tab 内容通用 ========== */
.ws-tab-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

/* ========== 文件栏 ========== */
.ws-files-bar {
  margin: 8px 16px 0;
  background: rgba(124, 58, 237, 0.04);
  border-radius: 10px;
  border: 1px solid rgba(124, 58, 237, 0.1);
  overflow: hidden;
}
.ws-files-bar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  cursor: pointer;
}
.ws-files-bar-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: #7c3aed;
}
.ws-files-bar-toggle {
  font-size: 12px !important;
  color: #7c3aed !important;
}
.ws-files-list {
  padding: 0 8px 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.ws-file-card {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  background: white;
  border-radius: 8px;
  border: 1px solid #e2e8f0;
  cursor: pointer;
  transition: all 0.2s;
  min-width: 180px;
}
.ws-file-card:hover {
  border-color: #7c3aed;
  box-shadow: 0 2px 8px rgba(124, 58, 237, 0.1);
}
.ws-file-icon {
  width: 28px;
  height: 28px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  background: #f1f5f9;
  flex-shrink: 0;
}
.file-pdf { background: rgba(239, 68, 68, 0.1); }
.file-doc { background: rgba(59, 130, 246, 0.1); }
.file-image { background: rgba(16, 185, 129, 0.1); }
.file-excel { background: rgba(34, 197, 94, 0.1); }
.ws-file-info {
  flex: 1;
  min-width: 0;
}
.ws-file-name {
  font-size: 12px;
  font-weight: 500;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.ws-file-meta {
  font-size: 10px;
  color: #94a3b8;
}
.ws-file-download {
  color: #64748b !important;
}

/* ========== 消息气泡优化 ========== */
.ws-collab-msg-avatar {
  width: 36px;
  height: 36px;
  font-size: 16px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.ws-msg-status {
  display: inline-flex;
  align-items: center;
  font-size: 12px;
}
.ws-msg-status.status-sent { color: #10b981; }
.ws-msg-status.status-thinking { color: #f59e0b; }
.ws-msg-status.status-done { color: #10b981; }
.ws-msg-status.status-failed { color: #ef4444; }
.pulse-icon {
  animation: pulseRotate 2s linear infinite;
}
@keyframes pulseRotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.ws-msg-files {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.ws-msg-file-chip {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px;
  background: rgba(124, 58, 237, 0.08);
  border-radius: 12px;
  font-size: 12px;
  color: #7c3aed;
  cursor: pointer;
  transition: all 0.2s;
}
.ws-msg-file-chip:hover {
  background: rgba(124, 58, 237, 0.15);
}
.msg-file-icon { font-size: 14px; }
.msg-file-name {
  max-width: 120px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 拖拽上传区 */
.ws-drop-zone {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(124, 58, 237, 0.95);
  backdrop-filter: blur(10px);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  z-index: 10;
  border-radius: 12px;
}
.drop-zone-icon {
  font-size: 32px;
  color: white;
}
.drop-zone-text {
  color: white;
  font-size: 14px;
  font-weight: 500;
}

.ws-upload-trigger {
  display: inline-flex;
}

/* ========== 白板样式 ========== */
.ws-whiteboard-content {
  position: relative;
}
.ws-whiteboard-toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 8px 12px;
  background: rgba(248, 250, 252, 0.8);
  border-bottom: 1px solid #f1f5f9;
  flex-wrap: wrap;
}
.ws-wb-tool {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 10px;
  border: none;
  background: transparent;
  border-radius: 8px;
  color: #64748b;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s;
}
.ws-wb-tool:hover {
  background: rgba(124, 58, 237, 0.1);
  color: #7c3aed;
}
.ws-wb-tool.active {
  background: rgba(124, 58, 237, 0.15);
  color: #7c3aed;
  font-weight: 500;
}
.ws-wb-tool-icon {
  font-size: 14px;
}
.ws-wb-tool-divider {
  width: 1px;
  height: 20px;
  background: #e2e8f0;
  margin: 0 4px;
}
.ws-wb-color-picker {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 0 8px;
}
.wb-color-label {
  font-size: 12px;
  color: #64748b;
  margin-right: 4px;
}
.wb-color-dot {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: all 0.2s;
}
.wb-color-dot:hover {
  transform: scale(1.2);
}
.wb-color-dot.active {
  border-color: white;
  box-shadow: 0 0 0 2px #7c3aed;
  transform: scale(1.15);
}

.ws-whiteboard-canvas {
  flex: 1;
  position: relative;
  background: 
    radial-gradient(circle, rgba(124, 58, 237, 0.05) 1px, transparent 1px),
    linear-gradient(135deg, #fafafa, #f5f5f5);
  background-size: 20px 20px, 100% 100%;
  overflow: hidden;
  cursor: default;
}

.wb-sticky-note {
  position: absolute;
  width: 160px;
  min-height: 100px;
  border-radius: 8px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
  cursor: move;
  transition: box-shadow 0.2s;
  z-index: 5;
}
.wb-sticky-note:hover {
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
}
.wb-note-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  font-size: 12px;
  font-weight: 600;
  color: rgba(0, 0, 0, 0.6);
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}
.wb-note-delete {
  width: 18px;
  height: 18px;
  border: none;
  background: transparent;
  color: rgba(0, 0, 0, 0.4);
  cursor: pointer;
  font-size: 14px;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.wb-note-delete:hover {
  background: rgba(0, 0, 0, 0.1);
  color: rgba(0, 0, 0, 0.7);
}
.wb-note-content {
  padding: 8px 10px;
  font-size: 13px;
  color: rgba(0, 0, 0, 0.75);
  outline: none;
  min-height: 40px;
  word-wrap: break-word;
}

.wb-text-box {
  position: absolute;
  padding: 6px 10px;
  cursor: move;
  z-index: 5;
  min-width: 80px;
}
.wb-text-box > div {
  outline: none;
  font-size: 14px;
  font-weight: 500;
}
.wb-text-delete {
  position: absolute;
  top: -8px;
  right: -8px;
  width: 18px;
  height: 18px;
  border: none;
  background: #ef4444;
  color: white;
  cursor: pointer;
  font-size: 12px;
  border-radius: 50%;
  display: none;
  align-items: center;
  justify-content: center;
}
.wb-text-box:hover .wb-text-delete {
  display: flex;
}

.wb-draw-layer {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 3;
}

.wb-empty-hint {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  text-align: center;
  color: #94a3b8;
  pointer-events: none;
}
.wb-empty-icon { font-size: 48px; margin-bottom: 8px; opacity: 0.5; }
.wb-empty-text { font-size: 14px; font-weight: 500; }
.wb-empty-tips { font-size: 12px; margin-top: 4px; opacity: 0.7; }

.ws-whiteboard-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: rgba(248, 250, 252, 0.8);
  border-top: 1px solid #f1f5f9;
}
.wb-stats {
  font-size: 12px;
  color: #64748b;
}

/* ========== 文件 Tab 样式 ========== */
.ws-files-upload-area {
  margin: 12px 16px;
  padding: 24px;
  border: 2px dashed #cbd5e1;
  border-radius: 12px;
  text-align: center;
  transition: all 0.3s;
  background: rgba(248, 250, 252, 0.5);
}
.ws-files-upload-area.drag-over {
  border-color: #7c3aed;
  background: rgba(124, 58, 237, 0.05);
}
.upload-area-icon {
  font-size: 32px;
  color: #94a3b8;
  margin-bottom: 8px;
}
.upload-area-text {
  font-size: 14px;
  color: #64748b;
  margin-bottom: 4px;
}
.upload-area-hint {
  font-size: 12px;
  color: #94a3b8;
  margin-bottom: 12px;
}

.ws-files-scroll {
  flex: 1;
  padding: 0 16px 16px;
}
.ws-files-empty {
  padding: 40px 0;
}
.ws-files-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 12px;
}
.ws-file-card-large {
  background: white;
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.3s;
}
.ws-file-card-large:hover {
  transform: translateY(-3px);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.1);
  border-color: rgba(124, 58, 237, 0.3);
}
.ws-file-preview {
  height: 80px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f8fafc;
  font-size: 36px;
}
.preview-pdf { background: linear-gradient(135deg, rgba(239, 68, 68, 0.1), rgba(249, 115, 22, 0.1)); }
.preview-doc { background: linear-gradient(135deg, rgba(59, 130, 246, 0.1), rgba(6, 182, 212, 0.1)); }
.preview-image { background: linear-gradient(135deg, rgba(16, 185, 129, 0.1), rgba(20, 184, 166, 0.1)); }
.preview-excel { background: linear-gradient(135deg, rgba(34, 197, 94, 0.1), rgba(16, 185, 129, 0.1)); }
.file-preview-icon { opacity: 0.8; }
.ws-file-card-body {
  padding: 10px 12px;
}
.ws-file-name-row {
  margin-bottom: 4px;
}
.ws-file-name-large {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  display: block;
}
.ws-file-meta-row {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: #94a3b8;
  margin-bottom: 8px;
}
.ws-file-actions-row {
  display: flex;
  gap: 4px;
}
.ws-file-actions-row .el-button {
  flex: 1;
  font-size: 11px !important;
  padding: 4px 0 !important;
  color: #64748b !important;
}
.ws-file-actions-row .el-button:hover {
  color: #7c3aed !important;
}

/* ========== 历史记录侧边栏 ========== */
.ws-history-panel {
  position: absolute;
  right: 0;
  top: 44px;
  bottom: 0;
  width: 280px;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(20px);
  border-left: 1px solid #e2e8f0;
  box-shadow: -4px 0 20px rgba(0, 0, 0, 0.08);
  z-index: 20;
  display: flex;
  flex-direction: column;
}
.ws-history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid #f1f5f9;
}
.ws-history-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  font-size: 14px;
  color: #0f172a;
}
.ws-history-close {
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
.ws-history-close:hover {
  background: #f1f5f9;
  color: #ef4444;
}
.ws-history-scroll {
  flex: 1;
  overflow: hidden;
}
.ws-history-timeline {
  padding: 12px 16px;
  position: relative;
}
.ws-history-item {
  position: relative;
  padding-left: 24px;
  padding-bottom: 16px;
  cursor: pointer;
  transition: all 0.2s;
}
.ws-history-item:hover {
  transform: translateX(2px);
}
.ws-history-item:hover .ws-history-event-title {
  color: #7c3aed;
}
.ws-history-dot {
  position: absolute;
  left: 0;
  top: 4px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: #e2e8f0;
  border: 2px solid white;
  box-shadow: 0 0 0 2px #e2e8f0;
  z-index: 2;
}
.ws-history-item.event-message .ws-history-dot { background: #7c3aed; box-shadow: 0 0 0 2px rgba(124, 58, 237, 0.3); }
.ws-history-item.event-file .ws-history-dot { background: #06b6d4; box-shadow: 0 0 0 2px rgba(6, 182, 212, 0.3); }
.ws-history-item.event-phase .ws-history-dot { background: #f59e0b; box-shadow: 0 0 0 2px rgba(245, 158, 11, 0.3); }
.ws-history-item.event-whiteboard .ws-history-dot { background: #10b981; box-shadow: 0 0 0 2px rgba(16, 185, 129, 0.3); }
.ws-history-item.event-mode .ws-history-dot { background: #ec4899; box-shadow: 0 0 0 2px rgba(236, 72, 153, 0.3); }

.ws-history-line {
  position: absolute;
  left: 5px;
  top: 16px;
  bottom: 0;
  width: 2px;
  background: #e2e8f0;
  z-index: 1;
}
.ws-history-content {
  background: rgba(248, 250, 252, 0.8);
  border-radius: 8px;
  padding: 8px 10px;
  transition: all 0.2s;
}
.ws-history-item:hover .ws-history-content {
  background: rgba(124, 58, 237, 0.06);
}
.ws-history-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
}
.ws-history-icon { font-size: 14px; }
.ws-history-event-title {
  font-size: 12px;
  font-weight: 600;
  color: #0f172a;
  transition: color 0.2s;
}
.ws-history-desc {
  font-size: 11px;
  color: #64748b;
  margin-bottom: 4px;
}
.ws-history-time {
  font-size: 10px;
  color: #94a3b8;
}

/* ========== 过渡动画 ========== */
.slide-fade-enter-active,
.slide-fade-leave-active {
  transition: all 0.3s ease;
}
.slide-fade-enter-from,
.slide-fade-leave-to {
  transform: translateX(20px);
  opacity: 0;
}

.mode-transition .ws-mode-tab {
  transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1);
}

/* ========== 任务编排模式 ========== */
.task-orch-view {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  overflow: hidden;
  background: linear-gradient(135deg, rgba(248, 250, 252, 0.8), rgba(241, 245, 249, 0.9));
}

/* 顶部控制栏 */
.orch-top-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-radius: 14px;
  flex-shrink: 0;
  position: relative;
  overflow: hidden;
}
.orch-top-bar::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: linear-gradient(90deg, #f59e0b, #ef4444, #8b5cf6);
}

.orch-progress-section {
  flex: 1;
  max-width: 60%;
}
.orch-progress-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.orch-progress-title {
  font-weight: 700;
  font-size: 15px;
  display: flex;
  align-items: center;
  gap: 8px;
  color: #0f172a;
}
.orch-title-icon {
  font-size: 18px;
}
.orch-progress-stats {
  display: flex;
  gap: 6px;
}
.orch-progress-bar-wrap {
  display: flex;
  align-items: center;
  gap: 12px;
}
.orch-progress-bar {
  flex: 1;
  height: 8px;
  background: rgba(148, 163, 184, 0.2);
  border-radius: 10px;
  position: relative;
  overflow: hidden;
}
.orch-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #10b981, #14b8a6, #06b6d4);
  border-radius: 10px;
  transition: width 0.5s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
}
.orch-progress-glow {
  position: absolute;
  top: 0;
  left: 0;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(16, 185, 129, 0.4), transparent);
  border-radius: 10px;
  animation: progressGlow 2s ease-in-out infinite;
}
@keyframes progressGlow {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}
.orch-progress-text {
  font-weight: 700;
  font-size: 14px;
  color: #10b981;
  min-width: 45px;
  text-align: right;
}

.orch-control-section {
  display: flex;
  align-items: center;
  gap: 10px;
}
.orch-mode-select {
  width: 110px;
}
.orch-btn-secondary {
  background: rgba(248, 250, 252, 0.8);
  border: 1px solid #e2e8f0;
  color: #475569;
  transition: all 0.2s;
}
.orch-btn-secondary:hover {
  background: white;
  border-color: #cbd5e1;
  color: #0f172a;
}
.orch-btn-primary {
  min-width: 100px;
}

/* 三栏主区域 */
.orch-main-area {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 1.2fr 1fr;
  gap: 12px;
  min-height: 0;
}

.orch-panel {
  display: flex;
  flex-direction: column;
  border-radius: 14px;
  overflow: hidden;
  position: relative;
}
.orch-panel::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background: linear-gradient(90deg, rgba(139, 92, 246, 0.3), rgba(6, 182, 212, 0.3));
  opacity: 0;
  transition: opacity 0.3s;
}
.orch-panel:hover::before {
  opacity: 1;
}

.orch-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.8);
  flex-shrink: 0;
  background: linear-gradient(180deg, rgba(248, 250, 252, 0.9), rgba(241, 245, 249, 0.5));
}
.orch-panel-title {
  font-weight: 700;
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 8px;
  color: #0f172a;
}
.orch-panel-icon {
  font-size: 16px;
}
.orch-task-count {
  font-size: 12px;
  color: #64748b;
  background: rgba(148, 163, 184, 0.15);
  padding: 2px 8px;
  border-radius: 10px;
}

/* 左栏：任务拆解 */
.orch-task-input-section {
  padding: 12px 16px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.6);
  flex-shrink: 0;
}
.orch-input-label {
  font-size: 12px;
  font-weight: 600;
  color: #64748b;
  margin-bottom: 8px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.orch-task-input :deep(.el-textarea__inner) {
  border-radius: 10px;
  border-color: #e2e8f0;
  font-size: 13px;
  transition: all 0.2s;
}
.orch-task-input :deep(.el-textarea__inner:focus) {
  border-color: #8b5cf6;
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.1);
}
.orch-input-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}
.orch-decompose-btn {
  flex: 1;
}
.orch-add-manual-btn {
  background: white;
  border: 1px solid #e2e8f0;
}

.orch-subtask-list {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.orch-list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
}
.orch-list-title {
  font-size: 13px;
  font-weight: 600;
  color: #475569;
}
.orch-list-actions {
  display: flex;
  gap: 4px;
}

.orch-subtask-scroll {
  flex: 1;
  padding: 0 12px 12px;
}

.orch-subtask-card {
  background: white;
  border: 1.5px solid #e2e8f0;
  border-radius: 12px;
  padding: 12px;
  margin-bottom: 10px;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
}
.orch-subtask-card:hover {
  border-color: #c4b5fd;
  box-shadow: 0 4px 12px rgba(139, 92, 246, 0.1);
  transform: translateY(-1px);
}
.orch-subtask-card.is-selected {
  border-color: #8b5cf6;
  box-shadow: 0 0 0 3px rgba(139, 92, 246, 0.15), 0 4px 12px rgba(139, 92, 246, 0.1);
}
.orch-subtask-card.is-dragging {
  opacity: 0.5;
  transform: scale(0.98);
}
.orch-subtask-card.drag-over {
  border-color: #10b981;
  border-style: dashed;
  background: rgba(16, 185, 129, 0.05);
}

.subtask-card-header {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  margin-bottom: 8px;
}
.subtask-index {
  width: 26px;
  height: 26px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-weight: 700;
  font-size: 12px;
  flex-shrink: 0;
}
.subtask-title-row {
  flex: 1;
  min-width: 0;
}
.subtask-title {
  font-weight: 600;
  font-size: 13px;
  color: #0f172a;
  display: block;
  margin-bottom: 4px;
}
.subtask-status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  background: rgba(148, 163, 184, 0.15);
  color: #64748b;
}
.subtask-status-badge .status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: #64748b;
}
.subtask-status-badge.status-pending { background: rgba(100, 116, 139, 0.15); color: #64748b; }
.subtask-status-badge.status-pending .status-dot { background: #64748b; }
.subtask-status-badge.status-waiting { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
.subtask-status-badge.status-waiting .status-dot { background: #f59e0b; }
.subtask-status-badge.status-inProgress { background: rgba(59, 130, 246, 0.15); color: #3b82f6; }
.subtask-status-badge.status-inProgress .status-dot { background: #3b82f6; animation: statusPulse 1.5s ease-in-out infinite; }
.subtask-status-badge.status-reviewing { background: rgba(139, 92, 246, 0.15); color: #8b5cf6; }
.subtask-status-badge.status-reviewing .status-dot { background: #8b5cf6; }
.subtask-status-badge.status-completed { background: rgba(16, 185, 129, 0.15); color: #10b981; }
.subtask-status-badge.status-completed .status-dot { background: #10b981; }
.subtask-status-badge.status-atRisk { background: rgba(249, 115, 22, 0.15); color: #f97316; }
.subtask-status-badge.status-atRisk .status-dot { background: #f97316; animation: statusPulse 1s ease-in-out infinite; }
.subtask-status-badge.status-failed { background: rgba(239, 68, 68, 0.15); color: #ef4444; }
.subtask-status-badge.status-failed .status-dot { background: #ef4444; }
.subtask-status-badge.status-archived { background: rgba(100, 116, 139, 0.15); color: #64748b; }
.subtask-status-badge.status-archived .status-dot { background: #64748b; }

@keyframes statusPulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.6; transform: scale(1.2); }
}

.subtask-card-body {
  padding-left: 36px;
}
.subtask-desc {
  font-size: 12px;
  color: #64748b;
  line-height: 1.5;
  margin: 0 0 8px 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.subtask-meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.subtask-time {
  font-size: 11px;
  color: #94a3b8;
  display: flex;
  align-items: center;
  gap: 3px;
}
.subtask-deps {
  margin-top: 8px;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}
.deps-label {
  font-size: 11px;
  color: #94a3b8;
}
.dep-tag {
  font-size: 10px;
  padding: 1px 6px;
  background: rgba(245, 158, 11, 0.1);
  color: #f59e0b;
  border-radius: 6px;
  font-weight: 600;
}

.subtask-card-actions {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  gap: 2px;
  opacity: 0;
  transition: opacity 0.2s;
}
.orch-subtask-card:hover .subtask-card-actions {
  opacity: 1;
}
.subtask-action-btn {
  width: 24px;
  height: 24px;
  border: none;
  background: rgba(248, 250, 252, 0.9);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  font-size: 12px;
  transition: all 0.2s;
}
.subtask-action-btn:hover {
  background: #e2e8f0;
  color: #0f172a;
}
.subtask-action-btn.delete:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}

.subtask-expanded-detail {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed #e2e8f0;
  padding-left: 36px;
}
.detail-section {
  margin-bottom: 10px;
}
.detail-label {
  font-size: 11px;
  font-weight: 600;
  color: #94a3b8;
  text-transform: uppercase;
  margin-bottom: 6px;
  letter-spacing: 0.5px;
}
.assigned-experts {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.assigned-expert-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  color: white;
  font-weight: 600;
  border: 2px solid white;
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}
.add-expert-btn {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 1px dashed #cbd5e1;
  background: transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #94a3b8;
  font-size: 11px;
  transition: all 0.2s;
}
.add-expert-btn:hover {
  border-color: #8b5cf6;
  color: #8b5cf6;
  background: rgba(139, 92, 246, 0.05);
}
.task-result-text {
  font-size: 12px;
  color: #475569;
  line-height: 1.6;
  padding: 8px 10px;
  background: rgba(16, 185, 129, 0.05);
  border-radius: 8px;
  border-left: 3px solid #10b981;
}

.orch-empty-hint p {
  margin: 4px 0;
  color: #94a3b8;
  font-size: 13px;
}
.orch-empty-hint .hint-sub {
  font-size: 11px;
  color: #cbd5e1;
}

/* 中栏：专家分配 */
.orch-auto-assign-btn {
  color: #8b5cf6;
  font-weight: 600;
}

.orch-expert-pool {
  padding: 12px 16px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.6);
  flex-shrink: 0;
}
.orch-pool-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}
.orch-pool-title {
  font-size: 12px;
  font-weight: 600;
  color: #475569;
}
.orch-pool-count {
  font-size: 11px;
  color: #94a3b8;
}
.orch-expert-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 8px;
}
.orch-expert-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  background: white;
  border: 1.5px solid #e2e8f0;
  border-radius: 10px;
  cursor: grab;
  transition: all 0.2s;
  position: relative;
}
.orch-expert-chip:hover {
  border-color: #c4b5fd;
  box-shadow: 0 2px 8px rgba(139, 92, 246, 0.1);
  transform: translateY(-1px);
}
.orch-expert-chip:active {
  cursor: grabbing;
}
.orch-expert-chip.is-busy {
  opacity: 0.6;
}
.chip-avatar {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  flex-shrink: 0;
  position: relative;
}
.chip-status-dot {
  position: absolute;
  bottom: -1px;
  right: -1px;
  width: 9px;
  height: 9px;
  border-radius: 50%;
  border: 2px solid white;
}
.chip-status-dot.dot-active { background: #10b981; }
.chip-status-dot.dot-busy { background: #f59e0b; }
.chip-status-dot.dot-idle { background: #94a3b8; }
.chip-status-dot.dot-offline { background: #cbd5e1; }

.chip-info {
  flex: 1;
  min-width: 0;
}
.chip-name {
  font-size: 12px;
  font-weight: 600;
  color: #0f172a;
  display: block;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.chip-role {
  font-size: 10px;
  color: #94a3b8;
  display: block;
}
.chip-load {
  width: 30px;
  flex-shrink: 0;
}
.load-bar {
  height: 4px;
  background: rgba(148, 163, 184, 0.2);
  border-radius: 2px;
  overflow: hidden;
}
.load-fill {
  height: 100%;
  background: linear-gradient(90deg, #10b981, #14b8a6);
  border-radius: 2px;
  transition: width 0.3s;
}

.orch-assignment-board {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.orch-board-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
}
.orch-board-title {
  font-size: 12px;
  font-weight: 600;
  color: #475569;
}
.orch-board-hint {
  font-size: 10px;
  color: #94a3b8;
}
.orch-board-scroll {
  flex: 1;
  padding: 0 12px 12px;
}

.orch-task-assign-card {
  display: flex;
  align-items: stretch;
  gap: 10px;
  padding: 12px;
  background: white;
  border: 1.5px solid #e2e8f0;
  border-radius: 12px;
  margin-bottom: 10px;
  cursor: pointer;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
}
.orch-task-assign-card:hover {
  border-color: #c4b5fd;
  box-shadow: 0 4px 12px rgba(139, 92, 246, 0.1);
}
.orch-task-assign-card.drag-over {
  border-color: #10b981;
  border-style: dashed;
  background: rgba(16, 185, 129, 0.05);
  transform: scale(1.01);
}
.orch-task-assign-card.status-completed {
  border-left: 3px solid #10b981;
}
.orch-task-assign-card.status-inProgress {
  border-left: 3px solid #3b82f6;
}
.orch-task-assign-card.status-failed {
  border-left: 3px solid #ef4444;
}
.assign-card-left {
  flex-shrink: 0;
}
.assign-task-index {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-weight: 700;
  font-size: 12px;
}
.assign-card-body {
  flex: 1;
  min-width: 0;
}
.assign-task-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 4px;
}
.assign-task-title {
  font-weight: 600;
  font-size: 13px;
  color: #0f172a;
}
.assign-task-desc {
  font-size: 12px;
  color: #64748b;
  line-height: 1.4;
  margin: 0 0 8px 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.assign-experts-row {
  margin-top: auto;
}
.assigned-experts-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
}
.assigned-expert-chip {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 2px 4px 2px 2px;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  background: #f8fafc;
}
.chip-avatar-sm {
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
}
.chip-name-sm {
  font-size: 11px;
  font-weight: 500;
  color: #475569;
}
.chip-remove {
  width: 16px;
  height: 16px;
  border: none;
  background: transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #94a3b8;
  font-size: 10px;
  border-radius: 50%;
  transition: all 0.2s;
}
.chip-remove:hover {
  background: rgba(239, 68, 68, 0.1);
  color: #ef4444;
}
.add-expert-inline-btn {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 3px 8px;
  border: 1px dashed #cbd5e1;
  background: transparent;
  border-radius: 12px;
  cursor: pointer;
  font-size: 11px;
  color: #94a3b8;
  transition: all 0.2s;
}
.add-expert-inline-btn:hover {
  border-color: #8b5cf6;
  color: #8b5cf6;
  background: rgba(139, 92, 246, 0.05);
}
.assign-card-right {
  display: flex;
  flex-direction: column;
  justify-content: center;
  flex-shrink: 0;
}
.task-time-estimate {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: #94a3b8;
}

/* 右栏：时间线 */
.orch-timeline-actions {
  display: flex;
  gap: 4px;
}

.orch-gantt-container {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.gantt-header {
  display: flex;
  padding: 8px 16px;
  border-bottom: 1px solid rgba(226, 232, 240, 0.6);
  flex-shrink: 0;
  background: rgba(248, 250, 252, 0.5);
}
.gantt-task-col {
  width: 100px;
  flex-shrink: 0;
  font-size: 11px;
  font-weight: 600;
  color: #64748b;
}
.gantt-time-col {
  flex: 1;
  min-width: 0;
}
.gantt-time-scale {
  display: flex;
  justify-content: space-between;
}
.time-slot {
  font-size: 10px;
  color: #94a3b8;
  font-weight: 500;
}

.gantt-body-scroll {
  flex: 1;
  padding: 4px 0 12px;
}
.gantt-body {
  padding: 0 16px;
}
.gantt-row {
  display: flex;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid rgba(226, 232, 240, 0.4);
  cursor: pointer;
  transition: background 0.2s;
}
.gantt-row:hover {
  background: rgba(139, 92, 246, 0.03);
}
.gantt-row.is-selected {
  background: rgba(139, 92, 246, 0.08);
}
.gantt-task-label {
  width: 100px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: #475569;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.gantt-task-idx {
  font-weight: 700;
  color: #8b5cf6;
}
.gantt-task-name {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.gantt-bar-area {
  flex: 1;
  height: 28px;
  position: relative;
}
.gantt-grid {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  justify-content: space-between;
}
.grid-line {
  width: 1px;
  height: 100%;
  background: rgba(226, 232, 240, 0.6);
}
.gantt-task-bar {
  position: absolute;
  top: 4px;
  height: 20px;
  border-radius: 6px;
  min-width: 40px;
  overflow: hidden;
  transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}
.gantt-bar-fill {
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg, #94a3b8, #64748b);
  opacity: 0.8;
}
.gantt-bar-glow {
  position: absolute;
  inset: 0;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.3), transparent);
  animation: ganttShimmer 2s ease-in-out infinite;
}
@keyframes ganttShimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
.gantt-bar-label {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 10px;
  color: white;
  font-weight: 600;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
}
.gantt-task-bar.status-pending .gantt-bar-fill {
  background: linear-gradient(135deg, #94a3b8, #64748b);
}
.gantt-task-bar.status-waiting .gantt-bar-fill {
  background: linear-gradient(135deg, #fbbf24, #f59e0b);
}
.gantt-task-bar.status-inProgress .gantt-bar-fill {
  background: linear-gradient(135deg, #60a5fa, #3b82f6);
}
.gantt-task-bar.status-reviewing .gantt-bar-fill {
  background: linear-gradient(135deg, #a78bfa, #8b5cf6);
}
.gantt-task-bar.status-completed .gantt-bar-fill {
  background: linear-gradient(135deg, #34d399, #10b981);
}
.gantt-task-bar.status-atRisk .gantt-bar-fill {
  background: linear-gradient(135deg, #fb923c, #f97316);
}
.gantt-task-bar.status-failed .gantt-bar-fill {
  background: linear-gradient(135deg, #f87171, #ef4444);
}

/* 时间线列表视图 */
.orch-timeline-list {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.timeline-scroll {
  flex: 1;
}
.timeline-list-inner {
  padding: 12px 16px;
}
.timeline-item {
  display: flex;
  gap: 12px;
  position: relative;
  padding-bottom: 16px;
  cursor: pointer;
  transition: opacity 0.2s;
}
.timeline-item:hover {
  opacity: 0.9;
}
.timeline-item.is-selected .timeline-dot {
  transform: scale(1.1);
  box-shadow: 0 0 0 4px rgba(139, 92, 246, 0.2);
}
.timeline-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: #e2e8f0;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 12px;
  font-weight: 700;
  color: white;
  z-index: 1;
  transition: all 0.25s;
}
.timeline-dot.status-pending { background: linear-gradient(135deg, #94a3b8, #64748b); }
.timeline-dot.status-waiting { background: linear-gradient(135deg, #fbbf24, #f59e0b); }
.timeline-dot.status-inProgress { 
  background: linear-gradient(135deg, #60a5fa, #3b82f6);
  animation: timelinePulse 1.5s ease-in-out infinite;
}
.timeline-dot.status-reviewing { background: linear-gradient(135deg, #a78bfa, #8b5cf6); }
.timeline-dot.status-completed { background: linear-gradient(135deg, #34d399, #10b981); }
.timeline-dot.status-atRisk { background: linear-gradient(135deg, #fb923c, #f97316); }
.timeline-dot.status-failed { background: linear-gradient(135deg, #f87171, #ef4444); }

@keyframes timelinePulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(59, 130, 246, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(59, 130, 246, 0); }
}

.timeline-line {
  position: absolute;
  left: 13px;
  top: 28px;
  bottom: 0;
  width: 2px;
  background: linear-gradient(180deg, #e2e8f0, rgba(226, 232, 240, 0.3));
}
.timeline-content {
  flex: 1;
  padding-top: 4px;
}
.timeline-task-title {
  font-weight: 600;
  font-size: 13px;
  color: #0f172a;
  margin-bottom: 4px;
}
.timeline-task-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}
.timeline-status {
  font-size: 11px;
  font-weight: 500;
  padding: 1px 8px;
  border-radius: 10px;
}
.timeline-status.status-pending { background: rgba(100, 116, 139, 0.15); color: #64748b; }
.timeline-status.status-waiting { background: rgba(245, 158, 11, 0.15); color: #f59e0b; }
.timeline-status.status-inProgress { background: rgba(59, 130, 246, 0.15); color: #3b82f6; }
.timeline-status.status-reviewing { background: rgba(139, 92, 246, 0.15); color: #8b5cf6; }
.timeline-status.status-completed { background: rgba(16, 185, 129, 0.15); color: #10b981; }
.timeline-status.status-atRisk { background: rgba(249, 115, 22, 0.15); color: #f97316; }
.timeline-status.status-failed { background: rgba(239, 68, 68, 0.15); color: #ef4444; }
.timeline-time {
  font-size: 11px;
  color: #94a3b8;
  display: flex;
  align-items: center;
  gap: 3px;
}
.timeline-experts {
  display: flex;
  align-items: center;
  gap: -4px;
}
.timeline-expert-avatar {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  color: white;
  border: 2px solid white;
  margin-left: -4px;
}
.timeline-expert-avatar:first-child {
  margin-left: 0;
}
.timeline-more-experts {
  font-size: 10px;
  color: #94a3b8;
  margin-left: 4px;
}

/* 风险预警区 */
.orch-risk-section {
  padding: 10px 16px;
  border-top: 1px solid rgba(226, 232, 240, 0.6);
  background: rgba(249, 115, 22, 0.03);
  flex-shrink: 0;
}
.risk-section-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 8px;
}
.risk-icon {
  font-size: 14px;
}
.risk-title {
  font-size: 12px;
  font-weight: 600;
  color: #f97316;
}
.risk-badge {
  margin-left: auto;
}
.risk-task-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.risk-task-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  background: white;
  border: 1px solid rgba(249, 115, 22, 0.2);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
}
.risk-task-item:hover {
  background: rgba(249, 115, 22, 0.05);
  border-color: #f97316;
}
.risk-task-name {
  font-size: 12px;
  color: #475569;
}

/* ========== 响应式优化 ========== */
@media (max-width: 1200px) {
  .ws-kpi-card {
    padding: 12px 16px;
  }
  .ws-kpi-value {
    font-size: 20px;
  }
  .ws-kpi-icon {
    width: 40px;
    height: 40px;
    font-size: 18px;
  }
}

/* 任务编排模式响应式 */
@media (max-width: 1400px) {
  .orch-main-area {
    grid-template-columns: 1fr 1fr;
  }
  .orch-panel-right {
    grid-column: 1 / -1;
    max-height: 250px;
  }
}

@media (max-width: 1100px) {
  .orch-main-area {
    grid-template-columns: 1fr;
  }
  .orch-expert-grid {
    grid-template-columns: repeat(3, 1fr);
  }
  .orch-progress-section {
    max-width: 100%;
  }
  .orch-control-section {
    flex-wrap: wrap;
  }
}

@media (max-width: 768px) {
  .task-orch-view {
    padding: 8px;
    gap: 8px;
  }
  .orch-top-bar {
    flex-direction: column;
    gap: 10px;
    padding: 12px;
  }
  .orch-expert-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>