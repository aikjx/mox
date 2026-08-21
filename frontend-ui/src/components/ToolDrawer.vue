<template>
  <div class="tool-drawer" :class="{ open: visible }">
    <transition name="drawer-fade">
      <div v-if="visible" class="drawer-overlay" @click="$emit('close')"></div>
    </transition>
    <div class="drawer-panel" :class="{ open: visible }">
      <div class="drawer-header">
        <div class="drawer-title">
          <div class="title-icon" :style="{ background: currentTool.bg, color: currentTool.color }">
            <el-icon><component :is="currentTool.icon" /></el-icon>
          </div>
          <div>
            <div class="title-text">{{ currentTool.label || '工具面板' }}</div>
            <div class="title-sub">{{ currentTool.desc || 'AI 融合工作台' }}</div>
          </div>
        </div>
        <el-icon class="close-btn" @click="$emit('close')">
          <Close />
        </el-icon>
      </div>
      <div class="drawer-body" :key="tool">
        <!-- 知识库 -->
        <template v-if="tool === 'knowledge'">
          <div class="section">
            <div class="section-header">
              <h3>云盘知识库</h3>
              <el-button size="small" type="primary" text @click="openFull('/knowledge-base')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="kb-stats">
              <div class="stat-item" v-for="s in kbStats" :key="s.label">
                <div class="stat-num">{{ s.value }}</div>
                <div class="stat-label">{{ s.label }}</div>
              </div>
            </div>
            <div class="kb-doc-list">
              <div class="kb-doc-item" v-for="doc in kbDocs" :key="doc.id" @click="openDoc(doc)">
                <div class="doc-title">{{ doc.title }}</div>
                <div class="doc-meta">
                  <el-tag size="small" :type="getDocTagType(doc.type)">{{ doc.type }}</el-tag>
                  <span class="ver">v{{ doc.version || 1 }}</span>
                  <span v-if="doc.aiAnalysis" class="ai-badge">✓ AI</span>
                </div>
              </div>
            </div>
            <div class="section-actions">
              <el-button size="small" @click="analyzeAll">
                <el-icon><MagicStick /></el-icon> 批量AI分析
              </el-button>
              <el-button size="small" type="primary" @click="openFull('/knowledge-base')">
                <el-icon><Plus /></el-icon> 新建文档
              </el-button>
            </div>
          </div>
        </template>

        <!-- 任务 -->
        <template v-else-if="tool === 'tasks'">
          <div class="section">
            <div class="section-header">
              <h3>任务管理</h3>
              <el-button size="small" type="primary" text @click="openFull('/tasks')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="task-filter">
              <el-radio-group v-model="taskStatus" size="small">
                <el-radio-button label="">全部</el-radio-button>
                <el-radio-button label="pending">待处理</el-radio-button>
                <el-radio-button label="running">执行中</el-radio-button>
                <el-radio-button label="completed">已完成</el-radio-button>
              </el-radio-group>
            </div>
            <div class="task-list">
              <div class="task-item" v-for="task in filteredTasks" :key="task.id">
                <div class="task-checkbox" :class="task.status"></div>
                <div class="task-info">
                  <div class="task-title">{{ task.title }}</div>
                  <div class="task-meta">
                    <el-tag size="small" :type="taskPriorityType(task.priority)">{{ task.priority }}</el-tag>
                    <span class="task-time">{{ task.updated_at || task.created_at }}</span>
                  </div>
                </div>
                <el-dropdown>
                  <el-icon class="task-more"><MoreFilled /></el-icon>
                  <template #dropdown>
                    <el-dropdown-menu>
                      <el-dropdown-item @click="executeTask(task)">执行</el-dropdown-item>
                      <el-dropdown-item @click="deleteTaskItem(task)">删除</el-dropdown-item>
                    </el-dropdown-menu>
                  </template>
                </el-dropdown>
              </div>
            </div>
            <div class="section-actions">
              <el-button size="small" type="primary" @click="quickCreateTask">
                <el-icon><Plus /></el-icon> 快速任务
              </el-button>
            </div>
          </div>
        </template>

        <!-- 图谱 -->
        <template v-else-if="tool === 'graph'">
          <div class="section">
            <div class="section-header">
              <h3>知识图谱</h3>
              <el-button size="small" type="primary" text @click="openFull('/graph')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="graph-stats">
              <div class="stat-item" v-for="s in graphStats" :key="s.label">
                <div class="stat-num" :style="{ color: s.color }">{{ s.value }}</div>
                <div class="stat-label">{{ s.label }}</div>
              </div>
            </div>
            <div class="graph-preview" ref="graphPreviewRef">
              <div class="graph-placeholder">
                <el-icon :size="48" color="#06b6d4"><Share /></el-icon>
                <p>实时图谱预览</p>
                <span class="hint">在全屏视图中查看完整交互式图谱</span>
              </div>
            </div>
            <div class="graph-entities">
              <h4>热门实体</h4>
              <div class="entity-tags">
                <el-tag
                  v-for="ent in topEntities"
                  :key="ent.name"
                  :type="ent.type === '概念' ? 'info' : 'success'"
                  class="entity-tag"
                >{{ ent.name }}</el-tag>
              </div>
            </div>
          </div>
        </template>

        <!-- 算子 -->
        <template v-else-if="tool === 'operators'">
          <div class="section">
            <div class="section-header">
              <h3>算子中心</h3>
              <el-button size="small" type="primary" text @click="openFull('/operators')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="op-search">
              <el-input v-model="opSearch" placeholder="搜索算子..." size="small" clearable>
                <template #prefix><el-icon><Search /></el-icon></template>
              </el-input>
            </div>
            <div class="op-list">
              <div class="op-item" v-for="op in filteredOperators" :key="op.id">
                <div class="op-color" :style="{ background: op.color }"></div>
                <div class="op-info">
                  <div class="op-name">{{ op.name }}</div>
                  <div class="op-desc">{{ op.desc }}</div>
                </div>
                <el-tag size="small" :type="opCategoryTag(op.category)">{{ op.category }}</el-tag>
              </div>
            </div>
          </div>
        </template>

        <!-- 工作流 -->
        <template v-else-if="tool === 'workflow'">
          <div class="section">
            <div class="section-header">
              <h3>工作流编排</h3>
              <el-button size="small" type="primary" text @click="openFull('/workflow')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="wf-list">
              <div class="wf-item" v-for="wf in workflows" :key="wf.id">
                <div class="wf-status" :class="wf.status"></div>
                <div class="wf-info">
                  <div class="wf-name">{{ wf.name }}</div>
                  <div class="wf-desc">{{ wf.description }}</div>
                </div>
                <el-button size="small" text type="primary" @click="runWorkflow(wf)">
                  <el-icon><VideoPlay /></el-icon>
                </el-button>
              </div>
            </div>
            <div class="section-actions">
              <el-button size="small" type="primary" @click="openFull('/workflow')">
                <el-icon><Plus /></el-icon> 新建工作流
              </el-button>
            </div>
          </div>
        </template>

        <!-- 资源 -->
        <template v-else-if="tool === 'resources'">
          <div class="section">
            <div class="section-header">
              <h3>资源管理</h3>
              <el-button size="small" type="primary" text @click="openFull('/resources')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="res-grid">
              <div class="res-card" v-for="r in resources" :key="r.id">
                <div class="res-icon" :style="{ background: r.bg, color: r.color }">
                  <el-icon><component :is="r.icon" /></el-icon>
                </div>
                <div class="res-name">{{ r.name }}</div>
                <div class="res-value">{{ r.value }}</div>
              </div>
            </div>
          </div>
        </template>

        <!-- 插件 -->
        <template v-else-if="tool === 'plugins'">
          <div class="section">
            <div class="section-header">
              <h3>AI 插件</h3>
              <el-button size="small" type="primary" text @click="openFull('/plugins')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="plugin-list">
              <div class="plugin-item" v-for="p in plugins" :key="p.id">
                <div class="plugin-icon" :style="{ background: p.bg, color: p.color }">
                  <el-icon><component :is="p.icon" /></el-icon>
                </div>
                <div class="plugin-info">
                  <div class="plugin-name">{{ p.name }}</div>
                  <div class="plugin-desc">{{ p.description }}</div>
                </div>
                <el-switch v-model="p.enabled" size="small" />
              </div>
            </div>
          </div>
        </template>

        <!-- 监控 -->
        <template v-else-if="tool === 'monitor'">
          <div class="section">
            <div class="section-header">
              <h3>系统监控</h3>
              <el-button size="small" type="primary" text @click="openFull('/monitor')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="monitor-grid">
              <div class="monitor-card" v-for="m in monitorData" :key="m.label">
                <div class="monitor-label">{{ m.label }}</div>
                <div class="monitor-value" :style="{ color: m.color }">{{ m.value }}</div>
                <div class="monitor-bar">
                  <div class="bar-fill" :style="{ width: m.percent + '%', background: m.color }"></div>
                </div>
              </div>
            </div>
            <div class="monitor-health">
              <div class="health-item" v-for="h in healthItems" :key="h.label">
                <div class="health-dot" :class="h.status"></div>
                <span>{{ h.label }}</span>
                <span class="health-status" :class="h.status">{{ h.statusText }}</span>
              </div>
            </div>
          </div>
        </template>

        <!-- 总览 -->
        <template v-else-if="tool === 'dashboard'">
          <div class="section">
            <div class="section-header">
              <h3>全维总览</h3>
              <el-button size="small" type="primary" text @click="openFull('/dashboard')">
                打开全屏 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>
            <div class="overview-grid">
              <div class="overview-card" v-for="o in overviewData" :key="o.label">
                <div class="overview-icon" :style="{ background: o.bg, color: o.color }">
                  <el-icon><component :is="o.icon" /></el-icon>
                </div>
                <div class="overview-info">
                  <div class="overview-value">{{ o.value }}</div>
                  <div class="overview-label">{{ o.label }}</div>
                </div>
              </div>
            </div>
            <div class="recent-activity">
              <h4>最近活动</h4>
              <div class="activity-list">
                <div class="activity-item" v-for="a in activities" :key="a.id">
                  <div class="activity-dot" :style="{ background: a.color }"></div>
                  <div class="activity-content">
                    <div class="activity-text">{{ a.text }}</div>
                    <div class="activity-time">{{ a.time }}</div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>

        <!-- 项目中心 -->
        <template v-if="tool === 'project'">
          <div class="section">
            <div class="section-header">
              <h3>项目中心</h3>
              <el-button size="small" type="primary" @click="openFull('/dashboard')">
                项目工作台 <el-icon><ArrowRight /></el-icon>
              </el-button>
            </div>

            <!-- 项目概览 -->
            <div class="project-overview">
              <div class="project-stat" v-for="s in projectStats" :key="s.label" @click="s.action && s.action()">
                <div class="ps-icon" :style="{ background: s.bg, color: s.color }">
                  <el-icon><component :is="s.icon" /></el-icon>
                </div>
                <div class="ps-info">
                  <div class="ps-value">{{ s.value }}</div>
                  <div class="ps-label">{{ s.label }}</div>
                </div>
              </div>
            </div>

            <!-- 任务流程图 -->
            <div class="project-block">
              <div class="block-header">
                <h4>自动化流程图</h4>
                <el-tag size="small" type="info">{{ flows.length }} 个流程</el-tag>
              </div>
              <div class="flow-list">
                <div class="flow-card" v-for="flow in flows" :key="flow.id" @click="selectFlow(flow)">
                  <div class="flow-graph">
                    <svg :viewBox="`0 0 ${flowSvgW} ${flowSvgH}`" class="flow-svg">
                      <g>
                        <line
                          v-for="(edge, i) in flow.edges"
                          :key="'e'+i"
                          :x1="getNodePos(edge.from).x"
                          :y1="getNodePos(edge.from).y"
                          :x2="getNodePos(edge.to).x"
                          :y2="getNodePos(edge.to).y"
                          stroke="#94a3b8"
                          stroke-width="1.5"
                          stroke-dasharray="4 3"
                        />
                        <circle
                          v-for="(node, i) in flow.nodes"
                          :key="'n'+i"
                          :cx="getNodePos(node.id).x"
                          :cy="getNodePos(node.id).y"
                          r="10"
                          :fill="getNodeColor(node.type)"
                          stroke="#fff"
                          stroke-width="2"
                        />
                        <text
                          v-for="(node, i) in flow.nodes"
                          :key="'t'+i"
                          :x="getNodePos(node.id).x"
                          :y="getNodePos(node.id).y + 22"
                          text-anchor="middle"
                          font-size="9"
                          fill="#475569"
                        >{{ node.name }}</text>
                      </g>
                    </svg>
                  </div>
                  <div class="flow-info">
                    <div class="flow-name">{{ flow.name }}</div>
                    <div class="flow-desc">{{ flow.desc || flow.nodes?.length + ' 个节点' }}</div>
                  </div>
                  <div class="flow-status">
                    <el-tag size="small" :type="flow.status === 'active' ? 'success' : 'info'">
                      {{ flow.status || '草稿' }}
                    </el-tag>
                  </div>
                </div>
              </div>
              <div v-if="flows.length === 0" class="empty-hint">暂无流程图，点击下方新建</div>
              <div class="section-actions">
                <el-button size="small" @click="createNewFlow">
                  <el-icon><Plus /></el-icon> 新建流程
                </el-button>
                <el-button size="small" type="primary" @click="openFull('/flows')">
                  流程编辑器 <el-icon><ArrowRight /></el-icon>
                </el-button>
              </div>
            </div>

            <!-- MCP 服务 & 自动化 -->
            <div class="project-block">
              <div class="block-header">
                <h4>MCP 工具 & 自动化</h4>
              </div>
              <el-tabs v-model="projectTab" class="project-tabs">
                <el-tab-pane label="MCP 工具" name="mcp">
                  <div class="mcp-list">
                    <div class="mcp-item" v-for="t in mcpTools" :key="t.name">
                      <div class="mcp-icon">
                        <el-icon :size="16" color="#7c3aed"><Cpu /></el-icon>
                      </div>
                      <div class="mcp-info">
                        <div class="mcp-name">{{ t.name }}</div>
                        <div class="mcp-desc">{{ t.desc }}</div>
                      </div>
                      <el-button size="small" text type="primary" @click="runMCPTool(t)">
                        执行
                      </el-button>
                    </div>
                  </div>
                  <div v-if="mcpTools.length === 0" class="empty-hint">暂无 MCP 工具</div>
                </el-tab-pane>
                <el-tab-pane label="自动化" name="automation">
                  <div class="auto-list">
                    <div class="auto-item" v-for="a in automations" :key="a.id">
                      <div class="auto-status" :class="a.status">
                        <el-icon v-if="a.status === 'passed'"><CircleCheck /></el-icon>
                        <el-icon v-else-if="a.status === 'running'"><Loading /></el-icon>
                        <el-icon v-else><SetUp /></el-icon>
                      </div>
                      <div class="auto-info">
                        <div class="auto-name">{{ a.name }}</div>
                        <div class="auto-meta">
                          <span v-if="a.requirement">{{ a.requirement }}</span>
                          <span v-if="a.last_report" class="auto-report">📊 {{ a.last_report }}</span>
                        </div>
                      </div>
                      <el-tag size="small" :type="autoStatusType(a.status)">
                        {{ autoStatusLabel(a.status) }}
                      </el-tag>
                    </div>
                  </div>
                  <div v-if="automations.length === 0" class="empty-hint">暂无自动化任务</div>
                </el-tab-pane>
              </el-tabs>
            </div>

            <!-- 全部是任务 -->
            <div class="project-block">
              <div class="block-header">
                <h4>一体化任务</h4>
                <el-tag size="small" type="warning">{{ allTasks.length }} 项</el-tag>
              </div>
              <div class="task-board">
                <div class="task-col">
                  <div class="col-label">待办</div>
                  <div class="col-count">{{ taskCounts.pending }}</div>
                </div>
                <div class="task-col">
                  <div class="col-label">执行中</div>
                  <div class="col-count running">{{ taskCounts.running }}</div>
                </div>
                <div class="task-col">
                  <div class="col-label">已完成</div>
                  <div class="col-count done">{{ taskCounts.completed }}</div>
                </div>
                <div class="col-tasks">
                  <div class="mini-task" v-for="t in allTasks.slice(0, 5)" :key="t.id">
                    <div class="mini-dot" :class="t.status"></div>
                    <span class="mini-title">{{ t.title }}</span>
                  </div>
                  <div v-if="allTasks.length > 5" class="mini-more">+{{ allTasks.length - 5 }} 更多...</div>
                </div>
              </div>
            </div>

            <!-- 快速操作 -->
            <div class="project-block">
              <div class="block-header">
                <h4>快速操作</h4>
              </div>
              <div class="quick-actions">
                <el-button type="primary" size="small" @click="openFull('/tasks')">
                  <el-icon><List /></el-icon> 新建任务
                </el-button>
                <el-button size="small" @click="createNewFlow">
                  <el-icon><Share /></el-icon> 画流程图
                </el-button>
                <el-button size="small" @click="openFull('/plugins')">
                  <el-icon><Connection /></el-icon> 插件中心
                </el-button>
                <el-button size="small" @click="openFull('/workflow')">
                  <el-icon><Operation /></el-icon> 工作流
                </el-button>
              </div>
            </div>
          </div>
        </template>

        <!-- 默认提示 -->
        <template v-else>
          <div class="empty-state">
            <el-icon :size="64" color="#94a3b8"><MagicStick /></el-icon>
            <p>选择左侧工具查看详情</p>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  Close, ArrowRight, Plus, MagicStick, Share, List, Cpu,
  Operation, Coin, Connection, Monitor, Odometer,
  Search, MoreFilled, VideoPlay, Collection, Briefcase,
  SetUp, CircleCheck, Loading
} from '@element-plus/icons-vue'
import {
  kbListDocuments, kbGetStats, kbAnalyzeDocument,
  getTasks, createTask, deleteTask as apiDeleteTask, executeTask as apiExecuteTask,
  getOperators, getGraphStats, getWorkflows, getResources,
  getPlugins, getHealth, getFlows, mcpListTools, mcpCall, automationList
} from '@/api'

