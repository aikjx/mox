<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">算子中心</h2>
        <p class="page-subtitle">管理标准算子与自定义算子，编排并执行算子工作流链</p>
      </div>
      <div class="page-header-actions">
        <el-button type="primary" @click="goAIGenerate">
          <el-icon><Promotion /></el-icon> AI生成算子
        </el-button>
        <el-button type="primary" plain @click="showRegister = true">
          <el-icon><Plus /></el-icon> 注册算子
        </el-button>
      </div>
    </div>

    <div class="page-content">

    <!-- 操作流程引导：4步完成算子编排 -->
    <div class="flow-guide">
      <div class="flow-step" :class="{ active: currentStep >= 1, done: currentStep > 1 }">
        <div class="step-num">1</div>
        <div class="step-info">
          <div class="step-title">选择算子</div>
          <div class="step-desc">从左侧算子库点击加入</div>
        </div>
      </div>
      <div class="step-arrow" :class="{ active: currentStep > 1 }">→</div>
      <div class="flow-step" :class="{ active: currentStep >= 2, done: currentStep > 2 }">
        <div class="step-num">2</div>
        <div class="step-info">
          <div class="step-title">编排链路</div>
          <div class="step-desc">调整顺序，点击×移除</div>
        </div>
      </div>
      <div class="step-arrow" :class="{ active: currentStep > 2 }">→</div>
      <div class="flow-step" :class="{ active: currentStep >= 3, done: currentStep > 3 }">
        <div class="step-num">3</div>
        <div class="step-info">
          <div class="step-title">配置参数</div>
          <div class="step-desc">输入向量与缩放因子</div>
        </div>
      </div>
      <div class="step-arrow" :class="{ active: currentStep > 3 }">→</div>
      <div class="flow-step" :class="{ active: currentStep >= 4, done: currentStep > 4 }">
        <div class="step-num">4</div>
        <div class="step-info">
          <div class="step-title">执行查看</div>
          <div class="step-desc">点击执行，查看结果</div>
        </div>
      </div>
      <el-button v-if="!selectedOrder.length" size="small" type="primary" plain @click="loadDemo" class="demo-btn">
        ⚡ 一键示例
      </el-button>
    </div>

    <div class="grid grid-2 main-grid">
      <!-- 算子库 -->
      <div class="panel card-pad" v-loading="opsLoading" element-loading-text="算子库加载中...">`r`n        <h3 class="section-title">算子库（{{ operators.length }}）</h3>
        <div class="cats">
          <span
            v-for="c in OPERATOR_CATEGORIES"
            :key="c.key"
            class="cat"
            :class="{ on: cat === c.key }"
            @click="cat = c.key"
          >{{ c.label }}</span>
        </div>
        <el-input
          v-model="kw"
          placeholder="搜索算子名称 / 描述"
          clearable
          :prefix-icon="Search"
          style="margin-bottom: 12px"
        />
        <el-scrollbar height="360px">
          <div
            v-for="op in filtered"
            :key="op.id"
            class="op-item"
            :class="{ sel: selected.has(op.id) }"
            @click="toggle(op.id)"
          >
            <div class="op-check" :class="{ on: selected.has(op.id) }">
              <el-icon v-if="selected.has(op.id)"><Check /></el-icon>
            </div>
            <div class="op-body">
              <div class="op-name">
                {{ op.name }}
                <span class="badge primary">{{ op.category }}</span>
              </div>
              <div class="op-desc">{{ op.description }}</div>
            </div>
            <el-icon class="op-detail" @click.stop="openDetail(op)"><InfoFilled /></el-icon>
          </div>
          <el-empty v-if="!filtered.length" description="未找到算子" :image-size="60" />
        </el-scrollbar>
      </div>

      <!-- 执行台 -->
      <div class="panel card-pad">
        <h3 class="section-title">工作流执行台</h3>
        <div class="chain">
          <span class="chain-label">已选链路：</span>
          <template v-if="selectedOrder.length">
            <span class="chip" v-for="(id, i) in selectedOrder" :key="id">
              {{ nameOf(id) }}
              <el-icon class="chip-x" @click="removeAt(i)"><Close /></el-icon>
              <span v-if="i < selectedOrder.length - 1" class="arrow">→</span>
            </span>
          </template>
          <template v-else>
            <div class="empty-guide">
              <div class="empty-icon">👆</div>
              <div class="empty-text">点击左侧算子卡片，加入执行链路</div>
              <div class="empty-hint">或点击上方「⚡ 一键示例」快速体验</div>
            </div>
          </template>
        </div>

        <el-form label-width="92px" class="exec-form">
          <el-form-item label="输入向量">
            <el-input
              v-model="inputVec"
              placeholder="例如：1,2,3,4"
              :prefix-icon="Coordinate"
            />
          </el-form-item>
          <el-form-item label="缩放因子">
            <el-slider v-model="scale" :min="0.1" :max="5" :step="0.1" show-input />
          </el-form-item>
          <el-button
            type="primary"
            :loading="running"
            :disabled="!selectedOrder.length"
            @click="run"
            style="width: 100%"
          >
            <el-icon><VideoPlay /></el-icon> 执行工作流
          </el-button>
        </el-form>

        <div v-if="result" class="result">
          <div class="result-head">
            <span class="badge" :class="result.success ? 'success' : 'warning'">
              {{ result.success ? '执行成功' : '执行失败' }}
            </span>
            <span class="muted">{{ result.execution_time_ms }} ms</span>
          </div>
          <div v-if="result.error" class="err">{{ result.error }}</div>
          <div v-if="result.output" class="out-vec">
            <span class="muted">输出：</span>{{ fmtVec(result.output) }}
          </div>
          <div v-if="result.metrics" class="metrics">
            <div>输入范数：{{ result.metrics.input_norm?.toFixed(4) }}</div>
            <div>输出范数：{{ result.metrics.output_norm?.toFixed(4) }}</div>
            <div>残差：{{ result.metrics.l1_residual?.toFixed(4) }}</div>
          </div>
          <div v-if="result.input && result.output" ref="cmpEl" class="cmp-chart"></div>
          <div v-if="result.logs?.length" class="logs">
            <div v-for="(l, i) in result.logs" :key="i" class="log-line">{{ l }}</div>
          </div>
        </div>
      </div>
    </div>
    </div>

    <!-- 算子详情抽屉 -->
    <el-drawer v-model="showDetail" :title="detail?.name || '算子详情'" size="420px">
      <div v-if="detail" class="detail">
        <div class="detail-badge">
          <span class="badge primary">{{ detail.category }}</span>
          <span class="badge info">ID: {{ detail.id }}</span>
        </div>
        <p class="detail-desc">{{ detail.description }}</p>
        <el-divider>元数据</el-divider>
        <div class="kv"><span>算子类型</span><b>{{ detail.operator_type || '—' }}</b></div>
        <div class="kv"><span>参数数</span><b>{{ detail.parameters?.length || 0 }}</b></div>
        <div class="kv"><span>状态</span><b class="ok">可用</b></div>
        <el-button type="primary" class="detail-run" @click="quickAdd(detail)">
          <el-icon><Plus /></el-icon> 加入执行链
        </el-button>
      </div>
    </el-drawer>

    <!-- 注册弹窗 -->
    <el-dialog v-model="showRegister" title="注册自定义算子" width="480px">
      <el-form label-width="90px">
        <el-form-item label="算子 ID">
          <el-input v-model="reg.id" placeholder="唯一标识，如 my_op" />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="reg.name" placeholder="展示名称" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="reg.operator_type" style="width: 100%">
            <el-option label="function" value="function" />
            <el-option label="linear" value="linear" />
            <el-option label="custom" value="custom" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showRegister = false">取消</el-button>
        <el-button type="primary" :loading="reging" @click="doRegister">注册</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, nextTick, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import * as echarts from '@/echarts'
