<template>
  <div class="page-container">
    <div class="page-header">
      <div class="page-header-left">
        <h2 class="page-title">项目中心</h2>
        <p class="page-subtitle">
          全维项目化归类 · 平台系统 / MCP / 插件 / APP / PC / Skills / Loop / Graph Agents / 自动化 …
          覆盖璇玑全部 {{ stats.catalog_total || catalogTotal }} 项资源
        </p>
      </div>
      <div class="page-header-actions">
        <el-button type="primary" @click="openCreate">
          <el-icon><Plus /></el-icon> 新建项目
        </el-button>
      </div>
    </div>

    <div class="page-content">
    <!-- 操作引导：4步用好项目中心 -->
    <div class="flow-guide">
      <div class="fg-item"><span class="fg-num">1</span><span>新建项目</span><small>填写名称/类型/描述</small></div>
      <div class="fg-arrow">→</div>
      <div class="fg-item"><span class="fg-num">2</span><span>顶栏选择</span><small>自动注入全页上下文</small></div>
      <div class="fg-arrow">→</div>
      <div class="fg-item"><span class="fg-num">3</span><span>绑定资源</span><small>会话/任务/图谱/算子全归类</small></div>
      <div class="fg-arrow">→</div>
      <div class="fg-item"><span class="fg-num">4</span><span>查看统计</span><small>项目进度/资源分布一目了然</small></div>
    </div>

    <!-- 统计卡 -->
    <div class="grid grid-4 stat-grid">
      <div class="panel stat-card">
        <div class="stat-num">{{ stats.total ?? 0 }}</div>
        <div class="stat-label">项目总数</div>
      </div>
      <div class="panel stat-card">
        <div class="stat-num" style="color: var(--success)">{{ stats.active ?? 0 }}</div>
        <div class="stat-label">进行中项目</div>
      </div>
      <div class="panel stat-card">
        <div class="stat-num" style="color: var(--brand)">{{ stats.bound_resources ?? 0 }}</div>
        <div class="stat-label">已归档资源</div>
      </div>
      <div class="panel stat-card">
        <div class="stat-num" style="color: var(--accent)">{{ stats.catalog_total ?? catalogTotal }}</div>
        <div class="stat-label">目录资源 · {{ resourceTypeCount }} 类全维</div>
      </div>
    </div>

    <!-- 类型分布 -->
    <div class="panel card-pad type-strip" v-if="categories.length">
      <div
        v-for="c in categories"
        :key="c.key"
        class="type-chip"
        :class="{ active: filterCategory === c.key }"
        :style="filterCategory === c.key ? { background: c.color + '22', borderColor: c.color } : {}"
        @click="filterCategory = filterCategory === c.key ? '' : c.key"
      >
        <el-icon :style="{ color: c.color }"><component :is="c.icon" /></el-icon>
        <span>{{ c.label }}</span>
        <span class="type-count">{{ (stats.by_category || {})[c.key] || 0 }}</span>
      </div>
    </div>

    <div class="proj-body">
      <!-- 左：项目列表 -->
      <div class="panel proj-list">
        <div class="list-head">
          <el-input v-model="keyword" placeholder="搜索项目…" size="small" clearable>
            <template #prefix><el-icon><Search /></el-icon></template>
          </el-input>
        </div>
        <el-scrollbar class="list-scroll">
          <div
            v-for="p in filteredProjects"
            :key="p.id"
            class="proj-item"
            :class="{ active: p.id === current?.id }"
            @click="selectProject(p.id)"
          >
            <div class="proj-item-head">
              <span class="proj-dot" :style="{ background: p.color || '#64748b' }"></span>
              <span class="proj-name">{{ p.name || '未命名项目' }}</span>
              <span class="badge" :class="statusClass(p.status)">{{ statusLabel(p.status) }}</span>
            </div>
            <div class="proj-item-meta">
              <span>{{ categoryLabel(p.category) }}</span>
              <span class="proj-count">{{ p.resource_count ?? (p.resources || []).length }} 项资源</span>
            </div>
          </div>
          <el-empty v-if="!filteredProjects.length" description="暂无项目，点击右上角新建" :image-size="60" />
        </el-scrollbar>
      </div>

      <!-- 右：项目详情 -->
      <div class="panel detail" v-if="current">
        <div class="detail-head">
          <div class="detail-title-wrap">
            <div class="detail-title">
              <span class="proj-dot lg" :style="{ background: current.color }"></span>
              {{ current.name || '未命名项目' }}
              <span class="badge" :class="statusClass(current.status)">{{ statusLabel(current.status) }}</span>
            </div>
            <div class="detail-sub">
              <el-tag size="small" effect="plain" :color="categoryOf(current.category)?.color + '22'">
                {{ categoryOf(current.category)?.label || current.category }}
              </el-tag>
              <span class="muted">{{ current.description || '暂无描述' }}</span>
            </div>
          </div>
          <div class="detail-actions">
            <el-button size="small" @click="openEdit">
              <el-icon><Edit /></el-icon> 编辑
            </el-button>
            <el-button size="small" type="danger" plain @click="removeProject">
              <el-icon><Delete /></el-icon> 删除
            </el-button>
          </div>
        </div>

        <!-- 快速操作：一键跳转到相关模块，自动带上项目上下文 -->
        <div class="quick-actions">
          <div class="qa-label">快速操作</div>
          <div class="qa-buttons">
            <el-button size="small" type="primary" @click="goModule('ai')">
              <el-icon><Promotion /></el-icon> AI全维开发
            </el-button>
            <el-button size="small" @click="goModule('ai')">
              <el-icon><ChatDotRound /></el-icon> AI对话
            </el-button>
            <el-button size="small" @click="goModule('tasks')">
              <el-icon><List /></el-icon> 任务管理
            </el-button>
            <el-button size="small" @click="goModule('graph')">
              <el-icon><Share /></el-icon> 知识图谱
            </el-button>
            <el-button size="small" @click="goModule('workflow')">
              <el-icon><Connection /></el-icon> 工作流
            </el-button>
            <el-button size="small" @click="goModule('expert-center')">
              <el-icon><User /></el-icon> 专家联盟
            </el-button>
            <el-button size="small" @click="goModule('knowledge-base')">
              <el-icon><Folder /></el-icon> 知识库
            </el-button>
          </div>
        </div>

        <!-- 资源归类区 -->
        <div class="res-head">
          <h3 class="section-title">资源归类 · {{ (current.resources || []).length }} 项</h3>
          <el-button type="primary" size="small" @click="openBinder">
            <el-icon><Plus /></el-icon> 归类资源
          </el-button>
        </div>

        <div v-for="(group, type) in groupedResources" :key="type" class="res-group">
          <div class="res-group-head">
            <span class="res-type-badge">{{ typeLabel(type) }}</span>
            <span class="muted">{{ group.length }} 项</span>
            <el-button
              v-if="resourceRoute(type)"
              size="small" text type="primary"
              @click="$router.push(resourceRoute(type))"
            >
              前往模块 <el-icon><ArrowRight /></el-icon>
            </el-button>
          </div>
          <div v-for="r in group" :key="r.rid" class="res-row">
            <div class="res-info">
              <span class="res-name">{{ r.resource_name }}</span>
              <span class="badge" :class="liveClass(r.live_status)">{{ liveLabel(r.live_status) }}</span>
              <span class="muted res-desc">{{ r.live_desc || r.note || '' }}</span>
            </div>
            <div class="res-ops">
              <el-button size="small" text @click="editNote(r)">备注</el-button>
              <el-button size="small" text type="danger" @click="unbind(r)">移除</el-button>
            </div>
          </div>
        </div>
        <el-empty
          v-if="!current.resources || !current.resources.length"
          description="尚未归类任何资源，点击「归类资源」从全维目录选取"
          :image-size="80"
        />
      </div>
      <div class="panel detail empty-detail" v-else>
        <el-empty description="选择或新建一个项目，开始全维归类" :image-size="100" />
      </div>
    </div>
    </div>

    <!-- 新建 / 编辑项目 -->
    <el-dialog v-model="dlg.visible" :title="dlg.isEdit ? '编辑项目' : '新建项目'" width="520px">
      <el-form label-width="76px">
        <el-form-item label="项目名称">
          <el-input v-model="dlg.form.name" placeholder="如：璇玑平台系统" maxlength="60" />
        </el-form-item>
        <el-form-item label="项目类型">
          <div class="cat-picker">
            <div
              v-for="c in categories"
              :key="c.key"
              class="cat-option"
              :class="{ active: dlg.form.category === c.key }"
              :style="dlg.form.category === c.key ? { borderColor: c.color, background: c.color + '14' } : {}"
              @click="dlg.form.category = c.key"
            >
              <el-icon :style="{ color: c.color }"><component :is="c.icon" /></el-icon>
              <span>{{ c.label }}</span>
            </div>
          </div>
        </el-form-item>
        <el-form-item label="状态">
          <el-radio-group v-model="dlg.form.status">
            <el-radio value="active">进行中</el-radio>
            <el-radio value="done">已完成</el-radio>
            <el-radio value="archived">已归档</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="dlg.form.description" type="textarea" :rows="3" placeholder="项目说明（可选）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dlg.visible = false">取消</el-button>
        <el-button type="primary" :loading="dlg.saving" @click="saveProject">保存</el-button>
      </template>
    </el-dialog>

    <!-- 全维目录归档抽屉 -->
    <el-drawer v-model="binder.visible" title="全维资源目录 · 归类到项目" size="620px">
      <div class="binder-head">
        <el-input v-model="binder.keyword" placeholder="搜索全部资源…" clearable>
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <div class="binder-picked" v-if="binder.picked.size">
          已选 <b>{{ binder.picked.size }}</b> 项
          <el-button size="small" type="primary" :loading="binder.binding" @click="doBind">
            归档到「{{ current?.name }}」
          </el-button>
        </div>
      </div>
      <el-scrollbar class="binder-scroll">
        <el-collapse v-model="binder.openGroups">
          <el-collapse-item v-for="g in filteredGroups" :key="g.type" :name="g.type">
            <template #title>
              <span class="binder-group-title">
                <span class="res-type-badge">{{ g.label }}</span>
                <span class="muted">{{ g.count }} 项</span>
              </span>
            </template>
            <div
              v-for="it in g.items"
              :key="it.id"
              class="binder-item"
              :class="{ picked: binder.picked.has(g.type + ':' + it.id), bound: isBound(g.type, it.id) }"
              @click="togglePick(g.type, it.id, it)"
            >
              <div class="binder-item-main">
                <span class="res-name">{{ it.name }}</span>
                <span class="badge" :class="liveClass(it.status)">{{ it.status }}</span>
                <span class="muted binder-desc">{{ it.desc }}</span>
              </div>
              <el-tag v-if="isBound(g.type, it.id)" size="small" type="info">已归档</el-tag>
              <el-icon v-else-if="binder.picked.has(g.type + ':' + it.id)" class="pick-mark"><CircleCheckFilled /></el-icon>
            </div>
          </el-collapse-item>
        </el-collapse>
        <el-empty v-if="!filteredGroups.length" description="未匹配到资源" :image-size="70" />
      </el-scrollbar>
    </el-drawer>

    <!-- 资源备注编辑 -->
    <el-dialog v-model="noteDlg.visible" title="资源备注" width="420px">
      <el-input v-model="noteDlg.note" type="textarea" :rows="3" placeholder="该资源在此项目中的定位 / 用途说明" />
      <template #footer>
        <el-button @click="noteDlg.visible = false">取消</el-button>
        <el-button type="primary" @click="saveNote">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useProject } from '@/composables/projectContext.js'
