<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">工作流编排</h2>
        <p class="page-subtitle">基于 BPMN 引擎驱动 AI 执行 · 流程图设计 / 模板 / 实例</p>
      </div>
      <div class="page-header-actions">
        <el-button type="primary" @click="goAIGenerate">
          <el-icon><Promotion /></el-icon> AI生成工作流
        </el-button>
        <el-button @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
        <el-button type="primary" plain @click="showCreate = true">
          <el-icon><Plus /></el-icon> 新建流程图
        </el-button>
      </div>
    </div>

    <!-- 外层 Tab：编排 / 插件 / MCP / 自动化 -->
    <el-tabs v-model="outerTab" class="wf-outer-tabs" @tab-change="onOuterTabChange">
      <el-tab-pane label="流程编排" name="flows" />
      <el-tab-pane label="插件中心" name="plugins" />
      <el-tab-pane label="MCP 兼容" name="mcp" />
      <el-tab-pane label="自动化" name="automation" />
    </el-tabs>

    <div class="page-content" v-show="outerTab === 'flows'">

    <el-tabs v-model="tab">
      <el-tab-pane label="流程图" name="flows">
        <div class="grid grid-3">
          <div class="panel flow-card" v-for="f in flows" :key="f.id">
            <div class="flow-top">
              <div class="flow-name">{{ f.name || f.id }}</div>
              <el-dropdown trigger="click">
                <el-icon class="more"><MoreFilled /></el-icon>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item @click="runFlow(f)">
                      <el-icon><VideoPlay /></el-icon> 执行
                    </el-dropdown-item>
                    <el-dropdown-item @click="doValidateFlow(f)">
                      <el-icon><CircleCheck /></el-icon> 校验
                    </el-dropdown-item>
                    <el-dropdown-item @click="viewFlowDetail(f)">
                      <el-icon><View /></el-icon> 详情
                    </el-dropdown-item>
                    <el-dropdown-item divided @click="quickOpen(f, 'video')">
                      <el-icon><VideoCamera /></el-icon> 查看视频
                    </el-dropdown-item>
                    <el-dropdown-item @click="quickOpen(f, 'log')">
                      <el-icon><Document /></el-icon> 查看日志
                    </el-dropdown-item>
                    <el-dropdown-item divided @click="delFlow(f)">
                      <el-icon><Delete /></el-icon> 删除
                    </el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
            <div class="flow-meta">
              <span class="badge info">{{ f.nodes?.length || 0 }} 节点</span>
              <span class="badge primary">{{ f.edges?.length || 0 }} 连线</span>
            </div>
            <div class="flow-desc">{{ f.description || '暂无描述' }}</div>
          </div>
          <el-empty v-if="!flows.length" description="暂无流程图" :image-size="70" />
        </div>
      </el-tab-pane>

      <el-tab-pane label="业务工作流" name="biz">
        <div class="grid grid-2">
          <div class="panel card-pad">
            <h3 class="section-title">可用模板</h3>
            <div class="tpl-list">
              <div class="tpl" v-for="t in templates" :key="t.id">
                <div>
                  <div class="tpl-name">{{ t.name }}</div>
                  <div class="tpl-desc">{{ t.description }}</div>
                </div>
                <el-button size="small" type="primary" plain @click="saveFromTpl(t)">选用</el-button>
              </div>
            </div>
          </div>
          <div class="panel card-pad">
            <h3 class="section-title">工作流实例</h3>
            <el-empty v-if="!instances.length" description="暂无实例" :image-size="60" />
            <div v-else class="inst-list">
              <div class="inst" v-for="(it, i) in instances" :key="i">
                <span class="badge" :class="(it.status || 'running') === 'completed' ? 'success' : 'warning'">
                  {{ it.status || 'running' }}
                </span>
                <span class="inst-id">{{ it.id || it.workflow_id || '#' + i }}</span>
              </div>
            </div>
          </div>
          <div class="panel card-pad">
            <h3 class="section-title">AI 已保存工作流</h3>
            <div class="tpl-list">
              <div class="tpl" v-for="w in savedWorkflows" :key="w.id">
                <div>
                  <div class="tpl-name">{{ w.name }}</div>
                  <div class="tpl-desc">{{ w.description || (w.nodes_count + ' 个节点') }}</div>
                </div>
                <el-button size="small" type="primary" plain @click="loadSavedWorkflow(w)">载入执行</el-button>
              </div>
            </div>
            <el-empty v-if="!savedWorkflows.length" description="暂无已保存工作流" :image-size="60" />
          </div>
        </div>
      </el-tab-pane>

      <el-tab-pane label="保存 / 执行" name="exec">
        <div class="panel card-pad exec-panel">
          <el-form label-width="100px" style="max-width: 640px">
            <el-form-item label="工作流名称">
              <el-input v-model="execForm.name" placeholder="如 数据归一化流水线" />
            </el-form-item>
            <el-form-item label="节点定义">
              <el-input
                v-model="execForm.definition"
                type="textarea"
                :rows="6"
                placeholder='JSON 定义，例如 {"nodes":[{"id":"n1","node_type":"Start","name":"开始","config":{}}],"edges":[],"variables":{}}'
              />
            </el-form-item>
            <div class="exec-btns">
              <el-button @click="saveWorkflowDef">保存定义</el-button>
              <el-button type="primary" :loading="execing" @click="execWorkflowDef">
                <el-icon><VideoPlay /></el-icon> 执行工作流
              </el-button>
            </div>
          </el-form>
          <pre v-if="execResult" class="exec-out">{{ JSON.stringify(execResult, null, 2) }}</pre>
        </div>
        <div class="panel card-pad" style="margin-top: 14px">
          <h3 class="section-title">节点类型参考</h3>
          <div class="nt-grid">
            <div class="nt-card" v-for="t in nodeTypes" :key="t.type">
              <div class="nt-head">
                <span class="nt-type mono">{{ t.type }}</span>
                <span class="nt-name">{{ t.name }}</span>
              </div>
              <div class="nt-desc">{{ t.description }}</div>
              <div v-if="t.config_fields?.length" class="nt-fields">
                <div v-for="f in t.config_fields" :key="f.name" class="nt-field">
                  <code>{{ f.name }}</code>：{{ f.label }}
                  <span v-if="f.options?.length" class="muted">[{{ f.options.join(' / ') }}]</span>
                </div>
              </div>
            </div>
            <el-empty v-if="!nodeTypes.length" description="暂无节点类型" :image-size="60" />
          </div>
        </div>
      </el-tab-pane>
    </el-tabs>
    </div>

    <el-dialog v-model="showCreate" title="新建流程图" width="460px">
      <el-form label-width="80px">
        <el-form-item label="名称">
          <el-input v-model="createForm.name" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="createForm.description" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showCreate = false">取消</el-button>
        <el-button type="primary" :loading="creating" @click="doCreate">创建</el-button>
      </template>
    </el-dialog>

    <FlowDetailDialog
      v-model="flowDetailOpen"
      :flow-detail="flowDetail"
      :initial-panel="detailPanel"
    />

    <!-- 插件中心 Tab -->
    <div v-show="outerTab === 'plugins'" class="tab-panel">
      <PluginsPanel />
    </div>

    <!-- MCP 兼容 Tab -->
    <div v-show="outerTab === 'mcp'" class="tab-panel">
      <McpPanel />
    </div>

    <!-- 自动化 Tab -->
    <div v-show="outerTab === 'automation'" class="tab-panel">
      <AutomationPanel />
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount, watch, defineAsyncComponent } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { VideoCamera, Document, Promotion } from '@element-plus/icons-vue'
import { useProject } from '@/composables/projectContext.js'
import {
  getFlows,
  createFlow,
  deleteFlow,
  validateFlow,
  executeFlow,
  getWorkflowTemplates,
  getWorkflowInstances,
  getWorkflows,
  getFlow,
  getFlowNodeTypes,
  saveWorkflow,
  executeWorkflowDef
} from '@/api'
import FlowDetailDialog from '@/components/FlowDetailDialog.vue'

