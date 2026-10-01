// 智能协作状态：持有「模式 + 输入 + 最近一次结果 + 本次运行历史」，视图只读绑定、只发意图。
// 校验与 body 组装全在 contract/collab.js，本文件不重复后端规则。
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import { allianceApi } from '@/modules/expert-alliance/api'
import {
  COLLAB_MODE, collabConstraintValues, collabControlValue, collabMode, collabProblem, debateCapacityText
} from '@/modules/expert-alliance/contract'
import { collabOutcome, collabQuestionText, collabRefId } from '@/modules/expert-alliance/model'

// 面板展示的最近运行条数（后端 sessions 才是持久记录）
export const HISTORY_CAP = 20

export const useAllianceCollabStore = defineStore('allianceCollab', () => {
  const api = allianceApi

  // smart 模式无需选择专家、后端自行意图匹配，作为默认入口
  const mode = ref(COLLAB_MODE.SMART)
  const input = reactive({ question: '', topic: '', algorithm_description: '', expertIds: [], domain: '', context: '' })
  const controls = reactive({})
  const result = ref(null)
  const history = ref([])
  const loading = reactive({ run: false })
  const error = reactive({ run: '' })

  const current = computed(() => collabMode(mode.value))
  /** 后端认识的文本字段名随模式变化，视图按它取放 */
  const textField = computed(() => current.value.field)
  const text = computed({
    get: () => input[textField.value] || '',
    set: (v) => { input[textField.value] = v }
  })
  const needsExperts = computed(() => current.value.expertChoice !== 'none')
  const controlList = computed(() => collabControlValue(current.value, controls).map((c) => ({
    ...c,
    ...(current.value.controls.find((d) => d.wire === c.wire) || {})
  })))
  const constraintList = computed(() => collabConstraintValues(current.value, controls))
  const pickedCount = computed(() => input.expertIds.length)
  const capacityNote = computed(() =>
    mode.value === COLLAB_MODE.DEBATE ? debateCapacityText(pickedCount.value) : ''
  )
  const validation = computed(() => collabProblem(current.value, { ...input, ...controls }))
  const runnable = computed(() => !validation.value && !loading.run)
  const blocked = computed(() => collabOutcome(result.value) === 'blocked')
  // 结果自带 mode 戳，按戳取渲染口径：后端换字段名时这里先空，而不是错渲染
  const resultKind = computed(() => (result.value ? collabMode(result.value.mode)?.resultKind || '' : ''))

  function syncControls() {
    for (const key of Object.keys(controls)) delete controls[key]
    for (const c of collabControlValue(current.value, {})) controls[c.wire] = c.value
    // 约束子键同样按模式重置为后端的「不发即默认」值
    for (const f of collabConstraintValues(current.value, {})) controls[f.key] = f.value
  }

  function setMode(key) {
    const def = collabMode(key)
    if (!def) return false
    mode.value = def.key
    result.value = null
    error.run = ''
    syncControls()
    return true
  }

  /** 选择专家：模式决定语义——单选替换、多选切换、不选专家的模式一律忽略 */
  function toggleExpert(id) {
    if (!id || !needsExperts.value) return false
    const choice = current.value.expertChoice
    if (choice === 'one') {
      input.expertIds = [id]
      return true
    }
    const idx = input.expertIds.indexOf(id)
    if (idx >= 0) input.expertIds.splice(idx, 1)
    else input.expertIds.push(id)
    return true
  }

  function clearExperts() {
    input.expertIds = []
  }

  function pushHistory(entry) {
    history.value.unshift(entry)
    if (history.value.length > HISTORY_CAP) history.value.length = HISTORY_CAP
  }

  async function run() {
    if (validation.value) return null
    loading.run = true
    error.run = ''
    result.value = null
    try {
      const payload = await api.collaborate(mode.value, { ...input, ...controls })
      result.value = payload
      pushHistory({
        mode: mode.value,
        modeLabel: current.value.label,
        refId: collabRefId(payload),
        text: collabQuestionText(payload),
        at: new Date().toISOString(),
        ok: true
      })
      return payload
    } catch (e) {
      error.run = e?.msg || e?.message || '协作请求失败'
      pushHistory({
        mode: mode.value,
        modeLabel: current.value.label,
        refId: '',
        text: String(input[textField.value] || '').trim(),
        at: new Date().toISOString(),
        ok: false
      })
      return null
    } finally {
      loading.run = false
    }
  }

  function reset() {
    input.question = ''
    input.topic = ''
    input.algorithm_description = ''
    input.expertIds = []
    input.domain = ''
    input.context = ''
    result.value = null
    error.run = ''
    syncControls()
  }

  syncControls()

  return {
    mode, input, controls, result, history, loading, error,
    current, textField, text, needsExperts, controlList, constraintList, pickedCount, capacityNote,
    validation, runnable, blocked, resultKind,
    setMode, toggleExpert, clearExperts, run, reset
  }
})
