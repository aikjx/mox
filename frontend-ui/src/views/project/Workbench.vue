<template>
  <div class="layout">
    <SessionSidebar
      :sessions="sessions"
      :active-id="activeSession"
      :online="online"
      @new="newSession"
      @select="selectSession"
    />

    <div class="main">
      <div class="col chat-col">
        <ChatView
          ref="chatRef"
          :session-id="activeSession"
          @action="onAction"
          @thinking="onThinking"
        />
      </div>
      <div class="divider"></div>
      <div class="col graph-col">
        <FlowGraph ref="graphRef" />
      </div>
    </div>

    <div class="topbar">
      <div class="tb-title">企业级算子编排 · AI 对话与业务流程可视化</div>
      <div class="tb-status">
        <el-tag :type="online ? 'success' : 'warning'" size="small" effect="dark">
          {{ online ? '运行中' : '连接中' }}
        </el-tag>
        <span class="tb-clock">{{ clock }}</span>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { ElMessage } from 'element-plus'
import SessionSidebar from '@/components/SessionSidebar.vue'
import ChatView from '@/views/ai/ChatView.vue'
import FlowGraph from '@/views/graph/FlowGraph.vue'
import { getStatus, executeFlow, getGraph, getFlows, createExpertSession, getExpertSessions } from '@/api'

const chatRef = ref(null)
const graphRef = ref(null)

const sessions = ref([])
const activeSession = ref('')
const online = ref(false)
const clock = ref('')
let seq = 0
let timer = null
let statusTimer = null

function genId() {
  return 'sess-' + Date.now().toString(36) + '-' + (++seq).toString(36)
}

async function loadSessions() {
  try {
    const data = await getExpertSessions({ limit: 20 })
    const list = Array.isArray(data) ? data : (data?.list || data?.items || data?.sessions || [])
    if (list.length) {
      sessions.value = list.map((s) => ({
        id: s.id,
        title: s.title || '未命名会话',
        time: s.created_at ? new Date(s.created_at).toLocaleDateString('zh-CN') : '',
      }))
      activeSession.value = sessions.value[0].id
      return
    }
  } catch (e) {
    // 后端未就绪时不报错，由 newSession 创建本地会话
    console.warn('[Workbench] 加载会话列表失败:', e?.message)
  }
  // 无历史会话时创建一个新的本地会话
  await newSession()
}

async function newSession() {
  const title = '新会话 ' + new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  let id = genId()
  try {
    const created = await createExpertSession({ title, type: 'workbench' })
    if (created && created.id) {
      id = created.id
    }
  } catch (e) {
    // 后端未就绪时使用本地 ID，不阻塞用户操作
    console.warn('[Workbench] 创建会话失败，使用本地 ID:', e?.message)
  }
  sessions.value.unshift({
    id,
    title,
    time: new Date().toLocaleDateString('zh-CN'),
  })
  activeSession.value = id
}

function selectSession(id) {
  activeSession.value = id
}

async function onAction(a) {
  const type = (a.action_type || '').toLowerCase()
  try {
    if (type === 'execute_workflow' || type === 'show_graph') {
      const ops = a.payload?.operators
      if (ops && ops.length) {
        const flowId = await ensureFlow(ops)
        if (flowId) {
          const resp = await executeFlow({ flow_id: flowId, input: {} })
          if (resp.result && resp.result.node_results) {
            graphRef.value?.showTrace(resp.result.node_results)
            ElMessage.success('流程执行完成，已高亮处理轨迹')
          }
        }
      }
      if (type === 'show_graph') {
        await graphRef.value?.reload()
      }
    } else if (type === 'view_operator') {
      ElMessage.info('查看算子：' + (a.payload?.operator || a.label))
    } else {
      ElMessage.info(a.label)
    }
  } catch (e) {
    ElMessage.error(e.message || '操作失败')
  }
}

// 后端待提供: 根据算子列表自动匹配/创建工作流（当前仅取第一个可用 flow）
async function ensureFlow(operators) {
  try {
    const list = await getFlows()
    const flows = list.flows || []
    if (flows.length) return flows[0].id
  } catch (e) {}
  return null
}

// 后端待提供: AI 思考状态流式推送回调（当前为空实现，SSE 接入后启用）
function onThinking(v) {}

async function checkStatus() {
  try {
    await getStatus()
    online.value = true
  } catch (e) {
    online.value = false
  }
}

function tick() {
  clock.value = new Date().toLocaleTimeString('zh-CN')
}

onMounted(async () => {
  loadSessions()
  tick()
  timer = setInterval(tick, 1000)
  await checkStatus()
  statusTimer = setInterval(checkStatus, 15000)
})

onBeforeUnmount(() => {
  timer && clearInterval(timer)
  statusTimer && clearInterval(statusTimer)
})
</script>

<style scoped>
.layout { display: flex; height: 100vh; overflow: hidden; }
.main { flex: 1; display: flex; min-width: 0; position: relative; }
.col { display: flex; flex-direction: column; min-width: 0; }
.chat-col { flex: 1 1 46%; min-width: 380px; }
.graph-col { flex: 1 1 54%; min-width: 360px; }
.divider { width: 1px; background: var(--border); flex-shrink: 0; }
.topbar { display: none; }
</style>
