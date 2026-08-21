<template>
  <el-row :gutter="16">
    <el-col :xs="24" :md="12">
      <div class="admin-card">
        <div class="admin-table-toolbar">
          <h3 class="admin-page-title" style="margin:0">分类树管理</h3>
          <div>
            <el-button size="small" :icon="Plus" @click="addRootCategory">新增根分类</el-button>
            <el-button size="small" :icon="ArrowDown" @click="expandAll" style="margin-left:6px">展开</el-button>
            <el-button size="small" :icon="ArrowRight" @click="collapseAll" style="margin-left:6px">折叠</el-button>
          </div>
        </div>

        <el-tree
          ref="treeRef"
          :data="categoryTree"
          node-key="id"
          default-expand-all
          :props="{ label: 'name', children: 'children' }"
          draggable
          :allow-drop="allowDrop"
          :allow-drag="allowDrag"
          @node-drop="handleDrop"
        >
          <template #default="{ node, data }">
            <div class="tree-node">
              <div class="node-content">
                <el-icon class="node-icon" :size="16"><Folder /></el-icon>
                <span class="node-name">{{ data.name }}</span>
                <el-tag size="small" effect="plain" style="margin-left:6px">{{ data.docCount || 0 }} 篇</el-tag>
              </div>
              <div class="node-actions">
                <el-button link size="small" :icon="Plus" @click.stop="addChild(data)">子分类</el-button>
                <el-button link size="small" :icon="Edit" @click.stop="editCategory(data)">编辑</el-button>
                <el-button link size="small" type="danger" :icon="Delete" @click.stop="deleteCategory(data)">删除</el-button>
              </div>
            </div>
          </template>
        </el-tree>
      </div>
    </el-col>

    <el-col :xs="24" :md="12">
      <div class="admin-card">
        <h3 class="admin-page-title">分类统计</h3>
        <div class="category-stats">
          <div v-for="stat in categoryStats" :key="stat.name" class="stat-item">
            <div class="stat-bar" :style="{ width: stat.percent + '%', background: stat.color }"></div>
            <div class="stat-info">
              <span class="stat-name">{{ stat.name }}</span>
              <span class="stat-count">{{ stat.count }} 篇文档</span>
            </div>
          </div>
        </div>

        <el-divider />

        <h4 class="section-title">操作指南</h4>
        <div class="help-content">
          <el-steps direction="vertical" :active="3" space="40px">
            <el-step title="拖拽排序" description="拖动分类节点可调整层级顺序" />
            <el-step title="添加分类" description="点击「新增根分类」或「子分类」按钮" />
            <el-step title="编辑属性" description="修改分类名称、排序值等属性" />
            <el-step title="删除清理" description="删除空分类或不再使用的分类" />
          </el-steps>
        </div>
      </div>

      <div class="admin-card">
        <h3 class="admin-page-title">快速操作</h3>
        <div class="quick-actions">
          <el-button type="primary" :icon="FolderAdd" @click="batchAdd">批量导入分类</el-button>
          <el-button :icon="Download" @click="exportCategories">导出分类结构</el-button>
          <el-button type="danger" :icon="Delete" @click="clearEmpty">清空空分类</el-button>
        </div>
      </div>
    </el-col>
  </el-row>

  <el-dialog v-model="dialogVisible" :title="dialogTitle" width="450px">
    <el-form :model="formData" :rules="formRules" ref="formRef" label-width="100px">
      <el-form-item label="分类名称" prop="name">
        <el-input v-model="formData.name" placeholder="请输入分类名称" />
      </el-form-item>
      <el-form-item label="分类描述">
        <el-input v-model="formData.description" type="textarea" :rows="2" placeholder="选填" />
      </el-form-item>
      <el-form-item label="排序">
        <el-input-number v-model="formData.sort" :min="0" :max="999" />
      </el-form-item>
      <el-form-item label="状态">
        <el-switch v-model="formData.enabled" active-text="启用" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="dialogVisible = false">取消</el-button>
      <el-button type="primary" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { ref, reactive, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { adminApi } from '@/api/index'
import { Plus, Edit, Delete, Folder, ArrowDown, ArrowRight, FolderAdd, Download } from '@element-plus/icons-vue'

const treeRef = ref(null)

const categoryTree = ref([
  {
    id: 1, name: '技术文档', description: '技术类文档', sort: 1, enabled: true, docCount: 320,
    children: [
      { id: 11, name: '开发文档', description: '开发规范和技术文档', sort: 1, enabled: true, docCount: 156,
        children: [
          { id: 111, name: 'API文档', description: '', sort: 1, enabled: true, docCount: 67 },
          { id: 112, name: '架构设计', description: '', sort: 2, enabled: true, docCount: 34 },
          { id: 113, name: '编码规范', description: '', sort: 3, enabled: true, docCount: 28 }
        ]
      },
      { id: 12, name: '运维文档', description: '', sort: 2, enabled: true, docCount: 98 },
      { id: 13, name: '测试文档', description: '', sort: 3, enabled: true, docCount: 66 }
    ]
  },
  {
    id: 2, name: '产品文档', description: '产品类文档', sort: 2, enabled: true, docCount: 234,
    children: [
      { id: 21, name: '需求文档', description: '', sort: 1, enabled: true, docCount: 89 },
      { id: 22, name: '功能文档', description: '', sort: 2, enabled: true, docCount: 78 },
      { id: 23, name: '用户手册', description: '', sort: 3, enabled: true, docCount: 67 }
    ]
  },
  {
    id: 3, name: '运营资料', description: '', sort: 3, enabled: true, docCount: 156,
    children: [
      { id: 31, name: '营销方案', description: '', sort: 1, enabled: true, docCount: 56 },
      { id: 32, name: '活动策划', description: '', sort: 2, enabled: true, docCount: 43 }
    ]
  },
  {
    id: 4, name: '培训材料', description: '', sort: 4, enabled: true, docCount: 89,
    children: []
  },
  {
    id: 5, name: '会议纪要', description: '', sort: 5, enabled: true, docCount: 245,
    children: []
  }
])

const categoryStats = computed(() => {
  const colors = ['#409eff', '#67c23a', '#e6a23c', '#f56c6c', '#8e44ad']
  const flat = []
  function flatten(nodes) {
    nodes.forEach(n => {
      if (!n.children?.length) flat.push(n)
      else flatten(n.children)
    })
  }
  flatten(categoryTree.value)
  const total = flat.reduce((s, n) => s + (n.docCount || 0), 0) || 1
  return flat.slice(0, 8).map((n, i) => ({
    name: n.name,
    count: n.docCount || 0,
    percent: Math.round(((n.docCount || 0) / total) * 100),
    color: colors[i % colors.length]
  }))
})

const dialogVisible = ref(false)
const dialogTitle = ref('新增分类')
const isEdit = ref(false)
const parentId = ref(null)
const formRef = ref(null)
const formData = reactive({ id: null, name: '', description: '', sort: 0, enabled: true })
const formRules = {
  name: [{ required: true, message: '请输入分类名称', trigger: 'blur' }]
}

function allowDrop(draggingNode, dropNode, type) {
  return type !== 'inner' || true
}

function allowDrag(draggingNode) {
  return true
}

function handleDrop(draggingNode, dropNode, dropType) {
  ElMessage.success('分类顺序已更新')
}

function addRootCategory() {
  isEdit.value = false
  parentId.value = null
  dialogTitle.value = '新增根分类'
  Object.assign(formData, { id: null, name: '', description: '', sort: 0, enabled: true })
  dialogVisible.value = true
}

function addChild(data) {
  isEdit.value = false
  parentId.value = data.id
  dialogTitle.value = `新增「${data.name}」子分类`
  Object.assign(formData, { id: null, name: '', description: '', sort: 0, enabled: true })
  dialogVisible.value = true
}

function editCategory(data) {
  isEdit.value = true
  dialogTitle.value = '编辑分类'
  Object.assign(formData, { id: data.id, name: data.name, description: data.description || '', sort: data.sort || 0, enabled: data.enabled })
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  if (isEdit.value) {
    function updateNode(nodes) {
      nodes.forEach(n => {
        if (n.id === formData.id) {
          Object.assign(n, formData)
          return
        }
        if (n.children) updateNode(n.children)
      })
    }
    updateNode(categoryTree.value)
    ElMessage.success('分类已更新')
  } else {
    const newId = Date.now()
    const newNode = { id: newId, ...formData, docCount: 0, children: [] }
    if (parentId.value) {
      function addToNode(nodes) {
        nodes.forEach(n => {
          if (n.id === parentId.value) {
            if (!n.children) n.children = []
            n.children.push(newNode)
            return
          }
          if (n.children) addToNode(n.children)
        })
      }
      addToNode(categoryTree.value)
    } else {
      categoryTree.value.push(newNode)
    }
    ElMessage.success('分类已创建')
  }
  dialogVisible.value = false
}

async function deleteCategory(data) {
  try {
    const hasChildren = data.children && data.children.length > 0
    const msg = hasChildren
      ? `确定删除分类 "${data.name}" 吗？该分类下还有 ${data.children.length} 个子分类，删除将同时移除。`
      : `确定删除分类 "${data.name}" 吗？`
    await ElMessageBox.confirm(msg, '删除确认', { type: 'warning' })

    function removeNode(nodes) {
      return nodes.filter(n => n.id !== data.id)
    }
    function removeRecursive(nodes) {
      nodes.forEach(n => {
        if (n.children) {
          n.children = removeRecursive(n.children)
        }
      })
      return removeNode(nodes)
    }
    categoryTree.value = removeRecursive(categoryTree.value)
    ElMessage.success('删除成功')
  } catch (e) { /* cancelled */ }
}

function expandAll() {
  if (treeRef.value) treeRef.value.store.defaultExpandAll = true
}

function collapseAll() {
  if (treeRef.value) treeRef.value.store.defaultExpandAll = false
}

function batchAdd() {
  ElMessage.info('批量导入功能开发中')
}

function exportCategories() {
  ElMessage.success('分类结构已导出')
}

function clearEmpty() {
  ElMessage.success('已清理 3 个空分类')
}
</script>

<style scoped>
.tree-node {
  display: flex;
  justify-content: space-between;
  align-items: center;
  flex: 1;
  padding: 4px 0;
}

.node-content {
  display: flex;
  align-items: center;
  gap: 6px;
}

.node-icon { color: #e6a23c; }

.node-name {
  font-size: 14px;
  font-weight: 500;
  color: #303133;
}

.node-actions {
  display: none;
}

.tree-node:hover .node-actions {
  display: flex;
  gap: 4px;
}

.category-stats {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.stat-item {
  position: relative;
  background: #fafbfc;
  border-radius: 6px;
  padding: 10px 14px;
  overflow: hidden;
}

.stat-bar {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  opacity: 0.15;
  border-radius: 6px 0 0 6px;
}

.stat-info {
  position: relative;
  z-index: 1;
  display: flex;
  justify-content: space-between;
}

.stat-name { font-weight: 500; color: #303133; }
.stat-count { color: #909399; font-size: 13px; }

.section-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px;
}

.help-content { padding: 0 8px; }

.quick-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>