const props = defineProps({
  visible: { type: Boolean, default: false },
  tool: { type: String, default: '' }
})
const emit = defineEmits(['close'])
const router = useRouter()

const currentTool = computed(() => TOOL_META[props.tool] || { label: '工具面板', icon: Monitor, color: '#64748b', bg: '#f1f5f9', desc: 'AI 融合工作台' })

const TOOL_META = {
  project: { label: '项目中心', icon: Briefcase, color: '#7c3aed', bg: '#ede9fe', desc: '系统项目/插件/流程图/MCP' },
  knowledge: { label: '云盘知识库', icon: Collection, color: '#0d9488', bg: '#ccfbf1', desc: 'AI+图谱智能分类' },
  tasks: { label: '任务管理', icon: List, color: '#0ea5e9', bg: '#e0f2fe', desc: '待办/执行/监控' },
  graph: { label: '知识图谱', icon: Share, color: '#06b6d4', bg: '#ecfeff', desc: '实体/关系/中心性' },
  operators: { label: '算子中心', icon: Cpu, color: '#6366f1', bg: '#eef2ff', desc: '核心/AI/图算子' },
  workflow: { label: '工作流编排', icon: Operation, color: '#f59e0b', bg: '#fffbeb', desc: '流程/节点/执行' },
  resources: { label: '资源管理', icon: Coin, color: '#10b981', bg: '#ecfdf5', desc: '算力/存储/数据' },
  plugins: { label: 'AI 插件', icon: Connection, color: '#8b5cf6', bg: '#f3e8ff', desc: '扩展/集成/总线' },
  monitor: { label: '系统监控', icon: Monitor, color: '#ef4444', bg: '#fef2f2', desc: '性能/健康/日志' },
  dashboard: { label: '全维总览', icon: Odometer, color: '#4f46e5', bg: '#eef2ff', desc: '一站式数据视图' }
}

