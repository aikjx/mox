<!--
  专家联盟统一工作台 · Expert Alliance Unified Workspace
  ======================================================
  架构原则：前端融合 · 后端模块化
  三栏布局：左(专家联盟) | 中(图谱画布+协作) | 右(知识库云盘)
  P0 体验整合：真实 API 集成 · 三栏联动 · 折叠展开
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
        <el-divider direction="vertical" class="ws-header-divider" />
        <el-select v-model="currentProject" size="small" class="ws-project-select" @change="onProjectChange">
          <el-option label="璇玑知识工程" value="xuanji" />
          <el-option label="MOX 平台架构" value="mox" />
          <el-option label="AI 算法实验室" value="ailab" />
        </el-select>
      </div>

      <div class="ws-header-center">
        <div class="ws-mode-tabs">
          <button
            v-for="mode in workModes"
            :key="mode.key"
            class="ws-mode-tab"
            :class="{ active: activeMode === mode.key }"
            @click="switchWorkMode(mode.key)"
          >
            <el-icon class="ws-mode-icon"><component :is="mode.iconComp" /></el-icon>
            <span class="ws-mode-label">{{ mode.label }}</span>
          </button>
        </div>
      </div>

      <div class="ws-header-right">
        <div class="ws-global-search">
          <el-icon><Search /></el-icon>
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
        <el-button type="primary" size="small" class="ws-ai-btn" @click="openAIAssistant">
          <el-icon><MagicStick /></el-icon>
          <span>AI 协作</span>
        </el-button>
        <el-badge :value="3" :hidden="!hasNotifications" class="ws-notif-badge">
          <el-button size="small" text class="ws-icon-btn" title="通知">
            <el-icon><Bell /></el-icon>
          </el-button>
        </el-badge>
        <el-avatar :size="32" class="ws-avatar" style="background: linear-gradient(135deg, #6366f1, #06b6d4)">U</el-avatar>
      </div>
    </header>

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
              <span class="ws-section-count">{{ filteredExperts.length }} 位</span>
            </div>
            <el-scrollbar class="ws-expert-scroll">
              <div
                v-for="expert in filteredExperts"
                :key="expert.id"
                class="ws-expert-item"
                :class="{ active: activeExpert?.id === expert.id, selected: isExpertSelected(expert.id) }"
                @click="handleExpertClick(expert)"
              >
                <div class="ws-expert-avatar" :style="{ background: expertColor(expert.type) }">
                  {{ expertEmoji(expert.type) }}
                </div>
                <div class="ws-expert-info">
                  <div class="ws-expert-name">
                    {{ expert.name }}
                    <span v-if="expert.status === 'active'" class="ws-online-dot" title="在线"></span>
                  </div>
                  <div class="ws-expert-role">{{ EXPERT_TYPES[expert.type] || expert.type }}</div>
                  <div v-if="expert.capabilities?.length" class="ws-expert-tags">
                    <span v-for="cap in expert.capabilities.slice(0, 2)" :key="cap" class="ws-cap-tag">{{ cap }}</span>
                  </div>
                </div>
                <div v-if="isExpertSelected(expert.id)" class="ws-expert-check">
                  <el-icon><CircleCheckFilled /></el-icon>
                </div>
                <div v-else class="ws-expert-status" :class="expertStatusClass(expert.status)">
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
              <button class="ws-tool-btn" :class="{ active: activeMode === 'debate' }" @click="triggerDebate">
                <span class="ws-tool-icon">⚔️</span>
                <span>专家辩论</span>
              </button>
              <button class="ws-tool-btn" :class="{ active: activeMode === 'orchestration' }" @click="triggerOrchestration">
                <span class="ws-tool-icon">🎯</span>
                <span>任务编排</span>
              </button>
              <button class="ws-tool-btn" @click="triggerVoting">
                <span class="ws-tool-icon">🗳️</span>
                <span>融合投票</span>
              </button>
              <button class="ws-tool-btn" :class="{ active: activeMode === 'collaboration' }" @click="triggerConsult">
                <span class="ws-tool-icon">💬</span>
                <span>多轮咨询</span>
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

        <!-- 底部协作对话栏 -->
        <div class="ws-collab-bar" :class="{ expanded: collabExpanded, 'is-running': allianceRunning }">
          <div class="ws-collab-header" @click="collabExpanded = !collabExpanded">
            <div class="ws-collab-title">
              <el-icon v-if="allianceRunning" class="ws-pulse-icon"><Promotion /></el-icon>
              <el-icon v-else><ChatLineSquare /></el-icon>
              <span>协作讨论 · {{ activeSession?.title || '未开始' }}</span>
              <el-tag v-if="allianceRunning" size="small" type="warning" effect="light" class="ws-running-tag">
                {{ currentPhaseLabel }}
              </el-tag>
              <span class="ws-collab-count">{{ collabMessages.length }} 条消息</span>
            </div>
            <div class="ws-collab-toggle">
              <el-icon v-if="collabExpanded"><ArrowDown /></el-icon>
              <el-icon v-else><ArrowUp /></el-icon>
            </div>
          </div>
          <div v-if="collabExpanded" class="ws-collab-body">
            <!-- 阶段进度条 -->
            <div v-if="allianceRunning && alliancePhases.length > 0" class="ws-alliance-phases">
              <div
                v-for="(phase, idx) in alliancePhases"
                :key="phase.key"
                class="ws-phase-step"
                :class="{ active: currentPhaseIndex === idx, done: currentPhaseIndex > idx }"
              >
                <div class="ws-phase-step-dot">{{ idx + 1 }}</div>
                <span class="ws-phase-step-label">{{ phase.label }}</span>
              </div>
            </div>

            <el-scrollbar class="ws-collab-messages" ref="messagesScrollRef">
              <div v-for="msg in collabMessages" :key="msg.id" class="ws-collab-msg" :class="[msg.role, msg.phase ? `phase-${msg.phase}` : '']">
                <div class="ws-collab-msg-avatar" :style="{ background: msg.color || '#64748b' }">
                  {{ msg.avatar || '?' }}
                </div>
                <div class="ws-collab-msg-content">
                  <div class="ws-collab-msg-meta">
                    <span class="ws-collab-msg-name">{{ msg.name }}</span>
                    <span v-if="msg.phase" class="ws-collab-msg-phase">
                      <el-tag size="small" effect="plain" :type="phaseTagType(msg.phase)">{{ phaseLabel(msg.phase) }}</el-tag>
                    </span>
                    <span class="ws-collab-msg-time">{{ msg.time }}</span>
                  </div>
                  <div class="ws-collab-msg-text" v-html="formatMessageText(msg.text)"></div>
                </div>
              </div>
              <div v-if="allianceRunning" class="ws-collab-msg assistant ws-typing">
                <div class="ws-collab-msg-avatar" style="background: linear-gradient(135deg, #6366f1, #06b6d4)">
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
              <div class="ws-collab-input-tools">
                <el-button size="small" text class="ws-tool-mini-btn" title="附件">
                  <el-icon><Paperclip /></el-icon>
                </el-button>
                <el-button size="small" text class="ws-tool-mini-btn" title="引用图谱节点" @click="insertNodeRef">
                  <el-icon><Share /></el-icon>
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
                <el-button type="primary" class="ws-send-btn" @click="sendCollabMsg" :loading="allianceRunning">
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
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { ElMessage, ElNotification } from 'element-plus'
import {
  Search, MagicStick, Bell, ArrowLeft, ArrowRight, Plus,
  ZoomIn, ZoomOut, FullScreen, DataAnalysis, Close,
  Document, ChatDotRound, ChatLineSquare, ArrowDown, ArrowUp,
  Folder, FolderOpened, Upload, Edit, CircleCheckFilled,
  Share, Link, Paperclip, Promotion, Loading, RefreshRight,
  UserFilled, SetUp, Pointer, Rank, Delete, CollectionTag
} from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'
import { runAllianceFullSSE, getAllianceCapabilities } from '@/api/alliance'
import {
  getExperts, getExpertGraph, listExpertSessions,
  expertDebate, expertOrchestrate, multiExpertConsult
} from '@/api/experts.api.js'
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

// ========== 工作模式 ==========
const activeMode = ref('collaboration')
const workModes = [
  { key: 'exploration', label: '知识探索', iconComp: 'Search' },
  { key: 'collaboration', label: '专家协作', iconComp: 'UserFilled' },
  { key: 'orchestration', label: '任务编排', iconComp: 'SetUp' },
  { key: 'analysis', label: '深度分析', iconComp: 'DataAnalysis' }
]

function switchWorkMode(mode) {
  activeMode.value = mode
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
  collabInput.value = '请编排以下任务的执行流程…'
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
})

// 监听选中专家变化，更新会话中的专家数
watch(selectedExpertIds, () => {
  if (activeSession.value) {
    activeSession.value.expert_count = selectedExpertIds.value.length
  }
}, { deep: true })
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
</style>