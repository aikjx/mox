<template>
  <div class="adm-dict">
    <div class="dict-layout">
      <!-- 左侧：字典类型列表 -->
      <div class="panel card-pad dict-left">
        <div class="panel-header">
          <h3 class="section-title">字典类型</h3>
          <el-button type="primary" :icon="Plus" size="small" @click="openTypeDialog(null)">新增</el-button>
        </div>
        <div class="type-search">
          <el-input
            v-model="typeSearch"
            placeholder="搜索字典名称/类型"
            clearable
            size="small"
            :prefix-icon="Search"
            @keyup.enter="loadDictTypes"
            @clear="loadDictTypes"
          />
        </div>
        <el-table
          :data="filteredDictTypes"
          v-loading="typeLoading"
          stripe
          size="small"
          style="width: 100%"
          highlight-current-row
          @row-click="handleTypeClick"
          max-height="500"
        >
          <el-table-column prop="name" label="字典名称" min-width="120" show-overflow-tooltip />
          <el-table-column prop="type" label="字典类型" min-width="120" show-overflow-tooltip>
            <template #default="{ row }">
              <span class="mono">{{ row.type }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="status" label="状态" width="60" align="center">
            <template #default="{ row }">
              <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">
                {{ row.status === 1 ? '正常' : '停用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="100" align="center">
            <template #default="{ row }">
              <el-button type="primary" link size="small" @click.stop="openTypeDialog(row)">编辑</el-button>
              <el-button type="danger" link size="small" @click.stop="handleDeleteType(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-empty v-if="!typeLoading && !filteredDictTypes.length" description="暂无字典类型" :image-size="60" />
      </div>

      <!-- 右侧：字典数据列表 -->
      <div class="panel card-pad dict-right">
        <div class="panel-header">
          <div class="panel-title-row">
            <h3 class="section-title">字典数据</h3>
            <el-tag v-if="currentDictType" type="primary" size="small">{{ currentDictType.name }} ({{ currentDictType.type }})</el-tag>
            <span v-else class="muted">请选择左侧字典类型</span>
          </div>
          <div class="panel-actions">
            <el-button
              type="primary"
              :icon="Plus"
              size="small"
              :disabled="!currentDictType"
              @click="openDataDialog(null)"
            >新增</el-button>
            <el-button :icon="Refresh" size="small" @click="loadDictData">刷新</el-button>
          </div>
        </div>

        <el-table
          :data="dictDataList"
          v-loading="dataLoading"
          stripe
          style="width: 100%"
          row-key="id"
        >
          <el-table-column type="index" label="序号" width="60" align="center" />
          <el-table-column prop="label" label="字典标签" min-width="140" />
          <el-table-column prop="value" label="字典键值" min-width="140">
            <template #default="{ row }">
              <span class="mono">{{ row.value }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="sort" label="排序" width="80" align="center" />
          <el-table-column prop="tagType" label="样式属性" width="110" align="center">
            <template #default="{ row }">
              <el-tag :type="row.tagType || 'info'" size="small">{{ row.label }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="status" label="状态" width="80" align="center">
            <template #default="{ row }">
              <el-tag :type="row.status === 1 ? 'success' : 'danger'" size="small">
                {{ row.status === 1 ? '正常' : '停用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="remark" label="备注" min-width="160" show-overflow-tooltip />
          <el-table-column label="操作" width="150" fixed="right" align="center">
            <template #default="{ row }">
              <el-button type="primary" link size="small" :icon="Top" :disabled="row.sort <= 1" @click="handleMove(row, -1)">上移</el-button>
              <el-button type="primary" link size="small" :icon="Bottom" @click="handleMove(row, 1)">下移</el-button>
              <el-button type="primary" link size="small" :icon="Edit" @click="openDataDialog(row)">编辑</el-button>
              <el-button type="danger" link size="small" :icon="Delete" @click="handleDeleteData(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>

        <el-empty v-if="!dataLoading && currentDictType && !dictDataList.length" description="暂无字典数据" :image-size="60" />
      </div>
    </div>

    <!-- 字典类型表单对话框 -->
    <el-dialog
      v-model="typeDialogVisible"
      :title="typeDialogTitle"
      width="480px"
      :close-on-click-modal="false"
    >
      <el-form ref="typeFormRef" :model="typeForm" :rules="typeFormRules" label-width="90px">
        <el-form-item label="字典名称" prop="name">
          <el-input v-model="typeForm.name" placeholder="请输入字典名称" maxlength="50" show-word-limit />
        </el-form-item>
        <el-form-item label="字典类型" prop="type">
          <el-input
            v-model="typeForm.type"
            placeholder="如 sys_normal_disable"
            maxlength="100"
            :disabled="typeDialogMode === 'edit'"
          />
          <div class="form-tip">字典类型唯一标识，创建后不可修改</div>
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="typeForm.status">
            <el-radio :value="1">正常</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="typeForm.remark" type="textarea" :rows="2" placeholder="请输入备注" maxlength="200" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="typeDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="typeSubmitting" @click="handleSubmitType">确定</el-button>
      </template>
    </el-dialog>

    <!-- 字典数据表单对话框 -->
    <el-dialog
      v-model="dataDialogVisible"
      :title="dataDialogTitle"
      width="520px"
      :close-on-click-modal="false"
    >
      <el-form ref="dataFormRef" :model="dataForm" :rules="dataFormRules" label-width="90px">
        <el-form-item label="字典标签" prop="label">
          <el-input v-model="dataForm.label" placeholder="请输入字典标签" maxlength="100" show-word-limit />
        </el-form-item>
        <el-form-item label="字典键值" prop="value">
          <el-input v-model="dataForm.value" placeholder="请输入字典键值" maxlength="100" />
        </el-form-item>
        <el-form-item label="排序号" prop="sort">
          <el-input-number v-model="dataForm.sort" :min="0" :max="999" controls-position="right" />
        </el-form-item>
        <el-form-item label="样式属性" prop="tagType">
          <el-select v-model="dataForm.tagType" placeholder="请选择标签类型" style="width: 100%">
            <el-option label="success（成功/绿色）" value="success">
              <el-tag type="success" size="small">success</el-tag>
            </el-option>
            <el-option label="warning（警告/橙色）" value="warning">
              <el-tag type="warning" size="small">warning</el-tag>
            </el-option>
            <el-option label="info（信息/灰色）" value="info">
              <el-tag type="info" size="small">info</el-tag>
            </el-option>
            <el-option label="danger（危险/红色）" value="danger">
              <el-tag type="danger" size="small">danger</el-tag>
            </el-option>
            <el-option label="primary（主要/蓝色）" value="primary">
              <el-tag type="primary" size="small">primary</el-tag>
            </el-option>
          </el-select>
          <div class="form-tip">用于控制前端标签显示颜色</div>
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="dataForm.status">
            <el-radio :value="1">正常</el-radio>
            <el-radio :value="0">停用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="dataForm.remark" type="textarea" :rows="2" placeholder="请输入备注" maxlength="200" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dataDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="dataSubmitting" @click="handleSubmitData">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Search, Plus, Edit, Delete, Refresh, Top, Bottom } from '@element-plus/icons-vue'
import {
  getDictTypeList, createDictType, updateDictType, deleteDictType,
  getDictDataList, createDictData, updateDictData, deleteDictData
} from '@/api'

const typeLoading = ref(false)
const dataLoading = ref(false)
const typeSearch = ref('')
const dictTypes = ref([])
const currentDictType = ref(null)
const dictDataList = ref([])

const filteredDictTypes = computed(() => {
  const kw = typeSearch.value.trim().toLowerCase()
  if (!kw) return dictTypes.value
  return dictTypes.value.filter(t =>
    t.name?.toLowerCase().includes(kw) || t.type?.toLowerCase().includes(kw)
  )
})

// ===== 字典类型 =====
const typeDialogVisible = ref(false)
const typeDialogMode = ref('add')
const typeDialogTitle = computed(() => typeDialogMode.value === 'add' ? '新增字典类型' : '编辑字典类型')
const typeSubmitting = ref(false)
const typeFormRef = ref(null)
const typeForm = reactive({
  id: null,
  name: '',
  type: '',
  status: 1,
  remark: ''
})

const typeFormRules = {
  name: [{ required: true, message: '请输入字典名称', trigger: 'blur' }],
  type: [{ required: true, message: '请输入字典类型', trigger: 'blur' }]
}

// Mock 数据
const mockDictTypes = [
  { id: 1, name: '系统开关', type: 'sys_normal_disable', status: 1, remark: '系统通用启用/停用状态' },
  { id: 2, name: '用户性别', type: 'sys_user_sex', status: 1, remark: '用户性别选项' },
  { id: 3, name: '操作类型', type: 'sys_oper_type', status: 1, remark: '审计日志操作类型' },
  { id: 4, name: '登录状态', type: 'sys_login_status', status: 1, remark: '登录日志状态' },
  { id: 5, name: '数据状态', type: 'sys_data_status', status: 0, remark: '已停用测试字典' }
]

const mockDictDataMap = {
  sys_normal_disable: [
    { id: 101, dictType: 'sys_normal_disable', label: '正常', value: '1', sort: 1, tagType: 'success', status: 1, remark: '' },
    { id: 102, dictType: 'sys_normal_disable', label: '停用', value: '0', sort: 2, tagType: 'danger', status: 1, remark: '' }
  ],
  sys_user_sex: [
    { id: 201, dictType: 'sys_user_sex', label: '男', value: '1', sort: 1, tagType: 'primary', status: 1, remark: '' },
    { id: 202, dictType: 'sys_user_sex', label: '女', value: '2', sort: 2, tagType: 'danger', status: 1, remark: '' },
    { id: 203, dictType: 'sys_user_sex', label: '未知', value: '0', sort: 3, tagType: 'info', status: 1, remark: '' }
  ],
  sys_oper_type: [
    { id: 301, dictType: 'sys_oper_type', label: '新增', value: '1', sort: 1, tagType: 'success', status: 1, remark: '' },
    { id: 302, dictType: 'sys_oper_type', label: '修改', value: '2', sort: 2, tagType: 'warning', status: 1, remark: '' },
    { id: 303, dictType: 'sys_oper_type', label: '删除', value: '3', sort: 3, tagType: 'danger', status: 1, remark: '' },
    { id: 304, dictType: 'sys_oper_type', label: '查询', value: '4', sort: 4, tagType: 'info', status: 1, remark: '' },
    { id: 305, dictType: 'sys_oper_type', label: '导出', value: '5', sort: 5, tagType: 'primary', status: 1, remark: '' },
    { id: 306, dictType: 'sys_oper_type', label: '导入', value: '6', sort: 6, tagType: 'primary', status: 1, remark: '' },
    { id: 307, dictType: 'sys_oper_type', label: '其他', value: '99', sort: 99, tagType: 'info', status: 1, remark: '' }
  ],
  sys_login_status: [
    { id: 401, dictType: 'sys_login_status', label: '成功', value: '1', sort: 1, tagType: 'success', status: 1, remark: '' },
    { id: 402, dictType: 'sys_login_status', label: '失败', value: '0', sort: 2, tagType: 'danger', status: 1, remark: '' }
  ],
  sys_data_status: []
}

async function loadDictTypes() {
  typeLoading.value = true
  try {
    const data = await getDictTypeList({ keyword: typeSearch.value.trim() })
    dictTypes.value = Array.isArray(data) ? data : (Array.isArray(data?.list) ? data.list : [])
    if (!dictTypes.value.length) {
      dictTypes.value = mockDictTypes
    }
    // 自动选中第一个
    if (dictTypes.value.length && !currentDictType.value) {
      currentDictType.value = dictTypes.value[0]
      loadDictData()
    }
  } catch (e) {
    dictTypes.value = mockDictTypes
    if (!currentDictType.value && dictTypes.value.length) {
      currentDictType.value = dictTypes.value[0]
      dictDataList.value = mockDictDataMap[currentDictType.value.type] || []
    }
  } finally {
    typeLoading.value = false
  }
}

function handleTypeClick(row) {
  currentDictType.value = row
  loadDictData()
}

function openTypeDialog(row) {
  typeDialogMode.value = row ? 'edit' : 'add'
  if (row) {
    Object.assign(typeForm, {
      id: row.id,
      name: row.name,
      type: row.type,
      status: row.status,
      remark: row.remark || ''
    })
  } else {
    Object.assign(typeForm, {
      id: null,
      name: '',
      type: '',
      status: 1,
      remark: ''
    })
  }
  typeFormRef.value?.clearValidate()
  typeDialogVisible.value = true
}

async function handleSubmitType() {
  try {
    await typeFormRef.value.validate()
  } catch {
    return
  }
  typeSubmitting.value = true
  try {
    if (typeDialogMode.value === 'add') {
      await createDictType({ ...typeForm })
      ElMessage.success('新增字典类型成功')
    } else {
      await updateDictType(typeForm.id, { ...typeForm })
      ElMessage.success('修改字典类型成功')
    }
    typeDialogVisible.value = false
    await loadDictTypes()
  } catch (e) {
    ElMessage.error((typeDialogMode.value === 'add' ? '新增失败：' : '修改失败：') + e.message)
  } finally {
    typeSubmitting.value = false
  }
}

async function handleDeleteType(row) {
  try {
    await ElMessageBox.confirm(
      `确定删除字典类型「${row.name}」吗？删除后该类型下的所有字典数据也将被删除。`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteDictType(row.id)
    ElMessage.success('删除成功')
    if (currentDictType.value?.id === row.id) {
      currentDictType.value = null
      dictDataList.value = []
    }
    await loadDictTypes()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

// ===== 字典数据 =====
const dataDialogVisible = ref(false)
const dataDialogMode = ref('add')
const dataDialogTitle = computed(() => dataDialogMode.value === 'add' ? '新增字典数据' : '编辑字典数据')
const dataSubmitting = ref(false)
const dataFormRef = ref(null)
const dataForm = reactive({
  id: null,
  dictType: '',
  label: '',
  value: '',
  sort: 1,
  tagType: 'info',
  status: 1,
  remark: ''
})

const dataFormRules = {
  label: [{ required: true, message: '请输入字典标签', trigger: 'blur' }],
  value: [{ required: true, message: '请输入字典键值', trigger: 'blur' }]
}

async function loadDictData() {
  if (!currentDictType.value) return
  dataLoading.value = true
  try {
    const data = await getDictDataList({ dictType: currentDictType.value.type })
    const list = Array.isArray(data) ? data : (Array.isArray(data?.list) ? data.list : [])
    dictDataList.value = list.sort((a, b) => (a.sort ?? 0) - (b.sort ?? 0))
    if (!dictDataList.value.length) {
      dictDataList.value = mockDictDataMap[currentDictType.value.type] || []
    }
  } catch (e) {
    dictDataList.value = mockDictDataMap[currentDictType.value.type] || []
  } finally {
    dataLoading.value = false
  }
}

function openDataDialog(row) {
  dataDialogMode.value = row ? 'edit' : 'add'
  if (row) {
    Object.assign(dataForm, {
      id: row.id,
      dictType: row.dictType || currentDictType.value?.type,
      label: row.label,
      value: row.value,
      sort: row.sort ?? 1,
      tagType: row.tagType || 'info',
      status: row.status,
      remark: row.remark || ''
    })
  } else {
    const maxSort = dictDataList.value.length
      ? Math.max(...dictDataList.value.map(d => d.sort ?? 0))
      : 0
    Object.assign(dataForm, {
      id: null,
      dictType: currentDictType.value?.type,
      label: '',
      value: '',
      sort: maxSort + 1,
      tagType: 'info',
      status: 1,
      remark: ''
    })
  }
  dataFormRef.value?.clearValidate()
  dataDialogVisible.value = true
}

async function handleSubmitData() {
  try {
    await dataFormRef.value.validate()
  } catch {
    return
  }
  dataSubmitting.value = true
  try {
    const payload = { ...dataForm, dictType: currentDictType.value.type }
    if (dataDialogMode.value === 'add') {
      await createDictData(payload)
      ElMessage.success('新增字典数据成功')
    } else {
      await updateDictData(dataForm.id, payload)
      ElMessage.success('修改字典数据成功')
    }
    dataDialogVisible.value = false
    await loadDictData()
  } catch (e) {
    ElMessage.error((dataDialogMode.value === 'add' ? '新增失败：' : '修改失败：') + e.message)
  } finally {
    dataSubmitting.value = false
  }
}

async function handleDeleteData(row) {
  try {
    await ElMessageBox.confirm(
      `确定删除字典数据「${row.label}」吗？`,
      '删除确认',
      { type: 'warning' }
    )
    await deleteDictData(row.id)
    ElMessage.success('删除成功')
    await loadDictData()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('删除失败：' + e.message)
  }
}

async function handleMove(row, direction) {
  const list = dictDataList.value
  const idx = list.findIndex(d => d.id === row.id)
  if (idx < 0) return
  const targetIdx = idx + direction
  if (targetIdx < 0 || targetIdx >= list.length) return
  // 交换排序号
  const target = list[targetIdx]
  const oldSort = row.sort
  const newSort = target.sort
  try {
    await updateDictData(row.id, { ...row, sort: newSort })
    await updateDictData(target.id, { ...target, sort: oldSort })
    ElMessage.success('排序已更新')
    await loadDictData()
  } catch (e) {
    // Mock 模式下也更新界面
    ;[list[idx], list[targetIdx]] = [list[targetIdx], list[idx]]
    list[idx].sort = newSort
    list[targetIdx].sort = oldSort
    dictDataList.value = [...list.sort((a, b) => (a.sort ?? 0) - (b.sort ?? 0))]
  }
}

onMounted(loadDictTypes)
</script>

<style scoped>
.dict-layout {
  display: grid;
  grid-template-columns: 320px 1fr;
  gap: 16px;
}
.dict-left, .dict-right { min-height: 400px; }
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.panel-title-row { display: flex; align-items: center; gap: 10px; }
.panel-actions { display: flex; gap: 8px; }
.type-search { margin-bottom: 10px; }
.mono {
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  color: var(--text-2);
}
.muted { color: var(--text-3); font-size: 12px; }
.form-tip { font-size: 12px; color: var(--text-3); margin-top: 4px; }
</style>
