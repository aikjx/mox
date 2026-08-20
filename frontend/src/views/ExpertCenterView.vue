<template>
  <div class="expert-center">
    <div class="head">
      <div>
        <h2 class="page-title">专家联盟中心</h2>
        <p class="page-subtitle">管理企业级专家团队，实现多专家协同咨询与辩论</p>
      </div>
      <div class="head-actions">
        <el-button type="primary" @click="showRegister = true">
          <el-icon><Plus /></el-icon> 注册专家
        </el-button>
        <el-button @click="loadExperts"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <div class="grid grid-3 main-grid">
      <!-- 专家列表 -->
      <div class="panel card-pad span1">
        <h3 class="section-title">专家库（{{ experts.length }}）</h3>
        <div class="filter-bar">
          <el-select v-model="filterType" placeholder="专家类型" clearable size="small">
            <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
          </el-select>
          <el-input v-model="keyword" placeholder="搜索专家" clearable size="small" style="flex: 1" />
        </div>
        <el-scrollbar height="420px">
          <div
            v-for="exp in filteredExperts"
            :key="exp.id"
            class="expert-card"
            :class="{ sel: isSelected(exp.id) }"
            @click="toggleSelect(exp)"
          >
            <div class="expert-avatar" :style="{ background: getColor(exp.type) }">
              <el-icon><component :is="getIcon(exp.type)" /></el-icon>
            </div>
            <div class="expert-info">
              <div class="expert-name">{{ exp.name }}</div>
              <div class="expert-type">{{ typeLabel(exp.type) }}</div>
              <div class="expert-caps">
                <el-tag v-for="cap in exp.capabilities.slice(0, 3)" :key="cap" size="small" type="info">
                  {{ cap }}
                </el-tag>
              </div>
            </div>
            <div class="expert-status" :class="exp.status">
              {{ exp.status === 'active' ? '在线' : '离线' }}
            </div>
          </div>
          <el-empty v-if="!filteredExperts.length" description="暂无专家" :image-size="60" />
        </el-scrollbar>
      </div>

      <!-- 咨询工作台 -->
      <div class="panel card-pad span2">
        <div class="consult-header">
          <h3 class="section-title">专家咨询工作台</h3>
          <div class="mode-switch">
            <el-radio-group v-model="mode" size="small">
              <el-radio-button value="single">单专家咨询</el-radio-button>
              <el-radio-button value="multi">多专家协同</el-radio-button>
              <el-radio-button value="debate">专家辩论</el-radio-button>
            </el-radio-group>
          </div>
        </div>

        <div v-if="mode === 'single'" class="single-consult">
          <div class="selected-area">
            <span class="muted">已选择：</span>
            <template v-if="selectedCount()">
              <span class="chip" v-for="id in selectedExpertIds" :key="id">
                {{ getExpertName(id) }}
                <el-icon class="chip-x" @click="removeExpert(id)"><Close /></el-icon>
              </span>
            </template>
            <span v-else class="muted">请从左侧选择一位专家</span>
          </div>
          <div class="consult-input">
            <el-input
              v-model="question"
              type="textarea"
              :rows="3"
              placeholder="请输入你的问题..."
            />
            <el-button
              type="primary"
              :loading="consulting"
              :disabled="!selectedCount() || !question.trim()"
              @click="doConsult"
              style="width: 100%; margin-top: 10px"
            >
              <el-icon><Promotion /></el-icon> 开始咨询
            </el-button>
          </div>
        </div>

        <div v-else-if="mode === 'multi'" class="multi-consult">
          <div class="selected-area">
            <span class="muted">已选择 {{ selectedCount() }} 位专家：</span>
            <span class="chip" v-for="id in selectedExpertIds" :key="id">
              {{ getExpertName(id) }}
            </span>
          </div>
          <el-input
            v-model="question"
            type="textarea"
            :rows="3"
            placeholder="请输入需要多位专家协同分析的问题..."
          />
          <el-button
            type="primary"
            :loading="consulting"
            :disabled="selectedCount() < 2 || !question.trim()"
            @click="doMultiConsult"
            style="width: 100%; margin-top: 10px"
          >
            <el-icon><ChatDotRound /></el-icon> 协同分析
          </el-button>
        </div>

        <div v-else class="debate-mode">
          <div class="selected-area">
            <span class="muted">已选择 {{ selectedCount() }} 位专家参与辩论：</span>
            <span class="chip" v-for="id in selectedExpertIds" :key="id">
              {{ getExpertName(id) }}
            </span>
          </div>
          <el-input-number v-model="rounds" :min="2" :max="5" label="辩论轮数" style="margin: 10px 0" />
          <el-input
            v-model="question"
            type="textarea"
            :rows="3"
            placeholder="请输入辩论主题..."
          />
          <el-button
            type="primary"
            :loading="consulting"
            :disabled="selectedCount() < 2 || !question.trim()"
            @click="doDebate"
            style="width: 100%; margin-top: 10px"
          >
            <el-icon><MagicStick /></el-icon> 开始辩论
          </el-button>
        </div>

        <div v-if="results.length" class="results">
          <h4 class="results-title">咨询结果</h4>
          <el-scrollbar height="300px">
            <div v-for="(r, i) in results" :key="i" class="result-item">
              <div class="result-head">
                <span class="expert-badge" :style="{ background: getColorByType(r.expert?.type) }">
                  {{ r.expert?.name || '专家' }}
                </span>
                <span v-if="r.confidence" class="confidence">置信度: {{ (r.confidence * 100).toFixed(0) }}%</span>
              </div>
              <div class="result-content">{{ r.response }}</div>
            </div>
          </el-scrollbar>
        </div>

        <div v-if="debateHistory.length" class="debate-summary">
          <h4 class="results-title">辩论总结</h4>
          <div class="debate-final">{{ debateHistory[debateHistory.length - 1].final_synthesis }}</div>
        </div>
      </div>
    </div>

    <!-- 注册弹窗 -->
    <el-dialog v-model="showRegister" title="注册新专家" width="480px">
      <el-form label-width="90px">
        <el-form-item label="专家名称">
          <el-input v-model="newExpert.name" placeholder="如：数据库专家" />
        </el-form-item>
        <el-form-item label="专家类型">
          <el-select v-model="newExpert.type" style="width: 100%">
            <el-option v-for="t in expertTypes" :key="t" :label="typeLabel(t)" :value="t" />
          </el-select>
        </el-form-item>
        <el-form-item label="能力标签">
          <el-input v-model="newExpert.capabilities_str" placeholder="用逗号分隔，如：性能优化,索引调优" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="newExpert.description" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRegister = false">取消</el-button>
        <el-button type="primary" :loading="registering" @click="doRegister">注册</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Refresh, Close, Promotion, ChatDotRound, MagicStick } from '@element-plus/icons-vue'
