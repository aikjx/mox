<template>
  <div class="acw">
    <el-tabs :model-value="store.mode" class="acw-tabs" @update:model-value="store.setMode($event)">
      <el-tab-pane v-for="m in modes" :key="m.key" :name="m.key" :label="m.label" />
    </el-tabs>

    <p class="acw-outcome">{{ store.current.outcome }}</p>

    <div class="acw-form">
      <label class="acw-field">
        <span class="acw-label">{{ store.current.fieldLabel }}</span>
        <el-input
          v-model="store.input[store.textField]"
          type="textarea"
          :rows="3"
          :placeholder="store.current.placeholder"
          :disabled="store.loading.run"
        />
      </label>

      <div v-for="c in store.controlList" :key="c.wire" class="acw-control">
        <span class="acw-label">{{ c.label }}</span>
        <el-input-number v-model="store.controls[c.wire]" :min="c.min" :max="c.max" :step="1" size="small" />
        <em class="acw-hint">后端 {{ c.wire }} 允许 {{ c.min }}–{{ c.max }}，超出会被夹到边界</em>
      </div>

      <div v-for="f in store.constraintList" :key="f.key" class="acw-control">
        <span class="acw-label">{{ f.label }}</span>
        <el-switch v-if="f.kind === 'switch'" v-model="store.controls[f.key]" :disabled="store.loading.run" size="small" />
        <el-input-number
          v-else
          v-model="store.controls[f.key]"
          :min="f.min"
          :max="f.max"
          :step="f.step"
          :value-on-clear="null"
          :disabled="store.loading.run"
          size="small"
        />
        <em class="acw-hint">constraints.{{ f.key }}：{{ f.hint }}</em>
      </div>

      <label v-for="f in optionalTextFields" :key="f.wire" class="acw-field">
        <span class="acw-label">{{ f.label }}（{{ f.hint }}）</span>
        <el-input v-model="store.input[f.wire]" size="small" :placeholder="f.placeholder" :disabled="store.loading.run" />
      </label>

      <div v-if="store.needsExperts" class="acw-picker">
        <div class="acw-picker-head">
          <span class="acw-label">{{ expertPickerLabel }}</span>
          <el-button v-if="store.pickedCount" size="small" text @click="store.clearExperts()">清空（{{ store.pickedCount }}）</el-button>
          <em v-if="store.capacityNote" class="acw-capacity">{{ store.capacityNote }}</em>
        </div>
        <div v-if="!experts.length" class="acw-muted">专家列表尚未加载，可先留空由后端自动匹配。</div>
        <div v-else class="acw-chips">
          <button
            v-for="e in experts"
            :key="e.id"
            type="button"
            class="acw-chip"
            :class="{ 'is-picked': isPicked(e.id) }"
            :disabled="store.loading.run"
            @click="store.toggleExpert(e.id)"
          >
            <b>{{ e.name }}</b>
            <span>{{ e.title || '未填头衔' }} · {{ availabilityLabel(e) }}</span>
          </button>
        </div>
      </div>

      <div class="acw-run">
        <el-button type="primary" :loading="store.loading.run" :disabled="!store.runnable" @click="store.run()">
          运行协作
        </el-button>
        <span v-if="store.validation" class="acw-warn">{{ store.validation }}</span>
      </div>
    </div>

    <el-alert v-if="store.error.run" type="error" show-icon :closable="false" :title="`协作失败：${store.error.run}`" />
    <el-alert
      v-else-if="store.blocked"
      type="warning"
      show-icon
      :closable="false"
      title="结果被治理闸门拦截"
      :description="blockedNote"
    />

    <!-- 路由：只有候选与推荐，没有专家回复 -->
    <section v-if="store.resultKind === 'routing'" class="acw-result">
      <div class="acw-recommend">
        <b>{{ recommendationTitle }}</b>
        <p>{{ store.result.recommendation.reason }}</p>
      </div>
      <p class="acw-muted">共扫描 {{ store.result.totalScanned }} 位在册专家，返回 {{ store.result.candidates.length }} 位候选；路由只排序作答，不产出回复。</p>
      <ul class="acw-rows">
        <li v-for="row in store.result.candidates" :key="row.id" class="acw-row">
          <div class="acw-row-main">
            <b>{{ row.name }}</b>
            <span>{{ row.title || '未填头衔' }} · 匹配度 {{ row.matchScore.toFixed(3) }} · 评分 {{ row.avgRating.toFixed(1) }} · 累计 {{ row.totalConsultations }} 次</span>
          </div>
          <em class="acw-side">{{ row.status }} · 响应 {{ row.avgResponseMinutes }} 分钟 · 负载 {{ row.currentLoad }}</em>
        </li>
      </ul>
    </section>

    <!-- 融合：多专家并行作答 + 加权投票 -->
    <section v-else-if="store.resultKind === 'fusion'" class="acw-result">
      <div class="acw-fused">
        <span class="acw-tag">融合结论</span>
        <p class="acw-pre">{{ store.result.fusion.summary }}</p>
        <div class="acw-meta">
          <span>共识度 {{ consensusDisplay }}</span>
          <span>融合置信度 {{ confidenceText(store.result.fusion.confidence) }}</span>
          <span>{{ store.result.contributions.length }} 位专家作答</span>
        </div>
        <p class="acw-dominant">{{ store.result.fusion.dominantView || '无主导观点' }}</p>
        <ul v-if="store.result.fusion.alternativeViews.length" class="acw-list">
          <li v-for="(v, i) in store.result.fusion.alternativeViews" :key="`alt-${i}`" class="acw-pre">{{ v }}</li>
        </ul>
      </div>
      <div v-for="row in store.result.contributions" :key="row.id" class="acw-answer">
        <div class="acw-answer-head">
          <b>{{ row.name }}</b>
          <span>匹配度 {{ row.matchScore.toFixed(3) }} · {{ answerSourceText(row.answer) }} · 置信度 {{ confidenceText(row.answer.confidence) }}</span>
        </div>
        <p class="acw-pre">{{ row.answer.solution }}</p>
      </div>
    </section>

    <!-- 辩论：逐轮交锋与裁决 -->
    <section v-else-if="store.resultKind === 'debate'" class="acw-result">
      <div class="acw-recommend">
        <b>{{ store.result.verdict.winner || '未产生胜方' }}胜出</b>
        <p class="acw-pre">{{ store.result.verdict.summary }}</p>
        <div class="acw-meta">
          <span>{{ store.result.rounds }} 轮</span>
          <span>{{ store.result.verdict.consensusLevel }}</span>
          <span>{{ store.result.participants.length }} 位上场</span>
        </div>
      </div>
      <ul class="acw-rows">
        <li v-for="p in store.result.participants" :key="p.id" class="acw-row">
          <div class="acw-row-main">
            <b>{{ p.name }}</b>
            <span>{{ p.side === 'pro' ? '正方' : '反方' }}</span>
          </div>
          <em class="acw-side">{{ p.finalScore.toFixed(2) }} 分</em>
        </li>
      </ul>
      <ol class="acw-rounds">
        <li v-for="r in store.result.log" :key="r.round" class="acw-round">
          <div class="acw-round-head">第 {{ r.round }} 轮 · 正方 {{ r.proScore.toFixed(2) }} / 反方 {{ r.conScore.toFixed(2) }}</div>
          <p class="acw-pre">{{ r.proArgument }}</p>
          <p class="acw-pre">{{ r.conArgument }}</p>
        </li>
      </ol>
    </section>

    <!-- 复杂度分析 -->
    <section v-else-if="store.resultKind === 'complexity'" class="acw-result">
      <div class="acw-complexity">
        <span class="acw-tag" :class="`is-${complexityLevel(store.result.complexity.time).tag}`">{{ store.result.complexity.bigO }}</span>
        <p class="acw-pre">{{ store.result.complexity.explanation }}</p>
        <div class="acw-meta">
          <span>时间 {{ store.result.complexity.time }}</span>
          <span>空间 {{ store.result.complexity.space }}</span>
          <span>可行性 {{ confidenceText(store.result.feasibility.score) }}</span>
        </div>
      </div>
      <div v-if="store.result.feasibility.blockers.length" class="acw-block">
        <b>阻塞项</b>
        <ul class="acw-list">
          <li v-for="(b, i) in store.result.feasibility.blockers" :key="`blk-${i}`">{{ b }}</li>
        </ul>
      </div>
      <div class="acw-block">
        <b>风险提示</b>
        <ul class="acw-list">
          <li v-for="(r, i) in store.result.feasibility.risks" :key="`risk-${i}`">{{ r }}</li>
        </ul>
      </div>
      <div class="acw-block">
        <b>优化建议</b>
        <ul class="acw-list">
          <li v-for="(s, i) in store.result.suggestions" :key="`sug-${i}`">{{ s }}</li>
        </ul>
      </div>
      <p class="acw-muted">推荐专家：{{ recommendedText }}</p>
    </section>

    <!-- 咨询类：单专家与智能咨询共用一套正文结构 -->
    <section v-else-if="store.resultKind === 'answer'" class="acw-result">
      <div class="acw-answer-head">
        <b>{{ answerExpertName }}</b>
        <span>{{ answerSourceText(store.result.answer) }} · 置信度 {{ confidenceText(store.result.answer.confidence) }}</span>
      </div>
      <p v-if="store.result.intent" class="acw-meta"><span>意图分类：{{ intentLabel(store.result.intent) }}</span></p>
      <div class="acw-answer">
        <p class="acw-pre">{{ store.result.answer.analysis }}</p>
        <p class="acw-pre is-solution">{{ store.result.answer.solution }}</p>
      </div>
      <div v-if="store.result.answer.actionItems?.length" class="acw-block">
        <b>行动项</b>
        <ul class="acw-list">
          <li v-for="(a, i) in store.result.answer.actionItems" :key="`act-${i}`">{{ a }}</li>
        </ul>
      </div>
      <div v-if="store.result.answer.risk" class="acw-block">
        <b>风险评估（后端固定为 medium 档）</b>
        <ul class="acw-list">
          <li>{{ store.result.answer.risk.technical }}</li>
          <li>{{ store.result.answer.risk.schedule }}</li>
          <li>{{ store.result.answer.risk.resource }}</li>
        </ul>
      </div>
      <ul v-if="store.result.answer.references.length" class="acw-list acw-refs">
        <li v-for="(ref, i) in store.result.answer.references" :key="`ref-${i}`">{{ ref }}</li>
      </ul>
      <p v-if="store.result.relatedExperts?.length" class="acw-muted">相关专家：{{ relatedText }}</p>
    </section>

    <div v-if="store.history.length" class="acw-history">
      <div class="acw-history-head">
        <span class="acw-label">本次运行记录</span>
        <em class="acw-muted">后端会把协作写入专家会话（multi/debate/smart/single），此处只是本页面内的最近 {{ historyCap }} 条</em>
      </div>
      <ul class="acw-rows">
        <li v-for="(h, i) in store.history" :key="`h-${i}`" class="acw-row">
          <div class="acw-row-main">
            <b>{{ h.modeLabel }}</b>
            <span>{{ h.text || '（无文本回显）' }}</span>
          </div>
          <em class="acw-side" :class="{ 'is-failed': !h.ok }">{{ h.ok ? h.refId || '已完成' : '失败' }}</em>
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup>
// 智能协作工作台：六个后端原生协作端点的唯一入口，替代存量两份互不兼容的实现。
// 直接绑协作 store——六模式的输入/控件/结果状态量大，逐 prop 透传只会把 store 形状抄进视图。
import { computed } from 'vue'
import { COLLAB_MODES, answerSourceText, confidenceText, consensusText, intentLabel, complexityLevel } from '@/modules/expert-alliance/contract'
import { HISTORY_CAP } from '@/modules/expert-alliance/store'

