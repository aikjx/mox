import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useUserStore = defineStore('user', () => {
  const username = ref('admin')
  const role = ref('super_admin')
  const permissions = ref([])

  function setUser(user) {
    username.value = user.username || user.name || 'admin'
    role.value = user.role || 'super_admin'
    permissions.value = user.permissions || []
  }

  function hasPermission(key) {
    if (role.value === 'super_admin') return true
    return permissions.value.includes(key)
  }

  return { username, role, permissions, setUser, hasPermission }
})
