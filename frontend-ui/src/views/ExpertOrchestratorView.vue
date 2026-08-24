<template>
  <div class="expert-orchestrator">
    <div class="head">
      <div>
        <h2 class="page-title">V2 编排引擎控制台</h2>
        <p class="page-subtitle">插件化编排 · Plan/Act 双模式 · 学习闭环 · 事件驱动</p>
      </div>
      <div class="head-actions">
        <el-button @click="loadAll" :loading="loading">
          <el-icon><Refresh /></el-icon> 刷新
        </el-button>
      </div>
    </div>

    <el-tabs v-model="activeTab" type="border-card" class="main-tabs">
      <!-- 编排执行 -->
      <el-tab-pane label="编排执行" name="execute">
        <el-row :gutter="16">
          <el-col :span="14">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>AI 编排执行</h3>
                <el-tag type="success" v-if="stats?.activePlugins">
                  {{ stats.activePlugins }} 个插件就绪
                </el-tag>
              </div>
              <div class="orchestrate-form">
                <el-form :model="form" label-width="100px" size="large">
                  <el-form-item label="问题描述">
                    <el-input
                      v-model="form.question"
                      type="textarea"
                      :rows="4"
                      placeholder="请输入您的问题，编排引擎将自动选择最佳 Pipeline 和专家组合..."
                    />
                  </el-form-item>
                  <el-form-item label="Pipeline 模式">
                    <el-radio-group v-model="form.pipeline">
                      <el-radio value="standard">标准（6步全流程）</el-radio>
                      <el-radio value="plan_act">Plan/Act 双模式</el-radio>
                      <el-radio value="fast_path">快速路径</el-radio>
                      <el-radio value="deep_analysis">深度分析</el-radio>
                    </el-radio-group>
                  </el-form-item>
                  <el-form-item label="选项">
                    <el-checkbox v-model="form.enableCheckpoints">启用检查点</el-checkbox>
                    <el-checkbox v-model="form.enableLearning">启用学习闭环</el-checkbox>
                  </el-form-item>
                  <el-form-item>
                    <el-button type="primary" @click="runOrchestrate" :loading="running" size="large">
                      <el-icon><Promotion /></el-icon> 执行编排
                    </el-button>
                    <el-button @click="generatePlan" :loading="planGenerating" size="large">
                      <el-icon><Document /></el-icon> 仅生成计划
                    </el-button>
                  </el-form-item>
                </el-form>
              </div>

              <div v-if="currentPlan" class="plan-section">
                <h4>📋 生成的执行计划</h4>
                <div class="plan-info">
                  <el-tag>策略: {{ currentPlan.strategy }}</el-tag>
                  <el-tag>步骤数: {{ currentPlan.steps?.length || 0 }}</el-tag>
                </div>
                <el-timeline v-if="currentPlan.steps?.length">
                  <el-timeline-item
                    v-for="(step, idx) in currentPlan.steps"
                    :key="step.id"
                    :timestamp="`Step ${idx + 1}`"
                    :color="'#' + (['67C23A','409EFF','E6A23C','F56C6C','909399'][idx % 5])"
                  >
                    <div class="step-card">
                      <div class="step-desc">{{ step.description }}</div>
                      <div class="step-meta">
                        <el-tag size="small">{{ step.action }}</el-tag>
                        <span class="step-duration">{{ step.estimatedDuration }}ms</span>
                      </div>
                    </div>
                  </el-timeline-item>
                </el-timeline>
              </div>
            </div>
          </el-col>

          <el-col :span="10">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>Pipeline 说明</h3>
              </div>
              <div class="pipeline-guide">
                <div v-for="p in pipelineInfo" :key="p.name" class="pipeline-card">
                  <div class="pipeline-name" :style="{ color: p.color }">{{ p.label }}</div>
                  <div class="pipeline-steps">{{ p.steps }}</div>
                  <div class="pipeline-desc">{{ p.desc }}</div>
                </div>
              </div>
            </div>
          </el-col>
        </el-row>

        <!-- 执行结果 -->
        <div v-if="result" class="result-section">
          <el-divider content-position="left">执行结果</el-divider>
          <div class="result-card" :class="result.status">
            <div class="result-header">
              <el-tag :type="result.status === 'success' ? 'success' : 'danger'" size="large">
                {{ result.status === 'success' ? '✅ 执行成功' : '❌ 执行失败' }}
              </el-tag>
              <span class="result-time">耗时 {{ result.duration }}ms</span>
              <span v-if="result.checkpoints" class="result-checkpoints">检查点: {{ result.checkpoints }}</span>
            </div>
            <div class="result-content">
              <pre class="result-json">{{ formatJSON(result) }}</pre>
            </div>
            <div v-if="result.state?.execution?.expertsConsulted?.length" class="result-experts">
              <h4>参与专家</h4>
              <el-tag v-for="e in result.state.execution.expertsConsulted" :key="e.id" class="expert-tag">
                {{ e.id }} ({{ e.role }} · {{ (e.score * 100).toFixed(0) }}分)
              </el-tag>
            </div>
          </div>
        </div>
      </el-tab-pane>

      <!-- 插件管理 -->
      <el-tab-pane label="插件市场" name="plugins">
        <div class="plugins-list">
          <div v-for="plugin in plugins" :key="plugin.name" class="plugin-card">
            <div class="plugin-icon">
              <el-icon :size="32" color="#409EFF"><Link /></el-icon>
            </div>
            <div class="plugin-info">
              <div class="plugin-name">{{ plugin.name }}</div>
              <div class="plugin-desc">{{ plugin.description }}</div>
              <div class="plugin-version">v{{ plugin.version }}</div>
            </div>
            <div class="plugin-status">
              <el-tag type="success" size="small">已加载</el-tag>
            </div>
          </div>
        </div>
        <el-empty v-if="!plugins.length" description="暂无加载的插件" />
      </el-tab-pane>

      <!-- 统计面板 -->
      <el-tab-pane label="统计监控" name="stats">
        <div class="stats-grid">
          <div class="stat-card success">
            <div class="stat-icon">
              <el-icon :size="32"><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ stats?.totalTurns || 0 }}</div>
              <div class="stat-label">总执行次数</div>
            </div>
          </div>
          <div class="stat-card">
            <div class="stat-icon">
              <el-icon :size="32"><Timer /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ stats?.avgDuration || 0 }}ms</div>
              <div class="stat-label">平均耗时</div>
            </div>
          </div>
          <div class="stat-card info">
            <div class="stat-icon">
              <el-icon :size="32"><Grid /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ stats?.activePlugins || 0 }}</div>
              <div class="stat-label">活跃插件</div>
            </div>
          </div>
          <div class="stat-card warning">
            <div class="stat-icon">
              <el-icon :size="32"><Lightning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-value">{{ successRate }}%</div>
              <div class="stat-label">成功率</div>
            </div>
          </div>
        </div>

        <el-row :gutter="16" style="margin-top: 16px;">
          <el-col :span="12">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>执行状态分布</h3>
              </div>
              <div class="status-dist">
                <div v-for="(count, status) in stats?.byStatus" :key="status" class="status-bar-row">
                  <span class="status-name">{{ status }}</span>
                  <el-progress :percentage="((count / (stats?.totalTurns || 1)) * 100).toFixed(0)" :color="status === 'success' ? '#67C23A' : '#F56C6C'" />
                  <span class="status-count">{{ count }}</span>
                </div>
              </div>
            </div>
          </el-col>
          <el-col :span="12">
            <div class="panel card-pad">
              <div class="panel-head">
                <h3>Pipeline 使用分布</h3>
              </div>
              <div class="status-dist">
                <div v-for="(count, mode) in stats?.byMode" :key="mode" class="status-bar-row">
                  <span class="status-name">{{ mode }}</span>
                  <el-progress :percentage="((count / (stats?.totalTurns || 1)) * 100).toFixed(0)" />
                  <span class="status-count">{{ count }}</span>
                </div>
              </div>
            </div>
          </el-col>
        </el-row>

        <!-- 执行历史 -->
        <div class="panel card-pad" style="margin-top: 16px;">
          <div class="panel-head">
            <h3>最近执行记录</h3>
            <el-button size="small" @click="loadHistory">刷新</el-button>
          </div>
          <el-table :data="history" stripe style="width: 100%" max-height="300">
            <el-table-column prop="input.mode" label="Pipeline" width="120" />
            <el-table-column label="输入" min-width="200">
              <template #default="{ row }">
                {{ row.input?.question?.slice(0, 50) || row.input?.message?.slice(0, 50) || '-' }}
              </template>
            </el-table-column>
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="row.result.status === 'success' ? 'success' : 'danger'" size="small">
                  {{ row.result.status }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="result.duration" label="耗时(ms)" width="100" />
            <el-table-column label="时间" width="180">
              <template #default="{ row }">
                {{ formatTime(row.timestamp) }}
              </template>
            </el-table-column>
          </el-table>
        </div>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import {
  Refresh, Promotion, Document, Link, CircleCheck, Timer, Grid, Lightning
} from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import {
  expertOrchestrate, expertGeneratePlan,
  getOrchestrationStats, listOrchestrationPlugins, getOrchestrationHistory
} from '@/api/index.js'

const loading = ref(false)
const running = ref(false)
const planGenerating = ref(false)
const activeTab = ref('execute')

const form = ref({
  question: '',
  pipeline: 'standard',
  enableCheckpoints: true,
  enableLearning: true
})

const result = ref(null)
const currentPlan = ref(null)
const plugins = ref([])
const stats = ref(null)
const history = ref([])

const pipelineInfo = [
  { name: 'standard', label: '标准流程', steps: '感知→记忆→规划→执行→反思→学习', desc: '完整的6步全流程，适合通用场景', color: '#409EFF' },
  { name: 'plan_act', label: 'Plan/Act', steps: '感知→规划→记忆→执行→反思→学习', desc: '双模式执行，先生成计划再逐步执行', color: '#67C23A' },
  { name: 'fast_path', label: '快速路径', steps: '感知→执行→反思', desc: '精简3步，适合简单问题快速响应', color: '#E6A23C' },
  { name: 'deep_analysis', label: '深度分析', steps: '多轮迭代分析', desc: '支持多轮迭代，适合复杂分析任务', color: '#9C27B0' }
]

const successRate = computed(() => {
  if (!stats.value?.byStatus) return 0
  const total = stats.value.totalTurns || 1
  const success = stats.value.byStatus.success || 0
  return ((success / total) * 100).toFixed(1)
})

async function loadAll() {
  loading.value = true
  try {
    await Promise.all([loadStats(), loadPlugins(), loadHistory()])
  } finally {
    loading.value = false
  }
}

async function loadStats() {
  try {
    stats.value = await getOrchestrationStats()
  } catch (e) {
    console.error('Load stats error:', e)
  }
}

async function loadPlugins() {
  try {
    plugins.value = await listOrchestrationPlugins()
  } catch (e) {
    console.error('Load plugins error:', e)
  }
}

async function loadHistory() {
  try {
    const r = await getOrchestrationHistory({ limit: 20 })
    // 契约兼容：后端返回 { history, total } 对象或直出数组；el-table :data 要求数组
    history.value = Array.isArray(r) ? r : (r?.history || [])
  } catch (e) {
    console.error('Load history error:', e)
  }
}

async function runOrchestrate() {
  if (!form.value.question?.trim()) {
    ElMessage.warning('请输入问题描述')
    return
  }
  running.value = true
  result.value = null
  currentPlan.value = null
  try {
    const data = await expertOrchestrate({
      question: form.value.question,
      pipeline: form.value.pipeline,
      enableCheckpoints: form.value.enableCheckpoints,
      enableLearning: form.value.enableLearning
    })
    result.value = data.orchestration || data
    if (data.plan) currentPlan.value = data.plan
    ElMessage.success('编排执行完成')
    await loadStats()
    await loadHistory()
  } catch (e) {
    ElMessage.error('执行失败: ' + e.message)
  } finally {
    running.value = false
  }
}

async function generatePlan() {
  if (!form.value.question?.trim()) {
    ElMessage.warning('请输入问题描述')
    return
  }
  planGenerating.value = true
  currentPlan.value = null
  try {
    const data = await expertGeneratePlan({
      question: form.value.question,
      pipeline: form.value.pipeline
    })
    if (data.plan) {
      currentPlan.value = data.plan
      ElMessage.success('计划生成成功')
    }
  } catch (e) {
    ElMessage.error('生成失败: ' + e.message)
  } finally {
    planGenerating.value = false
  }
}

function formatJSON(obj) {
  try {
    return JSON.stringify(obj, null, 2)
  } catch {
    return String(obj)
  }
}

function formatTime(ts) {
  if (!ts) return '-'
  const d = new Date(ts)
  return d.toLocaleString('zh-CN', { hour12: false })
}

onMounted(() => {
  loadAll()
})
</script>

<style scoped>
.expert-orchestrator {
  padding: 16px;
}
.head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
  background: linear-gradient(135deg, #667eea, #764ba2);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}
.page-subtitle {
  color: #909399;
  font-size: 13px;
  margin: 4px 0 0 0;
}
.card-pad {
  padding: 20px;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid #f0f0f0;
}
.panel-head h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}
.orchestrate-form {
  padding: 8px 0;
}
.pipeline-guide {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.pipeline-card {
  padding: 12px;
  background: #f8f9fa;
  border-radius: 8px;
  border-left: 3px solid #409EFF;
}
.pipeline-name {
  font-weight: 600;
  font-size: 14px;
}
.pipeline-steps {
  font-size: 12px;
  color: #606266;
  margin: 4px 0;
}
.pipeline-desc {
  font-size: 12px;
  color: #909399;
}
.plan-section {
  margin-top: 20px;
  padding: 16px;
  background: #f8f9fa;
  border-radius: 8px;
}
.plan-section h4 {
  margin: 0 0 12px 0;
  font-size: 14px;
}
.plan-info {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}
.step-card {
  background: #fff;
  padding: 10px 12px;
  border-radius: 6px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
}
.step-desc {
  font-weight: 500;
  font-size: 13px;
  margin-bottom: 4px;
}
.step-meta {
  display: flex;
  gap: 8px;
  align-items: center;
}
.step-duration {
  font-size: 12px;
  color: #909399;
}
.result-section {
  margin-top: 20px;
}
.result-card {
  padding: 16px;
  border-radius: 12px;
  border: 2px solid;
}
.result-card.success {
  background: #f0f9eb;
  border-color: #67C23A;
}
.result-card.error {
  background: #fef0f0;
  border-color: #F56C6C;
}
.result-header {
  display: flex;
  gap: 16px;
  align-items: center;
  margin-bottom: 12px;
}
.result-time {
  font-size: 13px;
  color: #606266;
}
.result-checkpoints {
  font-size: 13px;
  color: #909399;
}
.result-json {
  background: #fff;
  padding: 12px;
  border-radius: 6px;
  overflow-x: auto;
  max-height: 400px;
  font-size: 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
.result-experts {
  margin-top: 12px;
}
.expert-tag {
  margin: 4px;
}
.plugins-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}
.plugin-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
  transition: transform 0.2s, box-shadow 0.2s;
}
.plugin-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.08);
}
.plugin-icon {
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #ecf5ff;
  border-radius: 12px;
}
.plugin-info {
  flex: 1;
}
.plugin-name {
  font-weight: 600;
  font-size: 15px;
  color: #303133;
}
.plugin-desc {
  font-size: 12px;
  color: #606266;
  margin: 4px 0;
}
.plugin-version {
  font-size: 12px;
  color: #909399;
}
.plugin-status {
  flex-shrink: 0;
}
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 16px;
}
.stat-card {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 20px;
  background: #fff;
  border-radius: 12px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.06);
}
.stat-card .stat-icon {
  width: 56px;
  height: 56px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  background: #ecf5ff;
  color: #409EFF;
}
.stat-card.success .stat-icon {
  background: #f0f9eb;
  color: #67C23A;
}
.stat-card.info .stat-icon {
  background: #f4f4f5;
  color: #909399;
}
.stat-card.warning .stat-icon {
  background: #fdf6ec;
  color: #E6A23C;
}
.stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #303133;
}
.stat-label {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}
.status-dist {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.status-bar-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.status-name {
  width: 80px;
  font-size: 13px;
  color: #606266;
  text-transform: capitalize;
}
.status-bar-row .el-progress {
  flex: 1;
}
.status-count {
  width: 50px;
  text-align: right;
  font-size: 13px;
  color: #303133;
  font-weight: 500;
}
</style>