/**
 * 权限 Store - permission.store.js
 *
 * 企业级权限管理核心层：
 * - state: menus, permissions, roles, routes, deptId, dataScope, customDeptIds, loaded
 * - getters: hasPermission, hasAnyPermission, hasAllPermissions, hasRole, hasAnyRole,
 *            isAdmin, canAccessDept, dataScopeLabel
 * - actions: loadPermissions, generateRoutes, filterMenusByPermission,
 *            setPermissions, setRoles, setDataScope, reset
 *
 * 数据权限范围（dataScope）：
 * - all: 全部数据权限
 * - dept: 本部门数据权限
 * - deptAndBelow: 本部门及以下数据权限
 * - self: 仅本人数据权限
 * - custom: 自定义数据权限
 */

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import http from '@/api/http'
import { NAV_MODULES, NAV_GROUPS } from '@/constants/nav.config'

// ===== 常量 =====

const PERMISSIONS_KEY = 'mox-perm-permissions'
const ROLES_KEY = 'mox-perm-roles'
const DATA_SCOPE_KEY = 'mox-perm-data-scope'
const DEPT_ID_KEY = 'mox-perm-dept-id'
const CUSTOM_DEPT_IDS_KEY = 'mox-perm-custom-dept-ids'

// 数据权限范围枚举
export const DATA_SCOPE = {
  ALL: 'all',
  DEPT: 'dept',
  DEPT_AND_BELOW: 'deptAndBelow',
  SELF: 'self',
  CUSTOM: 'custom',
}

// 数据权限中文名称映射
const DATA_SCOPE_LABELS = {
  [DATA_SCOPE.ALL]: '全部数据权限',
  [DATA_SCOPE.DEPT]: '本部门数据权限',
  [DATA_SCOPE.DEPT_AND_BELOW]: '本部门及以下数据权限',
  [DATA_SCOPE.SELF]: '仅本人数据权限',
  [DATA_SCOPE.CUSTOM]: '自定义数据权限',
}

// 超级管理员角色标识
const ADMIN_ROLE = 'admin'
const SUPER_ADMIN_ROLE = 'super_admin'

// ===== 存储辅助函数 =====

function _safeGet(key, fallback) {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return fallback
    if (typeof fallback === 'object' && fallback !== null) {
      return JSON.parse(raw)
    }
    return raw
  } catch {
    return fallback
  }
}

function _safeSet(key, value) {
  try {
    if (typeof value === 'object' && value !== null) {
      localStorage.setItem(key, JSON.stringify(value))
    } else {
      localStorage.setItem(key, String(value))
    }
  } catch (e) {
    // dev only: 权限存储失败属内部工具错误
    console.warn('[permissionStore] 存储失败:', key, e)
  }
}

function _safeRemove(key) {
  try {
    localStorage.removeItem(key)
  } catch {}
}

// ===== Store 定义 =====

