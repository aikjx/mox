<template>
  <div class="detail" v-loading="loading">
    <div class="head">
      <div>
        <el-button text @click="$router.push('/market')"><el-icon><ArrowLeft /></el-icon> 返回商城</el-button>
        <h2 class="page-title">{{ pkg.name }}</h2>
        <div class="badges">
          <span class="badge primary">{{ pkg.category || '未分类' }}</span>
          <span class="badge info">v{{ pkg.version }}</span>
          <span class="badge" v-if="pkg.forked_from">派生自 {{ pkg.forked_from }}</span>
          <span class="muted">{{ pkg.author || '匿名' }} · 克隆 {{ pkg.clone_count }} · 更新 {{ shortTime(pkg.updated_at) }}</span>
        </div>
      </div>
      <div class="head-actions">
        <el-button @click="clonePkg"><el-icon><CopyDocument /></el-icon> 克隆此包</el-button>
        <el-button @click="exportApp"><el-icon><Download /></el-icon> 导出应用包</el-button>
        <el-button type="primary" :loading="saving" @click="saveAll"><el-icon><Select /></el-icon> 保存修改</el-button>
        <el-button type="danger" plain @click="delPkg"><el-icon><Delete /></el-icon> 删除</el-button>
      </div>
    </div>

    <div class="grid grid-2">
      <!-- 左：需求 + 功能点 -->
      <div class="col">
        <div class="panel card-pad">
          <h3 class="section-title">★ 需求描述（核心）</h3>
          <el-input
            v-model="pkg.requirement"
            type="textarea"
            :rows="8"
            placeholder="把需求写清楚，流程图与功能点都可据此快速调整"
          />
        </div>

        <div class="panel card-pad">
          <div class="section-head">
            <h3 class="section-title">功能点清单</h3>
            <el-button size="small" @click="addFeature"><el-icon><Plus /></el-icon> 新增</el-button>
          </div>
          <div v-for="(f, i) in pkg.features" :key="f.id" class="feature">
            <el-input v-model="f.title" placeholder="功能标题" class="f-title" />
            <el-input v-model="f.description" placeholder="说明" class="f-desc" />
            <el-select v-model="f.priority" class="f-prio">
              <el-option label="高" value="high" />
              <el-option label="中" value="medium" />
              <el-option label="低" value="low" />
            </el-select>
            <el-select v-model="f.status" class="f-stat">
              <el-option label="待办" value="todo" />
              <el-option label="进行中" value="doing" />
              <el-option label="完成" value="done" />
            </el-select>
            <el-icon class="f-del" @click="pkg.features.splice(i, 1)"><Close /></el-icon>
          </div>
          <el-empty v-if="!pkg.features.length" description="暂无功能点" :image-size="50" />
        </div>
      </div>

      <!-- 右：可编辑业务流程图（原生 SVG，零依赖） -->
      <div class="panel card-pad flow-panel">
        <div class="section-head">
          <h3 class="section-title">业务流程图（可拖拽编辑）</h3>
          <div class="flow-tools">
            <el-select v-model="newNodeType" size="small" style="width:120px">
              <el-option label="处理" value="process" />
              <el-option label="开始" value="start" />
              <el-option label="结束" value="end" />
              <el-option label="判断" value="decision" />
              <el-option label="输入输出" value="io" />
              <el-option label="算子" value="operator" />
            </el-select>
            <el-button size="small" @click="addNode"><el-icon><Plus /></el-icon> 加节点</el-button>
            <el-button size="small" type="danger" plain :disabled="!selectedNode" @click="delNode">删节点</el-button>
            <el-button size="small" @click="connectMode = !connectMode">
              {{ connectMode ? '取消连线' : '连线模式' }}
            </el-button>
          </div>
        </div>

        <div
          class="canvas"
          ref="canvasEl"
          @mousedown.self="onCanvasDown"
          @mousemove="onCanvasMove"
          @mouseup="onCanvasUp"
        >
          <!-- 连线 -->
          <svg class="edges" :width="canvasW" :height="canvasH">
            <defs>
              <marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto">
                <path d="M0,0 L8,3 L0,6 Z" fill="#94a3b8" />
              </marker>
            </defs>
            <line
              v-for="e in pkg.edges"
              :key="e.id"
              :x1="nodeCenter(e.source).x" :y1="nodeCenter(e.source).y"
              :x2="nodeCenter(e.target).x" :y2="nodeCenter(e.target).y"
              stroke="#94a3b8" stroke-width="2" marker-end="url(#arrow)"
            />
            <text
              v-for="e in pkg.edges"
              :key="'t' + e.id"
              :x="(nodeCenter(e.source).x + nodeCenter(e.target).x) / 2"
              :y="(nodeCenter(e.source).y + nodeCenter(e.target).y) / 2 - 4"
              fill="#64748b" font-size="11"
            >{{ e.label || '' }}</text>
          </svg>

          <!-- 节点 -->
          <div
            v-for="n in pkg.nodes"
            :key="n.id"
            class="node"
            :class="[n.node_type, { sel: selectedNode === n.id, connect: connectMode }]"
            :style="{ left: n.x + 'px', top: n.y + 'px' }"
            @mousedown.stop="onNodeDown(n, $event)"
            @click.stop="onNodeClick(n)"
          >
            <div class="node-label">{{ n.label }}</div>
            <el-popover placement="bottom" :width="220" trigger="click">
              <template #reference><el-icon class="node-edit"><Edit /></el-icon></template>
              <div>
                <el-input v-model="n.label" size="small" placeholder="节点名" style="margin-bottom:6px" />
                <el-input v-model="n.note" size="small" type="textarea" :rows="2" placeholder="备注" />
              </div>
            </el-popover>
          </div>
        </div>

        <div class="flow-tip" v-if="connectMode">
          连线模式：先点起点节点，再点终点节点即可建立连线
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, CopyDocument, Select, Plus, Close, Edit, Download } from '@element-plus/icons-vue'
import { marketGet, marketUpdate, marketClone, marketExport, marketDelete } from '@/api'

