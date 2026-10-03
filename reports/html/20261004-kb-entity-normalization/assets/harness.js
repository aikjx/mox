import { createApp, ref, onMounted } from 'vue'
import { createPinia } from 'pinia'
import KnowledgeEntityLinks from '/frontend-ui/src/components/knowledge/KnowledgeEntityLinks.vue'
import { useAuthStore } from '/frontend-ui/src/stores/auth.store.js'
import { kbGetDocument } from '/frontend-ui/src/api/kb.api.js'
import '/frontend-ui/src/styles/global.css'
import '/frontend-ui/src/styles/themes/index.css'
createApp({
  components: { KnowledgeEntityLinks },
  setup() {
    const doc = ref(null), auth = useAuthStore()
    onMounted(async () => { doc.value = await kbGetDocument(window.__kbTarget) })
    window.__kbSwitch = token => {
      auth.accessToken = token
      auth.userInfo = { id: 'alice', tenant_id: 'b', roles: ['user'], enabled: true }
    }
    return { doc }
  },
  template: '<main style="max-width:1000px;margin:40px auto;padding:20px"><h1>知识实体归一验收</h1><KnowledgeEntityLinks v-if="doc" :document="doc" /></main>'
}).use(createPinia()).mount('#app')