const router = useRouter()
const route = useRoute()

// 外层大 Tab：编排 / 插件 / MCP / 自动化
const outerTab = ref(route.query.tab || 'flows')
watch(() => route.query.tab, (t) => {
  if (t) outerTab.value = t
})
function onOuterTabChange(tab) {
  router.replace({ query: { ...route.query, tab } })
}

// 懒加载子面板
const PluginsPanel = defineAsyncComponent(() => import('@/components/PluginsPanel.vue'))
const McpPanel = defineAsyncComponent(() => import('@/components/McpPanel.vue'))
const AutomationPanel = defineAsyncComponent(() => import('@/components/AutomationPanel.vue'))

// AI生成工作流：跳转到AI助手，带上工作流上下文
function goAIGenerate() {
  router.push({ path: '/ai', query: { source: 'workflow', action: 'generate' } })
}

const tab = ref('flows')
const flows = ref([])
const templates = ref([])
const instances = ref([])
const savedWorkflows = ref([])
const nodeTypes = ref([])
const flowDetail = ref(null)
const flowDetailOpen = ref(false)
const detailPanel = ref('')

const showCreate = ref(false)
const creating = ref(false)
const createForm = ref({ name: '', description: '' })

const execForm = ref({ name: '', definition: '' })
const execing = ref(false)
const execResult = ref(null)

