<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">企业级专家管理控制台</h2>
        <p class="page-subtitle">会话持久化 · 调度策略 · 能力图谱 · 架构协作</p>
      </div>
      <div class="page-header-actions">
        <el-button @click="loadAll" :loading="loading">
          <el-icon><Refresh /></el-icon> 全量刷新
        </el-button>
        <el-button type="primary" @click="runDiagnostic" :loading="diagnosticLoading">
          <el-icon><DataAnalysis /></el-icon> 系统诊断
        </el-button>
      </div>
    </div>

    <div class="page-content">

    <el-tabs v-model="activeTab" type="border-card" class="main-tabs">
      <!-- 总览仪表盘 -->
      <el-tab-pane label="仪表盘" name="overview">
        <div class="kpi-grid">
          <div class="kpi-card" v-for="kpi in kpiCards" :key="kpi.label" :class="kpi.color">
            <div class="kpi-icon"><el-icon :size="28"><component :is="kpi.icon" /></el-icon></div>
            <div class="kpi-body">
              <div class="kpi-value">{{ kpi.value }}</div>
              <div class="kpi-label">{{ kpi.label }}</div>
              <div class="kpi-desc">{{ kpi.desc }}</div>
            </div>
            <div v-if="kpi.trend" class="kpi-trend" :class="kpi.trend > 0 ? 'up' : 'down'">
              {{ kpi.trend > 0 ? '↑' : '↓' }} {{ Math.abs(kpi.trend) }}%
            </div>
          </div>
        </div>

        <el-row :gutter="16" class="main-row">
          <el-col :span="14">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>专家能力图谱</h3>
                <el-button size="small" @click="loadGraphStats">刷新</el-button>
              </div>
              <div v-if="graphStats" class="graph-overview">
                <div class="graph-stats-row">
                  <div class="graph-stat">
                    <span class="stat-val">{{ graphStats.total_nodes }}</span>
                    <span class="stat-lbl">专家节点</span>
                  </div>
                  <div class="graph-stat">
                    <span class="stat-val">{{ graphStats.total_edges }}</span>
                    <span class="stat-lbl">协作关系</span>
                  </div>
                  <div class="graph-stat">
                    <span class="stat-val">{{ (graphStats.density * 100).toFixed(1) }}%</span>
                    <span class="stat-lbl">网络密度</span>
                  </div>
                  <div class="graph-stat">
                    <span class="stat-val">{{ graphStats.communities?.length || 0 }}</span>
                    <span class="stat-lbl">专家社群</span>
                  </div>
                </div>
                <div class="communities-section">
                  <h4>专家社群划分</h4>
                  <div v-if="graphStats.communities?.length" class="communities-list">
                    <div v-for="c in graphStats.communities" :key="c.id" class="community-card">
                      <div class="community-name">{{ c.id }} ({{ c.size }}人)</div>
                      <div class="community-type">主导类型: {{ c.dominant_type || '未知' }}</div>
                      <div class="community-members">
                        <el-tag v-for="m in c.members?.slice(0, 4)" :key="m.id" size="small" effect="plain">
                          {{ m.label || m.name }}
                        </el-tag>
                        <span v-if="c.size > 4" class="more">+{{ c.size - 4 }}</span>
                      </div>
                    </div>
                  </div>
                  <el-empty v-else description="暂无社群数据" :image-size="60" />
                </div>
                <div class="type-distribution">
                  <h4>专家类型分布</h4>
                  <div class="type-bars">
                    <div v-for="(count, type) in graphStats.type_distribution" :key="type" class="type-bar-row">
                      <span class="type-name">{{ typeLabels[type] || type }}</span>
                      <div class="type-bar">
                        <div class="type-bar-fill" :style="{ width: (count * 100 / Math.max(...Object.values(graphStats.type_distribution))) + '%' }"></div>
                      </div>
                      <span class="type-count">{{ count }}</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </el-col>
          <el-col :span="10">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>调度策略引擎</h3>
                <el-select v-model="dispatcherConfig" size="small" style="width: 140px" @change="updateStrategy">
                  <el-option v-for="s in strategyOptions" :key="s.value" :label="s.label" :value="s.value" />
                </el-select>
              </div>
              <div v-if="dispatcherStatus" class="dispatcher-panel">
                <div class="strategy-desc">{{ currentStrategyDesc }}</div>
                <div class="circuit-breaker-section">
                  <h4>熔断器状态</h4>
                  <div v-if="dispatcherStatus.circuit_breaker?.states?.length" class="cb-list">
                    <div v-for="cb in dispatcherStatus.circuit_breaker.states" :key="cb.key" class="cb-item" :class="cb.status">
                      <span class="cb-key">{{ cb.key }}</span>
                      <span class="cb-status">{{ cbStatusLabel(cb.status) }}</span>
                      <span class="cb-failures">失败 {{ cb.failures }}</span>
                    </div>
                  </div>
                  <div v-else class="cb-empty">所有专家服务正常</div>
                </div>
                <div class="dispatch-log">
                  <h4>最近调度</h4>
                  <div v-if="dispatcherStatus.dispatcher?.recent_dispatches?.length" class="dispatch-list">
                    <div v-for="d in dispatcherStatus.dispatcher.recent_dispatches" :key="d.id" class="dispatch-item">
                      <span class="ds-expert">{{ d.expert_id }}</span>
                      <span class="ds-strategy">{{ strategyLabels[d.strategy] || d.strategy }}</span>
                      <span class="ds-time">{{ formatTime(d.timestamp) }}</span>
                    </div>
                  </div>
                  <el-empty v-else description="暂无调度记录" :image-size="40" />
                </div>
              </div>
            </div>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- 会话管理 -->
      <el-tab-pane label="会话中心" name="sessions">
        <div class="session-toolbar">
          <el-input v-model="sessionSearch" placeholder="搜索会话标题或内容" style="width: 300px" clearable>
            <template #prefix><el-icon><Search /></el-icon></template>
          </el-input>
          <el-select v-model="sessionFilterStatus" placeholder="状态" style="width: 120px" clearable>
            <!-- 选项集合取自契约：旧版只有 active/archived 两档，已关闭的会话根本筛不到（§5.38） -->
            <el-option v-for="o in SESSION_STATUSES" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
          <el-select v-model="sessionFilterMode" placeholder="模式" style="width: 140px" clearable>
            <!-- 旧版是一张私表：smart/multi_expert/algorithm 都不是 session_type 的取值，选了就筛空（§5.38） -->
            <el-option v-for="o in SESSION_TYPES" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
          <el-button type="primary" @click="loadSessions"><el-icon><Search /></el-icon> 查询</el-button>
          <el-button @click="createNewSession"><el-icon><Plus /></el-icon> 新建会话</el-button>
        </div>

        <div class="session-grid">
          <div v-for="s in filteredSessions" :key="s.id" class="session-card" @click="openSession(s)">
            <div class="session-header">
              <el-tag :type="sessionStatusColor(s.status)" size="small">{{ sessionStatusLabel(s.status) }}</el-tag>
              <el-tag size="small" effect="plain">{{ sessionTypeLabel(s.sessionType) }}</el-tag>
              <span class="session-updated">{{ formatTime(s.updated_at) }}</span>
            </div>
            <div class="session-title">{{ s.title || '新对话' }}</div>
            <div class="session-preview">{{ getSessionPreview(s) }}</div>
            <div class="session-meta">
              <span><el-icon><ChatDotRound /></el-icon> {{ s.messages?.length || 0 }} 消息</span>
              <span v-if="s.metadata?.tags?.length">
                <el-tag v-for="t in s.metadata.tags" :key="t" size="small" type="info">{{ t }}</el-tag>
              </span>
            </div>
          </div>
        </div>

        <div v-if="sessions.length === 0" class="empty-state">
          <el-empty description="暂无会话记录">
            <el-button type="primary" @click="createNewSession">创建新会话</el-button>
          </el-empty>
        </div>
      </el-tab-pane>

      <!-- 图谱分析 -->
      <el-tab-pane label="能力图谱" name="graph">
        <el-row :gutter="16">
          <el-col :span="16">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>专家协作图谱</h3>
                <div>
                  <el-button size="small" @click="loadGraph">加载图谱</el-button>
                  <el-button size="small" type="primary" @click="rebuildGraph">重建图谱</el-button>
                </div>
              </div>
              <div v-if="graphData" class="graph-visual">
                <div class="graph-legend">
                  <span class="legend-item" v-for="(label, type) in typeLabels" :key="type">
                    <span class="legend-dot" :style="{ background: getTypeColor(type) }"></span>
                    {{ label }}
                  </span>
                </div>
                <div class="graph-nodes">
                  <div v-for="node in graphData.nodes?.slice(0, 30)" :key="node.id"
                       class="graph-node"
                       :style="{ left: getNodePosition(node.id, graphData.nodes).x + '%', top: getNodePosition(node.id, graphData.nodes).y + '%', background: getTypeColor(node.type) }"
                       @click="exploreExpert(node)">
                    <span class="node-label">{{ node.label }}</span>
                  </div>
                </div>
                <div v-if="graphData.nodes?.length > 30" class="graph-more">仅展示前 30 位专家</div>
              </div>
              <el-empty v-else description="点击加载图谱查看" :image-size="80" />
            </div>
          </el-col>
          <el-col :span="8">
            <div class="panel card-pad">
              <div class="panel-head"><h3>专家详情</h3></div>
              <div v-if="selectedExpert" class="expert-detail">
                <div class="expert-header">
                  <el-avatar :size="48" :style="{ background: getTypeColor(selectedExpert.type) }">
                    {{ selectedExpert.label?.charAt(0) || 'E' }}
                  </el-avatar>
                  <div>
                    <h4>{{ selectedExpert.label }}</h4>
                    <el-tag size="small">{{ typeLabels[selectedExpert.type] || selectedExpert.type }}</el-tag>
                  </div>
                </div>
                <div class="expert-capabilities">
                  <h5>能力标签</h5>
                  <el-tag v-for="cap in selectedExpert.capabilities" :key="cap" size="small" type="info">{{ cap }}</el-tag>
                </div>
                <div class="expert-metrics" v-if="selectedExpert.metrics">
                  <h5>绩效指标</h5>
                  <div class="metric-row">
                    <span>咨询次数</span>
                    <el-progress :percentage="Math.min(100, (selectedExpert.metrics.consult_count || 0) / 10)" :stroke-width="8" />
                  </div>
                  <div class="metric-row">
                    <span>成功率</span>
                    <el-progress :percentage="(selectedExpert.metrics.success_rate || 0) * 100" :color="successRateColor" :stroke-width="8" />
                  </div>
                  <div class="metric-row">
                    <span>平均置信度</span>
                    <el-progress :percentage="(selectedExpert.metrics.avg_confidence || 0) * 100" :color="confidenceColor" :stroke-width="8" />
                  </div>
                </div>
                <div class="expert-collaborators">
                  <h5>协作专家 ({{ selectedCollaborators.length }})</h5>
                  <div v-if="selectedCollaborators.length" class="collab-list">
                    <div v-for="c in selectedCollaborators" :key="c.expert_id" class="collab-item">
                      <span>{{ c.expert?.label || c.expert_id }}</span>
                      <span class="collab-weight">权重: {{ c.weight }}</span>
                    </div>
                  </div>
                  <el-empty v-else description="暂无协作数据" :image-size="40" />
                </div>
              </div>
              <el-empty v-else description="点击左侧专家查看详情" :image-size="60" />
            </div>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- 流程编排（业务流程+算法流程统一图谱） -->
      <el-tab-pane label="流程编排" name="flow">
        <el-row :gutter="16">
          <el-col :span="16">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>AI 流程图谱</h3>
                <div class="flow-head-actions">
                  <el-select v-model="flowFocus" size="small" style="width: 150px" placeholder="聚焦视图" clearable @change="renderFlowGraph">
                    <el-option label="全部节点" value="all" />
                    <el-option label="仅流水线骨架" value="pipeline" />
                    <el-option label="能力与引擎" value="capability" />
                  </el-select>
                  <el-button size="small" @click="loadFlowGraph" :loading="flowLoading">
                    <el-icon><Refresh /></el-icon> 刷新
                  </el-button>
                </div>
              </div>
              <div v-if="flowGraphData" class="flow-stats-row">
                <div class="flow-stat">
                  <span class="stat-val">{{ flowGraphData.stats?.node_count }}</span>
                  <span class="stat-lbl">节点</span>
                </div>
                <div class="flow-stat">
                  <span class="stat-val">{{ flowGraphData.stats?.edge_count }}</span>
                  <span class="stat-lbl">连线</span>
                </div>
                <div class="flow-stat">
                  <span class="stat-val">{{ flowGraphData.stats?.by_type?.step || 0 }}</span>
                  <span class="stat-lbl">流水线步骤</span>
                </div>
                <div class="flow-stat">
                  <span class="stat-val">{{ flowGraphData.stats?.by_type?.capability || 0 }}</span>
                  <span class="stat-lbl">AI 能力</span>
                </div>
                <div class="flow-stat">
                  <span class="stat-val">{{ flowGraphData.stats?.by_type?.engine || 0 }}</span>
                  <span class="stat-lbl">委托引擎</span>
                </div>
              </div>
              <div v-if="flowGraphData" class="flow-legend">
                <span class="legend-item" v-for="(meta, type) in flowNodeTypes" :key="type">
                  <span class="legend-dot" :style="{ background: `var(${meta.token})` }"></span>
                  {{ meta.label }}
                </span>
                <span class="legend-edge">
                  <i class="edge-line flows"></i>流转
                  <i class="edge-line triggers"></i>触发
                  <i class="edge-line delegates"></i>委托
                  <i class="edge-line degrades"></i>降级
                </span>
              </div>
              <div v-show="flowGraphData" ref="flowChart" class="flow-chart"></div>
              <el-empty v-if="!flowGraphData && !flowLoading" description="点击刷新加载流程图谱" :image-size="80" />
            </div>
          </el-col>
          <el-col :span="8">
            <div class="panel card-pad flow-side">
              <div class="panel-head"><h3>专家联盟六阶段流水线</h3></div>
              <div class="pipeline-steps">
                <div v-for="(stage, idx) in pipelineStages" :key="stage.key" class="pipeline-stage">
                  <div class="stage-dot" :class="{ active: activeStage === stage.key }" @click="activeStage = stage.key">
                    {{ idx + 1 }}
                  </div>
                  <div class="stage-body" v-show="true">
                    <div class="stage-title">{{ stage.title }}</div>
                    <div class="stage-desc">{{ stage.desc }}</div>
                    <div v-if="activeStage === stage.key" class="stage-detail">
                      <div class="stage-api">{{ stage.api }}</div>
                    </div>
                  </div>
                </div>
              </div>
              <div class="flow-formula" v-if="flowGraphData?.formulas">
                <h4>激活扩散公式</h4>
                <div class="formula-code">{{ flowGraphData.formulas.activation_spread }}</div>
                <div class="formula-note">{{ flowGraphData.formulas.note }}</div>
              </div>
            </div>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- 企业协作 -->
      <el-tab-pane label="企业协作" name="enterprise">
        <el-row :gutter="16">
          <el-col :span="12">
            <div class="panel card-pad">
              <div class="panel-head"><h3>智能协作咨询</h3></div>
              <div class="enterprise-form">
                <el-form :model="enterpriseForm" label-width="100px" label-position="right">
                  <el-form-item label="问题描述">
                    <el-input v-model="enterpriseForm.question" type="textarea" :rows="4" placeholder="请输入需要专家协作分析的问题..." />
                  </el-form-item>
                  <el-form-item label="协作模式">
                    <el-select v-model="enterpriseForm.mode" style="width: 100%">
                      <el-option label="智能路由 (推荐)" value="smart" />
                      <el-option label="单专家咨询" value="single" />
                      <el-option label="多专家协同" value="multi_expert" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="调度策略">
                    <el-select v-model="enterpriseForm.strategy" style="width: 100%">
                      <el-option v-for="s in strategyOptions" :key="s.value" :label="s.label" :value="s.value" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="业务标签">
                    <el-select v-model="enterpriseForm.tags" multiple filterable allow-create style="width: 100%">
                      <el-option label="性能" value="性能" />
                      <el-option label="架构" value="架构" />
                      <el-option label="算法" value="算法" />
                      <el-option label="数据" value="数据" />
                      <el-option label="安全" value="安全" />
                      <el-option label="AI" value="AI" />
                    </el-select>
                  </el-form-item>
                  <el-form-item>
                    <el-button type="primary" @click="submitEnterpriseConsult" :loading="enterpriseLoading">
                      <el-icon><Connection /></el-icon> 发起协作
                    </el-button>
                  </el-form-item>
                </el-form>
              </div>
            </div>
          </el-col>
          <el-col :span="12">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>协作结果</h3>
                <el-button size="small" @click="loadOptimalTeam" :disabled="!enterpriseForm.question">
                  <el-icon><User /></el-icon> 推荐专家团队
                </el-button>
              </div>
              <div v-if="enterpriseResult" class="enterprise-result">
                <div class="result-section">
                  <h4>调度信息</h4>
                  <div class="result-meta">
                    <el-tag v-if="enterpriseResult.dispatch?.success" type="success">调度成功</el-tag>
                    <el-tag v-else type="danger">调度失败</el-tag>
                    <span v-if="enterpriseResult.dispatch?.dispatch" class="strategy-tag">
                      {{ strategyLabels[enterpriseResult.dispatch.dispatch.strategy] || enterpriseResult.dispatch.dispatch.strategy }}
                    </span>
                  </div>
                </div>
                <div class="result-section" v-if="enterpriseResult.dispatch?.result?.response">
                  <h4>专家回答</h4>
                  <div class="response-box">{{ enterpriseResult.dispatch.result.response }}</div>
                </div>
                <div class="result-section" v-if="enterpriseResult.optimal_team">
                  <h4>最优专家团队</h4>
                  <div class="team-list">
                    <div v-for="(s, idx) in enterpriseResult.optimal_team.scores" :key="idx" class="team-member">
                      <el-tag :style="{ background: getTypeColor(s.expert.type), color: '#fff' }">{{ s.expert.name }}</el-tag>
                      <span class="team-score">匹配度: {{ (s.score * 100).toFixed(1) }}%</span>
                    </div>
                  </div>
                </div>
                <div class="result-section" v-if="enterpriseResult.context_used">
                  <h4>上下文记忆</h4>
                  <div class="context-info">
                    <el-tag type="info" size="small">已使用历史上下文</el-tag>
                    <span>找到 {{ enterpriseResult.similar_history_found }} 条相似历史</span>
                  </div>
                </div>
              </div>
              <el-empty v-else description="发起协作后查看结果" :image-size="80" />
            </div>
          </el-col>
        </el-row>
      </el-tab-pane>
    </el-tabs>
    </div>

    <!-- 企业级系统诊断结果 -->
    <el-dialog v-model="diagnosticVisible" title="企业级系统诊断报告" width="560px" :close-on-click-modal="false">
      <div v-if="diagnosticLoading" class="diag-loading">
        <el-icon class="is-loading" :size="28"><Refresh /></el-icon>
        <span>正在执行架构维度健康检查…</span>
      </div>
      <div v-else class="diag-body">
        <div class="diag-time">诊断时间：{{ diagnosticTime }} · 覆盖 会话 / 图谱 / 调度 三维度</div>
        <div class="diag-items">
          <div v-for="item in diagnosticItems" :key="item.label" class="diag-item">
            <span class="diag-icon" :class="item.level">
              <el-icon :size="16">
                <component :is="item.level === 'ok' ? CircleCheckFilled : (item.level === 'warn' ? WarningFilled : CircleCloseFilled)" />
              </el-icon>
            </span>
            <div class="diag-content">
              <div class="diag-title">
                <span>{{ item.label }}</span>
                <span class="diag-value">{{ item.value }}</span>
              </div>
              <div class="diag-desc">{{ item.desc }}</div>
            </div>
            <span class="diag-level" :class="item.level">{{ item.levelText }}</span>
          </div>
        </div>
        <div class="diag-summary" :class="diagnosticSummaryLevel">
          <el-icon :size="16">
            <component :is="diagnosticSummaryLevel === 'ok' ? CircleCheckFilled : (diagnosticSummaryLevel === 'warn' ? WarningFilled : CircleCloseFilled)" />
          </el-icon>
          <span>{{ diagnosticSummary }}</span>
        </div>
      </div>
      <template #footer>
        <el-button @click="diagnosticVisible = false">关闭</el-button>
        <el-button type="primary" :loading="diagnosticLoading" @click="runDiagnostic">重新诊断</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount, watch, nextTick, markRaw } from 'vue'
