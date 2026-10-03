/**
 * 访问凭证页 DSL —— 与 views/admin/panels/AdminAccess.vue 的列表/创建/吊销功能等价。
 * 服务端分页与真实凭证状态共用 IAM 契约；创建回执通过一次性对话框展示。
 */
import { markRaw, h } from 'vue'
import { Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'
import { getApiKeys, createApiKey, revokeApiKey } from '@/api'
import { parseApiKeyPage, apiKeyEligibilityLabel } from '@/utils'
import { useAuthStore } from '@/stores/auth.store'

// 引擎 pageNum/pageSize 映射为 API page/page_size，不拉取全量。
async function listKeys({ pageNum = 1, pageSize = 20 } = {}) {
  const data = parseApiKeyPage(await getApiKeys({ page: pageNum, page_size: pageSize }))
  return { list: data.items, total: data.total }
}

async function showCreatedKey(created) {
  const key = created?.api_key || created?.key || ''
  if (!key) throw new Error('未收到实际凭证，请刷新核对创建结果')
  await ElMessageBox.alert(
    h('pre', { style: 'white-space:pre-wrap;word-break:break-all;background:var(--bg-panel-2);padding:12px;border-radius:8px;' }, key),
    '凭证已创建（明文仅此一次展示）',
    { confirmButtonText: '我已保存', type: 'warning' }
  )
}

export const accessPage = markRaw({
  key: 'access',
  identityScope: () => {
    const auth = useAuthStore()
    return [auth.accessToken, auth.userId, auth.tenantId]
  },
  onIdentityChange: () => ElMessageBox.close(),

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
    serverPagination: true,
    defaultPageSize: 20,
    columns: [
      { prop: 'name', label: '凭证名称', minWidth: 160 },
      { prop: 'user_id', label: '归属用户', minWidth: 180 },
      { prop: 'createdAt', label: '创建时间', width: 180, widget: 'date' },
      { prop: 'last_used_at', label: '最近使用', width: 180, widget: 'date', emptyText: '从未使用' },
      { prop: 'expires_at', label: '到期时间', width: 180, widget: 'date', emptyText: '未设定' },
      { prop: 'eligibility', label: '基础配置状态', width: 180, widget: 'tag',
        tagTypeOf: (row) => row.eligibility === 'eligible' ? 'success' : 'warning',
        labelOf: (row) => apiKeyEligibilityLabel(row.eligibility) },
    ],
    rowActions: [
      { label: '吊销', type: 'danger', show: (row) => row.active,
        handler: async (row, ctx) => {
          try {
            await ElMessageBox.confirm(
              `确定吊销凭证「${row.name}」吗？吊销后使用该凭证的请求将立即失效。`,
              '吊销确认', { type: 'warning' }
            )
            if (!ctx.isCurrent()) return
            await revokeApiKey(row.id)
            if (!ctx.isCurrent()) return
            ElMessage.success(`凭证「${row.name}」已吊销`)
            ctx.loadList()
          } catch (e) {
            if (ctx.isCurrent() && e !== 'cancel' && e?.message) ElMessage.error('吊销失败：' + e.message)
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
      { prop: 'expires_at', label: '到期时间', type: 'input', placeholder: '可选未来 RFC3339 时间' },
    ],
    buildPayload: (formData) => ({
      name: formData.name.trim(),
      expires_at: formData.expires_at?.trim() || null,
    }),
    afterCreate: showCreatedKey,
  },
})
