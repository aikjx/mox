import { defineStore } from 'pinia'
import { computed, ref, watch } from 'vue'
import { useAuthStore } from '@/stores'
import { allianceApi } from '@/modules/expert-alliance/api'

export const useAllianceWebhooksStore = defineStore('allianceWebhooks', () => {
  const auth = useAuthStore()
  const items = ref([])
  const total = ref(0)
  const page = ref(1)
  const busy = ref(false)
  const loaded = ref(false)
  const error = ref('')
  const notice = ref('')
  const canManage = computed(() => !!auth.accessToken && auth.roles.some(role => ['super_admin', 'tenant_admin'].includes(role)))
  let epoch = 0
  watch(() => [auth.accessToken, auth.userInfo?.id, auth.userInfo?.tenant_id, auth.roles.join(',')], () => {
    epoch++
    items.value = []
    total.value = 0
    page.value = 1
    busy.value = false
    loaded.value = false
    error.value = ''
    notice.value = ''
  }, { flush: 'sync' })

  async function refresh(current) {
    const result = await allianceApi.listWebhooks(page.value)
    if (current !== epoch) return
    items.value = result.items
    total.value = result.total
    loaded.value = true
  }

  async function run(operation) {
    if (busy.value || !canManage.value) return null
    const current = epoch
    busy.value = true
    error.value = ''
    notice.value = ''
    try { return await operation(current) } catch (failure) {
      if (current === epoch) error.value = failure?.message || '订阅请求失败，请刷新后核对'
      return null
    } finally { if (current === epoch) busy.value = false }
  }

  async function load(nextPage = page.value) {
    return run(async current => {
      page.value = nextPage
      await refresh(current)
      return current === epoch
    })
  }

  async function create(input) {
    return run(async current => {
      const result = await allianceApi.createWebhook(input)
      if (current !== epoch) return null
      notice.value = '订阅已保存。投递结果以接收方记录为准。'
      page.value = 1
      // 已确认的提交与后续列表读取分别呈现，避免误导用户重复创建。
      try { await refresh(current) } catch (failure) {
        if (current === epoch) error.value = `订阅已保存，列表刷新失败：${failure.message}`
      }
      return current === epoch ? result : null
    })
  }

  async function remove(id) {
    return run(async current => {
      await allianceApi.deleteWebhook(id)
      if (current !== epoch) return null
      items.value = items.value.filter(item => item.id !== id)
      total.value = Math.max(0, total.value - 1)
      notice.value = '订阅已删除。已进入投递中的请求可能仍会完成。'
      page.value = 1
      try { await refresh(current) } catch (failure) {
        if (current === epoch) error.value = `订阅已删除，列表刷新失败：${failure.message}`
      }
      return current === epoch
    })
  }
  return { items, total, page, busy, loaded, error, notice, canManage, load, create, remove }
})