import {
  getProjects, getProjectTypes, getProjectCatalog, getProjectStats,
  getProject, createProject, updateProject, deleteProject,
  bindProjectResources, unbindProjectResource, updateProjectResourceNote
} from '@/api'

const router = useRouter()

// ===== 状态 =====
const projects = ref([])
const categories = ref([])
const resourceTypes = ref([])
const catalogGroups = ref([])
const stats = ref({})
const current = ref(null)
const keyword = ref('')
const filterCategory = ref('')

const dlg = ref({ visible: false, isEdit: false, saving: false, form: { name: '', category: 'custom', status: 'active', description: '' } })
const binder = ref({ visible: false, keyword: '', picked: new Map(), openGroups: [], binding: false })
const noteDlg = ref({ visible: false, rid: '', note: '' })

// ===== 计算 =====
const catalogTotal = computed(() => catalogGroups.value.reduce((n, g) => n + g.count, 0))
const resourceTypeCount = computed(() => catalogGroups.value.length)

const filteredProjects = computed(() => {
  let list = projects.value
  if (filterCategory.value) list = list.filter((p) => p.category === filterCategory.value)
  if (keyword.value.trim()) {
    const k = keyword.value.trim().toLowerCase()
    list = list.filter((p) => (p.name || '').toLowerCase().includes(k) || (p.description || '').includes(keyword.value.trim()))
  }
  return list
})

