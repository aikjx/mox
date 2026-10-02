<template>
  <el-dialog :model-value="visible" title="IAM 业务权限" width="min(680px, calc(100vw - 32px))" destroy-on-close @update:model-value="emit('update:visible', $event)">
    <p>角色「{{ role?.name }}」的直接权限。保存会替换直接授权；继承权限仍由父角色决定。此操作需要本租户真实超级管理员。</p>
    <el-alert v-if="error" :title="error" type="error" show-icon :closable="false" />
    <div class="iam-toolbar">
      <el-button :loading="loading" :disabled="saving || registering" @click="load">重新加载</el-button>
      <el-button :loading="registering" :disabled="loading || saving" @click="register">注册站内消息发送权限点</el-button>
    </div>
    <div v-loading="loading" class="iam-permissions">
      <el-checkbox-group v-if="ready" v-model="selected" :disabled="saving || registering">
        <el-checkbox v-for="item in catalog" :key="item.id" :value="item.id" :disabled="item.status !== 'active' && !selected.includes(item.id)">
          {{ item.name }} · {{ item.code }} · {{ item.tenant_id }}{{ item.status !== 'active' ? '（停用）' : '' }}
        </el-checkbox>
      </el-checkbox-group>
      <el-empty v-if="ready && !catalog.length" description="暂无真实 IAM 权限点" />
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">关闭</el-button>
      <el-button type="primary" :loading="saving" :disabled="!ready || loading || registering" @click="save">保存直接权限</el-button>
    </template>
  </el-dialog>
</template>

<script setup>
import { ref, watch, onBeforeUnmount } from 'vue'
import { useAuthStore } from '@/stores'
import { getIamPermissionCatalog, getIamRolePermissions, registerMessageSendPermission, replaceIamRolePermissions } from '@/api'
import { ElMessage } from 'element-plus/es/components/message/index'

const props = defineProps({ visible: Boolean, role: { type: Object, default: null } })
const emit = defineEmits(['update:visible'])
const auth = useAuthStore()
const catalog = ref([])
const selected = ref([])
const version = ref(null)
const ready = ref(false)
const loading = ref(false)
const saving = ref(false)
const registering = ref(false)
const error = ref('')
let generation = 0
let action = 0

async function load() {
  const request = ++generation
  ready.value = false
  catalog.value = []
  selected.value = []
  version.value = null
  error.value = ''
  if (!props.visible || !props.role?.id || !auth.accessToken) { loading.value = false; return }
  loading.value = true
  try {
    const [items, snapshot] = await Promise.all([getIamPermissionCatalog(), getIamRolePermissions(props.role.id)])
    if (request !== generation) return
    if (!Array.isArray(items?.items) || !Array.isArray(snapshot?.permission_ids) || !Number.isSafeInteger(snapshot?.version) || snapshot.version < 1 || snapshot.role_id !== props.role.id) throw new Error('权限接口返回格式无效')
    catalog.value = items.items
    selected.value = snapshot.permission_ids
    version.value = snapshot.version
    ready.value = true
  } catch (cause) {
    if (request === generation) error.value = cause?.message || '权限加载失败'
  } finally {
    if (request === generation) loading.value = false
  }
}
async function register() {
  if (registering.value || saving.value) return
  const request = generation
  const operation = ++action
  registering.value = true
  error.value = ''
  try {
    await registerMessageSendPermission()
    if (request !== generation) return
    if (ready.value) {
      const items = await getIamPermissionCatalog()
      if (request !== generation) return
      if (!Array.isArray(items?.items)) throw new Error('权限目录格式无效')
      catalog.value = items.items
    } else await load()
  } catch (cause) {
    if (request === generation) error.value = cause?.message || '权限点注册失败'
  } finally { if (operation === action) registering.value = false }
}
async function save() {
  if (!ready.value || saving.value) return
  const request = generation
  const operation = ++action
  saving.value = true
  error.value = ''
  try {
    await replaceIamRolePermissions(props.role.id, { version: version.value, permission_ids: [...selected.value] })
    if (request !== generation) return
    ElMessage.success('权限与审计记录已提交')
    await load()
  } catch (cause) {
    if (request === generation) {
      ready.value = false
      error.value = `${cause?.message || '保存未确认成功'}；请重新加载后核对`
    }
  } finally { if (operation === action) saving.value = false }
}
watch(() => [props.visible, props.role?.id, auth.accessToken, auth.userId, auth.tenantId], () => {
  action++
  saving.value = false
  registering.value = false
  load()
}, { immediate: true, flush: 'sync' })
onBeforeUnmount(() => { generation++; action++ })
</script>

<style scoped>
.iam-toolbar { display: flex; flex-wrap: wrap; gap: 12px; margin: 16px 0; }
.iam-permissions { max-height: 420px; min-height: 100px; overflow: auto; }
.iam-permissions :deep(.el-checkbox) { display: flex; height: auto; min-height: 36px; white-space: normal; }
.iam-permissions :deep(.el-checkbox__label) { white-space: normal; overflow-wrap: anywhere; line-height: 22px; }
</style>
