<template>
  <section class="agt">
    <h2 class="agt-title">最优团队组建</h2>
    <p class="agt-dim">
      <code>POST /api/expert-graph/optimal-team</code> · 带权集合覆盖贪心（selection_strategy 由后端回传）
    </p>

    <!-- 表单按契约字段表生成：新增/删除字段只改 contract/graph.js，不在这里各写一份 -->
    <div class="agt-form">
      <div v-for="f in fields" :key="f.key" class="agt-field">
        <label class="agt-label" :for="`agt-${f.key}`">
          {{ f.label }}
          <small v-if="f.key === 'max_members' || f.key === 'min_rating'" class="agt-backend">后端不校验</small>
        </label>

        <el-select v-if="f.kind === 'tags'" :id="`agt-${f.key}`" multiple filterable allow-create default-first-option
          :model-value="draftOf(f.key)" :placeholder="tagPlaceholder(f.key)" size="small" class="agt-wide"
          @change="(v) => store.setTeamValue(localOf(f.key), v)">
          <el-option v-for="o in optionsOf(f.key)" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>

        <el-input-number v-else-if="f.kind === 'number'" :id="`agt-${f.key}`" :model-value="draftOf(f.key)"
          :min="f.min" :max="f.max" :step="f.step || 1" :precision="f.key === 'min_rating' ? 1 : 0" size="small"
          controls-position="right" @change="(v) => store.setTeamValue(localOf(f.key), v)" />

        <el-input v-else :id="`agt-${f.key}`" :model-value="draftOf(f.key)" type="textarea" :rows="2"
          placeholder="例如：为数据仓库做查询优化并补齐调度治理" @input="(v) => store.setTeamValue('goal', v)" />

        <p class="agt-hint">{{ hintOf(f.key) }}</p>
      </div>
    </div>

    <div class="agt-actions">
      <el-button type="primary" size="small" :loading="store.loading.team" :disabled="!!store.teamProblem"
        @click="submit">
        组建团队
      </el-button>
      <span v-if="store.teamProblem" class="agt-warn">{{ store.teamProblem }}</span>
    </div>

    <el-alert v-if="store.error.team" class="agt-alert" type="error" show-icon :closable="false" title="组建失败"
      :description="store.error.team" />

    <template v-if="team">
      <el-alert v-if="!team.members.length" class="agt-alert" type="warning" show-icon :closable="false"
        title="没有专家入选" :description="emptyReason" />
      <p v-else class="agt-summary">
        {{ coverageText(team.coverage) }} · 团队总分 {{ team.teamScore.toFixed(3) }}
        <span class="agt-dim">（{{ team.teamId }} · {{ team.createdAt }}）</span>
      </p>
      <ol v-if="team.members.length" class="agt-members">
        <li v-for="m in team.members" :key="m.id" class="agt-member">
          <header class="agt-member-head">
            <button class="agt-jump" type="button" @click="store.selectNode(m.id)">{{ m.name || m.id }}</button>
            <el-tag size="small" effect="plain">{{ roleLabel(m.role) }}</el-tag>
            <span class="agt-dim">评分 {{ m.avgRating.toFixed(2) }} · 贡献 {{ m.matchScore.toFixed(3) }}</span>
          </header>
          <p class="agt-dim">{{ m.title || '后端未填头衔' }}</p>
          <p class="agt-covers">
            <span v-for="s in m.coveredSkills" :key="`s-${s}`" class="agt-chip">{{ s }}</span>
            <span v-for="d in m.coveredDomains" :key="`d-${d}`" class="agt-chip is-domain">{{ d }}</span>
            <span v-if="!m.coveredSkills.length && !m.coveredDomains.length" class="agt-dim">未命中任何需求项（贪心补位）</span>
          </p>
        </li>
      </ol>
    </template>
  </section>
</template>