import { getExperts, registerExpert, consultExpert, multiExpertConsult, expertDebate } from '@/api'

const experts = ref([])
const filterType = ref('')
const keyword = ref('')
const selectedExpertIds = ref([])
const mode = ref('single')
const question = ref('')
const consulting = ref(false)
const results = ref([])
const debateHistory = ref([])
const rounds = ref(2)

const showRegister = ref(false)
const registering = ref(false)
const newExpert = ref({ name: '', type: 'algorithm', capabilities_str: '', description: '' })

const typeLabels = {
  algorithm: '算法专家',
  architecture: '架构专家',
  data: '数据专家',
  ai: 'AI专家',
  workflow: '工作流专家',
  operator: '算子系统专家',
  graph: '知识图谱专家',
  security: '安全专家',
  performance: '性能优化专家',
  monitor: '可观测性专家',
  market: '商业智能专家',
  mcp: 'MCP协议专家',
  automation: '自动化专家',
  requirement: '需求工程专家',
  fusion: '融合专家',
  custom: '自定义专家'
}

const expertTypes = computed(() => Object.keys(typeLabels))

function typeLabel(t) {
  return typeLabels[t] || t
}

function getColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#0891b2', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', operator: '#8b5cf6',
    graph: '#06b6d4', security: '#ef4444', performance: '#14b8a6',
    monitor: '#f97316', market: '#f43f5e', mcp: '#a855f7',
    automation: '#0ea5e9', requirement: '#16a34a', fusion: '#7c3aed',
    custom: '#64748b'
  }
  return colors[type] || colors.custom
}

