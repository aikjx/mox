<template>
  <div class="ops">
    <div class="head">
      <div>
        <h2 class="page-title">算子中心</h2>
        <p class="page-subtitle">管理标准算子与自定义算子，编排并执行算子工作流链</p>
      </div>
      <el-button type="primary" @click="showRegister = true">
        <el-icon><Plus /></el-icon> 注册算子
      </el-button>
    </div>

    <div class="grid grid-2 main-grid">
      <!-- 算子库 -->
      <div class="panel card-pad">
        <h3 class="section-title">算子库（{{ operators.length }}）</h3>
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
          <span v-else class="muted">请从左侧选择算子</span>
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
          <div v-if="result.logs?.length" class="logs">
            <div v-for="(l, i) in result.logs" :key="i" class="log-line">{{ l }}</div>
          </div>
        </div>
      </div>
    </div>

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
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Search, Coordinate } from '@element-plus/icons-vue'
import { getOperators, registerOperator, executeWorkflow } from '@/api'

const operators = ref([])
const kw = ref('')
const selected = ref(new Set())
const selectedOrder = ref([])
const inputVec = ref('1,2,3,4')
const scale = ref(2.0)
const running = ref(false)
const result = ref(null)

const showRegister = ref(false)
const reging = ref(false)
const reg = ref({ id: '', name: '', operator_type: 'function' })

const filtered = computed(() => {
  const k = kw.value.trim().toLowerCase()
  if (!k) return operators.value
  return operators.value.filter(
    (o) => o.name.toLowerCase().includes(k) || (o.description || '').toLowerCase().includes(k)
  )
})

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
  try {
    operators.value = await getOperators()
  } catch (e) {
    ElMessage.error('算子列表加载失败：' + e.message)
  }
}

onMounted(loadOps)
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
  border: 2px solid #cbd5e1;
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
