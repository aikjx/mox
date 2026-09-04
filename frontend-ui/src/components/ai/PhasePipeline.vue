<template>
  <div class="phase-pipeline" :class="{ 'phase-pipeline--running': isRunning }">
    <!-- 阶段进度条 -->
    <div class="phase-pipeline__track">
      <div
        v-for="(phase, index) in phases"
        :key="phase.key"
        class="phase-pipeline__node"
        :class="getPhaseClass(phase.key, index)"
        @click="handlePhaseClick(phase.key)"
      >
        <!-- 节点图标 -->
        <div class="phase-pipeline__icon">
          <span v-if="getPhaseStatus(phase.key) === 'done'">✓</span>
          <span v-else-if="getPhaseStatus(phase.key) === 'running'" class="phase-pipeline__spinner"></span>
          <span v-else>{{ index + 1 }}</span>
        </div>
        <!-- 阶段标签 -->
        <div class="phase-pipeline__label">
          <div class="phase-pipeline__name">{{ phase.label }}</div>
          <div v-if="getPhaseLatency(phase.key) > 0" class="phase-pipeline__latency">
            {{ formatLatency(getPhaseLatency(phase.key)) }}
          </div>
        </div>
        <!-- 连接线 -->
        <div v-if="index < phases.length - 1" class="phase-pipeline__connector"
          :class="{ 'phase-pipeline__connector--active': isPhasePast(phase.key) }">
        </div>
      </div>
    </div>

    <!-- 当前阶段详情 -->
    <transition name="fade">
      <div v-if="currentPhaseDetail" class="phase-pipeline__detail">
        <div class="phase-pipeline__detail-header">
          <span class="phase-pipeline__detail-icon">{{ currentPhaseDetail.icon }}</span>
          <span class="phase-pipeline__detail-title">{{ currentPhaseDetail.label }}</span>
          <span class="phase-pipeline__detail-status" :class="`status--${currentPhaseStatus}`">
            {{ statusText }}
          </span>
        </div>

        <!-- Intent 阶段详情 -->
        <div v-if="currentPhase === 'intent' && intentResult" class="phase-pipeline__detail-content">
          <div class="intent-result">
            <div class="intent-result__row">
              <span class="intent-result__label">意图类型：</span>
              <span class="intent-result__value intent-result__value--primary">{{ intentResult.intent }}</span>
            </div>
            <div class="intent-result__row">
              <span class="intent-result__label">置信度：</span>
              <span class="intent-result__value">
                <el-progress :percentage="Math.round(intentResult.confidence * 100)" :stroke-width="8" />
              </span>
            </div>
            <div v-if="intentResult.dimensions && intentResult.dimensions.length" class="intent-result__row">
              <span class="intent-result__label">匹配维度：</span>
              <div class="intent-result__tags">
                <el-tag v-for="dim in intentResult.dimensions" :key="dim" size="small" type="info">
                  {{ dim }}
                </el-tag>
              </div>
            </div>
          </div>
        </div>

        <!-- Team 阶段详情 -->
        <div v-if="currentPhase === 'team' && teamResult" class="phase-pipeline__detail-content">
          <div class="team-result">
            <div class="team-result__header">
              <span>已组建 {{ teamResult.experts?.length || 0 }} 人专家团队</span>
              <el-tag size="small" type="success">覆盖率 {{ Math.round((teamResult.coverage || 0) * 100) }}%</el-tag>
            </div>
            <div class="team-result__experts">
              <expert-card
                v-for="expert in teamResult.experts"
                :key="expert.expert_id"
                :expert="expert"
                :compact="true"
              />
            </div>
          </div>
        </div>

        <!-- Debate 阶段详情 -->
        <div v-if="currentPhase === 'debate' && debateResult" class="phase-pipeline__detail-content">
          <div class="debate-result">
            <div class="debate-result__header">
              <div class="debate-result__consensus">
                <span class="debate-result__label">共识度</span>
                <span class="debate-result__value" :style="{ color: getConsensusColor(debateResult.consensus) }">
                  {{ (debateResult.consensus * 100).toFixed(1) }}%
                </span>
              </div>
              <div class="debate-result__meta">
                <el-tag size="small">{{ debateResult.debate_rounds || 0 }} 轮辩论</el-tag>
                <el-tag size="small" type="info">{{ debateResult.opinions?.length || 0 }} 个观点</el-tag>
              </div>
            </div>
            <!-- 观点列表 -->
            <div class="debate-result__opinions">
              <div v-for="opinion in debateResult.opinions" :key="opinion.expert_id" class="opinion-item">
                <div class="opinion-item__header">
                  <span class="opinion-item__expert">{{ opinion.expert_id }}</span>
                  <div class="opinion-item__scores">
                    <el-tooltip content="分数" placement="top">
                      <span class="opinion-item__score">📊 {{ (opinion.score * 100).toFixed(0) }}</span>
                    </el-tooltip>
                    <el-tooltip content="置信度" placement="top">
                      <span class="opinion-item__confidence">🎯 {{ (opinion.confidence * 100).toFixed(0) }}</span>
                    </el-tooltip>
                  </div>
                </div>
                <div class="opinion-item__answer" v-html="formatAnswer(opinion.answer)"></div>
              </div>
            </div>
          </div>
        </div>

        <!-- Synthesize 阶段详情 -->
        <div v-if="currentPhase === 'synthesize' && synthesisResult" class="phase-pipeline__detail-content">
          <div class="synthesis-result">
            <div class="synthesis-result__markdown" v-html="formatMarkdown(synthesisResult.synthesis)"></div>
          </div>
        </div>

        <!-- Gate 阶段详情 -->
        <div v-if="currentPhase === 'gate' && gateResult" class="phase-pipeline__detail-content">
          <gate-result :result="gateResult" />
        </div>

        <!-- Learn 阶段详情 -->
        <div v-if="currentPhase === 'learn' && learnResult" class="phase-pipeline__detail-content">
          <div class="learn-result">
            <el-alert
              :title="`学习完成：累计 ${learnResult.learn_count || 0} 次学习`"
              type="success"
              :closable="false"
              show-icon
            />
          </div>
        </div>

        <!-- Done 阶段详情 -->
        <div v-if="currentPhase === 'done'" class="phase-pipeline__detail-content">
          <div class="done-result">
            <el-result icon="success" title="分析完成" :sub-title="`总耗时 ${formatLatency(totalLatency)}`">
              <template #extra>
                <el-button type="primary" @click="$emit('export')">导出报告</el-button>
                <el-button @click="$emit('restart')">重新分析</el-button>
              </template>
            </el-result>
          </div>
        </div>
      </div>
    </transition>

    <!-- 降级提示 -->
    <transition name="fade">
      <el-alert
        v-if="isDegraded"
        title="当前运行在降级模式"
        :description="degradeReason || 'LLM 服务不可用，已切换到本地规则模式'"
        type="warning"
        :closable="false"
        show-icon
        class="phase-pipeline__degraded"
      />
    </transition>
  </div>
