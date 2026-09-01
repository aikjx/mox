<template>
  <div class="pp-wrap" :class="{ 'pp-sb-mode': variant === 'sidebar', 'pp-sb-collapsed': collapsed && variant === 'sidebar' }">
    <!-- ===== 顶栏 / 快捷栏 横向变体 ===== -->
    <template v-if="variant === 'top'">
      <el-select
      :model-value="currentId"
      filterable
      size="default"
      :loading="listLoading"
      placeholder="选择项目…"
      class="pp-select"
      popper-class="pp-popper"
      @change="onChange"
      @visible-change="onVisible"
    >
      <template #prefix>
        <span class="pp-dot" :style="{ background: color || '#64748b' }"></span>
      </template>
      <template #default>
        <el-option-group label="进行中">
          <el-option
            v-for="p in activeProjects"
            :key="p.id"
            :label="p.name"
            :value="p.id"
          >
            <div class="pp-opt">
              <span class="pp-dot" :style="{ background: p.color || '#64748b' }"></span>
              <div class="pp-opt-body">
                <span class="pp-opt-name">{{ p.name || '未命名项目' }}</span>
                <span class="pp-opt-desc" v-if="p.description">{{ p.description }}</span>
              </div>
              <el-tag size="small" effect="plain" type="success">进行中</el-tag>
            </div>
          </el-option>
        </el-option-group>
        <el-option-group label="规划中" v-if="planningProjects.length">
          <el-option
            v-for="p in planningProjects"
            :key="p.id"
            :label="p.name"
            :value="p.id"
          >
            <div class="pp-opt">
              <span class="pp-dot" :style="{ background: p.color || '#64748b' }"></span>
              <div class="pp-opt-body">
                <span class="pp-opt-name">{{ p.name || '未命名项目' }}</span>
                <span class="pp-opt-desc" v-if="p.description">{{ p.description }}</span>
              </div>
              <el-tag size="small" effect="plain" type="warning">规划中</el-tag>
            </div>
          </el-option>
        </el-option-group>
        <el-option-group label="已完成" v-if="doneProjects.length">
          <el-option
            v-for="p in doneProjects"
            :key="p.id"
            :label="p.name"
            :value="p.id"
          >
            <div class="pp-opt">
              <span class="pp-dot" :style="{ background: p.color || '#64748b' }"></span>
              <div class="pp-opt-body">
                <span class="pp-opt-name">{{ p.name || '未命名项目' }}</span>
                <span class="pp-opt-desc" v-if="p.description">{{ p.description }}</span>
              </div>
              <el-tag size="small" effect="plain" type="info">已完成</el-tag>
            </div>
          </el-option>
        </el-option-group>
      </template>

      <template #empty>
        <div class="pp-empty">
          <el-empty description="暂无项目" :image-size="44" />
          <el-button type="primary" size="small" @click="openCreate">
            <el-icon><Plus /></el-icon> 创建第一个项目
          </el-button>
        </div>
      </template>
    </el-select>

    <el-button class="pp-new" circle size="small" @click="openCreate" title="新建项目">
      <el-icon :size="13"><Plus /></el-icon>
    </el-button>

    <el-button class="pp-manage" circle size="small" @click="goProjects" title="管理项目">
      <el-icon :size="13"><Setting /></el-icon>
    </el-button>
    </template>

    <!-- ===== 侧边栏变体 ===== -->
    <template v-else>
      <el-popover
        placement="right-start"
        :width="340"
        trigger="click"
        popper-class="pp-sb-popper"
        :show-arrow="false"
        :teleported="true"
      >
        <template #reference>
          <button class="pp-sb-trigger" :title="currentProject ? (currentProject.name || '未命名项目') : '选择项目…'">
            <span class="pp-dot" :style="{ background: color || '#64748b' }"></span>
            <span v-if="!collapsed" class="pp-sb-name">{{ currentProject ? (currentProject.name || '未命名项目') : '选择项目' }}</span>
            <el-icon v-if="!collapsed" class="pp-sb-caret"><ArrowDown /></el-icon>
          </button>
        </template>

        <div class="pp-sb-panel">
          <el-input
            v-model="filterText"
            size="default"
            placeholder="搜索项目…"
            :prefix-icon="Search"
            clearable
          />
          <div class="pp-sb-groups">
            <div v-for="g in groups" :key="g.label" class="pp-sb-group" v-show="g.items.length">
              <div class="pp-sb-group-label">{{ g.label }}</div>
              <div
                v-for="p in g.items"
                :key="p.id"
                class="pp-sb-opt"
                :class="{ active: p.id === currentId }"
                @click="pick(p)"
              >
                <span class="pp-dot" :style="{ background: p.color || '#64748b' }"></span>
                <div class="pp-sb-opt-body">
                  <span class="pp-sb-opt-name">{{ p.name || '未命名项目' }}</span>
                  <span v-if="p.description" class="pp-sb-opt-desc">{{ p.description }}</span>
                </div>
                <el-tag size="small" :type="statusTagType(p.status)">{{ statusLabel(p.status) }}</el-tag>
              </div>
            </div>
            <div v-if="!groups.some((g) => g.items.length)" class="pp-sb-empty">
              <el-empty description="暂无项目" :image-size="44" />
            </div>
          </div>
          <div class="pp-sb-footer">
            <el-button size="small" type="primary" text :icon="Plus" @click="openCreate">新建项目</el-button>
            <el-button size="small" text :icon="Setting" @click="goProjects">管理项目</el-button>
          </div>
        </div>
      </el-popover>
    </template>

    <!-- 新建对话框 -->
    <el-dialog v-model="dlg.open" title="新建项目" width="520px" class="pp-new-dialog" :close-on-click-modal="false">
      <el-form :model="dlg.form" label-width="80px" size="default">
        <el-form-item label="项目名称" required>
          <el-input
            v-model="dlg.form.name"
            placeholder="例如：公司官网需求图谱"
            maxlength="48"
            show-word-limit
            @input="onNameInput"
          >
            <template #append>
              <el-button :loading="aiLoading" @click="aiRecommend" :disabled="!dlg.form.name.trim()">
                <el-icon><MagicStick /></el-icon> AI 推荐
              </el-button>
            </template>
          </el-input>
        </el-form-item>

        <!-- AI 推荐结果预览 -->
        <div v-if="aiSuggestion" class="ai-suggestion">
          <div class="ai-sug-header">
            <el-icon style="color:#8b5cf6"><MagicStick /></el-icon>
            <span>AI 推荐配置</span>
            <el-button size="small" text @click="applyAiSuggestion">应用全部</el-button>
          </div>
          <div class="ai-sug-body">
            <div class="ai-sug-item">
              <span class="ai-sug-label">项目类型</span>
              <span class="ai-sug-value">{{ aiSuggestion.categoryLabel }}</span>
            </div>
            <div class="ai-sug-item">
              <span class="ai-sug-label">起始阶段</span>
              <span class="ai-sug-value">{{ aiSuggestion.phaseLabel }}</span>
            </div>
            <div class="ai-sug-item">
              <span class="ai-sug-label">技术栈</span>
              <span class="ai-sug-value">{{ aiSuggestion.techStack }}</span>
            </div>
          </div>
        </div>

        <el-form-item label="项目类型">
          <el-select v-model="dlg.form.category" style="width: 100%">
            <el-option v-for="c in typeOptions" :key="c.key" :label="c.label" :value="c.key">
              <span class="pp-type-opt"><el-icon :style="{color:c.color}"><component :is="c.icon"/></el-icon> {{ c.label }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item label="当前阶段">
          <el-radio-group v-model="dlg.form.phase">
            <el-radio-button value="requirement">需求</el-radio-button>
            <el-radio-button value="architecture">架构</el-radio-button>
            <el-radio-button value="develop">开发</el-radio-button>
            <el-radio-button value="release">发布</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="状态">
          <el-radio-group v-model="dlg.form.status">
            <el-radio-button value="active">进行中</el-radio-button>
            <el-radio-button value="planning">规划中</el-radio-button>
            <el-radio-button value="done">已完成</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="技术栈">
          <el-input v-model="dlg.form.tech_stack" placeholder="如：Vue3+SpringBoot+MySQL+Redis" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="dlg.form.description" type="textarea" :rows="2" maxlength="200" placeholder="一句话说明项目目标…" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dlg.open = false">取消</el-button>
        <el-button type="primary" :loading="dlg.saving" @click="doCreate">创建并切换</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { Plus, Folder, FolderOpened, Cpu, MagicStick, Share, Shop, Link, Connection, Monitor, User, DataBoard, DataAnalysis, Setting, Aim, ArrowDown, Search } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useProject } from '@/composables/projectContext.js'