const props = defineProps({
  experts: { type: Array, default: () => [] },
  store: { type: Object, required: true }
})

const modes = COLLAB_MODES

// 可选文本入参：按模式 wires 声明决定是否出现，视图不硬编码后端字段名
const OPTIONAL_TEXT_FIELDS = {
  domain: { label: '领域过滤', hint: '后端按领域名子串匹配', placeholder: '留空表示不限领域' },
  context: { label: '补充上下文', hint: '与问题一并发给专家', placeholder: '例如：现有栈为 Spring Cloud + K8s，需保留灰度发布' }
}
const optionalTextFields = computed(() =>
  (props.store.current.wires || [])
    .filter((wire) => OPTIONAL_TEXT_FIELDS[wire] && wire !== props.store.current.field)
    .map((wire) => ({ wire, ...OPTIONAL_TEXT_FIELDS[wire] }))
)

const isPicked = (id) => props.store.input.expertIds.includes(id)
const expertPickerLabel = computed(() =>
  props.store.current.expertChoice === 'one' ? '选择 1 位专家' : '参与专家（不选则由后端自动匹配）'
)
const availabilityLabel = (e) => (e.online ? '在线' : e.status === 'busy' ? '忙碌' : '离线')

const recommendationTitle = computed(() => {
  const rec = props.store.result?.recommendation
  if (!rec?.expertId) return '无满足约束的可用专家'
  const hit = props.store.result.candidates.find((c) => c.id === rec.expertId)
  return `推荐 ${hit?.name || rec.expertId}`
})

