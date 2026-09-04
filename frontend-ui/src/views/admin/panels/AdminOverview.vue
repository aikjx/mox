<template>
  <div class="adm-overview" v-loading="loading">
    <div class="grid grid-4 kpi-row">
      <div class="panel kpi" v-for="k in kpis" :key="k.label">
        <div class="kpi-icon" :style="{ background: k.bg, color: k.color }">
          <el-icon :size="22"><component :is="k.icon" /></el-icon>
        </div>
        <div>
          <div class="kpi-value">{{ k.value }}</div>
          <div class="kpi-label">{{ k.label }}</div>
        </div>
      </div>
    </div>

    <div class="grid grid-2">
      <div class="panel card-pad">
        <h3 class="section-title">系统信息</h3>
        <div class="info-grid">
          <div class="info-item" v-for="i in systemInfo" :key="i.label">
            <span class="info-label">{{ i.label }}</span>
            <span class="info-value">{{ i.value }}</span>
          </div>
        </div>
      </div>

      <div class="panel card-pad">
        <h3 class="section-title">LLM 网关概况</h3>
        <div class="info-grid">
          <div class="info-item" v-for="i in llmInfo" :key="i.label">
            <span class="info-label">{{ i.label }}</span>
            <span class="info-value">{{ i.value }}</span>
          </div>
        </div>
      </div>
    </div>

    <div class="grid grid-2">
      <div class="panel card-pad">
        <h3 class="section-title">安全健康</h3>
        <div class="sec-head">
          <span class="badge" :class="secBadge.cls">安全状态：{{ secBadge.text }}</span>
          <span class="badge info">限流拦截：{{ security.rate_limiters_blocked || 0 }}</span>
        </div>
        <el-empty v-if="!security.recommendations?.length" description="暂无安全建议" :image-size="60" />
        <ul v-else class="rec-list">
          <li v-for="(r, i) in security.recommendations" :key="i">{{ r }}</li>
        </ul>
      </div>

      <div class="panel card-pad">
        <h3 class="section-title">快捷入口</h3>
        <div class="quick-grid">
          <div class="quick-item" v-for="q in quickLinks" :key="q.label" @click="$router.push(q.to)">
            <el-icon :size="18" :color="q.color"><component :is="q.icon" /></el-icon>
            <span>{{ q.label }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import {
  getSecurityStatus, getStorageStatus, getModules,
  getLlmStats, getFullStatus
} from '@/api'

const security = ref({})
const storage = ref({})
const modules = ref([])
const llm = ref({})
const full = ref({})
const loading = ref(false)

function fmtUptime(sec) {
  if (!sec && sec !== 0) return '-'
  const d = Math.floor(sec / 86400)
  const h = Math.floor((sec % 86400) / 3600)
  const m = Math.floor((sec % 3600) / 60)
  if (d > 0) return `${d}天${h}小时`
  if (h > 0) return `${h}小时${m}分`
  return `${m}分`
}

const kpis = computed(() => [
  { label: '活跃 API 凭证', value: security.value.active_api_keys ?? '-', icon: 'Key', color: '#4f46e5', bg: '#eef2ff' },
  { label: '审计记录', value: security.value.audit_log_entries ?? '-', icon: 'Document', color: '#0d9488', bg: '#ccfbf1' },
  { label: '存储实体', value: storage.value.totalEntities ?? '-', icon: 'Coin', color: '#d97706', bg: '#fef3c7' },
  { label: '已加载模块', value: modules.value.length ?? '-', icon: 'Box', color: '#0369a1', bg: '#e0f2fe' }
])

const systemInfo = computed(() => [
  { label: '系统版本', value: full.value.version || '-' },
  { label: '运行状态', value: full.value.status || '-' },
  { label: '运行时长', value: fmtUptime(full.value.uptime) },
  { label: '算子数量', value: full.value.operators_count ?? '-' },
  { label: '图谱节点 / 边', value: full.value.graph ? `${full.value.graph.nodes} / ${full.value.graph.edges}` : '-' },
  { label: '执行次数', value: full.value.executions_count ?? '-' },
  { label: '执行成功率', value: full.value.success_rate != null ? `${full.value.success_rate}%` : '-' },
  { label: '存储提供方', value: storage.value.provider || '-' }
])

const llmInfo = computed(() => [
  { label: '供应商数量', value: llm.value.providers ?? '-' },
  { label: '总请求数', value: llm.value.total_requests ?? '-' },
  { label: '总 Token 数', value: llm.value.total_tokens ?? '-' },
  { label: '请求成功率', value: llm.value.success_rate != null ? `${llm.value.success_rate}%` : '-' }
])

const secBadge = computed(() => {
  const h = security.value.security_health
  return h === 'good'
    ? { cls: 'success', text: '良好' }
    : { cls: 'warning', text: h || '未知' }
})

const quickLinks = [
  { label: '访问凭证', to: '/admin?tab=access', icon: 'Key', color: '#4f46e5' },
  { label: '审计日志', to: '/admin?tab=audit', icon: 'List', color: '#0d9488' },
  { label: '存储与模块', to: '/admin?tab=storage', icon: 'Coin', color: '#d97706' },
  { label: 'HITL 审批', to: '/admin?tab=hitl', icon: 'Clock', color: '#f59e0b' },
  { label: '大模型配置', to: '/llm-config', icon: 'Setting', color: '#6366f1' },
  { label: '云盘知识库', to: '/knowledge-base', icon: 'Collection', color: '#0d9488' }
]

onMounted(async () => {
  loading.value = true
  const tasks = [
    getSecurityStatus().then(d => { security.value = d || {} }).catch(() => {}),
    getStorageStatus().then(d => { storage.value = d || {} }).catch(() => {}),
    getModules().then(d => { modules.value = Array.isArray(d) ? d : [] }).catch(() => {}),
    getLlmStats().then(d => { llm.value = d || {} }).catch(() => {}),
    getFullStatus().then(d => { full.value = d || {} }).catch(() => {})
  ]
  await Promise.all(tasks)
  loading.value = false
})
</script>

<style scoped>
.kpi-row { margin-bottom: 16px; }
.kpi {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 18px;
}
.kpi-icon {
  width: 46px;
  height: 46px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
}
.kpi-value { font-size: 22px; font-weight: 700; color: var(--text-1); line-height: 1.2; }
.kpi-label { font-size: 12px; color: var(--text-3); margin-top: 3px; }
.info-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px 24px; }
.info-item { display: flex; justify-content: space-between; gap: 12px; font-size: 13px; }
.info-label { color: var(--text-3); }
.info-value { color: var(--text-1); font-weight: 600; text-align: right; }
.sec-head { display: flex; gap: 10px; margin-bottom: 12px; }
.rec-list { margin: 0; padding-left: 18px; color: var(--text-2); font-size: 13px; line-height: 2; }
.quick-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
.quick-item {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 13px 14px;
  border: 1px solid var(--border-light);
  border-radius: 10px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-2);
  transition: all var(--transition);
}
.quick-item:hover { border-color: var(--brand); color: var(--brand); background: var(--brand-soft); }
</style>
