<template>
  <div>
    <div class="admin-card">
      <div class="admin-table-toolbar">
        <div class="toolbar-search">
          <el-input
            v-model="searchText"
            placeholder="搜索知识库名称"
            :prefix-icon="Search"
            style="width: 260px"
            clearable
            @keyup.enter="handleSearch"
          />
          <el-select v-model="filterCategory" placeholder="全部分类" clearable style="width: 160px; margin-left: 10px">
            <el-option v-for="c in categoryOptions" :key="c" :label="c" :value="c" />
          </el-select>
          <el-select v-model="filterAccess" placeholder="访问级别" clearable style="width: 140px; margin-left: 10px">
            <el-option label="公开" value="public" />
            <el-option label="组织内" value="organization" />
            <el-option label="私有" value="private" />
          </el-select>
          <el-button type="primary" :icon="Search" @click="handleSearch" style="margin-left: 10px">搜索</el-button>
          <el-button :icon="Refresh" @click="resetSearch">重置</el-button>
        </div>
        <el-button type="primary" :icon="Plus" @click="openCreateDialog">新建知识库</el-button>
      </div>

      <el-row :gutter="16" class="kb-grid">
        <el-col
          v-for="kb in filteredKnowledgeBases"
          :key="kb.id"
          :xs="24" :sm="12" :md="8" :lg="6"
        >
          <div class="kb-card" @click="$router.push('/knowledge/permissions')">
            <div class="kb-header">
              <div class="kb-icon" :style="{ background: getKbColor(kb.category) }">
                <el-icon :size="24"><Collection /></el-icon>
              </div>
              <el-dropdown trigger="click" @click.stop>
                <el-icon class="kb-more"><MoreFilled /></el-icon>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item :icon="Edit" @click="openEditDialog(kb)">编辑</el-dropdown-item>
                    <el-dropdown-item :icon="Lock" @click="$router.push('/knowledge/permissions')">权限配置</el-dropdown-item>
                    <el-dropdown-item :icon="Folder" @click="$router.push('/knowledge/categories')">分类管理</el-dropdown-item>
                    <el-dropdown-item divided :icon="Delete" @click="handleDelete(kb)">删除</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
            <div class="kb-body">
              <h4 class="kb-name" :title="kb.name">{{ kb.name }}</h4>
              <p class="kb-desc" :title="kb.description">{{ kb.description || '暂无描述' }}</p>
            </div>
            <div class="kb-meta">
              <div class="meta-item">
                <el-icon><Document /></el-icon>
                <span>{{ kb.docCount }} 文档</span>
              </div>
              <div class="meta-item">
                <el-icon><User /></el-icon>
                <span>{{ kb.owner }}</span>
              </div>
              <div class="meta-item">
                <el-icon><View /></el-icon>
                <span>{{ kb.viewCount.toLocaleString() }}</span>
              </div>
            </div>
            <div class="kb-footer">
              <el-tag :type="getAccessTagType(kb.accessLevel)" size="small" effect="light">
                {{ getAccessLabel(kb.accessLevel) }}
              </el-tag>
              <span class="kb-updated">{{ kb.updatedAt }}</span>
            </div>
          </div>
        </el-col>
      </el-row>

      <div v-if="filteredKnowledgeBases.length === 0" class="empty-state">
        <el-empty description="暂无匹配的知识库" />
      </div>

      <div class="pagination-wrapper" v-if="filteredKnowledgeBases.length > 0">
        <el-pagination
          v-model:current-page="currentPage"
          v-model:page-size="pageSize"
          :page-sizes="[12, 24, 48]"
          :total="filteredKnowledgeBases.length"
          layout="total, sizes, prev, pager, next"
          background
        />
      </div>
    </div>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="550px">
      <el-form :model="formData" :rules="formRules" ref="formRef" label-width="100px">
        <el-form-item label="名称" prop="name">
          <el-input v-model="formData.name" placeholder="请输入知识库名称" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="formData.description" type="textarea" :rows="3" placeholder="请输入描述信息" />
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="分类" prop="category">
              <el-select v-model="formData.category" placeholder="请选择分类" style="width: 100%" filterable allow-create filterable>
                <el-option v-for="c in categoryOptions" :key="c" :label="c" :value="c" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="访问级别" prop="accessLevel">
              <el-select v-model="formData.accessLevel" style="width: 100%">
                <el-option label="公开" value="public" />
                <el-option label="组织内" value="organization" />
                <el-option label="私有" value="private" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="存储类型">
              <el-select v-model="formData.storageType" style="width: 100%">
                <el-option label="本地存储" value="local" />
                <el-option label="阿里云OSS" value="oss" />
                <el-option label="AWS S3" value="s3" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="文档解析">
              <el-switch v-model="formData.autoParse" active-text="自动" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Search, Refresh, Plus, Edit, Delete, Lock, Folder } from '@element-plus/icons-vue'

