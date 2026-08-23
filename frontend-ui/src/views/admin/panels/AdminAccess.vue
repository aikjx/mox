<template>
  <div class="adm-access">
    <div class="panel card-pad">
      <div class="toolbar">
        <div class="toolbar-left">
          <span class="badge primary">访问凭证即系统身份：后端仅存 SHA-256 哈希，明文仅创建时展示一次</span>
        </div>
        <div class="toolbar-right">
          <el-button :icon="Refresh" :loading="loading" @click="load">刷新</el-button>
          <el-button type="primary" :icon="Plus" @click="openCreate">新建凭证</el-button>
        </div>
      </div>

      <el-table :data="keys" v-loading="loading" stripe style="width: 100%">
        <el-table-column prop="name" label="凭证名称" min-width="160" />
        <el-table-column label="权限" width="220">
          <template #default="{ row }">
            <el-tag v-for="p in row.permissions" :key="p" size="small" style="margin-right: 6px">{{ p }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="180">
          <template #default="{ row }">{{ fmtTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column prop="lastUsed" label="最近使用" width="180">
          <template #default="{ row }">{{ row.lastUsed ? fmtTime(row.lastUsed) : '从未使用' }}</template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <span class="badge" :class="row.active ? 'success' : 'warning'">{{ row.active ? '活跃' : '已吊销' }}</span>
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
            ? `有效 · ${validateResult.name} · 权限：${(validateResult.permissions || []).join(', ') || '无'}`
            : `无效 · ${validateResult.reason || '未知原因'}` }}
        </span>
      </div>
    </div>

    <!-- 新建凭证 -->
    <el-dialog v-model="createVisible" title="新建访问凭证" width="480px">
      <el-form label-width="90px">
        <el-form-item label="凭证名称" required>
          <el-input v-model="createForm.name" placeholder="例如：运维巡检客户端" maxlength="64" />
        </el-form-item>
        <el-form-item label="权限">
          <el-checkbox-group v-model="createForm.permissions">
            <el-checkbox value="read">read（读取）</el-checkbox>
            <el-checkbox value="write">write（写入）</el-checkbox>
            <el-checkbox value="admin">admin（管理）</el-checkbox>
          </el-checkbox-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="creating" @click="handleCreate">创建</el-button>
      </template>
    </el-dialog>

    <!-- 明文展示（仅一次） -->
    <el-dialog v-model="keyVisible" title="凭证已创建（明文仅此一次展示）" width="560px">
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
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Plus, Delete, CopyDocument } from '@element-plus/icons-vue'
import { getApiKeys, createApiKey, revokeApiKey, validateApiKey } from '@/api'

const loading = ref(false)
const keys = ref([])

const createVisible = ref(false)
const creating = ref(false)
const createForm = reactive({ name: '', permissions: ['read'] })

const keyVisible = ref(false)
const createdKey = ref('')

const validateKeyText = ref('')
const validating = ref(false)
const validateResult = ref(null)

function fmtTime(t) {
  if (!t) return '-'
  try { return new Date(t).toLocaleString() } catch { return String(t) }
}

async function load() {
  loading.value = true
  try {
    const data = await getApiKeys()
    keys.value = Array.isArray(data) ? data : []
  } catch (e) {
    ElMessage.error('加载凭证列表失败：' + e.message)
  } finally {
    loading.value = false
  }
}

function openCreate() {
  createForm.name = ''
  createForm.permissions = ['read']
  createVisible.value = true
}

async function handleCreate() {
  if (!createForm.name.trim()) {
    ElMessage.warning('请输入凭证名称')
    return
  }
  creating.value = true
  try {
    const data = await createApiKey({
      name: createForm.name.trim(),
      permissions: createForm.permissions.length ? createForm.permissions : ['read']
    })
    createdKey.value = data?.key || ''
    createVisible.value = false
    keyVisible.value = true
    await load()
  } catch (e) {
    ElMessage.error('创建失败：' + e.message)
  } finally {
    creating.value = false
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
  try {
    await ElMessageBox.confirm(
      `确定吊销凭证「${row.name}」吗？吊销后使用该凭证的请求将立即失效。`,
      '吊销确认',
      { type: 'warning' }
    )
    await revokeApiKey(row.id)
    ElMessage.success(`凭证「${row.name}」已吊销`)
    await load()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('吊销失败：' + e.message)
  }
}

async function handleValidate() {
  const key = validateKeyText.value.trim()
  if (!key) {
    ElMessage.warning('请输入待校验的 Key 明文')
    return
  }
  validating.value = true
  try {
    validateResult.value = await validateApiKey(key)
  } catch (e) {
    validateResult.value = { valid: false, reason: e.message }
  } finally {
    validating.value = false
  }
}

onMounted(load)
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