const groupedResources = computed(() => {
  const map = {}
  for (const r of current.value?.resources || []) {
    (map[r.resource_type] = map[r.resource_type] || []).push(r)
  }
  return map
})

const filteredGroups = computed(() => {
  const k = binder.value.keyword.trim().toLowerCase()
  if (!k) return catalogGroups.value
  return catalogGroups.value
    .map((g) => ({ ...g, items: g.items.filter((it) => (it.name || '').toLowerCase().includes(k) || (it.desc || '').includes(k)) }))
    .filter((g) => g.items.length)
})

// ===== 工具 =====
const categoryOf = (key) => categories.value.find((c) => c.key === key)
const categoryLabel = (key) => categoryOf(key)?.label || key || '未分类'
const typeLabel = (key) => resourceTypes.value.find((t) => t.key === key)?.label || key
const resourceRoute = (key) => resourceTypes.value.find((t) => t.key === key)?.route || ''

const statusLabel = (s) => ({ active: '进行中', done: '已完成', archived: '已归档' }[s] || s || '进行中')
const statusClass = (s) => ({ active: 'success', done: 'info', archived: 'warning' }[s] || 'info')
const liveLabel = (s) => ({ online: '在线', running: '运行中', active: '活跃', valid: '有效', missing: '已失效', disabled: '已停用', stopped: '已停止', invalid: '无效' }[s] || s || '-')
const liveClass = (s) => {
  if (['online', 'running', 'active', 'valid', 'published', 'registered'].includes(s)) return 'success'
  if (['missing', 'invalid'].includes(s)) return 'danger'
  if (['disabled', 'stopped'].includes(s)) return 'warning'
  return 'info'
}