</template>

<script setup>
import { computed, ref } from 'vue'
import ExpertCard from './ExpertCard.vue'
import GateResult from './GateResult.vue'

// ── Props ──────────────────────────────────────────────────────────────────

const props = defineProps({
  // 当前阶段
  currentPhase: {
    type: String,
    default: null
  },
  // 各阶段结果
  intentResult: { type: Object, default: null },
  teamResult: { type: Object, default: null },
  debateResult: { type: Object, default: null },
  synthesisResult: { type: Object, default: null },
  gateResult: { type: Object, default: null },
  learnResult: { type: Object, default: null },
  // 各阶段耗时
  phaseLatencies: {
    type: Object,
    default: () => ({})
  },
  // 是否运行中
  isRunning: {
    type: Boolean,
    default: false
  },
  // 是否降级
  isDegraded: {
    type: Boolean,
    default: false
  },
  // 降级原因
  degradeReason: {
    type: String,
    default: null
  }
})

// ── Emits ──────────────────────────────────────────────────────────────────

const emit = defineEmits(['phase-click', 'export', 'restart'])

// ── 阶段定义 ────────────────────────────────────────────────────────────────

const phases = [
  { key: 'intent', label: '意图识别', icon: '🎯' },
  { key: 'team', label: '组队匹配', icon: '👥' },
  { key: 'debate', label: '专家辩论', icon: '💬' },
  { key: 'synthesize', label: '综合归纳', icon: '📝' },
  { key: 'gate', label: '质量门禁', icon: '🚦' },
  { key: 'learn', label: '知识学习', icon: '🧠' },
  { key: 'done', label: '完成', icon: '✅' }
]