const route = useRoute()
const router = useRouter()
const loading = ref(true)
const saving = ref(false)

const pkg = reactive({
  id: '', name: '', category: '', author: '', version: '', summary: '',
  requirement: '', nodes: [], edges: [], features: [], tags: [],
  updated_at: '', clone_count: 0, forked_from: null
})

const canvasEl = ref(null)
const canvasW = ref(900)
const canvasH = ref(540)
const newNodeType = ref('process')
const connectMode = ref(false)
const selectedNode = ref(null)

// 拖拽状态
let drag = null // { id, offsetX, offsetY }
let connectFrom = null

function shortTime(s) {
  if (!s) return '—'
  return s.slice(0, 19).replace('T', ' ')
}
function nodeCenter(id) {
  const n = pkg.nodes.find((x) => x.id === id)
  if (!n) return { x: 0, y: 0 }
  return { x: n.x + 70, y: n.y + 26 }
}

async function load() {
  loading.value = true
  try {
    const r = await marketGet(route.params.id)
    if (!r.success) {
      ElMessage.error(r.error || '加载失败')
      return
    }
    Object.assign(pkg, r.package)
  } catch (e) {
    ElMessage.error('加载失败：' + e.message)
  } finally {
    loading.value = false
  }
}

function addNode() {
  const id = 'n' + Date.now().toString(36)
  pkg.nodes.push({
    id, label: '新节点', node_type: newNodeType.value,
    x: 120 + Math.random() * 200, y: 80 + Math.random() * 160, note: ''
  })
}
function delNode() {
  if (!selectedNode.value) return
  pkg.nodes = pkg.nodes.filter((n) => n.id !== selectedNode.value)
  pkg.edges = pkg.edges.filter((e) => e.source !== selectedNode.value && e.target !== selectedNode.value)
  selectedNode.value = null
}
function addFeature() {
  pkg.features.push({ id: 'f' + Date.now().toString(36), title: '', description: '', priority: 'medium', status: 'todo' })
}

function onNodeDown(n, ev) {
  if (connectMode.value) return
  selectedNode.value = n.id
  const rect = canvasEl.value.getBoundingClientRect()
  drag = { id: n.id, offsetX: ev.clientX - rect.left - n.x, offsetY: ev.clientY - rect.top - n.y }
}
function onCanvasDown() {
  selectedNode.value = null
}
function onCanvasMove(ev) {
  if (!drag) return
  const rect = canvasEl.value.getBoundingClientRect()
  const n = pkg.nodes.find((x) => x.id === drag.id)
  if (!n) return
  n.x = Math.max(0, ev.clientX - rect.left - drag.offsetX)
  n.y = Math.max(0, ev.clientY - rect.top - drag.offsetY)
}
function onCanvasUp() {
  drag = null
}
function onNodeClick(n) {
  if (!connectMode.value) {
    selectedNode.value = n.id
    return
  }
  if (!connectFrom) {
    connectFrom = n.id
    selectedNode.value = n.id
  } else if (connectFrom !== n.id) {
    const exists = pkg.edges.some((e) => e.source === connectFrom && e.target === n.id)
    if (!exists) {
      pkg.edges.push({ id: 'e' + Date.now().toString(36), source: connectFrom, target: n.id, label: '' })
    }
    connectFrom = null
  }
}

async function saveAll() {
  if (!pkg.requirement.trim()) return ElMessage.warning('需求描述不能为空')
  saving.value = true
  try {
    await marketUpdate(pkg.id, {
      name: pkg.name, category: pkg.category, author: pkg.author,
      version: pkg.version, summary: pkg.summary, requirement: pkg.requirement,
      nodes: pkg.nodes, edges: pkg.edges, features: pkg.features, tags: pkg.tags
    })
    ElMessage.success('已保存')
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    saving.value = false
  }
}
async function clonePkg() {
  try {
    const r = await marketClone(pkg.id)
    ElMessage.success('已克隆，进入新包编辑')
    router.push(`/market/${r.id}`)
  } catch (e) {
    ElMessage.error(e.message)
  }
}