import * as echarts from '@/echarts'
import {
  DataAnalysis, Refresh, Plus, Search, User, Connection, ChatDotRound,
  CircleCheckFilled, WarningFilled, CircleCloseFilled
} from '@element-plus/icons-vue'
import { allianceApi } from '@/modules/expert-alliance/api'
// 合法例外：
//   enterpriseConsult  → /experts/enterprise/consult 在 contract UNMOUNTED_ROUTES 里 rejected（模板桩，无 LLM），模块不挂载
//   getEngineFlowGraph → /ai/engine/flow-graph 属 AI 引擎域，非 alliance 契约面
import { enterpriseConsult, getEngineFlowGraph } from '@/api'

// legacy 方法名 → 模块 allianceApi；返回值补视图既有 snake_case 字段别名，视图逻辑不改
const api = {
  getExpertSessions: (filters) => allianceApi.listSessions({
    status: filters?.status, sessionType: filters?.session_type
  }).then((r) => ({ sessions: (r.items || []).map((s) => ({ ...s, updated_at: s.lastActiveAt })) })),
  getExpertGraphStats: () => allianceApi.graphStats().then((s) => ({ ...s, total_nodes: s.totalNodes, total_edges: s.totalEdges })),
  getDispatcherStatus: () => allianceApi.dispatcherStatus().then((d) => ({ ...d, circuit_breakers: d.circuitBreakers || [] })),
  getExpertGraph: () => allianceApi.graphOverview(),
  updateDispatcherConfig: (patch) => allianceApi.updateDispatcherConfig(patch),
  rebuildExpertGraph: () => allianceApi.rebuildGraph(),
  createExpertSession: (draft) => allianceApi.createSession({ title: draft.title }).then((s) => ({ ...s, updated_at: s.lastActiveAt })),
  getExpertGraphCollaborators: (id, limit) => allianceApi.graphCollaborators(id, limit),
  findOptimalTeam: (input) => allianceApi.optimalTeam({ goal: input.question, maxMembers: input.size }),
  enterpriseConsult,
  getEngineFlowGraph
}
import { tokenColor, themeRevision } from '@/constants'
import { formatDateTimeLocaleOr } from '@/utils'
import { sessionStatusLabel, sessionTypeLabel, SESSION_STATUSES, SESSION_TYPES, SESSION_STATUS, MESSAGE_ROLE, BREAKER_STATE } from '@/modules/expert-alliance/contract'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'

