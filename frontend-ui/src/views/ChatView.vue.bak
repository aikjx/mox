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
          <el-tooltip content="对话内容自动整理进知识图谱（全自动）" placement="bottom">
            <el-switch
              v-model="autoSync"
              inline-prompt
              active-text="自动入图"
              inactive-text="手动"
              @change="onToggleAutoSync"
            />
          </el-tooltip>
          <el-tooltip content="从后端恢复会话历史（跨设备同步）" placement="bottom">
            <el-button text @click="openBackendHistory"><el-icon><Clock /></el-icon></el-button>
          </el-tooltip>
          <el-tooltip content="导出对话+图谱迁移包" placement="bottom">
            <el-button text @click="exportBundle"><el-icon><Download /></el-icon></el-button>
          </el-tooltip>
          <el-tooltip content="导入迁移包" placement="bottom">
            <el-button text @click="pickImport"><el-icon><Upload /></el-icon></el-button>
          </el-tooltip>
          <el-tooltip content="将当前对话转换为任务" placement="bottom">
            <el-button text type="primary" @click="convertToTask" :loading="convertingTask">
              <el-icon><List /></el-icon> 转任务
            </el-button>
          </el-tooltip>
          <el-tooltip content="🏗️ 基于对话一键创建项目：自动生成需求图谱+流程图+PRD+ERD+需求-库关联矩阵+产品专家联盟企业级流水线报告，并归档云盘" placement="bottom">
            <el-button type="success" @click="openProjectDialog">
              <el-icon><FolderAdd /></el-icon> 创建项目
            </el-button>
          </el-tooltip>
          <el-tooltip content="开启后：AI分析对话自动创建任务并执行" placement="bottom">
            <el-switch
              v-model="autoTaskMode"
              inline-prompt
              active-text="任务模式"
              inactive-text="对话模式"
              @change="onAutoTaskToggle"
              style="margin-left: 8px"
            />
          </el-tooltip>
          <el-tooltip content="需求流程模式：选择问题，进入设计→分析→开发→测试→修复→优化流程" placement="bottom">
            <el-switch
              v-model="requirementFlowMode"
              inline-prompt
              active-text="流程模式"
              inactive-text=""
              @change="onRequirementFlowToggle"
              style="margin-left: 8px"
            />
          </el-tooltip>
          <input ref="importInput" type="file" accept="application/json" hidden @change="onImportFile" />
          <el-button text @click="clearChat"><el-icon><Delete /></el-icon> 清空</el-button>
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
      <div v-if="requirementFlowMode" style="position: fixed; top: 10px; right: 10px; z-index: 9999; background: #7c3aed; color: white; padding: 4px 10px; border-radius: 12px; font-size: 12px;">
        流程模式已开启
      </div>

      <div ref="scrollEl" class="chat-body">
        <!-- 需求流程模式：问题选择面板 -->
        <div v-if="requirementFlowMode && !currentIssue" class="flow-empty">
          <div class="flow-empty-header">
            <div class="flow-icon">🎯</div>
            <div class="flow-title">选择你的问题</div>
            <div class="flow-desc">选择问题类型，通过对话完成全维分析、文档生成、流程设计、开发测试优化</div>
          </div>
          <div class="issue-grid">
            <div
              v-for="issue in ISSUE_CATEGORIES"
              :key="issue.key"
              class="issue-card"
              :class="{ active: selectedIssue?.key === issue.key, primary: issue.primary }"
              @click="selectIssue(issue)"
            >
              <div class="issue-emoji">{{ issue.emoji }}</div>
              <div class="issue-name">{{ issue.label }}</div>
              <div class="issue-desc">{{ issue.desc }}</div>
              <div v-if="issue.primary" class="issue-badge">推荐</div>
            </div>
          </div>
          <div class="custom-issue-area">
            <div class="custom-label">💡 自定义问题：</div>
            <div class="custom-input-row">
              <el-input
                v-model="customIssueText"
                placeholder="输入你的自定义问题，系统将全维分析处理..."
                size="large"
              />
              <el-button
                type="primary"
                size="large"
                :disabled="!customIssueText.trim()"
                @click="createCustomIssue"
              >
                + 添加
              </el-button>
            </div>
          </div>
          <div v-if="selectedIssue" class="flow-start-area">
            <div class="flow-summary">
              已选择：<b>{{ selectedIssue.label }}</b> — {{ selectedIssue.desc }}
            </div>
            <div class="flow-start-actions">
              <el-button type="primary" size="large" @click="startRequirementFlow">
                🚀 启动全维流程
              </el-button>
              <el-button size="large" @click="quickAnalyze">
                ⚡ 快速分析
              </el-button>
            </div>
          </div>
        </div>

        <!-- 需求流程模式：进度追踪 -->
        <div v-if="requirementFlowMode && currentIssue" class="flow-tracker">
          <div class="flow-tracker-header">
            <span class="flow-track-icon">{{ currentIssue.emoji }}</span>
            <span class="flow-track-title">{{ currentIssue.label }}</span>
            <span class="flow-track-status">{{ FLOW_STAGES[currentStage].label }}</span>
            <el-dropdown trigger="click" @command="handleFlowCommand">
              <el-button size="small" type="primary">
                🎨 全维操作 <el-icon class="el-icon--right"><ArrowDown /></el-icon>
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="full_analysis">🔍 全维分析报告</el-dropdown-item>
                  <el-dropdown-item command="gen_requirement_doc">📝 生成需求文档</el-dropdown-item>
                  <el-dropdown-item command="optimize_doc">✨ 优化需求文档</el-dropdown-item>
                  <el-dropdown-item command="gen_flow_diagram">🔄 生成业务流程图</el-dropdown-item>
                  <el-dropdown-item command="gen_graph">📊 生成知识图谱</el-dropdown-item>
                  <el-dropdown-item command="gen_arch_diagram">🏗️ 生成架构图</el-dropdown-item>
                  <el-dropdown-item command="dev_test_fix">💻 开发测试修复</el-dropdown-item>
                  <el-dropdown-item command="full_complete">🚀 一键全维完成</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
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
          <p>我是算子统一系统 AI 助手，可以帮你<b>分析算法</b>、<b>推荐算子</b>、<b>解释图谱</b>。</p>
          <p v-if="autoTaskMode" class="task-mode-hint">🚀 任务模式已开启：对话中将自动创建并执行任务</p>
          <div class="suggestions">
            <el-tag v-for="q in quickQuestions" :key="q" class="q" @click="sendQuick(q)">{{ q }}</el-tag>
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
            @goto-task="goToTaskDetail"
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
                  <div style="display:flex; gap:4px; flex-wrap: wrap;">
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

      <div class="chat-input">
        <el-input
          v-model="draft"
          type="textarea"
          :rows="2"
          resize="none"
          placeholder="输入消息，Enter 发送 / Shift+Enter 换行"
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
          <el-button type="primary" :loading="thinking" @click="send">
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
import { ref, nextTick, onMounted, onUnmounted, watch, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { List, Loading, ArrowDown, Link, Document, FolderAdd } from '@element-plus/icons-vue'
import MessageBubble from '@/components/MessageBubble.vue'
import SessionSidebar from '@/components/SessionSidebar.vue'
import ToolDock from '@/components/ToolDock.vue'
import ToolDrawer from '@/components/ToolDrawer.vue'
import { AI_EXPERT_PRESETS } from '@/types'
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
  createProject
} from '@/api'