async function loadAll() {
  try {
    const [f, t, ins, sw] = await Promise.all([
      getFlows().catch(() => []),
      getWorkflowTemplates().catch(() => []),
      getWorkflowInstances().catch(() => []),
      getWorkflows().catch(() => [])
    ])
    flows.value = Array.isArray(f) ? f : f.flows || f.data || []
    templates.value = t.templates || t.data || t || []
    instances.value = ins.instances || ins.data || ins || []
    savedWorkflows.value = sw.workflows || sw.data || (Array.isArray(sw) ? sw : [])
  } catch (e) {
    ElMessage.error('加载失败：' + e.message)
  }
  getFlowNodeTypes()
    .then((r) => { nodeTypes.value = r.types || [] })
    .catch(() => {})
}

async function viewFlowDetail(f) {
  detailPanel.value = ''
  await openDetail(f)
}

/** 快捷入口：直接打开视频/日志面板 */
async function quickOpen(f, panel) {
  detailPanel.value = panel
  await openDetail(f)
}

async function openDetail(f) {
  try {
    const r = await getFlow(f.id)
    flowDetail.value = r.flow || r || f
    flowDetailOpen.value = true
  } catch (e) {
    flowDetail.value = f
    flowDetailOpen.value = true
  }
}

function loadSavedWorkflow(w) {
  execForm.value.name = w.name
  execForm.value.definition = JSON.stringify({ workflow_id: w.id, name: w.name }, null, 2)
  tab.value = 'exec'
  ElMessage.success('已载入「' + w.name + '」，可直接执行或保存')
}

async function doCreate() {
  if (!createForm.value.name) {
    ElMessage.warning('请填写名称')
    return
  }
  creating.value = true
  try {
    await createFlow(createForm.value)
    ElMessage.success('创建成功')
    showCreate.value = false
    createForm.value = { name: '', description: '' }
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    creating.value = false
  }
}

async function runFlow(f) {
  try {
    const r = await executeFlow({ flow_id: f.id, input: {} })
    ElMessage.success('执行已触发')
    execResult.value = r
  } catch (e) {
    ElMessage.error(e.message)
  }
}
async function doValidateFlow(f) {
  try {
    const r = await validateFlow(f)
    ElMessage.success('校验通过')
    execResult.value = r
  } catch (e) {
    ElMessage.error(e.message)
  }
}
async function delFlow(f) {
  await ElMessageBox.confirm(`确认删除流程图「${f.name || f.id}」？`, '提示', {
    type: 'warning'
  })
  try {
    await deleteFlow(f.id)
    ElMessage.success('已删除')
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
  }
}

function saveFromTpl(t) {
  execForm.value.name = t.name
  execForm.value.definition = JSON.stringify({ template_id: t.id, name: t.name }, null, 2)
  tab.value = 'exec'
}

async function saveWorkflowDef() {
  const wf = buildWorkflowPayload(execForm.value)
  if (!wf) return
  try {
    await saveWorkflow(wf)
    ElMessage.success('已保存')
    await loadAll()
  } catch (e) {
    ElMessage.error(e.message)
  }
}
async function execWorkflowDef() {
  const wf = buildWorkflowPayload(execForm.value)
  if (!wf) return
  execing.value = true
  try {
    execResult.value = await executeWorkflowDef({ workflow: wf })
    ElMessage.success('执行已触发')
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    execing.value = false
  }
}
/**
 * 将「名称 + JSON 定义」规范化为后端 BusinessWorkflow 契约：
 * { id, name, description, nodes, edges, variables, start_node_id, created_at }
 * 定义可包含 nodes/edges/variables/start_node_id，缺失字段自动补默认值。
 */