import { getProjectTypes } from '@/api'

const props = defineProps({
  variant: { type: String, default: 'top' },   // 'top' 横向（顶栏/快捷栏） | 'sidebar' 侧边栏
  collapsed: { type: Boolean, default: false } // 侧边栏折叠态（仅 sidebar 变体生效）
})

const router = useRouter()
const { currentProject, projectList, listLoading, setCurrentProject, createAndSelect } = useProject()

// —— 侧边栏变体：搜索 + 分组 ——
const filterText = ref('')
const groups = computed(() => {
  const q = (filterText.value || '').trim().toLowerCase()
  const match = (list) => list.filter((p) => !q
    || (p.name || '').toLowerCase().includes(q)
    || (p.description || '').toLowerCase().includes(q))
  return [
    { label: '进行中', items: match(activeProjects.value) },
    { label: '规划中', items: match(planningProjects.value) },
    { label: '已完成', items: match(doneProjects.value) }
  ]
})
function pick(p) { onChange(p.id) }

const currentId = computed(() => currentProject.value?.id || null)
const color = computed(() => currentProject.value?.color || null)

const dlg = ref({
  open: false,
  saving: false,
  form: { name: '', category: 'platform', status: 'active', phase: 'requirement', description: '', tech_stack: '' }
})
const typeOptions = ref([])
const aiLoading = ref(false)
const aiSuggestion = ref(null)

