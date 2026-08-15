<template>
  <div class="mcp">
    <div class="head">
      <div>
        <h2 class="page-title">MCP 兼容中心</h2>
        <p class="page-subtitle">把系统内的算子与插件以标准 Model Context Protocol 暴露，兼容任意开源 MCP 客户端</p>
      </div>
      <el-tag type="success" effect="dark" round><el-icon><CircleCheck /></el-icon>&nbsp;端点已就绪</el-tag>
    </div>

    <el-row :gutter="16">
      <!-- 左：端点 / 连接配置 -->
      <el-col :span="10">
        <div class="panel card-pad">
          <h3 class="section-title">MCP 端点</h3>
          <div class="kv">
            <div class="k">传输协议</div>
            <div class="v">JSON-RPC 2.0 · Streamable HTTP</div>
          </div>
          <div class="kv">
            <div class="k">接入地址</div>
            <div class="v code">{{ endpoint }}</div>
          </div>
          <div class="kv">
            <div class="k">支持方法</div>
            <div class="v">initialize / tools/list / tools/call / ping</div>
          </div>
          <el-alert type="info" :closable="false" show-icon class="mcp-alert">
            <template #title>兼容开源生态</template>
            可直接对接 Claude Desktop、Cursor、Cline、Continue 等任意支持 MCP 的客户端，无需任何改造即可调用本系统算子与插件。
          </el-alert>

          <h3 class="section-title" style="margin-top:18px">客户端配置示例</h3>
          <el-tabs v-model="cfgTab">
            <el-tab-pane label="Claude Desktop" name="claude">
              <pre class="code-block">{{ claudeConfig }}</pre>
            </el-tab-pane>
            <el-tab-pane label="Cursor / Cline" name="cursor">
              <pre class="code-block">{{ cursorConfig }}</pre>
            </el-tab-pane>
          </el-tabs>
          <el-button size="small" @click="copyConfig">{{ copied ? '已复制' : '复制配置' }}</el-button>
        </div>
      </el-col>

      <!-- 右：Tools 列表 + 在线调用 -->
      <el-col :span="14">
        <div class="panel card-pad">
          <div class="tool-head">
            <h3 class="section-title">已暴露的 MCP Tools（{{ tools.length }}）</h3>
            <el-button size="small" :loading="loading" @click="loadTools">
              <el-icon><Refresh /></el-icon> 刷新
            </el-button>
          </div>
          <el-input v-model="filter" placeholder="搜索 tool 名称 / 描述" clearable size="small" class="tool-filter" />
          <div class="tool-list">
            <div
              v-for="t in filteredTools"
              :key="t.name"
              class="tool-item"
              :class="{ active: selected?.name === t.name }"
              @click="selected = t"
            >
              <div class="tool-name">
                <el-icon v-if="t.annotations?.source === 'ous-plugin'"><Connection /></el-icon>
                <el-icon v-else><Operation /></el-icon>
                {{ t.name }}
              </div>
              <div class="tool-desc">{{ t.description }}</div>
            </div>
            <el-empty v-if="!filteredTools.length" description="暂无工具，请刷新" :image-size="60" />
          </div>

          <div class="tool-tester" v-if="selected">
            <h4 class="section-title">在线调用测试 · {{ selected.name }}</h4>
            <el-input
              v-model="callArgs"
              type="textarea"
              :rows="4"
              placeholder='参数 JSON，例如：{"input":[1,2,3]}'
            />
            <el-button
              type="primary"
              size="small"
              :loading="calling"
              @click="callTool"
              style="margin-top:8px"
            >
              <el-icon><Promotion /></el-icon> 调用
            </el-button>
            <pre class="code-block result" v-if="callResult">{{ callResult }}</pre>
          </div>
        </div>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { mcpListTools, mcpCall } from '@/api'

