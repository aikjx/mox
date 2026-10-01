/**
 * 租户管理页 DSL —— 与 views/admin/panels/AdminTenant.vue 功能等价。
 * 纯声明：列/搜索/表单项/按钮/统计卡；副作用动作（切换租户、启停）以 handler 内联。
 */
import { markRaw } from 'vue'
import { Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'
import {
  getTenantList, createTenant, updateTenant, deleteTenant, switchTenant,
} from '@/api'

const PLAN_LABEL = { free: '免费', pro: '专业', enterprise: '企业', ultimate: '旗舰' }
const PLAN_TAG = { free: 'info', pro: '', enterprise: 'warning', ultimate: 'danger' }
const STATUS_LABEL = { active: '活跃', inactive: '停用', trial: '试用' }
const STATUS_TAG = { active: 'success', trial: 'warning', inactive: 'danger' }

async function handleSwitchTenant(row) {
  try {
    await ElMessageBox.confirm(
      `即将切换到租户「${row.name}」（${row.code}），切换后将刷新当前会话。`,
      '切换租户', { type: 'info' }
    )
  } catch { return }
  try {
    const res = await switchTenant(row.id)
    if (res?.switched) {
      ElMessage.success(`已切换到租户「${row.name}」`)
      window.dispatchEvent(new CustomEvent('tenant-switched', { detail: res }))
    }
  } catch (e) {
    ElMessage.error('切换失败: ' + (e?.message || e))
  }
}

async function handleToggleStatus(row) {
  const next = row.status === 'active' ? 'inactive' : 'active'
  try {
    await updateTenant(row.id, { status: next })
    ElMessage.success(`租户已${next === 'active' ? '启用' : '停用'}`)
  } catch (e) {
    ElMessage.error('状态切换失败: ' + (e?.message || e))
  }
}

export const tenantPage = markRaw({
  key: 'tenant',

  api: {
    list: () => getTenantList(),
    create: (payload) => createTenant(payload),
    update: (id, payload) => updateTenant(id, payload),
    remove: (id) => deleteTenant(id),
  },

  search: {
    showKeyword: true,
    keywordPlaceholder: '搜索租户名称/编码',
    fields: [
      { prop: 'status', label: '状态', type: 'select', options: [
        { label: '活跃', value: 'active' },
        { label: '停用', value: 'inactive' },
        { label: '试用', value: 'trial' },
      ] },
    ],
  },

  toolbar: [
    { label: '新建租户', type: 'primary', icon: Plus, action: 'create' },
    { label: '刷新', icon: Refresh, action: 'reload' },
  ],

  stats: [
    { label: '租户总数', icon: '🏢', color: '#8BC8EA', value: (rows) => rows.length },
    { label: '活跃租户', icon: '✅', color: '#52C41A', value: (rows) => rows.filter(t => t.status === 'active').length },
    { label: '试用租户', icon: '⏳', color: '#FAAD14', value: (rows) => rows.filter(t => t.status === 'trial').length },
    { label: '企业版', icon: '💎', color: '#C9A7E8',
      value: (rows) => rows.filter(t => t.plan === 'enterprise' || t.plan === 'ultimate').length },
  ],

  list: {
    rowKey: 'id',
    showPagination: false,           // 原版无分页（小数据集全量展示）
    columns: [
      { prop: 'code', label: '租户编码', width: 140, widget: 'tag',
        tagTypeOf: (row) => row.status === 'active' ? 'success' : 'info',
        labelOf: (row) => row.code },
      { prop: 'name', label: '租户名称', minWidth: 180 },
      { prop: 'mode', label: '隔离模式', width: 110, widget: 'tag',
        tagTypeOf: (row) => row.mode === 'physical' ? 'warning' : '',
        labelOf: (row) => row.mode === 'physical' ? '物理隔离' : '逻辑隔离' },
      { prop: 'plan', label: '套餐', width: 100, widget: 'tag',
        tagTypeOf: (row) => PLAN_TAG[row.plan] || '',
        labelOf: (row) => PLAN_LABEL[row.plan] || row.plan },
      { prop: 'status', label: '状态', width: 90, widget: 'tag',
        tagTypeOf: (row) => STATUS_TAG[row.status] || 'info',
        labelOf: (row) => STATUS_LABEL[row.status] || row.status },
      { prop: 'createdAt', label: '创建时间', width: 170 },
    ],
    rowActions: [
      { label: '切换', type: 'primary', handler: handleSwitchTenant },
      { label: '编辑', type: 'primary', action: 'edit' },
      { label: (row) => row.status === 'active' ? '停用' : '启用',
        type: 'warning', show: (row) => row.code !== 'T001', handler: handleToggleStatus },
      { label: '删除', type: 'danger', show: (row) => row.code !== 'T001', action: 'delete',
        confirm: (row) => `确定删除租户「${row.name}」？该操作不可恢复，租户下所有数据将被清除。` },
    ],
  },

  form: {
    width: '520px',
    createTitle: '新建租户',
    editTitle: '编辑租户',
    fields: [
      { prop: 'code', label: '租户编码', type: 'input', placeholder: '如 T002',
        disabledOnEdit: true,
        rules: [{ required: true, message: '请输入租户编码', trigger: 'blur' }] },
      { prop: 'name', label: '租户名称', type: 'input', placeholder: '如 某某科技有限公司',
        rules: [{ required: true, message: '请输入租户名称', trigger: 'blur' }] },
      { prop: 'mode', label: '隔离模式', type: 'radio', defaultValue: 'logical',
        options: [{ label: '逻辑隔离', value: 'logical' }, { label: '物理隔离', value: 'physical' }] },
      { prop: 'plan', label: '套餐', type: 'select', defaultValue: 'free',
        options: [
          { label: '免费版', value: 'free' }, { label: '专业版', value: 'pro' },
          { label: '企业版', value: 'enterprise' }, { label: '旗舰版', value: 'ultimate' },
        ] },
      { prop: 'status', label: '状态', type: 'radio', defaultValue: 'active', visibleOnEdit: true,
        options: [
          { label: '活跃', value: 'active' }, { label: '停用', value: 'inactive' }, { label: '试用', value: 'trial' },
        ] },
    ],
    // 与原版一致：创建不提交 status，编辑不提交 code/mode
    buildPayload(formData, isEdit) {
      return isEdit
        ? { name: formData.name, status: formData.status, plan: formData.plan }
        : { code: formData.code, name: formData.name, mode: formData.mode, plan: formData.plan }
    },
  },
})