const isBound = (type, id) => (current.value?.resources || []).some((r) => r.resource_type === type && String(r.resource_id) === String(id))

// ===== 加载 =====
async function loadAll() {
  const [ps, ts, cat, st] = await Promise.all([getProjects(), getProjectTypes(), getProjectCatalog(), getProjectStats()])
  projects.value = ps || []
  categories.value = (ts && ts.categories) || []
  resourceTypes.value = (ts && ts.resource_types) || []
  catalogGroups.value = (cat && cat.groups) || []
  stats.value = st || {}
  binder.value.openGroups = (catalogGroups.value).slice(0, 3).map((g) => g.type)
}

async function selectProject(id) {
  current.value = await getProject(id)
}

// 快速跳转到相关模块，自动带上项目上下文
function goModule(path) {
  if (!current.value) return
  // 项目上下文已通过全局 provide 注入，跳转后目标页面自动使用当前项目
  router.push(`/${path}`)
  ElMessage.info(`已进入「${current.value.name || '未命名项目'}」项目上下文`)
}

// ===== 项目 CRUD =====
function openCreate() {
  dlg.value = { visible: true, isEdit: false, saving: false, form: { name: '', category: 'platform', status: 'active', description: '' } }
}
function openEdit() {
  dlg.value = { visible: true, isEdit: true, saving: false, form: { name: current.value.name, category: current.value.category, status: current.value.status || 'active', description: current.value.description || '' } }
}
async function saveProject() {
  const f = dlg.value.form
  if (!f.name.trim()) return ElMessage.warning('请输入项目名称')
  dlg.value.saving = true
  try {
    if (dlg.value.isEdit) {
      await updateProject(current.value.id, f)
      ElMessage.success('项目已更新')
    } else {
      const created = await createProject(f)
      ElMessage.success('项目已创建')
      await refreshList()
      await selectProject(created.id)
    }
    if (dlg.value.isEdit) await refreshCurrent()
    dlg.value.visible = false
  } finally {
    dlg.value.saving = false
  }
}
async function removeProject() {
  await ElMessageBox.confirm(`确定删除项目「${current.value.name || '未命名项目'}」？资源本身不受影响，仅解除归类。`, '删除项目', { type: 'warning' })
  await deleteProject(current.value.id)
  ElMessage.success('已删除')
  current.value = null
  await refreshList()
}
async function refreshList() {
  const [ps, st] = await Promise.all([getProjects(), getProjectStats()])
  projects.value = ps
  stats.value = st
}
async function refreshCurrent() {
  if (current.value) current.value = await getProject(current.value.id)
}

