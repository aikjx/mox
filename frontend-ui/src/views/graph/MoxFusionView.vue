<template>
  <div class="page-container fusion">
    <el-row :gutter="16">
      <el-col :span="8">
        <el-card shadow="never" class="panel">
          <template #header><b>① 业务蓝图（归一化输入）</b></template>
          <el-input
            v-model="blueprintText"
            type="textarea"
            :rows="14"
            placeholder='粘贴 FlowGraph JSON：{ "nodes":[{ "id":"n1","name":"采集","type":"operator","params":{} }], "edges":[{ "from":"n1","to":"n2" }] }'
          />
          <el-button type="primary" class="mt" @click="normalize" :loading="loadingNorm">
            mox 模块化系统架构归一化（双璇玑十四维 + 璇玑）
          </el-button>
        </el-card>
      </el-col>

      <el-col :span="8">
        <el-card shadow="never" class="panel">
          <template #header><b>② 治理结论</b></template>
          <template v-if="report">
            <el-descriptions :column="1" border size="small">
              <el-descriptions-item label="治理评分">{{ report.governance?.score ?? '—' }}</el-descriptions-item>
              <el-descriptions-item label="治理闸门">{{ report.governance?.gate ?? '—' }}</el-descriptions-item>
              <el-descriptions-item label="优化指标">{{ report.optimization?.metric ?? '—' }}</el-descriptions-item>
              <el-descriptions-item label="优化算法">{{ report.optimization?.algorithm ?? '—' }}</el-descriptions-item>
            </el-descriptions>
            <el-divider>治理 8 闸门（全量门禁）</el-divider>
            <div v-for="g in report.governance?.gate_detail?.gates || []" :key="g.id.code" class="gate-row">
              <el-tag :type="g.passed ? 'success' : 'danger'" size="small">
                {{ g.id.code }}·{{ g.id.name }}
              </el-tag>
              <span class="gate-reason" :class="{ fail: !g.passed }">{{ g.passed ? '通过' : g.reason }}</span>
            </div>
            <el-divider>优化后流程图节点</el-divider>
            <el-tag v-for="n in optimizedNodes" :key="n.id" class="node-tag" type="success">
              {{ n.name || n.id }} · {{ n.type }}
            </el-tag>
            <el-divider>有向边</el-divider>
            <div v-for="e in optimizedEdges" :key="e.from + '>' + e.to" class="edge-line">
              {{ e.from }} → {{ e.to }}
            </div>
          </template>
          <el-empty v-else description="尚未归一化" />
        </el-card>
      </el-col>

      <el-col :span="8">
        <el-card shadow="never" class="panel">
          <template #header><b>③ 融合上传（算子市场/插件·应用平台）</b></template>
          <el-form label-width="80px" size="small">
            <el-form-item label="包名称">
              <el-input v-model="pkgName" placeholder="mox 模块化系统架构融合算子" />
            </el-form-item>
            <el-form-item label="自然语言需求">
              <el-input v-model="pkgReq" type="textarea" :rows="3" placeholder="该流程解决的业务问题" />
            </el-form-item>
            <el-form-item label="来源任务ID">
              <el-input v-model="taskId" placeholder="双璇玑任务闭环 ID（I-07 追溯）" />
            </el-form-item>
            <el-form-item label="双验收联动">
              <el-switch v-model="taskDone" active-text="需求侧任务 Done" />
              <span class="hint">（I-05：任务 Done ∧ 融合验证通过才可上架）</span>
            </el-form-item>
          </el-form>
          <el-alert
            v-if="report && !dualAcceptable"
            class="mt"
            type="warning"
            :title="'双验收未达成：' + dualReason"
            :closable="false"
            show-icon
          />
          <el-button
            type="success"
            :disabled="!report || !dualAcceptable"
            :loading="loadingPublish"
            @click="publish"
          >
            一键归一化上传到平台
          </el-button>
          <el-alert
            v-if="publishResult"
            class="mt"
            :type="publishResult.published ? 'success' : 'error'"
            :title="publishResult.published ? `已上传：包 ${publishResult.package?.id}` : '上架被管制门禁拦截'"
            :description="publishResult.published
              ? `节点 ${publishResult.package?.nodes} / 边 ${publishResult.package?.edges}；治理评分 ${publishResult.governance?.score}`
              : (publishResult.reason || '上架被管制门禁拦截')"
            show-icon
            :closable="false"
          />
          <el-divider v-if="publishResult?.published && publishResult.provenance">产物来源追溯</el-divider>
          <el-descriptions v-if="publishResult?.published && publishResult.provenance" :column="1" border size="small">
            <el-descriptions-item label="璇玑验证">{{ publishResult.provenance.algo_verified ? '通过' : '否决' }}</el-descriptions-item>
            <el-descriptions-item label="8 闸门">{{ publishResult.provenance.gates_passed ? '全过' : '未全过' }}</el-descriptions-item>
            <el-descriptions-item label="关键路径(前/后)">{{ publishResult.provenance.critical_path_before }} → {{ publishResult.provenance.critical_path_after }}</el-descriptions-item>
            <el-descriptions-item label="加速比">{{ publishResult.provenance.speedup.toFixed(2) }}×</el-descriptions-item>
            <el-descriptions-item label="冲突数">{{ publishResult.provenance.conflicts }}</el-descriptions-item>
            <el-descriptions-item label="专家均分">{{ publishResult.provenance.expert_score.toFixed(1) }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { moxOptimize, moxPublish } from '@/api'

// 蓝图文本由用户输入或从后端加载，初始为空
const blueprintText = ref('')

const report = ref(null)
const tenant = ref('default')
const loadingNorm = ref(false)
const loadingPublish = ref(false)
const pkgName = ref('')
const pkgReq = ref('')
const publishResult = ref(null)
const taskId = ref('')
const taskDone = ref(false)

const optimizedGraph = computed(() => report.value?.optimization?.optimized_graph || report.value?.optimization || null)
const optimizedNodes = computed(() => optimizedGraph.value?.nodes || [])
const optimizedEdges = computed(() => optimizedGraph.value?.edges || [])

// I-05 双验收联动：需求侧任务 Done ∧ 融合侧璇玑验证（8 闸门全过且未否决）
const dualAcceptable = computed(() => {
  if (!report.value) return false
  const gd = report.value.governance?.gate_detail
  const algoOk = !gd?.algorithm_veto
  const gateOk = gd?.approved
  return taskDone.value && algoOk && gateOk
})
const dualReason = computed(() => {
  if (!report.value) return '尚未归一化'
  const gd = report.value.governance?.gate_detail
  const parts = []
  if (!taskDone.value) parts.push('需求侧任务未标记 Done')
  if (gd?.algorithm_veto) parts.push('融合侧璇玑验证否决')
  if (gd && !gd.approved) parts.push('治理门禁未通过：' + (gd.reason || ''))
  return parts.join('；') || '双验收达成，可上架'
})

async function normalize() {
  let flow
  try {
    flow = JSON.parse(blueprintText.value)
  } catch (e) {
    ElMessage.error('蓝图 JSON 解析失败：' + e.message)
    return
  }
  loadingNorm.value = true
  try {
    const r = await moxOptimize(flow, tenant.value)
    report.value = r
    pkgName.value = pkgName.value || 'mox 模块化系统架构融合算子'
    ElMessage.success('归一化完成，治理闸门：' + (r.governance?.gate || '—'))
  } catch (e) {
    ElMessage.error('归一化失败：' + e.message)
  } finally {
    loadingNorm.value = false
  }
}

async function publish() {
  if (!report.value) return
  loadingPublish.value = true
  try {
    const r = await moxPublish({
      flow: JSON.parse(blueprintText.value),
      name: pkgName.value || undefined,
      description: undefined,
      requirement: pkgReq.value || undefined,
      tags: undefined,
      task_done: taskDone.value,
      task_id: taskId.value || undefined
    })
    if (r.published) {
      publishResult.value = r
      ElMessage.success('已上传到算子市场（插件/应用平台），包 ID：' + r.package.id)
    } else {
      ElMessage.error('上架被管制门禁拦截：' + (r.reason || r.error || '未知错误'))
    }
  } catch (e) {
    ElMessage.error('上传失败：' + e.message)
  } finally {
    loadingPublish.value = false
  }
}
</script>

<style scoped>
.fusion { padding: 12px; }
.panel { min-height: 100%; }
.mt { margin-top: 10px; }
.node-tag { margin: 0 6px 6px 0; }
.edge-line { font-size: 12px; color: #909399; line-height: 1.6; }
.gate-row { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.gate-reason { font-size: 12px; color: #67C23A; }
.gate-reason.fail { color: #F56C6C; }
.hint { font-size: 11px; color: #909399; margin-left: 6px; }
</style>