function getColorByType(type) {
  return getColor(type)
}

function getIcon(type) {
  const icons = {
    algorithm: 'TrendCharts', architecture: 'Grid', data: 'Coin',
    ai: 'MagicStick', workflow: 'Operation', operator: 'Cpu',
    graph: 'Share', security: 'Lock', performance: 'Lightning',
    monitor: 'DataLine', market: 'Shop', mcp: 'Link',
    automation: 'MagicStick', requirement: 'Tickets', fusion: 'Aim',
    custom: 'User'
  }
  return icons[type] || icons.custom
}

const filteredExperts = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return experts.value.filter(e => {
    const matchType = !filterType.value || e.type === filterType.value
    const matchKw = !kw || 
      e.name.toLowerCase().includes(kw) ||
      (e.description || '').toLowerCase().includes(kw) ||
      (e.capabilities || []).some(c => c.toLowerCase().includes(kw))
    return matchType && matchKw
  })
})

function getExpertName(id) {
  return experts.value.find(e => e.id === id)?.name || id
}

function toggleSelect(exp) {
  if (exp.status !== 'active') {
    ElMessage.warning('该专家当前不在线')
    return
  }
  const idx = selectedExpertIds.value.indexOf(exp.id)
  if (idx !== -1) {
    selectedExpertIds.value.splice(idx, 1)
  } else {
    if (mode.value === 'single' && selectedExpertIds.value.length >= 1) {
      selectedExpertIds.value = [exp.id]
    } else {
      selectedExpertIds.value.push(exp.id)
    }
  }
}

function isSelected(id) {
  return selectedExpertIds.value.includes(id)
}

function selectedCount() {
  return selectedExpertIds.value.length
}

function removeExpert(id) {
  const idx = selectedExpertIds.value.indexOf(id)
  if (idx !== -1) selectedExpertIds.value.splice(idx, 1)
}

async function loadExperts() {
  try {
    experts.value = await getExperts()
  } catch (e) {
    ElMessage.error('加载专家列表失败：' + e.message)
  }
}