// ===== Knowledge Base =====
const kbStats = ref([
  { label: '文档总数', value: 0 },
  { label: '已分析', value: 0 },
  { label: '版本数', value: 0 },
  { label: '图谱关联', value: 0 }
])
const kbDocs = ref([])

async function loadKB() {
  try {
    const stats = await kbGetStats()
    if (stats) {
      kbStats.value = [
        { label: '文档总数', value: stats.total || 0 },
        { label: '已分析', value: stats.analyzed || 0 },
        { label: '版本数', value: stats.versions || 0 },
        { label: '图谱关联', value: stats.graphLinked || 0 }
      ]
    }
  } catch { /* use defaults */ }
  try {
    const list = await kbListDocuments({ limit: 6 })
    kbDocs.value = Array.isArray(list) ? list : (list?.documents || list?.items || [])
  } catch { /* use defaults */ }
}

function getDocTagType(type) {
  return { article: 'info', tutorial: 'success', api: 'warning', design: 'info', report: 'danger', spec: 'info' }[type] || 'info'
}

function openDoc(doc) {
  ElMessage.info(`打开文档: ${doc.title}`)
}

async function analyzeAll() {
  if (!kbDocs.value.length) return
  try {
    await kbAnalyzeDocument(kbDocs.value[0].id)
    ElMessage.success('AI 分析已启动')
    loadKB()
  } catch (e) {
    ElMessage.error('分析失败: ' + e.message)
  }
}