import { ElMessage } from 'element-plus'
import { Search, Coordinate, InfoFilled, Plus, Promotion } from '@element-plus/icons-vue'
import { OPERATOR_CATEGORIES } from '@/types'
import { getOperators, registerOperator, executeWorkflow } from '@/api'

const router = useRouter()

// AI生成算子：跳转到AI助手，带上算子上下文
function goAIGenerate() {
  router.push({ path: '/ai', query: { source: 'operators', action: 'generate' } })
}

const operators = ref([])
const kw = ref('')
const cat = ref('all')
const selected = ref(new Set())
const selectedOrder = ref([])
const inputVec = ref('1,2,3,4')
const scale = ref(2.0)
const running = ref(false)
const opsLoading = ref(false)
const result = ref(null)
const cmpEl = ref(null)
let cmpChart = null

const showRegister = ref(false)
const reging = ref(false)
const reg = ref({ id: '', name: '', operator_type: 'function' })

const showDetail = ref(false)
const detail = ref(null)

const filtered = computed(() => {
  const k = kw.value.trim().toLowerCase()
  return operators.value.filter((o) => {
    const matchK = !k || o.name.toLowerCase().includes(k) || (o.description || '').toLowerCase().includes(k)
    const matchC = cat.value === 'all' || o.category === cat.value
    return matchK && matchC
  })
})

