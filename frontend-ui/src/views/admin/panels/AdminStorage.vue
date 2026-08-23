<template>
  <div class="adm-storage">
    <div class="grid grid-2">
      <div class="panel card-pad">
        <h3 class="section-title">存储状态</h3>
        <div class="info-grid">
          <div class="info-item"><span class="info-label">当前提供方</span><span class="info-value">{{ status.provider || '-' }}</span></div>
          <div class="info-item"><span class="info-label">引擎名称</span><span class="info-value">{{ status.name || '-' }}</span></div>
          <div class="info-item"><span class="info-label">实体总数</span><span class="info-value">{{ status.totalEntities ?? '-' }}</span></div>
        </div>
        <h4 class="sub-title">实体分布</h4>
        <el-table :data="status.entitiesByType || []" size="small" stripe max-height="240">
          <el-table-column prop="entity_type" label="实体类型" />
          <el-table-column prop="cnt" label="数量" width="100" />
        </el-table>
        <div class="feature-tags">
          <el-tag v-for="(v, k) in status.features" :key="k" size="small" :type="v ? 'success' : 'info'">
            {{ k }}：{{ v ? '启用' : '关闭' }}
          </el-tag>
        </div>
      </div>

      <div class="panel card-pad">
        <h3 class="section-title">提供方切换</h3>
        <el-radio-group v-model="selectedProvider" class="provider-group">
          <el-radio v-for="p in providers" :key="p.name || p.id" :value="p.name || p.id" class="provider-item">
            <div class="provider-info">
              <b>{{ p.name || p.id }}</b>
              <span class="provider-desc">{{ p.description || p.desc || p.type || '' }}</span>
            </div>
          </el-radio>
        </el-radio-group>
        <div class="switch-row">
          <el-button
            type="primary"
            :loading="switching"
            :disabled="!selectedProvider || selectedProvider === status.provider"
            @click="handleSwitch"
          >切换到此提供方</el-button>
          <span class="muted" v-if="selectedProvider === status.provider">当前即此提供方</span>
        </div>
      </div>
    </div>

    <div class="panel card-pad">
      <h3 class="section-title">已加载模块</h3>
      <el-table :data="modules" v-loading="loading" stripe style="width: 100%">
        <el-table-column prop="name" label="模块" min-width="160" />
        <el-table-column prop="description" label="描述" min-width="240" show-overflow-tooltip />
        <el-table-column prop="version" label="版本" width="100" />
        <el-table-column prop="routes" label="路由数" width="90" />
      </el-table>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getStorageProviders, switchStorageProvider, getStorageStatus, getModules } from '@/api'

const loading = ref(false)
const switching = ref(false)
const status = ref({})
const providers = ref([])
const modules = ref([])
const selectedProvider = ref('')

async function load() {
  loading.value = true
  try {
    const [st, ps, ms] = await Promise.all([
      getStorageStatus().catch(() => ({})),
      getStorageProviders().catch(() => []),
      getModules().catch(() => [])
    ])
    status.value = st || {}
    providers.value = Array.isArray(ps) ? ps : []
    modules.value = Array.isArray(ms) ? ms : []
    selectedProvider.value = status.value.provider || ''
  } finally {
    loading.value = false
  }
}

async function handleSwitch() {
  const target = selectedProvider.value
  try {
    await ElMessageBox.confirm(
      `确定将存储提供方切换为「${target}」吗？切换后数据读写将立即走新引擎。`,
      '切换确认',
      { type: 'warning' }
    )
    switching.value = true
    await switchStorageProvider(target)
    ElMessage.success(`已切换到 ${target}`)
    await load()
  } catch (e) {
    if (e !== 'cancel' && e?.message) ElMessage.error('切换失败：' + e.message)
  } finally {
    switching.value = false
  }
}

onMounted(load)
</script>

<style scoped>
.info-grid { display: grid; grid-template-columns: 1fr; gap: 10px; margin-bottom: 6px; }
.info-item { display: flex; justify-content: space-between; gap: 12px; font-size: 13px; }
.info-label { color: var(--text-3); }
.info-value { color: var(--text-1); font-weight: 600; }
.sub-title { font-size: 13px; font-weight: 600; color: var(--text-2); margin: 14px 0 8px; }
.feature-tags { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 12px; }
.provider-group { display: flex; flex-direction: column; gap: 10px; align-items: stretch; }
.provider-item { height: auto; margin-right: 0; }
.provider-info { display: flex; flex-direction: column; gap: 2px; padding: 4px 0; }
.provider-desc { font-size: 12px; color: var(--text-3); font-weight: 400; }
.switch-row { display: flex; align-items: center; gap: 12px; margin-top: 16px; }
.muted { font-size: 12px; color: var(--text-3); }
</style>
