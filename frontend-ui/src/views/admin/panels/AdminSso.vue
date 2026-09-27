<template>
  <div class="admin-sso">
    <el-card shadow="never" class="protocols-card">
      <template #header>
        <span class="card-title">支持的 SSO 协议</span>
      </template>
      <div class="protocol-chips">
        <el-tag v-for="p in protocols" :key="p.code" effect="plain" class="chip">
          {{ p.name }} · {{ p.description }}
        </el-tag>
      </div>
    </el-card>

    <el-card shadow="never">
      <template #header>
        <div class="table-header">
          <span class="card-title">SSO 提供商</span>
          <el-button v-permission="'sso:create'" type="primary" :icon="Plus" @click="openCreate">新增提供商</el-button>
        </div>
      </template>

      <el-table v-loading="loading" :data="providers" border stripe>
        <el-table-column prop="name" label="名称" min-width="140" />
        <el-table-column prop="protocol" label="协议" width="110">
          <template #default="{ row }">
            <el-tag size="small" effect="plain">{{ row.protocol }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-switch
              v-permission="'sso:update'"
              :model-value="row.status === 'enabled'"
              @change="toggleStatus(row)"
            />
            <span v-if="!canToggleStatus" :class="row.status === 'enabled' ? 'ok' : 'muted'">
              {{ row.status === 'enabled' ? '启用' : '停用' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="client_id" label="Client ID" min-width="160" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="muted">{{ row.client_id || '未配置' }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="sort_order" label="排序" width="80" />
        <el-table-column prop="updated_at" label="更新时间" width="180">
          <template #default="{ row }">{{ formatTime(row.updated_at) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="160" fixed="right">
          <template #default="{ row }">
            <el-button v-permission="'sso:update'" link type="primary" @click="openEdit(row)">编辑</el-button>
            <el-popconfirm
              v-permission="'sso:delete'"
              title="确定删除该提供商？"
              @confirm="removeProvider(row)"
            >
              <template #reference>
                <el-button link type="danger">删除</el-button>
              </template>
            </el-popconfirm>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <FormDialog
      v-model:visible="dialogVisible"
      :title="isEdit ? '编辑 SSO 提供商' : '新增 SSO 提供商'"
      :form-schema="formSchema"
      :edit-data="formData"
      :width="620"
      label-width="110px"
      :submitting="submitting"
      @submit="onSubmit"
    />
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus/es/components/message/index.mjs'
import 'element-plus/es/components/message/style/css.mjs'
import { Plus } from '@element-plus/icons-vue'
import { usePermissionStore } from '@/stores'
import { FormDialog } from '@/components'
import {
  getSsoProtocols, getSsoProviders, createSsoProvider,
  updateSsoProvider, deleteSsoProvider,
} from '@/api'

const loading = ref(false)
const permissionStore = usePermissionStore()
const canToggleStatus = computed(() => permissionStore.hasPermission('sso:update'))
const submitting = ref(false)
const protocols = ref([])
const providers = ref([])
const dialogVisible = ref(false)
const isEdit = ref(false)
const editingId = ref('')

const formData = reactive({})

async function loadProtocols() {
  try {
    protocols.value = await getSsoProtocols()
  } catch { /* 协议展示失败不阻断列表 */ }
}

async function loadList() {
  loading.value = true
  try {
    providers.value = await getSsoProviders()
  } catch (err) {
    ElMessage.error(err.message || '加载提供商失败')
  } finally {
    loading.value = false
  }
}

function openCreate() {
  isEdit.value = false
  editingId.value = ''
  Object.keys(formData).forEach(k => delete formData[k])
  dialogVisible.value = true
}

function openEdit(row) {
  isEdit.value = true
  editingId.value = row.provider_id
  Object.assign(formData, row)
  dialogVisible.value = true
}

async function toggleStatus(row) {
  const next = row.status === 'enabled' ? 'disabled' : 'enabled'
  try {
    await updateSsoProvider(row.provider_id, { status: next })
    row.status = next
    ElMessage.success(next === 'enabled' ? `已启用「${row.name}」` : `已停用「${row.name}」`)
  } catch (err) {
    ElMessage.error(err.message || '状态更新失败')
  }
}

async function removeProvider(row) {
  try {
    await deleteSsoProvider(row.provider_id)
    ElMessage.success(`已删除「${row.name}」`)
    await loadList()
  } catch (err) {
    ElMessage.error(err.message || '删除失败')
  }
}

async function onSubmit(payload) {
  submitting.value = true
  try {
    const body = {
      name: payload.name?.trim(),
      protocol: payload.protocol,
      client_id: payload.client_id || '',
      client_secret: payload.client_secret || '',
      auth_endpoint: payload.auth_endpoint || '',
      token_endpoint: payload.token_endpoint || '',
      userinfo_endpoint: payload.userinfo_endpoint || '',
      logout_endpoint: payload.logout_endpoint || '',
      redirect_uri: payload.redirect_uri || '',
      scopes: splitScopes(payload.scopes),
      sort_order: Number(payload.sort_order) || 0,
      is_default: !!payload.is_default,
    }
    if (isEdit.value) {
      await updateSsoProvider(editingId.value, body)
      ElMessage.success('更新成功')
    } else {
      await createSsoProvider(body)
      ElMessage.success('创建成功（默认停用，配置 Client 后再启用）')
    }
    dialogVisible.value = false
    await loadList()
  } catch (err) {
    ElMessage.error(err.message || '保存失败')
  } finally {
    submitting.value = false
  }
}

function splitScopes(v) {
  if (Array.isArray(v)) return v
  return String(v || '').split(/[\s,]+/).filter(Boolean)
}

function formatTime(t) {
  if (!t) return '-'
  return String(t).replace('T', ' ').replace(/\.\d+Z?$/, '')
}

// FormDialog 支持 visible(formData) 按协议分支显示字段
const isOauthLike = (fd) => ['oauth2', 'oidc'].includes(fd.protocol)
const formSchema = [
  { prop: 'name', label: '名称', required: true,
    rules: [{ required: true, message: '请输入名称', trigger: 'blur' }] },
  { prop: 'protocol', label: '协议类型', type: 'select', disabled: () => isEdit.value,
    options: [
      { label: 'OAuth 2.0', value: 'oauth2' },
      { label: 'OpenID Connect', value: 'oidc' },
      { label: 'SAML 2.0', value: 'saml' },
      { label: 'CAS', value: 'cas' },
      { label: 'LDAP', value: 'ldap' },
    ],
    rules: [{ required: true, message: '请选择协议', trigger: 'change' }] },
  { prop: 'client_id', label: 'Client ID' },
  { prop: 'client_secret', label: 'Client Secret', type: 'input', inputType: 'password', showPassword: true },
  { prop: 'auth_endpoint', label: '授权端点', visible: isOauthLike },
  { prop: 'token_endpoint', label: 'Token 端点', visible: isOauthLike },
  { prop: 'userinfo_endpoint', label: 'UserInfo 端点', visible: isOauthLike },
  { prop: 'logout_endpoint', label: '登出端点', visible: isOauthLike },
  { prop: 'redirect_uri', label: '回调地址', visible: isOauthLike, placeholder: 'https://your-app/callback' },
  { prop: 'scopes', label: 'Scopes', placeholder: '空格或逗号分隔，如 openid email',
    visible: (fd) => fd.protocol === 'oidc' },
  { prop: 'sort_order', label: '排序', type: 'number' },
  { prop: 'is_default', label: '默认提供商', type: 'switch' },
]

onMounted(() => { loadProtocols(); loadList() })
</script>

<style scoped>
.admin-sso { display: flex; flex-direction: column; gap: 16px; }
.card-title { font-weight: 600; }
.protocols-card :deep(.el-card__body) { display: flex; flex-wrap: wrap; gap: 8px; }
.chip { margin: 2px; }
.table-header { display: flex; justify-content: space-between; align-items: center; }
.muted { color: #909399; }
.ok { color: #67c23a; }
</style>