export const usePermissionStore = defineStore('permission', () => {
  // ===== State =====

  /** 动态菜单树（根据权限过滤后的侧边栏菜单） */
  const menus = ref([])

  /** 权限标识列表（如 'system:user:add', 'system:role:edit'） */
  const permissions = ref(_safeGet(PERMISSIONS_KEY, []))

  /** 当前用户角色列表 */
  const roles = ref(_safeGet(ROLES_KEY, []))

  /** 可访问路由（动态生成） */
  const routes = ref([])

  /** 当前用户所属部门ID */
  const deptId = ref(_safeGet(DEPT_ID_KEY, ''))

  /** 数据权限范围（all/dept/deptAndBelow/self/custom） */
  const dataScope = ref(_safeGet(DATA_SCOPE_KEY, DATA_SCOPE.SELF))

  /** 自定义数据权限的部门ID列表 */
  const customDeptIds = ref(_safeGet(CUSTOM_DEPT_IDS_KEY, []))

  /** 权限是否已加载 */
  const loaded = ref(false)

  /** 加载中状态 */
  const loading = ref(false)

  // ===== Getters =====

  /** 是否超级管理员 */
  const isAdmin = computed(() => {
    return roles.value.includes(ADMIN_ROLE) || roles.value.includes(SUPER_ADMIN_ROLE)
  })

  /** 数据权限中文名称 */
  const dataScopeLabel = computed(() => {
    return DATA_SCOPE_LABELS[dataScope.value] || '未知数据权限'
  })

  /**
   * 检查是否有单个权限
   * @param {string} perm 权限标识
   * @returns {boolean}
   */
  function hasPermission(perm) {
    if (!perm) return true
    if (isAdmin.value) return true
    return permissions.value.includes(perm)
  }

  /**
   * 检查是否有任一权限
   * @param {string[]} perms 权限标识列表
   * @returns {boolean}
   */
  function hasAnyPermission(perms) {
    if (!perms || !Array.isArray(perms) || perms.length === 0) return true
    if (isAdmin.value) return true
    return perms.some(p => permissions.value.includes(p))
  }

  /**
   * 检查是否有所有权限
   * @param {string[]} perms 权限标识列表
   * @returns {boolean}
   */
  function hasAllPermissions(perms) {
    if (!perms || !Array.isArray(perms) || perms.length === 0) return true
    if (isAdmin.value) return true
    return perms.every(p => permissions.value.includes(p))
  }

  /**
   * 检查是否有某个角色
   * @param {string} role 角色名
   * @returns {boolean}
   */
  function hasRole(role) {
    if (!role) return true
    return roles.value.includes(role)
  }

  /**
   * 检查是否有任一角色
   * @param {string[]} roleList 角色列表
   * @returns {boolean}
   */
  function hasAnyRole(roleList) {
    if (!roleList || !Array.isArray(roleList) || roleList.length === 0) return true
    return roleList.some(r => roles.value.includes(r))
  }

  /**
   * 能否访问某部门数据（基于数据权限范围）
   * @param {string|number} targetDeptId 目标部门ID
   * @returns {boolean}
   */
  function canAccessDept(targetDeptId) {
    // 超级管理员可访问所有数据
    if (isAdmin.value) return true
    // 全部数据权限
    if (dataScope.value === DATA_SCOPE.ALL) return true
    // 未设置部门ID时，仅本人数据
    if (!targetDeptId) return dataScope.value === DATA_SCOPE.SELF
    // 本部门数据权限
    if (dataScope.value === DATA_SCOPE.DEPT) {
      return String(targetDeptId) === String(deptId.value)
    }
    // 本部门及以下（简化判断：实际需结合部门树）
    if (dataScope.value === DATA_SCOPE.DEPT_AND_BELOW) {
      return String(targetDeptId) === String(deptId.value)
    }
    // 自定义数据权限
    if (dataScope.value === DATA_SCOPE.CUSTOM) {
      return customDeptIds.value.some(id => String(id) === String(targetDeptId))
    }
    // 仅本人数据权限
    if (dataScope.value === DATA_SCOPE.SELF) {
      return false
    }
    return false
  }

  // ===== Actions =====

  /**
   * 设置权限列表
   * @param {string[]} perms 权限标识数组
   */
  function setPermissions(perms) {
    permissions.value = Array.isArray(perms) ? perms : []
    _safeSet(PERMISSIONS_KEY, permissions.value)
  }

  /**
   * 设置角色列表
   * @param {string[]} roleList 角色数组
   */
  function setRoles(roleList) {
    roles.value = Array.isArray(roleList) ? roleList : []
    _safeSet(ROLES_KEY, roles.value)
  }

  /**
   * 设置数据权限
   * @param {string} scope 数据权限范围
   * @param {string[]} [deptIds] 自定义部门ID列表（custom 模式下使用）
   */
  function setDataScope(scope, deptIds) {
    dataScope.value = scope || DATA_SCOPE.SELF
    _safeSet(DATA_SCOPE_KEY, dataScope.value)

    if (scope === DATA_SCOPE.CUSTOM && deptIds) {
      customDeptIds.value = Array.isArray(deptIds) ? deptIds : []
      _safeSet(CUSTOM_DEPT_IDS_KEY, customDeptIds.value)
    }
  }

  /**
   * 设置部门ID
   * @param {string|number} id 部门ID
   */
  function setDeptId(id) {
    deptId.value = id || ''
    _safeSet(DEPT_ID_KEY, deptId.value)
  }

  /**
   * 递归过滤菜单（按权限）
   * @param {Array} menuList 菜单列表
   * @returns {Array} 过滤后的菜单
   */
  function filterMenusByPermission(menuList) {
    if (!menuList || !Array.isArray(menuList)) return []

    return menuList
      .filter(menu => {
        // 没有权限要求的菜单直接显示
        if (!menu.permission && !menu.perms) return true
        // 超级管理员显示所有菜单
        if (isAdmin.value) return true

        const permCheck = menu.permission || menu.perms
        if (Array.isArray(permCheck)) {
          return hasAnyPermission(permCheck)
        }
        return hasPermission(permCheck)
      })
      .map(menu => {
        // 递归过滤子菜单
        if (menu.children && menu.children.length > 0) {
          const filteredChildren = filterMenusByPermission(menu.children)
          return { ...menu, children: filteredChildren }
        }
        return { ...menu }
      })
      .filter(menu => {
        // 有子菜单但过滤后为空，且自身无路径的菜单移除
        if (menu.children && menu.children.length === 0 && !menu.path) {
          return false
        }
        return true
      })
  }

  /**
   * 根据权限生成可访问路由
   * 基于 NAV_MODULES 和权限列表生成动态路由
   * @returns {Array} 可访问的路由配置
   */
  function generateRoutes() {
    // 从导航配置中过滤出有权限的模块
    const accessibleModules = NAV_MODULES.filter(m => {
      if (!m.permission) return true
      if (isAdmin.value) return true
      return hasPermission(m.permission)
    })

    routes.value = accessibleModules
    return accessibleModules
  }

  /**
   * 生成动态菜单树（分组结构）
   * 基于 NAV_GROUPS 和权限过滤生成侧边栏菜单
   * @returns {Array} 带分组的菜单树
   */
  function generateMenus() {
    const moduleSet = new Set(
      NAV_MODULES
        .filter(m => {
          if (!m.permission) return true
          if (isAdmin.value) return true
          return hasPermission(m.permission)
        })
        .map(m => m.key)
    )

    // 过滤分组内的模块
    const filteredGroups = NAV_GROUPS
      .map(g => ({
        ...g,
        items: g.items.filter(key => moduleSet.has(key))
      }))
      .filter(g => g.items.length > 0)

    menus.value = filteredGroups
    return filteredGroups
  }

  /**
   * 从后端加载权限/角色/菜单
   * 调用 /api/system/permissions
   * @returns {Promise<object>} 权限数据
   */
  async function loadPermissions() {
    loading.value = true

    try {
      // 调用后端权限接口，失败时正确处理错误并设置空权限状态
      const data = await http.get('/system/permissions')

      // 设置角色
      if (data.roles && Array.isArray(data.roles)) {
        setRoles(data.roles)
      }

      // 设置权限
      if (data.permissions && Array.isArray(data.permissions)) {
        setPermissions(data.permissions)
      }

      // 设置部门信息
      if (data.deptId !== undefined && data.deptId !== null) {
        setDeptId(data.deptId)
      }

      // 设置数据权限
      if (data.dataScope) {
        setDataScope(data.dataScope, data.customDeptIds)
      }

      // 生成动态菜单和路由
      generateMenus()
      generateRoutes()

      loaded.value = true
      return data
    } catch (e) {
      console.error('[permissionStore] 加载权限失败:', e?.message)
      // API 失败：设置空权限状态，用户看到无权限提示而非假权限
      setRoles([])
      setPermissions([])
      generateMenus()
      generateRoutes()
      loaded.value = true
      ElMessage.error('权限加载失败：' + (e?.message || '未知错误'))
      throw e
    } finally {
      loading.value = false
    }
  }

  /**
   * 重置权限（登出时调用）
   */
  function reset() {
    menus.value = []
    permissions.value = []
    roles.value = []
    routes.value = []
    deptId.value = ''
    dataScope.value = DATA_SCOPE.SELF
    customDeptIds.value = []
    loaded.value = false
    loading.value = false

    _safeRemove(PERMISSIONS_KEY)
    _safeRemove(ROLES_KEY)
    _safeRemove(DATA_SCOPE_KEY)
    _safeRemove(DEPT_ID_KEY)
    _safeRemove(CUSTOM_DEPT_IDS_KEY)
  }

  // ===== 返回 =====

  return {
    // State
    menus,
    permissions,
    roles,
    routes,
    deptId,
    dataScope,
    customDeptIds,
    loaded,
    loading,
    // Getters
    isAdmin,
    dataScopeLabel,
    // Check Methods
    hasPermission,
    hasAnyPermission,
    hasAllPermissions,
    hasRole,
    hasAnyRole,
    canAccessDept,
    // Actions
    loadPermissions,
    generateRoutes,
    generateMenus,
    filterMenusByPermission,
    setPermissions,
    setRoles,
    setDataScope,
    setDeptId,
    reset,
  }
})
