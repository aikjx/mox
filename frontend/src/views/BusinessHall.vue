<template>
  <div class="hall">
    <header class="nav">
      <div class="brand" @click="go('/')">🧠 智算企业门户</div>
      <nav class="menu">
        <a @click="go('/')">首页</a>
        <a @click="go('/hall')" class="active">业务大厅</a>
        <a @click="go('/workbench')">工作台</a>
        <a @click="go('/login')">退出</a>
      </nav>
    </header>

    <section class="head">
      <h1>业务大厅</h1>
      <p>调用算子统一系统已注册的能力，一键发起业务流程办理（数据来自 /api/operators 与 /api/ai/flows）</p>
    </section>

    <section class="grid">
      <el-card v-for="op in operators" :key="op.name" class="op" shadow="hover">
        <div class="op-name">{{ op.name }}</div>
        <div class="op-desc">{{ op.description || '算子' }}</div>
        <div class="op-meta">
          <el-tag size="small" effect="plain">{{ op.category || '通用' }}</el-tag>
          <span class="io">in:{{ op.in_type || '?' }} → out:{{ op.out_type || '?' }}</span>
        </div>
        <el-button size="small" type="primary" plain @click="runOperator(op)">执行</el-button>
      </el-card>
    </section>

    <section class="flows" v-if="flows.length">
      <h2>可执行业务流程</h2>
      <el-table :data="flows" style="width:100%">
        <el-table-column prop="id" label="流程 ID" width="220" />
        <el-table-column prop="name" label="名称" />
        <el-table-column label="操作" width="160">
          <template #default="{ row }">
            <el-button size="small" @click="runFlow(row)">执行流程</el-button>
          </template>
        </el-table-column>
      </el-table>
    </section>

    <el-dialog v-model="resultVisible" title="执行结果" width="560px">
      <pre class="result">{{ resultText }}</pre>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { getOperators, executeFlow, executeWorkflow, getFlows } from '@/api'

const router = useRouter()
const go = (p) => router.push(p)

const operators = ref([])
const flows = ref([])
const resultVisible = ref(false)
const resultText = ref('')

async function load() {
  try {
    const ops = await getOperators()
    operators.value = (ops.operators || ops || []).map((o) =>
      typeof o === 'string' ? { name: o } : o
    )
  } catch (e) {
    operators.value = []
  }
  try {
    const fl = await getFlows()
    flows.value = fl.flows || []
  } catch (e) {
    flows.value = []
  }
}

async function runOperator(op) {
  resultText.value = '执行中…'
  resultVisible.value = true
  try {
    const resp = await executeWorkflow({
      workflow: [op.name],
      input: [1.0],
      parameters: {},
    })
    resultText.value = JSON.stringify(resp, null, 2)
    ElMessage.success(`算子「${op.name}」执行完成`)
  } catch (e) {
    resultText.value = '该算子需要参数，请在「算子工坊」中选择并填入参数后再执行。'
    ElMessage.warning('已转入参数引导模式')
    router.push({ name: 'operators' })
  }
}

async function runFlow(row) {
  try {
    const resp = await executeFlow({ flow_id: row.id, input: {} })
    resultText.value = JSON.stringify(resp, null, 2)
    resultVisible.value = true
  } catch (e) {
    ElMessage.error(e.message)
  }
}

onMounted(load)
</script>

<style scoped>
.hall { min-height: 100vh; background: #0b1020; color: #e6ebf5; }
.nav { display: flex; justify-content: space-between; align-items: center; padding: 14px 32px; border-bottom: 1px solid rgba(255,255,255,.06); }
.brand { cursor: pointer; font-weight: 700; }
.menu a { margin-left: 20px; cursor: pointer; color: #c4cdec; }
.menu a.active { color: #6ea8ff; font-weight: 600; }
.head { padding: 30px 32px 10px; }
.head h1 { margin: 0 0 8px; }
.head p { color: #9fb0d6; }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 16px; padding: 16px 32px; }
.op { background: #0e1428; border: 1px solid rgba(255,255,255,.08); }
.op-name { font-weight: 600; margin-bottom: 6px; }
.op-desc { color: #9fb0d6; font-size: 13px; margin-bottom: 10px; }
.op-meta { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
.io { font-size: 11px; color: #8b9bc0; }
.flows { padding: 10px 32px 40px; }
.flows h2 { font-size: 16px; margin-bottom: 12px; }
.result { background: #060a16; padding: 14px; border-radius: 8px; max-height: 360px; overflow: auto; color: #9fe6b0; font-size: 12px; }
</style>