const consensusDisplay = computed(() =>
  consensusText(props.store.result?.fusion, props.store.result?.contributions.length)
)

/**
 * 拦截说明：后端把被否决的回复换成带原因的替换文本，正文仍会返回但不是专家原文，
 * 所以文案说「替换后的拦截说明」，不谎称没有内容出网。
 */
const blockedNote = computed(() => {
  const r = props.store.result || {}
  const answers = [...(r.contributions || []).map((c) => c.answer), r.answer, r.fusion].filter(Boolean)
  const reasons = [...new Set(answers.map((a) => a.vetoReason).filter(Boolean))]
  return `后端治理闸门否决了原始回复${reasons.length ? `（原因：${reasons.join('、')}）` : ''}，下方展示的是替换后的拦截说明，不是专家原文。`
})

const answerExpertName = computed(() => {
  const r = props.store.result || {}
  return r.expertName || r.expert?.name || '匹配专家'
})

const relatedText = computed(() =>
  (props.store.result?.relatedExperts || []).map((e) => `${e.name}（${e.title || '未填头衔'}）`).join('、')
)

const recommendedText = computed(() =>
  (props.store.result?.recommendedExperts || [])
    .map((e) => `${e.name} 匹配度 ${e.matchScore.toFixed(3)}`)
    .join('、') || '后端未匹配到相关专家'
)

