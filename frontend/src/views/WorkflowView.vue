<template>
  <div class="wf">
    <div class="head">
      <div>
        <h2 class="page-title">工作流编排</h2>
        <p class="page-subtitle">基于 BPMN 引擎驱动 AI 执行 · 流程图设计 / 模板 / 实例</p>
      </div>
      <div class="head-actions">
        <el-button @click="loadAll"><el-icon><Refresh /></el-icon> 刷新</el-button>
        <el-button type="primary" @click="showCreate = true">
          <el-icon><Plus /></el-icon> 新建流程图
        </el-button>
      </div>
    </div>

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
                placeholder='JSON 定义，例如 {"nodes":[...],"edges":[...]}'
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
      </el-tab-pane>
    </el-tabs>

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
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  getFlows,
  createFlow,
  deleteFlow,
  validateFlow,
  executeFlow,
  getWorkflowTemplates,
  getWorkflowInstances,
  saveWorkflow,
  executeWorkflowDef
} from '@/api'

const tab = ref('flows')
const flows = ref([])
const templates = ref([])
const instances = ref([])

const showCreate = ref(false)
const creating = ref(false)
const createForm = ref({ name: '', description: '' })

const execForm = ref({ name: '', definition: '' })
const execing = ref(false)
const execResult = ref(null)

async function loadAll() {
  try {
    const [f, t, ins] = await Promise.all([
      getFlows().catch(() => []),
      getWorkflowTemplates().catch(() => []),
      getWorkflowInstances().catch(() => [])
    ])
    flows.value = Array.isArray(f) ? f : f.flows || f.data || []
    templates.value = t.templates || t.data || t || []
    instances.value = ins.instances || ins.data || ins || []
  } catch (e) {
    ElMessage.error('加载失败：' + e.message)
  }
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
    const r = await executeFlow({ id: f.id, input: {} })
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
  try {
    await saveWorkflow({
      name: execForm.value.name,
      definition: safeParse(execForm.value.definition)
    })
    ElMessage.success('已保存')
  } catch (e) {
    ElMessage.error(e.message)
  }
}
async function execWorkflowDef() {
  execing.value = true
  try {
    execResult.value = await executeWorkflowDef({
      name: execForm.value.name,
      definition: safeParse(execForm.value.definition)
    })
    ElMessage.success('执行已触发')
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    execing.value = false
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
</script>

<style scoped>
.wf {
  display: flex;
  flex-direction: column;
  gap: 16px;
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
</style>
