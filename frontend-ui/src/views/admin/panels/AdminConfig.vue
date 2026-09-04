<template>
  <div class="adm-config">
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <el-input
            v-model="searchForm.name"
            placeholder="参数名称"
            clearable
            style="width: 200px"
            :prefix-icon="Search"
            @keyup.enter="handleSearch"
          />
          <el-input
            v-model="searchForm.key"
            placeholder="参数键名"
            clearable
            style="width: 220px"
            @keyup.enter="handleSearch"
          >
            <template #prefix><span class="mono">key:</span></template>
          </el-input>
          <el-button type="primary" :icon="Search" @click="handleSearch">搜索</el-button>
          <el-button :icon="Refresh" @click="resetSearch">重置</el-button>
        </div>
        <div class="toolbar-right">
          <el-button type="success" :icon="RefreshRight" :loading="refreshing" @click="handleRefreshCache">刷新缓存</el-button>
          <el-button type="primary" :icon="Plus" @click="openDialog(null)">新增参数</el-button>
        </div>
      </div>

      <el-table :data="configList" v-loading="loading" stripe style="width: 100%">
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="name" label="参数名称" min-width="160" />
        <el-table-column prop="key" label="参数键名" min-width="220">
          <template #default="{ row }">
            <span class="mono">{{ row.key }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="value" label="参数键值" min-width="200" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="mono">{{ row.value }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="isBuiltin" label="系统内置" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.isBuiltin ? 'warning' : 'info'" size="small">
              {{ row.isBuiltin ? '是' : '否' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="90" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">
              {{ row.status === 1 ? '正常' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="180" show-overflow-tooltip />
        <el-table-column prop="createdAt" label="创建时间" width="180">
          <template #default="{ row }">{{ fmtTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="160" fixed="right" align="center">
          <template #default="{ row }">
            <el-button type="primary" link size="small" :icon="Edit" @click="openDialog(row)">编辑</el-button>
            <el-button
              type="danger"
              link
              size="small"
              :icon="Delete"
              :disabled="row.isBuiltin"
              @click="handleDelete(row)"
            >删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-row">
        <el-pagination
          v-model:current-page="pagination.pageNum"
          v-model:page-size="pagination.pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="pagination.total"
          layout="total, sizes, prev, pager, next, jumper"
          background
          @size-change="handleSearch"
          @current-change="handleSearch"
        />
      </div>

      <el-empty v-if="!loading && !configList.length" description="暂无参数配置" />
    </div>

    <!-- 参数表单对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="560px"
      :close-on-click-modal="false"
    >
      <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px">
        <el-form-item label="参数名称" prop="name">
          <el-input v-model="form.name" placeholder="请输入参数名称" maxlength="100" show-word-limit />
        </el-form-item>
        <el-form-item label="参数键名" prop="key">
          <el-input
            v-model="form.key"
            placeholder="如 sys.user.initPassword"
            maxlength="100"
            :disabled="dialogMode === 'edit'"
          />
          <div class="form-tip">参数键名唯一标识，创建后不可修改</div>
        </el-form-item>
        <el-form-item label="参数键值" prop="value">
          <el-input
            v-model="form.value"
            type="textarea"
            :rows="3"
            placeholder="请输入参数键值"
            maxlength="500"
            show-word-limit
          />
        </el-form-item>
        <el-form-item label="系统内置" prop="isBuiltin">
          <el-radio-group v-model="form.isBuiltin">
            <el-radio :value="true">是</el-radio>
            <el-radio :value="false">否</el-radio>
          </el-radio-group>
          <div class="form-tip">内置参数不可删除，只能修改键值</div>
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="form.status">
            <el-radio :value="1">正常</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="form.remark" type="textarea" :rows="2" placeholder="请输入备注" maxlength="200" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Refresh, Plus, Edit, Delete, RefreshRight } from '@element-plus/icons-vue'
import { getConfigList, createConfig, updateConfig, deleteConfig, refreshConfigCache } from '@/api'

const loading = ref(false)
const submitting = ref(false)
const refreshing = ref(false)
const configList = ref([])

const searchForm = reactive({
  name: '',
  key: ''
})

const pagination = reactive({
  pageNum: 1,
  pageSize: 10,
  total: 0
})

const dialogVisible = ref(false)
const dialogMode = ref('add')
const dialogTitle = computed(() => dialogMode.value === 'add' ? '新增参数配置' : '编辑参数配置')
const formRef = ref(null)

const form = reactive({
  id: null,
  name: '',
  key: '',
  value: '',
  isBuiltin: false,
  status: 1,
  remark: ''
})

const formRules = {
  name: [{ required: true, message: '请输入参数名称', trigger: 'blur' }],
  key: [{ required: true, message: '请输入参数键名', trigger: 'blur' }],
  value: [{ required: true, message: '请输入参数键值', trigger: 'blur' }]
}

function fmtTime(t) {
  if (!t) return '-'
  try { return new Date(t).toLocaleString() } catch { return String(t) }
}

async function loadConfigs() {
  loading.value = true
  try {
    const params = {
      pageNum: pagination.pageNum,
      pageSize: pagination.pageSize,
      name: searchForm.name.trim(),
      key: searchForm.key.trim()
    }
    const data = await getConfigList(params)
    if (data && Array.isArray(data.list)) {
      configList.value = data.list
      pagination.total = data.total || 0
    } else if (Array.isArray(data)) {
      configList.value = data
      pagination.total = data.length
    } else {
      throw new Error('数据格式错误')
    }
  } catch (e) {
    ElMessage.error('参数配置加载失败: ' + (e?.message || e))
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  pagination.pageNum = 1
  loadConfigs()
}

function resetSearch() {
  searchForm.name = ''
  searchForm.key = ''
  pagination.pageNum = 1
  loadConfigs()
}

function openDialog(row) {
  dialogMode.value = row ? 'edit' : 'add'
  if (row) {
    Object.assign(form, {
      id: row.id,
      name: row.name,
      key: row.key,
      value: row.value,
      isBuiltin: !!row.isBuiltin,
      status: row.status,
      remark: row.remark || ''
    })
  } else {
    Object.assign(form, {
      id: null,
      name: '',
      key: '',
      value: '',
      isBuiltin: false,
      status: 1,
      remark: ''
    })
  }
  formRef.value?.clearValidate()
  dialogVisible.value = true
}

async function handleSubmit() {
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  submitting.value = true
  try {
    if (dialogMode.value === 'add') {
      await createConfig({ ...form })
      ElMessage.success('新增参数成功')
    } else {
      await updateConfig(form.id, { ...form })
      ElMessage.success('修改参数成功')
    }
    dialogVisible.value = false
    await loadConfigs()
  } catch (e) {
    ElMessage.error((dialogMode.value === 'add' ? '新增失败：' : '修改失败：') + e.message)
  } finally {
    submitting.value = false
  }
}

async function handleDelete(row) {
  if (row.isBuiltin) {
    ElMessage.warning('系统内置参数不可删除')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确定删除参数「${row.name}」吗？删除后不可恢复。`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteConfig(row.id)
    ElMessage.success('删除成功')
    await loadConfigs()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

async function handleRefreshCache() {
  refreshing.value = true
  try {
    await refreshConfigCache()
    ElMessage.success('缓存刷新成功')
    await loadConfigs()
  } catch (e) {
    ElMessage.error('缓存刷新失败：' + e.message)
  } finally {
    refreshing.value = false
  }
}

onMounted(loadConfigs)
</script>

<style scoped>
.toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; flex-wrap: wrap; gap: 10px; }
.toolbar-left { display: flex; gap: 8px; align-items: center; }
.toolbar-right { display: flex; gap: 8px; }
.mono {
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  color: var(--text-2);
  word-break: break-all;
}
.pagination-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 14px;
}
.form-tip { font-size: 12px; color: var(--text-3); margin-top: 4px; }
</style>