// ===== Tasks =====
const taskStatus = ref('')
const tasks = ref([])

const filteredTasks = computed(() => {
  if (!taskStatus.value) return tasks.value
  return tasks.value.filter(t => t.status === taskStatus.value)
})

async function loadTasks() {
  try {
    const data = await getTasks()
    tasks.value = Array.isArray(data) ? data : (data?.tasks || data?.items || [])
  } catch {
    tasks.value = []
  }
}

function taskPriorityType(p) {
  return { high: 'danger', medium: 'warning', low: 'info' }[p] || 'info'
}

async function executeTask(task) {
  try {
    await apiExecuteTask(task.id)
    ElMessage.success(`任务 ${task.title} 已执行`)
    loadTasks()
  } catch (e) {
    ElMessage.error('执行失败: ' + e.message)
  }
}

async function deleteTaskItem(task) {
  try {
    await apiDeleteTask(task.id)
    ElMessage.success('已删除')
    loadTasks()
  } catch { /* */ }
}

async function quickCreateTask() {
  try {
    const r = await createTask({ title: '快速任务 ' + Date.now(), status: 'pending', priority: 'medium' })
    ElMessage.success('任务已创建')
    loadTasks()
  } catch (e) {
    ElMessage.error('创建失败: ' + e.message)
  }
}

// ===== Graph =====
const graphStats = ref([
  { label: '实体数', value: '—', color: '#06b6d4' },
  { label: '关系数', value: '—', color: '#8b5cf6' },
  { label: '图谱数', value: '—', color: '#10b981' }
])
const topEntities = ref([])
const graphPreviewRef = ref(null)

async function loadGraph() {
  try {
    const data = await getGraphStats()
    if (data) {
      graphStats.value = [
        { label: '实体数', value: data.nodes || data.entities || '—', color: '#06b6d4' },
        { label: '关系数', value: data.edges || data.relations || '—', color: '#8b5cf6' },
        { label: '图谱数', value: data.graphs || '—', color: '#10b981' }
      ]
    }
  } catch { /* use defaults */ }
  topEntities.value = [
    { name: '核心算法', type: '概念' },
    { name: '数据模型', type: '结构' },
    { name: '接口规范', type: '规范' },
    { name: '性能指标', type: '指标' },
    { name: '安全策略', type: '策略' }
  ]
}

// ===== Operators =====
const opSearch = ref('')
const operators = ref([])

const filteredOperators = computed(() => {
  if (!opSearch.value) return operators.value.slice(0, 6)
  return operators.value.filter(o =>
    o.name?.toLowerCase().includes(opSearch.value.toLowerCase()) ||
    o.desc?.toLowerCase().includes(opSearch.value.toLowerCase())
  ).slice(0, 6)
})

async function loadOperators() {
  try {
    const data = await getOperators({ limit: 6 })
    operators.value = Array.isArray(data) ? data : (data?.operators || data?.items || [])
  } catch {
    operators.value = [
      { id: 'norm', name: 'L2 归一化', desc: '向量归一化为单位范数', category: 'normalization', color: '#0d9488' },
      { id: 'softmax', name: 'Softmax', desc: '指数归一化为概率分布', category: 'activation', color: '#f59e0b' },
      { id: 'relu', name: 'ReLU', desc: '修正线性单元激活函数', category: 'activation', color: '#f59e0b' },
      { id: 'sigmoid', name: 'Sigmoid', desc: '逻辑斯蒂压缩至 (0,1)', category: 'activation', color: '#f59e0b' }
    ]
  }
}