// ===== 资源绑定 =====
function openBinder() {
  binder.value.keyword = ''
  binder.value.picked = new Map()
  binder.value.visible = true
}
function togglePick(type, id, item) {
  const key = type + ':' + id
  if (binder.value.picked.has(key)) binder.value.picked.delete(key)
  else binder.value.picked.set(key, { type, id, name: item.name })
}
async function doBind() {
  binder.value.binding = true
  try {
    const items = [...binder.value.picked.values()]
    const r = await bindProjectResources(current.value.id, { items })
    ElMessage.success(`已归档 ${r.added} 项资源`)
    binder.value.visible = false
    await refreshCurrent()
    await refreshList()
  } finally {
    binder.value.binding = false
  }
}
async function unbind(r) {
  await unbindProjectResource(current.value.id, r.rid)
  ElMessage.success('已移除归类')
  await refreshCurrent()
  await refreshList()
}

// ===== 备注 =====
function editNote(r) {
  noteDlg.value = { visible: true, rid: r.rid, note: r.note || '' }
}
async function saveNote() {
  await updateProjectResourceNote(current.value.id, noteDlg.value.rid, { note: noteDlg.value.note })
  ElMessage.success('备注已保存')
  noteDlg.value.visible = false
  await refreshCurrent()
}

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
/* 操作引导条 */
.flow-guide {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 12px 16px;
  background: linear-gradient(135deg, rgba(16,185,129,0.06), rgba(6,182,212,0.04));
  border: 1px solid rgba(16,185,129,0.15);
  border-radius: 10px;
  margin-bottom: 14px;
  flex-wrap: wrap;
}
.fg-item {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 140px;
}
.fg-num {
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: linear-gradient(135deg, #10b981, #06b6d4);
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 12px;
  flex-shrink: 0;
}
.fg-item span {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}
.fg-item small {
  font-size: 11px;
  color: var(--text-secondary);
  display: block;
  margin-top: 1px;
}
.fg-arrow {
  color: #10b981;
  font-size: 14px;
  font-weight: 700;
  flex-shrink: 0;
}
.stat-grid { margin-bottom: 14px; }
.stat-card { padding: 18px 20px; text-align: center; }
.stat-num { font-size: 28px; font-weight: 800; line-height: 1.2; }
.stat-label { font-size: 12px; color: var(--text-3); margin-top: 4px; }

.type-strip { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 14px; }
.type-chip {
  display: inline-flex; align-items: center; gap: 6px;
  padding: 5px 12px; border-radius: 999px; border: 1px solid var(--border);
  font-size: 12px; color: var(--text-2); cursor: pointer; user-select: none;
  transition: all 0.15s;
}
.type-chip:hover { border-color: var(--brand); }
.type-count { font-weight: 700; }

.proj-body { display: grid; grid-template-columns: 300px 1fr; gap: 14px; align-items: start; }
.proj-list { padding: 12px; }
.list-head { margin-bottom: 10px; }
.list-scroll { height: calc(100vh - 430px); min-height: 300px; }
.proj-item {
  padding: 10px 12px; border-radius: 10px; cursor: pointer;
  border: 1px solid transparent; transition: all 0.15s; margin-bottom: 4px;
}
.proj-item:hover { background: var(--bg-page); }
.proj-item.active { background: var(--brand-soft); border-color: var(--brand-light); }
.proj-item-head { display: flex; align-items: center; gap: 8px; }
.proj-dot { width: 10px; height: 10px; border-radius: 3px; flex-shrink: 0; }
.proj-dot.lg { width: 14px; height: 14px; border-radius: 4px; }
.proj-name { font-weight: 600; font-size: 14px; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.proj-item-meta { display: flex; justify-content: space-between; margin-top: 5px; font-size: 12px; color: var(--text-3); padding-left: 18px; }
.proj-count { color: var(--brand); font-weight: 600; }

.detail { padding: 18px 20px; min-height: 400px; }
.empty-detail { display: grid; place-items: center; }
.detail-head { display: flex; justify-content: space-between; align-items: flex-start; padding-bottom: 14px; border-bottom: 1px solid var(--border); }
.detail-title { display: flex; align-items: center; gap: 10px; font-size: 18px; font-weight: 700; }
.detail-sub { display: flex; align-items: center; gap: 10px; margin-top: 8px; font-size: 13px; }
.detail-actions { display: flex; gap: 8px; flex-shrink: 0; }

/* 快速操作区 */
.quick-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  margin: 14px 0;
  background: linear-gradient(135deg, rgba(99,102,241,0.05), rgba(14,165,233,0.04));
  border: 1px solid rgba(99,102,241,0.12);
  border-radius: 10px;
}
.qa-label {
  font-size: 12px;
  font-weight: 600;
  color: #6366f1;
  white-space: nowrap;
  flex-shrink: 0;
}
.qa-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  flex: 1;
}
.qa-buttons .el-button {
  border-radius: 7px;
}