const phaseOrder = ['intent', 'team', 'debate', 'synthesize', 'gate', 'learn', 'done']

// ── 计算属性 ────────────────────────────────────────────────────────────────

const currentPhaseIndex = computed(() => {
  if (!props.currentPhase) return -1
  return phaseOrder.indexOf(props.currentPhase)
})

const currentPhaseDetail = computed(() => {
  if (!props.currentPhase) return null
  return phases.find(p => p.key === props.currentPhase)
})

const currentPhaseStatus = computed(() => {
  if (!props.currentPhase) return 'pending'
  if (props.isRunning && props.currentPhase !== 'done') return 'running'
  return 'done'
})

const statusText = computed(() => {
  switch (currentPhaseStatus.value) {
    case 'running': return '进行中...'
    case 'done': return '已完成'
    default: return '等待中'
  }
})

const totalLatency = computed(() => {
  return Object.values(props.phaseLatencies).reduce((sum, v) => sum + (v || 0), 0)
})

// ── 方法 ────────────────────────────────────────────────────────────────────

function getPhaseStatus(phaseKey) {
  const idx = phaseOrder.indexOf(phaseKey)
  if (currentPhaseIndex.value < 0) return 'pending'
  if (idx < currentPhaseIndex.value) return 'done'
  if (idx === currentPhaseIndex.value) {
    return props.isRunning && phaseKey !== 'done' ? 'running' : 'done'
  }
  return 'pending'
}

function getPhaseClass(phaseKey, index) {
  const status = getPhaseStatus(phaseKey)
  return {
    'phase-pipeline__node--done': status === 'done',
    'phase-pipeline__node--running': status === 'running',
    'phase-pipeline__node--pending': status === 'pending',
    'phase-pipeline__node--last': index === phases.length - 1
  }
}

function isPhasePast(phaseKey) {
  const idx = phaseOrder.indexOf(phaseKey)
  return idx < currentPhaseIndex.value
}

function getPhaseLatency(phaseKey) {
  return props.phaseLatencies[phaseKey] || 0
}

function handlePhaseClick(phaseKey) {
  emit('phase-click', phaseKey)
}

function formatLatency(ms) {
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
}

function getConsensusColor(consensus) {
  if (consensus >= 0.8) return '#10b981'
  if (consensus >= 0.6) return '#f59e0b'
  return '#ef4444'
}

function formatAnswer(answer) {
  if (!answer) return ''
  // 简单的 Markdown 转 HTML（标题、列表、加粗）
  return answer
    .replace(/^### (.*$)/gm, '<h4>$1</h4>')
    .replace(/^## (.*$)/gm, '<h3>$1</h3>')
    .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
    .replace(/^\d+\. (.*$)/gm, '<li>$1</li>')
    .replace(/^- (.*$)/gm, '<li>$1</li>')
    .replace(/\n/g, '<br>')
}

function formatMarkdown(text) {
  if (!text) return ''
  return formatAnswer(text)
}
</script>

<style scoped>
.phase-pipeline {
  width: 100%;
  padding: 16px;
  background: #fff;
  border-radius: 12px;
  border: 1px solid #e4e3dd;
}

.phase-pipeline--running {
  border-color: #6366f1;
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.1);
}

/* 阶段进度条 */
.phase-pipeline__track {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  position: relative;
  margin-bottom: 20px;
}

.phase-pipeline__node {
  display: flex;
  flex-direction: column;
  align-items: center;
  position: relative;
  flex: 1;
  cursor: pointer;
  transition: all 0.3s ease;
}

.phase-pipeline__node:hover {
  transform: translateY(-2px);
}

.phase-pipeline__icon {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: 600;
  background: #f4f3ee;
  color: #9ca3af;
  border: 2px solid #e4e3dd;
  transition: all 0.3s ease;
  z-index: 1;
}

.phase-pipeline__node--done .phase-pipeline__icon {
  background: #10b981;
  color: #fff;
  border-color: #10b981;
}

.phase-pipeline__node--running .phase-pipeline__icon {
  background: #6366f1;
  color: #fff;
  border-color: #6366f1;
  animation: pulse 1.5s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(99, 102, 241, 0.4); }
  50% { box-shadow: 0 0 0 8px rgba(99, 102, 241, 0); }
}

.phase-pipeline__spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.phase-pipeline__label {
  margin-top: 8px;
  text-align: center;
}

