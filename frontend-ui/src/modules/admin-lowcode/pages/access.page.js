/**
 * 访问凭证页 DSL —— 与 views/admin/panels/AdminAccess.vue 的列表/创建/吊销功能等价。
 * 注：凭证校验面板与"明文一次性展示"为开发工具型自定义 UI，保留手写（见 rollout 报告）。
 */
import { markRaw, h } from 'vue'
import { Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getApiKeys, createApiKey, revokeApiKey } from '@/api'

// 后端行结构 { id,name,apiKey,status:"active"|"revoked",scopes,createdAt,lastUsed }
// 归一化为面板期望：active(布尔) / permissions(数组)
async function listKeys() {
  const data = await getApiKeys()
  return (Array.isArray(data) ? data : []).map(r => ({
    ...r,
    active: r.active != null ? !!r.active : r.status === 'active',
    permissions: r.permissions || r.scopes || [],
  }))
}

async function showCreatedKey(created) {
  const key = created?.api_key || created?.key || ''
  if (!key) return
  await ElMessageBox.alert(
    h('pre', { style: 'white-space:pre-wrap;word-break:break-all;background:var(--bg-panel-2);padding:12px;border-radius:8px;' }, key),
    '凭证已创建（明文仅此一次展示）',
    { confirmButtonText: '我已保存', type: 'warning' }
  )
}

export const accessPage = markRaw({
  key: 'access',

  api: {
    list: listKeys,
    create: (payload) => createApiKey(payload),
    update: () => Promise.reject(new Error('凭证不支持编辑')),
    remove: (id) => revokeApiKey(id),
  },

  toolbar: [
    { label: '新建凭证', type: 'primary', icon: Plus, action: 'create' },
    { label: '刷新', icon: Refresh, action: 'reload' },
  ],

  list: {
    rowKey: 'id',
    showPagination: false,
    columns: [
      { prop: 'name', label: '凭证名称', minWidth: 160 },
      { prop: 'permissions', label: '权限', width: 220, widget: 'arrayTags' },
      { prop: 'createdAt', label: '创建时间', width: 180, widget: 'date' },
      { prop: 'lastUsed', label: '最近使用', width: 180, widget: 'date', emptyText: '从未使用' },
      { prop: 'status', label: '状态', width: 100, widget: 'tag',
        tagTypeOf: (row) => row.active ? 'success' : 'warning',
        labelOf: (row) => row.active ? '活跃' : '已吊销' },
    ],
    rowActions: [
      { label: '吊销', type: 'danger', show: (row) => row.active,
        handler: async (row, ctx) => {
          try {
            await ElMessageBox.confirm(
              `确定吊销凭证「${row.name}」吗？吊销后使用该凭证的请求将立即失效。`,
              '吊销确认', { type: 'warning' }
            )
            await revokeApiKey(row.id)
            ElMessage.success(`凭证「${row.name}」已吊销`)
            ctx.loadList()
          } catch (e) {
            if (e !== 'cancel' && e?.message) ElMessage.error('吊销失败：' + e.message)
          }
        } },
    ],
  },

  form: {
    width: '480px',
    createTitle: '新建访问凭证',
    editTitle: '新建访问凭证',
    fields: [
      { prop: 'name', label: '凭证名称', type: 'input', maxlength: 64, placeholder: '例如：运维巡检客户端',
        rules: [{ required: true, message: '请输入凭证名称', trigger: 'blur' }] },
      { prop: 'permissions', label: '权限', type: 'checkboxGroup', defaultValue: ['read'],
        options: [
          { label: 'read（读取）', value: 'read' },
          { label: 'write（写入）', value: 'write' },
          { label: 'admin（管理）', value: 'admin' },
        ] },
    ],
    buildPayload: (formData) => ({
      name: formData.name.trim(),
      permissions: formData.permissions?.length ? formData.permissions : ['read'],
    }),
    afterCreate: showCreatedKey,
  },
})