const activeTab = ref('overview')
const loading = ref(false)
const overview = ref(null)
const graphStats = ref(null)
const dispatcherStatus = ref(null)
const dispatcherConfig = ref('content_aware')
const dispatcherConfigData = ref(null)
const sessions = ref([])
const sessionSearch = ref('')
const sessionFilterStatus = ref('')
const sessionFilterMode = ref('')
const graphData = ref(null)
const selectedExpert = ref(null)
const selectedCollaborators = ref([])
const enterpriseForm = ref({ question: '', mode: 'smart', strategy: 'content_aware', tags: [] })
const enterpriseResult = ref(null)
const enterpriseLoading = ref(false)

// ===== 企业级系统诊断 =====
const diagnosticVisible = ref(false)
const diagnosticLoading = ref(false)
const diagnosticTime = ref('')
const diagnosticItems = ref([])
const diagnosticSummary = ref('')
const diagnosticSummaryLevel = ref('ok')

// ===== 流程编排 tab 状态 =====
const flowGraphData = ref(null)
const flowLoading = ref(false)
const flowFocus = ref('all')
const flowChart = ref(null)
const activeStage = ref('intent')
let flowInst = null

// 存档位名而不是色值：色值真值只在主题命名空间里。图例那侧 :style 能解析 var() 所以自己跟着换肤，
// 画布（echarts 的 SVG 呈现属性）解析不了，只能在 buildFlowOption 里按名现取。
const flowNodeTypes = {
  step: { label: '流水线步骤', token: '--cat-1', size: 46 },
  keyword: { label: '意图关键词', token: '--text-tertiary', size: 14 },
  capability: { label: 'AI 能力', token: '--cat-2', size: 36 },
  engine: { label: '委托引擎', token: '--cat-6', size: 30 }
}