<script setup>
// 最优团队面板：入参拼装交给 contract/graph.js，本组件只管渲染字段表与结果。
import { computed } from 'vue'
import {
  OPTIMAL_TEAM_BACKEND_RULES, OPTIMAL_TEAM_FIELDS, coverageText, optimalTeamRoleLabel
} from '@/modules/expert-alliance/contract'

const props = defineProps({
  store: { type: Object, required: true },
  skillOptions: { type: Array, default: () => [] }
})
const store = props.store

const fields = computed(() => OPTIMAL_TEAM_FIELDS.filter((f) => f.mounted))
const draft = computed(() => store.teamDraft)
const team = computed(() => store.team)

// wire 字段名 → store 草稿键：映射表由契约字段表推出，两边都不能凭想象加键
const LOCAL_BY_WIRE = Object.fromEntries(
  OPTIMAL_TEAM_FIELDS.map((f) => [
    f.key,
    f.key.replace(/_([a-z])/g, (_, c) => c.toUpperCase())
  ])
)
const localOf = (key) => LOCAL_BY_WIRE[key] || key
const draftOf = (key) => draft.value[localOf(key)]
const roleLabel = (role) => optimalTeamRoleLabel(role)

/** 候选项也按字段分流：技能取注册表里的真实技能串，能力域取图谱里的域节点 */
function optionsOf(key) {
  return key === 'required_skills' ? props.skillOptions : props.store.domainOptions
}

function tagPlaceholder(key) {
  return key === 'required_skills'
    ? `从 ${props.skillOptions.length} 个已注册技能里挑，或直接输入`
    : '选能力域（列表取自当前图谱）'
}

function hintOf(key) {
  if (key === 'required_skills' || key === 'required_domains') {
    return '空数组不进请求体（serde default 等价），后端把两类需求合并成一个需求集做覆盖'
  }
  if (key === 'max_members') return OPTIMAL_TEAM_BACKEND_RULES.maxMembers
  if (key === 'min_rating') return OPTIMAL_TEAM_BACKEND_RULES.minRating
  return OPTIMAL_TEAM_BACKEND_RULES.goal
}

const emptyReason = computed(() => `候选门槛：${OPTIMAL_TEAM_BACKEND_RULES.availability}`)

function submit() {
  store.formTeam()
}
</script>

<style scoped>
.agt { display: flex; flex-direction: column; gap: 10px; }
.agt-title { margin: 0; font-size: 15px; color: var(--text-primary); }
.agt-dim { font-size: 12px; color: var(--text-muted); }
.agt-form { display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 10px; }
.agt-field { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
.agt-label { font-size: 12px; color: var(--text-secondary); display: flex; align-items: baseline; gap: 6px; }
.agt-backend {
  font-size: 10px;
  padding: 0 4px;
  border-radius: var(--radius-sm);
  background: var(--bg-hover);
  color: var(--warning);
}
.agt-wide { width: 100%; }
.agt-hint { margin: 0; font-size: 11px; line-height: 1.5; color: var(--text-muted); }
.agt-actions { display: flex; align-items: center; gap: 8px; }
.agt-warn { font-size: 12px; color: var(--danger); }
.agt-alert { margin: 0; }
.agt-summary { margin: 0; font-size: 13px; color: var(--text-primary); }
.agt-members, .agt-covers { list-style: none; margin: 0; padding: 0; }
.agt-members { display: flex; flex-direction: column; gap: 6px; }
.agt-member { padding: 8px; border: 1px solid var(--border-light); border-radius: var(--radius-sm); background: var(--bg-card); }
.agt-member-head { display: flex; flex-wrap: wrap; align-items: baseline; gap: 8px; font-size: 12px; }
.agt-jump { background: none; border: 0; padding: 0; font-size: 13px; color: var(--accent-light); cursor: pointer; }
.agt-jump:hover { text-decoration: underline; }
.agt-covers { display: flex; flex-wrap: wrap; gap: 4px; }
.agt-chip {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: var(--radius-sm);
  background: var(--bg-hover);
  color: var(--text-secondary);
}
.agt-chip.is-domain { color: var(--warning); }
</style>
