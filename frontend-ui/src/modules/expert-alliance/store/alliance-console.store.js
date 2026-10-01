// 联盟控制台状态：唯一持有联盟数据的前序状态，视图只读绑定、只发意图。
import { defineStore } from 'pinia'
import { computed, reactive, ref } from 'vue'
import { allianceApi } from '@/modules/expert-alliance/api'
import { isActiveTaskStatus, isTerminalTaskStatus } from '@/modules/expert-alliance/contract'
import { dispatchInvalid, dispatchPatch, dispatchRows, dispatchRunFindings, dispatchRunProblem, breakerEmptyNote, successRateNote } from '@/modules/expert-alliance/contract'
import { expertNameOr } from '@/modules/expert-alliance/contract'

export const useAllianceConsoleStore = defineStore('allianceConsole', () => {
  const api = allianceApi

  const runtime = ref(null)
  const tasks = ref([])
  const total = ref(0)
  const selectedId = ref('')
  const detail = reactive({ task: null, nodes: [], dag: null, fusion: null, execution: null, plan: null })
  const logs = ref([])
  const loading = reactive({ runtime: false, tasks: false, detail: false, create: false, config: false, toggle: false, dispatch: false, candidates: false, status: false, reset: false })
  const error = reactive({ runtime: '', tasks: '', detail: '', create: '', config: '', toggle: '', dispatch: '', candidates: '', status: '', reset: '' })

  const selectedTask = computed(() => tasks.value.find((t) => t.id === selectedId.value) ?? detail.task)
  const activeTasks = computed(() => tasks.value.filter((t) => isActiveTaskStatus(t.status)))
  const hasRunningTask = computed(() => activeTasks.value.some((t) => t.id === selectedId.value))

  function pick(source, target) {
    return source || target
  }

  async function loadRuntime() {
    loading.runtime = true
    error.runtime = ''
    try {
      runtime.value = await api.getRuntime()
    } catch (e) {
      error.runtime = e?.msg || e?.message || '运行时状态获取失败'
      runtime.value = null
    } finally {
      loading.runtime = false
    }
  }

  async function loadTasks() {
    loading.tasks = true
    error.tasks = ''
    try {
      const res = await api.listTasks()
      tasks.value = res.items
      total.value = res.total
      if (!selectedId.value && res.items.length) await selectTask(res.items[0].id)
    } catch (e) {
      error.tasks = e?.msg || e?.message || '任务列表获取失败'
      tasks.value = []
      total.value = 0
    } finally {
      loading.tasks = false
    }
  }

  function resetDetail() {
    detail.task = null
    detail.nodes = []
    detail.dag = null
    detail.fusion = null
    detail.execution = null
    detail.plan = null
    logs.value = []
  }

  async function selectTask(id) {
    if (!id) return
    selectedId.value = id
    loading.detail = true
    error.detail = ''
    resetDetail()
    try {
      const settled = await Promise.allSettled([
        api.getTask(id), api.getNodes(id), api.getDag(id), api.getExecutionStatus(id)
      ])
      const [task, nodes, dag, execution] = settled
      if (task.status === 'rejected') throw task.reason
      detail.task = task.value
      detail.nodes = pick(nodes.status === 'fulfilled' ? nodes.value.items : null, [])
      detail.dag = dag.status === 'fulfilled' ? dag.value : null
      detail.execution = execution.status === 'fulfilled' ? execution.value : null

      const optional = await Promise.allSettled([api.getFusion(id), api.getPlan(id), api.getLogs(id)])
      const [fusion, plan, logList] = optional
      if (fusion.status === 'fulfilled') detail.fusion = fusion.value
      if (plan.status === 'fulfilled') detail.plan = plan.value
      if (logList.status === 'fulfilled') logs.value = logList.value.items
    } catch (e) {
      error.detail = e?.msg || e?.message || '任务详情获取失败'
    } finally {
      loading.detail = false
    }
  }

  async function createTask(input) {
    loading.create = true
    error.create = ''
    try {
      const created = await api.createTask(input)
      await loadTasks()
      await selectTask(created.id)
      return created
    } catch (e) {
      error.create = e?.msg || e?.message || '任务创建失败'
      return null
    } finally {
      loading.create = false
    }
  }

  async function controlTask(verb, id = selectedId.value) {
    if (!id) return null
    error.detail = ''
    try {
      const res = await api.controlTask(id, verb)
      const fresh = await api.getTask(id).catch(() => null)
      if (fresh) {
        detail.task = fresh
        const idx = tasks.value.findIndex((t) => t.id === id)
        if (idx >= 0) tasks.value[idx] = fresh
      }
      return res
    } catch (e) {
      error.detail = e?.msg || e?.message || '任务操作失败'
      return null
    }
  }

  /** SSE / 轮询共用的日志入栈，按 seq 去重 */
  function pushLog(entry) {
    if (!entry || !entry.message) return
    const seq = entry.seq
    if (typeof seq === 'number' && logs.value.some((l) => l.seq === seq)) return
    logs.value.push(entry)
    if (logs.value.length > 500) logs.value.splice(0, logs.value.length - 500)
  }

  function replaceLogs(items) {
    logs.value = items.slice(-500)
  }

  function isTerminal(id) {
    const t = tasks.value.find((x) => x.id === id) ?? detail.task
    return t ? isTerminalTaskStatus(t.status) : false
  }

  // ── 调度器配置：后端 PUT 是合并式（缺省键保持原值），因此草稿只发差异项 ──
  const dispatcherConfig = ref(null)
  const configDraft = reactive({})

  const configRows = computed(() => dispatchRows(dispatcherConfig.value || {}))
  const configPatch = computed(() => dispatchPatch(dispatcherConfig.value || {}, configDraft))
  const configInvalid = computed(() => dispatchInvalid(configDraft))
  const configChanged = computed(() => Object.keys(configPatch.value.patch).length > 0)

  function primeConfigDraft(config) {
    for (const row of dispatchRows(config)) configDraft[row.key] = row.value
  }

  function setConfigValue(key, value) {
    configDraft[key] = value
  }

  async function loadDispatcherConfig() {
    loading.config = true
    error.config = ''
    try {
      dispatcherConfig.value = await api.getDispatcherConfig()
      primeConfigDraft(dispatcherConfig.value)
    } catch (e) {
      error.config = e?.msg || e?.message || '调度配置获取失败'
      dispatcherConfig.value = null
    } finally {
      loading.config = false
    }
  }

  /** 保存：校验不过就不发（后端四条 400 分支已在契约里前移）；成功以响应回读整份合并结果 */
  async function saveDispatcherConfig() {
    const { patch, problem } = configPatch.value
    if (problem) {
      error.config = problem
      return null
    }
    if (!Object.keys(patch).length) return dispatcherConfig.value
    loading.config = true
    error.config = ''
    try {
      dispatcherConfig.value = await api.updateDispatcherConfig(patch)
      primeConfigDraft(dispatcherConfig.value)
      return dispatcherConfig.value
    } catch (e) {
      error.config = e?.msg || e?.message || '调度配置保存失败'
      return null
    } finally {
      loading.config = false
    }
  }

  // ── 分发实跑：证明"改过的策略真的被走到"，而不是只把配置读写一遍 ──
  const dispatchResult = ref(null)
  const dispatchCandidates = ref([])
  const dispatchFindings = computed(() => dispatchRunFindings(dispatchResult.value, dispatcherConfig.value))

  /** 指定专家下拉：只有在线/忙碌（即 is_expert_available 可能放行）的人才有意义，但后端还会再查熔断与并发 */
  async function loadDispatchCandidates() {
    loading.candidates = true
    error.candidates = ''
    try {
      const res = await api.listExperts({ status: 'online', page: 1, pageSize: 100 })
      dispatchCandidates.value = res.items.map((e) => ({ id: e.id, name: e.name, status: e.status }))
    } catch (e) {
      // 专家目录取不到只让"指定专家"这一项空着，实跑本身可以不带 expert_ids
      error.candidates = e?.msg || e?.message || '可指定专家获取失败'
      dispatchCandidates.value = []
    } finally {
      loading.candidates = false
    }
  }

  async function runDispatch(form = {}) {
    const problem = dispatchRunProblem(form)
    if (problem) {
      error.dispatch = problem
      return null
    }
    loading.dispatch = true
    error.dispatch = ''
    try {
      dispatchResult.value = await api.runDispatch(form)
      return dispatchResult.value
    } catch (e) {
      // 后端"无可用专家"回 503，真实原因只可能是全员离线 / 熔断到阈 / 并发满（is_expert_available）
      error.dispatch = e?.msg || e?.message || '分发实跑失败'
      // 上一次的实跑结果留着：它自带 dispatch_id 与 created_at，能自证是哪一次的证据，
      // 一次失败不该把已有结论擦成空白
      return null
    } finally {
      loading.dispatch = false
    }
  }

  // ── 调度状态与负载重置 ──────────────────────────────────────────────
  // 这一面对外存在已久（experts.dispatch.status 一直在挂载集里），但只有 api 一层：
  // store/views 零消费 ⇒ 重置动作没有读数侧，成功与否只能信后端自报的一个布尔。

  const dispatcherStatus = ref(null)
  /** 最近一次重置的回执：单专家带 previousLoad，全量带 resetCount；两者都不含"该专家存在"的证据 */
  const resetReceipt = ref(null)

  const breakerNote = computed(() => breakerEmptyNote(dispatcherStatus.value?.circuitBreakers))
  const successNote = computed(() => successRateNote(dispatcherStatus.value))
  /** 状态端点只给 expert_id，姓名按专家目录的缓存补齐，取不到就照实显示 id */
  const loadRows = computed(() => (dispatcherStatus.value?.expertLoads || []).map((l) => ({
    ...l,
    name: expertNameOr({ id: l.expertId, name: dispatchCandidates.value.find((c) => c.id === l.expertId)?.name }, '')
  })))

  async function loadDispatcherStatus() {
    loading.status = true
    error.status = ''
    try {
      dispatcherStatus.value = await api.dispatcherStatus()
    } catch (e) {
      error.status = e?.msg || e?.message || '调度状态获取失败'
      // 取不到就把表清空，而不是留着上一次进程读数冒充新鲜数据
      dispatcherStatus.value = null
    } finally {
      loading.status = false
    }
  }

  /**
   * 单个专家的负载重置。回执不能当存在性证据（未知 id 也回 reset:true），
   * 而且后端不落库，所以界面唯一能信的是重取回来的状态表。
   * 不在本地把 currentLoad 抹成 0：那会让"我点了"看起来像"后端改了"。
   */
  async function resetExpertLoad(id, reason) {
    if (!id) return null
    loading.reset = true
    error.reset = ''
    try {
      resetReceipt.value = { scope: 'one', ...(await api.resetDispatcherLoad(id, reason)) }
      await loadDispatcherStatus()
      return resetReceipt.value
    } catch (e) {
      error.reset = e?.msg || e?.message || '负载重置失败'
      // 失败时保留上一次的读数与回执：它们各自带着自己的时间戳，能自证是哪一次的结果
      return null
    } finally {
      loading.reset = false
    }
  }

  /** 全量重置：整表 current_load 归零 + 熔断 map.clear()，同样不落库，同样要重取才看得见 */
  async function resetAllLoads() {
    loading.reset = true
    error.reset = ''
    try {
      resetReceipt.value = { scope: 'all', ...(await api.resetAllDispatcherLoads()) }
      await loadDispatcherStatus()
      return resetReceipt.value
    } catch (e) {
      error.reset = e?.msg || e?.message || '全量负载重置失败'
      return null
    } finally {
      loading.reset = false
    }
  }

  /**
   * 标记完成 / 重新打开。本地分支会把非 failed、非 cancelled 的节点整批置完成，
   * 所以节点表必须重取；远程分支不回传状态，只能等重取的详情说话。
   * 期间用户切了任务就不写 detail——否则上一个任务的状态会出现在下一个任务名下。
   */
  async function toggleTaskDone(id = selectedId.value) {
    if (!id) return null
    loading.toggle = true
    error.toggle = ''
    try {
      const res = await api.toggleTaskDone(id)
      if (selectedId.value !== id) return res
      const [fresh, nodes] = await Promise.all([
        api.getTask(id).catch(() => null),
        api.getNodes(id).catch(() => null)
      ])
      if (fresh) {
        detail.task = fresh
        const idx = tasks.value.findIndex((t) => t.id === id)
        if (idx >= 0) tasks.value[idx] = fresh
      }
      if (nodes) detail.nodes = nodes.items
      return res
    } catch (e) {
      error.toggle = e?.msg || e?.message || '任务完成状态切换失败'
      return null
    } finally {
      loading.toggle = false
    }
  }

  return {
    runtime, tasks, total, selectedId, selectedTask, detail, logs,
    loading, error, activeTasks, hasRunningTask,
    loadRuntime, loadTasks, selectTask, createTask, controlTask, pushLog, replaceLogs, isTerminal,
    dispatcherConfig, configDraft, configRows, configPatch, configInvalid, configChanged,
    setConfigValue, loadDispatcherConfig, saveDispatcherConfig,
    dispatchResult, dispatchCandidates, dispatchFindings, runDispatch, loadDispatchCandidates,
    dispatcherStatus, resetReceipt, loadRows, breakerNote, successNote,
    loadDispatcherStatus, resetExpertLoad, resetAllLoads,
    toggleTaskDone
  }
})
