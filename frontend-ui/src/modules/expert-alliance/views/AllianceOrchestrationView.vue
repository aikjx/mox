<template>
  <div class="aov">
    <header class="aov-head">
      <div>
        <h1 class="aov-title">专家编排台</h1>
        <p class="aov-sub">
          六个网关端点里的编排面：<strong>步骤按固定模板编排，正文来自真实模型分析</strong>。
          每一格的来源都按 contract/orchestration.js 标注，不做整体定性。
        </p>
      </div>
      <div class="aov-actions">
        <el-button :icon="Refresh" :loading="store.loading.stats || store.loading.history" @click="refreshReads">刷新读数</el-button>
      </div>
    </header>

    <EventConnectionStatus :connection="eventStream.connection" @reconnect="eventStream.start()" />
    <el-alert v-if="store.volatility" type="info" :closable="false" show-icon :title="store.volatility" />

    <section v-if="store.liveEvents.length" class="aov-card aov-live">
      <h2 class="aov-h">
        实时事件流
        <span class="aov-live-dot"></span>
        <el-tag size="small" type="success" effect="plain">SSE · 免轮询</el-tag>
      </h2>
      <ul class="aov-events">
        <li v-for="ev in store.liveEvents" :key="ev.id || ev.receivedAt" class="aov-event">
          <code class="aov-event-kind">{{ ev.kind }}</code>
          <span v-if="ev.planId" class="aov-event-plan">
            计划 {{ ev.planId }}<template v-if="ev.from"> · {{ ev.from }} → {{ ev.to }}</template>
            <template v-if="ev.title"> · {{ ev.title }}</template>
          </span>
          <span v-else-if="ev.expertId" class="aov-event-plan">专家 {{ expertNames[ev.expertId] || ev.expertId }}</span>
          <span class="aov-event-time">{{ ev.occurredAt || ev.receivedAt }}</span>
        </li>
      </ul>
    </section>

    <div class="aov-body">
      <section class="aov-card">
        <h2 class="aov-h">编排输入</h2>
        <el-form label-position="top" @submit.prevent>
          <el-form-item label="任务描述（task）">
            <el-input v-model="store.form.task" type="textarea" :rows="3" placeholder="要编排什么？留空后端直接 400" />
          </el-form-item>
          <el-form-item label="任务类型（task_type）">
            <el-select v-model="store.form.taskType" clearable placeholder="缺省 general">
              <el-option v-for="t in store.taskTypeTables" :key="t" :label="t" :value="t" />
            </el-select>
            <span v-if="store.fallbackNote" class="aov-hint">{{ store.fallbackNote }}</span>
          </el-form-item>
          <el-form-item label="融合策略（fusion_strategy）">
            <el-select v-model="store.form.fusionStrategy" clearable placeholder="缺省 weighted">
              <el-option v-for="f in fusionOptions" :key="f" :label="f" :value="f" />
            </el-select>
          </el-form-item>
          <el-form-item label="参与人数上限（max_experts）">
            <el-input-number v-model="store.form.maxExperts" :min="0" :max="20" />
            <span class="aov-hint">0 是合法值：take(0) 得到零位专家，计划照样生成</span>
          </el-form-item>
          <el-form-item label="指定专家（expert_ids，留空＝自动匹配）">
            <el-select v-model="store.form.expertIds" multiple clearable collapse-tags placeholder="不指定">
              <el-option v-for="e in expertOptions" :key="e.value" :label="e.label" :value="e.value" />
            </el-select>
          </el-form-item>
        </el-form>
        <div class="aov-btns">
          <el-button type="primary" :loading="store.loading.run" :disabled="!store.runnable" @click="store.runOrchestrate()">一键编排执行</el-button>
          <el-button :loading="store.loading.plan" :disabled="!!store.validation" @click="store.generatePlan()">只生成计划</el-button>
          <el-button :loading="store.loading.execute" :disabled="!store.executable" @click="store.executePlan()">执行该计划</el-button>
          <el-button text @click="store.reset()">清空</el-button>
        </div>
        <p v-if="store.validation" class="aov-warn">{{ store.validation }}</p>
        <p v-if="store.error.run || store.error.plan || store.error.execute" class="aov-warn">
          {{ store.error.run || store.error.plan || store.error.execute }}
        </p>
      </section>

      <section class="aov-card">
        <h2 class="aov-h">
          依赖链
          <el-tag v-if="store.plan?.status" size="small" :type="tierOf('plan.status')?.tone === 'danger' ? 'danger' : 'info'" effect="plain">
            plan.status = {{ store.plan.status }}（{{ tierLabel('plan.status') }}）
          </el-tag>
        </h2>
        <p v-if="!store.steps.length" class="aov-empty">还没有计划：先"一键编排"或"只生成计划"。</p>
        <template v-else>
          <ol class="aov-steps">
            <li v-for="s in store.steps" :key="s.stepId" class="aov-step">
              <span class="aov-step-name">{{ s.name }}</span>
              <code class="aov-step-type">{{ s.stepType || '—' }}</code>
              <span class="aov-step-desc">{{ s.description }}</span>
              <span class="aov-step-meta">
                专家 {{ s.expertId ? (expertNames[s.expertId] || s.expertId) : '未指派' }}
                · 依赖 {{ s.dependsOn.length ? s.dependsOn.join(', ') : '（首步）' }}
                · {{ orchStatusLabel(s.status) }}
              </span>
            </li>
          </ol>
          <p class="aov-note">{{ store.topologyNote }}</p>
          <p v-for="n in store.expertNotes" :key="n" class="aov-warn">{{ n }}</p>
        </template>
      </section>
    </div>

    <section v-if="store.orchestration" class="aov-card">
      <h2 class="aov-h">编排结果 · {{ store.orchestration.orchestrationId }}</h2>
      <div class="aov-kv">
        <div><span>task_type</span><code>{{ store.orchestration.taskType }}</code></div>
        <div>
          <span>execution.status</span>
          <code>{{ store.orchestration.execution.status }}</code>
          <el-tag size="small" type="danger" effect="plain">{{ tierLabel('orchestrate.execution.status') }}</el-tag>
        </div>
        <div><span>steps</span><code>{{ store.orchestration.execution.stepsCompleted }} / {{ store.orchestration.execution.stepsTotal }}</code></div>
        <div><span>duration_ms</span><code>{{ store.orchestration.execution.durationMs }}</code><el-tag size="small" type="success" effect="plain">{{ tierLabel('orchestrate.execution.duration_ms') }}</el-tag></div>
      </div>
      <div class="aov-chips">
        <el-tag v-for="e in store.orchestration.experts" :key="e.id" size="small" effect="plain">{{ expertNameOr(e, e.id) }}（{{ e.title || '无职称' }}）</el-tag>
        <span v-if="!store.orchestration.experts.length" class="aov-empty">本次没有匹配到专家</span>
      </div>
      <div v-if="store.orchestration.result" class="aov-fusion">
        <p class="aov-line">
          <strong>summary</strong>
          <el-tag size="small" type="warning" effect="plain">{{ tierLabel('orchestrate.result.summary') }}</el-tag>
          <br>{{ store.orchestration.result.summary }}
        </p>
        <ul class="aov-list">
          <li v-for="(k, i) in store.orchestration.result.keyFindings" :key="`k${i}`">{{ k }}</li>
        </ul>
        <ul class="aov-list">
          <li v-for="(r, i) in store.orchestration.result.recommendations" :key="`r${i}`">{{ r }}</li>
        </ul>
        <p class="aov-line">
          confidence <code>{{ store.orchestration.result.confidence }}</code>
          <el-tag size="small" type="danger" effect="plain">{{ tierLabel('orchestrate.result.confidence') }}</el-tag>
          · fusion_strategy <code>{{ store.orchestration.result.fusionStrategy }}</code>
          <el-tag size="small" type="success" effect="plain">{{ tierLabel('orchestrate.result.fusion_strategy') }}</el-tag>
        </p>
      </div>
      <el-alert v-if="store.disclaimer" type="warning" :closable="false" show-icon :title="store.disclaimer" />
    </section>

    <section v-if="store.execution" class="aov-card">
      <h2 class="aov-h">
        计划执行 · {{ store.execution.executionId }}
        <el-tag size="small" :type="store.outcome.failed ? 'danger' : 'success'" effect="dark">{{ orchStatusLabel(store.outcome.status) || '—' }}</el-tag>
      </h2>
      <el-alert v-if="store.outcome.failed" type="error" :closable="false" show-icon :title="store.outcome.error || '拓扑排序失败'" :description="store.outcome.note" />
      <el-alert v-else type="info" :closable="false" :title="store.outcome.note" />
      <ol class="aov-steps">
        <li v-for="s in store.execution.stepsExecuted" :key="s.stepId" class="aov-step">
          <span class="aov-step-name">{{ s.name }}</span>
          <el-tag size="small" type="danger" effect="plain">{{ orchStatusLabel(s.status) }}（{{ tierLabel('execute.steps_executed[].status') }}）</el-tag>
          <span class="aov-step-meta">
            耗时 {{ s.durationMs }}ms（{{ tierLabel('execute.steps_executed[].duration_ms') }}）
            · 置信度 {{ s.result?.confidence ?? '—' }}（{{ tierLabel('execute.steps_executed[].result.confidence') }}）
            · 步骤专家 {{ s.result?.expert ? s.result.expert.name : 'null（' + tierLabel('execute.steps_executed[].result.expert') + '）' }}
          </span>
          <p v-if="s.result" class="aov-step-desc">{{ s.result.summary }}</p>
        </li>
      </ol>
      <div v-if="store.execution.finalResult" class="aov-fusion">
        <p class="aov-line">{{ store.execution.finalResult.summary }}</p>
        <ul class="aov-list">
          <li v-for="(k, i) in store.execution.finalResult.keyFindings" :key="`ek${i}`">{{ k }}</li>
        </ul>
      </div>
    </section>

    <div class="aov-wide">
      <section class="aov-card">
        <h2 class="aov-h">编排统计（进程内）</h2>
        <p v-if="!store.stats" class="aov-empty">{{ store.error.stats || '尚未读取' }}</p>
        <template v-else>
          <div class="aov-cells">
            <div v-for="c in store.statCells" :key="c.key" class="aov-cell" :class="{ 'is-zero': c.zero }">
              <span class="aov-cell-label">{{ c.label }}</span>
              <span class="aov-cell-value">{{ c.value }}</span>
              <el-tag v-if="c.zero" size="small" type="danger" effect="plain">无写入路径</el-tag>
            </div>
          </div>
          <p class="aov-line">
            成功率 <code>{{ store.stats.successRate }}</code> · 平均耗时 <code>{{ store.stats.avgDurationMs }}</code>ms
            · 平均步数 <code>{{ store.stats.avgStepsPerPlan }}</code> · 执行次数 <code>{{ store.stats.totalExecutions }}</code>
          </p>
          <p v-for="z in store.zeroCounterNotes" :key="z.key" class="aov-note">{{ z.text }}</p>
          <p v-if="store.statusSplit" class="aov-warn">{{ store.statusSplit }}</p>
          <div v-if="store.stats.topUsedExperts.length" class="aov-chips">
            <el-tag v-for="t in store.stats.topUsedExperts" :key="t.expertId" size="small" effect="plain">
              {{ expertNames[t.expertId] || t.expertId }} × {{ t.usageCount }}
            </el-tag>
          </div>
          <p class="aov-line">融合策略分布 <code>{{ jsonBrief(store.stats.fusionStrategyDistribution) }}</code></p>
          <p class="aov-line">任务类型分布 <code>{{ jsonBrief(store.stats.taskTypeDistribution) }}</code></p>
        </template>
      </section>

      <section class="aov-card">
        <h2 class="aov-h">执行历史（仅本次进程）</h2>
        <div class="aov-btns">
          <el-select v-model="store.filters.status" clearable placeholder="status 过滤" size="small" @change="store.setHistoryFilter('status', $event)">
            <!-- §5.54 F25：档名交 ORCH_STATUS，文案交 orchStatusLabel（原来是内联三档字面量表＋把 wire 串直接当标签印出） -->
            <el-option v-for="s in [ORCH_STATUS.COMPLETED, ORCH_STATUS.PARTIAL, ORCH_STATUS.FAILED]" :key="s" :label="orchStatusLabel(s)" :value="s" />
          </el-select>
          <el-input v-model="store.filters.taskType" clearable placeholder="task_type 过滤" size="small" @change="store.setHistoryFilter('taskType', $event)" />
        </div>
        <p v-if="store.historyAnomaly" class="aov-warn">{{ store.historyAnomaly }}</p>
        <table class="aov-table">
          <thead>
            <tr><th>status</th><th>task_type</th><th>步数</th><th>耗时</th><th>创建</th><th>专家</th></tr>
          </thead>
          <tbody>
            <tr v-for="r in store.history.records" :key="r.executionId">
              <td>{{ r.status }}</td>
              <td>{{ r.taskType }}</td>
              <td>{{ r.stepsCompleted }}/{{ r.stepsTotal }}</td>
              <td>{{ r.durationMs }}ms</td>
              <td>{{ r.createdAt }}</td>
              <td>{{ r.expertIds.length }}</td>
            </tr>
            <tr v-if="!store.history.records.length"><td colspan="6" class="aov-empty">{{ store.error.history || '本页没有记录' }}</td></tr>
          </tbody>
        </table>
        <div class="aov-btns">
          <el-button size="small" :disabled="store.history.page <= 1 || store.loading.history" @click="store.loadHistory(store.history.page - 1)">上一页</el-button>
          <span class="aov-note">第 {{ store.history.page }} 页 · 共 {{ store.historyPages.total }} 行 · {{ store.historyPages.pages }} 页（页码总数由前端按 total/page_size 算得，后端不回 total_pages）</span>
          <el-button size="small" :disabled="store.history.page >= store.historyPages.pages || store.loading.history" @click="store.loadHistory(store.history.page + 1)">下一页</el-button>
        </div>
      </section>
    </div>

    <section class="aov-card">
      <h2 class="aov-h">这一页与后端的对应关系</h2>
      <ul class="aov-notes">
        <li>
          拓扑排序是<strong>真的</strong>：<code>topological_sort</code>（:39-92）用 Kahn 算法并带环检测；
          但 <code>generate_plan</code> 产出的依赖恒为单链（:175-178），所以这条链不可能成环——
          执行仍可能因专家不可用、模型失败或治理否决而失败。
        </li>
        <li>
          选人是<strong>真的</strong>：<code>compute_match_score</code> 对注册表打分，门槛 0.2、take(max_experts)（:544-551）。
          步骤按候选 argmax 绑定专家（:159-172）；去幻影化后每步直接走真实模型咨询，结果行不再回带 expert 字段。
        </li>
        <li>
          步骤正文是<strong>真实模型咨询</strong>：去幻影化后每步调真实模型，喂给融合的中间块为 5 键（:452-453，evidence_kind=model_response），
          不再是 step_type 查表的写死文案，confidence 也不再恒 0.85。
        </li>
        <li>
          <code>plan/execute</code> 成环时后端返回 <strong>HTTP 200 + status:"failed"</strong>（:396-405、:457-471），
          所以这一页判成败只读 body，状态码不参与。
        </li>
        <li>
          只有 <code>plans_ready</code> 恒为 0：plan.status 的写入路径为 draft（:204）、running（:410）、
          终态 failed/completed/partial（:457-464）；failed 已被统计与历史如实记录。
        </li>
        <li>
          历史分页<strong>不走</strong>后端的 <code>parse_pagination</code>：page/page_size 直接 parse（:903-904），
          没有 1..=200 上限，page_size=0 会让本页恒空而 total 非零。本页把 page_size 夹到 200 以内。
        </li>
        <li>
          <code>orchestration/plugins</code> 那条路由没有挂：六条硬编码数组、version 一律 2.0.0，
          声明的 <code>webhook_url</code>/<code>retry_count</code> 无人读取，仍不作为可用能力展示。
        </li>
      </ul>
      <h3 class="aov-h3">常量与模拟字段清单（与源码逐条对齐，后端改掉即测试先红）</h3>
      <ul class="aov-notes">
        <li v-for="s in store.simulatedLegend" :key="s.id">
          <code>{{ s.at }}</code> {{ s.field }} —— {{ s.text }}
        </li>
      </ul>
    </section>
  </div>
