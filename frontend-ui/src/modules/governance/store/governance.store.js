// 治理台状态层：五个读通道各自记账，错误在此单点收敛（视图只呈现，不 catch）。
import { computed, reactive, ref } from 'vue'
import { defineStore } from 'pinia'
import { ElMessage } from 'element-plus/es/components/message/index'
import { governanceApi } from '@/modules/governance/api'
import { DEFAULT_THRESHOLDS, DIM_IDS } from '@/modules/governance/contract'
import { missingDims } from '@/modules/governance/model'

const CHANNELS = ['dashboard', 'experts', 'vetoes', 'audit', 'config']

const emptyLoading = () => Object.fromEntries(CHANNELS.map((c) => [c, false]))
const emptyError = () => Object.fromEntries(CHANNELS.map((c) => [c, '']))

export const useGovernanceStore = defineStore('governance', () => {
  const dashboard = ref(null)
  const expertsStatus = ref(null)
  const vetoPage = ref(null)
  const auditPage = ref(null)
  const expertConfig = ref(null)

  const loading = reactive(emptyLoading())
  const error = reactive(emptyError())

  const vetoFilter = reactive({ page: 1, pageSize: 20, dimension: '', blocked: undefined, flowId: '' })
  const auditFilter = reactive({ page: 1, pageSize: 20, flowId: '', action: '' })

  // 后端权威阈值取配置端点；没取到时用 ExpertConfig::default 的三档，并把这个事实显式带出去
  const thresholds = computed(() => {
    const t = expertConfig.value?.thresholds
    const ok = t && Number.isFinite(t.vetoThreshold) && Number.isFinite(t.warnThreshold)
    return { ...(ok ? { ...DEFAULT_THRESHOLDS, ...t } : DEFAULT_THRESHOLDS), fromConfig: !!ok }
  })

  // 状态端点按 states.get(dim) 逐维取，缺维就是缺行 ⇒ 界面要说出缺了谁，不能只显示"7 维健康"
  // 但"还没读到"不等于"后端没有这一维"：状态端点没回答前无从判断，不许谎报缺 14 维
  const absentDims = computed(() => {
    const src = expertsStatus.value
    if (!src) return []
    const present = [
      ...(src.business?.experts ?? []),
      ...(src.dev?.experts ?? [])
    ].map((e) => e.dimension)
    return missingDims(present)
  })

  const dimensionRows = computed(() => {
    const src = expertsStatus.value
      ? [...(expertsStatus.value.business.experts ?? []), ...(expertsStatus.value.dev.experts ?? [])]
      : (dashboard.value?.experts ?? [])
    const byDim = new Map(src.map((e) => [e.dimension, e]))
    return DIM_IDS.map((id) => byDim.get(id) ?? { dimension: id, absent: true })
  })

  const vetoRatePercent = computed(() => {
    const v = dashboard.value?.vetoRate
    return Number.isFinite(v) ? Math.round(v * 1000) / 10 : null
  })

  async function run(channel, fn, onOk) {
    loading[channel] = true
    error[channel] = ''
    try {
      onOk(await fn())
      return true
    } catch (e) {
      error[channel] = e?.message || '加载失败'
      ElMessage.error(`${channel} 加载失败：${error[channel]}`)
      return false
    } finally {
      loading[channel] = false
    }
  }

  return {
    dashboard,
    expertsStatus,
    vetoPage,
    auditPage,
    expertConfig,
    loading,
    error,
    vetoFilter,
    auditFilter,
    thresholds,
    absentDims,
    dimensionRows,
    vetoRatePercent,

    loadDashboard: () => run('dashboard', () => governanceApi.getDashboard(), (v) => (dashboard.value = v)),
    loadExperts: () => run('experts', () => governanceApi.getExpertsStatus(), (v) => (expertsStatus.value = v)),
    loadVetoes: () =>
      run('vetoes', () => governanceApi.listVetoEvents({ ...vetoFilter }), (v) => (vetoPage.value = v)),
    loadAudit: () => run('audit', () => governanceApi.listAuditLogs({ ...auditFilter }), (v) => (auditPage.value = v)),
    loadConfig: () => run('config', () => governanceApi.getExpertConfig(), (v) => (expertConfig.value = v)),

    async loadAll() {
      await Promise.all([
        this.loadDashboard(),
        this.loadExperts(),
        this.loadVetoes(),
        this.loadAudit(),
        this.loadConfig()
      ])
    }
  }
})
