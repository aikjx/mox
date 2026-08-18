<template>
  <div class="bc">
    <div class="head">
      <div>
        <h2 class="page-title">机器人中心</h2>
        <p class="page-subtitle">RPA 流程管理 · 支持外部平台（如政数局）带 dId 跳转直达流程详情</p>
      </div>
      <div class="head-actions">
        <el-button @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
      </div>
    </div>

    <el-alert
      v-if="fromGov"
      type="info"
      show-icon
      :closable="false"
      title="外部平台跳转"
      :description="'正在定位流程（dId=' + dId + '）…'"
      class="jump-tip"
    />

    <div class="grid grid-3">
      <div class="panel flow-card" v-for="f in flows" :key="f.id">
        <div class="flow-top">
          <div class="flow-name">{{ f.name || f.id }}</div>
          <el-icon class="more"><Cpu /></el-icon>
        </div>
        <div class="flow-meta">
          <span class="badge info">{{ f.nodes?.length || 0 }} 节点</span>
          <span class="badge primary">{{ f.edges?.length || 0 }} 连线</span>
        </div>
        <div class="flow-desc">{{ f.description || '暂无描述' }}</div>
        <div class="flow-actions">
          <el-button size="small" type="primary" plain @click="viewFlowDetail(f)">
            <el-icon><View /></el-icon> 查看详情
          </el-button>
          <div class="flow-quick">
            <el-tooltip content="查看执行录屏" placement="top">
              <el-button size="small" circle class="q-video" @click="quickOpen(f, 'video')">
                <el-icon><VideoCamera /></el-icon>
              </el-button>
            </el-tooltip>
            <el-tooltip content="查看执行日志" placement="top">
              <el-button size="small" circle class="q-log" @click="quickOpen(f, 'log')">
                <el-icon><Document /></el-icon>
              </el-button>
            </el-tooltip>
          </div>
        </div>
      </div>
      <el-empty v-if="!flows.length" description="暂无流程" :image-size="70" />
    </div>

    <FlowDetailDialog
      v-model="flowDetailOpen"
      :flow-detail="flowDetail"
      :initial-panel="detailPanel"
    />
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { VideoCamera, Document } from '@element-plus/icons-vue'
import { getFlows, getFlow } from '@/api'
import FlowDetailDialog from '@/components/FlowDetailDialog.vue'

const route = useRoute()
const flows = ref([])
const flowDetail = ref(null)
const flowDetailOpen = ref(false)
const detailPanel = ref('')
const fromGov = ref(false)
const dId = ref('')

async function loadAll() {
  try {
    const f = await getFlows()
    flows.value = Array.isArray(f) ? f : f.flows || f.data || []
  } catch (e) {
    ElMessage.error('加载失败：' + e.message)
  }
}

async function viewFlowDetail(f) {
  detailPanel.value = ''
  await openDetail(f)
}

/** 卡片快捷入口：跳过详情，直接打开视频/日志面板 */
async function quickOpen(f, panel) {
  detailPanel.value = panel
  await openDetail(f)
}

async function openDetail(f) {
  try {
    const r = await getFlow(f.id)
    flowDetail.value = r.flow || r || f
    flowDetailOpen.value = true
  } catch (e) {
    flowDetail.value = f
    flowDetailOpen.value = true
  }
}

onMounted(async () => {
  loadAll()
  // 支持政数局等外部平台跳转：#/botCenter?dId=xxx → 自动弹出对应流程详情
  const q = route.query
  const id = (q.dId || q.did || '').toString().trim()
  if (id) {
    fromGov.value = true
    dId.value = id
    try {
      const r = await getFlow(id)
      flowDetail.value = r.flow || r
      flowDetailOpen.value = true
      ElMessage.success('已打开流程详情')
    } catch (e) {
      ElMessage.warning('未找到 dId=' + id + ' 对应的流程详情，请在下方列表中选择')
    }
  }
})
</script>

<style scoped>
.bc {
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
  gap: 8px;
}
.jump-tip {
  max-width: 640px;
}
.flow-card {
  padding: 16px 18px;
  display: flex;
  flex-direction: column;
}
.flow-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.flow-name {
  font-weight: 700;
  font-size: 15px;
}
.more {
  font-size: 18px;
  color: var(--text-3);
}
.flow-meta {
  display: flex;
  gap: 8px;
  margin: 10px 0;
}
.flow-desc {
  font-size: 13px;
  color: var(--text-3);
  min-height: 38px;
}
.flow-actions {
  margin-top: auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.flow-quick {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}
.q-video {
  --el-button-bg-color: #eef2ff;
  --el-button-border-color: #c7d2fe;
  --el-button-text-color: #4f46e5;
  --el-button-hover-bg-color: #e0e7ff;
  --el-button-hover-border-color: #a5b4fc;
  --el-button-hover-text-color: #4338ca;
}
.q-log {
  --el-button-bg-color: #fff7ed;
  --el-button-border-color: #fed7aa;
  --el-button-text-color: #ea580c;
  --el-button-hover-bg-color: #ffedd5;
  --el-button-hover-border-color: #fdba74;
  --el-button-hover-text-color: #c2410c;
}
</style>
