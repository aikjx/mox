<template>
  <section class="kb-reference-panel" aria-label="文档实体引用" :aria-busy="busy">
    <h4>引用知识库实体</h4>
    <p>搜索当前可读文档中的实体。来源版本变化或权限撤回后，该引用不再展示。</p>
    <p v-if="error" role="alert">{{ error }}</p>
    <button type="button" :disabled="busy" @click="refresh">刷新关联</button>
    <form @submit.prevent="search">
      <label>实体名称 <input v-model="query" maxlength="256" placeholder="输入实体名称" /></label>
      <button type="submit" :disabled="busy || !query.trim()">搜索</button>
    </form>
    <ul aria-label="实体搜索结果">
      <li v-for="entity in results" :key="key(entity)">
        <span>{{ entity.name }} · {{ entity.type }} · 来源 {{ entity.source_doc_id }} / {{ entity.source_version }}</span>
        <button type="button" :disabled="busy || !snapshot" @click="mutate(entity, false)">引用 {{ entity.name }}</button>
      </li>
    </ul>
    <p v-if="searched && !results.length" role="status">没有匹配的可读实体</p>
    <ul aria-label="已关联实体">
      <li v-for="entity in snapshot?.linked_entities || []" :key="key(entity)">
        <span>{{ entity.name }} · 来源 {{ entity.source_doc_id }} / {{ entity.source_version }}</span>
        <button type="button" :disabled="busy" @click="mutate(entity, true)">移除 {{ entity.name }}</button>
      </li>
    </ul>
    <p v-if="snapshot && !snapshot.linked_entities.length" role="status">暂无可展示的实体引用</p>
  </section>
</template>

<script setup>
import { ref, watch, onBeforeUnmount } from 'vue'
import { useAuthStore } from '@/stores'
import { kbGetEntities, kbSearchEntities, kbLinkEntity, kbUnlinkEntity } from '@/api'
import { kbEntityMutation } from '@/utils'

const props = defineProps({ document: { type: Object, required: true } })
const auth = useAuthStore()
const query = ref(''), results = ref([]), snapshot = ref(null)
const busy = ref(false), error = ref(''), searched = ref(false)
let epoch = 0
const key = entity => JSON.stringify([entity.source_doc_id, entity.source_version, entity.id])
async function refresh() {
  const current = ++epoch
  const id = props.document?.id
  snapshot.value = null
  error.value = ''
  if (!id) return
  busy.value = true
  try {
    const data = await kbGetEntities(id)
    if (current === epoch) snapshot.value = data
  } catch (e) {
    if (current === epoch) error.value = e.message || '关联读取失败，请刷新'
  } finally { if (current === epoch) busy.value = false }
}
async function search() {
  if (busy.value || !query.value.trim()) return
  const current = ++epoch
  results.value = []
  busy.value = true
  error.value = ''
  try {
    const data = await kbSearchEntities({ q: query.value.trim(), limit: 20 })
    if (current === epoch) { results.value = data; searched.value = true }
  } catch (e) {
    if (current === epoch) error.value = e.message || '实体搜索失败'
  } finally { if (current === epoch) busy.value = false }
}
async function mutate(entity, remove) {
  if (busy.value || !snapshot.value) return
  const current = ++epoch
  busy.value = true
  error.value = ''
  try {
    const request = kbEntityMutation({ ...props.document, current_version: snapshot.value.current_version, acl_revision: snapshot.value.acl_revision }, entity, snapshot.value.links_revision)
    const call = remove ? kbUnlinkEntity : kbLinkEntity
    const data = await call(props.document.id, request)
    if (current === epoch) snapshot.value = data
  } catch (e) {
    if (current === epoch) { snapshot.value = null; error.value = `${e.message || '关联写入失败'}；请刷新后重试` }
  } finally { if (current === epoch) busy.value = false }
}
watch(() => [props.document?.id, props.document?.current_version, props.document?.acl_revision,
  auth.accessToken, auth.userId, auth.tenantId], () => {
  epoch++
  query.value = ''; results.value = []; searched.value = false; busy.value = false
  refresh()
}, { immediate: true, flush: 'sync' })
onBeforeUnmount(() => { epoch++ })
</script>

<style scoped>
.kb-reference-panel { margin-top: 1.5rem; padding: 1rem; border: 1px solid var(--border); border-radius: var(--radius-md); }
.kb-reference-panel form, .kb-reference-panel li { display: flex; flex-wrap: wrap; align-items: center; gap: .75rem; margin: .75rem 0; }
.kb-reference-panel ul { padding-left: 1.25rem; }
.kb-reference-panel input, .kb-reference-panel button { font: inherit; color: var(--text-primary); background: var(--bg-card); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: .4rem .6rem; }
.kb-reference-panel button { cursor: pointer; }
.kb-reference-panel button:disabled { opacity: .6; cursor: default; }
.kb-reference-panel :focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>
