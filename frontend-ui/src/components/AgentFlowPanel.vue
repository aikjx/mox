<template>
  <div class="agent-flow-panel" :class="{ collapsed: collapsed }">
    <!-- 面板头部 -->
    <div class="afp-header">
      <div class="afp-title">
        <span class="afp-logo"><el-icon><Cpu /></el-icon></span>
        <div class="afp-title-text">
          <div class="afp-name">Agent 处理流程</div>
          <div class="afp-sub">实时 · 对话联动</div>
        </div>
      </div>
      <div class="afp-header-actions">
        <el-tooltip content="收起面板" placement="left">
          <button class="afp-icon-btn" @click="collapsed = !collapsed">
            <el-icon><component :is="collapsed ? Expand : Fold" /></el-icon>
          </button>
        </el-tooltip>
      </div>
    </div>

    <template v-if="!collapsed">
      <!-- 视图切换：流程图 / 代码 DSL / 明细 -->
      <div class="afp-tabs">
        <button
          v-for="t in viewTabs"
          :key="t.key"
          class="afp-tab"
          :class="{ active: view === t.key }"
          @click="view = t.key"
        >
          <el-icon><component :is="t.icon" /></el-icon>
          <span>{{ t.label }}</span>
          <span v-if="t.key === 'flow' && agents.length" class="afp-tab-count">{{ agents.length }}</span>
        </button>
      </div>

      <div class="afp-body">
        <!-- ===== 视图1：Agent 处理流程图 ===== -->
        <div v-show="view === 'flow'" class="afp-flow">
          <!-- 空态 -->
          <div v-if="!agents.length" class="afp-empty">
            <div class="empty-icon"><el-icon><Share /></el-icon></div>
            <div class="empty-title">等待任务下发</div>
            <div class="empty-desc">在左侧对话中描述目标，AI 将拆解为 Agent 处理流程并在此实时展示</div>
          </div>

          <!-- 流程图：Agent 节点链 -->
          <div v-else class="afp-graph">
            <div
              v-for="(a, idx) in agents"
              :key="a.id"
              class="agent-node"
              :class="[a.status, { last: idx === agents.length - 1 }]"
            >
              <div class="node-line">
                <div class="node-dot">
                  <el-icon v-if="a.status === 'done'"><Check /></el-icon>
                  <el-icon v-else-if="a.status === 'running'" class="spin"><Loading /></el-icon>
                  <el-icon v-else-if="a.status === 'error'"><Close /></el-icon>
                  <span v-else>{{ idx + 1 }}</span>
                </div>
                <div v-if="idx < agents.length - 1" class="node-connector">
                  <div class="connector-line"></div>
                  <div class="connector-arrow"><el-icon><ArrowDown /></el-icon></div>
                </div>
              </div>
              <div class="node-card" :class="a.status">
                <div class="node-card-head">
                  <div class="node-agent-icon" :style="{ background: a.gradient }">{{ a.emoji }}</div>
                  <div class="node-agent-meta">
                    <div class="node-agent-name">{{ a.name }}</div>
                    <div class="node-agent-role">{{ a.role }}</div>
                  </div>
                  <el-tag
                    size="small"
                    class="node-status-tag"
                    :type="statusTagType(a.status)"
                    effect="light"
                  >{{ statusText(a.status) }}</el-tag>
                </div>
                <div class="node-desc" v-if="a.desc">{{ a.desc }}</div>
                <div class="node-tools" v-if="a.tools && a.tools.length">
                  <span v-for="t in a.tools" :key="t" class="node-tool">{{ t }}</span>
                </div>
                <div class="node-result" v-if="a.result && a.status === 'done'">
                  <div class="node-result-label">输出</div>
                  <div class="node-result-text">{{ a.result }}</div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- ===== 视图2：代码 / DSL 视图（豆包式右侧代码） ===== -->
        <div v-show="view === 'code'" class="afp-code">
          <div v-if="!codeText" class="afp-empty">
            <div class="empty-icon"><el-icon><Document /></el-icon></div>
            <div class="empty-title">暂无 DSL 定义</div>
            <div class="empty-desc">任务执行后，将在此生成对应的 Agent 编排 DSL（YAML）</div>
          </div>
          <template v-else>
            <div class="code-toolbar">
              <span class="code-filename"><el-icon><Document /></el-icon> agent-flow.yaml</span>
              <button class="code-copy" @click="copyCode">
                <el-icon><CopyDocument /></el-icon> 复制
              </button>
            </div>
            <pre class="code-block" ref="codeRef"><code>{{ codeText }}</code></pre>
          </template>
        </div>

        <!-- ===== 视图3：执行明细 ===== -->
        <div v-show="view === 'detail'" class="afp-detail">
          <div class="detail-summary" v-if="agents.length">
            <div class="d-summary-item">
              <div class="d-summary-num">{{ doneCount }}</div>
              <div class="d-summary-label">已完成</div>
            </div>
            <div class="d-summary-item">
              <div class="d-summary-num running">{{ runningCount }}</div>
              <div class="d-summary-label">进行中</div>
            </div>
            <div class="d-summary-item">
              <div class="d-summary-num error">{{ errorCount }}</div>
              <div class="d-summary-label">异常</div>
            </div>
            <div class="d-summary-item">
              <div class="d-summary-num">{{ agents.length }}</div>
              <div class="d-summary-label">Agent 数</div>
            </div>
          </div>
          <div class="detail-list" v-if="agents.length">
            <div v-for="a in agents" :key="'d-'+a.id" class="detail-row">
              <span class="d-emoji">{{ a.emoji }}</span>
              <span class="d-name">{{ a.name }}</span>
              <span class="d-role">{{ a.role }}</span>
              <span class="d-cost" v-if="a.cost">{{ a.cost }}</span>
            </div>
          </div>
          <div v-else class="afp-empty">
            <div class="empty-icon"><el-icon><List /></el-icon></div>
            <div class="empty-title">暂无执行记录</div>
          </div>
        </div>
      </div>

      <!-- 面板底部：全局状态 -->
      <div class="afp-footer" v-if="agents.length">
        <div class="footer-status" :class="overallStatus">
          <span class="footer-dot"></span>
          {{ overallStatusText }}
        </div>
        <div class="footer-cost" v-if="totalCost">{{ totalCost }}</div>
      </div>
    </template>
  </div>
