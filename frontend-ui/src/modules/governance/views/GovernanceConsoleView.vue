<template>
  <div class="gv">
    <header class="gv-head">
      <div class="gv-head-text">
        <h1 class="gv-title">联盟治理台</h1>
        <p class="gv-sub">
          双璇玑十四维：健康分、否决事件、审计链与阈值配置 · 后端为编排器 :3001，经网关 :3080 通配反代可达
        </p>
      </div>
      <div class="gv-head-actions">
        <el-tag :type="chainTag.type" effect="plain" size="small">{{ chainTag.text }}</el-tag>
        <el-button :icon="Refresh" :loading="anyLoading" @click="store.loadAll()">刷新</el-button>
      </div>
    </header>

    <el-alert
      class="gv-alert"
      type="info"
      show-icon
      :closable="false"
      title="这些都是进程内读数"
      description="专家状态、否决事件与审计链都住在编排器的内存锁里（GovernanceState），进程重启即归零；本页显示的是「本次启动以来」，不是历史总量。触发一次评估（POST /assess）才会写入新事件。"
    />

    <section class="gv-card">
      <div class="gv-card-head">
        <h2 class="gv-card-title">概览</h2>
        <span class="gv-note">{{ ENDPOINTS.dashboard.method }} {{ ENDPOINTS.dashboard.path }} · 读数时间 {{ serverTime(store.dashboard?.serverTsMs) }}</span>
      </div>
      <p v-if="store.error.dashboard" class="gv-err">{{ store.error.dashboard }}</p>
      <el-skeleton v-else-if="store.loading.dashboard && !store.dashboard" :rows="2" animated />
      <template v-else-if="store.dashboard">
        <div class="gv-kpis">
          <div v-for="k in kpiCards" :key="k.label" class="gv-kpi">
            <b class="gv-kpi-value">{{ k.value }}</b>
            <span class="gv-kpi-label">{{ k.label }}</span>
            <em class="gv-kpi-hint">{{ k.hint }}</em>
          </div>
        </div>
        <p class="gv-honest">
          「放行 / 草稿 / 待评审」三格不是流程表统计：后端用否决事件反推
          （approved = 最近事件数 − 拦截数，review = draft / 2 的整数除，handlers/governance.rs:508-517），
          所以它们只能说明"这批事件里有多少被拦"，不能当作流程总数来读。
        </p>
      </template>
      <p v-else class="gv-err">概览未取到，下面的区块只反映各自端点自己的读数。</p>
    </section>

    <section class="gv-card">
      <div class="gv-card-head">
        <h2 class="gv-card-title">十四维专家健康</h2>
        <span class="gv-note">
          {{ ENDPOINTS.expertsStatus.method }} {{ ENDPOINTS.expertsStatus.path }} · 阈值
          {{ store.thresholds.vetoThreshold }} / {{ store.thresholds.warnThreshold }}
          <em class="gv-threshold-src">{{ store.thresholds.fromConfig ? '读自配置端点' : '配置未取到，显示 ExpertConfig::default 缺省值' }}</em>
        </span>
      </div>
      <p v-if="store.error.experts" class="gv-err">{{ store.error.experts }}</p>
      <p v-if="!store.expertsStatus" class="gv-warn">
        状态端点 {{ store.loading.experts ? '读数中' : '未回答' }}，下面退回概览端点里的 expertStates（两份内存锁同源但可能不同步）。
      </p>
      <p v-else-if="store.absentDims.length" class="gv-warn">
        缺 {{ store.absentDims.length }} 维（{{ store.absentDims.join('、') }}）：该端点按 states.get(dim) 逐维取，缺维就是不在这张表里，
        而不是"健康分为 0"。
      </p>
      <div class="gv-leagues">
        <div v-for="lg in leagues" :key="lg.league" class="gv-league">
          <h3 class="gv-league-title">{{ lg.leagueLabel }} · 均值 {{ fixed(lg.averageHealth, 3) }}</h3>
          <ul class="gv-dims">
            <li v-for="row in lg.rows" :key="row.dimension" class="gv-dim">
              <div class="gv-dim-head">
                <span class="gv-dim-label">{{ row.dimLabel || row.dimension }}</span>
                <code class="gv-dim-id">{{ row.dimension }}</code>
                <el-tag v-if="row.absent" size="small" type="info" effect="plain">后端未回该维</el-tag>
                <el-tag v-else-if="row.pending" size="small" type="info" effect="plain">读数中</el-tag>
                <template v-else>
                  <el-tag v-if="!row.enabled" size="small" type="warning" effect="plain">已停用</el-tag>
                  <span class="gv-dim-score" :class="`tone-${row.tone}`">{{ fixed(row.healthScore, 3) }}</span>
                </template>
              </div>
              <div v-if="!row.absent && !row.pending" class="gv-bar" :class="`tone-${row.tone}`">
                <span class="gv-bar-fill" :style="{ width: barWidth(row.healthScore) }" />
              </div>
              <div v-if="!row.absent && !row.pending" class="gv-dim-foot">
                <span>否决 {{ row.vetoCount }} / 检查 {{ row.totalChecks }}{{ row.checked ? '' : '（本次启动还没检查过）' }}</span>
                <span>更新 {{ serverTime(row.lastUpdatedMs) }}</span>
              </div>
            </li>
          </ul>
        </div>
      </div>
    </section>

    <section class="gv-card">
      <div class="gv-card-head">
        <h2 class="gv-card-title">否决事件</h2>
        <span class="gv-note">
          {{ ENDPOINTS.vetoEvents.method }} {{ ENDPOINTS.vetoEvents.path }} · 服务端筛选，参数按 camelCase 下发
        </span>
        <div class="gv-filters">
          <el-select v-model="store.vetoFilter.dimension" size="small" clearable placeholder="全部维度" class="gv-filter-dim" @change="reloadVetoes">
            <el-option v-for="d in DIMENSIONS" :key="d.id" :label="`${d.label}（${d.id}）`" :value="d.id" />
          </el-select>
          <el-select v-model="store.vetoFilter.blocked" size="small" clearable placeholder="拦截与否" class="gv-filter-blocked" @change="reloadVetoes">
            <el-option label="仅拦截" :value="true" />
            <el-option label="仅放行" :value="false" />
          </el-select>
        </div>
      </div>
      <p v-if="store.error.vetoes" class="gv-err">{{ store.error.vetoes }}</p>
      <el-table v-else :data="store.vetoPage?.rows || []" size="small" max-height="360">
        <el-table-column label="时间" width="150">
          <template #default="{ row }">{{ absTime(row.tsMs) }}</template>
        </el-table-column>
        <el-table-column label="流程" min-width="180">
          <template #default="{ row }">
            <span class="gv-flow-name">{{ row.flowName || '（无流程名）' }}</span>
            <code class="gv-mono">{{ row.flowId }}</code>
          </template>
        </el-table-column>
        <el-table-column label="维度" width="150">
          <template #default="{ row }">{{ row.dimLabel }}<code class="gv-mono gv-mono-dim">{{ row.dimension }}</code></template>
        </el-table-column>
        <el-table-column label="严重度" width="100">
          <template #default="{ row }">
            <el-tag :type="row.isCritical ? 'danger' : 'warning'" size="small" effect="light">{{ row.severity || '—' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="是否拦截" width="100">
          <template #default="{ row }">
            <el-tag :type="row.blocked ? 'danger' : 'success'" size="small" effect="plain">{{ row.blocked ? '拦截' : '放行' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="理由" min-width="240">
          <template #default="{ row }">
            <span class="gv-reason">{{ row.reason }}</span>
            <em v-if="row.gate" class="gv-gate">闸门 {{ row.gate.status }} · 阻断风险 {{ row.gate.blockingRisks }}{{ row.gate.algorithmVeto ? ' · 璇玑否决' : '' }}</em>
          </template>
        </el-table-column>
      </el-table>
      <div class="gv-pager">
        <span class="gv-note">
          第 {{ store.vetoPage?.page || 1 }} / {{ store.vetoPage?.totalPages || 0 }} 页 · 共 {{ store.vetoPage?.total ?? 0 }} 条 ·
          每页 {{ store.vetoPage?.pageSize || store.vetoFilter.pageSize }}（后端把 page_size 夹到 {{ PAGE_SIZE_MAX }}）
        </span>
        <el-button size="small" :disabled="(store.vetoPage?.page || 1) <= 1 || store.loading.vetoes" @click="stepVetoes(-1)">上一页</el-button>
        <el-button size="small" :disabled="!hasNextVeto || store.loading.vetoes" @click="stepVetoes(1)">下一页</el-button>
      </div>
    </section>

    <section class="gv-card">
      <div class="gv-card-head">
        <h2 class="gv-card-title">审计链</h2>
        <span class="gv-note">
          {{ ENDPOINTS.auditLogs.method }} {{ ENDPOINTS.auditLogs.path }} · 链校验
          {{ store.dashboard ? (store.dashboard.auditChainVerified ? '通过' : '未通过') : '未知' }} ·
          事件数 {{ store.dashboard?.auditEventCount ?? '—' }}
        </span>
      </div>
      <p v-if="store.error.audit" class="gv-err">{{ store.error.audit }}</p>
      <el-table v-else :data="store.auditPage?.rows || []" size="small" max-height="300">
        <el-table-column label="时间" width="150">
          <template #default="{ row }">{{ absTime(row.tsMs) }}</template>
        </el-table-column>
        <el-table-column prop="subject" label="主体" width="150" />
        <el-table-column prop="action" label="动作" width="150" />
        <el-table-column label="对象" min-width="160">
          <template #default="{ row }"><code class="gv-mono">{{ row.flowId }}</code></template>
        </el-table-column>
        <el-table-column prop="decision" label="结论" width="110" />
        <el-table-column label="哈希" min-width="200">
          <template #default="{ row }">
            <code class="gv-mono">{{ row.hash.slice(0, 12) }}…</code>
            <em class="gv-gate">前驱 {{ row.prevHash.slice(0, 8) || '（链首）' }}</em>
          </template>
        </el-table-column>
      </el-table>
      <div class="gv-pager">
        <span class="gv-note">第 {{ store.auditPage?.page || 1 }} / {{ store.auditPage?.totalPages || 0 }} 页 · 共 {{ store.auditPage?.total ?? 0 }} 条</span>
        <el-button size="small" :disabled="(store.auditPage?.page || 1) <= 1 || store.loading.audit" @click="stepAudit(-1)">上一页</el-button>
        <el-button size="small" :disabled="!hasNextAudit || store.loading.audit" @click="stepAudit(1)">下一页</el-button>
      </div>
    </section>

    <section class="gv-card">
      <div class="gv-card-head">
        <h2 class="gv-card-title">维度权重与阈值</h2>
        <span class="gv-note">{{ ENDPOINTS.expertConfig.method }} {{ ENDPOINTS.expertConfig.path }}</span>
      </div>
      <p v-if="store.error.config" class="gv-err">{{ store.error.config }}</p>
      <template v-else-if="store.expertConfig">
        <p class="gv-note">
          version {{ store.expertConfig.version }} · 由 {{ store.expertConfig.updatedBy || '（未署名）' }} 更新于 {{ serverTime(store.expertConfig.updatedAtMs) }}
        </p>
        <div class="gv-weights">
          <div v-for="w in weightRows" :key="w.league" class="gv-league">
            <h3 class="gv-league-title">{{ w.leagueLabel }}</h3>
            <ul class="gv-weight-list">
              <li v-for="row in w.rows" :key="row.dimension" class="gv-weight">
                <span class="gv-weight-label">{{ row.dimLabel }}</span>
                <b class="gv-weight-value" :class="{ 'gv-weight-absent': !row.present }">{{ row.present ? fixed(row.weight, 2) : '未配置' }}</b>
              </li>
            </ul>
          </div>
        </div>
        <p class="gv-honest">
          阈值只影响这一页的颜色分档：否决事件的生成条件是
          <code class="gv-mono">score &lt; 0.5</code> 的字面量（handlers/governance.rs:996-999），
          它不读 {{ VETO_EVENT_THRESHOLD_LITERAL }} 之外的任何配置 ⇒ 改 veto_threshold 不会改变哪些维度产生否决事件。
        </p>
      </template>
      <p v-else class="gv-err">配置未取到，本页阈值分档用的是 ExpertConfig::default 的缺省值。</p>
    </section>

    <footer class="gv-foot">
      <p>本域十条路由的端点清单、字段名与列表键由 <code class="gv-mono">modules/governance/contract/governance-contract.test.js</code> 现场解析编排器 Rust 源码守护；PUT 两条与 WS、assess 三条在本页没有出口（详见 §5.13 的只报不改清单）。</p>
    </footer>
  </div>
</template>

<script setup>
import { computed, onMounted } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { ENDPOINTS, DIMENSIONS, LEAGUE, PAGE_SIZE_MAX, VETO_EVENT_THRESHOLD_LITERAL, leagueLabel } from '@/modules/governance/contract'
import { useGovernanceStore } from '@/modules/governance/store'
import { formatDateTime, timeAgoOrDate } from '@/utils'

const store = useGovernanceStore()

const anyLoading = computed(() => Object.values(store.loading).some(Boolean))

// 状态端点是十四维的权威出口；它没到时退回概览里的 expertStates（两份内存锁同源但可能不同步）
const leagues = computed(() => {
  const src = store.expertsStatus
  // 状态端点在飞、概览也没给这一维 ⇒ 只能判"读数中"，判"后端未回该维"要有答案才成立
  const pending = !src && store.loading.experts
  const rows = src ? [...src.business.experts, ...src.dev.experts] : (store.dashboard?.experts ?? [])
  const byDim = new Map(rows.map((r) => [r.dimension, r]))
  const pick = (ids) =>
    ids.map((id) => byDim.get(id) ?? {
      dimension: id,
      absent: !pending,
      pending,
      dimLabel: DIMENSIONS.find((d) => d.id === id)?.label ?? id
    })
  return [
    {
      league: LEAGUE.business,
      leagueLabel: leagueLabel(LEAGUE.business),
      averageHealth: src ? src.business.averageHealth : (store.dashboard?.leagueHealth?.business ?? NaN),
      rows: pick(DIMENSIONS.filter((d) => d.league === LEAGUE.business).map((d) => d.id))
    },
    {
      league: LEAGUE.dev,
      leagueLabel: leagueLabel(LEAGUE.dev),
      averageHealth: src ? src.dev.averageHealth : (store.dashboard?.leagueHealth?.dev ?? NaN),
      rows: pick(DIMENSIONS.filter((d) => d.league === LEAGUE.dev).map((d) => d.id))
    }
  ]
})

const weightRows = computed(() => {
  const c = store.expertConfig
  if (!c) return []
  return [
    { league: LEAGUE.business, leagueLabel: leagueLabel(LEAGUE.business), rows: c.businessWeights },
    { league: LEAGUE.dev, leagueLabel: leagueLabel(LEAGUE.dev), rows: c.devWeights }
  ]
})

const kpiCards = computed(() => {
  const d = store.dashboard
  if (!d) return []
  return [
    { label: '本次启动事件数', value: d.counts.totalFlows, hint: 'totalFlows 实为 veto_events 条数（:453-455）' },
    { label: '被拦截', value: d.counts.blockedFlows, hint: 'blocked=true 的事件数' },
    { label: '否决率', value: store.vetoRatePercent === null ? '—' : `${store.vetoRatePercent}%`, hint: '拦截数 / 事件数，空表为 0' },
    { label: '审计条目', value: d.auditEventCount, hint: 'AuditChain 进程内长度' },
    { label: '业务璇玑均分', value: fixed(d.leagueHealth.business, 3), hint: '7 维健康分之和 ÷ 7（缺维按 0 计入分母）' },
    { label: '开发璇玑均分', value: fixed(d.leagueHealth.dev, 3), hint: '同上，dev_dims ÷ 7' }
  ]
})

const chainTag = computed(() => {
  if (store.loading.dashboard) return { type: 'info', text: '读数中' }
  if (!store.dashboard) return { type: 'danger', text: '链状态未知' }
  return store.dashboard.auditChainVerified
    ? { type: 'success', text: '审计链校验通过' }
    : { type: 'danger', text: '审计链校验未通过' }
})

const hasNextVeto = computed(() => (store.vetoPage?.page || 0) < (store.vetoPage?.totalPages || 0))
const hasNextAudit = computed(() => (store.auditPage?.page || 0) < (store.auditPage?.totalPages || 0))

function stepVetoes(delta) {
  store.vetoFilter.page = Math.max(1, (store.vetoPage?.page || 1) + delta)
  store.loadVetoes()
}

function stepAudit(delta) {
  store.auditFilter.page = Math.max(1, (store.auditPage?.page || 1) + delta)
  store.loadAudit()
}

function reloadVetoes() {
  store.vetoFilter.page = 1
  store.loadVetoes()
}

const fixed = (v, n) => (Number.isFinite(Number(v)) ? Number(v).toFixed(n) : '—')
const barWidth = (v) => (Number.isFinite(Number(v)) ? `${Math.max(0, Math.min(100, Number(v) * 100))}%` : '0%')
// 后端的 ts 是 epoch 秒，归一层已换成毫秒；这里只走 utils/time 的单源口径
const absTime = (ms) => formatDateTime(ms, '—')
const serverTime = (ms) => timeAgoOrDate(ms, Date.now(), '—')

onMounted(() => {
  store.loadAll()
})
</script>

<style scoped>
.gv {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.gv-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.gv-title {
  margin: 0;
  font-size: 19px;
  font-weight: 600;
  color: var(--text-primary);
}
.gv-sub {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-muted);
}
.gv-head-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.gv-card {
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
}
.gv-card-head {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 10px;
}
.gv-card-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}
.gv-note {
  font-size: 11px;
  color: var(--text-muted);
}
.gv-threshold-src {
  font-style: normal;
  color: var(--warning);
}
.gv-err {
  margin: 6px 0;
  font-size: 12px;
  color: var(--danger);
}
.gv-warn {
  margin: 6px 0;
  font-size: 12px;
  color: var(--warning);
}
.gv-kpis {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 10px;
}
.gv-kpi {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
}
.gv-kpi-value {
  font-size: 20px;
  font-weight: 600;
  color: var(--accent-light);
}
.gv-kpi-label {
  font-size: 12px;
  color: var(--text-secondary);
}
.gv-kpi-hint {
  font-size: 11px;
  font-style: normal;
  color: var(--text-muted);
}
.gv-honest {
  margin: 10px 0 0;
  padding: 8px 10px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border-left: 2px solid var(--accent-dim);
  border-radius: var(--radius-sm);
}
.gv-leagues {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 14px;
}
.gv-league-title {
  margin: 0 0 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.gv-dims {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.gv-dim {
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
}
.gv-dim-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.gv-dim-label {
  font-size: 12px;
  color: var(--text-primary);
}
.gv-dim-score {
  margin-left: auto;
  font-size: 13px;
  font-weight: 600;
}
.gv-dim-foot {
  display: flex;
  justify-content: space-between;
  gap: 10px;
  margin-top: 4px;
  font-size: 11px;
  color: var(--text-muted);
}
.gv-bar {
  margin-top: 6px;
  height: 4px;
  border-radius: var(--radius-sm);
  background: var(--bg-tertiary);
  overflow: hidden;
}
.gv-bar-fill {
  display: block;
  height: 100%;
}
.gv-mono {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11px;
  color: var(--text-muted);
}
.gv-mono-dim {
  margin-left: 6px;
}
.gv-flow-name {
  display: block;
  font-size: 12px;
  color: var(--text-primary);
}
.gv-reason {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
}
.gv-gate {
  display: block;
  font-size: 11px;
  font-style: normal;
  color: var(--text-muted);
}
.gv-filters {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: auto;
}
.gv-filter-dim {
  width: 180px;
}
.gv-filter-blocked {
  width: 120px;
}
.gv-pager {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 8px;
}
.gv-pager .gv-note {
  margin-right: auto;
}
.gv-weights {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 14px;
}
.gv-weight-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.gv-weight {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  font-size: 12px;
  color: var(--text-secondary);
}
.gv-weight-value {
  font-size: 12px;
  color: var(--text-primary);
}
.gv-weight-absent {
  color: var(--warning);
}
.gv-foot {
  font-size: 11px;
  color: var(--text-muted);
}
.tone-success,
.tone-success .gv-dim-score,
.tone-success .gv-weight-value {
  color: var(--success);
}
.tone-warning,
.tone-warning .gv-dim-score {
  color: var(--warning);
}
.tone-danger,
.tone-danger .gv-dim-score {
  color: var(--danger);
}
.gv-bar.tone-success .gv-bar-fill { background: var(--success-fill); }
.gv-bar.tone-warning .gv-bar-fill { background: var(--warning-fill); }
.gv-bar.tone-danger .gv-bar-fill { background: var(--danger-fill); }
.gv-bar.tone-unknown .gv-bar-fill { background: var(--text-tertiary); }
</style>