const flowEdgeStyles = {
  flows_to: { token: '--cat-1', width: 3, type: 'solid' },
  triggers: { token: '--text-tertiary', width: 1, type: 'solid' },
  delegates_to: { token: '--cat-2', width: 2, type: 'dashed' },
  degrades_to: { token: '--danger', width: 1.5, type: 'dashed' }
}

const pipelineStages = [
  { key: 'intent', title: '意图识别', desc: '激活扩散：命中关键词→能力激活（个性化 PageRank）', api: 'classifyIntent · INTENT_PATTERNS 15 意图域' },
  { key: 'team', title: '最优组队', desc: '能力匹配 + 图谱协同增益 + 负载均衡多目标选择', api: 'composeTeam · ExpertGraph 边权' },
  { key: 'deliberate', title: '并行辩论', desc: '并行咨询 + 2 轮交叉评审收敛（加权表决）', api: 'deliberate · Jaccard 共识度' },
  { key: 'synthesize', title: '综合合成', desc: '置信度加权 → 网关生成结构化 JSON 报告', api: 'synthesize · 首席分析师 Prompt' },
  { key: 'gate', title: '质量门禁', desc: '置信度阈值 + 共识度校验，A/B/C/D 分级', api: 'qualityGate · 可降级重试' },
  { key: 'learn', title: '反馈学习', desc: '意图先验回写 + 专家 metrics 更新', api: 'learn · alliance_intent_priors' }
]

const strategyOptions = [
  { value: 'round_robin', label: '轮询策略' },
  { value: 'least_loaded', label: '最少负载策略' },
  { value: 'performance_based', label: '性能优先策略' },
  { value: 'content_aware', label: '内容感知策略' },
  { value: 'affinity', label: '亲和度策略' }
]

const strategyLabels = {
  round_robin: '轮询', least_loaded: '最少负载', performance_based: '性能优先',
  content_aware: '内容感知', affinity: '亲和度'
}

const strategyDescs = {
  round_robin: '按顺序轮流分配请求到各专家',
  least_loaded: '优先分配给当前负载最轻的专家',
  performance_based: '根据历史绩效选择最优专家',
  content_aware: '根据问题内容智能匹配最合适的专家',
  affinity: '为每个用户/会话固定分配同一专家'
}

const typeLabels = EXPERT_TYPE_SHORT_LABELS

// 会话类型词表不在这里：契约的 SESSION_TYPES + sessionTypeLabel 是唯一权威（§5.38）


const currentStrategyDesc = computed(() => strategyDescs[dispatcherConfig.value] || '')

const kpiCards = computed(() => [
  {
    label: '会话总数', value: sessions.value.length,
    desc: '持久化存储的对话会话', color: 'primary',
    icon: markRaw(ChatDotRound)
  },
  {
    label: '活跃会话',
    value: sessions.value.filter(s => s.status === SESSION_STATUS.ACTIVE).length,
    desc: '当前进行中的会话', color: 'success',
    icon: markRaw(DataAnalysis)
  },
  {
    label: '图谱专家',
    value: graphStats.value?.total_nodes || 0,
    desc: '能力图谱中的专家节点', color: 'info',
    icon: markRaw(User)
  },
  {
    label: '调度总次数',
    value: (dispatcherStatus.value?.total_dispatches ?? dispatcherStatus.value?.dispatcher?.total_dispatches) || 0,
    desc: '历史累计调度次数', color: 'warning',
    icon: markRaw(Refresh)
  },
  {
    label: '熔断器触发',
    value: (dispatcherStatus.value?.circuit_breakers || dispatcherStatus.value?.circuit_breaker?.states || []).filter(s => s.status === BREAKER_STATE.OPEN).length || 0,
    desc: '当前熔断中的专家数', color: 'danger',
    icon: markRaw(DataAnalysis)
  },
  {
    label: '知识沉淀',
    value: (graphStats.value?.total_edges || 0) + ' 条关系',
    desc: '专家协作网络连线', color: 'primary',
    icon: markRaw(Connection)
  }
])

const filteredSessions = computed(() => {
  return sessions.value.filter(s => {
    if (sessionSearch.value) {
      const kw = sessionSearch.value.toLowerCase()
      if (!s.title?.toLowerCase().includes(kw) &&
          !s.messages?.some(m => m.content?.toLowerCase().includes(kw))) return false
    }
    if (sessionFilterStatus.value && s.status !== sessionFilterStatus.value) return false
    if (sessionFilterMode.value && s.sessionType !== sessionFilterMode.value) return false
    return true
  })
})