</template>

<script setup>
import { ref, computed, watch, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Cpu, Expand, Fold, Share, Check, Close, Loading, ArrowDown,
  Document, CopyDocument, List
} from '@element-plus/icons-vue'

const props = defineProps({
  // Agent 流程数据：{ id, name, role, emoji, gradient, status, desc, tools, result, cost }
  agents: { type: Array, default: () => [] },
  // 外部控制收起状态（可选）
  modelValue: { type: Boolean, default: true }
})
const emit = defineEmits(['update:modelValue'])

const collapsed = ref(!props.modelValue)
const view = ref('flow')
const codeRef = ref(null)

const viewTabs = [
  { key: 'flow', label: '流程图', icon: Share },
  { key: 'code', label: '代码 DSL', icon: Document },
  { key: 'detail', label: '明细', icon: List }
]

/* ===== 由 agents 派生 DSL 代码 ===== */
const codeText = computed(() => {
  if (!props.agents.length) return ''
  const lines = []
  lines.push('# MOX Agent Flow DSL · 由对话自动生成')
  lines.push('version: 1.0')
  lines.push('name: agent-execution-flow')
  lines.push('agents:')
  props.agents.forEach((a, i) => {
    lines.push(`  - id: agent_${i + 1}`)
    lines.push(`    name: "${a.name}"`)
    lines.push(`    role: "${a.role}"`)
    if (a.tools && a.tools.length) {
      lines.push(`    tools: [${a.tools.map(t => `"${t}"`).join(', ')}]`)
    }
    lines.push(`    on: ${a.status}`)
    if (i < props.agents.length - 1) {
      lines.push(`    next: agent_${i + 2}`)
    } else {
      lines.push(`    next: null`)
    }
  })
  lines.push('policy:')
  lines.push('  human_in_the_loop: high_risk_only')
  lines.push('  audit: enabled')
  return lines.join('\n')
})