</template>

<script setup>
// 编排台：装配 + 来源标注。所有规则、键集、常量清单都在 contract/orchestration.js，
// 本文件不重复后端边界，只把它算好的判据渲染出来。
import { computed, onMounted, onUnmounted } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { EventConnectionStatus } from '@/modules/expert-alliance/components'
import { FUSION_STRATEGY } from '@/modules/expert-alliance/contract'
import { ORCH_PROVENANCE } from '@/modules/expert-alliance/contract'
import { expertNameOr, orchStatusLabel, ORCH_STATUS } from '@/modules/expert-alliance/contract'
import { useAllianceOrchStore } from '@/modules/expert-alliance/store'
import { useAllianceExpertsStore } from '@/modules/expert-alliance/store'
import { useAllianceEventStream } from '@/modules/expert-alliance/composables/useAllianceEventStream'

const store = useAllianceOrchStore()
const expertStore = useAllianceExpertsStore()

const fusionOptions = computed(() => Object.values(FUSION_STRATEGY))
// 候选与 id→名字映射都指回 store 那一份，视图不再自己 map 注册表 name：
// 注册表里有两行的 name 在写入侧就丢成了 '???????'（见 contract/graph.js）。
const expertOptions = computed(() => expertStore.expertOptions)
const expertNames = computed(() => expertStore.expertNames)

