<template>
  <section v-if="store.canManage" class="webhooks" aria-labelledby="webhook-heading">
    <header class="webhooks-head">
      <h2 id="webhook-heading">事件订阅</h2>
      <el-button :loading="store.busy" @click="store.load()">读取订阅</el-button>
    </header>
    <p>仅投递到运维授权的接收地址。订阅保存成功不代表事件已经送达；目前最多尝试两次。</p>
    <el-alert v-if="store.error" type="error" :title="store.error" :closable="false" show-icon />
    <p v-if="store.notice" role="status">{{ store.notice }}</p>
    <el-form label-position="top" @submit.prevent="create">
      <el-form-item label="接收地址">
        <el-input v-model="url" maxlength="2048" placeholder="https://运维授权域名/接收路径" :disabled="store.busy" />
      </el-form-item>
      <el-form-item label="事件类型（不选择表示全部）">
        <el-checkbox-group v-model="eventTypes" :disabled="store.busy">
          <el-checkbox v-for="event in events" :key="event.value" :value="event.value">{{ event.label }}</el-checkbox>
        </el-checkbox-group>
      </el-form-item>
      <el-button native-type="submit" type="primary" :disabled="!url.trim() || store.busy">保存订阅</el-button>
    </el-form>
    <el-table v-if="store.loaded" :data="store.items" empty-text="本页暂无订阅" class="webhooks-table">
      <el-table-column prop="url" label="接收地址" min-width="240" />
      <el-table-column label="事件类型" min-width="180">
        <template #default="{ row }">{{ row.eventTypes.length ? row.eventTypes.map(type => events.find(event => event.value === type)?.label || type).join('、') : '全部事件' }}</template>
      </el-table-column>
      <el-table-column label="操作" width="90">
        <template #default="{ row }"><el-button type="danger" link :disabled="store.busy" @click="pendingDelete = row">删除</el-button></template>
      </el-table-column>
    </el-table>
    <el-pagination v-if="store.loaded && store.total > 20" :current-page="store.page" :page-size="20" :total="store.total" :disabled="store.busy" layout="prev, pager, next" @current-change="store.load" />
    <el-dialog :model-value="!!pendingDelete" title="删除事件订阅" width="min(520px, 92vw)" :close-on-click-modal="false" @close="pendingDelete = null">
      <p>确认停止向以下地址投递后续事件？已进入投递中的请求可能仍会完成。</p>
      <p class="webhooks-url">{{ pendingDelete?.url }}</p>
      <template #footer>
        <el-button :disabled="store.busy" @click="pendingDelete = null">取消</el-button>
        <el-button type="danger" :loading="store.busy" @click="remove">确认删除</el-button>
      </template>
    </el-dialog>
  </section>
</template>
<script setup>
import { ref, watch } from 'vue'
import { useAllianceWebhooksStore } from '@/modules/expert-alliance/store'
import { useAuthStore } from '@/stores'
import { WEBHOOK_EVENTS } from '@/modules/expert-alliance/contract'
const store = useAllianceWebhooksStore()
const auth = useAuthStore()
const url = ref('')
const eventTypes = ref([])
const pendingDelete = ref(null)
const events = WEBHOOK_EVENTS
watch(() => [auth.accessToken, auth.userInfo?.tenant_id, auth.userInfo?.id], () => {
  url.value = ''; eventTypes.value = []; pendingDelete.value = null
}, { flush: 'sync' })
async function create() {
  const result = await store.create({ url: url.value, eventTypes: eventTypes.value })
  if (result) { url.value = ''; eventTypes.value = [] }
}
async function remove() {
  if (!pendingDelete.value) return
  if (await store.remove(pendingDelete.value.id)) pendingDelete.value = null
}
</script>
<style scoped>
.webhooks { margin-bottom: 14px; padding: 14px; border: 1px solid var(--border); border-radius: var(--radius-md); background: var(--bg-card); }
.webhooks-head { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.webhooks h2 { margin: 0; font-size: 16px; color: var(--text-primary); }
.webhooks p { color: var(--text-secondary); font-size: 12px; line-height: 1.6; }
.webhooks-table { margin: 12px 0; }
.webhooks-url { overflow-wrap: anywhere; }
</style>