const doneCount = computed(() => props.agents.filter(a => a.status === 'done').length)
const runningCount = computed(() => props.agents.filter(a => a.status === 'running').length)
const errorCount = computed(() => props.agents.filter(a => a.status === 'error').length)
const totalCost = computed(() => {
  const hasCost = props.agents.some(a => a.cost)
  if (!hasCost) return ''
  const total = props.agents.reduce((s, a) => s + (parseFloat(a.cost) || 0), 0)
  return `≈ ¥${total.toFixed(4)}`
})
const overallStatus = computed(() => {
  if (errorCount.value) return 'error'
  if (runningCount.value) return 'running'
  if (props.agents.length && doneCount.value === props.agents.length) return 'done'
  return 'pending'
})
const overallStatusText = computed(() => {
  if (overallStatus.value === 'error') return `存在 ${errorCount.value} 个异常 Agent`
  if (overallStatus.value === 'running') return `正在执行 ${runningCount.value} 个 Agent`
  if (overallStatus.value === 'done') return '全部 Agent 执行完成'
  return '流程待启动'
})

function statusTagType(s) {
  if (s === 'done') return 'success'
  if (s === 'running') return 'primary'
  if (s === 'error') return 'danger'
  return 'info'
}
function statusText(s) {
  if (s === 'done') return '完成'
  if (s === 'running') return '执行中'
  if (s === 'error') return '异常'
  return '待执行'
}

/* 收起状态同步到父组件 */
watch(collapsed, (v) => emit('update:modelValue', !v))
watch(() => props.modelValue, (v) => { collapsed.value = !v })

/* 任务完成后自动切到代码视图（像豆包一样展示 DSL） */
watch(doneCount, async (v) => {
  if (props.agents.length && v === props.agents.length) {
    await nextTick()
    if (view.value !== 'code') view.value = 'code'
  }
})

function copyCode() {
  if (!codeText.value) return
  navigator.clipboard.writeText(codeText.value)
  ElMessage.success('DSL 已复制')
}
</script>