const tierLabel = (path) => store.provenance(path)?.label || '真实计算'
const tierOf = (path) => {
  const p = store.provenance(path)
  return p ? { ...p, tone: ORCH_PROVENANCE[p.tier]?.tone } : null
}
const jsonBrief = (obj) => {
  const entries = Object.entries(obj || {})
  if (!entries.length) return '（空）'
  return entries.map(([k, v]) => `${k}:${v}`).join(' · ')
}

async function refreshReads() {
  await Promise.all([store.loadStats(), store.loadHistory(1)])
}

// T4 SSE 真实挂载：订阅本租户业务事件帧（PlanCreated/PlanStatusChanged/ExpertRegistered/ExpertDisabled）。
// onEvent 把帧交给 store.applyAllianceEvent → liveEvents 立即可见 + 带 plan_id 的帧防抖真拉统计/历史，
// 帧只提示刷新。恢复控制器在开流和缺口时重新读取，状态栏呈现错误与手动重连。
const eventStream = useAllianceEventStream({
  onEvent: (kind, envelope) => store.applyAllianceEvent(kind, envelope),
  refresh: async () => {
    await refreshReads()
    if (store.error.stats || store.error.history) throw new Error(store.error.stats || store.error.history)
  }
})

onMounted(() => {
  refreshReads()
  if (!expertStore.experts.length) expertStore.loadExperts()
  eventStream.start()
})