.phase-pipeline__name {
  font-size: 12px;
  font-weight: 500;
  color: #6b7280;
  white-space: nowrap;
}

.phase-pipeline__node--done .phase-pipeline__name,
.phase-pipeline__node--running .phase-pipeline__name {
  color: #1a1b1c;
}

.phase-pipeline__latency {
  font-size: 10px;
  color: #9ca3af;
  margin-top: 2px;
}

/* 连接线 */
.phase-pipeline__connector {
  position: absolute;
  top: 18px;
  left: 50%;
  width: 100%;
  height: 2px;
  background: #e4e3dd;
  z-index: 0;
}

.phase-pipeline__connector--active {
  background: #10b981;
}

/* 阶段详情 */
.phase-pipeline__detail {
  margin-top: 16px;
  padding: 16px;
  background: #f9fafb;
  border-radius: 8px;
  border: 1px solid #e4e3dd;
}

.phase-pipeline__detail-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid #e4e3dd;
}

.phase-pipeline__detail-icon {
  font-size: 18px;
}

.phase-pipeline__detail-title {
  font-size: 15px;
  font-weight: 600;
  color: #1a1b1c;
  flex: 1;
}

.phase-pipeline__detail-status {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 4px;
}

.status--running {
  background: #eef2ff;
  color: #6366f1;
}

.status--done {
  background: #ecfdf5;
  color: #10b981;
}

/* Intent 结果 */
.intent-result__row {
  display: flex;
  align-items: center;
  margin-bottom: 8px;
}

.intent-result__label {
  font-size: 13px;
  color: #6b7280;
  min-width: 80px;
}

.intent-result__value {
  font-size: 13px;
  color: #1a1b1c;
}

.intent-result__value--primary {
  font-weight: 600;
  color: #6366f1;
}

.intent-result__tags {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
}

/* Team 结果 */
.team-result__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
  font-size: 13px;
  color: #6b7280;
}

.team-result__experts {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 8px;
}

/* Debate 结果 */
.debate-result__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.debate-result__consensus {
  display: flex;
  align-items: center;
  gap: 8px;
}

.debate-result__label {
  font-size: 13px;
  color: #6b7280;
}

.debate-result__value {
  font-size: 20px;
  font-weight: 700;
}

.debate-result__meta {
  display: flex;
  gap: 8px;
}

.debate-result__opinions {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.opinion-item {
  padding: 12px;
  background: #fff;
  border-radius: 8px;
  border: 1px solid #e4e3dd;
}

.opinion-item__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.opinion-item__expert {
  font-size: 13px;
  font-weight: 600;
  color: #6366f1;
}

.opinion-item__scores {
  display: flex;
  gap: 12px;
}

.opinion-item__score,
.opinion-item__confidence {
  font-size: 12px;
  color: #6b7280;
}

.opinion-item__answer {
  font-size: 13px;
  color: #374151;
  line-height: 1.6;
}

.opinion-item__answer :deep(h4) {
  font-size: 13px;
  font-weight: 600;
  margin: 8px 0 4px;
  color: #1a1b1c;
}

.opinion-item__answer :deep(li) {
  margin-left: 16px;
  margin-bottom: 2px;
}

/* Synthesis 结果 */
.synthesis-result__markdown {
  font-size: 13px;
  line-height: 1.7;
  color: #374151;
}

.synthesis-result__markdown :deep(h3) {
  font-size: 15px;
  font-weight: 600;
  margin: 16px 0 8px;
  color: #1a1b1c;
}

.synthesis-result__markdown :deep(h4) {
  font-size: 14px;
  font-weight: 600;
  margin: 12px 0 6px;
  color: #1a1b1c;
}

.synthesis-result__markdown :deep(li) {
  margin-left: 20px;
  margin-bottom: 4px;
}

.synthesis-result__markdown :deep(strong) {
  color: #1a1b1c;
}

/* Done 结果 */
.done-result {
  text-align: center;
}

/* 降级提示 */
.phase-pipeline__degraded {
  margin-top: 12px;
}

/* 过渡动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* 响应式 */
@media (max-width: 768px) {
  .phase-pipeline__name {
    font-size: 10px;
  }

  .phase-pipeline__icon {
    width: 28px;
    height: 28px;
    font-size: 12px;
  }

  .team-result__experts {
    grid-template-columns: 1fr;
  }
}
</style>
