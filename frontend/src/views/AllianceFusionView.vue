<template>
  <div class="fusion">
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
            全维归一化（双联盟十四维 + 璇玑）
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
              <el-input v-model="pkgName" placeholder="全维融合算子" />
            </el-form-item>
            <el-form-item label="自然语言需求">
              <el-input v-model="pkgReq" type="textarea" :rows="3" placeholder="该流程解决的业务问题" />
            </el-form-item>
          </el-form>
          <el-button
            type="success"
            :disabled="!report"
            :loading="loadingPublish"
            @click="publish"
          >
            一键归一化上传到平台
          </el-button>
          <el-alert
            v-if="publishResult"
            class="mt"
            type="success"
            :title="`已上传：包 ${publishResult.package?.id}`"
            :description="`节点 ${publishResult.package?.nodes} / 边 ${publishResult.package?.edges}；治理评分 ${publishResult.governance?.score}`"
            show-icon
            :closable="false"
          />
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { allianceOptimize, alliancePublish } from '../api'

const blueprintText = ref(JSON.stringify({
  nodes: [
    { id: 'n1', name: '采集需求', type: 'operator', params: {} },
    { id: 'n2', name: 'AI 合规审查', type: 'ai_task', params: { prompt: '审查合规风险' } },
    { id: 'n3', name: '条件分流', type: 'condition', params: { expr: '${compliant}==true' } },
    { id: 'n4', name: '归档', type: 'operator', params: {} }
  ],
  edges: [
    { from: 'n1', to: 'n2' },
    { from: 'n2', to: 'n3' },
    { from: 'n3', to: 'n4' }
  ]
}, null, 2))

const report = ref(null)
const loadingNorm = ref(false)
const loadingPublish = ref(false)
const pkgName = ref('')
const pkgReq = ref('')
const publishResult = ref(null)

const optimizedGraph = computed(() => report.value?.optimization?.optimized_graph || report.value?.optimization || null)
const optimizedNodes = computed(() => optimizedGraph.value?.nodes || [])
const optimizedEdges = computed(() => optimizedGraph.value?.edges || [])

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
    const r = await allianceOptimize(flow)
    report.value = r
    pkgName.value = pkgName.value || '全维融合算子'
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
    const r = await alliancePublish({
      flow: JSON.parse(blueprintText.value),
      name: pkgName.value || undefined,
      description: undefined,
      requirement: pkgReq.value || undefined,
      tags: undefined
    })
    if (r.published) {
      publishResult.value = r
      ElMessage.success('已上传到算子市场（插件/应用平台），包 ID：' + r.package.id)
    } else {
      ElMessage.error('上传失败：' + (r.error || '未知错误'))
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
</style>