// AI 智能推荐：根据项目名推断类型/阶段/技术栈
const AI_RULES = [
  { pattern: /知识|图谱|graph|知识图谱|关联/i, category: 'knowledge', phase: 'architecture', tech: 'Neo4j+Elasticsearch+Python+Vue3' },
  { pattern: /电商|商城|购物|订单|商品/i, category: 'business', phase: 'develop', tech: 'Vue3+SpringBoot+MySQL+Redis+RabbitMQ' },
  { pattern: /官网|门户|展示|企业站/i, category: 'platform', phase: 'requirement', tech: 'Vue3+Nuxt+Node.js+MySQL' },
  { pattern: /AI|智能|大模型|llm|gpt|对话/i, category: 'ai', phase: 'develop', tech: 'Python+LangChain+FastAPI+Vue3+向量数据库' },
  { pattern: /数据|分析|bi|报表|可视化/i, category: 'data', phase: 'architecture', tech: 'Python+Pandas+ECharts+MySQL+ClickHouse' },
  { pattern: /运维|监控|devops|部署/i, category: 'devops', phase: 'release', tech: 'K8s+Prometheus+Grafana+Jenkins+Docker' },
  { pattern: /社交|社区|论坛|用户/i, category: 'social', phase: 'requirement', tech: 'Vue3+Node.js+MongoDB+Redis+WebSocket' },
  { pattern: /教育|学习|课程|培训/i, category: 'education', phase: 'requirement', tech: 'Vue3+SpringBoot+MySQL+Redis+WebRTC' },
  { pattern: /金融|银行|支付|交易/i, category: 'finance', phase: 'architecture', tech: 'Java+SpringCloud+MySQL+Redis+Kafka' },
  { pattern: /医疗|医院|健康/i, category: 'medical', phase: 'requirement', tech: 'Vue3+SpringBoot+MySQL+HIS集成' }
]

function onNameInput() {
  aiSuggestion.value = null
}

async function aiRecommend() {
  const name = dlg.value.form.name.trim()
  if (!name) return
  aiLoading.value = true
  // 模拟 AI 推理延迟
  await new Promise(r => setTimeout(r, 600))
  // 规则匹配（前端模拟 AI 推荐）
  let matched = AI_RULES.find(r => r.pattern.test(name))
  if (!matched) matched = { category: 'platform', phase: 'requirement', tech: 'Vue3+SpringBoot+MySQL+Redis' }
  const catLabel = typeOptions.value.find(c => c.key === matched.category)?.label || matched.category
  const phaseLabels = { requirement: '需求阶段', architecture: '架构阶段', develop: '开发阶段', release: '发布阶段' }
  aiSuggestion.value = {
    category: matched.category,
    categoryLabel: catLabel,
    phase: matched.phase,
    phaseLabel: phaseLabels[matched.phase],
    techStack: matched.tech
  }
  aiLoading.value = false
}

function applyAiSuggestion() {
  if (!aiSuggestion.value) return
  dlg.value.form.category = aiSuggestion.value.category
  dlg.value.form.phase = aiSuggestion.value.phase
  dlg.value.form.tech_stack = aiSuggestion.value.techStack
  aiSuggestion.value = null
}