const successRateColor = computed(() => {
  const rate = (selectedExpert.value?.metrics?.success_rate || 0) * 100
  // el-progress 的 color 最终落到 SVG 的 stroke 呈现属性上，那里读不了 var() ⇒ 按档位现取色值。
  // computed 里调 tokenColor 就自动挂上 themeRevision，换肤会自己重算。
  if (rate >= 90) return tokenColor('--success')
  if (rate >= 70) return tokenColor('--warning')
  return tokenColor('--danger')
})

const confidenceColor = computed(() => {
  const conf = (selectedExpert.value?.metrics?.avg_confidence || 0) * 100
  if (conf >= 80) return tokenColor('--cat-1')
  if (conf >= 60) return tokenColor('--cat-2')
  return tokenColor('--warning')
})

async function loadAll() {
  loading.value = true
  try {
    await Promise.all([
      loadSessions(),
      loadGraphStats(),
      loadDispatcherStatus()
    ])
    ElMessage.success('数据加载完成')
  } catch (e) {
    ElMessage.error('加载失败: ' + e.message)
  } finally {
    loading.value = false
  }
}

// ===== 企业级系统诊断：聚合会话/图谱/调度三维度健康状态 =====
async function runDiagnostic() {
  diagnosticVisible.value = true
  diagnosticLoading.value = true
  try {
    const now = new Date()
    diagnosticTime.value = formatDateTimeLocaleOr(now)
    // 并行采集三组实时状态
    const [sessionRes, graphRes, dispRes] = await Promise.all([
      api.getExpertSessions({}),
      api.getExpertGraphStats(),
      api.getDispatcherStatus()
    ])
    const sessList = Array.isArray(sessionRes) ? sessionRes : (Array.isArray(sessionRes && sessionRes.sessions) ? sessionRes.sessions : [])
    const graph = graphRes || {}
    const disp = dispRes || {}

    // 维度1：会话体系
    const totalSessions = sessList.length
    const activeSessions = sessList.filter((s) => (s.status || SESSION_STATUS.ACTIVE) !== SESSION_STATUS.ARCHIVED).length
    const sessionLevel = totalSessions > 0 ? 'ok' : 'warn'
    const sessionItem = {
      label: '会话体系',
      value: `${totalSessions} 总 / ${activeSessions} 活跃`,
      desc: totalSessions > 0 ? '会话持久化正常，历史会话可检索' : '暂无会话记录，可前往会话中心新建',
      level: sessionLevel,
      levelText: sessionLevel === 'ok' ? '正常' : '待激活'
    }

    // 维度2：专家能力图谱
    const totalNodes = graph.total_nodes || 0
    const density = graph.density || 0
    const graphLevel = totalNodes > 0 ? 'ok' : 'warn'
    const graphItem = {
      label: '能力图谱',
      value: `${totalNodes} 节点 / 密度 ${(density * 100).toFixed(1)}%`,
      desc: totalNodes > 0 ? `已沉淀 ${graph.total_edges || 0} 条协作关系` : '图谱无专家节点，请先注册或重建图谱',
      level: graphLevel,
      levelText: graphLevel === 'ok' ? '正常' : '待构建'
    }

    // 维度3：调度引擎与熔断器
    const cbStates = disp.circuit_breakers || disp.circuit_breaker?.states || []
    const openCbs = cbStates.filter((c) => c.status === BREAKER_STATE.OPEN || c.status === 'half_open')
    const dispatchCount = (disp.recent_dispatches || disp.dispatcher?.recent_dispatches || []).length
    const dispLevel = openCbs.length > 0 ? 'warn' : 'ok'
    const dispItem = {
      label: '调度引擎',
      value: `${cbStates.length} 专家 · 熔断 ${openCbs.length}`,
      desc: openCbs.length > 0
        ? `⚠ ${openCbs.map((c) => c.key).join('、')} 处于熔断/半开状态，需关注`
        : (dispatchCount > 0 ? `最近 ${dispatchCount} 次调度运行正常` : '无熔断异常，最近暂无调度记录'),
      level: dispLevel,
      levelText: dispLevel === 'ok' ? '正常' : '需关注'
    }

    // 汇总
    const levels = [sessionItem, graphItem, dispItem].map((i) => i.level)
    const hasWarn = levels.includes('warn')
    const allOk = levels.every((l) => l === 'ok')
    diagnosticSummaryLevel.value = allOk ? 'ok' : (hasWarn ? 'warn' : 'err')
    diagnosticSummary.value = allOk
      ? '架构维度健康：会话、图谱、调度引擎运行状态良好'
      : '存在需关注项：请查看下列维度详情并采取对应措施'

    diagnosticItems.value = [sessionItem, graphItem, dispItem]
  } catch (e) {
    diagnosticItems.value = [{
      label: '诊断执行',
      value: '失败',
      desc: '无法连接专家服务端点：' + (e.message || '网络异常'),
      level: 'err',
      levelText: '异常'
    }]
    diagnosticSummaryLevel.value = 'err'
    diagnosticSummary.value = '诊断执行失败，请确认后端专家服务已启动'
  } finally {
    diagnosticLoading.value = false
  }
}

async function loadSessions() {
  try {
    const res = await api.getExpertSessions({
      status: sessionFilterStatus.value || undefined,
      session_type: sessionFilterMode.value || undefined
    })
    // 兼容分页信封 {sessions:[...], total, page} 与裸数组两种形态
    sessions.value = Array.isArray(res) ? res : (Array.isArray(res && res.sessions) ? res.sessions : [])
  } catch (e) {
    sessions.value = []
  }
}

async function loadGraphStats() {
  try {
    const res = await api.getExpertGraphStats()
    graphStats.value = res
  } catch (e) {
    // use local data
  }
}

async function loadGraph() {
  try {
    const res = await api.getExpertGraph()
    graphData.value = res
  } catch (e) {
    ElMessage.error('加载图谱失败')
  }
}

async function loadDispatcherStatus() {
  try {
    const res = await api.getDispatcherStatus()
    dispatcherStatus.value = res
  } catch (e) {
    // use local defaults
  }
}

async function updateStrategy() {
  try {
    await api.updateDispatcherConfig({ strategy: dispatcherConfig.value })
    ElMessage.success(`策略已更新为: ${strategyLabels[dispatcherConfig.value]}`)
  } catch (e) {
    ElMessage.error('策略更新失败')
  }
}

async function rebuildGraph() {
  try {
    await api.rebuildExpertGraph()
    await loadGraphStats()
    ElMessage.success('图谱已重建')
  } catch (e) {
    ElMessage.error('重建失败')
  }
}

async function createNewSession() {
  try {
    const title = prompt('请输入会话标题') || '新对话'
    const session = await api.createExpertSession({ title, mode: 'smart' })
    sessions.value.unshift(session)
    ElMessage.success('会话已创建')
  } catch (e) {
    ElMessage.error('创建失败')
  }
}

async function openSession(session) {
  ElMessageBox.alert(
    `<strong>${session.title}</strong><br/>模式: ${sessionTypeLabel(session.sessionType)}<br/>消息数: ${session.messages?.length || 0}<br/>更新时间: ${formatTime(session.updated_at)}`,
    '会话详情',
    { dangerouslyUseHTMLString: true, confirmButtonText: '确定' }
  )
}

function getSessionPreview(session) {
  const lastMsg = session.messages?.filter(m => m.role === MESSAGE_ROLE.USER).pop()
  return lastMsg?.content?.slice(0, 80) || '暂无消息'
}

function sessionStatusColor(status) {
  return status === SESSION_STATUS.ACTIVE ? 'success' : 'info'
}

