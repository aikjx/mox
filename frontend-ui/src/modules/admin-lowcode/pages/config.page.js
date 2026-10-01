/**
 * 参数配置页 DSL —— 与 views/admin/panels/AdminConfig.vue 功能等价。
 * 首个服务端分页（serverPagination）CRUD 页面。
 */
import { markRaw } from 'vue'
import { Plus, Refresh, RefreshRight } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import {
  getConfigList, createConfig, updateConfig, deleteConfig, refreshConfigCache,
} from '@/api'

export const configPage = markRaw({
  key: 'config',

  api: {
    list: (params) => getConfigList(params),
    create: (payload) => createConfig(payload),
    update: (id, payload) => updateConfig(id, payload),
    remove: (id) => deleteConfig(id),
  },

  search: {
    showKeyword: false,
    fields: [
      { prop: 'name', label: '参数名称', type: 'input', placeholder: '参数名称' },
      { prop: 'key', label: '参数键名', type: 'input', placeholder: '参数键名' },
    ],
  },

  toolbar: [
    { label: '刷新缓存', type: 'success', icon: RefreshRight,
      handler: async () => { await refreshConfigCache(); ElMessage.success('缓存刷新成功') } },
    { label: '新增参数', type: 'primary', icon: Plus, action: 'create' },
  ],

  list: {
    rowKey: 'id',
    serverPagination: true,
    defaultPageSize: 10,
    columns: [
      { prop: 'name', label: '参数名称', minWidth: 160 },
      { prop: 'key', label: '参数键名', minWidth: 220 },
      { prop: 'value', label: '参数键值', minWidth: 200 },
      { prop: 'isBuiltin', label: '系统内置', width: 100, widget: 'dict', align: 'center',
        map: { true: '是', false: '否' }, tagType: { true: 'warning', false: 'info' } },
      { prop: 'status', label: '状态', width: 90, widget: 'dict', align: 'center',
        map: { 1: '正常', 0: '停用' }, tagType: { 1: 'success', 0: 'danger' } },
      { prop: 'remark', label: '备注', minWidth: 180 },
      { prop: 'createdAt', label: '创建时间', width: 180, widget: 'date' },
    ],
    rowActions: [
      { label: '编辑', type: 'primary', action: 'edit' },
      { label: '删除', type: 'danger', action: 'delete',
        disabled: (row) => !!row.isBuiltin,
        confirm: (row) => `确定删除参数「${row.name}」吗？删除后不可恢复。` },
    ],
  },

  form: {
    width: '560px',
    createTitle: '新增参数配置',
    editTitle: '编辑参数配置',
    labelWidth: '100px',
    fields: [
      { prop: 'name', label: '参数名称', type: 'input', maxlength: 100, showWordLimit: true,
        rules: [{ required: true, message: '请输入参数名称', trigger: 'blur' }] },
      { prop: 'key', label: '参数键名', type: 'input', maxlength: 100, disabledOnEdit: true,
        placeholder: '如 sys.user.initPassword',
        rules: [{ required: true, message: '请输入参数键名', trigger: 'blur' }] },
      { prop: 'value', label: '参数键值', type: 'textarea', rows: 3, maxlength: 500, showWordLimit: true,
        rules: [{ required: true, message: '请输入参数键值', trigger: 'blur' }] },
      { prop: 'isBuiltin', label: '系统内置', type: 'radio', defaultValue: false,
        options: [{ label: '是', value: true }, { label: '否', value: false }] },
      { prop: 'status', label: '状态', type: 'radio', defaultValue: 1,
        options: [{ label: '正常', value: 1 }, { label: '停用', value: 0 }] },
      { prop: 'remark', label: '备注', type: 'textarea', rows: 2, maxlength: 200 },
    ],
    buildPayload: (formData) => ({ ...formData }),
  },
})