onUnmounted(() => { eventStream.stop(); store.clearLiveEvents() })
</script>

<style scoped>
.aov { display: flex; flex-direction: column; gap: 12px; }
.aov-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.aov-title { margin: 0; font-size: 20px; color: var(--text-primary); }
.aov-sub { margin: 4px 0 0; font-size: 12px; line-height: 1.6; color: var(--text-secondary); }
.aov-actions { display: flex; align-items: center; gap: 8px; }
.aov-body { display: grid; gap: 12px; align-items: start; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); }
.aov-wide { display: grid; gap: 12px; align-items: start; grid-template-columns: repeat(auto-fit, minmax(380px, 1fr)); }
.aov-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  min-width: 0;
}
.aov-h { margin: 0; font-size: 14px; color: var(--text-primary); display: flex; align-items: center; gap: 8px; }
.aov-h3 { margin: 8px 0 0; font-size: 13px; color: var(--text-primary); }
.aov-btns { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.aov-hint { margin-left: 8px; font-size: 11px; color: var(--text-secondary); }
.aov-note { margin: 0; font-size: 11px; line-height: 1.6; color: var(--text-secondary); }
.aov-warn { margin: 0; font-size: 11px; line-height: 1.6; color: var(--danger); }
.aov-empty { margin: 0; font-size: 12px; color: var(--text-secondary); }
.aov-line { margin: 0; font-size: 12px; line-height: 1.7; color: var(--text-primary); }
.aov-kv { display: grid; gap: 6px; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); }
.aov-kv > div { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-secondary); }
.aov-chips { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.aov-steps { margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 6px; }
.aov-step { font-size: 12px; color: var(--text-primary); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.aov-step-name { font-weight: 600; }
.aov-step-type { font-size: 11px; color: var(--accent-light); }
.aov-step-desc { font-size: 11px; color: var(--text-secondary); width: 100%; }
.aov-step-meta { font-size: 11px; color: var(--text-secondary); }
.aov-fusion { display: flex; flex-direction: column; gap: 4px; padding: 8px; border: 1px dashed var(--border); border-radius: var(--radius-sm); }
.aov-list { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 3px; font-size: 12px; color: var(--text-secondary); }
.aov-live { flex-direction: column; }
.aov-live-dot { width: 8px; height: 8px; border-radius: 50%; background: var(--success); display: inline-block; }
.aov-events { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 4px; font-size: 12px; color: var(--text-primary); }
.aov-event { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.aov-event-kind { font-size: 11px; color: var(--accent-light); }
.aov-event-plan { font-size: 12px; color: var(--text-secondary); }
.aov-event-time { font-size: 11px; color: var(--text-secondary); margin-left: auto; }
.aov-cells { display: grid; gap: 6px; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); }
.aov-cell { display: flex; flex-direction: column; gap: 2px; font-size: 12px; color: var(--text-secondary); }
.aov-cell-value { font-size: 16px; color: var(--text-primary); }
.aov-cell.is-zero .aov-cell-value { color: var(--danger); }
.aov-table { width: 100%; border-collapse: collapse; font-size: 12px; }
.aov-table th, .aov-table td { padding: 4px 6px; border-bottom: 1px solid var(--border); text-align: left; color: var(--text-primary); }
.aov-table th { color: var(--text-secondary); font-weight: 500; }
.aov-notes { margin: 0; padding-left: 16px; display: flex; flex-direction: column; gap: 6px; font-size: 12px; line-height: 1.6; color: var(--text-secondary); }
.aov-notes code { font-size: 11px; color: var(--accent-light); }
@media (max-width: 1180px) {
  .aov-body { grid-template-columns: minmax(0, 1fr); }
}
</style>