// 按状态分组
const activeProjects = computed(() => (projectList.value || []).filter(p => p.status === 'active').sort((a,b) => (a.name||'').localeCompare(b.name||'')))
const planningProjects = computed(() => (projectList.value || []).filter(p => p.status === 'planning').sort((a,b) => (a.name||'').localeCompare(b.name||'')))
const doneProjects = computed(() => (projectList.value || []).filter(p => ['done','completed','archived'].includes(p.status)).sort((a,b) => (a.name||'').localeCompare(b.name||'')))

function statusLabel(s) {
  return { active: '进行中', planning: '规划中', done: '已完成', archived: '已归档' }[s] || s || '进行中'
}
function statusTagType(s) {
  return { active: 'success', planning: 'warning', done: 'info', archived: '' }[s] || 'info'
}

async function onChange(id) {
  const p = (projectList.value || []).find(x => x.id === id)
  await setCurrentProject(id)
  if (p) {
    ElMessage.success(`已切换到项目「${p.name || '未命名项目'}」`)
  }
}
function onVisible(v) { if (v) { /* 打开时无需刷新，全局 provide 已预加载 */ } }
function openCreate() { dlg.value = { open: true, saving: false, form: { name: '', category: typeOptions.value[0]?.key || 'platform', status: 'active', description: '' } } }
function goProjects() { router.push('/projects') }
async function doCreate() {
  const f = dlg.value.form
  if (!f.name?.trim()) return ElMessage.warning('请输入项目名称')
  dlg.value.saving = true
  try {
    await createAndSelect(f)
    ElMessage.success(`已创建并切换到项目「${f.name}」`)
    dlg.value.open = false
  } finally {
    dlg.value.saving = false
  }
}

onMounted(async () => {
  try {
    const r = await getProjectTypes()
    typeOptions.value = r?.categories || [
      { key: 'platform', label: '平台系统', color: '#4f46e5', icon: 'DataBoard' },
      { key: 'custom', label: '自定义应用', color: '#0d9488', icon: 'FolderOpened' },
      { key: 'algorithm', label: '算法联盟', color: '#d97706', icon: 'DataAnalysis' },
      { key: 'architecture', label: '架构开发', color: '#6366f1', icon: 'Cpu' },
      { key: 'graph', label: '图谱应用', color: '#06b6d4', icon: 'Share' },
      { key: 'automation', label: '自动化', color: '#f97316', icon: 'MagicStick' }
    ]
  } catch {
    typeOptions.value = [
      { key: 'platform', label: '平台系统', color: '#4f46e5', icon: Folder },
      { key: 'custom', label: '自定义应用', color: '#0d9488', icon: FolderOpened },
      { key: 'algorithm', label: '算法联盟', color: '#d97706', icon: DataAnalysis },
      { key: 'architecture', label: '架构开发', color: '#6366f1', icon: Cpu },
      { key: 'graph', label: '图谱应用', color: '#06b6d4', icon: Share },
      { key: 'automation', label: '自动化', color: '#f97316', icon: MagicStick }
    ]
  }
})
</script>

<style scoped>
.pp-wrap {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 36px;
  flex-shrink: 0;
}
.pp-select {
  height: 36px;
  width: 248px;
}
:deep(.pp-select .el-select__wrapper) {
  height: 36px;
  min-height: 36px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  transition: all 150ms ease;
  box-shadow: 0 1px 2px -1px rgba(15, 23, 42, 0.04);
}
:deep(.pp-select:hover .el-select__wrapper),
:deep(.pp-select.is-focused .el-select__wrapper) {
  border-color: #6366f1;
  box-shadow: 0 0 0 3px rgba(99, 102, 241, 0.12);
}
:deep(.pp-select .el-select__placeholder),
:deep(.pp-select .el-select__selected-item) {
  font-size: 13px;
  font-weight: 500;
}
.pp-dot {
  flex: 0 0 auto;
  width: 10px; height: 10px; border-radius: 3px;
  box-shadow: 0 0 0 1px rgba(255,255,255,0.9);
}

.pp-new {
  width: 36px !important;
  height: 36px !important;
  background: var(--bg-card);
  border: 1px dashed #cbd5e1 !important;
  color: #64748b;
  border-radius: 10px !important;
  transition: all 150ms ease;
}
.pp-new:hover {
  border-style: solid !important;
  border-color: #6366f1 !important;
  color: #4f46e5 !important;
  background: var(--accent-dim) !important;
}

