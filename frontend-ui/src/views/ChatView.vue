<template>
  <div class="chat">
    <SessionSidebar
      :sessions="sessions"
      :active-id="currentSession"
      :online="online"
      @select="selectSession"
      @new="newSession"
    />

    <div class="chat-main">
      <div class="chat-header">
        <div class="chat-title">
          <el-icon><ChatDotRound /></el-icon>
          <span>AI 智能助手</span>
          <span class="badge info">意图识别 · 算子推荐 · 算法分析</span>
        </div>
        <div class="chat-tools">
          <div class="expert-selector">
            <span class="muted">专家模式:</span>
            <el-select v-model="selectedExpert" size="small" placeholder="选择专家">
              <el-option
                v-for="preset in AI_EXPERT_PRESETS"
                :key="preset.key"
                :label="preset.label"
                :value="preset"
              />
            </el-select>
          </div>

          <div class="ct-divider"></div>

          <!-- 分组1：会话管理 -->
          <el-tooltip content="新建对话" placement="bottom">
            <el-button class="ct-btn ct-btn-new" text @click="newSession"><el-icon><DocumentAdd /></el-icon> 新建</el-button>
          </el-tooltip>
          <el-tooltip content="清空当前会话" placement="bottom">
            <el-button class="ct-btn" text @click="clearChat"><el-icon><Delete /></el-icon> 清空</el-button>
          </el-tooltip>
          <el-tooltip content="从后端恢复会话历史" placement="bottom">
            <el-button class="ct-btn" text @click="openBackendHistory"><el-icon><Clock /></el-icon> 历史</el-button>
          </el-tooltip>

          <div class="ct-divider"></div>

          <!-- 分组2：数据管理 -->
          <el-tooltip content="导出对话+图谱迁移包" placement="bottom">
            <el-button class="ct-btn" text @click="exportBundle"><el-icon><Download /></el-icon> 导出</el-button>
          </el-tooltip>
          <el-tooltip content="导入迁移包" placement="bottom">
            <el-button class="ct-btn" text @click="pickImport"><el-icon><Upload /></el-icon> 导入</el-button>
          </el-tooltip>

          <div class="ct-divider"></div>

          <!-- 分组3：流程联动 -->
          <el-tooltip content="将当前对话转换为任务" placement="bottom">
            <el-button class="ct-btn" text type="primary" plain @click="convertToTask" :loading="convertingTask">
              <el-icon><List /></el-icon> 转任务
            </el-button>
          </el-tooltip>
          <el-tooltip content="基于对话一键创建项目" placement="bottom">
            <el-button class="ct-btn" type="success" plain @click="openProjectDialog">
              <el-icon><FolderAdd /></el-icon> 创建项目
            </el-button>
          </el-tooltip>

          <div class="ct-divider"></div>

          <!-- 分组4：核心功能 · 全维分析 CTA 主按钮 -->
          <el-tooltip content="架构开发专家联盟 · 一键启动全维分析（需求→架构→实现→测试→验收）" placement="bottom">
            <el-button class="ct-cta-full" @click="() => { triggerFullAnalysis(); triggerAlliance(); }">
              <el-icon><Promotion /></el-icon>
              <span>全维分析</span>
            </el-button>
          </el-tooltip>

          <!-- T11: 专家联盟 5 阶段 Chip（AC-10） -->
          <div class="alliance-chips" v-if="allianceRunning || alliancePhase">
            <div
              v-for="p in PHASE_CHIPS"
              :key="p"
              class="chip"
              :class="{
                active: alliancePhase === p,
                done: alliancePhase && PHASE_CHIPS.indexOf(alliancePhase) > PHASE_CHIPS.indexOf(p)
              }"
            >
              <span class="dot" />
              <span class="lbl">{{
                p === 'intent' ? '①意图'
                : p === 'team' ? '②组队'
                : p === 'debate' ? '③辩论'
                : p === 'gate' ? '④门禁'
                : '⑤完成'
              }}</span>
            </div>
          </div>

          <div class="ct-divider"></div>

          <el-tooltip content="对话内容自动整理进知识图谱（全自动）" placement="bottom">
            <el-switch
              v-model="autoSync"
              inline-prompt
              active-text="自动入图"
              inactive-text="手动"
              @change="onToggleAutoSync"
            />
          </el-tooltip>
          <el-tooltip content="开启后：AI分析对话自动创建任务并执行" placement="bottom">
            <el-switch
              v-model="autoTaskMode"
              inline-prompt
              active-text="任务模式"
              inactive-text="对话模式"
              @change="onAutoTaskToggle"
            />
          </el-tooltip>
          <el-tooltip content="需求流程模式：选择问题，进入设计→分析→开发→测试→修复→优化流程" placement="bottom">
            <el-switch
              v-model="requirementFlowMode"
              inline-prompt
              active-text="流程模式"
              inactive-text=""
              @change="onRequirementFlowToggle"
            />
          </el-tooltip>

          <input ref="importInput" type="file" accept="application/json" hidden @change="onImportFile" />
        </div>
      </div>

      <!-- 项目对话上下文 bar（跟进项目模式下显示） -->
      <div v-if="chatMode === 'followup'" class="project-context-bar">
        <div class="pcb-left">
          <div class="pcb-project">
            <span class="pcb-icon">📁</span>
            <div class="pcb-info">
              <div class="pcb-name">
                <template v-if="currentProject">{{ currentProject.name }}</template>
                <template v-else>未选择项目</template>
                <el-tag v-if="currentProject" size="small" effect="plain" :type="projectStatusType">{{ projectStatusLabel }}</el-tag>
              </div>
              <div class="pcb-desc">{{ currentProject?.description || '选择项目后，以项目为中心进行全链路系统开发' }}</div>
            </div>
          </div>
          <div class="pcb-actions">
            <el-button v-if="currentProject" size="small" text @click="openEditProject">
              <el-icon><Edit /></el-icon> 编辑
            </el-button>
            <el-button size="small" text @click="openCreateProject">
              <el-icon><FolderAdd /></el-icon> 新建
            </el-button>
            <el-button v-if="currentProject" size="small" text @click="openResourcePanel">
              <el-icon><Files /></el-icon> 资源
            </el-button>
          </div>
          <el-button v-if="!currentProject" size="small" type="primary" @click="goProjects">
            <el-icon><Plus /></el-icon> 选择项目
          </el-button>
        </div>

        <div class="pcb-right">
          <div class="pcb-agent">
            <span class="pcb-label">推荐智能体：</span>
            <el-tag size="small" type="success" effect="dark" class="agent-tag">
              {{ recommendedExpert?.label || '通用助手' }}
            </el-tag>
            <span class="pcb-hint">{{ recommendedExpert?.hint || '通用问答' }}</span>
          </div>
          <div class="pcb-stages">
            <span
              v-for="(stage, i) in DEV_STAGES"
              :key="stage.key"
              class="pcb-stage"
              :class="{ active: devStage === i, done: devStage > i }"
              @click="devStage = i"
            >{{ stage.label }}</span>
          </div>
          <el-button
            v-if="currentProject"
            size="small"
            type="warning"
            class="full-dev-btn"
            :loading="fullDevRunning"
            @click="startFullDev"
          >
            <el-icon><Promotion /></el-icon>
            {{ fullDevRunning ? `开发中 ${fullDevStage+1}/5` : 'AI全维开发' }}
          </el-button>
        </div>
      </div>

      <!-- 新建/编辑项目对话框 -->
      <el-dialog v-model="projectDlg.visible" :title="projectDlg.isEdit ? '编辑项目' : '新建项目'" width="560px">
        <el-form label-width="80px">
          <el-form-item label="项目名称">
            <el-input v-model="projectDlg.form.name" placeholder="如：政务信创门户改造系统" maxlength="60" />
          </el-form-item>
          <el-form-item label="项目类型">
            <div class="cat-picker">
              <div
                v-for="c in PROJECT_CATEGORIES"
                :key="c.key"
                class="cat-option"
                :class="{ active: projectDlg.form.category === c.key }"
                :style="projectDlg.form.category === c.key ? { borderColor: c.color, background: c.color + '14' } : {}"
                @click="projectDlg.form.category = c.key"
              >
                <span class="cat-dot" :style="{ background: c.color }"></span>
                <span>{{ c.label }}</span>
              </div>
            </div>
            <div class="muted" style="margin-top:4px;font-size:12px;">{{ PROJECT_CATEGORIES.find(c=>c.key===projectDlg.form.category)?.desc }}</div>
          </el-form-item>
          <el-form-item label="状态">
            <el-radio-group v-model="projectDlg.form.status">
              <el-radio value="active">进行中</el-radio>
              <el-radio value="done">已完成</el-radio>
              <el-radio value="archived">已归档</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="代码路径">
            <el-input v-model="projectDlg.form.code_path" placeholder="如：D:/projects/my-app 或 git@github.com:org/repo.git" />
          </el-form-item>
          <el-form-item label="技术栈">
            <el-input v-model="projectDlg.form.tech_stack" placeholder="如：Vue3+SpringBoot+MySQL+Redis" />
          </el-form-item>
          <el-form-item label="项目描述">
            <el-input v-model="projectDlg.form.description" type="textarea" :rows="3" placeholder="项目目标、范围、核心功能等说明" />
          </el-form-item>
        </el-form>
        <template #footer>
          <el-button @click="projectDlg.visible = false">取消</el-button>
          <el-button type="primary" :loading="projectDlg.submitting" @click="saveProjectFromChat">保存</el-button>
        </template>
      </el-dialog>

      <!-- 项目资源面板抽屉 -->
      <el-drawer v-model="resourcePanel.visible" title="项目资源总览" size="720px">
        <template v-if="currentProject">
          <div class="res-panel-head">
            <div class="res-project-info">
              <div class="res-pname">{{ currentProject.name }}</div>
              <div class="res-pdesc">{{ currentProject.description || '暂无描述' }}</div>
            </div>
            <el-tag size="small" :type="projectStatusType">{{ projectStatusLabel }}</el-tag>
          </div>

          <el-tabs v-model="resourcePanel.activeTab" class="res-tabs">
            <!-- 总览 -->
            <el-tab-pane label="总览" name="overview">
              <div class="res-overview">
                <div class="res-stat-grid">
                  <div class="res-stat-card" @click="resourcePanel.activeTab = 'graph'">
                    <div class="res-stat-icon graph"><el-icon><Share /></el-icon></div>
                    <div class="res-stat-num">{{ currentProject.resources?.filter(r => r.resource_type === 'graph').length || 0 }}</div>
                    <div class="res-stat-label">知识图谱</div>
                  </div>
                  <div class="res-stat-card" @click="resourcePanel.activeTab = 'requirement'">
                    <div class="res-stat-icon req"><el-icon><Document /></el-icon></div>
                    <div class="res-stat-num">{{ currentProject.resources?.filter(r => r.resource_type === 'requirement').length || 0 }}</div>
                    <div class="res-stat-label">需求文档</div>
                  </div>
                  <div class="res-stat-card" @click="resourcePanel.activeTab = 'data'">
                    <div class="res-stat-icon data"><el-icon><DataLine /></el-icon></div>
                    <div class="res-stat-num">{{ currentProject.resources?.filter(r => r.resource_type === 'data').length || 0 }}</div>
                    <div class="res-stat-label">数据资源</div>
                  </div>
                  <div class="res-stat-card" @click="resourcePanel.activeTab = 'knowledge'">
                    <div class="res-stat-icon kb"><el-icon><Files /></el-icon></div>
                    <div class="res-stat-num">{{ currentProject.resources?.filter(r => r.resource_type === 'knowledge').length || 0 }}</div>
                    <div class="res-stat-label">知识库</div>
                  </div>
                  <div class="res-stat-card" @click="resourcePanel.activeTab = 'code'">
                    <div class="res-stat-icon code"><el-icon><Files /></el-icon></div>
                    <div class="res-stat-num">{{ currentProject.code_path ? 1 : 0 }}</div>
                    <div class="res-stat-label">代码路径</div>
                  </div>
                  <div class="res-stat-card" @click="resourcePanel.activeTab = 'task'">
                    <div class="res-stat-icon task"><el-icon><List /></el-icon></div>
                    <div class="res-stat-num">{{ currentProject.resources?.filter(r => r.resource_type === 'task').length || 0 }}</div>
                    <div class="res-stat-label">任务</div>
                  </div>
                </div>
                <div class="res-meta">
                  <div class="res-meta-item"><span class="res-meta-label">项目类型：</span>{{ PROJECT_CATEGORIES.find(c => c.key === currentProject.category)?.label || currentProject.category || '未分类' }}</div>
                  <div class="res-meta-item"><span class="res-meta-label">代码路径：</span>{{ currentProject.code_path || '未配置' }}</div>
                  <div class="res-meta-item"><span class="res-meta-label">技术栈：</span>{{ currentProject.tech_stack || '未配置' }}</div>
                  <div class="res-meta-item"><span class="res-meta-label">创建时间：</span>{{ currentProject.created_at || '-' }}</div>
                </div>
              </div>
            </el-tab-pane>

            <!-- 知识图谱 -->
            <el-tab-pane label="知识图谱" name="graph">
              <div class="res-list">
                <div v-if="!currentProject.resources?.filter(r => r.resource_type === 'graph').length" class="res-empty">暂无知识图谱，AI对话中可生成</div>
                <div v-for="r in currentProject.resources?.filter(r => r.resource_type === 'graph')" :key="r.id" class="res-item">
                  <el-icon class="res-item-icon graph"><Share /></el-icon>
                  <div class="res-item-info">
                    <div class="res-item-name">{{ r.name || r.title || '未命名图谱' }}</div>
                    <div class="res-item-meta">{{ r.resource_type }} · {{ r.created_at || '' }}</div>
                  </div>
                  <el-button size="small" text @click="router.push('/graph')">查看</el-button>
                </div>
              </div>
            </el-tab-pane>

            <!-- 需求文档 -->
            <el-tab-pane label="需求" name="requirement">
              <div class="res-list">
                <div v-if="!currentProject.resources?.filter(r => r.resource_type === 'requirement').length" class="res-empty">暂无需求文档，AI对话中可生成</div>
                <div v-for="r in currentProject.resources?.filter(r => r.resource_type === 'requirement')" :key="r.id" class="res-item">
                  <el-icon class="res-item-icon req"><Document /></el-icon>
                  <div class="res-item-info">
                    <div class="res-item-name">{{ r.name || r.title || '未命名需求' }}</div>
                    <div class="res-item-meta">{{ r.resource_type }} · {{ r.created_at || '' }}</div>
                  </div>
                  <el-button size="small" text @click="router.push('/caomei')">查看</el-button>
                </div>
              </div>
            </el-tab-pane>

            <!-- 数据资源 -->
            <el-tab-pane label="数据" name="data">
              <div class="res-list">
                <div v-if="!currentProject.resources?.filter(r => r.resource_type === 'data').length" class="res-empty">暂无数据资源</div>
                <div v-for="r in currentProject.resources?.filter(r => r.resource_type === 'data')" :key="r.id" class="res-item">
                  <el-icon class="res-item-icon data"><DataLine /></el-icon>
                  <div class="res-item-info">
                    <div class="res-item-name">{{ r.name || r.title || '未命名数据' }}</div>
                    <div class="res-item-meta">{{ r.resource_type }} · {{ r.created_at || '' }}</div>
                  </div>
                </div>
              </div>
            </el-tab-pane>

            <!-- 知识库 -->
            <el-tab-pane label="知识库" name="knowledge">
              <div class="res-list">
                <div v-if="!currentProject.resources?.filter(r => r.resource_type === 'knowledge').length" class="res-empty">暂无知识库文档</div>
                <div v-for="r in currentProject.resources?.filter(r => r.resource_type === 'knowledge')" :key="r.id" class="res-item">
                  <el-icon class="res-item-icon kb"><Files /></el-icon>
                  <div class="res-item-info">
                    <div class="res-item-name">{{ r.name || r.title || '未命名文档' }}</div>
                    <div class="res-item-meta">{{ r.resource_type }} · {{ r.created_at || '' }}</div>
                  </div>
                  <el-button size="small" text @click="router.push('/knowledge-base')">查看</el-button>
                </div>
              </div>
            </el-tab-pane>

            <!-- 代码路径 -->
            <el-tab-pane label="代码" name="code">
              <div class="res-code">
                <div class="res-code-item">
                  <div class="res-code-label">代码仓库路径</div>
                  <el-input v-model="currentProject.code_path" placeholder="如：D:/projects/my-app 或 git@github.com:org/repo.git" />
                </div>
                <div class="res-code-item">
                  <div class="res-code-label">技术栈</div>
                  <el-input v-model="currentProject.tech_stack" placeholder="如：Vue3+SpringBoot+MySQL+Redis" />
                </div>
                <div class="res-code-actions">
                  <el-button type="primary" size="small" @click="saveProjectFromChat">保存配置</el-button>
                  <el-button size="small" @click="ElMessage.info('正在打开代码编辑器...')">打开编辑器</el-button>
                </div>
              </div>
            </el-tab-pane>

            <!-- 任务 -->
            <el-tab-pane label="任务" name="task">
              <div class="res-list">
                <div v-if="!currentProject.resources?.filter(r => r.resource_type === 'task').length" class="res-empty">暂无任务，AI对话中可「转任务」</div>
                <div v-for="r in currentProject.resources?.filter(r => r.resource_type === 'task')" :key="r.id" class="res-item">
                  <el-icon class="res-item-icon task"><List /></el-icon>
                  <div class="res-item-info">
                    <div class="res-item-name">{{ r.name || r.title || '未命名任务' }}</div>
                    <div class="res-item-meta">{{ r.resource_type }} · {{ r.created_at || '' }}</div>
                  </div>
                  <el-button size="small" text @click="router.push('/tasks')">查看</el-button>
                </div>
              </div>
            </el-tab-pane>
          </el-tabs>
        </template>
        <el-empty v-else description="请先选择项目" />
      </el-drawer>

      <!-- 全维分析 · 5 阶段 Chip 指示器（FR11） -->
      <div class="analysis-stages" aria-label="全维分析阶段">
        <div
          v-for="(stage, i) in ANALYSIS_STAGES"
          :key="stage.key"
          class="stage-chip"
          :class="[{ active: currentStage >= i && requirementFlowMode }, `stage-${stage.key}`]"
          @click="jumpAnalysisStage(i)"
        >
          <span class="stage-dot"></span>
          <span class="stage-label">{{ stage.label }}</span>
          <span v-if="i < 4" class="stage-arrow" aria-hidden="true">›</span>
        </div>
      </div>

      <!-- 后端历史恢复 -->
      <el-dialog v-model="historyOpen" title="从后端恢复会话" width="440px">
        <div class="hist-tip">这些会话由后端持久化（跨设备共享），点击即可载入对话历史。</div>
        <el-empty v-if="!backendSessions.length" description="暂无后端会话" :image-size="60" />
        <div v-else class="hist-list">
          <div
            class="hist-item"
            v-for="s in backendSessions"
            :key="s.id"
            @click="restoreFromBackend(s)"
          >
            <div class="hist-title">{{ s.title || s.id }}</div>
            <div class="hist-meta">{{ s.id }} · {{ s.updated_at || '' }}</div>
          </div>
        </div>
      </el-dialog>

      <!-- 调试指示器（可删除） -->
      <div v-if="requirementFlowMode" class="flow-mode-indicator">
        流程模式已开启
      </div>

      <div ref="scrollEl" class="chat-body">
        <!-- 需求流程模式：问题选择面板（重构版 · 分组卡片） -->
        <div v-if="requirementFlowMode && !currentIssue" class="flow-empty">
          <div class="flow-empty-header">
            <div class="flow-title-row">
              <span class="flow-icon-badge">🎯</span>
              <div>
                <div class="flow-title">选择问题类型</div>
                <div class="flow-desc">选一个方向，AI 帮你从需求到交付全链路搞定</div>
              </div>
            </div>
          </div>

          <!-- 分组：分析设计 -->
          <div class="issue-group">
            <div class="issue-group-title">
              <span class="group-dot" style="background:#6366f1"></span>
              分析设计
            </div>
            <div class="issue-grid">
              <div
                v-for="issue in issueGroups.analysis"
                :key="issue.key"
                class="issue-card"
                :class="{ active: selectedIssue?.key === issue.key }"
                @click="selectIssue(issue)"
              >
                <div class="issue-icon" :style="{ background: issue.color + '18', color: issue.color }">
                  {{ issue.emoji }}
                </div>
                <div class="issue-info">
                  <div class="issue-name">{{ issue.label }}</div>
                  <div class="issue-desc">{{ issue.desc }}</div>
                </div>
                <div v-if="issue.primary" class="issue-badge">推荐</div>
              </div>
            </div>
          </div>

          <!-- 分组：开发集成 -->
          <div class="issue-group">
            <div class="issue-group-title">
              <span class="group-dot" style="background:#10b981"></span>
              开发集成
            </div>
            <div class="issue-grid">
              <div
                v-for="issue in issueGroups.dev"
                :key="issue.key"
                class="issue-card"
                :class="{ active: selectedIssue?.key === issue.key }"
                @click="selectIssue(issue)"
              >
                <div class="issue-icon" :style="{ background: issue.color + '18', color: issue.color }">
                  {{ issue.emoji }}
                </div>
                <div class="issue-info">
                  <div class="issue-name">{{ issue.label }}</div>
                  <div class="issue-desc">{{ issue.desc }}</div>
                </div>
              </div>
            </div>
          </div>

          <!-- 自定义问题卡片 -->
          <div class="custom-issue-card" :class="{ active: selectedIssue?.isCustom }">
            <div class="custom-icon" style="background:#f59e0b18;color:#f59e0b">💡</div>
            <div class="custom-info">
              <div class="custom-name">自定义问题</div>
              <div class="custom-input-row">
                <el-input
                  v-model="customIssueText"
                  placeholder="输入你的问题，系统将全维分析处理..."
                  size="default"
                  @keyup.enter="createCustomIssue"
                />
                <el-button
                  type="primary"
                  :disabled="!customIssueText.trim()"
                  @click="createCustomIssue"
                >
                  添加
                </el-button>
              </div>
            </div>
          </div>

          <!-- 启动栏（选中后显示） -->
          <div v-if="selectedIssue" class="flow-start-bar">
            <div class="flow-start-info">
              <span class="start-icon">{{ selectedIssue.emoji }}</span>
              <div class="start-text">
                <div class="start-name">{{ selectedIssue.label }}</div>
                <div class="start-desc">{{ selectedIssue.desc }}</div>
              </div>
            </div>
            <div class="flow-start-actions">
              <el-button type="primary" size="large" @click="startFlow">🚀 开始</el-button>
            </div>
          </div>
        </div>

        <!-- 需求流程模式：进度追踪 -->
        <div v-if="requirementFlowMode && currentIssue" class="flow-tracker">
          <div class="flow-tracker-header">
            <span class="flow-track-icon">{{ currentIssue.emoji }}</span>
            <span class="flow-track-title">{{ currentIssue.label }}</span>
            <span class="flow-track-status">{{ FLOW_STAGES[currentStage].label }}</span>
            <el-button size="small" text @click="resetFlowIssue">🔄 更换问题</el-button>
          </div>
          <div class="flow-stages">
            <div
              v-for="(stage, i) in FLOW_STAGES"
              :key="i"
              class="stage-node"
              :class="{
                active: i === currentStage,
                done: i < currentStage,
              }"
            >
              <div class="stage-circle">{{ i < currentStage ? '✓' : i + 1 }}</div>
              <div class="stage-label">{{ stage.label }}</div>
            </div>
            <div
              v-for="(stage, i) in FLOW_STAGES.slice(0, -1)"
              :key="'line-' + i"
              class="stage-line"
              :class="{ done: i < currentStage }"
            ></div>
          </div>
          <div class="flow-stage-hint">{{ FLOW_STAGES[currentStage].hint }}</div>
          <div class="flow-stage-actions">
            <el-button
              size="small"
              v-if="currentStage > 0"
              @click="currentStage--"
            >
              ← 上一阶段
            </el-button>
            <el-button
              size="small"
              type="primary"
              v-if="currentStage < 5"
              @click="advanceStage"
            >
              下一阶段 →
            </el-button>
          </div>
          <div class="flow-quick-actions">
            <el-button size="small" @click="fullAnalysis" :loading="analyzing">🔍 全维分析</el-button>
            <el-button size="small" @click="generateRequirementDoc" :loading="generatingDoc">📝 需求文档</el-button>
            <el-button size="small" @click="generateFlowDiagram" :loading="generatingDiagram">🔄 流程图</el-button>
            <el-button size="small" @click="generateRequirementGraph" :loading="generatingGraph">📊 知识图谱</el-button>
            <el-button size="small" type="primary" @click="doDevTestFix" :loading="devTesting">💻 开发测试</el-button>
          </div>
        </div>

        <!-- 需求流程模式：成果展示区 -->
        <div v-if="requirementFlowMode && flowResults.length" class="flow-results">
          <div class="results-header">
            <span class="results-title">🎁 全维成果</span>
            <el-button size="small" text @click="flowResults = []">清空</el-button>
          </div>
          <div class="results-list">
            <div
              v-for="(r, i) in flowResults"
              :key="r.id"
              class="result-item"
              :class="[r.type, { expanded: r.expanded }]"
            >
              <div style="display: flex; gap: 10px; flex: 1;">
                <div class="result-icon">{{ r.icon }}</div>
                <div class="result-content" v-if="!r.expanded">
                  <div class="result-title">{{ r.title }}</div>
                  <div class="result-body">{{ r.content }}</div>
                </div>
                <div class="result-content" v-else>
                  <div class="result-title">{{ r.title }}</div>
                  <div class="result-body">{{ r.content }}</div>
                  <div class="result-detail">{{ r.detail }}</div>
                </div>
                <div v-if="r.expandable" class="result-actions">
                  <el-button size="small" text @click="toggleResultExpand(i)">
                    {{ r.expanded ? '收起' : '查看详情' }}
                  </el-button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="!messages.length && !requirementFlowMode" class="empty">
          <div class="empty-orb"><el-icon><ChatLineRound /></el-icon></div>
          <p class="empty-title">我是璇玑 AI 助手，帮你从需求到交付全链路搞定</p>
          <!-- 操作引导：3步快速上手 -->
          <div class="quick-steps">
            <div class="qs-item">
              <div class="qs-num">1</div>
              <div class="qs-text">选择专家模式<br><span>通用/算法/架构/算子等8种</span></div>
            </div>
            <div class="qs-arrow">→</div>
            <div class="qs-item">
              <div class="qs-num">2</div>
              <div class="qs-text">输入问题或点快捷问法<br><span>支持联网搜索/语音输入</span></div>
            </div>
            <div class="qs-arrow">→</div>
            <div class="qs-item">
              <div class="qs-num">3</div>
              <div class="qs-text">用全维操作沉淀成果<br><span>转任务/建项目/生成文档/流程图</span></div>
            </div>
          </div>
          <p v-if="autoTaskMode" class="task-mode-hint">🚀 任务模式已开启：对话中将自动创建并执行任务</p>
          <div class="suggestions-grid" aria-label="快捷问法">
            <div
              v-for="(q, i) in quickQuestionsFull"
              :key="i"
              class="qq-card"
              @click="sendQuick(q.prompt)"
            >
              <div class="qq-icon" :class="`qq-icon-${i % 6}`">{{ q.icon }}</div>
              <div class="qq-body">
                <div class="qq-title">{{ q.title }}</div>
                <div class="qq-desc">{{ q.desc }}</div>
              </div>
              <div class="qq-arrow" aria-hidden="true">→</div>
            </div>
          </div>
        </div>
        <div v-else-if="messages.length && requirementFlowMode && currentIssue" class="flow-current-issue-bar">
          <div class="flow-bar-content">
            <span class="flow-bar-icon">{{ currentIssue.emoji }}</span>
            <span class="flow-bar-title">{{ currentIssue.label }}</span>
            <el-button size="small" text @click="resetFlowIssue">🔄 更换问题</el-button>
          </div>
        </div>
        <template v-else>
          <MessageBubble
            v-for="(m, i) in messages"
            :key="i"
            :msg="m"
            :session-messages="messages"
            @goto-task="goToTaskDetail"
            @rate="(m2,r)=>onRate(m2,r)"
            @share="m2=>onShare(m2)"
            @regenerate="m2=>onRegenerate(m2)"
            @to-doc="(m2,p)=>onToDoc(m2,p)"
            @favorite="(m2,f)=>onFavorite(m2,f)"
            @followup="(m2,prompt)=>onFollowup(m2,prompt)"
            @feedback="(m2,payload)=>onFeedback(m2,payload)"
          />
        </template>
        <div v-if="thinking" class="typing">
          <span></span><span></span><span></span>
        </div>
      </div>

      <!-- 项目一体化产物面板 · 产品专家联盟交付 -->
      <div v-if="projectArtifact" class="artifact-panel">
        <div class="art-head">
          <div class="art-title">
            <span class="art-emoji">🏗️</span>
            <span v-if="projectArtifact.project" class="art-proj">
              {{ projectArtifact.project.name }}
              <el-tag size="small" effect="plain" type="info">{{ projectArtifact.project.category }}</el-tag>
            </span>
            <span v-else class="art-proj">联盟交付物</span>
            <span class="art-meta muted">
              <template v-if="projectArtifact.alliance_plan">
                联盟评分 <b :class="{ hl: projectArtifact.alliance_plan.overall_score >= 80 }">
                  {{ projectArtifact.alliance_plan.overall_score }}/100
                </b>
                <el-tag
                  size="small"
                  :type="projectArtifact.alliance_plan.overall_verdict === 'RELEASE_L3_PASS' ? 'success'
                          : projectArtifact.alliance_plan.overall_verdict === 'CONDITIONAL_L2_PASS' ? 'warning' : 'danger'"
                >
                  {{ projectArtifact.alliance_plan.overall_verdict }}
                </el-tag>
              </template>
              <template v-if="projectArtifact.kb_published">
                · 云盘归档 <b>{{ projectArtifact.kb_published.published_count }}</b> 份
              </template>
            </span>
          </div>
          <div class="art-actions">
            <el-button size="small" type="primary" plain @click="goToProject" v-if="projectArtifact.project">
              前往项目中心
            </el-button>
            <el-button size="small" @click="runAllianceOnly" :loading="devTesting">
              🏛️ 重跑联盟流水线
            </el-button>
            <el-button size="small" text @click="projectArtifact = null">关闭</el-button>
          </div>
        </div>
        <el-tabs v-model="artifactTab" size="small" class="art-tabs">
          <el-tab-pane label="📌 总览" name="overview">
            <div v-if="projectArtifact.project" class="over-grid">
              <div class="over-card">
                <div class="over-k">项目ID</div>
                <div class="over-v mono">{{ projectArtifact.project.id }}</div>
              </div>
              <div class="over-card">
                <div class="over-k">项目状态</div>
                <div class="over-v">{{ projectArtifact.project.status }}</div>
              </div>
              <div class="over-card">
                <div class="over-k">知识图谱</div>
                <div class="over-v">
                  {{ projectArtifact.requirement_graph?.nodes?.length || 0 }} 节点 /
                  {{ projectArtifact.requirement_graph?.edges?.length || 0 }} 边
                  <el-button link type="primary" @click="goToGraph">查看大图</el-button>
                </div>
              </div>
              <div class="over-card">
                <div class="over-k">云盘产物</div>
                <div class="over-v">
                  {{ projectArtifact.kb_published?.published_count || 0 }} 份
                </div>
              </div>
            </div>
            <div v-if="projectArtifact.kb_published?.documents?.length" class="kb-links">
              <div class="kb-title">📂 云盘知识库产物：</div>
              <div class="kb-list">
                <div
                  v-for="d in projectArtifact.kb_published.documents"
                  :key="d.id"
                  class="kb-item"
                  @click="goToKbDoc(d.id)"
                >
                  <span class="kb-tag">{{ d.category }}</span>
                  <span class="kb-name">{{ d.title }}</span>
                </div>
              </div>
            </div>
          </el-tab-pane>

          <el-tab-pane :label="`📊 图谱 (${projectArtifact?.requirement_graph?.nodes?.length || 0})`" name="graph">
            <div v-if="projectArtifact.requirement_graph" class="graph-mini">
              <div class="legend">
                <span class="lg lg-project">项目</span>
                <span class="lg lg-goal">目标</span>
                <span class="lg lg-actor">角色</span>
                <span class="lg lg-usecase">用例</span>
                <span class="lg lg-decision">决策</span>
                <span class="lg lg-data">数据</span>
                <span class="lg lg-end">终点</span>
              </div>
              <el-descriptions :column="2" size="small" border>
                <el-descriptions-item v-for="n in (projectArtifact.requirement_graph.nodes || []).slice(0, 16)" :key="n.id" :label="n.id" :label-class-name="'nd nd-'+n.type">
                  <span>{{ n.label }}</span>
                  <el-tag size="small" effect="plain" style="margin-left:6px">{{ n.type }}</el-tag>
                </el-descriptions-item>
              </el-descriptions>
              <div class="edges-box">
                <div class="edges-title">🔗 关系边 ({{ projectArtifact.requirement_graph.edges?.length || 0 }})</div>
                <div v-for="(e, i) in (projectArtifact.requirement_graph.edges || []).slice(0, 30)" :key="i" class="edge-row">
                  <code>{{ e.source }}</code>
                  <span class="edge-type">{{ e.type }}·{{ e.label }}</span>
                  <code>→ {{ e.target }}</code>
                </div>
              </div>
            </div>
          </el-tab-pane>

          <el-tab-pane label="🔄 流程图" name="flow">
            <pre class="mermaid-box">{{ projectArtifact.flow_diagram || '（暂无流程图）' }}</pre>
          </el-tab-pane>

          <el-tab-pane label="📝 PRD 需求文档" name="prd">
            <div class="md-box"><pre>{{ projectArtifact.requirement_doc || '（暂无文档）' }}</pre></div>
          </el-tab-pane>

          <el-tab-pane label="🗃️ 数据库 ERD + DDL" name="erd">
            <div class="erd-head">
              <h4>ER 关系图</h4>
            </div>
            <pre class="mermaid-box">{{ projectArtifact.erd?.erd || '' }}</pre>
            <div class="erd-head">
              <h4>DDL 建表语句</h4>
            </div>
            <pre class="code-box"><code>{{ projectArtifact.erd?.ddl || '' }}</code></pre>
            <div class="erd-head">
              <h4>数据表明细 ({{ projectArtifact.erd?.tables?.length || 0 }})</h4>
            </div>
            <el-table :data="projectArtifact.erd?.tables || []" size="small" border>
              <el-table-column prop="table" label="表名" width="220">
                <template #default="{ row }"><code>{{ row.table }}</code></template>
              </el-table-column>
              <el-table-column prop="comment" label="说明" />
              <el-table-column label="字段">
                <template #default="{ row }">
                  <div class="field-tags-row">
                    <el-tag v-for="f in (row.fields||[])" :key="f" size="small" effect="plain">{{ f }}</el-tag>
                  </div>
                </template>
              </el-table-column>
            </el-table>
          </el-tab-pane>

          <el-tab-pane :label="`🔗 需求-库关联 (${projectArtifact?.db_link?.coverage_score || 0})`" name="dblink">
            <div v-if="projectArtifact.db_link" class="db-head">
              <el-alert
                :title="`关联完整性评分：${projectArtifact.db_link.coverage_score}/100 · 级别：${projectArtifact.db_link.coverage_level}`"
                :type="projectArtifact.db_link.coverage_score >= 80 ? 'success' : projectArtifact.db_link.coverage_score >= 60 ? 'warning' : 'error'"
                :closable="false"
                show-icon
              />
              <div style="margin-top: 6px" class="muted">💡 {{ projectArtifact.db_link.recommendation }}</div>
            </div>
            <el-table :data="projectArtifact.db_link?.mapping_matrix || []" size="small" border stripe>
              <el-table-column prop="requirement_id" label="需求ID" width="90">
                <template #default="{row}"><b>{{ row.requirement_id }}</b></template>
              </el-table-column>
              <el-table-column prop="requirement_text" label="需求摘要" min-width="220" show-overflow-tooltip />
              <el-table-column prop="table_name" label="表" width="160">
                <template #default="{row}"><code>{{ row.table_name }}</code></template>
              </el-table-column>
              <el-table-column prop="field_name" label="字段" width="120">
                <template #default="{row}"><code>{{ row.field_name }}</code></template>
              </el-table-column>
              <el-table-column label="关联类型" width="90">
                <template #default="{row}">
                  <el-tag size="small" :type="row.association_type==='direct'?'success':row.association_type==='reference'?'warning':'info'">
                    {{ row.association_type }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column label="置信度" width="90">
                <template #default="{row}">{{ Math.round((row.coverage_confidence||0)*100) }}%</template>
              </el-table-column>
              <el-table-column prop="semantic_note" label="语义说明" min-width="200" show-overflow-tooltip />
            </el-table>
          </el-tab-pane>

          <el-tab-pane :label="`🏛️ 联盟流水线 (${projectArtifact?.alliance_plan?.overall_score || 0}/100)`" name="alliance">
            <div v-if="projectArtifact.alliance_plan" class="alliance-box">
              <div class="alliance-score-row">
                <div class="score-ring">
                  <div class="score-value">{{ projectArtifact.alliance_plan.overall_score }}</div>
                  <div class="score-label">总分 / 100</div>
                </div>
                <div class="alliance-summary">
                  <div class="verdict-line">
                    结论：
                    <el-tag
                      size="large"
                      :type="projectArtifact.alliance_plan.overall_verdict === 'RELEASE_L3_PASS' ? 'success'
                              : projectArtifact.alliance_plan.overall_verdict === 'CONDITIONAL_L2_PASS' ? 'warning' : 'danger'"
                    >
                      {{ projectArtifact.alliance_plan.overall_verdict }}
                    </el-tag>
                  </div>
                  <div>通过闸门：<b>{{ projectArtifact.alliance_plan.passed_gates }}</b> / {{ projectArtifact.alliance_plan.total_gates }}</div>
                  <div>执行模式：{{ projectArtifact.alliance_plan.mode }} · 耗时 {{ projectArtifact.alliance_plan.duration_ms }}ms</div>
                  <div class="muted">{{ projectArtifact.alliance_plan.recommendation }}</div>
                </div>
              </div>
              <div class="alliance-stages">
                <div
                  v-for="s in projectArtifact.alliance_plan.stages"
                  :key="s.index"
                  class="stage-card"
                  :class="{ pass: s.status==='pass' }"
                >
                  <div class="stage-top">
                    <span class="stage-no">{{ s.index }}</span>
                    <span class="stage-name">{{ s.title }}</span>
                    <el-tag size="small" effect="plain">{{ s.expert }}专家</el-tag>
                    <span class="stage-score" :class="{ low: s.gate_score < s.gate_threshold }">
                      {{ s.gate_score }}<span class="thr">/{{ s.gate_threshold }}</span>
                    </span>
                  </div>
                  <div class="stage-summary">{{ s.summary }}</div>
                  <div class="stage-deliver">
                    交付物：
                    <el-tag v-for="d in s.deliverables" :key="d" size="small" style="margin:2px">{{ d }}</el-tag>
                  </div>
                </div>
              </div>
              <div class="gates-title">🧱 闸门清单</div>
              <el-table :data="projectArtifact.alliance_plan.gates" size="small" border>
                <el-table-column prop="name" label="闸门" width="140">
                  <template #default="{row}"><code>{{ row.name }}</code></template>
                </el-table-column>
                <el-table-column prop="stage" label="阶段" />
                <el-table-column label="阈值" width="100">
                  <template #default="{row}">≥ {{ row.threshold }}</template>
                </el-table-column>
                <el-table-column label="实际" width="100">
                  <template #default="{row}">
                    <b :class="row.pass ? 'txt-ok' : 'txt-bad'">{{ row.actual }}</b>
                  </template>
                </el-table-column>
                <el-table-column label="结论" width="120">
                  <template #default="{row}">
                    <el-tag size="small" :type="row.pass ? 'success' : 'danger'">
                      {{ row.pass ? '✅ PASS' : '❌ FAIL' }}
                    </el-tag>
                  </template>
                </el-table-column>
              </el-table>
            </div>
          </el-tab-pane>
        </el-tabs>
      </div>

      <!-- 对话快捷操作栏：新建对话 / 新建项目 / 切换项目 / 管理项目 -->
      <div class="chat-quick-bar">
        <template v-if="chatMode === 'followup' && currentProject">
          <div class="qb-current">
            <span class="qb-icon">📂</span>
            <span class="qb-name">{{ currentProject.name }}</span>
            <el-tag size="small" effect="plain" :type="projectStatusType">{{ projectStatusLabel }}</el-tag>
          </div>
          <el-button size="small" text @click="exitFollowup">💬 退出对话</el-button>
        </template>
        <template v-else>
          <el-button size="small" text @click="newSession">💬 新建对话</el-button>
        </template>
        <div class="qb-divider"></div>
        <el-button size="small" text @click="openCreateProject">📁 新建项目</el-button>
        <el-dropdown trigger="click" @command="switchToProject" :disabled="!projectList.length">
          <el-button size="small" text :loading="projectListLoading">
            🔄 切换项目 <el-icon class="el-icon--right"><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item v-for="p in projectList" :key="p.id" :command="p">
                {{ p.name }}
                <el-tag v-if="p.status === 'active'" size="small" type="success" effect="plain" style="margin-left:8px">进行中</el-tag>
              </el-dropdown-item>
              <el-dropdown-item v-if="!projectList.length && !projectListLoading" disabled>暂无项目</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <el-button size="small" text @click="goProjects">⚙️ 管理项目</el-button>
      </div>

      <div class="chat-input" :class="{ 'is-share': isShareMode }">
        <div v-if="isShareMode" class="share-readonly-banner">
          <el-icon><Link /></el-icon>
          <span>分享对话 · 只读模式（如需续问，<b @click="forkShareSession">点此创建可编辑副本</b>）</span>
        </div>
        <el-input
          v-model="draft"
          type="textarea"
          :rows="2"
          resize="none"
          :disabled="isShareMode"
          :placeholder="isShareMode ? '分享快照不可直接发送 · 点击上方「创建副本」继续对话' : '输入消息，Enter 发送 / Shift+Enter 换行'"
          @keydown.enter.exact.prevent="send"
        />
        <div class="input-actions">
          <el-tooltip :content="webSearchEnabled ? '联网已开启：回答前先检索实时信息' : '联网已关闭：仅使用模型知识回答'" placement="top">
            <div class="web-toggle" :class="{ on: webSearchEnabled }" @click="webSearchEnabled = !webSearchEnabled">
              <el-icon><Link /></el-icon>
              <span>联网</span>
            </div>
          </el-tooltip>
          <el-tooltip :content="artifactTooltip" placement="top">
            <div class="web-toggle art-toggle" :class="{ 'doc-on': artifactMode === 'document', 'code-on': artifactMode === 'code' }" @click="cycleArtifactMode">
              <el-icon><Document /></el-icon>
              <span>{{ artifactLabel }}</span>
            </div>
          </el-tooltip>
          <!-- T12 麦克风 UI + 语音状态 switch -->
          <div class="voice-row" @click.stop>
            <el-switch
              v-model="voiceUiOpen"
              size="small"
              active-text="语音"
              inactive-text="静音"
              @change="() => { if (voiceUiOpen) refreshVoiceHealth(); else voiceHealth = null }"
            />
            <button
              class="mic-btn"
              :class="{ recording: isRecording }"
              @click.stop="toggleMicRecording"
              :title="isRecording ? '点击停止录音并识别' : '点击开始录音（Web Speech → Rust ASR）'"
            >
              <el-icon :size="18"><Microphone /></el-icon>
              <span class="mic-level-bar">
                <span class="mic-level-fill" :style="{ height: (micLevel * 100).toFixed(0) + '%' }" />
              </span>
            </button>
          </div>
          <el-button type="primary" :loading="thinking" :disabled="isShareMode" @click="send">
            <el-icon><Promotion /></el-icon> 发送
          </el-button>
        </div>
      </div>
    </div>

    <!-- Right Tool Dock -->
    <ToolDock :active-tool="activeTool" @select="openTool" />

    <!-- Tool Drawer -->
    <ToolDrawer :visible="drawerVisible" :tool="activeTool" @close="closeTool" />

    <!-- 🏗️ 对话中创建项目 · 产品专家联盟一键生成 -->
    <el-dialog
      v-model="projectDlg.visible"
      title="🏗️ 从对话创建项目 · 一键生成需求→图谱→文档→云盘→联盟流水线"
      width="680px"
      :close-on-click-modal="false"
    >
      <el-form :model="projectDlg.form" label-width="90px" label-position="left">
        <el-form-item label="项目名称" required>
          <el-input v-model="projectDlg.form.name" maxlength="80" show-word-limit placeholder="请输入项目名称，例如：企业级CRM客户管理平台" />
        </el-form-item>
        <el-form-item label="项目类别">
          <el-radio-group v-model="projectDlg.form.category" style="display:flex; flex-wrap:wrap; gap:8px;">
            <el-radio-button
              v-for="c in PROJECT_CATEGORIES"
              :key="c.key"
              :label="c.key"
              size="default"
            >
              {{ c.label }}
            </el-radio-button>
          </el-radio-group>
          <div class="muted" style="margin-top:4px">{{ PROJECT_CATEGORIES.find(c=>c.key===projectDlg.form.category)?.desc }}</div>
        </el-form-item>
        <el-form-item label="项目描述">
          <el-input v-model="projectDlg.form.description" type="textarea" :rows="3"
            placeholder="简述项目背景、目标与范围（可留空，默认从对话内容提取）" />
        </el-form-item>
        <el-form-item label="默认负责人">
          <el-input v-model="projectDlg.form.owner" placeholder="默认：ai-alliance（产品专家联盟）" />
        </el-form-item>
        <el-form-item label="需求输入">
          <div class="req-source">
            <el-checkbox v-model="projectDlg.form.useChatContext" border>
              ✨ 复用当前对话内容（推荐）
            </el-checkbox>
            <el-checkbox v-model="projectDlg.form.autoPipeline" border>
              🏛️ 创建后自动执行 6 阶段联盟流水线
            </el-checkbox>
          </div>
        </el-form-item>
      </el-form>
      <div class="dlg-tip">
        <el-alert
          title="一键将创建：项目中心记录、需求知识图谱、业务流程图、PRD 需求文档、数据库 ERD+DDL、需求-库关联矩阵，并全部归档到知识库（云盘）。"
          type="info"
          :closable="false"
          show-icon
        />
      </div>
      <template #footer>
        <el-button @click="projectDlg.visible = false">取消</el-button>
        <el-button
          type="primary"
          :loading="projectDlg.submitting"
          @click="submitProjectFromChat"
        >
          {{ projectDlg.submitting ? '正在创建：图谱 → 文档 → 库 → 联盟流水线…' : '✅ 一键创建项目并生成全部交付物' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 对话转任务 -->
    <el-dialog v-model="convertDialogOpen" title="对话转任务" width="520px">
      <div v-if="!convertResult && convertingTask" class="convert-loading">
        <el-icon class="spin"><Loading /></el-icon>
        <span>AI 正在分析对话内容，提取任务信息...</span>
      </div>
      <div v-else-if="convertResult" class="convert-result">
        <el-alert v-if="convertResult.note" :title="convertResult.note" type="warning" :closable="false" style="margin-bottom: 12px" />
        <div class="result-section">
          <div class="result-label">📋 任务标题</div>
          <div class="result-value">{{ convertResult.task?.title || '未命名' }}</div>
        </div>
        <div class="result-section" v-if="convertResult.task?.description">
          <div class="result-label">📝 任务描述</div>
          <div class="result-value">{{ convertResult.task.description }}</div>
        </div>
        <div class="result-section" v-if="convertResult.task?.steps?.length">
          <div class="result-label">📌 执行步骤</div>
          <div class="result-steps">
            <div v-for="(s, i) in convertResult.task.steps" :key="i" class="result-step">
              <span class="step-num">{{ i + 1 }}</span> {{ s }}
            </div>
          </div>
        </div>
        <div class="result-meta">
          <el-tag :type="priorityTagType(convertResult.task?.priority)">优先级: {{ convertResult.task?.priority }}</el-tag>
          <el-tag type="info">状态: {{ convertResult.task?.status }}</el-tag>
        </div>
      </div>
      <div v-else class="convert-empty">
        <el-empty description="暂无对话可转换，请先进行对话" />
      </div>
      <template #footer>
        <el-button @click="convertDialogOpen = false">关闭</el-button>
        <el-button v-if="convertResult" type="primary" @click="goToTasks">
          <el-icon><List /></el-icon> 前往任务管理
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, nextTick, onMounted, onUnmounted, onBeforeUnmount, watch, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { List, Loading, ArrowDown, Link, Document, FolderAdd, FolderOpened, ChatDotRound, ChatLineRound, Delete, Upload, Download, Clock, Promotion, DocumentAdd, Microphone, Plus, Edit, Files, Setting, DataLine, Connection } from '@element-plus/icons-vue'
import MessageBubble from '@/components/MessageBubble.vue'
import SessionSidebar from '@/components/SessionSidebar.vue'
import ToolDock from '@/components/ToolDock.vue'
import ToolDrawer from '@/components/ToolDrawer.vue'
import { AI_EXPERT_PRESETS } from '@/types'
import { useProject } from '@/composables/projectContext.js'
import {
  aiChat,
  aiExpertChat,
  getAutoSyncStatus,
  toggleAutoSync,
  graphExport,
  graphImport,
  listDialogueSessions,
  getChatHistory,
  convertChatToTask,
  autoCreateTask,
  aiCaomeiParse,
  aiFullAnalysis,
  aiGenerateDoc,
  aiGenerateFlowDiagram,
  aiDevTestFix,
  aiFullComplete,
  aiOptimizeDoc,
  aiProjectFromChat,
  aiGenerateProjectGraph,
  aiLinkReqToDb,
  allianceEnterprisePipeline,
  aiGenerateErd,
  kbGetDocument,
  getProject,
  createProject,
  updateProject,
  getProjects
} from '@/api'

// ===== T11 专家联盟 SSE =====
import {
  runAllianceFullSSE,
  getAllianceCapabilities,
  getVoiceHealth,
} from '@/api/alliance'

// 5 阶段 Chip（AC-10）——前端展示用 5 个：Intent → Team → Debate → Gate → Done，Learn 隐藏
const PHASE_CHIPS = ['intent', 'team', 'debate', 'gate', 'done']
const allianceRunning = ref(false)
const alliancePhase = ref(null)
const allianceTraceId = ref('')
const allianceCapabilities = ref(null)
// FR11 与 T11 桥接：6 阶段 alliance SSE → 驱动 5 段式 Chip（需求/架构/实现/测试/验收）
// 解决：之前点击全维分析后 SSE 6 阶段完整跑完但 Chip 始终停在「需求」阶段的脱节问题。
watch(alliancePhase, (p) => {
  if (!p) return
  requirementFlowMode.value = true
  const idxMap = {
    intent: 0,      // 需求
    team: 1,        // 架构
    debate: 1,      // 架构
    synthesize: 2,  // 实现
    gate: 3,        // 测试
    learn: 4,       // 验收
    done: 4,        // 验收
  }
  const idx = idxMap[p] ?? 0
  if (idx >= currentStage.value) {
    currentStage.value = idx
  }
})

// T12 语音状态
const voiceHealth = ref(null)
const voiceUiOpen = ref(false)
const isRecording = ref(false)
const micLevel = ref(0) // 0..1
let audioCtx = null
let micAnalyser = null
let micRafId = 0
let micStream = null

const router = useRouter()
const route = useRoute()

const sessions = ref([])
const currentSession = ref(null)
const messages = ref([])
// 所有会话的消息映射（单一事实源）：切换会话不再丢失历史
const messagesMap = ref({})
const draft = ref('')
const thinking = ref(false)
const online = ref(false)
const scrollEl = ref(null)
// 联网搜索开关：开启后每轮对话先检索实时信息再回答（持久化记忆）
const webSearchEnabled = ref(localStorage.getItem('ous_web_search') === '1')
watch(webSearchEnabled, (v) => {
  localStorage.setItem('ous_web_search', v ? '1' : '0')
})
// 本地制品模式：off / document / code（开启后 AI 自动在本机创建文档/代码文件，持久化记忆）
const artifactMode = ref(localStorage.getItem('ous_artifact_mode') || 'off')
watch(artifactMode, (v) => {
  localStorage.setItem('ous_artifact_mode', v)
})
const artifactLabel = computed(() =>
  artifactMode.value === 'document' ? '文档' : artifactMode.value === 'code' ? '代码' : '本地'
)
const artifactTooltip = computed(() => {
  if (artifactMode.value === 'document') return '文档模式已开启：AI 自动在本机 workspace/artifacts/ 创建 Markdown 文档'
  if (artifactMode.value === 'code') return '代码模式已开启：AI 自动在本机 workspace/artifacts/ 创建源码文件'
  return '本地制品已关闭：点击切换 文档模式 → 代码模式 → 关闭'
})
function cycleArtifactMode() {
  artifactMode.value = artifactMode.value === 'off' ? 'document' : artifactMode.value === 'document' ? 'code' : 'off'
}
// 对话自动→知识图谱 全自动同步开关（默认开）
const autoSync = ref(true)
const importInput = ref(null)
// 专家模式
const selectedExpert = ref(AI_EXPERT_PRESETS[0])
// 项目上下文
const { currentProject } = useProject()
// 项目列表（用于切换项目下拉）
const projectList = ref([])
const projectListLoading = ref(false)
async function loadProjectList() {
  projectListLoading.value = true
  try {
    const r = await getProjects()
    projectList.value = Array.isArray(r) ? r : (r.projects || r.data || r.list || [])
  } catch (e) {
    projectList.value = []
  } finally {
    projectListLoading.value = false
  }
}
function switchToProject(p) {
  if (!p) return
  chatMode.value = 'followup'
  ElMessage.success(`已切换到项目：${p.name}`)
  loadProjectList()
}
function exitFollowup() {
  chatMode.value = 'default'
  ElMessage.info('已退出跟进项目，回到默认对话')
}
// 对话模式：default 默认对话 / project 项目对话
const chatMode = ref('default')
// 项目对话开发阶段：0需求 1架构 2实现 3测试 4验收
const devStage = ref(0)
const DEV_STAGES = [
  { key: 'req', label: '需求', expert: 'general', hint: '需求分析与业务建模' },
  { key: 'arch', label: '架构', expert: 'architecture', hint: '系统架构设计与技术选型' },
  { key: 'impl', label: '实现', expert: 'operator', hint: '代码实现与算子编排' },
  { key: 'test', label: '测试', expert: 'automation', hint: '测试策略与自动化测试' },
  { key: 'accept', label: '验收', expert: 'fusion', hint: '全维验收与发布准备' }
]
// 项目资源面板
const resourcePanel = ref({ visible: false, activeTab: 'overview', loading: false })
// AI全维开发状态
const fullDevRunning = ref(false)
const fullDevStage = ref(0)
const FULL_DEV_STAGES = ['需求分析', '架构设计', '代码生成', '测试验证', '部署上线']
// 流式打字定时器句柄：组件卸载时必须清理，避免泄漏
let streamTimer = null

// 后端会话历史（跨设备恢复）
const historyOpen = ref(false)
const backendSessions = ref([])
const loadingHistory = ref(false)

// 对话转任务
const convertDialogOpen = ref(false)
const convertingTask = ref(false)
const convertResult = ref(null)

// 自动任务模式：对话中自动创建并执行任务
const autoTaskMode = ref(false)

// 需求流程模式
const requirementFlowMode = ref(false)
const selectedIssue = ref(null)
const currentIssue = ref(null)
const currentStage = ref(0)
const generatingGraph = ref(false)
const customIssueText = ref('')
const flowResults = ref([])

// 全维操作状态
const analyzing = ref(false)
const generatingDoc = ref(false)
const generatingDiagram = ref(false)
const devTesting = ref(false)

// ========== 对话内项目创建 · 产品专家联盟 ==========
const PROJECT_CATEGORIES = [
  { key: 'platform', label: '平台系统', color: '#6366f1', desc: '基础平台/中台/网关类' },
  { key: 'business', label: '业务系统', color: '#0ea5e9', desc: 'CRM/ERP/财务/运营等业务' },
  { key: 'custom', label: '自定义项目', color: '#64748b', desc: '其他类型' },
  { key: 'algorithm', label: '算法工程', color: '#10b981', desc: '算法/模型/算子研发' },
  { key: 'graph', label: '图谱项目', color: '#7c3aed', desc: '知识图谱/关系挖掘类' },
  { key: 'app', label: '应用产品', color: '#f59e0b', desc: '独立APP/网页/SaaS产品' },
]
const projectDlg = ref({
  visible: false,
  isEdit: false,
  submitting: false,
  form: {
    name: '',
    category: 'platform',
    status: 'active',
    description: '',
    code_path: '',
    tech_stack: '',
    owner: 'ai-alliance',
    useChatContext: true,
    autoPipeline: true,
  }
})
const projectArtifact = ref(null) // 最近一次一键生成产物
const artifactTab = ref('graph')
const openProjectDialog = () => {
  // 预填充：标题取最近一条用户消息
  const recent = [...messages.value].reverse().find(m => m.role === 'user' && !m.system)
  projectDlg.value.form.name = recent ? String(recent.content).slice(0, 24) : ''
  projectDlg.value.form.description = messages.value
    .filter(m => m.role === 'user' && !m.system)
    .map(m => m.content).join('\n').slice(0, 600)
  projectDlg.value.visible = true
}
const submitProjectFromChat = async () => {
  const f = projectDlg.value.form
  if (!f.name || !String(f.name).trim()) {
    ElMessage.warning('请填写项目名称')
    return
  }
  projectDlg.value.submitting = true
  try {
    const payload = {
      name: String(f.name).trim(),
      category: f.category,
      description: f.description || '',
      owner: f.owner,
      requirement: f.useChatContext
        ? (messages.value.filter(m => m.role === 'user' && !m.system).map(m => m.content).join('\n') || f.description || f.name)
        : (f.description || f.name),
      session_id: currentSession.value,
      messages: messages.value.filter(m => m.content).map(m => ({ role: m.role, content: m.content })),
    }
    const result = await aiProjectFromChat(payload)
    projectArtifact.value = result
    // 会话消息内写一条系统卡片
    messages.value.push({
      role: 'system',
      content: `🏗️ 已在当前对话创建项目「${result.project.name}」\n` +
        `📊 需求图谱节点: ${result.requirement_graph?.nodes?.length || 0}\n` +
        `📝 云盘产物: ${result.kb_published?.published_count || 0} 份文档\n` +
        `🏛️ 联盟流水线: ${result.alliance_plan?.overall_score}/100 · ${result.alliance_plan?.overall_verdict}`,
      timestamp: Date.now(),
      system: true,
      project_from_chat: true,
      project_data: result,
    })
    ElMessage.success(`✅ 项目「${result.project.name}」已创建，联盟产物 ${result.kb_published?.published_count || 0} 份已归档云盘`)
    artifactTab.value = 'overview'
    projectDlg.value.visible = false
    projectDlg.value.form = { name: '', category: 'platform', description: '', owner: 'ai-alliance', useChatContext: true, autoPipeline: true }
    persist()
    await scroll()
  } catch (e) {
    console.error('[submitProjectFromChat]', e)
    ElMessage.error('项目创建失败：' + (e.message || '未知错误'))
  } finally {
    projectDlg.value.submitting = false
  }
}
// 独立运行联盟流水线
const runAllianceOnly = async () => {
  const ctx = messages.value.filter(m => m.role === 'user' && !m.system).map(m => m.content).join('\n') || (projectArtifact.value?.project?.name || '项目')
  devTesting.value = true
  try {
    const r = await allianceEnterprisePipeline({
      requirement: ctx,
      project_name: projectArtifact.value?.project?.name || '联盟评估',
      mode: 'parallel'
    })
    if (!projectArtifact.value) {
      projectArtifact.value = { alliance_plan: r }
    } else {
      projectArtifact.value = { ...projectArtifact.value, alliance_plan: r }
    }
    artifactTab.value = 'alliance'
    ElMessage.success(`🏛️ 联盟流水线完成：${r.overall_score}/100 · ${r.overall_verdict}`)
  } catch (e) {
    ElMessage.error('联盟流水线执行失败：' + (e.message || ''))
  } finally {
    devTesting.value = false
  }
}
const goToProject = () => {
  const pid = projectArtifact.value?.project?.id
  if (pid) router.push({ path: '/projects', query: { pid } })
}
const goToKbDoc = (docId) => {
  if (!docId) return
  router.push({ path: '/knowledge-base', query: { doc: docId } })
}
const goToGraph = () => {
  router.push('/graph')
}

// 问题类别（按分组整理，每组有独立主题色）
const ISSUE_CATEGORIES = [
  { key: 'requirement_graph', label: '需求知识图谱', emoji: '📊', desc: '生成项目需求的结构化知识图谱', primary: true, group: 'analysis', color: '#6366f1' },
  { key: 'business_process', label: '业务流程设计', emoji: '🔄', desc: '梳理和优化企业级业务流程', group: 'analysis', color: '#0ea5e9' },
  { key: 'system_arch', label: '系统架构设计', emoji: '🏗️', desc: '设计和评估系统架构方案', group: 'analysis', color: '#8b5cf6' },
  { key: 'plugin_dev', label: '插件开发', emoji: '🔌', desc: '开发和集成 AI 插件', group: 'dev', color: '#10b981' },
  { key: 'automation_flow', label: '自动化任务流程', emoji: '⚡', desc: '设计自动化执行流程', group: 'dev', color: '#f59e0b' },
  { key: 'mcp_integration', label: 'MCP 工具集成', emoji: '🔗', desc: '集成和配置 MCP 工具', group: 'dev', color: '#ef4444' }
]
// 分组计算属性
const issueGroups = computed(() => ({
  analysis: ISSUE_CATEGORIES.filter(i => i.group === 'analysis'),
  dev: ISSUE_CATEGORIES.filter(i => i.group === 'dev')
}))

// 6 阶段流程
const FLOW_STAGES = [
  { label: '设计', hint: '📝 请描述你的问题和需求，AI 将帮你进行设计分析...' },
  { label: '分析', hint: '🔍 AI 正在分析需求，梳理关键点和约束条件...' },
  { label: '开发', hint: '💻 基于分析结果，开始进行开发实现...' },
  { label: '测试', hint: '🧪 对开发成果进行测试验证...' },
  { label: '修复', hint: '🔧 修复发现的问题和缺陷...' },
  { label: '优化', hint: '✨ 持续优化和改进，生成最终成果...' }
]

function onRequirementFlowToggle(val) {
  if (val) {
    ElMessage.success('🎯 需求流程模式已开启')
  } else {
    currentIssue.value = null
    currentStage.value = 0
    selectedIssue.value = null
  }
  persist()
}

function resetFlowIssue() {
  currentIssue.value = null
  currentStage.value = 0
  selectedIssue.value = null
  flowResults.value = []
  persist()
  ElMessage.info('已重置，请重新选择问题')
}

function selectIssue(issue) {
  selectedIssue.value = issue
  persist()
}

// 统一入口：开始流程（合并原"快速分析"和"启动全维流程"）
async function startFlow() {
  if (!selectedIssue.value) return
  currentIssue.value = selectedIssue.value
  currentStage.value = 0
  const hasUserMsg = messages.value.some(m => m.role === 'user' && !m.system)
  if (!hasUserMsg) {
    // 无用户消息：自动触发全维分析
    const initMsg = `【${currentIssue.value.label}】⚡ 开始全维分析\n\n将自动完成：全维分析 → 需求文档 → 流程图 → 知识图谱 → 开发测试，请稍候...`
    draft.value = initMsg
    await send()
    setTimeout(() => fullAnalysis(), 2000)
  } else {
    // 有用户消息：进入对话引导
    const stage = FLOW_STAGES[0]
    const initMsg = `【${currentIssue.value.label}】进入${stage.label}阶段：${stage.hint}\n\n请详细描述你的需求背景、目标和关键约束，我将在此基础上进行设计分析。`
    draft.value = initMsg
    await send()
  }
}

async function startRequirementFlow() {
  if (!selectedIssue.value) return
  currentIssue.value = selectedIssue.value
  currentStage.value = 0
  // 自动进入对话：发送第一个引导消息
  const stage = FLOW_STAGES[0]
  const initMsg = `【${currentIssue.value.label}】进入${stage.label}阶段：${stage.hint}\n\n请详细描述你的需求背景、目标和关键约束，我将在此基础上进行设计分析。`
  draft.value = initMsg
  await send()
}

function advanceStage() {
  if (currentStage.value < FLOW_STAGES.length - 1) {
    currentStage.value++
    const stage = FLOW_STAGES[currentStage.value]
    messages.value.push({
      role: 'system',
      content: `📌 进入【${stage.label}】阶段：${stage.hint}`,
      timestamp: Date.now(),
      system: true
    })
    persist()
  }
}

async function generateRequirementGraph() {
  generatingGraph.value = true
  try {
    const text = messages.value
      .filter(m => m.role === 'user' && !m.system)
      .map(m => m.content)
      .join('\n')
    const r = await aiCaomeiParse({
      requirement: text,
      session_id: currentSession.value
    })
    if (r && r.graph) {
      messages.value.push({
        role: 'system',
        content: `✅ 需求知识图谱已生成！\n📊 节点数: ${r.graph.nodes?.length || 0}\n🔗 关系数: ${r.graph.edges?.length || 0}\n\n你可以在「知识图谱」页面查看完整图谱。`,
        timestamp: Date.now(),
        system: true,
        graph_data: r.graph
      })
      ElMessage.success('需求知识图谱生成成功！')
    } else {
      messages.value.push({
        role: 'system',
        content: '✅ 需求分析完成，但未检测到足够的需求内容。请补充更多需求描述。',
        timestamp: Date.now(),
        system: true
      })
    }
  } catch (e) {
    const nodes = []
    const edges = []
    const userMsgs = messages.value.filter(m => m.role === 'user' && !m.system)
    userMsgs.forEach((msg, i) => {
      nodes.push({ id: `req_${i}`, label: msg.content.slice(0, 30), type: 'requirement' })
    })
    messages.value.push({
      role: 'system',
      content: `📋 已基于对话内容构建基础需求图谱：\n📊 节点数: ${nodes.length}\n💡 可前往「知识图谱」页面查看并完善。`,
      timestamp: Date.now(),
      system: true,
      graph_data: { nodes, edges }
    })
    ElMessage.info('已生成基础需求图谱')
  } finally {
    generatingGraph.value = false
    persist()
  }
}

// ===== 自定义问题 =====
function createCustomIssue() {
  const text = customIssueText.value.trim()
  if (!text) return
  const issue = {
    key: 'custom_' + Date.now(),
    label: text.slice(0, 12),
    emoji: '🎯',
    desc: text,
    isCustom: true
  }
  ISSUE_CATEGORIES.push(issue)
  selectedIssue.value = issue
  customIssueText.value = ''
  ElMessage.success('自定义问题已添加')
}

// ===== 快速分析 =====
async function quickAnalyze() {
  if (!selectedIssue.value) return
  currentIssue.value = selectedIssue.value
  currentStage.value = 2 // 直接跳到开发阶段
  const initMsg = `【${currentIssue.value.label}】⚡ 快速全维分析模式\n\n将自动完成：全维分析 → 需求文档 → 流程图 → 开发测试 → 优化，请稍候...`
  draft.value = initMsg
  await send()
  // 自动触发全维分析
  setTimeout(() => fullAnalysis(), 2000)
}

// ===== 全维分析（真实 AI 驱动） =====
async function fullAnalysis() {
  analyzing.value = true
  const issue = currentIssue.value || selectedIssue.value
  if (!issue) return
  try {
    const userMsgs = messages.value.filter(m => m.role === 'user' && !m.system)
    const context = userMsgs.map(m => m.content).join('\n') || issue.desc
    
    ElMessage.info('🔍 正在进行 AI 全维分析，请稍候...')
    
    const result = await aiFullAnalysis({
      requirement: context,
      issue_type: issue.label,
      context: context
    })
    
    const analysisReport = result.analysis || '全维分析完成，但未获取到分析内容。'
    
    messages.value.push({
      role: 'assistant',
      content: analysisReport,
      timestamp: Date.now()
    })
    
    addFlowResult({
      type: 'analysis',
      icon: '🔍',
      title: '全维分析报告（AI 生成）',
      content: `已通过 AI 引擎完成 ${issue.label} 的全维分析，涵盖需求、业务、技术、风险、可行性 6 个维度。`,
      expandable: true,
      expanded: true,
      detail: analysisReport
    })
    
    if (currentStage.value < 1) currentStage.value = 1
    ElMessage.success('✅ AI 全维分析完成！')
    await scroll()
  } catch (e) {
    console.error('[fullAnalysis]', e)
    ElMessage.error('全维分析失败：' + e.message)
  } finally {
    analyzing.value = false
    persist()
  }
}

// ===== 生成需求文档 =====
async function generateRequirementDoc() {
  generatingDoc.value = true
  const issue = currentIssue.value || selectedIssue.value
  if (!issue) return
  try {
    const userMsgs = messages.value.filter(m => m.role === 'user' && !m.system)
    const context = userMsgs.map(m => m.content).join('\n') || issue.desc

    ElMessage.info('📝 AI 正在生成需求文档，请稍候...')

    const result = await aiGenerateDoc({
      requirement: context,
      issue_type: issue.label,
      context: context
    })

    const doc = result.document || result.doc || result.content || '需求文档生成完成，但未获取到文档内容。'

    messages.value.push({
      role: 'assistant',
      content: `📝 **需求文档已生成**\n\n${result.summary || 'AI 已根据对话内容生成需求文档，请查看详情。'}`,
      timestamp: Date.now()
    })

    addFlowResult({
      type: 'doc',
      icon: '📝',
      title: '需求文档（AI 生成）',
      content: `${issue.label} 的需求文档已通过 AI 引擎生成完成。`,
      expandable: true,
      expanded: true,
      detail: doc
    })

    if (currentStage.value < 1) currentStage.value = 1
    ElMessage.success('✅ 需求文档生成完成！')
    await scroll()
  } catch (e) {
    console.error('[generateRequirementDoc]', e)
    ElMessage.error('需求文档生成失败：' + e.message)
  } finally {
    generatingDoc.value = false
    persist()
  }
}

// ===== 优化需求文档 =====
async function optimizeRequirementDoc() {
  generatingDoc.value = true
  const issue = currentIssue.value || selectedIssue.value
  if (!issue) return
  try {
    const userMsgs = messages.value.filter(m => m.role === 'user' && !m.system)
    const context = userMsgs.map(m => m.content).join('\n') || issue.desc

    ElMessage.info('✨ AI 正在优化需求文档，请稍候...')

    const result = await aiOptimizeDoc({
      requirement: context,
      issue_type: issue.label,
      context: context
    })

    const optimizedDoc = result.document || result.doc || result.content || '文档优化完成，但未获取到优化内容。'

    messages.value.push({
      role: 'assistant',
      content: `✨ **需求文档已优化**\n\n${result.summary || 'AI 已根据反馈优化需求文档，请查看详情。'}`,
      timestamp: Date.now()
    })

    addFlowResult({
      type: 'optimize',
      icon: '✨',
      title: '文档优化（AI 生成）',
      content: `${issue.label} 的需求文档已通过 AI 引擎优化完成。`,
      expandable: true,
      expanded: true,
      detail: optimizedDoc
    })

    ElMessage.success('✅ 需求文档优化完成！')
    await scroll()
  } catch (e) {
    console.error('[optimizeRequirementDoc]', e)
    ElMessage.error('文档优化失败：' + e.message)
  } finally {
    generatingDoc.value = false
    persist()
  }
}

// ===== 生成业务流程图 =====
async function generateFlowDiagram() {
  generatingDiagram.value = true
  const issue = currentIssue.value || selectedIssue.value
  if (!issue) return
  try {
    const userMsgs = messages.value.filter(m => m.role === 'user' && !m.system)
    const context = userMsgs.map(m => m.content).join('\n') || issue.desc

    ElMessage.info('🔄 AI 正在生成业务流程图，请稍候...')

    const result = await aiGenerateFlowDiagram({
      requirement: context,
      issue_type: issue.label,
      context: context
    })

    const diagramContent = result.diagram || result.flow_diagram || result.content || result.mermaid || '流程图生成完成，但未获取到流程图内容。'

    messages.value.push({
      role: 'assistant',
      content: `🔄 **业务流程图已生成**\n\n${result.summary || 'AI 已根据需求生成业务流程图，请查看详情。'}`,
      timestamp: Date.now()
    })

    addFlowResult({
      type: 'diagram',
      icon: '🔄',
      title: '业务流程图（AI 生成）',
      content: `${issue.label} 的业务流程图已通过 AI 引擎生成完成。`,
      expandable: true,
      expanded: true,
      detail: diagramContent
    })

    if (currentStage.value < 2) currentStage.value = 2
    ElMessage.success('✅ 业务流程图生成成功！')
    await scroll()
  } catch (e) {
    console.error('[generateFlowDiagram]', e)
    ElMessage.error('流程图生成失败：' + e.message)
  } finally {
    generatingDiagram.value = false
    persist()
  }
}

// ===== 开发测试修复 =====
async function doDevTestFix() {
  devTesting.value = true
  const issue = currentIssue.value || selectedIssue.value
  if (!issue) return
  try {
    const userMsgs = messages.value.filter(m => m.role === 'user' && !m.system)
    const context = userMsgs.map(m => m.content).join('\n') || issue.desc

    ElMessage.info('💻 AI 正在执行开发测试修复，请稍候...')

    const result = await aiDevTestFix({
      requirement: context,
      issue_type: issue.label,
      context: context
    })

    const devReport = result.report || result.content || result.summary || '开发测试修复完成，但未获取到详细报告。'

    messages.value.push({
      role: 'assistant',
      content: `💻 **开发测试修复完成**\n\n${result.summary || 'AI 已完成开发、测试和修复工作，请查看详情。'}`,
      timestamp: Date.now()
    })

    addFlowResult({
      type: 'dev',
      icon: '💻',
      title: '开发测试修复报告（AI 生成）',
      content: `${issue.label} 的开发测试修复工作已通过 AI 引擎完成。`,
      expandable: true,
      expanded: true,
      detail: devReport
    })

    if (currentStage.value < 5) currentStage.value = 5
    ElMessage.success('✅ 开发测试修复完成！')
    await scroll()
  } catch (e) {
    console.error('[doDevTestFix]', e)
    ElMessage.error('开发测试失败：' + e.message)
  } finally {
    devTesting.value = false
    persist()
  }
}

// ===== 全维完成 =====
async function fullComplete() {
  const issue = currentIssue.value || selectedIssue.value
  if (!issue) return
  try {
    ElMessage.info('🚀 AI 正在执行一键全维完成，请稍候...')

    const userMsgs = messages.value.filter(m => m.role === 'user' && !m.system)
    const context = userMsgs.map(m => m.content).join('\n') || issue.desc

    const result = await aiFullComplete({
      requirement: context,
      issue_type: issue.label,
      context: context
    })

    const completeDetail = result.summary || result.report || result.content || '全维完成执行成功。'

    messages.value.push({
      role: 'assistant',
      content: `🚀 **全维完成（AI 生成）**\n\n${completeDetail}`,
      timestamp: Date.now()
    })

    addFlowResult({
      type: 'complete',
      icon: '🚀',
      title: '全维完成（AI 生成）',
      content: `${issue.label} 的全维流程已通过 AI 引擎一键完成。`,
      expandable: true,
      expanded: true,
      detail: completeDetail
    })

    if (currentStage.value < FLOW_STAGES.length - 1) {
      currentStage.value = FLOW_STAGES.length - 1
    }
    ElMessage.success('🎉 AI 全维完成！')
    await scroll()
  } catch (e) {
    console.error('[fullComplete]', e)
    ElMessage.error('全维完成失败：' + e.message)
  } finally {
    persist()
  }
}

// ===== 流程命令处理 =====
function handleFlowCommand(command) {
  const handlers = {
    full_analysis: fullAnalysis,
    gen_requirement_doc: generateRequirementDoc,
    optimize_doc: optimizeRequirementDoc,
    gen_flow_diagram: generateFlowDiagram,
    gen_graph: generateRequirementGraph,
    dev_test_fix: doDevTestFix,
    full_complete: fullComplete
  }
  const handler = handlers[command]
  if (handler) handler()
}

// ===== 成果管理 =====
function addFlowResult(result) {
  flowResults.value.push({
    ...result,
    id: Date.now() + Math.random()
  })
  // 自动展开第一个
  if (flowResults.value.length === 1) {
    flowResults.value[0].expanded = true
  }
}

function toggleResultExpand(index) {
  flowResults.value[index].expanded = !flowResults.value[index].expanded
}

// 工具抽屉
const activeTool = ref('')
const drawerVisible = ref(false)

function openTool(toolKey) {
  if (activeTool.value === toolKey && drawerVisible.value) {
    drawerVisible.value = false
    activeTool.value = ''
  } else {
    activeTool.value = toolKey
    drawerVisible.value = true
  }
}

function closeTool() {
  drawerVisible.value = false
  setTimeout(() => { activeTool.value = '' }, 300)
}

function onAutoTaskToggle(val) {
  if (val) {
    ElMessage.success('🚀 任务模式已开启：AI将自动分析对话并创建执行任务')
  }
  persist()
}

async function convertToTask() {
  if (!messages.value.length) {
    ElMessage.warning('请先进行对话，再转换为任务')
    return
  }
  convertDialogOpen.value = true
  convertResult.value = null
  convertingTask.value = true
  try {
    const r = await convertChatToTask({
      session_id: currentSession.value,
      messages: messages.value.map(m => ({ role: m.role, content: m.content })).filter(m => m.content),
      text: messages.value.filter(m => m.role === 'user').map(m => m.content).join('\n')
    })
    convertResult.value = r
    ElMessage.success('已成功转换为任务')
  } catch (e) {
    ElMessage.error('转换失败: ' + e.message)
    convertDialogOpen.value = false
  } finally {
    convertingTask.value = false
  }
}

function goToTasks() {
  convertDialogOpen.value = false
  router.push('/tasks')
}

function goToTaskDetail(taskId) {
  router.push({ path: '/tasks', query: { task: taskId } })
}

function priorityTagType(p) {
  return { high: 'danger', medium: 'warning', low: 'info' }[p] || 'info'
}

async function openBackendHistory() {
  historyOpen.value = true
  if (backendSessions.value.length) return
  loadingHistory.value = true
  try {
    const r = await listDialogueSessions()
    backendSessions.value = r.sessions || []
  } catch (e) {
    ElMessage.error('后端会话加载失败：' + e.message)
  } finally {
    loadingHistory.value = false
  }
}
async function restoreFromBackend(s) {
  if (loadingHistory.value) return
  loadingHistory.value = true
  try {
    const msgs = await getChatHistory(s.id)
    const list = Array.isArray(msgs) ? msgs : []
    if (!list.length) {
      ElMessage.info('该会话暂无后端聊天记录（仅自动入图会话会持久化）')
      return
    }
    // 后端 ChatMessage 转为前端 MessageBubble 格式
    setMessages(list.map((m) => ({
      role: String(m.role || '').toLowerCase() === 'user' ? 'user' : 'assistant',
      content: m.content || '',
      timestamp: m.timestamp || Date.now(),
      referenced_operators: m.referenced_operators || [],
      confidence: m.metadata && m.metadata.confidence != null
        ? m.metadata.confidence
        : undefined,
    })))
    // 同步到本地会话列表，保证可切换
    if (!sessions.value.find((x) => x.id === s.id)) {
      sessions.value.unshift({
        id: s.id,
        title: s.title || s.id,
        time: (s.updated_at || '').slice(0, 16).replace('T', ' '),
      })
    }
    currentSession.value = s.id
    persist()
    historyOpen.value = false
    ElMessage.success(`已恢复 ${messages.value.length} 条历史消息`)
    await scroll()
  } catch (e) {
    ElMessage.error('恢复失败：' + e.message)
  } finally {
    loadingHistory.value = false
  }
}

const quickQuestions = [
  '帮我推荐一个归一化算子',
  '解释一下知识图谱的中心性',
  '如何编排一个工作流链？',
  '算法复杂度怎么分析？'
]

// FR11 · 快捷问法（3×2 φ 卡片）：架构开发专家联盟 6 大高频场景
const quickQuestionsFull = [
  { icon: '📊', title: '算子推荐', desc: '帮我推荐一个归一化算子', prompt: '帮我推荐一个归一化算子，并比较几种方法的适用场景。' },
  { icon: '🔗', title: '图谱中心性', desc: '解释一下知识图谱的中心性', prompt: '请系统解释知识图谱的度中心性、介数中心性、紧密中心性三种指标的计算方法与差异。' },
  { icon: '🧵', title: '工作流编排', desc: '如何编排一个工作流链？', prompt: '如何编排一个包含数据预处理→算子执行→后处理的可靠工作流链？请给出架构与故障恢复方案。' },
  { icon: '⏱️', title: '复杂度分析', desc: '算法复杂度怎么分析？', prompt: '请用大 O 记号系统解析排序/图遍历/矩阵运算三类算法的复杂度，并给出企业级算法选型决策树。' },
  { icon: '🏗️', title: '全维架构设计', desc: '给我的系统做一次架构评估', prompt: '请对一个典型的分布式图谱+AI混合系统做全维架构评估：分层部署、数据流、SLA 指标、容灾与可观测性。' },
  { icon: '🛰️', title: '图谱报告生成', desc: '生成一份完整分析报告', prompt: '基于「知识图谱关联关系系统」，请生成一份覆盖节点、边、中心性、社区检测、可视化建议的全维度分析报告（含 Mermaid 图）。' },
]

// FR11 · 全维分析 5 阶段指示器（与 FLOW_STAGES 6 阶段独立并行，用于顶栏概览跳转）
const ANALYSIS_STAGES = [
  { key: 'req',    label: '需求' },
  { key: 'arch',   label: '架构' },
  { key: 'impl',   label: '实现' },
  { key: 'test',   label: '测试' },
  { key: 'accept', label: '验收' },
]

const STORE_KEY = 'ous_sessions'
const STATE_KEY = 'ous_chat_state'

function loadStore() {
  try {
    const raw = localStorage.getItem(STORE_KEY)
    if (raw) {
      const data = JSON.parse(raw)
      sessions.value = data.sessions || []
      const cur = data.current
      if (cur && sessions.value.find((s) => s.id === cur)) {
        currentSession.value = cur
        messagesMap.value = data.messages || {}
        messages.value = messagesMap.value[cur] || []
      }
    }
    // 恢复需求流程模式状态
    const stateRaw = localStorage.getItem(STATE_KEY)
    if (stateRaw) {
      const state = JSON.parse(stateRaw)
      if (state.requirementFlowMode !== undefined) requirementFlowMode.value = state.requirementFlowMode
      if (state.autoTaskMode !== undefined) autoTaskMode.value = state.autoTaskMode
      if (state.currentStage !== undefined) currentStage.value = state.currentStage
      if (state.selectedIssue) selectedIssue.value = state.selectedIssue
    }
  } catch (e) { /* ignore */ }
}
// 同时写入当前会话消息与映射，保证两者引用一致
function setMessages(arr) {
  messages.value = arr
  messagesMap.value = { ...messagesMap.value, [currentSession.value]: arr }
}
function persist() {
  try {
    const msgs = {}
    for (const s of sessions.value) msgs[s.id] = messagesMap.value[s.id] || []
    localStorage.setItem(
      STORE_KEY,
      JSON.stringify({
        sessions: sessions.value,
        current: currentSession.value,
        messages: msgs
      })
    )
    // 保存需求流程模式状态
    localStorage.setItem(
      STATE_KEY,
      JSON.stringify({
        requirementFlowMode: requirementFlowMode.value,
        autoTaskMode: autoTaskMode.value,
        currentStage: currentStage.value,
        selectedIssue: selectedIssue.value,
        currentIssue: currentIssue.value
      })
    )
  } catch (e) { /* ignore */ }
}

function newSession() {
  const id = 's-' + Math.random().toString(36).slice(2, 9)
  const s = { id, title: '新会话', time: new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }) }
  sessions.value.unshift(s)
  currentSession.value = id
  setMessages([])
  persist()
}
function selectSession(id) {
  currentSession.value = id
  setMessages(messagesMap.value[id] || [])
  persist()
}
async function sendQuick(q) {
  draft.value = q
  await send()
}

async function forkShareSession() {
  if (!currentSession.value || !messages.value.length) {
    newSession(); return
  }
  const cur = messages.value.slice()
  const oldId = currentSession.value
  const oldS = sessions.value.find(x => x.id === oldId)
  newSession()
  const newId = currentSession.value
  const cloned = cur.map((m, i) => ({
    ...m,
    id: 'fork-' + (m.id || ('m' + i)) + '-' + Date.now().toString(36).slice(-4),
    share_snapshot: false,
    timestamp: Number(m.timestamp) || (Date.now() + i),
    raw: undefined,
  }))
  messagesMap.value[newId] = cloned
  setMessages(cloned)
  const ns = sessions.value.find(x => x.id === newId)
  if (ns) ns.title = (oldS?.title ? oldS.title + ' · 副本' : '分享对话 · 副本')
  persist()
  nextTick(scroll)
  ElMessage.success({ message: '已创建可编辑副本，可继续发送对话', duration: 1800 })
}
async function send() {
  if (isShareMode.value) {
    ElMessage.warning('分享快照为只读模式，如需续问请点击「创建可编辑副本」')
    return
  }
  const text = draft.value.trim()
  if (!text || thinking.value) return
  if (!currentSession.value) newSession()
  const userMsg = { role: 'user', content: text, timestamp: Date.now() }
  messages.value.push(userMsg)
  const s = sessions.value.find((x) => x.id === currentSession.value)
  if (s && s.title === '新会话') s.title = text.slice(0, 14)
  draft.value = ''
  thinking.value = true
  await scroll()

  try {
    const expertType = selectedExpert?.value?.type
    const artifactPayload = artifactMode.value !== 'off' ? { artifact_mode: artifactMode.value } : {}
    let res
    if (expertType) {
      res = await aiExpertChat({
        messages: messages.value
          .filter(m => m.content)
          .map(m => ({ role: m.role, content: m.content })),
        expert_type: expertType,
        session_id: currentSession.value,
        web_search: webSearchEnabled.value,
        ...artifactPayload
      })
    } else {
      res = await aiChat({ session_id: currentSession.value, message: text, web_search: webSearchEnabled.value, ...artifactPayload })
    }

    if (!res || (!res.reply && !res.response && !res.message)) {
      throw new Error('服务器无响应')
    }

    const fullText = (res.reply || res.response || res.message || '（无回复）').toString()
    online.value = true

    const wsMeta = res.metadata?.web_search
    const artMeta = res.metadata?.artifacts || null

    messages.value.push({
      role: 'assistant',
      content: fullText,
      timestamp: Date.now(),
      referenced_operators: res.metadata?.related_operators || [],
      confidence: res.metadata?.confidence ?? null,
      web_search: wsMeta || null,
      artifacts: artMeta
    })

    // 需求流程模式：AI回复后检查是否需要推进阶段
    if (requirementFlowMode.value && currentIssue.value && currentStage.value < FLOW_STAGES.length - 1) {
      // 简单的阶段推进逻辑：根据对话轮数自动推进
      const userTurns = messages.value.filter(m => m.role === 'user' && !m.system).length
      const stageThresholds = [1, 3, 5, 8, 12] // 每个阶段需要的轮数
      const nextThreshold = stageThresholds[currentStage.value]
      if (userTurns >= nextThreshold) {
        advanceStage()
      }
    }

    // 任务模式：AI自动创建并执行任务
    if (autoTaskMode.value) {
      messages.value.push({
        role: 'system',
        content: '🤖 正在分析消息是否为任务...',
        timestamp: Date.now(),
        system: true
      })
      await scroll()
      try {
        const autoResult = await autoCreateTask({
          session_id: currentSession.value,
          message: text,
          messages: messages.value.map(m => ({ role: m.role, content: m.content })).filter(m => m.content && !m.system)
        })
        if (autoResult.is_task && autoResult.task) {
          const execInfo = autoResult.execution
          let taskMsg = `✅ 已创建任务「${autoResult.task.title}」`
          if (execInfo) {
            taskMsg += `\n📊 执行状态: ${execInfo.status}`
            if (execInfo.result) taskMsg += `\n📋 执行结果: ${execInfo.result}`
          } else {
            taskMsg += `\n⏳ 状态: 待处理`
          }
          if (autoResult.task.steps?.length) {
            taskMsg += `\n📌 步骤: ${autoResult.task.steps.join(' → ')}`
          }
          messages.value.push({
            role: 'system',
            content: taskMsg,
            timestamp: Date.now(),
            system: true,
            task_id: autoResult.task.id,
            task_data: autoResult.task,
            execution: autoResult.execution
          })
          ElMessage.success(`🚀 任务已创建${execInfo ? '并执行' : ''}：${autoResult.task.title}`)
        } else {
          messages.value.push({
            role: 'system',
            content: 'ℹ️ 此消息不需要创建任务，继续对话即可。',
            timestamp: Date.now(),
            system: true
          })
        }
      } catch (autoErr) {
        messages.value.push({
          role: 'system',
          content: '⚠️ 自动任务分析失败：' + (autoErr.message || '未知错误'),
          timestamp: Date.now(),
          system: true
        })
      }
      persist()
      await scroll()
    }

    persist()
    await scroll()
  } catch (e) {
    messages.value.push({
      role: 'assistant',
      content: '⚠️ ' + (e.message || '请求失败'),
      timestamp: Date.now()
    })
    online.value = false
    ElMessage.error(e.message || '请求失败')
    persist()
    await scroll()
  } finally {
    thinking.value = false
  }
}

function clearChat() {
  setMessages([])
  persist()
}
async function scroll() {
  await nextTick()
  if (scrollEl.value) scrollEl.value.scrollTop = scrollEl.value.scrollHeight
}

// ==================== 分享链接：Base64 Token → 本地会话恢复 ====================
const SHARE_TOKEN_KEY = 'share_session_token_v1'
/** 把 snapshot.msgs 转成 MessageBubble 兼容的内部消息格式（role/content/timestamp/id/system） */
function restoreMessagesFromSnapshot(msgs) {
  if (!Array.isArray(msgs)) return []
  return msgs
    .filter(m => m && typeof m.content === 'string')
    .map((m, i) => ({
      id: 'share-' + (m.id || ('m' + i + '-' + Date.now().toString(36))),
      role: ['user', 'assistant', 'system'].includes(m.role) ? m.role : (m.system ? 'system' : 'assistant'),
      content: String(m.content || ''),
      timestamp: Number(m.timestamp) || (Date.now() + i),
      senderName: m.senderName || undefined,
      confidence: m.confidence ?? undefined,
      system: !!m.system,
      raw: m,
      share_snapshot: true, // 标记：只读快照，禁止发送续问（在输入框发送前判断）
    }))
}
/** 从 route.params.token / route.query.share / hash 三路解析分享快照 */
function resolveShareToken() {
  // 1) /share/:token 路由参数（hash router，url 形如 #/share/eyJ...）
  if (route.params?.token) return String(route.params.token)
  // 2) 兼容：/ai?share=TOKEN 或 /ai?share_token=TOKEN
  if (route.query?.share) return String(route.query.share)
  if (route.query?.share_token) return String(route.query.share_token)
  // 3) 兼容旧版本：hash 尾部 #share=TOKEN（search 里）
  const search = typeof location !== 'undefined' ? location.search : ''
  const m = search.match(/[?&]share(_token)?=([^&]+)/)
  if (m) return m[2]
  return null
}
/** 解码 token → snapshot（容错：null 表示无效） */
function decodeShareToken(token) {
  if (!token || typeof token !== 'string') return null
  const t = token.trim().replace(/^[\s"'`]+|[\s"'`]+$/g, '')
  if (!t) return null
  try {
    const json = decodeURIComponent(escape(atob(t)))
    const snap = JSON.parse(json)
    if (!snap || typeof snap !== 'object') return null
    if (!Array.isArray(snap.msgs)) return null
    if (!snap.v) snap.v = 1
    if (!snap.ts) snap.ts = Date.now()
    return snap
  } catch (e) {
    console.warn('[Share] token decode failed:', e?.message || e, 'token[:40]=', t.slice(0, 40))
    return null
  }
}
// 分享模式标记（只读：发送输入框禁用）
const isShareMode = computed(() => !!route.meta?.shareMode || !!resolveShareToken())
/** 把 snapshot 恢复为本地会话（加入左侧 sessions 列表，激活并显示） */
function restoreSharedSnapshot(tokenParam) {
  const token = tokenParam || resolveShareToken()
  if (!token) return { ok: false, reason: 'no-token' }
  const snap = decodeShareToken(token)
  if (!snap) {
    ElMessage.error('分享链接无效或已损坏，请重新生成')
    return { ok: false, reason: 'bad-token' }
  }
  const msgs = restoreMessagesFromSnapshot(snap.msgs)
  if (!msgs.length) {
    ElMessage.warning('分享内容为空，无法恢复对话')
    return { ok: false, reason: 'empty' }
  }
  // 幂等：如果该 snapshot 已经存在于 sessions（同一 token）则只切换不重复创建
  const sessionId = 'share-' + (token.slice(-8) || Math.random().toString(36).slice(2, 8))
  const title = (msgs[0].content || '').replace(/\s+/g, ' ').slice(0, 16) + '…分享'
  let s = sessions.value.find((x) => x.id === sessionId)
  if (!s) {
    s = {
      id: sessionId,
      title: title || '分享对话',
      time: new Date(snap.ts || Date.now()).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
      shareToken: token.slice(0, 20) + (token.length > 20 ? '…' : ''),
      readOnly: true,
    }
    sessions.value.unshift(s)
  }
  messagesMap.value[sessionId] = msgs
  currentSession.value = sessionId
  setMessages(msgs)
  persist()
  nextTick(scroll)
  ElMessage.success({
    message: `已载入分享对话（共 ${msgs.length} 条消息，只读模式）`,
    duration: 2200,
    showClose: true,
  })
  return { ok: true, session: s, count: msgs.length }
}

watch(messages, persist, { deep: true })

// 监听路由 token 变化（同一个 ChatView 组件，从 /ai → /share/xxx 不会重挂）
watch(
  () => [route.params?.token, route.query?.share, route.query?.share_token, route.path],
  () => {
    const token = resolveShareToken()
    if (!token) return
    // 避免 onMounted 已跑完但路由没触发；如果没有会话就是首次挂载，等 onMounted 处理
    if (!sessions.value.length) return
    restoreSharedSnapshot(token)
  },
  { flush: 'post' }
)

onMounted(async () => {
  loadStore()
  loadProjectList()
  // 支持 URL 参数：?flow=1 开启流程模式（支持 hash 路由）
  const hash = window.location.hash
  const hashParams = new URLSearchParams(hash.split('?')[1] || '')
  const searchParams = new URLSearchParams(window.location.search)
  if (hashParams.get('flow') === '1' || searchParams.get('flow') === '1') {
    await nextTick()
    requirementFlowMode.value = true
    persist()
  }

  // —— 分享快照优先：如 URL 带 token，优先恢复（不需要先有本地会话兜底）——
  const tok = resolveShareToken()
  if (tok) {
    const r = restoreSharedSnapshot(tok)
    if (r.ok) {
      // 快照恢复成功，跳过默认新会话
    } else if (!sessions.value.length) {
      newSession()
    }
  } else if (!sessions.value.length) {
    newSession()
  }

  // 拉取后端全自动同步开关状态 + 探测后端连接状态
  getAutoSyncStatus()
    .then((r) => {
      if (r) autoSync.value = !!(r.enabled ?? r.auto_sync ?? r.data?.auto_sync)
      online.value = true
    })
    .catch(() => {})
})

// 切换对话自动入图开关（全自动）
async function onToggleAutoSync(val) {
  try {
    await toggleAutoSync(val)
    ElMessage.success(val ? '已开启：对话自动整理进知识图谱' : '已关闭：手动模式')
  } catch {
    ElMessage.error('切换同步开关失败')
  }
}

// 导出对话 + 知识图谱 为单文件迁移包
async function exportBundle() {
  try {
    const res = await graphExport()
    const blob = new Blob([JSON.stringify(res, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `operator-bundle-${new Date().toISOString().slice(0, 10)}.json`
    a.click()
    URL.revokeObjectURL(url)
    ElMessage.success('迁移包已导出（对话 + 知识图谱）')
  } catch {
    ElMessage.error('导出失败')
  }
}

// 选择导入文件
function pickImport() {
  importInput.value?.click()
}

// 导入迁移包（对话 + 图谱）
async function onImportFile(e) {
  const file = e.target.files?.[0]
  if (!file) return
  try {
    const text = await file.text()
    const bundle = JSON.parse(text)
    const res = await graphImport(bundle)
    ElMessage.success(`导入完成：会话 ${res.imported.sessions} / 节点 ${res.imported.nodes}`)
  } catch {
    ElMessage.error('导入失败：文件格式不合法')
  } finally {
    e.target.value = ''
  }
}

// ========== MessageBubble 新事件占位（8 emits · 父级处理） ==========
function onRate(m, rating) {
  const txt = rating === 'like' ? '👍 已点赞 感谢反馈' : rating === 'dislike' ? '👎 已点踩 我们会改进' : '已取消评分'
  ElMessage.success({ message: txt, duration: 1500 })
  // TODO: 后端埋点 POST /ai/chat/rating
}
function onShare(m) {
  ElMessage.success({ message: '📤 分享已发送（埋点已记录）', duration: 1500 })
  // TODO: 后端埋点 POST /ai/chat/share
}
async function onToDoc(m, payload) {
  if (!payload || !payload.mode) return
  if (payload.mode === 'create-kb') {
    ElMessage.success('📁 已提交到云盘（占位），后续将自动创建知识库文档')
    return
  }
  if (payload.mode === 'export-md') {
    // 兜底剪贴板双写（MessageBubble 内已写过一次；此处仅 toast 确认）
    ElMessage.success({ message: '✅ Markdown 已写入剪贴板（可直接粘贴到任意编辑器）', duration: 1800 })
    return
  }
}
function onFavorite(m, f) {
  ElMessage({ message: f ? '⭐ 已加入收藏夹' : '已从收藏夹移除', type: 'success', duration: 1200, showClose: false })
  // TODO: localStorage 集合已由 MessageBubble 维护；此处可同步后端
}
function onFeedback(m, payload) {
  ElMessage.success({ message: '📮 反馈已提交，感谢助力专家联盟质量升级', duration: 2000 })
  // TODO: 后端 POST /ai/chat/feedback
}
function onFollowup(m, prompt) {
  // 回填前缀到 draft，空格隔开已有内容，自动聚焦 textarea
  const cur = draft.value?.trim()
  draft.value = cur ? (prompt + ' ' + cur) : prompt
  persist()
  // 聚焦输入框（无 ref 用 DOM 选择器兜底）
  nextTick(() => {
    try {
      const ta = document.querySelector('.chat-input textarea')
      if (ta) { ta.focus(); const len = ta.value?.length || 0; try { ta.setSelectionRange(len, len) } catch(_){} }
    } catch(_) {}
  })
  ElMessage.info({ message: '💬 追问提示已写入输入框，可直接继续输入后发送', duration: 1800 })
}
async function onRegenerate(m) {
  // 真·重生成：查找该条助手消息 index，移除（及之后可能的后续助手消息占位），复用最近一条用户消息 content 重跑 send
  const idx = messages.value.findIndex(x => x === m)
  if (idx <= 0) { ElMessage.warning('重生成失败：未找到对应上下文'); return }
  // 往前找最近一条用户消息（作为重发 prompt）
  let userIdx = -1
  for (let i = idx - 1; i >= 0; i--) {
    if (messages.value[i]?.role === 'user' && !messages.value[i].system) { userIdx = i; break }
  }
  if (userIdx < 0) { ElMessage.warning('重生成失败：没有可复读的用户提问'); return }
  const userContent = messages.value[userIdx].content
  // 从 idx 起清空之后的助手/系统占位消息（保留用户消息及之前）
  const kept = messages.value.slice(0, idx)
  setMessages(kept)
  draft.value = userContent
  ElMessage.success({ message: '♻️ 重新生成中…', duration: 1500 })
  persist()
  await send()
}
// ========== 顶栏：全维分析 CTA 触发 ==========
function triggerFullAnalysis() {
  if (!requirementFlowMode.value) {
    requirementFlowMode.value = true
    currentStage.value = 0
    ElMessage.success({ message: '🎯 已开启全维分析流程模式', duration: 1500 })
  }
  // 无选中问题 → 自动选一个（需求图谱 primary）
  if (!currentIssue.value) {
    const primary = ISSUE_CATEGORIES.find(c => c.primary) || ISSUE_CATEGORIES[0]
    selectedIssue.value = primary
    currentIssue.value = primary
    ElMessage.info({ message: `已选择问题域：${primary.label}`, duration: 1600 })
  }
  // 跳分析：如果没有任何用户消息 → 填默认 prompt 到 draft；否则直接跑 fullAnalysis
  const hasUserMsg = messages.value.some(m => m.role === 'user' && !m.system)
  if (!hasUserMsg) {
    draft.value = `【${currentIssue.value.label}】请进行全维分析，覆盖：\n① 需求梳理与结构化\n② 架构方案（服务拆分、数据流、SLA）\n③ 实现要点（技术栈、关键算法）\n④ 测试策略（单测/集成/E2E）\n⑤ 验收标准与发布闸门`
    ElMessage.success({ message: '✅ 全维分析框架已生成，点击「发送」即可执行', duration: 2500 })
  } else {
    fullAnalysis()
  }
  persist()
}
// ========== 分析阶段 Chip 点击跳转 ==========
function jumpAnalysisStage(idx) {
  if (idx == null || idx < 0 || idx >= ANALYSIS_STAGES.length) return
  // 开启流程模式
  if (!requirementFlowMode.value) {
    requirementFlowMode.value = true
    ElMessage.success({ message: '已进入流程模式：' + ANALYSIS_STAGES[idx].label, duration: 1400 })
  }
  // 映射到 FLOW_STAGES（6 段）：0→0 需求, 1→1 架构, 2→2~3 实现, 3→4 测试, 4→5 验收
  const MAP = [0, 1, 2, 4, 5]
  const target = MAP[idx] || 0
  currentStage.value = Math.max(0, Math.min(FLOW_STAGES.length - 1, target))
  // 推送一条系统提示
  const stage = FLOW_STAGES[currentStage.value]
  const ana = ANALYSIS_STAGES[idx]
  messages.value.push({
    role: 'system',
    content: `📌 跳转到【${ana.label}】分析阶段\n当前阶段：${stage.label}\n${stage.hint}`,
    timestamp: Date.now(),
    system: true,
  })
  persist()
  scroll()
}

// ========== T11 触发：全维分析（专家联盟 SSE） ==========
async function triggerAlliance(fromCurrentInput = true) {
  if (allianceRunning.value) return
  let query = ''
  if (fromCurrentInput && (draft?.value ?? '').trim()) {
    query = draft.value.trim()
  } else {
    const last = [...messages.value].reverse().find(m => m.role === 'user')
    query = last?.content?.toString().trim() ?? ''
  }
  if (!query) {
    ElMessage.warning('请先输入问题或发送一条消息后再做全维分析')
    return
  }
  allianceRunning.value = true
  alliancePhase.value = null
  allianceTraceId.value = ''
  try {
    await runAllianceFullSSE(
      {
        query,
        session_id: currentSession.value || null,
        team_size: 4,
        retry_on_c: true,
        enable_llm_debate: false,
      },
      (frame) => {
        alliancePhase.value = frame.phase
        allianceTraceId.value = frame.trace_id
        if (frame.phase === 'synthesize' && frame.payload?.markdown) {
          const md = String(frame.payload.markdown)
          messages.value.push({
            id: 'alliance-syn-' + (Math.random().toString(36).slice(2)),
            role: 'assistant',
            content: md,
            createdAt: new Date(),
            sessionId: currentSession.value,
          })
        }
        if (frame.phase === 'gate' && frame.payload?.score) {
          const g = frame.payload.score
          ElMessage({
            type: g.grade === 'D' ? 'error' : g.grade === 'A' ? 'success' : 'info',
            message: `质量门禁 ${g.grade} 级：综合分 ${(g.total * 100).toFixed(1)} / 100（公式 = ${g.formula ?? 'HC-8'}）`,
            duration: g.grade === 'D' ? 0 : 4200,
            showClose: true,
          })
        }
        if (frame.phase === 'done') {
          ElMessage.success(`全维分析完成（trace ${String(frame.trace_id || '').slice(0, 8)}…）`)
        }
      }
    )
  } catch (e) {
    ElMessage.warning(`专家联盟不可用，已降级为普通对话：${e?.message ?? e}`)
  } finally {
    allianceRunning.value = false
  }
}

// ========== T12: 麦克风录音 ==========
async function toggleMicRecording() {
  if (!isRecording.value) {
    try {
      audioCtx = audioCtx || new (window.AudioContext || window.webkitAudioContext)()
      micStream = await navigator.mediaDevices.getUserMedia({ audio: true })
      const src = audioCtx.createMediaStreamSource(micStream)
      micAnalyser = audioCtx.createAnalyser()
      micAnalyser.fftSize = 256
      src.connect(micAnalyser)
      const arr = new Uint8Array(micAnalyser.frequencyBinCount)
      const loop = () => {
        if (!micAnalyser || !isRecording.value) return
        micAnalyser.getByteTimeDomainData(arr)
        let sum = 0
        for (let i = 0; i < arr.length; i++) {
          const v = (arr[i] - 128) / 128
          sum += v * v
        }
        const rms = Math.sqrt(sum / arr.length)
        micLevel.value = Math.max(0, Math.min(1, rms * 3.2))
        micRafId = requestAnimationFrame(loop)
      }
      isRecording.value = true
      loop()
      voiceUiOpen.value = true
    } catch (e) {
      ElMessage.error('麦克风授权失败：' + (e?.message ?? e))
    }
  } else {
    isRecording.value = false
    cancelAnimationFrame(micRafId)
    micStream?.getTracks().forEach(t => t.stop())
    micStream = null
    micLevel.value = 0
    ElMessage.info('录音停止（ASR 提交需 xiaobai_voice 服务；当前 UI 就绪）')
  }
}

/** 刷新语音 health（T12） */
async function refreshVoiceHealth() {
  try { voiceHealth.value = await getVoiceHealth() } catch (e) { voiceHealth.value = { ok: false } }
}

onUnmounted(() => {
  if (streamTimer) clearInterval(streamTimer)
  try { cancelAnimationFrame(micRafId) } catch(_) {}
  try { micStream?.getTracks().forEach(t => t.stop()) } catch(_) {}
})

// ===== 项目对话：推荐智能体与项目状态 =====
const recommendedExpert = computed(() => {
  const stage = DEV_STAGES[devStage.value]
  const expert = AI_EXPERT_PRESETS.find(e => e.key === stage?.expert)
  return expert ? { ...expert, hint: stage?.hint } : null
})
// 跟进项目模式：是否有进行中的项目（用于显示"进行中"标签）
const isFollowupActive = computed(() => {
  const s = String(currentProject.value?.status || '').toLowerCase()
  return ['active', '进行中', 'in_progress'].includes(s)
})
const projectStatusType = computed(() => {
  const s = String(currentProject.value?.status || '').toLowerCase()
  if (['active', '进行中', 'in_progress'].includes(s)) return 'success'
  if (['done', 'completed', '已完成'].includes(s)) return 'info'
  if (['paused', 'blocked'].includes(s)) return 'warning'
  return 'info'
})
const projectStatusLabel = computed(() => {
  const s = String(currentProject.value?.status || '').toLowerCase()
  return { active: '进行中', in_progress: '进行中', done: '已完成', completed: '已完成',
    paused: '已暂停', blocked: '阻塞', planning: '规划中', pending: '待启动' }[s] || (currentProject.value?.status || '规划中')
})
function onChatModeChange(val) {
  if (val === 'followup' && !currentProject.value) {
    ElMessage.info('请先选择项目，进入跟进项目模式')
  }
  // 切换到跟进项目时，自动选择推荐专家
  if (val === 'followup' && recommendedExpert.value) {
    selectedExpert.value = recommendedExpert.value
  }
}
function switchChatMode(mode) {
  if (mode === 'create') {
    // 创建项目：直接打开创建项目对话框
    openCreateProject()
    return
  }
  chatMode.value = mode
  onChatModeChange(mode)
}
function goProjects() {
  router.push('/projects')
}
// ===== 项目 CRUD（对话内创建/编辑） =====
function openCreateProject() {
  projectDlg.value = {
    visible: true, isEdit: false, submitting: false,
    form: { name: '', category: 'platform', status: 'active', description: '', code_path: '', tech_stack: '', owner: 'ai-alliance', useChatContext: true, autoPipeline: true }
  }
}
function openEditProject() {
  if (!currentProject.value) return
  projectDlg.value = {
    visible: true, isEdit: true, submitting: false,
    form: {
      name: currentProject.value.name || '',
      category: currentProject.value.category || 'platform',
      status: currentProject.value.status || 'active',
      description: currentProject.value.description || '',
      code_path: currentProject.value.code_path || '',
      tech_stack: currentProject.value.tech_stack || '',
      owner: currentProject.value.owner || 'ai-alliance',
      useChatContext: true,
      autoPipeline: true
    }
  }
}
async function saveProjectFromChat() {
  const f = projectDlg.value.form
  if (!f.name.trim()) return ElMessage.warning('请输入项目名称')
  projectDlg.value.submitting = true
  try {
    if (projectDlg.value.isEdit) {
      await updateProject(currentProject.value.id, f)
      ElMessage.success('项目已更新')
      // 刷新当前项目
      const updated = await getProject(currentProject.value.id)
      if (updated) Object.assign(currentProject.value, updated)
    } else {
      const created = await createProject(f)
      ElMessage.success('项目已创建，自动进入跟进项目')
      // 切换到跟进项目模式
      chatMode.value = 'followup'
    }
    projectDlg.value.visible = false
  } finally {
    projectDlg.value.submitting = false
  }
}
// ===== 项目资源面板 =====
function openResourcePanel() {
  resourcePanel.value.visible = true
  resourcePanel.value.activeTab = 'overview'
}
// ===== AI全维自动开发 =====
async function startFullDev() {
  if (!currentProject.value) {
    ElMessage.warning('请先选择或创建项目')
    return
  }
  if (fullDevRunning.value) return
  fullDevRunning.value = true
  fullDevStage.value = 0
  try {
    // 阶段1：需求分析
    draft.value = `请对项目「${currentProject.value.name}」进行全维需求分析，输出：1.业务目标 2.功能需求清单 3.非功能需求 4.用户角色 5.核心业务流程。项目描述：${currentProject.value.description || '暂无'}`
    await send()
    fullDevStage.value = 1
    // 阶段2：架构设计
    draft.value = '基于以上需求，进行系统架构设计：1.整体架构图 2.技术选型 3.模块划分 4.数据库设计 5.API接口设计'
    await send()
    fullDevStage.value = 2
    // 阶段3：代码生成
    draft.value = '基于架构设计，生成核心代码：1.项目结构 2.核心模块代码 3.配置文件 4.数据库脚本 5.部署脚本'
    await send()
    fullDevStage.value = 3
    // 阶段4：测试验证
    draft.value = '生成测试方案并执行：1.单元测试用例 2.集成测试方案 3.性能测试指标 4.安全检查清单 5.测试报告'
    await send()
    fullDevStage.value = 4
    ElMessage.success('AI全维开发完成！项目已从需求到部署全链路生成')
  } catch (e) {
    ElMessage.error('全维开发中断：' + (e.message || '未知错误'))
  } finally {
    fullDevRunning.value = false
  }
}

// ===== 璇玑：以项目为核心的联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  onMounted(async () => {
    _offPj = _onProjectChange(async () => {})
    await _ensureProject().catch(() => {})
  })
  onBeforeUnmount(() => { _offPj && _offPj() })
}
</script>

<style scoped>
/* ===== 外层布局容器 · 黄金比例深空设计 ===== */
.chat {
  display: flex;
  height: calc(100vh - var(--header-h) - 86px);
  background: var(--bg-surface);
  border-radius: var(--radius-xl);
  border: 1px solid var(--border-ghost);
  box-shadow: var(--shadow-md), var(--shadow-inset);
  overflow: hidden;
  flex-direction: row;
}
.chat-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.chat-header {
  height: 86px;
  min-height: 86px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 26px;
  border-bottom: 1px solid var(--border-ghost);
  background:
    linear-gradient(180deg, var(--bg-surface) 0%, var(--bg-surface-2) 100%),
    radial-gradient(800px 200px at 100% 0%, rgba(99,102,241,0.06), transparent 60%);
  gap: 16px;
}
.chat-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-weight: 700;
  font-size: 15px;
  color: var(--text-primary);
}
/* 对话模式切换 */
.mode-switch {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-left: 8px;
  background: var(--bg-page);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 2px;
}
.mode-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 5px 12px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s;
}
.mode-btn:hover {
  color: var(--brand);
  background: var(--brand-50);
}
.mode-btn.active {
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  font-weight: 600;
  box-shadow: 0 2px 8px rgba(99,102,241,0.3);
}
.mode-badge {
  margin-left: 4px;
  padding: 1px 6px;
  font-size: 10px;
  font-weight: 600;
  background: #10b981;
  color: #fff;
  border-radius: 999px;
  line-height: 1.4;
}
.mode-btn.active .mode-badge {
  background: rgba(255,255,255,0.25);
  color: #fff;
}

/* 项目对话上下文 bar */
.project-context-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 16px;
  margin: 8px 0;
  background: linear-gradient(135deg, rgba(99,102,241,0.06), rgba(14,165,233,0.04));
  border: 1px solid rgba(99,102,241,0.15);
  border-radius: 12px;
}
.pcb-left {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
}
.pcb-project {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
}
.pcb-icon {
  font-size: 22px;
  flex-shrink: 0;
}
.pcb-info {
  flex: 1;
  min-width: 0;
}
.pcb-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.pcb-desc {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pcb-right {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
  flex-shrink: 0;
}
.pcb-agent {
  display: flex;
  align-items: center;
  gap: 6px;
}
.pcb-label {
  font-size: 11px;
  color: var(--text-secondary);
}
.agent-tag {
  font-weight: 600;
}
.pcb-hint {
  font-size: 11px;
  color: var(--text-dim);
}
.pcb-stages {
  display: flex;
  align-items: center;
  gap: 2px;
}
.pcb-stage {
  padding: 3px 10px;
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  background: var(--bg-page);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: all 0.15s;
}
.pcb-stage:first-child { border-radius: 6px 0 0 6px; }
.pcb-stage:last-child { border-radius: 0 6px 6px 0; }
.pcb-stage:hover {
  color: var(--brand);
  border-color: var(--brand);
}
.pcb-stage.active {
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  border-color: transparent;
  font-weight: 600;
}
.pcb-stage.done {
  background: #ecfdf5;
  color: #059669;
  border-color: #a7f3d0;
}
.pcb-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.pcb-actions .el-button {
  font-size: 12px !important;
  padding: 4px 8px !important;
}
.full-dev-btn {
  margin-top: 6px;
  font-weight: 600 !important;
  background: linear-gradient(135deg, #f59e0b, #ef4444) !important;
  border: none !important;
  color: #fff !important;
}
.full-dev-btn:hover {
  opacity: 0.9;
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(245,158,11,0.4);
}

/* 项目类型选择器 */
.cat-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.cat-option {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.15s;
  font-size: 13px;
}
.cat-option:hover {
  border-color: var(--brand);
}
.cat-option.active {
  font-weight: 600;
}
.cat-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* 项目资源面板 */
.res-panel-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 12px 16px;
  background: var(--bg-page);
  border-radius: 10px;
  margin-bottom: 16px;
}
.res-project-info { flex: 1; min-width: 0; }
.res-pname { font-size: 16px; font-weight: 700; color: var(--text-primary); }
.res-pdesc { font-size: 12px; color: var(--text-secondary); margin-top: 4px; }
.res-tabs { margin-top: 8px; }

/* 资源统计卡片 */
.res-overview { padding: 8px 0; }
.res-stat-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}
.res-stat-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 16px 12px;
  background: var(--bg-page);
  border: 1px solid var(--border);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
}
.res-stat-card:hover {
  border-color: var(--brand);
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(99,102,241,0.15);
}
.res-stat-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  margin-bottom: 8px;
}
.res-stat-icon.graph { background: #ede9fe; color: #7c3aed; }
.res-stat-icon.req { background: #dbeafe; color: #2563eb; }
.res-stat-icon.data { background: #d1fae5; color: #059669; }
.res-stat-icon.kb { background: #fef3c7; color: #d97706; }
.res-stat-icon.code { background: #fce7f3; color: #db2777; }
.res-stat-icon.task { background: #e0e7ff; color: #4f46e5; }
.res-stat-num { font-size: 24px; font-weight: 700; color: var(--text-primary); }
.res-stat-label { font-size: 12px; color: var(--text-secondary); margin-top: 2px; }

.res-meta {
  background: var(--bg-page);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 16px;
}
.res-meta-item {
  font-size: 13px;
  color: var(--text-secondary);
  padding: 4px 0;
}
.res-meta-label { color: var(--text-dim); font-weight: 500; }

/* 资源列表 */
.res-list { padding: 8px 0; }
.res-empty {
  text-align: center;
  padding: 40px 20px;
  color: var(--text-dim);
  font-size: 13px;
}
.res-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--bg-page);
  border: 1px solid var(--border);
  border-radius: 10px;
  margin-bottom: 8px;
  transition: all 0.15s;
}
.res-item:hover {
  border-color: var(--brand);
  background: var(--brand-50);
}
.res-item-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}
.res-item-icon.graph { background: #ede9fe; color: #7c3aed; }
.res-item-icon.req { background: #dbeafe; color: #2563eb; }
.res-item-icon.data { background: #d1fae5; color: #059669; }
.res-item-icon.kb { background: #fef3c7; color: #d97706; }
.res-item-icon.task { background: #e0e7ff; color: #4f46e5; }
.res-item-info { flex: 1; min-width: 0; }
.res-item-name { font-size: 14px; font-weight: 600; color: var(--text-primary); }
.res-item-meta { font-size: 11px; color: var(--text-dim); margin-top: 2px; }

/* 代码配置 */
.res-code { padding: 8px 0; }
.res-code-item { margin-bottom: 16px; }
.res-code-label { font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 8px; }
.res-code-actions { display: flex; gap: 10px; margin-top: 20px; }
.chat-tools {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.ct-divider {
  width: 1px;
  height: 22px;
  background: var(--border-ghost);
  margin: 0 4px;
  align-self: center;
  opacity: 0.85;
}
.ct-btn {
  font-size: 13px !important;
  padding: 6px 10px !important;
  gap: 4px;
  border-radius: 10px !important;
  transition: all .2s var(--ease) !important;
}
.ct-btn:hover {
  background: var(--brand-50) !important;
  border-color: transparent !important;
  transform: translateY(-1px);
}
.ct-btn-new { color: #6366f1 !important; font-weight: 600 !important; }
.ct-cta-full {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 18px 0 16px;
  border-radius: 19px;
  font-weight: 650;
  font-size: 13.5px;
  letter-spacing: 0.2px;
  color: #ffffff;
  background: linear-gradient(135deg, #6366f1 0%, #4f46e5 32%, #8b5cf6 68%, #a855f7 100%);
  border: none;
  box-shadow:
    0 10px 26px -10px rgba(99,102,241,0.65),
    0 2px 6px rgba(79,70,229,0.18),
    inset 0 1px 0 rgba(255,255,255,0.18);
  transition: all .25s var(--ease);
}
.ct-cta-full:hover {
  transform: translateY(-1px);
  box-shadow:
    0 16px 36px -12px rgba(99,102,241,0.8),
    0 4px 10px rgba(79,70,229,0.25),
    inset 0 1px 0 rgba(255,255,255,0.22);
}
.ct-cta-text { position: relative; top: -0.5px; }
.ct-cta-badge {
  display: inline-grid;
  place-items: center;
  width: 18px; height: 18px;
  border-radius: 50%;
  font-size: 10px;
  font-weight: 700;
  color: #6366f1;
  background: rgba(255,255,255,0.95);
  box-shadow: 0 1px 3px rgba(15,23,42,0.12);
  margin-left: 2px;
}
.expert-selector {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-right: 8px;
}
.expert-selector .muted {
  font-size: 12px;
  color: var(--text-tertiary);
}
.chat-body {
  flex: 1;
  overflow-y: auto;
  padding: var(--space-5) var(--space-6);
  background:
    radial-gradient(1200px 400px at 8% -10%, rgba(99, 102, 241, 0.07), transparent 60%),
    radial-gradient(1000px 500px at 100% 0%, rgba(6, 182, 212, 0.05), transparent 55%),
    var(--bg-deep-sky);
}
.chat-body::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}
.chat-body::-webkit-scrollbar-thumb {
  background: #c7cfdc;
  border-radius: 6px;
}
.chat-body::-webkit-scrollbar-thumb:hover {
  background: #a8b0bf;
}

/* ===== 全维分析 · 5 阶段 Chip 指示器 ===== */
.analysis-stages {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 26px;
  min-height: 42px;
  border-bottom: 1px solid var(--border-ghost);
  background: linear-gradient(180deg, rgba(99,102,241,0.03) 0%, transparent 100%);
  flex-wrap: wrap;
}
.stage-chip {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 5px 14px 5px 12px;
  border-radius: 999px;
  font-size: 12.5px;
  font-weight: 550;
  color: var(--text-tertiary);
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  cursor: pointer;
  transition: all .2s var(--ease);
  user-select: none;
}
.stage-chip:hover {
  border-color: var(--border-focus);
  color: var(--brand-700);
  transform: translateY(-1px);
  box-shadow: var(--shadow-ghost);
}
.stage-dot {
  width: 8px; height: 8px;
  border-radius: 50%;
  background: #cbd5e1;
  transition: background .2s, box-shadow .2s;
}
.stage-chip.active {
  color: #ffffff;
  border-color: transparent;
  box-shadow: 0 4px 14px -6px rgba(99,102,241,0.45), 0 1px 2px rgba(15,23,42,0.05);
}
.stage-chip.active .stage-dot {
  background: #ffffff;
  box-shadow: 0 0 0 3px rgba(255,255,255,0.2);
}
/* φ 色系递进 */
.stage-chip.stage-req.active    { background: linear-gradient(135deg,#3b82f6,#2563eb); }
.stage-chip.stage-arch.active   { background: linear-gradient(135deg,#6366f1,#8b5cf6); }
.stage-chip.stage-impl.active   { background: linear-gradient(135deg,#10b981,#0ea5a4); }
.stage-chip.stage-test.active   { background: linear-gradient(135deg,#f59e0b,#ef6c19); }
.stage-chip.stage-accept.active { background: linear-gradient(135deg,#ef4444,#dc2626); }
.stage-arrow {
  color: #cbd5e1;
  font-weight: 700;
  font-size: 14px;
  margin-left: 2px;
  line-height: 1;
}
.stage-chip.active + .stage-chip .stage-arrow {
  color: #c7d2fe;
}

/* chat-body 滚动内边距 · φ 呼吸 */
.chat-body > :first-child { margin-top: 0; }
.chat-body > :last-child  { margin-bottom: 20px; }

/* ===== 空状态 · 黄金留白 ===== */
.empty {
  text-align: center;
  color: var(--text-tertiary);
  margin-top: var(--space-7);
}
.empty-orb {
  width: 110px;
  height: 110px;
  margin: 0 auto var(--space-4);
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 38px;
  color: #fff;
  background: linear-gradient(135deg, #6366f1 0%, #0ea5e9 55%, #06b6d4 100%);
  box-shadow:
    0 14px 40px -10px rgba(99, 102, 241, 0.45),
    0 0 0 6px rgba(99, 102, 241, 0.08);
}
.empty-title {
  font-size: 16px !important;
  font-weight: 600 !important;
  color: var(--text-primary) !important;
  margin-bottom: 20px !important;
}
/* 3步快速上手引导 */
.quick-steps {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: 0 auto 24px;
  max-width: 680px;
  flex-wrap: wrap;
}
.qs-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  background: linear-gradient(135deg, rgba(99,102,241,0.06), rgba(14,165,233,0.04));
  border: 1px solid rgba(99,102,241,0.12);
  border-radius: 10px;
  flex: 1;
  min-width: 180px;
}
.qs-num {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  flex-shrink: 0;
}
.qs-text {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.4;
  text-align: left;
}
.qs-text span {
  font-size: 11px;
  font-weight: 400;
  color: var(--text-secondary);
}
.qs-arrow {
  color: #6366f1;
  font-size: 16px;
  font-weight: 700;
  flex-shrink: 0;
}
.empty p {
  max-width: 520px;
  margin: 0 auto var(--space-4);
  line-height: 1.75;
  font-size: 14px;
}
.empty p:first-of-type {
  color: var(--text-secondary);
}
.empty p:last-of-type {
  color: var(--text-tertiary);
}
.task-mode-hint {
  background: linear-gradient(135deg, #fef3c7, #fde68a, #fcd34d 120%);
  border: 1px solid rgba(245, 158, 11, 0.18);
  color: #92400e;
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-2xl);
  font-size: 13px;
  font-weight: 600;
  display: inline-block;
  margin: 0 auto var(--space-4);
}
.suggestions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
  justify-content: center;
}
.q {
  cursor: pointer;
  padding: 8px 14px;
  border-radius: 999px;
  background: var(--bg-surface);
  border: 1px solid var(--border-soft);
  font-size: 13px;
  color: var(--text-secondary);
  transition: all var(--dur-2) var(--ease);
}
.q:hover {
  background: var(--brand-50);
  border-color: rgba(99, 102, 241, 0.25);
  box-shadow: var(--shadow-ring);
  color: var(--brand-700);
}

/* ===== 快捷问法 · 3×2 Grid φ 卡片（FR11） ===== */
.suggestions-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  max-width: 680px;
  margin: 26px auto 0;
}
@media (max-width: 960px) {
  .suggestions-grid { grid-template-columns: repeat(2, 1fr); max-width: 560px; }
}
@media (max-width: 600px) {
  .suggestions-grid { grid-template-columns: 1fr; max-width: 360px; }
}
.qq-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 18px;
  height: 100%;
  min-height: 84px;
  border-radius: 14px;
  background: linear-gradient(135deg, var(--bg-surface) 0%, var(--bg-surface-2) 100%);
  border: 1px solid var(--border-ghost);
  box-shadow: 0 1px 2px rgba(15,23,42,0.04), 0 4px 14px -10px rgba(15,23,42,0.12);
  cursor: pointer;
  transition: all .25s var(--ease);
  position: relative;
  overflow: hidden;
}
.qq-card::before {
  content: "";
  position: absolute;
  top: 0; left: 0;
  width: 100%; height: 100%;
  background: radial-gradient(400px 120px at 100% 0%, rgba(99,102,241,0.06), transparent 60%);
  pointer-events: none;
  opacity: 0.6;
  transition: opacity .25s;
}
.qq-card:hover {
  transform: translateY(-3px);
  border-color: rgba(99,102,241,0.3);
  box-shadow:
    0 10px 28px -12px rgba(99,102,241,0.35),
    0 4px 10px -4px rgba(15,23,42,0.1);
}
.qq-card:hover::before { opacity: 1; }
.qq-icon {
  width: 42px; height: 42px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 20px;
  flex-shrink: 0;
  background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(139,92,246,0.08));
  border: 1px solid rgba(99,102,241,0.15);
}
.qq-icon-0 { background: linear-gradient(135deg, rgba(59,130,246,0.10), rgba(37,99,235,0.10)); border-color: rgba(59,130,246,0.22); }
.qq-icon-1 { background: linear-gradient(135deg, rgba(99,102,241,0.10), rgba(139,92,246,0.10)); border-color: rgba(99,102,241,0.22); }
.qq-icon-2 { background: linear-gradient(135deg, rgba(16,185,129,0.10), rgba(14,165,164,0.10)); border-color: rgba(16,185,129,0.22); }
.qq-icon-3 { background: linear-gradient(135deg, rgba(245,158,11,0.10), rgba(239,108,25,0.10)); border-color: rgba(245,158,11,0.22); }
.qq-icon-4 { background: linear-gradient(135deg, rgba(124,58,237,0.10), rgba(139,92,246,0.10)); border-color: rgba(124,58,237,0.22); }
.qq-icon-5 { background: linear-gradient(135deg, rgba(239,68,68,0.10), rgba(220,38,38,0.10)); border-color: rgba(239,68,68,0.22); }
.qq-body { flex: 1 1 auto; min-width: 0; }
.qq-title {
  font-size: 13.5px;
  font-weight: 650;
  color: var(--text-primary);
  margin-bottom: 3px;
}
.qq-desc {
  font-size: 12px;
  color: var(--text-tertiary);
  line-height: 1.55;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.qq-arrow {
  width: 22px; height: 22px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-tertiary);
  background: rgba(148,163,184,0.12);
  flex-shrink: 0;
  transition: all .2s;
}
.qq-card:hover .qq-arrow {
  color: #ffffff;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  transform: translateX(2px);
}

/* ===== 思考中 typing ===== */
.typing {
  display: flex;
  gap: 4px;
  padding: var(--space-3) var(--space-4);
  width: fit-content;
  background: var(--bg-raised);
  border-radius: var(--radius-lg);
  margin-bottom: var(--space-4);
  border: 1px solid var(--border-ghost);
  box-shadow: var(--shadow-ghost);
}
.typing span {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--text-quaternary);
  animation: blink 1.4s ease-in-out infinite;
}
.typing span:nth-child(2) { animation-delay: 0.2s; }
.typing span:nth-child(3) { animation-delay: 0.4s; }
@keyframes blink {
  0%, 60%, 100% { opacity: 0.25; }
  30% { opacity: 1; }
}

/* ===== 输入区 · 黄金比例渐变 ===== */
.share-readonly-banner {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  margin-bottom: 8px;
  border-radius: 8px;
  background: linear-gradient(90deg, rgba(99,102,241,0.08), rgba(6,182,212,0.08));
  border: 1px solid rgba(99,102,241,0.18);
  color: #334155;
  font-size: 12.5px;
}
.share-readonly-banner b { color: #4338ca; font-weight: 600; cursor: pointer; text-decoration: underline; }
.share-readonly-banner b:hover { color: #6366f1; }

/* ===== 对话快捷操作栏 ===== */
.chat-quick-bar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 26px 2px;
  border-top: 1px solid var(--border-ghost);
  background: var(--bg-surface-2);
  flex-wrap: wrap;
}
.chat-quick-bar .el-button {
  font-size: 12px !important;
  padding: 4px 10px !important;
  height: auto !important;
}
.qb-current {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: linear-gradient(135deg, rgba(99,102,241,0.08), rgba(14,165,233,0.06));
  border: 1px solid rgba(99,102,241,0.18);
  border-radius: 8px;
  margin-right: 4px;
}
.qb-icon {
  font-size: 14px;
}
.qb-name {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-primary);
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.qb-divider {
  width: 1px;
  height: 16px;
  background: var(--border-ghost);
  margin: 0 6px;
}

.chat-input.is-share {
  opacity: 0.96;
}
.chat-input.is-share .el-textarea :deep(textarea) {
  background: #f8fafc;
  color: #64748b;
  cursor: not-allowed;
}.chat-input {
  display: flex;
  gap: var(--space-3);
  padding: 16px 26px 22px;
  min-height: 164px;
  border-top: 1px solid var(--border-ghost);
  align-items: flex-end;
  background:
    linear-gradient(0deg, var(--bg-surface-2) 0%, var(--bg-surface) 100%),
    radial-gradient(1000px 300px at 0% 100%, rgba(14,165,233,0.04), transparent 60%);
}
.chat-input :deep(.el-textarea) {
  flex: 1;
}
.chat-input :deep(.el-textarea .el-textarea__inner) {
  min-height: 100px !important;
  padding: 14px 18px !important;
  font-size: 14.5px !important;
  line-height: 1.75 !important;
  border-radius: 14px !important;
  font-family: inherit !important;
}
.input-actions {
  display: flex;
  gap: var(--space-3);
  align-items: flex-end;
}

/* ===== Web / Artifact toggle ===== */
.web-toggle {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 38px;
  padding: 0 var(--space-4);
  border-radius: var(--radius-md);
  cursor: pointer;
  user-select: none;
  font-size: 13px;
  color: var(--text-tertiary);
  border: 1px solid var(--border-soft);
  background: var(--bg-input);
  transition: all var(--dur-2) var(--ease);
}
.web-toggle:hover {
  border-color: var(--border-focus);
  color: var(--brand-700);
  background: var(--brand-50);
}
.web-toggle.on {
  color: #0e7490;
  border-color: rgba(6, 182, 212, 0.35);
  background: var(--accent-50);
  box-shadow: inset 0 0 0 1px rgba(6, 182, 212, 0.10);
}
.web-toggle.on .el-icon {
  animation: web-pulse 2.4s ease-in-out infinite;
}
@keyframes web-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.55; }
}
.web-toggle.doc-on {
  color: #047857;
  border-color: rgba(22, 163, 74, 0.35);
  background: var(--success-50);
  box-shadow: inset 0 0 0 1px rgba(22, 163, 74, 0.10);
}
.web-toggle.doc-on .el-icon {
  animation: web-pulse 2.4s ease-in-out infinite;
}
.web-toggle.code-on {
  color: #6d28d9;
  border-color: rgba(124, 58, 237, 0.35);
  background: var(--violet-50);
  box-shadow: inset 0 0 0 1px rgba(124, 58, 237, 0.10);
}
.web-toggle.code-on .el-icon {
  animation: web-pulse 2.4s ease-in-out infinite;
}

/* ===== 历史恢复对话框 ===== */
.hist-tip {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-bottom: var(--space-3);
  line-height: 1.6;
}
.hist-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 360px;
  overflow-y: auto;
}
.hist-item {
  border: 1px solid var(--border-ghost);
  border-radius: var(--radius-md);
  padding: var(--space-3) var(--space-3);
  cursor: pointer;
  transition: all var(--dur-2) var(--ease);
  background: var(--bg-surface-2);
}
.hist-item:hover {
  border-color: rgba(79, 70, 229, 0.25);
  background: var(--brand-50);
  box-shadow: var(--shadow-ghost);
}
.hist-title {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  margin-bottom: var(--space-1);
}
.hist-meta {
  font-size: 12px;
  color: var(--text-quaternary);
  font-family: var(--font-mono, Consolas, monospace);
}

/* ===== 对话转任务 convert ===== */
.convert-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-3);
  padding: var(--space-5);
  color: var(--text-tertiary);
}
.convert-loading .spin {
  font-size: 28px;
  animation: spin 1s linear infinite;
  color: var(--brand-600);
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.convert-result {
  padding: var(--space-1) 0;
}
.result-section {
  margin-bottom: var(--space-3);
}
.result-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-tertiary);
  margin-bottom: var(--space-1);
}
.result-value {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary);
}
.result-steps {
  padding-left: var(--space-1);
}
.result-step {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) 0;
  font-size: 13px;
  color: var(--text-primary);
}
.result-step .step-num {
  width: 22px;
  height: 22px;
  background: var(--brand-600);
  color: #fff;
  border-radius: 50%;
  display: grid;
  place-items: center;
  font-size: 11px;
  font-weight: 700;
}
.result-meta {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-3);
  padding-top: var(--space-3);
  border-top: 1px solid var(--border-ghost);
}
.convert-empty {
  padding: var(--space-5) 0;
}

/* ===== 需求流程模式 · 问题选择面板（重构版 · 分组卡片） ===== */
.flow-empty {
  padding: 24px 28px 20px;
  max-width: 880px;
  margin: 0 auto;
}
.flow-empty-header {
  margin-bottom: 24px;
}
.flow-title-row {
  display: flex;
  align-items: center;
  gap: 14px;
}
.flow-icon-badge {
  width: 48px;
  height: 48px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  font-size: 24px;
  background: linear-gradient(135deg, #ede9fe, #dbeafe);
  flex-shrink: 0;
}
.flow-title {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
  line-height: 1.3;
}
.flow-desc {
  font-size: 13px;
  color: var(--text-tertiary);
  margin-top: 4px;
}

/* 分组 */
.issue-group {
  margin-bottom: 20px;
}
.issue-group-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 12px;
  padding-left: 2px;
}
.group-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

/* 卡片网格 */
.issue-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}
@media (max-width: 768px) {
  .issue-grid { grid-template-columns: 1fr; }
}

/* 问题卡片 */
.issue-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 16px;
  background: var(--bg-raised);
  border: 1.5px solid var(--border-ghost);
  border-radius: 12px;
  cursor: pointer;
  transition: all .2s ease;
  position: relative;
}
.issue-card:hover {
  border-color: rgba(99,102,241,0.35);
  transform: translateY(-2px);
  box-shadow: 0 8px 24px -12px rgba(99,102,241,0.25);
}
.issue-card.active {
  border-color: #6366f1;
  background: linear-gradient(135deg, #eef2ff, #faf5ff);
  box-shadow: 0 0 0 3px rgba(99,102,241,0.12);
}
.issue-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 18px;
  flex-shrink: 0;
}
.issue-info {
  flex: 1;
  min-width: 0;
}
.issue-name {
  font-size: 13.5px;
  font-weight: 650;
  color: var(--text-primary);
  margin-bottom: 3px;
}
.issue-desc {
  font-size: 11.5px;
  color: var(--text-tertiary);
  line-height: 1.5;
}
.issue-badge {
  position: absolute;
  top: 8px;
  right: 10px;
  font-size: 10px;
  padding: 2px 7px;
  background: linear-gradient(135deg, #6366f1, #8b5cf6);
  color: #fff;
  border-radius: 999px;
  font-weight: 600;
}

/* 自定义问题卡片 */
.custom-issue-card {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 16px;
  background: var(--bg-surface-2);
  border: 1.5px dashed rgba(245,158,11,0.3);
  border-radius: 12px;
  margin-bottom: 20px;
  transition: all .2s ease;
}
.custom-issue-card.active {
  border-color: #f59e0b;
  background: #fffbeb;
}
.custom-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 18px;
  flex-shrink: 0;
}
.custom-info {
  flex: 1;
  min-width: 0;
}
.custom-name {
  font-size: 13.5px;
  font-weight: 650;
  color: var(--text-primary);
  margin-bottom: 8px;
}
.custom-input-row {
  display: flex;
  gap: 8px;
}
.custom-input-row .el-input {
  flex: 1;
}

/* 启动栏 */
.flow-start-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 20px;
  background: linear-gradient(135deg, #eef2ff, #f0f9ff);
  border: 1px solid rgba(99,102,241,0.2);
  border-radius: 12px;
  position: sticky;
  bottom: 0;
}
.flow-start-info {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
}
.start-icon {
  font-size: 22px;
  flex-shrink: 0;
}
.start-text {
  min-width: 0;
}
.start-name {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}
.start-desc {
  font-size: 12px;
  color: var(--text-tertiary);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.flow-start-actions {
  display: flex;
  gap: 10px;
  flex-shrink: 0;
}

/* ===== 流程追踪器 ===== */
.flow-tracker {
  background: var(--bg-raised);
  border-radius: var(--radius-lg);
  padding: var(--space-4) var(--space-5);
  margin-bottom: var(--space-4);
  border: 1px solid var(--border-ghost);
  box-shadow: var(--shadow-ghost);
}
.flow-tracker-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: var(--space-4);
}
.flow-track-icon {
  font-size: 20px;
}
.flow-track-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
  flex: 1;
}
.flow-track-status {
  font-size: 12px;
  color: #6d28d9;
  font-weight: 600;
  padding: 4px 14px;
  background: var(--violet-50);
  border-radius: var(--radius-2xl);
}
.flow-stages {
  display: flex;
  align-items: center;
  justify-content: space-between;
  position: relative;
  margin-bottom: var(--space-3);
}
.stage-node {
  display: flex;
  flex-direction: column;
  align-items: center;
  z-index: 1;
}
.stage-circle {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  background: #e2e8f0;
  color: var(--text-tertiary);
  display: grid;
  place-items: center;
  font-size: 12px;
  font-weight: 700;
  margin-bottom: 6px;
  transition: all var(--dur-3) var(--ease);
}
.stage-node.active .stage-circle {
  background: #7c3aed;
  color: #fff;
  box-shadow: 0 0 0 5px rgba(124, 58, 237, 0.10);
  transform: scale(1.12);
}
.stage-node.done .stage-circle {
  background: #10b981;
  color: #fff;
}
.stage-label {
  font-size: 12px;
  color: var(--text-tertiary);
  font-weight: 500;
}
.stage-node.active .stage-label {
  color: #7c3aed;
  font-weight: 700;
}
.stage-node.done .stage-label {
  color: #10b981;
}
.stage-line {
  flex: 1;
  height: 2px;
  background: #e2e8f0;
  margin: 0 4px;
  margin-bottom: 24px;
  transition: background var(--dur-3) var(--ease);
}
.stage-line.done {
  background: linear-gradient(90deg, #10b981, #34d399);
}
.flow-stage-hint {
  font-size: 12px;
  color: #6d28d9;
  background: var(--violet-50);
  padding: 8px 12px;
  border-radius: var(--radius-md);
  line-height: 1.6;
}
.flow-stage-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-top: var(--space-3);
}

/* ===== 快速操作按钮组 ===== */
.flow-quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-3);
  padding: 12px 0 0;
  border-top: 1px solid var(--border-ghost);
}
.flow-quick-actions .el-button {
  flex: 1;
  min-width: 120px;
}

/* ===== 成果展示区 ===== */
.flow-results {
  margin-bottom: var(--space-4);
  background: linear-gradient(135deg, var(--bg-surface-2), #f1f5f9);
  border-radius: var(--radius-lg);
  padding: var(--space-4) var(--space-5);
  border: 1px solid var(--border-ghost);
}
.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-3);
}
.results-title {
  font-size: 15px;
  font-weight: 700;
  color: var(--text-primary);
}
.results-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  max-height: 400px;
  overflow-y: auto;
}
.result-item {
  display: flex;
  gap: var(--space-3);
  padding: 14px;
  background: var(--bg-raised);
  border-radius: var(--radius-md);
  border: 1px solid var(--border-ghost);
  transition: all var(--dur-2) var(--ease);
}
.result-item:hover {
  border-color: rgba(124, 58, 237, 0.25);
  box-shadow: 0 6px 18px -10px rgba(124, 58, 237, 0.20);
}
.result-item.analysis { border-left: 3px solid #3b82f6; }
.result-item.doc { border-left: 3px solid #10b981; }
.result-item.diagram { border-left: 3px solid #f59e0b; }
.result-item.dev { border-left: 3px solid #8b5cf6; }
.result-item.optimize { border-left: 3px solid #ec4899; }
.result-item.complete { border-left: 3px solid #7c3aed; }

.result-icon {
  font-size: 24px;
  flex-shrink: 0;
}
.result-content {
  flex: 1;
  min-width: 0;
}
.result-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 4px;
}
.result-body {
  font-size: 12px;
  color: var(--text-tertiary);
  line-height: 1.5;
}
.result-actions {
  flex-shrink: 0;
  align-self: flex-start;
}

/* 展开详情 */
.result-item.expanded {
  flex-direction: column;
  padding: 14px 16px;
}
.result-item.expanded .result-content {
  padding-top: var(--space-2);
  border-top: 1px dashed var(--border-soft);
}
.result-detail {
  margin-top: var(--space-3);
  padding: var(--space-3);
  background: var(--bg-surface-2);
  border: 1px solid var(--border-ghost);
  border-radius: var(--radius-md);
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.6;
  white-space: pre-wrap;
  max-height: 300px;
  overflow-y: auto;
}

/* ===== 当前问题栏 ===== */
.flow-current-issue-bar {
  background: linear-gradient(135deg, var(--violet-50), #e0e7ff);
  border: 1px solid rgba(124, 58, 237, 0.18);
  border-radius: var(--radius-lg);
  padding: var(--space-3) var(--space-4);
  margin-bottom: var(--space-4);
}
.flow-bar-content {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}
.flow-bar-icon {
  font-size: 20px;
}
.flow-bar-title {
  font-size: 14px;
  font-weight: 700;
  color: #4c1d95;
  flex: 1;
}

/* ===== 流程模式指示器 · 取代 Line 101 inline ===== */
.flow-mode-indicator {
  position: fixed;
  top: 80px;
  right: 26px;
  z-index: 2000;
  background: linear-gradient(135deg, #7c3aed 0%, #6366f1 100%);
  color: #fff;
  padding: 6px 14px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  box-shadow:
    0 6px 20px -6px rgba(124, 58, 237, 0.55),
    0 0 0 3px rgba(255, 255, 255, 0.75);
  letter-spacing: 0.02em;
}

/* ===== 字段标签行 · 取代 Line 427 inline ===== */
.field-tags-row {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

/* ========== 项目创建弹窗 ========== */
.req-source {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.dlg-tip {
  margin: var(--space-2) calc(var(--space-1) * -1) 0;
}
.muted {
  color: var(--text-tertiary);
  font-size: 12px;
}
.mono {
  font-family: var(--font-mono, Consolas, monospace);
}

/* ========== 产物面板 artifact-panel ========== */
.artifact-panel {
  margin: var(--space-4) var(--space-5) 0;
  background: linear-gradient(180deg, #f0f7ff 0%, var(--bg-raised) 40%);
  border: 1px solid rgba(59, 130, 246, 0.10);
  border-radius: var(--radius-lg);
  padding: var(--space-4) var(--space-5) var(--space-3);
  box-shadow: 0 10px 28px -16px rgba(37, 99, 235, 0.20);
}
.art-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-3);
  padding-bottom: var(--space-3);
  border-bottom: 1px dashed var(--border-soft);
}
.art-title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.art-emoji {
  font-size: 22px;
}
.art-proj {
  font-weight: 700;
  font-size: 15px;
  color: var(--text-primary);
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.art-meta {
  margin-left: var(--space-1);
}
.art-meta b.hl {
  color: #059669;
}
.art-actions {
  display: flex;
  gap: var(--space-2);
  flex-shrink: 0;
}
.art-tabs {
  margin-top: var(--space-2);
}
.art-tabs :deep(.el-tabs__item) {
  padding: 0 var(--space-3);
}

/* ===== 总览 over-grid ===== */
.over-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: var(--space-3);
}
@media (max-width: 960px) {
  .over-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
.over-card {
  background: var(--bg-raised);
  border: 1px solid var(--border-ghost);
  border-radius: var(--radius-md);
  padding: var(--space-3) var(--space-4);
  transition: all var(--dur-2) var(--ease);
}
.over-card:hover {
  box-shadow: var(--shadow-sm);
}
.over-k {
  font-size: 12px;
  color: var(--text-tertiary);
}
.over-v {
  font-weight: 600;
  color: var(--text-primary);
  margin-top: 2px;
  word-break: break-all;
}

/* ===== 云盘 KB 链接 ===== */
.kb-links {
  margin-top: var(--space-4);
}
.kb-title {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
}
.kb-list {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.kb-item {
  padding: 6px 14px;
  background: linear-gradient(135deg, #f0f9ff, #ecfeff);
  border: 1px solid rgba(14, 165, 233, 0.18);
  border-radius: 999px;
  font-size: 12px;
  cursor: pointer;
  transition: all var(--dur-2) var(--ease);
  display: inline-flex;
  gap: var(--space-2);
  align-items: center;
}
.kb-item:hover {
  transform: translateY(-1px);
  box-shadow: var(--shadow-ghost);
}
.kb-tag {
  color: #0369a1;
  font-weight: 600;
}
.kb-name {
  color: var(--text-primary);
}

/* ===== 图谱 mini legend ===== */
.legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
}
.lg {
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 11px;
  color: #fff;
}
.lg-project { background: #4f46e5; }
.lg-goal { background: #0ea5e9; }
.lg-actor { background: #10b981; }
.lg-usecase { background: #f59e0b; }
.lg-decision { background: #ef4444; }
.lg-data { background: #8b5cf6; }
.lg-end { background: #64748b; }
.nd {
  color: #fff !important;
  padding: 0 8px;
  border-radius: var(--radius-sm);
  font-size: 11px;
  font-family: var(--font-mono, Consolas, monospace);
}
.nd-project { background: #4f46e5; }
.nd-goal { background: #0ea5e9; }
.nd-actor { background: #10b981; color: #064e3b !important; }
.nd-usecase { background: #f59e0b; }
.nd-decision { background: #ef4444; }
.nd-data { background: #8b5cf6; }
.nd-end { background: #64748b; }
.nd-default { background: #94a3b8; }
.edges-box {
  margin-top: var(--space-3);
  background: var(--bg-surface-2);
  border: 1px solid var(--border-ghost);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
  max-height: 240px;
  overflow: auto;
}
.edges-title {
  font-weight: 600;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: var(--space-1);
}
.edge-row {
  font-size: 12px;
  padding: var(--space-1) 0;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.edge-type {
  color: var(--brand-600);
  background: var(--brand-50);
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  font-size: 11px;
}

/* ===== Mermaid / Code / MD box · 深空配色 ===== */
.mermaid-box,
.md-box,
.code-box {
  padding: 16px;
  border-radius: var(--radius-lg);
  font-size: 13px;
  line-height: 1.65;
  max-height: 520px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-mono, Consolas, 'JetBrains Mono', monospace);
  margin: var(--space-3) 0 var(--space-2);
}
.mermaid-box,
.code-box {
  background: #0b1120;
  color: #cbd5e1;
  border: 1px solid #0b1120;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04);
}
.md-box {
  background: var(--bg-surface-2);
  color: var(--text-primary);
  border: 1px solid var(--border-ghost);
  font-family: 'Inter', 'PingFang SC', 'Microsoft YaHei', sans-serif;
}
.erd-head h4 {
  margin: var(--space-3) 0 var(--space-2);
  font-size: 13px;
  color: var(--text-primary);
}

/* ========== 联盟 6 阶段 alliance-box ========== */
.alliance-box {
  padding: var(--space-1) var(--space-1);
}
.alliance-score-row {
  display: flex;
  gap: var(--space-5);
  align-items: center;
  background: linear-gradient(135deg, #eef2ff 0%, #ecfeff 60%, #faf5ff 100%);
  padding: var(--space-5);
  border-radius: var(--radius-lg);
  border: 1px solid rgba(124, 58, 237, 0.10);
  margin-bottom: var(--space-4);
}
.score-ring {
  width: 110px;
  height: 110px;
  flex-shrink: 0;
  border-radius: 50%;
  background:
    radial-gradient(circle, #fff 56%, transparent 57%),
    conic-gradient(#6366f1 0 var(--sc, 80%), #e2e8f0 0);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  box-shadow: 0 10px 26px -10px rgba(99, 102, 241, 0.45);
}
.score-value {
  font-size: 28px;
  font-weight: 800;
  color: #4338ca;
  line-height: 1.1;
}
.score-label {
  font-size: 11px;
  color: var(--text-tertiary);
}
.alliance-summary {
  flex: 1;
  min-width: 0;
}
.verdict-line {
  font-weight: 600;
  margin-bottom: var(--space-2);
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-primary);
}
.alliance-stages {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-3);
  margin-bottom: var(--space-3);
}
@media (max-width: 1100px) {
  .alliance-stages { grid-template-columns: repeat(2, 1fr); }
}
@media (max-width: 700px) {
  .alliance-stages { grid-template-columns: 1fr; }
}
.stage-card {
  background: var(--bg-raised);
  border: 1px solid var(--border-ghost);
  border-radius: var(--radius-md);
  padding: var(--space-4);
  transition: all var(--dur-2) var(--ease);
}
.stage-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-sm);
}
.stage-card.pass {
  border-color: rgba(16, 185, 129, 0.28);
  box-shadow: 0 10px 24px -14px rgba(16, 185, 129, 0.45);
}
.stage-top {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
  flex-wrap: wrap;
}
.stage-no {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--brand-600);
  color: #fff;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
}
.stage-card.pass .stage-no {
  background: #10b981;
}
.stage-name {
  font-weight: 700;
  color: var(--text-primary);
  flex: 1;
  min-width: 70px;
}
.stage-score {
  font-weight: 800;
  color: #059669;
  font-size: 14px;
}
.stage-score.low { color: #dc2626; }
.stage-score .thr {
  color: var(--text-quaternary);
  font-weight: 500;
  font-size: 12px;
}
.stage-summary {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: var(--space-2);
  line-height: 1.5;
}
.stage-deliver {
  font-size: 11px;
  color: var(--text-tertiary);
}
.gates-title {
  font-weight: 600;
  color: #334155;
  font-size: 13px;
  margin: 16px 0 10px;
}
.txt-ok { color: #059669; }
.txt-bad { color: #dc2626; }
.db-head {
  margin-bottom: var(--space-2);
}

/* ===== Element Plus 组件覆盖（scoped 内） ===== */
.chat :deep(.el-dialog) {
  border-radius: var(--radius-xl) !important;
  box-shadow: var(--shadow-lg) !important;
}
.chat :deep(.el-dialog__header) {
  padding: 20px 24px;
}
.chat :deep(.el-dialog__body) {
  padding: 10px 24px;
}
.chat :deep(.el-dialog__footer) {
  padding: 14px 24px 20px;
}
.chat :deep(.el-table) {
  --el-table-border-color: var(--border-ghost);
  --el-table-header-bg-color: var(--bg-surface-2);
  --el-table-row-hover-bg-color: var(--bg-surface-2);
  border-radius: var(--radius-md);
  overflow: hidden;
  border: 1px solid var(--border-ghost);
}

/* T11 alliance chips */
.alliance-chips { display: inline-flex; align-items: center; gap: 6px; margin-left: 12px; }
.alliance-chips .chip {
  display: inline-flex; align-items: center; gap: 6px;
  padding: 4px 10px; border-radius: 999px;
  background: rgba(86,105,179,0.08);
  color: #8a97b8; font-size: 12px;
  border: 1px solid rgba(138,151,184,0.2);
  transition: all .2s ease;
}
.alliance-chips .chip .dot { width: 6px; height: 6px; border-radius: 50%; background: #6d7c9e; }
.alliance-chips .chip.active {
  background: rgba(127,138,255,0.18); color: #c9d2ff;
  border-color: rgba(127,138,255,0.6);
  box-shadow: 0 0 14px rgba(127,138,255,0.15);
}
.alliance-chips .chip.active .dot { background: #7f8aff; animation: pulse 1.1s ease-in-out infinite; }
.alliance-chips .chip.done { color: #6ad8b0; border-color: rgba(106,216,176,0.4); background: rgba(106,216,176,0.08); }
.alliance-chips .chip.done .dot { background: #6ad8b0; }
@keyframes pulse { 0%,100% { transform: scale(1); opacity: 1 } 50% { transform: scale(1.5); opacity: .5 } }

/* T12 voice row */
.voice-row { display: inline-flex; align-items: center; gap: 10px; margin-right: 8px; }
.mic-btn {
  position: relative; width: 36px; height: 36px; border-radius: 50%;
  border: 1px solid rgba(138,151,184,0.3); background: rgba(86,105,179,0.06);
  color: #b9c3de; cursor: pointer; display: inline-flex; align-items: center; justify-content: center;
  transition: all .18s ease;
}
.mic-btn:hover { color: #fff; border-color: #7f8aff; background: rgba(127,138,255,0.15); }
.mic-btn.recording {
  color: #ff6a88; border-color: rgba(255,106,136,0.7);
  background: rgba(255,106,136,0.12);
  box-shadow: 0 0 0 0 rgba(255,106,136,0.35);
  animation: micPulse 1.1s ease-out infinite;
}
@keyframes micPulse {
  0% { box-shadow: 0 0 0 0 rgba(255,106,136,0.40); }
  70% { box-shadow: 0 0 0 14px rgba(255,106,136,0); }
  100% { box-shadow: 0 0 0 0 rgba(255,106,136,0); }
}
.mic-level-bar {
  position: absolute; right: -4px; top: 50%; transform: translateY(-50%);
  width: 3px; height: 24px; border-radius: 2px; background: rgba(138,151,184,0.18);
  overflow: hidden;
}
.mic-level-fill {
  display: block; width: 100%; background: linear-gradient(180deg,#7f8aff,#6ad8b0);
  position: absolute; left: 0; bottom: 0; transition: height .06s linear;
}
</style>