const router = useRouter()

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
  submitting: false,
  form: {
    name: '',
    category: 'platform',
    description: '',
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

// 问题类别
const ISSUE_CATEGORIES = [
  { key: 'requirement_graph', label: '需求知识图谱', emoji: '📊', desc: '生成项目需求的结构化知识图谱', primary: true },
  { key: 'business_process', label: '业务流程设计', emoji: '🔄', desc: '梳理和优化企业级业务流程' },
  { key: 'system_arch', label: '系统架构设计', emoji: '🏗️', desc: '设计和评估系统架构方案' },
  { key: 'plugin_dev', label: '插件开发', emoji: '🔌', desc: '开发和集成 AI 插件' },
  { key: 'automation_flow', label: '自动化任务流程', emoji: '⚡', desc: '设计自动化执行流程' },
  { key: 'mcp_integration', label: 'MCP 工具集成', emoji: '🔗', desc: '集成和配置 MCP 工具' }
]

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

async function send() {
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

watch(messages, persist, { deep: true })

onMounted(async () => {
  loadStore()
  // 支持 URL 参数：?flow=1 开启流程模式（支持 hash 路由）
  const hash = window.location.hash
  const hashParams = new URLSearchParams(hash.split('?')[1] || '')
  const searchParams = new URLSearchParams(window.location.search)
  if (hashParams.get('flow') === '1' || searchParams.get('flow') === '1') {
    await nextTick()
    requirementFlowMode.value = true
    persist()
  }
  if (!sessions.value.length) newSession()
  // 拉取后端全自动同步开关状态
  getAutoSyncStatus()
    .then((r) => { if (r) autoSync.value = !!(r.enabled ?? r.auto_sync ?? r.data?.auto_sync) })
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
onUnmounted(() => { if (streamTimer) clearInterval(streamTimer) })
</script>

<style scoped>
.chat {
  display: flex;
  height: calc(100vh - var(--header-h) - 42px - 44px);
  background: #fff;
  border-radius: var(--radius);
  box-shadow: var(--shadow-sm);
  overflow: hidden;
  flex-direction: row;
}
.chat-main { flex: 1; display: flex; flex-direction: column; min-width: 0; }
.chat-header {
  height: 56px; display: flex; align-items: center; justify-content: space-between;
  padding: 0 20px; border-bottom: 1px solid var(--border);
}
.chat-title { display: flex; align-items: center; gap: 8px; font-weight: 700; font-size: 15px; }
.chat-tools { display: flex; align-items: center; gap: 4px; }
.expert-selector {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-right: 8px;
}
.expert-selector .muted {
  font-size: 12px;
  color: var(--text-3);
}
.chat-body { flex: 1; overflow-y: auto; padding: 20px; background: #f8fafc; }
.empty { text-align: center; color: var(--text-3); margin-top: 50px; }
.empty-orb {
  width: 72px; height: 72px; margin: 0 auto 16px; border-radius: 50%;
  display: grid; place-items: center; font-size: 32px; color: #fff;
  background: linear-gradient(135deg, var(--brand-light), var(--accent));
  box-shadow: 0 10px 30px rgba(99, 102, 241, 0.4);
}
.empty p { max-width: 420px; margin: 0 auto 16px; line-height: 1.7; }
.task-mode-hint {
  background: linear-gradient(135deg, #fef3c7, #fde68a);
  color: #92400e;
  padding: 8px 16px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 600;
  display: inline-block;
  margin: 0 auto 16px;
}
.suggestions { display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; }
.q { cursor: pointer; }
.q:hover { background: var(--brand-soft); color: var(--brand-dark); }

.typing { display: flex; gap: 4px; padding: 12px 16px; width: fit-content; background: #fff; border-radius: 14px; margin-bottom: 14px; }
.typing span { width: 8px; height: 8px; border-radius: 50%; background: var(--text-3); animation: blink 1.2s infinite; }
.typing span:nth-child(2) { animation-delay: 0.2s; }
.typing span:nth-child(3) { animation-delay: 0.4s; }
@keyframes blink { 0%, 60%, 100% { opacity: 0.3; } 30% { opacity: 1; } }

.chat-input { display: flex; gap: 10px; padding: 14px 18px; border-top: 1px solid var(--border); align-items: flex-end; }
.chat-input :deep(.el-textarea) { flex: 1; }
.input-actions { display: flex; gap: 10px; align-items: flex-end; }
.web-toggle {
  display: flex; align-items: center; gap: 5px;
  height: 32px; padding: 0 12px; border-radius: 8px;
  cursor: pointer; user-select: none;
  font-size: 13px; color: var(--text-dim, #64748b);
  border: 1px solid var(--border, #e2e8f0);
  background: var(--bg-panel-2, #fff);
  transition: all 0.18s ease;
}
.web-toggle:hover { border-color: rgba(6, 182, 212, 0.5); color: #0891b2; }
.web-toggle.on {
  color: #0891b2;
  border-color: rgba(6, 182, 212, 0.55);
  background: rgba(6, 182, 212, 0.1);
  box-shadow: 0 0 0 1px rgba(6, 182, 212, 0.15) inset;
}
.web-toggle.on .el-icon { animation: web-pulse 2.4s ease-in-out infinite; }
@keyframes web-pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.55; }
}
/* 本地制品模式开关（文档/代码两态高亮） */
.web-toggle.doc-on {
  color: #16a34a;
  border-color: rgba(22, 163, 74, 0.55);
  background: rgba(22, 163, 74, 0.1);
  box-shadow: 0 0 0 1px rgba(22, 163, 74, 0.15) inset;
}
.web-toggle.doc-on .el-icon { animation: web-pulse 2.4s ease-in-out infinite; }
.web-toggle.code-on {
  color: #7c3aed;
  border-color: rgba(124, 58, 237, 0.55);
  background: rgba(124, 58, 237, 0.1);
  box-shadow: 0 0 0 1px rgba(124, 58, 237, 0.15) inset;
}
.web-toggle.code-on .el-icon { animation: web-pulse 2.4s ease-in-out infinite; }
.hist-tip {
  font-size: 12px; color: var(--text-3); margin-bottom: 12px; line-height: 1.6;
}
.hist-list { display: flex; flex-direction: column; gap: 8px; max-height: 360px; overflow-y: auto; }
.hist-item {
  border: 1px solid var(--border); border-radius: 10px; padding: 10px 12px; cursor: pointer;
  transition: all 0.2s;
}
.hist-item:hover { border-color: var(--brand); background: var(--brand-soft, #eef4ff); }
.hist-title { font-weight: 700; font-size: 14px; margin-bottom: 4px; }
.hist-meta { font-size: 12px; color: var(--text-3); font-family: var(--font-mono, monospace); }

.convert-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  padding: 20px;
  color: var(--text-3);
}
.convert-loading .spin {
  font-size: 28px;
  animation: spin 1s linear infinite;
  color: var(--brand);
}
@keyframes spin { to { transform: rotate(360deg); } }
.convert-result { padding: 4px 0; }
.result-section { margin-bottom: 12px; }
.result-label { font-size: 12px; font-weight: 600; color: var(--text-3); margin-bottom: 4px; }
.result-value { font-size: 14px; line-height: 1.6; color: var(--text-1); }
.result-steps { padding-left: 4px; }
.result-step {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 0; font-size: 13px; color: var(--text-1);
}
.result-step .step-num {
  width: 22px; height: 22px; background: var(--brand); color: #fff;
  border-radius: 50%; display: grid; place-items: center; font-size: 11px; font-weight: 700;
}
.result-meta { display: flex; gap: 8px; margin-top: 12px; padding-top: 12px; border-top: 1px solid var(--border); }
.convert-empty { padding: 20px 0; }

/* ===== 需求流程模式 ===== */
.flow-empty {
  text-align: center;
  padding: 30px 20px;
}
.flow-empty-header {
  margin-bottom: 24px;
}
.flow-icon {
  font-size: 48px;
  margin-bottom: 12px;
}
.flow-title {
  font-size: 22px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 8px;
}
.flow-desc {
  font-size: 14px;
  color: #64748b;
  line-height: 1.6;
}
.flow-desc b {
  color: #7c3aed;
}
.issue-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}
.issue-card {
  padding: 16px;
  background: #fff;
  border: 2px solid #e2e8f0;
  border-radius: 14px;
  cursor: pointer;
  transition: all 0.2s;
  text-align: left;
}
.issue-card:hover {
  border-color: #7c3aed;
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(124, 58, 237, 0.15);
}
.issue-card.active {
  border-color: #7c3aed;
  background: #f5f3ff;
}
.issue-emoji {
  font-size: 28px;
  margin-bottom: 8px;
}
.issue-name {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
  margin-bottom: 4px;
}
.issue-desc {
  font-size: 12px;
  color: #64748b;
  line-height: 1.4;
}
.flow-start-area {
  padding: 16px;
  background: linear-gradient(135deg, #f5f3ff, #ede9fe);
  border-radius: 14px;
  border: 1px solid #ddd6fe;
}
.flow-summary {
  font-size: 14px;
  color: #334155;
  margin-bottom: 12px;
}
.flow-summary b {
  color: #7c3aed;
}

/* 流程追踪器 */
.flow-tracker {
  background: #fff;
  border-radius: 12px;
  padding: 16px;
  margin-bottom: 16px;
  border: 1px solid #e2e8f0;
}
.flow-tracker-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
}
.flow-track-icon {
  font-size: 20px;
}
.flow-track-title {
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
  flex: 1;
}
.flow-track-status {
  font-size: 12px;
  color: #7c3aed;
  font-weight: 600;
  padding: 2px 10px;
  background: #ede9fe;
  border-radius: 20px;
}
.flow-stages {
  display: flex;
  align-items: center;
  justify-content: space-between;
  position: relative;
  margin-bottom: 10px;
}
.stage-node {
  display: flex;
  flex-direction: column;
  align-items: center;
  z-index: 1;
}
.stage-circle {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #e2e8f0;
  color: #64748b;
  display: grid;
  place-items: center;
  font-size: 12px;
  font-weight: 700;
  margin-bottom: 6px;
  transition: all 0.3s;
}
.stage-node.active .stage-circle {
  background: #7c3aed;
  color: #fff;
  box-shadow: 0 0 0 4px #ede9fe;
  transform: scale(1.1);
}
.stage-node.done .stage-circle {
  background: #10b981;
  color: #fff;
}
.stage-label {
  font-size: 11px;
  color: #64748b;
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
  transition: background 0.3s;
}
.stage-line.done {
  background: #10b981;
}
.flow-stage-hint {
  font-size: 12px;
  color: #7c3aed;
  background: #f5f3ff;
  padding: 8px 12px;
  border-radius: 8px;
  line-height: 1.5;
}
.flow-stage-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 10px;
}

/* 自定义问题输入 */
.custom-issue-area {
  margin-bottom: 20px;
  padding: 12px;
  background: #f8fafc;
  border-radius: 12px;
  border: 1px dashed #cbd5e1;
}
.custom-label {
  font-size: 13px;
  color: #64748b;
  margin-bottom: 8px;
  font-weight: 500;
}
.custom-input-row {
  display: flex;
  gap: 8px;
}
.custom-input-row .el-input {
  flex: 1;
}

/* 推荐标签 */
.issue-badge {
  position: absolute;
  top: 8px;
  right: 8px;
  font-size: 10px;
  padding: 2px 6px;
  background: linear-gradient(135deg, #7c3aed, #ec4899);
  color: #fff;
  border-radius: 10px;
  font-weight: 600;
}
.issue-card {
  position: relative;
}
.issue-card.primary {
  border-color: #7c3aed;
  background: linear-gradient(135deg, #f5f3ff, #fce7f3);
}

/* 启动按钮区 */
.flow-start-actions {
  display: flex;
  gap: 10px;
  justify-content: center;
}

/* 快速操作按钮组 */
.flow-quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid #f1f5f9;
}
.flow-quick-actions .el-button {
  flex: 1;
  min-width: 80px;
}

/* 成果展示区 */
.flow-results {
  margin-bottom: 16px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 14px;
  padding: 14px;
  border: 1px solid #e2e8f0;
}
.results-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}
.results-title {
  font-size: 15px;
  font-weight: 700;
  color: #0f172a;
}
.results-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  max-height: 400px;
  overflow-y: auto;
}
.result-item {
  display: flex;
  gap: 10px;
  padding: 12px;
  background: #fff;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
  transition: all 0.2s;
}
.result-item:hover {
  border-color: #7c3aed;
  box-shadow: 0 2px 8px rgba(124, 58, 237, 0.1);
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
  color: #0f172a;
  margin-bottom: 4px;
}
.result-body {
  font-size: 12px;
  color: #64748b;
  line-height: 1.5;
}
.result-actions {
  flex-shrink: 0;
  align-self: flex-start;
}

/* 展开详情 */
.result-item.expanded {
  flex-direction: column;
}
.result-item.expanded .result-content {
  padding-top: 8px;
  border-top: 1px dashed #e2e8f0;
}
.result-detail {
  margin-top: 10px;
  padding: 10px;
  background: #f8fafc;
  border-radius: 8px;
  font-size: 12px;
  color: #334155;
  line-height: 1.6;
  white-space: pre-wrap;
  max-height: 300px;
  overflow-y: auto;
}

/* 当前问题栏 */
.flow-current-issue-bar {
  background: linear-gradient(135deg, #ede9fe, #e0e7ff);
  border-radius: 10px;
  padding: 10px 14px;
  margin-bottom: 12px;
  border: 1px solid #c7d2fe;
}
.flow-bar-content {
  display: flex;
  align-items: center;
  gap: 10px;
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
/* ========== 项目创建弹窗 ========== */
.req-source { display: flex; flex-wrap: wrap; gap: 8px; }
.dlg-tip { margin: 8px -4px 0; }
.muted { color: var(--text-3); font-size: 12px; }
.mono { font-family: var(--font-mono, monospace); }

/* ========== 产物面板 ========== */
.artifact-panel {
  margin: 12px 16px 4px;
  background: linear-gradient(180deg, #f8fbff 0%, #ffffff 40%);
  border: 1px solid #dbeafe;
  border-radius: 14px;
  padding: 14px 16px 8px;
  box-shadow: 0 6px 20px -12px rgba(37, 99, 235, 0.25);
}
.art-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding-bottom: 10px;
  border-bottom: 1px dashed #cbd5e1;
}
.art-title { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.art-emoji { font-size: 22px; }
.art-proj { font-weight: 700; font-size: 15px; color: #0f172a; display: inline-flex; align-items: center; gap: 6px; }
.art-meta { margin-left: 4px; }
.art-meta b.hl { color: #059669; }
.art-actions { display: flex; gap: 6px; flex-shrink: 0; }
.art-tabs { margin-top: 8px; }
.art-tabs :deep(.el-tabs__item) { padding: 0 10px; }

/* 总览 */
.over-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 10px; }
.over-card {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  padding: 10px 12px;
}
.over-k { font-size: 12px; color: #64748b; }
.over-v { font-weight: 600; color: #0f172a; margin-top: 2px; word-break: break-all; }
@media (max-width: 960px) { .over-grid { grid-template-columns: repeat(2, 1fr); } }

.kb-links { margin-top: 14px; }
.kb-title { font-size: 13px; color: #475569; margin-bottom: 6px; }
.kb-list { display: flex; flex-wrap: wrap; gap: 6px; }
.kb-item {
  padding: 6px 10px;
  background: #f0f9ff;
  border: 1px solid #bae6fd;
  border-radius: 20px;
  font-size: 12px;
  cursor: pointer;
  transition: all .15s ease;
  display: inline-flex;
  gap: 6px;
  align-items: center;
}
.kb-item:hover { background: #e0f2fe; transform: translateY(-1px); }
.kb-tag { color: #0369a1; font-weight: 600; }
.kb-name { color: #0f172a; }

/* 图谱 mini */
.legend { display: flex; flex-wrap: wrap; gap: 6px; margin-bottom: 8px; }
.lg { padding: 2px 8px; border-radius: 10px; font-size: 11px; color: #fff; }
.lg-project { background: #4f46e5; }
.lg-goal { background: #0ea5e9; }
.lg-actor { background: #10b981; }
.lg-usecase { background: #f59e0b; }
.lg-decision { background: #ef4444; }
.lg-data { background: #8b5cf6; }
.lg-end { background: #64748b; }
.nd { color: #fff !important; padding: 0 8px; border-radius: 6px; font-size: 11px; font-family: var(--font-mono, monospace); }
.nd-project { background: #4f46e5; }
.nd-goal { background: #0ea5e9; }
.nd-actor { background: #10b981; color: #064e3b !important; }
.nd-usecase { background: #f59e0b; }
.nd-decision { background: #ef4444; }
.nd-data { background: #8b5cf6; }
.nd-end { background: #64748b; }
.nd-default { background: #94a3b8; }
.edges-box { margin-top: 10px; background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 8px 10px; max-height: 240px; overflow: auto; }
.edges-title { font-weight: 600; font-size: 12px; color: #334155; margin-bottom: 4px; }
.edge-row { font-size: 12px; padding: 2px 0; color: #475569; display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.edge-type { color: #6366f1; background: #eef2ff; padding: 0 6px; border-radius: 4px; font-size: 11px; }

.mermaid-box, .md-box, .code-box {
  background: #0f172a;
  color: #e2e8f0;
  padding: 12px 14px;
  border-radius: 10px;
  font-size: 12px;
  line-height: 1.6;
  max-height: 520px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-word;
  font-family: var(--font-mono, Consolas, monospace);
  margin: 8px 0 4px;
}
.code-box { border: 1px solid #1e293b; }
.md-box { background: #f8fafc; color: #0f172a; border: 1px solid #e2e8f0; }
.erd-head h4 { margin: 10px 0 6px; font-size: 13px; color: #0f172a; }

/* 联盟 6 阶段 */
.alliance-box { padding: 4px 2px; }
.alliance-score-row {
  display: flex; gap: 24px; align-items: center;
  background: linear-gradient(135deg, #eef2ff 0%, #ecfeff 100%);
  padding: 16px 20px;
  border-radius: 14px;
  border: 1px solid #c7d2fe;
  margin-bottom: 14px;
}
.score-ring {
  width: 110px; height: 110px; flex-shrink: 0;
  border-radius: 50%;
  background: radial-gradient(circle, #fff 56%, transparent 57%), conic-gradient(#6366f1 0 var(--sc, 80%), #e2e8f0 0);
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  box-shadow: 0 4px 18px -6px rgba(99, 102, 241, .35);
}
.score-value { font-size: 28px; font-weight: 800; color: #4338ca; line-height: 1.1; }
.score-label { font-size: 11px; color: #64748b; }
.alliance-summary { flex: 1; min-width: 0; }
.verdict-line { font-weight: 600; margin-bottom: 6px; display: inline-flex; align-items: center; gap: 8px; }
.alliance-stages {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
  margin-bottom: 12px;
}
@media (max-width: 1100px) { .alliance-stages { grid-template-columns: repeat(2, 1fr); } }
@media (max-width: 700px) { .alliance-stages { grid-template-columns: 1fr; } }
.stage-card {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  padding: 10px 12px;
  transition: all .15s ease;
}
.stage-card.pass { border-color: #86efac; box-shadow: 0 6px 14px -10px rgba(16, 185, 129, .4); }
.stage-top { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; flex-wrap: wrap; }
.stage-no {
  width: 24px; height: 24px; border-radius: 50%;
  background: #6366f1; color: #fff;
  display: inline-flex; align-items: center; justify-content: center;
  font-size: 12px; font-weight: 700;
}
.stage-card.pass .stage-no { background: #10b981; }
.stage-name { font-weight: 700; color: #0f172a; flex: 1; min-width: 70px; }
.stage-score { font-weight: 800; color: #059669; font-size: 14px; }
.stage-score.low { color: #dc2626; }
.stage-score .thr { color: #94a3b8; font-weight: 500; font-size: 12px; }
.stage-summary { font-size: 12px; color: #475569; margin-bottom: 6px; line-height: 1.5; }
.stage-deliver { font-size: 11px; }
.gates-title { font-weight: 600; color: #334155; font-size: 13px; margin: 10px 0 6px; }
.txt-ok { color: #059669; }
.txt-bad { color: #dc2626; }
.db-head { margin-bottom: 8px; }
</style>
