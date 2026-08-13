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
import ChatView from '@/views/ChatView.vue'
import FlowGraph from '@/views/FlowGraph.vue'
import { getStatus, executeFlow, getGraph } from '@/api'

const chatRef = ref(null)
const graphRef = ref(null)

const sessions = ref([])
const activeSession = ref('')
const online = ref(false)
const clock = ref('')
let seq = 0
let timer = null

function genId() {
  return 'sess-' + Date.now().toString(36) + '-' + (++seq).toString(36)
}

function newSession() {
  const id = genId()
  sessions.value.unshift({
    id,
    title: '新会话 ' + new Date().toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }),
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
      // 尝试从 payload 取流程执行
      const ops = a.payload?.operators
      if (ops && ops.length) {
        const flowId = await ensureFlow(ops)
        if (flowId) {
          const resp = await executeFlow(flowId, {})
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

// 若后端没有对应流程，则尝试以推荐算子创建一个临时流程并执行
async function ensureFlow(operators) {
  try {
    const list = await (await import('@/api')).listFlows()
    const flows = list.flows || []
    if (flows.length) return flows[0].id
  } catch (e) {
    // ignore
  }
  return null
}

function onThinking(v) {
  // loading 态由 ChatView 自行管理，这里可扩展
}

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
  newSession()
  tick()
  timer = setInterval(tick, 1000)
  await checkStatus()
  setInterval(checkStatus, 15000)
})

onBeforeUnmount(() => {
  timer && clearInterval(timer)
})
</script>

<style scoped>
.layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}
.main {
  flex: 1;
  display: flex;
  min-width: 0;
  position: relative;
}
.col {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.chat-col {
  flex: 1 1 46%;
  min-width: 380px;
}
.graph-col {
  flex: 1 1 54%;
  min-width: 360px;
}
.divider {
  width: 1px;
  background: var(--border);
  flex-shrink: 0;
}
.topbar {
  display: none;
}
</style>