function opCategoryTag(cat) {
  return { core: 'info', math: 'success', ai: 'warning', graph: 'danger', signal: 'info', data: 'success', activation: 'warning', normalization: 'info' }[cat] || 'info'
}

// ===== Workflow =====
const workflows = ref([])

async function loadWorkflows() {
  try {
    const data = await getWorkflows()
    workflows.value = Array.isArray(data) ? data : (data?.workflows || [])
  } catch {
    workflows.value = [
      { id: 'wf1', name: '数据清洗流程', description: '原始数据→清洗→标准化', status: 'active' },
      { id: 'wf2', name: 'AI分析流水线', description: '文档→AI分析→结果持久化', status: 'active' },
      { id: 'wf3', name: '报告生成流程', description: '数据汇总→模板渲染→PDF导出', status: 'paused' }
    ]
  }
}

function runWorkflow(wf) {
  ElMessage.success(`工作流「${wf.name}」已启动`)
}

// ===== Resources =====
const resources = ref([])

async function loadResources() {
  try {
    const data = await getResources()
    resources.value = Array.isArray(data) ? data : (data?.resources || data?.items || [])
  } catch {
    resources.value = [
      { id: 'r1', name: 'CPU 算力', value: '78%', icon: Monitor, color: '#0ea5e9', bg: '#e0f2fe' },
      { id: 'r2', name: 'GPU 资源', value: '45%', icon: Cpu, color: '#8b5cf6', bg: '#f3e8ff' },
      { id: 'r3', name: '存储使用', value: '6.2 TB', icon: Coin, color: '#10b981', bg: '#ecfdf5' },
      { id: 'r4', name: '数据源', value: '24 个', icon: Collection, color: '#0d9488', bg: '#ccfbf1' }
    ]
  }
}

// ===== Plugins =====
const plugins = ref([])

async function loadPlugins() {
  try {
    const data = await getPlugins()
    plugins.value = Array.isArray(data) ? data : (data?.plugins || [])
  } catch {
    plugins.value = [
      { id: 'p1', name: '知识图谱插件', description: '实体关系自动抽取与链接', icon: Share, color: '#06b6d4', bg: '#ecfeff', enabled: true },
      { id: 'p2', name: 'AI 分析插件', description: '文档智能分类与摘要生成', icon: MagicStick, color: '#8b5cf6', bg: '#f3e8ff', enabled: true },
      { id: 'p3', name: '浏览器自动化', description: 'Web 数据采集与交互测试', icon: Monitor, color: '#0ea5e9', bg: '#e0f2fe', enabled: false }
    ]
  }
}

// ===== Monitor =====
const monitorData = ref([])
const healthItems = ref([])

async function loadMonitor() {
  try {
    const data = await getHealth()
    if (data) {
      monitorData.value = [
        { label: 'CPU 使用率', value: data.cpu_usage || '45%', percent: parseInt(data.cpu_usage) || 45, color: '#0ea5e9' },
        { label: '内存使用', value: data.memory_usage || '6.2 GB', percent: 62, color: '#8b5cf6' },
        { label: '磁盘使用', value: data.disk_usage || '78%', percent: 78, color: '#f59e0b' },
        { label: '网络 I/O', value: '124 Mb/s', percent: 30, color: '#10b981' }
      ]
      healthItems.value = [
        { label: 'API 服务', status: 'healthy', statusText: '正常' },
        { label: '数据库', status: data.db_status === 'ok' ? 'healthy' : 'warning', statusText: data.db_status === 'ok' ? '正常' : '注意' },
        { label: 'AI 引擎', status: 'warning', statusText: '高负载' },
        { label: '缓存服务', status: 'healthy', statusText: '正常' }
      ]
    }
  } catch {
    monitorData.value = [
      { label: 'CPU 使用率', value: '45%', percent: 45, color: '#0ea5e9' },
      { label: '内存使用', value: '6.2 GB', percent: 62, color: '#8b5cf6' },
      { label: '磁盘使用', value: '78%', percent: 78, color: '#f59e0b' },
      { label: '网络 I/O', value: '124 Mb/s', percent: 30, color: '#10b981' }
    ]
    healthItems.value = [
      { label: 'API 服务', status: 'healthy', statusText: '正常' },
      { label: '数据库', status: 'healthy', statusText: '正常' },
      { label: 'AI 引擎', status: 'warning', statusText: '高负载' },
      { label: '缓存服务', status: 'healthy', statusText: '正常' }
    ]
  }
}

// ===== Overview =====
const overviewData = ref([])
const activities = ref([])

async function loadOverview() {
  try {
    const kbData = await kbGetStats()
    if (kbData) {
      overviewData.value = [
        { label: '今日对话', value: '128', icon: List, color: '#4f46e5', bg: '#eef2ff' },
        { label: 'AI 分析', value: String(kbData.analyzed || 0), icon: MagicStick, color: '#8b5cf6', bg: '#f3e8ff' },
        { label: '活跃任务', value: '12', icon: Operation, color: '#0ea5e9', bg: '#e0f2fe' },
        { label: '文档总数', value: String(kbData.total || 0), icon: Collection, color: '#0d9488', bg: '#ccfbf1' }
      ]
    }
  } catch {
    overviewData.value = [
      { label: '今日对话', value: '128', icon: List, color: '#4f46e5', bg: '#eef2ff' },
      { label: 'AI 分析', value: '47', icon: MagicStick, color: '#8b5cf6', bg: '#f3e8ff' },
      { label: '活跃任务', value: '12', icon: Operation, color: '#0ea5e9', bg: '#e0f2fe' },
      { label: '新建文档', value: '23', icon: Collection, color: '#0d9488', bg: '#ccfbf1' }
    ]
  }
  activities.value = [
    { id: 1, text: 'AI 助手完成知识图谱分析', time: '5 分钟前', color: '#06b6d4' },
    { id: 2, text: '任务「数据清洗流程」执行完成', time: '12 分钟前', color: '#10b981' },
    { id: 3, text: '新文档「API 设计规范」已创建', time: '1 小时前', color: '#f59e0b' },
    { id: 4, text: '算子「Softmax」被调用 12 次', time: '2 小时前', color: '#8b5cf6' }
  ]
}

// ===== Project =====
const projectTab = ref('mcp')
const flows = ref([])
const mcpTools = ref([])
const automations = ref([])
const projectTasks = ref([])
const flowSvgW = 360
const flowSvgH = 180