.res-head { display: flex; justify-content: space-between; align-items: center; margin: 16px 0 10px; }
.res-group { margin-bottom: 14px; }
.res-group-head { display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }
.res-type-badge {
  font-size: 12px; font-weight: 600; padding: 2px 10px; border-radius: 6px;
  background: var(--brand-soft); color: var(--brand-dark);
}
.res-row {
  display: flex; justify-content: space-between; align-items: center; gap: 10px;
  padding: 8px 12px; border: 1px solid var(--border); border-radius: 8px; margin-bottom: 4px;
  background: var(--bg-page);
}
.res-info { display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1; }
.res-name { font-size: 13px; font-weight: 600; white-space: nowrap; }
.res-desc { font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.res-ops { flex-shrink: 0; }

.cat-picker { display: flex; flex-wrap: wrap; gap: 8px; }
.cat-option {
  display: inline-flex; align-items: center; gap: 6px;
  padding: 6px 12px; border: 1px solid var(--border); border-radius: 8px;
  font-size: 12px; cursor: pointer; transition: all 0.15s;
}
.cat-option:hover { border-color: var(--brand); }

.binder-head { display: flex; flex-direction: column; gap: 10px; margin-bottom: 12px; }
.binder-picked { display: flex; align-items: center; gap: 10px; font-size: 13px; }
.binder-scroll { height: calc(100vh - 190px); }
.binder-group-title { display: flex; align-items: center; gap: 10px; }
.binder-item {
  display: flex; justify-content: space-between; align-items: center; gap: 8px;
  padding: 8px 10px; border: 1px solid var(--border); border-radius: 8px;
  margin-bottom: 4px; cursor: pointer; transition: all 0.12s;
}
.binder-item:hover { border-color: var(--brand-light); }
.binder-item.picked { border-color: var(--brand); background: var(--brand-soft); }
.binder-item.bound { opacity: 0.55; cursor: not-allowed; }
.binder-item-main { display: flex; align-items: center; gap: 8px; min-width: 0; flex: 1; }
.binder-desc { font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.pick-mark { color: var(--success); font-size: 18px; }
</style>