<style scoped>
/* ===== 面板容器 ===== */
.agent-flow-panel {
  width: 340px;
  min-width: 340px;
  max-width: 400px;
  background: var(--bg-surface-2, #fafbfe);
  border-left: 1px solid var(--border-soft, rgba(15, 23, 42, 0.09));
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  transition: width 0.3s cubic-bezier(0.22, 1, 0.36, 1);
  position: relative;
  z-index: 5;
}
.agent-flow-panel.collapsed {
  width: 0;
  min-width: 0;
  overflow: hidden;
  border-left-color: transparent;
}

/* ===== 头部 ===== */
.afp-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 14px 10px;
  min-height: 56px;
  border-bottom: 1px solid var(--border-soft, rgba(15, 23, 42, 0.07));
}
.afp-title { display: flex; align-items: center; gap: 10px; }
.afp-logo {
  width: 34px; height: 34px;
  border-radius: 10px;
  background: linear-gradient(135deg, var(--brand, #6366f1), var(--accent, #06b6d4));
  display: grid; place-items: center;
  color: #fff; font-size: 16px;
  flex-shrink: 0;
}
.afp-title-text { min-width: 0; }
.afp-name {
  font-size: 13px; font-weight: 700;
  color: var(--text-primary, #0b1120);
  letter-spacing: -0.2px;
}
.afp-sub { font-size: 10px; color: var(--text-tertiary, #64748b); margin-top: 2px; }
.afp-header-actions { display: flex; gap: 6px; }
.afp-icon-btn {
  width: 28px; height: 28px;
  border: none; background: transparent;
  border-radius: 7px;
  display: grid; place-items: center;
  color: var(--text-tertiary, #64748b);
  cursor: pointer;
  transition: all 0.2s;
}
.afp-icon-btn:hover { background: rgba(99, 102, 241, 0.1); color: var(--brand, #6366f1); }

/* ===== 视图切换 Tabs ===== */
.afp-tabs {
  display: flex;
  gap: 4px;
  padding: 10px 14px 0;
}
.afp-tab {
  flex: 1;
  display: flex; align-items: center; justify-content: center;
  gap: 5px;
  padding: 8px 4px;
  border: none;
  border-radius: 8px;
  background: transparent;
  font-size: 11px;
  color: var(--text-tertiary, #64748b);
  cursor: pointer;
  transition: all 0.2s;
}
.afp-tab:hover { background: rgba(99, 102, 241, 0.06); color: var(--brand, #6366f1); }
.afp-tab.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.12), rgba(6, 182, 212, 0.12));
  color: var(--brand, #6366f1);
  font-weight: 600;
}
.afp-tab-count {
  min-width: 16px; height: 16px;
  padding: 0 4px;
  border-radius: 8px;
  background: var(--brand, #6366f1);
  color: #fff;
  font-size: 9px;
  display: grid; place-items: center;
}

/* ===== 主体 ===== */
.afp-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px 14px;
}
.afp-body::-webkit-scrollbar { width: 4px; }
.afp-body::-webkit-scrollbar-thumb { background: rgba(99, 102, 241, 0.2); border-radius: 4px; }

/* 空态 */
.afp-empty {
  display: flex; flex-direction: column; align-items: center;
  justify-content: center;
  padding: 60px 20px;
  text-align: center;
}
.empty-icon {
  width: 56px; height: 56px;
  border-radius: 18px;
  background: var(--bg-surface-3, #f1f5f9);
  display: grid; place-items: center;
  color: var(--text-quaternary, #94a3b8);
  font-size: 24px;
  margin-bottom: 14px;
}
.empty-title {
  font-size: 13px; font-weight: 600;
  color: var(--text-secondary, #334155);
  margin-bottom: 6px;
}
.empty-desc {
  font-size: 11px;
  color: var(--text-tertiary, #64748b);
  line-height: 1.7;
  max-width: 220px;
}

/* ===== 流程图 ===== */
.afp-graph { display: flex; flex-direction: column; }
.agent-node { display: flex; gap: 12px; position: relative; }
.node-line { display: flex; flex-direction: column; align-items: center; flex-shrink: 0; }
.node-dot {
  width: 26px; height: 26px;
  border-radius: 50%;
  background: var(--bg-surface-3, #e2e8f0);
  border: 2px solid var(--text-quaternary, #94a3b8);
  display: grid; place-items: center;
  font-size: 11px; font-weight: 700;
  color: var(--text-quaternary, #94a3b8);
  z-index: 1;
  transition: all 0.3s;
}
.node-dot.running, .agent-node.running .node-dot {
  background: linear-gradient(135deg, var(--brand, #6366f1), var(--accent, #06b6d4));
  border-color: transparent;
  color: #fff;
  box-shadow: 0 0 0 4px rgba(99, 102, 241, 0.15);
}
.node-dot.done, .agent-node.done .node-dot {
  background: #10b981;
  border-color: transparent;
  color: #fff;
}
.node-dot.error, .agent-node.error .node-dot {
  background: #ef4444;
  border-color: transparent;
  color: #fff;
}
.node-connector { display: flex; flex-direction: column; align-items: center; flex: 1; }
.connector-line {
  width: 2px;
  flex: 1;
  min-height: 24px;
  background: linear-gradient(180deg, var(--text-quaternary, #94a3b8), rgba(99, 102, 241, 0.4));
}
.connector-arrow { color: var(--text-quaternary, #94a3b8); font-size: 12px; margin: 2px 0; }

.node-card {
  flex: 1;
  min-width: 0;
  margin-bottom: 14px;
  padding: 12px;
  border-radius: 12px;
  background: var(--bg-surface, #fff);
  border: 1px solid var(--border-soft, rgba(15, 23, 42, 0.09));
  transition: all 0.25s cubic-bezier(0.22, 1, 0.36, 1);
}
.node-card.running {
  border-color: var(--brand, #6366f1);
  box-shadow: 0 6px 20px rgba(99, 102, 241, 0.14);
}
.node-card.done { border-color: rgba(16, 185, 129, 0.35); }
.node-card.error { border-color: rgba(239, 68, 68, 0.45); background: rgba(239, 68, 68, 0.03); }
.node-card-head { display: flex; align-items: center; gap: 10px; }
.node-agent-icon {
  width: 34px; height: 34px;
  border-radius: 10px;
  display: grid; place-items: center;
  font-size: 17px;
  flex-shrink: 0;
}
.node-agent-meta { flex: 1; min-width: 0; }
.node-agent-name {
  font-size: 12px; font-weight: 700;
  color: var(--text-primary, #0b1120);
}
.node-agent-role {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  margin-top: 2px;
}
.node-status-tag { flex-shrink: 0; }
.node-desc {
  font-size: 11px;
  color: var(--text-secondary, #334155);
  line-height: 1.6;
  margin-top: 10px;
}
.node-tools {
  display: flex; flex-wrap: wrap; gap: 6px;
  margin-top: 10px;
}
.node-tool {
  font-size: 10px;
  padding: 3px 8px;
  border-radius: 6px;
  background: var(--bg-surface-3, #f1f5f9);
  color: var(--text-tertiary, #64748b);
}
.node-result {
  margin-top: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(16, 185, 129, 0.08);
  border: 1px solid rgba(16, 185, 129, 0.2);
}
.node-result-label { font-size: 10px; font-weight: 600; color: #059669; margin-bottom: 3px; }
.node-result-text { font-size: 11px; color: var(--text-secondary, #334155); line-height: 1.6; }

/* ===== 代码视图 ===== */
.afp-code { display: flex; flex-direction: column; }
.code-toolbar {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-deep, #0b1120);
  border-radius: 10px 10px 0 0;
}
.code-filename {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px;
  color: #94a3b8;
  font-family: 'SF Mono', 'Cascadia Code', Consolas, monospace;
}
.code-copy {
  display: flex; align-items: center; gap: 4px;
  border: 1px solid rgba(148, 163, 184, 0.3);
  background: transparent;
  color: #cbd5e1;
  font-size: 10px;
  padding: 4px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}
.code-copy:hover { background: rgba(99, 102, 241, 0.2); color: #fff; }
.code-block {
  margin: 0;
  padding: 12px;
  background: var(--bg-deep, #0b1120);
  border-radius: 0 0 10px 10px;
  overflow-x: auto;
  font-family: 'SF Mono', 'Cascadia Code', Consolas, monospace;
  font-size: 11px;
  line-height: 1.7;
  color: #a5b4fc;
  white-space: pre;
}
.code-block::-webkit-scrollbar { height: 4px; }
.code-block::-webkit-scrollbar-thumb { background: rgba(99, 102, 241, 0.3); border-radius: 4px; }

/* ===== 明细视图 ===== */
.afp-detail { display: flex; flex-direction: column; }
.detail-summary {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 14px;
}
.d-summary-item {
  padding: 10px 6px;
  border-radius: 10px;
  background: var(--bg-surface, #fff);
  border: 1px solid var(--border-soft, rgba(15, 23, 42, 0.09));
  text-align: center;
}
.d-summary-num {
  font-size: 18px; font-weight: 800;
  color: var(--brand, #6366f1);
}
.d-summary-num.running { color: #06b6d4; }
.d-summary-num.error { color: #ef4444; }
.d-summary-label {
  font-size: 10px;
  color: var(--text-tertiary, #64748b);
  margin-top: 3px;
}
.detail-list { display: flex; flex-direction: column; gap: 8px; }
.detail-row {
  display: flex; align-items: center; gap: 10px;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--bg-surface, #fff);
  border: 1px solid var(--border-soft, rgba(15, 23, 42, 0.09));
}
.d-emoji { font-size: 16px; }
.d-name {
  font-size: 12px; font-weight: 600;
  color: var(--text-primary, #0b1120);
  flex: 1;
}
.d-role { font-size: 10px; color: var(--text-tertiary, #64748b); }
.d-cost { font-size: 10px; color: var(--brand, #6366f1); font-weight: 600; }

/* ===== 底部 ===== */
.afp-footer {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 14px;
  border-top: 1px solid var(--border-soft, rgba(15, 23, 42, 0.07));
  font-size: 11px;
}
.footer-status { display: flex; align-items: center; gap: 6px; color: var(--text-tertiary, #64748b); }
.footer-dot {
  width: 7px; height: 7px; border-radius: 50%;
  background: #94a3b8;
}
.footer-status.running .footer-dot { background: var(--brand, #6366f1); animation: pulse 1.5s infinite; }
.footer-status.done .footer-dot { background: #10b981; }
.footer-status.error .footer-dot { background: #ef4444; }
.footer-cost { color: var(--brand, #6366f1); font-weight: 600; }

.spin { animation: spin 1s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}
</style>