// 操作流程当前步骤：根据用户操作自动判断
const currentStep = computed(() => {
  if (result.value) return 4
  if (selectedOrder.value.length > 0 && inputVec.value.trim()) return 3
  if (selectedOrder.value.length > 0) return 2
  return 1
})

// 一键加载示例链路
function loadDemo() {
  const demoOps = operators.value.slice(0, 3)
  demoOps.forEach((op) => {
    if (!selected.value.has(op.id)) {
      selected.value.add(op.id)
      selectedOrder.value.push(op.id)
    }
  })
  selected.value = new Set(selected.value)
  inputVec.value = '1,2,3,4'
  scale.value = 2.0
  ElMessage.success('已加载示例链路，点击「执行工作流」查看效果')
}

function nameOf(id) {
  return operators.value.find((o) => o.id === id)?.name || id
}
function toggle(id) {
  if (selected.value.has(id)) {
    selected.value.delete(id)
    selectedOrder.value = selectedOrder.value.filter((x) => x !== id)
  } else {
    selected.value.add(id)
    selectedOrder.value.push(id)
  }
  selected.value = new Set(selected.value)
}
function removeAt(i) {
  const id = selectedOrder.value[i]
  selectedOrder.value.splice(i, 1)
  selected.value.delete(id)
  selected.value = new Set(selected.value)
}
function fmtVec(v) {
  return Array.isArray(v) ? v.map((x) => (typeof x === 'number' ? x.toFixed(3) : x)).join(', ') : ''
}

async function run() {
  running.value = true
  result.value = null
  try {
    const input = inputVec.value
      .split(',')
      .map((s) => parseFloat(s.trim()))
      .filter((n) => !isNaN(n))
    const res = await executeWorkflow({
      workflow: selectedOrder.value,
      input,
      parameters: { scale: scale.value, factor: scale.value }
    })
    result.value = res
    ElMessage.success('执行完成')
    await nextTick()
    renderCmp(res)
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    running.value = false
  }
}

async function doRegister() {
  if (!reg.value.id || !reg.value.name) {
    ElMessage.warning('请填写 ID 与名称')
    return
  }
  reging.value = true
  try {
    await registerOperator(reg.value)
    ElMessage.success('注册成功')
    showRegister.value = false
    reg.value = { id: '', name: '', operator_type: 'function' }
    await loadOps()
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    reging.value = false
  }
}

async function loadOps() {
  opsLoading.value = true
  try {
    operators.value = await getOperators()
  } catch (e) {
    ElMessage.error('算子列表加载失败：' + e.message)
  } finally {
    opsLoading.value = false
  }
}

function openDetail(op) {
  detail.value = op
  showDetail.value = true
}
function quickAdd(op) {
  toggle(op.id)
  showDetail.value = false
}

function renderCmp(res) {
  if (!res.input || !res.output) return
  if (!cmpChart) cmpChart = echarts.init(cmpEl.value)
  const labels = res.input.map((_, i) => 'x' + (i + 1))
  const maxLen = Math.max(res.input.length, res.output.length)
  const input = [...res.input, ...Array(Math.max(0, maxLen - res.input.length)).fill(0)]
  const output = [...res.output, ...Array(Math.max(0, maxLen - res.output.length)).fill(0)]
  cmpChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['输入', '输出'], top: 0 },
    grid: { left: 40, right: 16, top: 32, bottom: 24 },
    xAxis: { type: 'category', data: labels, axisLabel: { color: '#94a3b8' } },
    yAxis: { type: 'value', axisLabel: { color: '#94a3b8' }, splitLine: { lineStyle: { color: '#f1f5f9' } } },
    series: [
      { name: '输入', type: 'bar', data: input, itemStyle: { color: '#94a3b8', borderRadius: [4, 4, 0, 0] }, barWidth: '32%' },
      { name: '输出', type: 'bar', data: output, itemStyle: { color: '#6366f1', borderRadius: [4, 4, 0, 0] }, barWidth: '32%' }
    ]
  })
}

function resize() {
  cmpChart && cmpChart.resize()
}
window.addEventListener('resize', resize)

onMounted(loadOps)
onBeforeUnmount(() => {
  window.removeEventListener('resize', resize)
  cmpChart && cmpChart.dispose()
})
</script>