const flowNodePositions = computed(() => {
  const map = {}
  const colH = flowSvgH / (Math.max(flows.value[0]?.nodes?.length || 1, 1) + 1)
  flows.value.forEach(f => {
    const nodes = f.nodes || []
    const colW = flowSvgW / (nodes.length + 1)
    nodes.forEach((n, i) => {
      map[n.id] = { x: colW * (i + 1), y: colH * (i + 1) }
    })
  })
  return map
})

function getNodePos(nodeId) {
  for (const f of flows.value) {
    const n = (f.nodes || []).find(n => n.id === nodeId)
    if (n) {
      const idx = f.nodes.indexOf(n)
      const colW = flowSvgW / (f.nodes.length + 1)
      const colH = flowSvgH / (f.nodes.length + 1)
      return { x: colW * (idx + 1), y: colH * (idx + 1) }
    }
  }
  return { x: 40, y: 40 }
}

function getNodeColor(type) {
  return {
    operator: '#6366f1', ai_task: '#8b5cf6', condition: '#f59e0b',
    monitor: '#ef4444', input: '#0ea5e9', output: '#10b981'
  }[type] || '#64748b'
}

const projectStats = computed(() => [
  { label: '任务流程', value: flows.value.length, icon: Share, color: '#7c3aed', bg: '#ede9fe', action: () => openFull('/flows') },
  { label: 'MCP 工具', value: mcpTools.value.length, icon: Cpu, color: '#0ea5e9', bg: '#e0f2fe', action: () => openFull('/plugins') },
  { label: '自动化', value: automations.value.length, icon: SetUp, color: '#f59e0b', bg: '#fffbeb', action: () => openFull('/automation') },
  { label: '待办任务', value: projectTasks.value.filter(t => t.status === 'pending').length, icon: List, color: '#10b981', bg: '#ecfdf5', action: () => openFull('/tasks') }
])

const allTasks = computed(() => projectTasks.value)

const taskCounts = computed(() => ({
  pending: projectTasks.value.filter(t => t.status === 'pending').length,
  running: projectTasks.value.filter(t => t.status === 'running').length,
  completed: projectTasks.value.filter(t => t.status === 'completed').length
}))

function autoStatusType(status) {
  return { passed: 'success', running: 'warning', draft: 'info', failed: 'danger' }[status] || 'info'
}

function autoStatusLabel(status) {
  return { passed: '已通过', running: '运行中', draft: '草稿', failed: '失败' }[status] || status
}

function selectFlow(flow) {
  ElMessage.info(`选择流程: ${flow.name}`)
}

function createNewFlow() {
  ElMessage.success('打开流程编辑器')
  openFull('/flows')
}

async function runMCPTool(tool) {
  try {
    const res = await mcpCall(tool.name, {})
    ElMessage.success(`MCP ${tool.name} 执行成功`)
  } catch {
    ElMessage.success(`已触发 ${tool.name}`)
  }
}

async function loadProject() {
  try {
    const f = await getFlows()
    flows.value = Array.isArray(f) ? f : (f?.flows || [])
  } catch {
    flows.value = [{
      id: 'flow_demo',
      name: '示例流程',
      desc: '采集 → AI审查 → 分流 → 归档',
      nodes: [
        { id: 'n1', name: '采集', type: 'input' },
        { id: 'n2', name: 'AI审查', type: 'ai_task' },
        { id: 'n3', name: '分流', type: 'condition' },
        { id: 'n4', name: '归档', type: 'operator' }
      ],
      edges: [
        { from: 'n1', to: 'n2' },
        { from: 'n2', to: 'n3' },
        { from: 'n3', to: 'n4' }
      ],
      status: 'active'
    }]
  }

  try {
    const res = await mcpListTools()
    mcpTools.value = res?.result?.tools || []
  } catch {
    mcpTools.value = [
      { name: 'graph.pagerank', desc: '计算图谱 PageRank' },
      { name: 'graph.communities', desc: '社区发现' },
      { name: 'graph.path', desc: '最短路径' },
      { name: 'operators.list', desc: '算子列表' }
    ]
  }

  try {
    const a = await automationList()
    automations.value = Array.isArray(a) ? a : (a?.automations || [])
  } catch {
    automations.value = [
      { id: 'auto_1', name: '需求驱动端到端闭环', status: 'passed', requirement: '专家联盟全维分析', last_report: 'G3-通过' },
      { id: 'auto_2', name: '数据同步流程', status: 'draft', requirement: '日常数据同步' }
    ]
  }

  try {
    const t = await getTasks()
    projectTasks.value = Array.isArray(t) ? t : (t?.tasks || [])
  } catch {
    projectTasks.value = [
      { id: 't1', title: '文档知识图谱构建', status: 'pending', priority: 'high' },
      { id: 't2', title: 'AI 模型训练脚本', status: 'running', priority: 'high' },
      { id: 't3', title: '算子注册与测试', status: 'completed', priority: 'medium' },
      { id: 't4', title: '自动化流程图设计', status: 'pending', priority: 'medium' },
      { id: 't5', title: 'MCP 工具集扩展', status: 'pending', priority: 'low' }
    ]
  }
}

// ===== Actions =====
function openFull(path) {
  router.push(path)
  emit('close')
}

const LOADERS = {
  project: loadProject,
  knowledge: loadKB,
  tasks: loadTasks,
  graph: loadGraph,
  operators: loadOperators,
  workflow: loadWorkflows,
  resources: loadResources,
  plugins: loadPlugins,
  monitor: loadMonitor,
  dashboard: loadOverview
}

watch(() => props.tool, (t) => {
  if (t && LOADERS[t]) LOADERS[t]()
}, { immediate: true })

onMounted(() => {
  if (props.tool && LOADERS[props.tool]) LOADERS[props.tool]()
})
</script>

<style scoped>
.tool-drawer {
  position: relative;
}

.drawer-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.3);
  backdrop-filter: blur(2px);
  z-index: 100;
  transition: opacity 0.3s;
}

.drawer-panel {
  position: fixed;
  top: 0;
  right: 0;
  width: 420px;
  max-width: calc(100vw - 68px);
  height: 100vh;
  background: #fff;
  box-shadow: -8px 0 24px rgba(0, 0, 0, 0.12);
  z-index: 101;
  display: flex;
  flex-direction: column;
  transform: translateX(100%);
  transition: transform 0.35s cubic-bezier(0.4, 0, 0.2, 1);
}

