<template>
  <div class="adm-access">
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <span class="badge primary">凭证绑定当前管理员，开放收件箱与通知；新凭证仅存哈希，明文仅展示一次</span>
        </div>
        <div class="toolbar-right">
          <el-button :icon="Refresh" :loading="loading" @click="load">刷新</el-button>
          <el-button type="primary" :icon="Plus" @click="openCreate">新建凭证</el-button>
        </div>
      </div>

      <el-table :data="keys" v-loading="loading" stripe style="width: 100%">
        <el-table-column prop="name" label="凭证名称" min-width="160" />
        <el-table-column label="开放流程" width="220">
          <template #default="{ row }">
            <el-tag size="small">收件箱与通知</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="180">
          <template #default="{ row }">{{ fmtTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column prop="lastUsed" label="最近使用" width="180">
          <template #default="{ row }">{{ row.last_used_at ? fmtTime(row.last_used_at) : '从未使用' }}</template>
        </el-table-column>
        <el-table-column label="到期时间" width="180">
          <template #default="{ row }">{{ row.expires_at ? fmtTime(row.expires_at) : '未设定' }}</template>
        </el-table-column>
        <el-table-column label="基础配置状态" width="180">
          <template #default="{ row }">
            <span class="badge" :class="row.eligibility === 'eligible' ? 'success' : 'warning'">{{ apiKeyEligibilityLabel(row.eligibility) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="110" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.active"
              type="danger"
              size="small"
              text
              :icon="Delete"
              @click="handleRevoke(row)"
            >吊销</el-button>
            <span v-else class="muted">-</span>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination v-model:current-page="page" v-model:page-size="pageSize" :total="total" :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next" @current-change="load" @size-change="changePageSize" />
      <p class="muted">基础配置状态来自数据库快照；重复凭证及实际业务权限仍须由每次鉴权确认。</p>
    </div>

    <div class="panel card-pad">
      <h3 class="section-title">凭证校验</h3>
      <div class="validate-row">
        <el-input
          v-model="validateKeyText"
          placeholder="粘贴待校验的 API Key 明文"
          clearable
          style="max-width: 420px"
        />
        <el-button type="primary" :loading="validating" @click="handleValidate">校验</el-button>
        <span v-if="validateResult" class="badge" :class="validateResult.valid ? 'success' : 'warning'">
          {{ validateResult.valid
            ? `有效 · 用户 ${validateResult.user_id} · 收件箱与通知`
            : `无效 · ${validateResult.reason || '未知原因'}` }}
        </span>
      </div>
    </div>

    <!-- 新建凭证 -->
    <el-dialog v-model="createVisible" title="新建访问凭证" width="min(640px, calc(100vw - 32px))">
      <el-form label-width="90px">
        <el-form-item label="凭证名称" required>
          <el-input v-model="createForm.name" placeholder="例如：运维巡检客户端" maxlength="64" />
        </el-form-item>
        <el-form-item label="有效期">
          <el-input v-model="createForm.expires_at" placeholder="可选 RFC3339，例如 2027-01-01T00:00:00+08:00" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="creating" @click="handleCreate">创建</el-button>
      </template>
    </el-dialog>

    <!-- 明文展示（仅一次） -->
    <el-dialog v-model="keyVisible" title="凭证已创建（明文仅此一次展示）" width="min(640px, calc(100vw - 32px))">
      <el-alert
        type="warning"
        :closable="false"
        title="请立即复制保存：后端只存哈希，关闭后无法再次查看明文"
        style="margin-bottom: 14px"
      />
      <pre class="key-pre">{{ createdKey }}</pre>
      <template #footer>
        <el-button :icon="CopyDocument" @click="copyKey">复制</el-button>
        <el-button type="primary" @click="keyVisible = false">我已保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { formatDateTimeLocaleOr as fmtTime, parseApiKeyPage, apiKeyEligibilityLabel } from '@/utils'
import { ref, reactive, watch, onBeforeUnmount } from 'vue'
import { useAuthStore } from '@/stores'
import { Refresh, Plus, Delete, CopyDocument } from '@element-plus/icons-vue'
import { getApiKeys, createApiKey, revokeApiKey, validateApiKey } from '@/api'
import { ElMessage } from 'element-plus/es/components/message/index'
import { ElMessageBox } from 'element-plus/es/components/message-box/index'

const loading = ref(false)
const keys = ref([])
const page = ref(1)
const pageSize = ref(20)
const total = ref(0)
const auth = useAuthStore()
let generation = 0
let listRequest = 0

const createVisible = ref(false)
const creating = ref(false)
const createForm = reactive({ name: '', expires_at: '' })

const keyVisible = ref(false)
const createdKey = ref('')

const validateKeyText = ref('')
const validating = ref(false)
const validateResult = ref(null)

async function load() {
  const scope = generation
  const request = ++listRequest
  loading.value = true
  try {
    const data = await getApiKeys({ page: page.value, page_size: pageSize.value })
    if (scope !== generation || request !== listRequest) return
    const result = parseApiKeyPage(data)
    keys.value = result.items
    total.value = result.total
  } catch (e) {
    if (scope !== generation || request !== listRequest) return
    keys.value = []
    total.value = 0
    ElMessage.error('加载凭证列表失败：' + e.message)
  } finally {
    if (scope === generation && request === listRequest) loading.value = false
  }
}
function changePageSize() { page.value = 1; load() }

function openCreate() {
  createForm.name = ''
  createForm.expires_at = ''
  createVisible.value = true
}

async function handleCreate() {
  if (creating.value) return
  const scope = generation
  if (!createForm.name.trim()) {
    ElMessage.warning('请输入凭证名称')
    return
  }
  creating.value = true
  try {
    const data = await createApiKey({
      name: createForm.name.trim(),
      expires_at: createForm.expires_at.trim() || null
    })
    if (scope !== generation) return
    // 后端创建返回 { id, name, api_key(明文仅此一次), active, createdAt }
    createdKey.value = data?.api_key || data?.key || ''
    if (!createdKey.value) throw new Error('未收到实际凭证，请刷新核对创建结果')
    createVisible.value = false
    keyVisible.value = true
    page.value = 1
    await load()
  } catch (e) {
    if (scope === generation) ElMessage.error('创建未确认成功，请刷新核对：' + e.message)
  } finally {
    if (scope === generation) creating.value = false
  }
}

async function copyKey() {
  try {
    await navigator.clipboard.writeText(createdKey.value)
    ElMessage.success('已复制到剪贴板')
  } catch {
    ElMessage.warning('复制失败，请手动选择复制')
  }
}

async function handleRevoke(row) {
  const scope = generation
  try {
    await ElMessageBox.confirm(
      `确定吊销凭证「${row.name}」吗？吊销后使用该凭证的请求将立即失效。`,
      '吊销确认',
      { type: 'warning' }
    )
    if (scope !== generation) return
    await revokeApiKey(row.id)
    if (scope !== generation) return
    ElMessage.success(`凭证「${row.name}」已吊销`)
    await load()
  } catch (e) {
    if (scope === generation && e !== 'cancel' && e?.message) ElMessage.error('吊销未确认成功：' + e.message)
  }
}

async function handleValidate() {
  if (validating.value) return
  const scope = generation
  const key = validateKeyText.value.trim()
  if (!key) {
    ElMessage.warning('请输入待校验的 Key 明文')
    return
  }
  validating.value = true
  try {
    const result = await validateApiKey(key)
    if (scope === generation) validateResult.value = result
  } catch (e) {
    if (scope === generation) {
      validateResult.value = null
      ElMessage.error('校验未完成：' + e.message)
    }
  } finally {
    if (scope === generation) validating.value = false
  }
}

watch(() => [auth.accessToken, auth.userId, auth.tenantId], () => {
  generation++
  keys.value = []
  page.value = 1
  total.value = 0
  createdKey.value = ''
  validateKeyText.value = ''
  validateResult.value = null
  keyVisible.value = false
  createVisible.value = false
  creating.value = false
  validating.value = false
  if (auth.accessToken) load()
  else loading.value = false
}, { immediate: true, flush: 'sync' })
watch(keyVisible, visible => { if (!visible) createdKey.value = '' })
onBeforeUnmount(() => { generation++; listRequest++; createdKey.value = '' })
</script>

<style scoped>
.toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; flex-wrap: wrap; gap: 10px; }
.toolbar-right { display: flex; gap: 8px; }
.validate-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.muted { color: var(--text-3); }
.key-pre {
  background: var(--bg-panel-2);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 14px;
  font-family: Consolas, Monaco, monospace;
  font-size: 12px;
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