<style scoped>
.ops {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
/* 操作流程引导 */
.flow-guide {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 18px;
  background: linear-gradient(135deg, rgba(79,70,229,0.06), rgba(124,58,237,0.04));
  border: 1px solid rgba(79,70,229,0.15);
  border-radius: 12px;
  flex-wrap: wrap;
}
.flow-step {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px;
  border-radius: 8px;
  opacity: 0.45;
  transition: all 0.25s;
}
.flow-step.active {
  opacity: 1;
  background: rgba(79,70,229,0.1);
}
.flow-step.done {
  opacity: 0.75;
}
.step-num {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: #e0e0e0;
  color: #666;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 13px;
  flex-shrink: 0;
}
.flow-step.active .step-num {
  background: linear-gradient(135deg, #4f46e5, #7c3aed);
  color: #fff;
  box-shadow: 0 2px 8px rgba(79,70,229,0.35);
}
.flow-step.done .step-num {
  background: #10b981;
  color: #fff;
}
.step-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.step-desc {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 1px;
}
.step-arrow {
  color: #ccc;
  font-size: 14px;
  font-weight: 700;
}
.step-arrow.active {
  color: #4f46e5;
}
.demo-btn {
  margin-left: auto;
}
/* 空状态引导 */
.empty-guide {
  text-align: center;
  padding: 28px 16px;
  color: var(--text-secondary);
}
.empty-icon {
  font-size: 32px;
  margin-bottom: 8px;
}
.empty-text {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
  margin-bottom: 4px;
}
.empty-hint {
  font-size: 12px;
  color: var(--text-secondary);
}
.card-pad {
  padding: 20px 22px;
}
.main-grid {
  align-items: start;
}
.op-item {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 10px 12px;
  border-radius: 9px;
  cursor: pointer;
  transition: all 0.18s;
  border: 1px solid transparent;
}
.op-item:hover {
  background: var(--bg-page);
}
.op-item.sel {
  background: var(--brand-soft);
  border-color: var(--brand-light);
}
.op-check {
  width: 18px;
  height: 18px;
  border-radius: 5px;
  border: 2px solid var(--border-light);
  display: grid;
  place-items: center;
  flex-shrink: 0;
  margin-top: 2px;
  color: #fff;
  font-size: 12px;
}
.op-check.on {
  background: var(--brand);
  border-color: var(--brand);
}
.op-name {
  font-weight: 600;
  font-size: 14px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.op-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 3px;
}
.op-detail {
  color: var(--text-3);
  cursor: pointer;
  font-size: 16px;
  flex-shrink: 0;
  align-self: center;
}
.op-detail:hover {
  color: var(--brand);
}
.cats {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}
.cat {
  font-size: 12px;
  padding: 3px 10px;
  border-radius: 999px;
  background: var(--bg-page);
  color: var(--text-2);
  cursor: pointer;
  transition: all 0.15s;
}
.cat:hover {
  color: var(--brand);
}
.cat.on {
  background: var(--brand);
  color: #fff;
}
.cmp-chart {
  width: 100%;
  height: 220px;
  margin-top: 12px;
}
.detail-badge {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}
.detail-desc {
  font-size: 13px;
  color: var(--text-2);
  line-height: 1.7;
}
.kv {
  display: flex;
  justify-content: space-between;
  padding: 8px 0;
  font-size: 13px;
  border-bottom: 1px solid var(--border-light);
}
.kv b {
  color: var(--text-1);
}
.kv b.ok {
  color: var(--success);
}
.detail-run {
  width: 100%;
  margin-top: 18px;
}
.chain {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  align-items: center;
  margin-bottom: 14px;
  min-height: 28px;
}
.chain-label {
  font-size: 13px;
  color: var(--text-2);
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
.arrow {
  color: var(--text-3);
  margin: 0 2px;
}
.muted {
  color: var(--text-3);
  font-size: 13px;
}
.exec-form {
  margin-top: 6px;
}
.result {
  margin-top: 16px;
  padding: 14px;
  background: var(--bg-page);
  border-radius: 10px;
}
.result-head {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}
.out-vec {
  font-size: 13px;
  margin: 6px 0;
  font-family: monospace;
}
.metrics {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: var(--text-2);
  margin: 6px 0;
}
.err {
  color: var(--danger);
  font-size: 13px;
}
.logs {
  margin-top: 8px;
  max-height: 140px;
  overflow: auto;
}
.log-line {
  font-size: 12px;
  color: var(--text-2);
  font-family: monospace;
  padding: 1px 0;
}
</style>