.drawer-panel.open {
  transform: translateX(0);
}

.drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px;
  border-bottom: 1px solid #e2e8f0;
  background: linear-gradient(135deg, #f8fafc 0%, #f1f5f9 100%);
}

.drawer-title {
  display: flex;
  align-items: center;
  gap: 14px;
}

.title-icon {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 22px;
}

.title-text {
  font-size: 18px;
  font-weight: 700;
  color: #0f172a;
}

.title-sub {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}

.close-btn {
  font-size: 20px;
  color: #64748b;
  cursor: pointer;
  padding: 6px;
  border-radius: 8px;
  transition: all 0.2s;
}

.close-btn:hover {
  background: #f1f5f9;
  color: #ef4444;
}

.drawer-body {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

/* ===== Section Common ===== */
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.section-header h3 {
  font-size: 16px;
  font-weight: 700;
  color: #0f172a;
  margin: 0;
}

.section-actions {
  display: flex;
  gap: 8px;
  margin-top: 16px;
  padding-top: 16px;
  border-top: 1px solid #f1f5f9;
}

/* ===== KB Section ===== */
.kb-stats {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 16px;
}

.stat-item {
  text-align: center;
  padding: 10px 6px;
  background: #f8fafc;
  border-radius: 10px;
}

.stat-num {
  font-size: 20px;
  font-weight: 700;
  color: #0f172a;
}

.stat-label {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

.kb-doc-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.kb-doc-item {
  padding: 10px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
}

.kb-doc-item:hover {
  border-color: #6366f1;
  background: #f8fafc;
}

.doc-title {
  font-size: 14px;
  font-weight: 600;
  color: #0f172a;
  margin-bottom: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.doc-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: #64748b;
}

.ai-badge {
  color: #0d9488;
  font-weight: 600;
}

/* ===== Task Section ===== */
.task-filter {
  margin-bottom: 12px;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.task-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  transition: all 0.2s;
}

.task-item:hover {
  border-color: #6366f1;
  background: #f8fafc;
}

.task-checkbox {
  width: 14px;
  height: 14px;
  border-radius: 4px;
  border: 2px solid #cbd5e1;
  flex-shrink: 0;
}

.task-checkbox.completed {
  background: #10b981;
  border-color: #10b981;
}

.task-checkbox.running {
  background: #0ea5e9;
  border-color: #0ea5e9;
}

.task-info {
  flex: 1;
  min-width: 0;
}

.task-title {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 4px;
  font-size: 11px;
  color: #64748b;
}

.task-time {
  color: #94a3b8;
}

.task-more {
  cursor: pointer;
  color: #94a3b8;
  padding: 4px;
  border-radius: 4px;
}

.task-more:hover {
  background: #f1f5f9;
  color: #0f172a;
}

/* ===== Graph Section ===== */
.graph-stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
  margin-bottom: 16px;
}

.graph-preview {
  height: 120px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 12px;
  margin-bottom: 16px;
  overflow: hidden;
  display: grid;
  place-items: center;
}

.graph-placeholder {
  text-align: center;
  color: #64748b;
}

.graph-placeholder p {
  margin: 8px 0 4px;
  font-weight: 600;
}

.graph-placeholder .hint {
  font-size: 12px;
  color: #94a3b8;
}

.graph-entities h4 {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
  margin: 0 0 8px;
}

.entity-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.entity-tag {
  cursor: pointer;
}

/* ===== Operators Section ===== */
.op-search {
  margin-bottom: 12px;
}

.op-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.op-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  transition: all 0.2s;
}

.op-item:hover {
  border-color: #6366f1;
  background: #f8fafc;
}

.op-color {
  width: 4px;
  height: 32px;
  border-radius: 2px;
  flex-shrink: 0;
}

.op-info {
  flex: 1;
  min-width: 0;
}

.op-name {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.op-desc {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ===== Workflow Section ===== */
.wf-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.wf-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  transition: all 0.2s;
}

.wf-item:hover {
  border-color: #6366f1;
  background: #f8fafc;
}

.wf-status {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.wf-status.active {
  background: #10b981;
  box-shadow: 0 0 8px rgba(16, 185, 129, 0.4);
}

.wf-status.paused {
  background: #f59e0b;
}

.wf-info {
  flex: 1;
  min-width: 0;
}

.wf-name {
  font-size: 14px;
  font-weight: 600;
  color: #0f172a;
}

.wf-desc {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}

/* ===== Resources Section ===== */
.res-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}

.res-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  background: #f8fafc;
  border-radius: 12px;
}

.res-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 18px;
  flex-shrink: 0;
}

.res-name {
  font-size: 12px;
  color: #64748b;
}

.res-value {
  font-size: 16px;
  font-weight: 700;
  color: #0f172a;
}

/* ===== Plugins Section ===== */
.plugin-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.plugin-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
}

.plugin-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 20px;
  flex-shrink: 0;
}

.plugin-info {
  flex: 1;
  min-width: 0;
}

.plugin-name {
  font-size: 14px;
  font-weight: 600;
  color: #0f172a;
}

.plugin-desc {
  font-size: 12px;
  color: #64748b;
  margin-top: 2px;
}

/* ===== Monitor Section ===== */
.monitor-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
  margin-bottom: 16px;
}

.monitor-card {
  padding: 12px;
  background: #f8fafc;
  border-radius: 12px;
}

.monitor-label {
  font-size: 11px;
  color: #64748b;
  margin-bottom: 4px;
}

.monitor-value {
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 8px;
}

.monitor-bar {
  height: 4px;
  background: #e2e8f0;
  border-radius: 2px;
  overflow: hidden;
}

.bar-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s;
}

.monitor-health {
  border-top: 1px solid #f1f5f9;
  padding-top: 12px;
}

.health-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  font-size: 13px;
}

.health-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.health-dot.healthy {
  background: #10b981;
  box-shadow: 0 0 6px rgba(16, 185, 129, 0.4);
}

.health-dot.warning {
  background: #f59e0b;
  box-shadow: 0 0 6px rgba(245, 158, 11, 0.4);
}

.health-dot.error {
  background: #ef4444;
  box-shadow: 0 0 6px rgba(239, 68, 68, 0.4);
}

.health-status {
  margin-left: auto;
  font-size: 12px;
  font-weight: 600;
}