// 会话状态词表不在这里：模块契约的 sessionStatusLabel 是唯一权威（§5.37）
// 旧副本只认 active/archived 两值，closed 会把英文原样印到界面上

function cbStatusLabel(status) {
  return status === BREAKER_STATE.CLOSED ? '正常' : status === BREAKER_STATE.OPEN ? '熔断中' : '半开'
}

function formatTime(ts) {
  if (!ts) return '-'
  const d = new Date(ts)
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
}

function getTypeColor(type) {
  return expertColor(type)
}

function getNodePosition(id, nodes) {
  const idx = nodes.findIndex(n => n.id === id)
  if (idx < 0) return { x: 10, y: 10 }
  const angle = (idx / nodes.length) * Math.PI * 2 - Math.PI / 2
  const radius = 30 + (idx % 3) * 15
  const cx = 50 + radius * Math.cos(angle)
  const cy = 50 + radius * Math.sin(angle)
  return { x: Math.max(5, Math.min(95, cx)), y: Math.max(5, Math.min(95, cy)) }
}

async function exploreExpert(node) {
  selectedExpert.value = node
  try {
    const res = await api.getExpertGraphCollaborators(node.id, 5)
    selectedCollaborators.value = res?.collaborators || []
  } catch (e) {
    selectedCollaborators.value = []
  }
}

async function loadOptimalTeam() {
  if (!enterpriseForm.value.question) return
  try {
    const res = await api.findOptimalTeam({
      question: enterpriseForm.value.question,
      size: 3
    })
    if (enterpriseResult.value) {
      enterpriseResult.value.optimal_team = res
    } else {
      enterpriseResult.value = { optimal_team: res }
    }
  } catch (e) {
    ElMessage.error('团队推荐失败')
  }
}

async function submitEnterpriseConsult() {
  enterpriseLoading.value = true
  try {
    const res = await api.enterpriseConsult({
      question: enterpriseForm.value.question,
      mode: enterpriseForm.value.mode,
      strategy: enterpriseForm.value.strategy,
      tags: enterpriseForm.value.tags
    })
    enterpriseResult.value = res
    ElMessage.success('协作完成')
  } catch (e) {
    ElMessage.error('协作失败: ' + e.message)
  } finally {
    enterpriseLoading.value = false
  }
}

// ===== 流程编排 tab：加载与渲染 =====
async function loadFlowGraph() {
  flowLoading.value = true
  try {
    const res = await api.getEngineFlowGraph()
    flowGraphData.value = res
    await nextTick()
    renderFlowGraph()
  } catch (e) {
    ElMessage.error('流程图谱加载失败: ' + (e.message || e))
  } finally {
    flowLoading.value = false
  }
}

function buildFlowOption() {
  const data = flowGraphData.value
  if (!data) return null

  // 聚焦视图过滤：pipeline 仅 step+flows_to；capability 仅 cap/eng+委托/降级边
  let nodes = data.nodes || []
  let edges = data.edges || []
  if (flowFocus.value === 'pipeline') {
    nodes = nodes.filter(n => n.type === 'step')
    edges = edges.filter(e => e.type === 'flows_to')
  } else if (flowFocus.value === 'capability') {
    nodes = nodes.filter(n => n.type === 'capability' || n.type === 'engine')
    edges = edges.filter(e => e.type === 'delegates_to' || e.type === 'degrades_to')
  }

  const chartNodes = nodes.map(n => {
    const meta = flowNodeTypes[n.type] || { token: '--text-quaternary', size: 16 }
    return {
      id: n.id,
      name: n.label,
      symbolSize: meta.size,
      category: n.type,
      itemStyle: { color: tokenColor(meta.token) },
      label: { show: n.type !== 'keyword' },
      _desc: n.desc || '',
      _type: n.type
    }
  })

  const chartEdges = edges.map(e => {
    const style = flowEdgeStyles[e.type] || { token: '--text-tertiary', width: 1, type: 'solid' }
    return {
      source: e.source,
      target: e.target,
      value: e.weight,
      lineStyle: {
        color: tokenColor(style.token),
        width: style.width,
        type: style.type,
        opacity: e.type === 'triggers' ? 0.3 : 0.7,
        curveness: 0.1
      }
    }
  })

  return {
    backgroundColor: 'transparent',
    tooltip: {
      backgroundColor: tokenColor('--bg-card'),
      borderColor: tokenColor('--border-normal'),
      textStyle: { color: tokenColor('--text-primary') },
      formatter: (p) => {
        if (p.dataType === 'node') {
          const t = flowNodeTypes[p.data._type]?.label || p.data._type
          return `<b>${p.data.name}</b><br/>类型：${t}${p.data._desc ? '<br/>' + p.data._desc : ''}`
        }
        return `${p.data.source} → ${p.data.target}`
      }
    },
    legend: { show: false },
    animationDuration: 600,
    series: [{
      type: 'graph',
      layout: 'force',
      roam: true,
      draggable: true,
      categories: Object.keys(flowNodeTypes).map(t => ({ name: flowNodeTypes[t].label })),
      data: chartNodes,
      links: chartEdges,
      label: { color: tokenColor('--text-secondary'), fontSize: 11 },
      emphasis: {
        focus: 'adjacency',
        lineStyle: { width: 4, opacity: 0.9 }
      },
      force: {
        repulsion: 200,
        edgeLength: [50, 140],
        gravity: 0.08,
        friction: 0.18
      },
      lineStyle: { curveness: 0.1 }
    }]
  }
}

function renderFlowGraph() {
  const option = buildFlowOption()
  if (!option) return
  const el = flowChart.value
  if (!el) return
  // 流程图谱使用 SVG renderer：canvas 渲染在嵌套/隐藏面板上下文可能不创建画布，
  // SVG 为纯 DOM 渲染，稳定可靠。
  if (!flowInst) {
    // 释放 DOM 上残留的旧实例：echarts 在 dom 记录 _echarts_instance_ 属性，
    // 不清除则 init 复用旧实例（canvas 版）而不创建 svg。
    const existing = echarts.getInstanceByDom(el)
    if (existing) { try { existing.dispose() } catch (e) {} }
    flowInst = echarts.init(el, null, { renderer: 'svg' })
  }
  flowInst.setOption(option, true)
  // v-show 面板布局可能滞后于数据到达：延迟数帧校正尺寸，
  // 容器 0 宽时可能无画布，dispose 重建（最多重试数次）
  let tries = 0
  const ensure = () => {
    if (!flowInst || !el) return
    flowInst.resize()
    const hasSvg = !!el.querySelector('svg')
    const hasWidth = el.offsetWidth > 0
    if ((!hasSvg || !hasWidth) && tries < 6) {
      tries++
      const cur = echarts.getInstanceByDom(el)
      if (cur) { try { cur.dispose() } catch (e) {} }
      flowInst = echarts.init(el, null, { renderer: 'svg' })
      flowInst.setOption(option, true)
      setTimeout(ensure, 120)
    }
  }
  setTimeout(ensure, 60)
}

function resizeFlowChart() {
  flowInst && flowInst.resize()
}

// 换肤后必须重画：画布上的档位是建 option 时现取的色值，不重画就停在旧皮上。
watch(themeRevision, () => {
  if (flowInst) renderFlowGraph()
})

watch(activeTab, async (tab) => {
  if (tab === 'flow') {
    if (!flowGraphData.value) {
      await loadFlowGraph()
    } else {
      await nextTick()
      renderFlowGraph()
    }
  }
})

