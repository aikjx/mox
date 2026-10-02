/** Read-only IAM audit metadata. The backend verifies actual administrator status. */
import { markRaw } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { getAuditLogs } from '@/api'
import { parseAuditPage } from '@/utils'
import { useAuthStore } from '@/stores'

export const auditPage = markRaw({
  key: 'iam-audit', readOnly: true,
  identityScope: () => {
    const auth = useAuthStore()
    return [auth.accessToken, auth.userId, auth.tenantId]
  },
  api: { list: async ({ pageNum = 1, pageSize = 20, ...filters } = {}) => {
    const data = parseAuditPage(await getAuditLogs({ page: pageNum, page_size: pageSize, ...filters }))
    return { list: data.items, total: data.total }
  } },
  toolbar: [{ label: '刷新', icon: Refresh, action: 'reload' }],
  search: { showKeyword: false, fields: [
    { prop: 'action', label: '动作（精确）', type: 'input' },
    { prop: 'actor', label: '操作者 ID（精确）', type: 'input' },
    { prop: 'since', label: '开始 RFC3339', type: 'input', placeholder: '含时区的时间' },
    { prop: 'until', label: '结束 RFC3339', type: 'input', placeholder: '含时区的时间' },
  ] },
  list: { rowKey: 'id', serverPagination: true, defaultPageSize: 20, columns: [
    { prop: 'id', label: '审计 ID', minWidth: 180 },
    { prop: 'action', label: '动作', minWidth: 200 },
    { prop: 'userId', label: '操作者', minWidth: 180 },
    { prop: 'resourceType', label: '资源类型', minWidth: 140 },
    { prop: 'resourceId', label: '资源 ID', minWidth: 180 },
    { prop: 'statusCode', label: '记录的 HTTP 状态', width: 160, widget: 'tag',
      labelOf: row => row.statusCode == null ? '未记录' : String(row.statusCode),
      tagTypeOf: row => row.statusCode == null ? 'info' : row.statusCode >= 400 ? 'danger' : 'success' },
    { prop: 'httpMethod', label: '请求方法', width: 100 },
    { prop: 'httpPath', label: '请求路径', minWidth: 180 },
    { prop: 'latencyMs', label: '耗时(ms)', width: 100 },
    { prop: 'createdAt', label: '记录时间', width: 180, widget: 'date' },
  ] },
})