/* 选项 */
.pp-opt {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  line-height: 1.2;
}
.pp-opt-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.pp-opt-name { font-weight: 500; font-size: 13px; color: var(--text-primary); }
.pp-opt-desc { font-size: 11px; color: #94a3b8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 200px; }

.pp-manage {
  width: 36px !important;
  height: 36px !important;
  background: var(--bg-card);
  border: 1px solid var(--border) !important;
  color: #64748b;
  border-radius: 10px !important;
  transition: all 150ms ease;
}
.pp-manage:hover {
  border-color: #6366f1 !important;
  color: #4f46e5 !important;
  background: var(--accent-dim) !important;
}

.pp-empty { padding: 10px 4px; display: flex; flex-direction: column; align-items: center; gap: 8px; }

.pp-type-opt { display: inline-flex; align-items: center; gap: 8px; }

/* ===== 侧边栏变体：触发器（位于 App 侧边栏内） ===== */
.pp-sb-mode {
  display: block;
  width: 100%;
  height: auto;
  padding: 6px 12px 4px;
  flex-shrink: 0;
}
.pp-sb-mode .pp-sb-trigger {
  width: 100%;
  height: 40px;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  border-radius: 11px;
  cursor: pointer;
  text-align: left;
  background: rgba(255, 255, 255, 0.045);
  border: 1px solid rgba(255, 255, 255, 0.09);
  color: #e2e8f0;
  transition: all 150ms ease;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
}
.pp-sb-mode .pp-sb-trigger:hover {
  background: rgba(255, 255, 255, 0.09);
  border-color: rgba(255, 255, 255, 0.18);
}
.pp-sb-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pp-sb-caret { color: #64748b; font-size: 12px; flex-shrink: 0; }
.pp-sb-mode.pp-sb-collapsed .pp-sb-trigger {
  justify-content: center;
  padding: 0;
  width: 40px;
  margin: 0 auto;
}

/* 对话框 */
:deep(.pp-new-dialog .el-dialog) {
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 28px 64px -20px rgba(15,23,42,0.22), 0 8px 20px -10px rgba(99,102,241,0.22);
}

/* AI 推荐卡片 */
.ai-suggestion {
  margin-bottom: 18px;
  background: linear-gradient(135deg, rgba(139, 92, 246, 0.08), rgba(99, 102, 241, 0.06));
  border: 1px solid rgba(139, 92, 246, 0.2);
  border-radius: 12px;
  padding: 12px 14px;
}
.ai-sug-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 700;
  font-size: 13px;
  color: #7c3aed;
  margin-bottom: 10px;
}
.ai-sug-header .el-button {
  margin-left: auto;
  font-size: 12px;
}
.ai-sug-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ai-sug-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
}
.ai-sug-label {
  color: var(--text-3);
}
.ai-sug-value {
  font-weight: 600;
  color: var(--text-1);
}
</style>

<style>
/* 侧边栏项目选择器弹层：el-popover teleport 到 body，需全局样式 */
.pp-sb-popper.el-popover {
  padding: 10px;
  border-radius: 14px;
  box-shadow: 0 28px 64px -20px rgba(15, 23, 42, 0.25), 0 8px 20px -10px rgba(99, 102, 241, 0.18);
}
.pp-sb-panel { display: flex; flex-direction: column; gap: 8px; }
.pp-sb-groups {
  max-height: 320px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 2px;
}
.pp-sb-group-label {
  font-size: 11px;
  color: #94a3b8;
  letter-spacing: 0.06em;
  padding: 6px 6px 2px;
  font-weight: 600;
}
.pp-sb-opt {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 9px;
  cursor: pointer;
  transition: background 120ms ease;
  line-height: 1.2;
}
.pp-sb-opt:hover { background: var(--bg-tertiary); }
.pp-sb-opt.active { background: var(--accent-dim); }
.pp-sb-opt-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.pp-sb-opt-name { font-weight: 500; font-size: 13px; color: var(--text-primary); }
.pp-sb-opt-desc { font-size: 11px; color: #94a3b8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.pp-sb-empty { padding: 8px 0; }
.pp-sb-footer {
  display: flex;
  justify-content: flex-end;
  gap: 4px;
  border-top: 1px solid var(--border-ghost);
  padding-top: 8px;
  margin-top: 4px;
}
</style>