async function delPkg() {
  try {
    await ElMessageBox.confirm(
      `确定删除「${pkg.name}」吗？该操作不可恢复。`,
      '删除算子包',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' }
    )
  } catch {
    return // 取消
  }
  try {
    await marketDelete(pkg.id)
    ElMessage.success('已删除')
    router.push('/market')
  } catch (e) {
    ElMessage.error('删除失败：' + e.message)
  }
}

// 导出为可下载分发的应用包：优先走后端 /market/:id/export（标准 FlowDefinition DSL 工程），
// 后端不可用时回退到本地 .ousapp JSON，保证导出功能始终可用
async function exportApp() {
  try {
    const dsl = await marketExport(pkg.id)
    const blob = new Blob([JSON.stringify(dsl, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${pkg.name || 'application'}.flow-definition.json`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    ElMessage.success('已导出标准 FlowDefinition DSL 工程（可被任何 OUS 实例导入）')
  } catch {
    // 回退：本地 .ousapp 导出
    const app = {
      format: 'ousapp',
      formatVersion: '1.0',
      exportedAt: new Date().toISOString(),
      app: {
        id: pkg.id,
        name: pkg.name,
        category: pkg.category || '未分类',
        version: pkg.version || '1.0.0',
        author: pkg.author || '匿名',
        summary: pkg.summary || '',
        requirement: pkg.requirement || '',
        features: pkg.features || [],
        nodes: pkg.nodes || [],
        edges: pkg.edges || [],
        tags: pkg.tags || [],
      },
      openapi: {
        info: { title: pkg.name, version: pkg.version || '1.0.0', description: pkg.summary },
        tags: pkg.tags || [],
      },
    }
    const blob = new Blob([JSON.stringify(app, null, 2)], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${pkg.name || 'application'}.ousapp`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
    ElMessage.success('已导出应用包（.ousapp），可分享给任何使用者一键导入')
  }
}

onMounted(load)
</script>

<style scoped>
.detail { display: flex; flex-direction: column; gap: 16px; }
.head { display: flex; align-items: flex-start; justify-content: space-between; }
.badges { display: flex; gap: 8px; align-items: center; margin-top: 6px; flex-wrap: wrap; }
.head-actions { display: flex; gap: 10px; }
.muted { font-size: 12px; color: var(--text-3); }
.badge { font-size: 11px; padding: 2px 9px; border-radius: 999px; background: var(--bg-page); color: var(--text-2); }
.badge.primary { background: var(--brand); color: #fff; }
.badge.info { background: var(--brand-soft); color: var(--brand-dark); }

.grid-2 { display: grid; grid-template-columns: 1fr 1.2fr; gap: 16px; align-items: start; }
.col { display: flex; flex-direction: column; gap: 16px; }
.card-pad { padding: 18px 20px; }
.section-title { font-size: 14px; font-weight: 700; color: var(--text-1); margin: 0; }
.section-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }

.feature { display: grid; grid-template-columns: 1.2fr 1.6fr 80px 90px 24px; gap: 6px; align-items: center; margin-bottom: 8px; }
.f-del { color: var(--text-3); cursor: pointer; justify-self: center; }
.f-del:hover { color: var(--danger); }

.flow-panel { display: flex; flex-direction: column; }
.flow-tools { display: flex; gap: 6px; }
.canvas {
  position: relative; width: 100%; height: 540px; margin-top: 12px;
  background:
    linear-gradient(rgba(99,102,241,0.05) 1px, transparent 1px) 0 0 / 22px 22px,
    linear-gradient(90deg, rgba(99,102,241,0.05) 1px, transparent 1px) 0 0 / 22px 22px,
    #fafbff;
  border: 1px solid var(--border); border-radius: 10px; overflow: hidden; user-select: none;
}
.edges { position: absolute; left: 0; top: 0; pointer-events: none; }
.node {
  position: absolute; width: 140px; min-height: 52px; padding: 8px 10px 10px;
  border-radius: 10px; background: #fff; border: 2px solid var(--brand-light);
  box-shadow: 0 4px 12px rgba(15,23,42,0.08); cursor: grab; display: flex; flex-direction: column; gap: 4px;
}
.node:active { cursor: grabbing; }
.node.sel { border-color: var(--brand); box-shadow: 0 0 0 3px rgba(99,102,241,0.25); }
.node.connect { cursor: crosshair; }
.node-label { font-size: 13px; font-weight: 600; color: var(--text-1); }
.node-edit { position: absolute; right: 6px; top: 6px; color: var(--text-3); cursor: pointer; }
.node-edit:hover { color: var(--brand); }
.node.start { border-color: #10b981; background: #ecfdf5; }
.node.end { border-color: #ef4444; background: #fef2f2; }
.node.decision { border-color: #f59e0b; background: #fffbeb; border-radius: 4px; }
.node.io { border-color: #06b6d4; background: #ecfeff; }
.node.operator { border-color: #8b5cf6; background: #f3e8ff; }
.flow-tip { margin-top: 10px; font-size: 12px; color: var(--brand-dark); background: var(--brand-soft); padding: 6px 10px; border-radius: 8px; }
</style>