const loading = ref(false)
const searchText = ref('')
const filterCategory = ref('')
const filterAccess = ref('')
const currentPage = ref(1)
const pageSize = ref(12)
const formRef = ref(null)

const categoryOptions = ref(['技术文档', '产品文档', '运营资料', '法律法规', '培训材料', '会议纪要', '其他'])

const knowledgeBases = ref([
  { id: 1, name: '产品文档库', description: '公司产品说明书、功能文档、API文档等', category: '产品文档', docCount: 156, accessLevel: 'organization', owner: '张三', viewCount: 3420, updatedAt: '2026-08-21', storageType: 'oss', autoParse: true },
  { id: 2, name: '技术手册', description: '技术规范、开发文档、架构设计文档', category: '技术文档', docCount: 234, accessLevel: 'private', owner: '李四', viewCount: 2180, updatedAt: '2026-08-20', storageType: 's3', autoParse: true },
  { id: 3, name: '运营知识库', description: '运营策略、活动策划、推广素材', category: '运营资料', docCount: 89, accessLevel: 'organization', owner: '王五', viewCount: 1560, updatedAt: '2026-08-19', storageType: 'local', autoParse: false },
  { id: 4, name: '公司规章制度', description: '公司内部规章制度、流程规范', category: '法律法规', docCount: 67, accessLevel: 'public', owner: 'admin', viewCount: 5420, updatedAt: '2026-08-18', storageType: 'local', autoParse: true },
  { id: 5, name: '培训资料', description: '新员工培训、技能提升材料', category: '培训材料', docCount: 123, accessLevel: 'organization', owner: '赵六', viewCount: 890, updatedAt: '2026-08-17', storageType: 'oss', autoParse: true },
  { id: 6, name: '会议纪要归档', description: '各类重要会议纪要归档', category: '会议纪要', docCount: 312, accessLevel: 'private', owner: 'admin', viewCount: 670, updatedAt: '2026-08-21', storageType: 's3', autoParse: false },
  { id: 7, name: '客户案例库', description: '成功案例、解决方案文档', category: '产品文档', docCount: 45, accessLevel: 'public', owner: '钱七', viewCount: 2340, updatedAt: '2026-08-16', storageType: 'oss', autoParse: true },
  { id: 8, name: 'FAQ资料库', description: '常见问题解答、故障排查', category: '技术文档', docCount: 267, accessLevel: 'public', owner: '孙八', viewCount: 4560, updatedAt: '2026-08-20', storageType: 'local', autoParse: true }
])

const filteredKnowledgeBases = computed(() => {
  return knowledgeBases.value.filter(kb => {
    const matchSearch = !searchText.value || kb.name.includes(searchText.value)
    const matchCategory = !filterCategory.value || kb.category === filterCategory.value
    const matchAccess = !filterAccess.value || kb.accessLevel === filterAccess.value
    return matchSearch && matchCategory && matchAccess
  })
})

const dialogVisible = ref(false)
const dialogTitle = ref('新建知识库')
const isEdit = ref(false)
const formData = reactive({ id: null, name: '', description: '', category: '', accessLevel: 'organization', storageType: 'local', autoParse: true })
const formRules = {
  name: [{ required: true, message: '请输入知识库名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择分类', trigger: 'change' }],
  accessLevel: [{ required: true, message: '请选择访问级别', trigger: 'change' }]
}

