import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useSystemStore = defineStore('system', () => {
  const version = ref('v3.0.0')
  const uptime = ref(0)
  const activeUsers = ref(0)
  const systemStatus = ref('operational')

  function setSystemInfo(info) {
    version.value = info.version || version.value
    uptime.value = info.uptime || 0
    activeUsers.value = info.activeUsers || 0
    systemStatus.value = info.status || 'operational'
  }

  return { version, uptime, activeUsers, systemStatus, setSystemInfo }
})
