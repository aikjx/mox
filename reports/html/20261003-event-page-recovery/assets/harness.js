import { createApp, computed, onMounted, watch } from 'vue'
import { createPinia } from 'pinia'
import EventConnectionStatus from '/frontend-ui/src/modules/expert-alliance/components/EventConnectionStatus.vue'
import { useAllianceEventStream } from '/frontend-ui/src/modules/expert-alliance/composables/useAllianceEventStream.js'
import { useAllianceExpertsStore } from '/frontend-ui/src/modules/expert-alliance/store/alliance-experts.store.js'
import { useAuthStore } from '/frontend-ui/src/stores/auth.store.js'
import '/frontend-ui/src/styles/themes/index.css'

createApp({
  components: { EventConnectionStatus },
  setup() {
    const auth = useAuthStore()
    const store = useAllianceExpertsStore()
    const stream = useAllianceEventStream({
      onEvent: (kind,envelope) => store.applyRegistryEvent(kind,envelope),
      refresh: async () => {
        await Promise.all([store.loadExperts(),store.loadStats()])
        if (store.error.list || store.error.stats) throw new Error(store.error.list || store.error.stats)
      }
    })
    const otherTenantStats = []
    watch(() => store.stats?.totalExperts, count => {
      if (auth.tenantId === 'other' && count !== undefined && count !== null) otherTenantStats.push(count)
    }, { flush: 'sync' })
    window.__recoveryTest = {
      loadStats: () => store.loadStats(),
      otherTenantStats,
      switchIdentity: token => {
        auth.accessToken=token
        auth.userInfo={id:'recovery-user',tenant_id:'other',roles:['tenant_admin'],enabled:true}
      }
    }
    onMounted(()=>stream.start())
    return { stream, count:computed(()=>store.experts.length), stats:computed(()=>store.stats?.totalExperts), tenant:computed(()=>auth.tenantId) }
  },
  template: '<main style="max-width:800px;margin:40px auto;padding:20px"><h1>专家事件恢复验收</h1><EventConnectionStatus :connection="stream.connection" @reconnect="stream.start()"/><p data-testid="tenant">当前租户：{{tenant}}</p><p data-testid="count">已读取专家：{{count}}</p><p data-testid="stats">统计专家：{{stats ?? "未读取"}}</p></main>'
}).use(createPinia()).mount('#app')