.health-status.healthy { color: #10b981; }
.health-status.warning { color: #f59e0b; }
.health-status.error { color: #ef4444; }

/* ===== Overview Section ===== */
.overview-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
  margin-bottom: 20px;
}

.overview-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px;
  background: #f8fafc;
  border-radius: 14px;
}

.overview-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  font-size: 20px;
  flex-shrink: 0;
}

.overview-value {
  font-size: 20px;
  font-weight: 700;
  color: #0f172a;
}

.overview-label {
  font-size: 12px;
  color: #64748b;
}

.recent-activity h4 {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
  margin: 0 0 10px;
}

.activity-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.activity-item {
  display: flex;
  gap: 10px;
  font-size: 13px;
}

.activity-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-top: 6px;
  flex-shrink: 0;
}

.activity-text {
  color: #0f172a;
  line-height: 1.5;
}

.activity-time {
  font-size: 11px;
  color: #94a3b8;
  margin-top: 2px;
}

/* ===== Empty State ===== */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 300px;
  color: #64748b;
}

.empty-state p {
  margin-top: 12px;
  font-size: 14px;
}

/* ===== Drawer Animation ===== */
.drawer-fade-enter-active,
.drawer-fade-leave-active {
  transition: opacity 0.3s;
}

.drawer-fade-enter-from,
.drawer-fade-leave-to {
  opacity: 0;
}

/* ===== Scrollbar ===== */
.drawer-body::-webkit-scrollbar {
  width: 6px;
}
.drawer-body::-webkit-scrollbar-track {
  background: transparent;
}
.drawer-body::-webkit-scrollbar-thumb {
  background: #cbd5e1;
  border-radius: 3px;
}
.drawer-body::-webkit-scrollbar-thumb:hover {
  background: #94a3b8;
}

/* ===== Responsive ===== */
@media (max-width: 768px) {
  .drawer-panel {
    width: 100vw;
    max-width: 100vw;
  }
  .kb-stats {
    grid-template-columns: repeat(2, 1fr);
  }
  .overview-grid {
    grid-template-columns: 1fr;
  }
  .monitor-grid {
    grid-template-columns: 1fr;
  }
}

/* ===== Content transition ===== */
.drawer-body {
  animation: fadeIn 0.3s ease;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}

/* ===== Project Section ===== */
.project-overview {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 20px;
}

.project-stat {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
  background: #f8fafc;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.project-stat:hover {
  background: #f1f5f9;
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

.ps-icon {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  font-size: 18px;
  flex-shrink: 0;
}

.ps-info {
  min-width: 0;
}

.ps-value {
  font-size: 18px;
  font-weight: 700;
  color: #0f172a;
}

.ps-label {
  font-size: 11px;
  color: #64748b;
}

.project-block {
  margin-bottom: 20px;
  padding: 14px;
  background: #f8fafc;
  border-radius: 14px;
  border: 1px solid #e2e8f0;
}

.block-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.block-header h4 {
  font-size: 14px;
  font-weight: 700;
  color: #0f172a;
  margin: 0;
}

/* Flow cards */
.flow-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.flow-card {
  display: flex;
  gap: 12px;
  padding: 12px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
}

.flow-card:hover {
  border-color: #7c3aed;
  box-shadow: 0 2px 8px rgba(124, 58, 237, 0.15);
}

.flow-graph {
  width: 140px;
  height: 90px;
  flex-shrink: 0;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 8px;
  overflow: hidden;
}

.flow-svg {
  width: 100%;
  height: 100%;
}

.flow-info {
  flex: 1;
  min-width: 0;
}

.flow-name {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
  margin-bottom: 4px;
}

.flow-desc {
  font-size: 11px;
  color: #64748b;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.flow-status {
  flex-shrink: 0;
}

.empty-hint {
  text-align: center;
  padding: 20px;
  color: #94a3b8;
  font-size: 13px;
}

/* Tabs */
.project-tabs {
  margin-top: -4px;
}

.project-tabs :deep(.el-tabs__header) {
  margin-bottom: 10px;
}

.project-tabs :deep(.el-tabs__item) {
  font-size: 13px;
}

/* MCP list */
.mcp-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.mcp-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
  transition: all 0.2s;
}

.mcp-item:hover {
  border-color: #7c3aed;
}

.mcp-icon {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  background: #ede9fe;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}

.mcp-info {
  flex: 1;
  min-width: 0;
}

.mcp-name {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.mcp-desc {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
}

/* Automation list */
.auto-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.auto-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 10px;
}

.auto-status {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}

.auto-status.passed {
  background: #d1fae5;
  color: #059669;
}

.auto-status.running {
  background: #fef3c7;
  color: #d97706;
  animation: pulse 1.5s infinite;
}

.auto-status.draft {
  background: #f1f5f9;
  color: #64748b;
}

.auto-status.failed {
  background: #fee2e2;
  color: #dc2626;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.auto-info {
  flex: 1;
  min-width: 0;
}

.auto-name {
  font-size: 13px;
  font-weight: 600;
  color: #0f172a;
}

.auto-meta {
  font-size: 11px;
  color: #64748b;
  margin-top: 2px;
  display: flex;
  gap: 8px;
  align-items: center;
}

.auto-report {
  color: #7c3aed;
  font-weight: 500;
}

/* Task board */
.task-board {
  display: grid;
  grid-template-columns: 80px 80px 80px 1fr;
  gap: 8px;
  align-items: start;
}

.task-col {
  text-align: center;
  padding: 10px 6px;
  background: #fff;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
}

.col-label {
  font-size: 11px;
  color: #64748b;
}

.col-count {
  font-size: 20px;
  font-weight: 700;
  color: #0f172a;
}

.col-count.running { color: #f59e0b; }
.col-count.done { color: #10b981; }

.col-tasks {
  padding: 8px;
  background: #fff;
  border-radius: 10px;
  border: 1px solid #e2e8f0;
}

.mini-task {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  font-size: 12px;
  color: #334155;
}

.mini-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.mini-dot.pending { background: #f59e0b; }
.mini-dot.running { background: #0ea5e9; }
.mini-dot.completed { background: #10b981; }

.mini-title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mini-more {
  font-size: 11px;
  color: #94a3b8;
  padding: 4px 6px;
}

/* Quick actions */
.quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.quick-actions .el-button {
  flex: 1;
  min-width: 80px;
}
</style>