const historyCap = HISTORY_CAP
</script>

<style scoped>
.acw { display: flex; flex-direction: column; gap: 12px; }
.acw-tabs { margin-top: -8px; }
.acw-outcome { margin: 0; font-size: 12px; color: var(--text-secondary); }
.acw-form { display: flex; flex-direction: column; gap: 10px; }
.acw-field { display: flex; flex-direction: column; gap: 4px; }
.acw-label { font-size: 12px; color: var(--text-secondary); }
.acw-control { display: flex; align-items: center; gap: 8px; }
.acw-hint { font-size: 11px; font-style: normal; color: var(--text-muted); }
.acw-run { display: flex; align-items: center; gap: 10px; }
.acw-warn { font-size: 12px; color: var(--warning); }
.acw-muted { margin: 0; font-size: 11px; color: var(--text-muted); }

.acw-picker { display: flex; flex-direction: column; gap: 6px; }
.acw-picker-head { display: flex; align-items: center; gap: 8px; }
.acw-capacity { font-size: 11px; font-style: normal; color: var(--warning); }
.acw-chips { display: flex; flex-wrap: wrap; gap: 6px; max-height: 148px; overflow-y: auto; }
.acw-chip {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
  cursor: pointer;
  transition: border-color var(--dur-2) var(--ease);
}
.acw-chip b { font-size: 12px; color: var(--text-primary); }
.acw-chip span { font-size: 11px; color: var(--text-muted); }
.acw-chip.is-picked { border-color: var(--accent); background: var(--accent-light); }

.acw-result { display: flex; flex-direction: column; gap: 10px; }
.acw-recommend, .acw-fused, .acw-complexity {
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-card);
}
.acw-recommend p, .acw-fused p, .acw-complexity p { margin: 6px 0 0; font-size: 12px; color: var(--text-secondary); }
.acw-tag {
  display: inline-block;
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  font-size: 11px;
  color: var(--on-accent);
  background: var(--accent-fill);
}
.acw-tag.is-danger { background: var(--danger-fill); color: var(--on-danger); }
.acw-tag.is-warning { background: var(--warning-fill); color: var(--on-warning); }
.acw-tag.is-success { background: var(--success-fill); color: var(--on-success); }
.acw-meta { display: flex; flex-wrap: wrap; gap: 10px; margin-top: 6px; font-size: 11px; color: var(--text-muted); }
.acw-dominant { margin: 6px 0 0; font-size: 12px; color: var(--text-primary); }
.acw-pre { margin: 6px 0 0; font-size: 12px; line-height: 1.6; color: var(--text-secondary); white-space: pre-wrap; }
.acw-pre.is-solution { color: var(--text-primary); }

.acw-rows, .acw-list, .acw-rounds { display: flex; flex-direction: column; gap: 6px; margin: 0; padding: 0; list-style: none; }
.acw-list { padding-left: 14px; list-style: disc; }
.acw-list li { font-size: 12px; line-height: 1.5; color: var(--text-secondary); }
.acw-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
}
.acw-row-main { display: flex; flex-direction: column; min-width: 0; }
.acw-row-main b { font-size: 13px; color: var(--text-primary); }
.acw-row-main span { font-size: 11px; color: var(--text-muted); }
.acw-side { font-size: 11px; font-style: normal; color: var(--text-secondary); flex-shrink: 0; }
.acw-side.is-failed { color: var(--danger); }

.acw-round { padding: 8px 10px; border: 1px solid var(--border); border-radius: var(--radius-sm); background: var(--bg-secondary); }
.acw-round-head { font-size: 12px; color: var(--text-primary); }
.acw-answer { padding: 8px 10px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-secondary); }
.acw-answer-head { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; }
.acw-answer-head b { font-size: 13px; color: var(--text-primary); }
.acw-answer-head span { font-size: 11px; color: var(--text-muted); }
.acw-block b { font-size: 12px; color: var(--text-primary); }
.acw-refs { padding-left: 18px; }
.acw-history { display: flex; flex-direction: column; gap: 6px; }
.acw-history-head { display: flex; align-items: baseline; gap: 8px; }
</style>