onMounted(() => {
  loadAll()
  window.addEventListener('resize', resizeFlowChart)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', resizeFlowChart)
  flowInst && flowInst.dispose()
  flowInst = null
})
</script>

<style scoped>
.expert-enterprise {
  padding: 20px;
  height: 100%;
  overflow: auto;
}
.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 22px;
  margin: 0 0 4px;
  background: linear-gradient(135deg, var(--cat-1), var(--cat-6));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.page-subtitle { color: var(--text-muted); font-size: 13px; margin: 0; }
.head-actions { display: flex; gap: 10px; }
.main-tabs { background: transparent; }
.main-row { margin-top: 16px; }
.panel { background: var(--bg-card); border-radius: 10px; box-shadow: 0 1px 3px rgba(0,0,0,0.06); }
.panel-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.panel-head h3 { margin: 0; font-size: 16px; color: var(--text-primary); }
.card-pad { padding: 20px; }
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
  margin-bottom: 16px;
}
.kpi-card {
  background: var(--bg-card);
  border-radius: 12px;
  padding: 18px;
  display: flex;
  gap: 14px;
  align-items: flex-start;
  border: 1px solid var(--border-ghost);
  transition: transform 0.2s, box-shadow 0.2s;
  position: relative;
  overflow: hidden;
}
.kpi-card:hover { transform: translateY(-2px); box-shadow: 0 4px 12px rgba(0,0,0,0.08); }
.kpi-card.primary { border-left: 4px solid var(--cat-1); }
.kpi-card.success { border-left: 4px solid var(--success); }
.kpi-card.info { border-left: 4px solid var(--cat-2); }
.kpi-card.warning { border-left: 4px solid var(--cat-4); }
.kpi-card.danger { border-left: 4px solid var(--danger); }
.kpi-icon { width: 48px; height: 48px; border-radius: 10px; display: flex; align-items: center; justify-content: center; background: var(--bg-tertiary); color: var(--cat-1); }
.kpi-body { flex: 1; }
.kpi-value { font-size: 24px; font-weight: 700; color: var(--text-primary); }
.kpi-label { font-size: 13px; color: var(--text-muted); margin: 2px 0; }
.kpi-desc { font-size: 11px; color: var(--text-muted); }
.kpi-trend { font-size: 12px; font-weight: 600; padding: 2px 8px; border-radius: 10px; }
.kpi-trend.up { color: var(--success); background: var(--success-dim); }
.kpi-trend.down { color: var(--danger); background: var(--danger-dim); }
.graph-stats-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-bottom: 20px; }
.graph-stat { text-align: center; padding: 12px; background: var(--bg-tertiary); border-radius: 8px; }
.stat-val { display: block; font-size: 22px; font-weight: 700; color: var(--cat-1); }
.stat-lbl { font-size: 12px; color: var(--text-muted); }
.communities-section, .type-distribution { margin-bottom: 18px; }
.communities-section h4, .type-distribution h4 { font-size: 14px; margin: 0 0 12px; color: var(--text-secondary); }
.communities-list { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; }
.community-card { padding: 12px; background: var(--bg-tertiary); border-radius: 8px; border: 1px solid var(--border); }
.community-name { font-weight: 600; font-size: 13px; color: var(--text-primary); }
.community-type { font-size: 11px; color: var(--text-muted); margin: 4px 0; }
.community-members { display: flex; flex-wrap: wrap; gap: 4px; }
.more { font-size: 11px; color: var(--text-muted); }
.type-bars { display: flex; flex-direction: column; gap: 6px; }
.type-bar-row { display: flex; align-items: center; gap: 8px; font-size: 12px; }
.type-name { width: 70px; color: var(--text-secondary); }
.type-bar { flex: 1; height: 12px; background: var(--bg-tertiary); border-radius: 6px; overflow: hidden; }
.type-bar-fill { height: 100%; background: linear-gradient(90deg, var(--cat-1), var(--cat-6)); border-radius: 6px; transition: width 0.5s; }
.type-count { width: 24px; text-align: right; color: var(--text-muted); }
.dispatcher-panel .strategy-desc { padding: 12px; background: var(--info-50); border-radius: 8px; color: var(--info); font-size: 13px; margin-bottom: 16px; }
.circuit-breaker-section { margin-bottom: 16px; }
.circuit-breaker-section h4 { font-size: 14px; margin: 0 0 10px; }
.cb-list { display: flex; flex-direction: column; gap: 6px; }
.cb-item { display: flex; justify-content: space-between; padding: 8px 12px; border-radius: 6px; background: var(--bg-tertiary); font-size: 12px; }
.cb-item.closed { border-left: 3px solid var(--success); }
.cb-item.open { border-left: 3px solid var(--danger); background: var(--danger-dim); }
.cb-item.half_open { border-left: 3px solid var(--cat-4); background: var(--warning-50); }
.cb-key { font-weight: 600; }
.cb-status { padding: 1px 6px; border-radius: 4px; font-size: 11px; }
.cb-item.closed .cb-status { background: var(--success-dim); color: var(--success); }
.cb-item.open .cb-status { background: var(--danger-dim); color: var(--danger); }
.cb-item.half_open .cb-status { background: var(--warning-dim); color: var(--warning); }
.cb-empty { padding: 12px; text-align: center; color: var(--text-muted); background: var(--bg-tertiary); border-radius: 8px; font-size: 13px; }
.dispatch-log h4 { font-size: 14px; margin: 0 0 10px; }
.dispatch-list { max-height: 180px; overflow: auto; }
.dispatch-item { display: flex; justify-content: space-between; padding: 6px 8px; font-size: 12px; border-bottom: 1px solid var(--border-ghost); }
.ds-expert { color: var(--text-primary); font-weight: 500; }
.ds-strategy { color: var(--cat-1); }
.ds-time { color: var(--text-muted); }
.session-toolbar { display: flex; gap: 10px; margin-bottom: 16px; flex-wrap: wrap; }
.session-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 14px; }
.session-card {
  background: var(--bg-card); border-radius: 10px; padding: 16px; cursor: pointer;
  transition: transform 0.15s, box-shadow 0.15s; border: 1px solid var(--border-ghost);
}
.session-card:hover { transform: translateY(-2px); box-shadow: 0 4px 12px rgba(0,0,0,0.08); }
.session-header { display: flex; gap: 6px; align-items: center; margin-bottom: 8px; }
.session-updated { margin-left: auto; font-size: 11px; color: var(--text-muted); }
.session-title { font-size: 15px; font-weight: 600; color: var(--text-primary); margin-bottom: 6px; }
.session-preview { font-size: 12px; color: var(--text-muted); margin-bottom: 10px; line-height: 1.5; }
.session-meta { display: flex; justify-content: space-between; align-items: center; font-size: 12px; color: var(--text-muted); }
.session-meta .el-tag { margin-right: 4px; }
.empty-state { padding: 40px; }
.graph-visual { height: 400px; position: relative; background: var(--bg-tertiary); border-radius: 8px; overflow: hidden; }
.graph-legend { display: flex; flex-wrap: wrap; gap: 10px; margin-bottom: 12px; }
.legend-item { display: flex; align-items: center; gap: 4px; font-size: 11px; color: var(--text-secondary); }
.legend-dot { width: 10px; height: 10px; border-radius: 50%; }
.graph-nodes { position: relative; width: 100%; height: 360px; }
.graph-node {
  position: absolute; width: 60px; height: 60px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  color: #fff; cursor: pointer; font-size: 11px; text-align: center;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15); transform: translate(-50%, -50%);
  transition: transform 0.2s;
}
.graph-node:hover { transform: translate(-50%, -50%) scale(1.15); z-index: 10; }
.node-label { pointer-events: none; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 50px; }
.graph-more { position: absolute; bottom: 10px; left: 50%; transform: translateX(-50%); font-size: 11px; color: var(--text-muted); }
.expert-detail { padding: 4px; }
.expert-header { display: flex; gap: 12px; align-items: center; margin-bottom: 16px; }
.expert-header h4 { margin: 0; font-size: 15px; }
.expert-capabilities, .expert-metrics, .expert-collaborators { margin-bottom: 16px; }
.expert-capabilities h5, .expert-metrics h5, .expert-collaborators h5 { font-size: 13px; margin: 0 0 8px; color: var(--text-secondary); }
.metric-row { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.metric-row span { width: 70px; font-size: 12px; color: var(--text-muted); }
.collab-list { display: flex; flex-direction: column; gap: 4px; }
.collab-item { display: flex; justify-content: space-between; padding: 6px 8px; background: var(--bg-tertiary); border-radius: 6px; font-size: 12px; }
.collab-weight { color: var(--cat-1); font-weight: 500; }
.enterprise-form { max-width: 500px; }
.enterprise-result { padding: 4px; }
.result-section { margin-bottom: 16px; padding: 12px; background: var(--bg-tertiary); border-radius: 8px; }
.result-section h4 { margin: 0 0 8px; font-size: 14px; color: var(--text-secondary); }
.result-meta { display: flex; gap: 8px; align-items: center; }
.strategy-tag { font-size: 12px; color: var(--cat-1); background: var(--accent-dim); padding: 2px 8px; border-radius: 4px; }
.response-box { padding: 12px; background: var(--bg-card); border-radius: 6px; border: 1px solid var(--border); font-size: 13px; line-height: 1.7; white-space: pre-wrap; }
.team-list { display: flex; flex-direction: column; gap: 6px; }
.team-member { display: flex; align-items: center; gap: 8px; padding: 6px 8px; background: var(--bg-card); border-radius: 6px; border: 1px solid var(--border); font-size: 12px; }
.team-score { color: var(--cat-1); font-weight: 500; margin-left: auto; }
.context-info { display: flex; gap: 8px; align-items: center; font-size: 13px; color: var(--text-muted); }
/* ===== 流程编排 tab ===== */
.flow-head-actions { display: flex; gap: 8px; align-items: center; }
.flow-stats-row { display: grid; grid-template-columns: repeat(5, 1fr); gap: 10px; margin-bottom: 14px; }
.flow-stat { text-align: center; padding: 10px; background: var(--bg-tertiary); border-radius: 8px; }
.flow-legend {
  display: flex; flex-wrap: wrap; gap: 12px; align-items: center;
  padding: 10px 12px; background: var(--bg-tertiary); border-radius: 8px; margin-bottom: 12px;
  font-size: 11px; color: var(--text-secondary);
}
.legend-edge { display: inline-flex; align-items: center; gap: 4px; margin-left: 8px; color: var(--text-muted); }
.edge-line { display: inline-block; width: 18px; height: 0; border-top: 2px solid var(--text-tertiary); margin: 0 2px 0 6px; }
.edge-line.flows { border-top: 3px solid var(--cat-1); }
.edge-line.triggers { border-top: 1px solid var(--text-tertiary); }
.edge-line.delegates { border-top: 2px dashed var(--cat-2); }
.edge-line.degrades { border-top: 2px dashed var(--danger); }
.flow-chart { height: 420px; width: 100%; }
.flow-side { display: flex; flex-direction: column; }
.pipeline-steps { display: flex; flex-direction: column; gap: 2px; flex: 1; }
.pipeline-stage { display: flex; gap: 12px; padding: 8px 4px; border-radius: 8px; transition: background 0.2s; cursor: default; }
.pipeline-stage:hover { background: var(--bg-tertiary); }
.stage-dot {
  width: 26px; height: 26px; border-radius: 50%; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  background: var(--bg-tertiary); color: var(--text-muted); font-size: 12px; font-weight: 600;
  transition: all 0.2s; cursor: pointer; margin-top: 2px;
}
.stage-dot.active { background: var(--cat-1-fill); color: var(--on-cat-1); box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.15); }
.stage-body { flex: 1; min-width: 0; }
.stage-title { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.stage-desc { font-size: 11px; color: var(--text-muted); line-height: 1.5; margin-top: 2px; }
.stage-detail { margin-top: 6px; }
.stage-api {
  font-size: 11px; color: var(--accent-light); background: var(--accent-dim);
  padding: 4px 8px; border-radius: 6px; display: inline-block;
}
.flow-formula { margin-top: 16px; padding: 12px; background: var(--bg-tertiary); border-radius: 8px; border: 1px solid var(--border); }
.flow-formula h4 { font-size: 13px; margin: 0 0 8px; color: var(--text-secondary); }
.formula-code {
  font-family: 'JetBrains Mono', Consolas, monospace; font-size: 11px;
  color: var(--accent-light); background: var(--accent-dim); padding: 8px; border-radius: 6px;
  overflow-x: auto; white-space: nowrap;
}
.formula-note { font-size: 11px; color: var(--text-muted); margin-top: 6px; line-height: 1.5; }

/* ===== 企业级系统诊断弹窗 ===== */
.diag-loading {
  display: flex; align-items: center; justify-content: center;
  gap: 10px; padding: 40px 0; color: var(--text-muted); font-size: 14px;
}
.diag-body { padding: 4px 0; }
.diag-time { font-size: 12px; color: var(--text-muted); margin-bottom: 14px; }
.diag-items { display: flex; flex-direction: column; gap: 10px; }
.diag-item {
  display: flex; align-items: flex-start; gap: 12px;
  padding: 12px 14px; border-radius: 10px;
  background: var(--bg-tertiary); border: 1px solid rgba(15,23,42,0.06);
}
.diag-icon { flex-shrink: 0; margin-top: 1px; }
.diag-icon.ok { color: var(--cat-3); }
.diag-icon.warn { color: var(--cat-4); }
.diag-icon.err { color: var(--danger); }
.diag-content { flex: 1; min-width: 0; }
.diag-title { display: flex; justify-content: space-between; align-items: center; gap: 8px; font-weight: 600; font-size: 13px; }
.diag-value { font-size: 12px; color: var(--cat-1); font-weight: 700; }
.diag-desc { font-size: 12px; color: var(--text-muted); margin-top: 3px; line-height: 1.5; }
.diag-level {
  flex-shrink: 0; font-size: 11px; font-weight: 600;
  padding: 2px 8px; border-radius: 999px;
}
.diag-level.ok { background: var(--success-50); color: var(--success); }
.diag-level.warn { background: var(--warning-50); color: var(--warning); }
.diag-level.err { background: var(--danger-dim); color: var(--danger); }
.diag-summary {
  display: flex; align-items: center; gap: 8px;
  margin-top: 14px; padding: 12px 14px; border-radius: 10px;
  font-size: 13px; font-weight: 600;
}
.diag-summary.ok { background: var(--success-50); color: var(--success); }
.diag-summary.warn { background: var(--warning-50); color: var(--warning); }
.diag-summary.err { background: var(--danger-dim); color: var(--danger); }
</style>