async function doConsult() {
  const expertId = selectedExpertIds.value[0]
  if (!expertId || !question.value.trim()) return
  
  consulting.value = true
  try {
    const result = await consultExpert(expertId, {
      messages: [{ role: 'user', content: question.value }]
    })
    results.value = [{
      expert: { id: expertId, name: getExpertName(expertId) },
      response: result.response,
      confidence: result.metadata?.confidence
    }]
    ElMessage.success('咨询完成')
  } catch (e) {
    ElMessage.error('咨询失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doMultiConsult() {
  const expertIds = [...selectedExpertIds.value]
  if (expertIds.length < 2 || !question.value.trim()) return
  
  consulting.value = true
  try {
    const result = await multiExpertConsult({
      question: question.value,
      expert_ids: expertIds
    })
    results.value = result.results.filter(r => r.success).map(r => ({
      expert: r.expert,
      response: r.response,
      confidence: r.confidence
    }))
    ElMessage.success(`协同分析完成，共 ${result.successful} 位专家参与`)
  } catch (e) {
    ElMessage.error('协同分析失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doDebate() {
  const expertIds = [...selectedExpertIds.value]
  if (expertIds.length < 2 || !question.value.trim()) return
  
  consulting.value = true
  try {
    const result = await expertDebate({
      question: question.value,
      expert_ids: expertIds,
      rounds: rounds.value
    })
    results.value = []
    result.history.forEach((round, idx) => {
      round.results.forEach(r => {
        if (r.success) {
          results.value.push({
            expert: r.expert,
            response: r.response,
            round: idx + 1,
            confidence: r.confidence
          })
        }
      })
    })
    debateHistory.value = [result]
    ElMessage.success(`辩论完成，共 ${rounds.value} 轮`)
  } catch (e) {
    ElMessage.error('辩论失败：' + e.message)
  } finally {
    consulting.value = false
  }
}

async function doRegister() {
  if (!newExpert.value.name || !newExpert.value.type) {
    ElMessage.warning('请填写专家名称和类型')
    return
  }
  
  registering.value = true
  try {
    await registerExpert({
      name: newExpert.value.name,
      type: newExpert.value.type,
      capabilities: (newExpert.value.capabilities_str || '').split(',').map(s => s.trim()).filter(Boolean),
      description: newExpert.value.description
    })
    ElMessage.success('注册成功')
    showRegister.value = false
    newExpert.value = { name: '', type: 'algorithm', capabilities_str: '', description: '' }
    await loadExperts()
  } catch (e) {
    ElMessage.error('注册失败：' + e.message)
  } finally {
    registering.value = false
  }
}

onMounted(loadExperts)
</script>

<style scoped>
.expert-center {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.head-actions {
  display: flex;
  gap: 10px;
}
.span1 { grid-column: span 1; }
.span2 { grid-column: span 2; }
.card-pad { padding: 20px 22px; }
.main-grid { align-items: start; }

.filter-bar {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.expert-card {
  display: flex;
  gap: 10px;
  align-items: center;
  padding: 12px;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
  border: 2px solid transparent;
  margin-bottom: 8px;
}
.expert-card:hover {
  background: var(--bg-page);
}
.expert-card.sel {
  background: var(--brand-soft);
  border-color: var(--brand);
}

.expert-avatar {
  width: 44px;
  height: 44px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 20px;
  flex-shrink: 0;
}

.expert-info {
  flex: 1;
  min-width: 0;
}
.expert-name {
  font-weight: 700;
  font-size: 14px;
}
.expert-type {
  font-size: 12px;
  color: var(--text-3);
  margin: 2px 0;
}
.expert-caps {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

.expert-status {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 999px;
  font-weight: 600;
}
.expert-status.active {
  background: #dcfce7;
  color: #16a34a;
}
.expert-status.inactive {
  background: #f1f5f9;
  color: #94a3b8;
}

.consult-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.selected-area {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--brand-soft);
  color: var(--brand-dark);
  padding: 3px 8px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 600;
}
.chip-x {
  cursor: pointer;
  font-size: 11px;
}
.chip-x:hover {
  color: var(--danger);
}

.muted {
  color: var(--text-3);
  font-size: 13px;
}

.consult-input {
  display: flex;
  flex-direction: column;
}

.results {
  margin-top: 20px;
  border-top: 1px solid var(--border);
  padding-top: 16px;
}
.results-title {
  font-size: 14px;
  font-weight: 700;
  margin-bottom: 12px;
}

.result-item {
  background: var(--bg-page);
  border-radius: 10px;
  padding: 14px;
  margin-bottom: 10px;
}
.result-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.expert-badge {
  color: #fff;
  padding: 3px 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
}
.confidence {
  font-size: 12px;
  color: var(--text-3);
}
.result-content {
  font-size: 13px;
  line-height: 1.7;
  white-space: pre-wrap;
}

.debate-summary {
  margin-top: 20px;
  border-top: 1px solid var(--border);
  padding-top: 16px;
  background: linear-gradient(135deg, #f8fafc, #f1f5f9);
  border-radius: 12px;
  padding: 16px;
}
.debate-final {
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;
}

.mode-switch {
  display: flex;
}

.debate-mode .el-input-number {
  width: auto;
}
</style>