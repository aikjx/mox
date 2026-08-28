<template>
  <div class="pp-wrap">
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
                <span class="pp-opt-name">{{ p.name }}</span>
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
                <span class="pp-opt-name">{{ p.name }}</span>
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
                <span class="pp-opt-name">{{ p.name }}</span>
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

    <!-- 新建对话框 -->
    <el-dialog v-model="dlg.open" title="新建项目" width="460px" class="pp-new-dialog" :close-on-click-modal="false">
      <el-form :model="dlg.form" label-width="80px" size="default">
        <el-form-item label="项目名称" required>
          <el-input v-model="dlg.form.name" placeholder="例如：公司官网需求图谱" maxlength="48" show-word-limit />
        </el-form-item>
        <el-form-item label="项目类型">
          <el-select v-model="dlg.form.category" style="width: 100%">
            <el-option v-for="c in typeOptions" :key="c.key" :label="c.label" :value="c.key">
              <span class="pp-type-opt"><el-icon :style="{color:c.color}"><component :is="c.icon"/></el-icon> {{ c.label }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-radio-group v-model="dlg.form.status">
            <el-radio-button value="active">进行中</el-radio-button>
            <el-radio-button value="planning">规划中</el-radio-button>
            <el-radio-button value="done">已完成</el-radio-button>
          </el-radio-group>
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
import { Plus, Folder, FolderOpened, Cpu, MagicStick, Share, Shop, Link, Connection, Monitor, User, DataBoard, DataAnalysis, Setting, Aim } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { useProject } from '@/composables/projectContext.js'
import { getProjectTypes } from '@/api'

const router = useRouter()
const { currentProject, projectList, listLoading, setCurrentProject, createAndSelect } = useProject()

const currentId = computed(() => currentProject.value?.id || null)
const color = computed(() => currentProject.value?.color || null)

const dlg = ref({
  open: false,
  saving: false,
  form: { name: '', category: 'platform', status: 'active', description: '' }
})
const typeOptions = ref([])

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
    ElMessage.success(`已切换到项目「${p.name}」`)
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
  background: #fff;
  border: 1px solid #e2e8f0;
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
  background: #fff;
  border: 1px dashed #cbd5e1 !important;
  color: #64748b;
  border-radius: 10px !important;
  transition: all 150ms ease;
}
.pp-new:hover {
  border-style: solid !important;
  border-color: #6366f1 !important;
  color: #4f46e5 !important;
  background: #eef2ff !important;
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
.pp-opt-name { font-weight: 500; font-size: 13px; color: #0f172a; }
.pp-opt-desc { font-size: 11px; color: #94a3b8; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 200px; }

.pp-manage {
  width: 36px !important;
  height: 36px !important;
  background: #fff;
  border: 1px solid #e2e8f0 !important;
  color: #64748b;
  border-radius: 10px !important;
  transition: all 150ms ease;
}
.pp-manage:hover {
  border-color: #6366f1 !important;
  color: #4f46e5 !important;
  background: #eef2ff !important;
}

.pp-empty { padding: 10px 4px; display: flex; flex-direction: column; align-items: center; gap: 8px; }

.pp-type-opt { display: inline-flex; align-items: center; gap: 8px; }

/* 对话框 */
:deep(.pp-new-dialog .el-dialog) {
  border-radius: 14px;
  overflow: hidden;
  box-shadow: 0 28px 64px -20px rgba(15,23,42,0.22), 0 8px 20px -10px rgba(99,102,241,0.22);
}
</style>