function getKbColor(category) {
  const colors = {
    '技术文档': 'linear-gradient(135deg, #409eff, #66b1ff)',
    '产品文档': 'linear-gradient(135deg, #67c23a, #95d475)',
    '运营资料': 'linear-gradient(135deg, #e6a23c, #f0c78a)',
    '法律法规': 'linear-gradient(135deg, #f56c6c, #f89898)',
    '培训材料': 'linear-gradient(135deg, #8e44ad, #bb6bd9)',
    '会议纪要': 'linear-gradient(135deg, #16a085, #48c9b0)',
    '其他': 'linear-gradient(135deg, #909399, #b1b3b8)'
  }
  return colors[category] || colors['其他']
}

function getAccessLabel(level) {
  return { public: '公开', organization: '组织内', private: '私有' }[level] || level
}

function getAccessTagType(level) {
  return { public: 'success', organization: 'warning', private: 'info' }[level] || ''
}

function handleSearch() { currentPage.value = 1 }
function resetSearch() {
  searchText.value = ''
  filterCategory.value = ''
  filterAccess.value = ''
  currentPage.value = 1
}

function openCreateDialog() {
  isEdit.value = false
  dialogTitle.value = '新建知识库'
  Object.assign(formData, { id: null, name: '', description: '', category: '', accessLevel: 'organization', storageType: 'local', autoParse: true })
  dialogVisible.value = true
}

function openEditDialog(kb) {
  isEdit.value = true
  dialogTitle.value = '编辑知识库'
  Object.assign(formData, kb)
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  try {
    if (isEdit.value) {
      await adminApi.updateKnowledgeBase(formData.id, formData)
      const idx = knowledgeBases.value.findIndex(k => k.id === formData.id)
      if (idx > -1) knowledgeBases.value[idx] = { ...knowledgeBases.value[idx], ...formData }
      ElMessage.success('知识库更新成功')
    } else {
      const newId = Math.max(...knowledgeBases.value.map(k => k.id)) + 1
      knowledgeBases.value.push({
        id: newId, ...formData,
        docCount: 0,
        owner: '当前用户',
        viewCount: 0,
        updatedAt: new Date().toISOString().split('T')[0]
      })
      ElMessage.success('知识库创建成功')
    }
    dialogVisible.value = false
  } catch (e) {
    if (e.response?.status === 400 || e.code === 'ERR_BAD_REQUEST') {
      ElMessage.success(isEdit.value ? '更新成功（模拟）' : '创建成功（模拟）')
      dialogVisible.value = false
    }
  }
}

async function handleDelete(kb) {
  try {
    await ElMessageBox.confirm(`确定删除知识库 "${kb}" 吗？此操作将删除所有关联文档。`, '删除确认', { type: 'warning' })
    try {
      await adminApi.deleteKnowledgeBase(kb.id)
    } catch (e) { /* mock */ }
    knowledgeBases.value = knowledgeBases.value.filter(k => k.id !== kb.id)
    ElMessage.success('删除成功')
  } catch (e) { /* cancelled */ }
}

onMounted(async () => {
  loading.value = true
  try {
    const data = await adminApi.getKnowledgeBases()
    if (data?.data) knowledgeBases.value = data.data
  } catch (e) { /* use mock data */ }
  loading.value = false
})
</script>

<style scoped>
.kb-grid { margin-bottom: 16px; }

.kb-card {
  background: #fff;
  border: 1px solid #ebeef5;
  border-radius: 10px;
  padding: 16px;
  margin-bottom: 16px;
  cursor: pointer;
  transition: all 0.3s;
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 220px;
}

.kb-card:hover {
  border-color: #409eff;
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.15);
  transform: translateY(-2px);
}

.kb-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.kb-icon {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  color: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
}

.kb-more {
  cursor: pointer;
  color: #c0c4cc;
  font-size: 20px;
  padding: 4px;
  border-radius: 4px;
  transition: color 0.2s;
}

.kb-more:hover { color: #409eff; }

.kb-body { flex: 1; }

.kb-name {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.kb-desc {
  font-size: 13px;
  color: #909399;
  margin: 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.kb-meta {
  display: flex;
  gap: 12px;
  font-size: 12px;
  color: #606266;
}

.meta-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.kb-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 8px;
  border-top: 1px solid #f2f3f5;
}

.kb-updated {
  font-size: 12px;
  color: #c0c4cc;
}

.empty-state {
  padding: 40px;
  text-align: center;
}

.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>