const endpoint = window.location.origin + '/api/mcp'
const cfgTab = ref('claude')
const copied = ref(false)
const loading = ref(false)
const calling = ref(false)
const filter = ref('')
const tools = ref([])
const selected = ref(null)
const callArgs = ref('{"input":[1,2,3]}')
const callResult = ref('')

const filteredTools = computed(() => {
  const f = filter.value.trim().toLowerCase()
  if (!f) return tools.value
  return tools.value.filter(
    (t) => t.name.toLowerCase().includes(f) || t.description.toLowerCase().includes(f)
  )
})

const claudeConfig = computed(() =>
  JSON.stringify(
    {
      mcpServers: {
        'operator-unified': {
          url: endpoint.value,
          transport: 'streamable-http',
        },
      },
    },
    null,
    2
  )
)
const cursorConfig = computed(() =>
  JSON.stringify(
    {
      mcp: {
        servers: {
          'operator-unified': { url: endpoint.value },
        },
      },
    },
    null,
    2
  )
)

async function loadTools() {
  loading.value = true
  try {
    const res = await mcpListTools()
    const r = res.data || res
    tools.value = (r.result && r.result.tools) || r.tools || []
    if (!selected.value && tools.value.length) selected.value = tools.value[0]
  } catch (e) {
    ElMessage.error('获取 MCP tools 失败：' + (e.message || e))
  } finally {
    loading.value = false
  }
}

async function callTool() {
  if (!selected.value) return
  let args
  try {
    args = JSON.parse(callArgs.value || '{}')
  } catch {
    ElMessage.error('参数不是合法 JSON')
    return
  }
  calling.value = true
  callResult.value = ''
  try {
    const res = await mcpCall(selected.value.name, args)
    const r = res.data || res
    callResult.value = JSON.stringify(r, null, 2)
  } catch (e) {
    callResult.value = '调用失败：' + (e.message || e)
  } finally {
    calling.value = false
  }
}

function copyConfig() {
  const txt = cfgTab.value === 'claude' ? claudeConfig.value : cursorConfig.value
  navigator.clipboard?.writeText(txt).then(
    () => {
      copied.value = true
      setTimeout(() => (copied.value = false), 1500)
    },
    () => ElMessage.warning('复制失败，请手动复制')
  )
}

onMounted(loadTools)
</script>

<style scoped>
.mcp { padding: 4px; }
.head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.section-title { font-size: 14px; font-weight: 700; color: var(--text, #1e293b); margin: 4px 0 12px; }
.kv { display: flex; gap: 10px; padding: 7px 0; border-bottom: 1px dashed #eef1f6; font-size: 13px; }
.k { width: 80px; color: #94a3b8; flex-shrink: 0; }
.v { color: #334155; }
.code { font-family: ui-monospace, monospace; background: #f1f5f9; padding: 2px 6px; border-radius: 5px; font-size: 12px; }
.mcp-alert { margin-top: 12px; }
.code-block { background: #0f172a; color: #e2e8f0; padding: 12px; border-radius: 8px; font-size: 12px; overflow: auto; margin: 8px 0 0; }
.tool-head { display: flex; justify-content: space-between; align-items: center; }
.tool-filter { margin-bottom: 10px; }
.tool-list { max-height: 360px; overflow: auto; }
.tool-item { padding: 10px 12px; border: 1px solid #eef1f6; border-radius: 8px; margin-bottom: 8px; cursor: pointer; transition: .15s; }
.tool-item:hover { border-color: #c7d2fe; background: #fafbff; }
.tool-item.active { border-color: #6366f1; background: #eef2ff; }
.tool-name { font-size: 13px; font-weight: 600; color: #334155; display: flex; align-items: center; gap: 6px; }
.tool-desc { font-size: 12px; color: #94a3b8; margin-top: 3px; }
.tool-tester { margin-top: 14px; border-top: 1px dashed #eef1f6; padding-top: 12px; }
.code-block.result { background: #0f172a; max-height: 260px; }
</style>