function buildWorkflowPayload(form) {
  if (!form.name) {
    ElMessage.warning('请填写工作流名称')
    return null
  }
  const def = safeParse(form.definition)
  const nodes = Array.isArray(def.nodes) ? def.nodes : []
  if (!nodes.length) {
    ElMessage.warning('节点定义中缺少 nodes 数组（示例：{"nodes":[...],"edges":[...]}）')
    return null
  }
  const now = new Date().toISOString()
  return {
    id: def.id || 'wf_' + Date.now(),
    name: form.name,
    description: def.description || '手动定义工作流',
    nodes,
    edges: Array.isArray(def.edges) ? def.edges : [],
    variables: def.variables || {},
    start_node_id: def.start_node_id || nodes[0].id || '',
    created_at: def.created_at || now
  }
}
function safeParse(s) {
  try {
    return JSON.parse(s)
  } catch {
    return {}
  }
}

onMounted(loadAll)

// ===== 璇玑：以项目为核心的联动 =====
{
  const { onChange: _onProjectChange, ensureProjectContext: _ensureProject } = useProject()
  let _offPj = null
  let _loaded = false
  onMounted(async () => {
    _offPj = _onProjectChange(async () => { loadAll() })
    await _ensureProject().catch(() => {})
    if (!_loaded) {
      _loaded = true
      loadAll()
    }
  })
  const _ob$ = onBeforeUnmount == null ? null : onBeforeUnmount(() => { _offPj && _offPj() })
  // 若脚本未引入 onBeforeUnmount，退化为 window beforeunload 兜底（页面关闭）
  if (typeof onBeforeUnmount === 'undefined') {
    // 不操作：Vue 路由离开时组件 destroy，本作用域已销毁
  }
}
</script>

<style scoped>
.wf {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
/* 外层 Tab 样式 */
.wf-outer-tabs {
  margin: 0 -2px;
}
:deep(.wf-outer-tabs .el-tabs__header) {
  margin-bottom: 0;
  padding: 0 6px;
  background: var(--bg-card);
  border-radius: 12px;
  border: 1px solid var(--border-1);
}
:deep(.wf-outer-tabs .el-tabs__nav-wrap::after) {
  display: none;
}
:deep(.wf-outer-tabs .el-tabs__item) {
  font-weight: 600;
  font-size: 14px;
  height: 44px;
  line-height: 44px;
}
.tab-panel {
  width: 100%;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.head-actions {
  display: flex;
  gap: 8px;
}
.card-pad {
  padding: 18px 20px;
}
.flow-card {
  padding: 16px 18px;
}
.flow-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.flow-name {
  font-weight: 700;
  font-size: 15px;
}
.more {
  cursor: pointer;
  font-size: 18px;
  color: var(--text-3);
}
.flow-meta {
  display: flex;
  gap: 8px;
  margin: 10px 0;
}
.flow-desc {
  font-size: 13px;
  color: var(--text-3);
  min-height: 38px;
}
.tpl-list,
.inst-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.tpl {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
  background: var(--bg-page);
  border-radius: 9px;
}
.tpl-name {
  font-weight: 600;
}
.tpl-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 2px;
}
.inst {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: var(--bg-page);
  border-radius: 8px;
}
.inst-id {
  font-size: 13px;
  color: var(--text-2);
  font-family: monospace;
}
.exec-panel {
  max-width: 720px;
}
.exec-btns {
  display: flex;
  gap: 10px;
}
.exec-out {
  margin-top: 16px;
  background: #0b1020;
  color: #a5b4fc;
  padding: 14px;
  border-radius: 10px;
  font-size: 12px;
  overflow: auto;
  max-height: 280px;
}
.nt-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 12px;
  margin-top: 12px;
}
.nt-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px;
}
.nt-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.nt-type {
  background: var(--brand-soft, #eef4ff);
  color: var(--brand, #3b6fe0);
  padding: 2px 8px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 700;
}
.nt-name {
  font-size: 13px;
  font-weight: 600;
}
.nt-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-bottom: 8px;
}
.nt-fields {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.nt-field {
  font-size: 12px;
  color: var(--text-2);
}
</style>
