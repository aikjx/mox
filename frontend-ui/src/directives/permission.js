/**
 * 权限指令集 - permission.js
 *
 * 提供 5 个 Vue 自定义指令，用于元素级别的权限控制：
 * - v-permission: 单个权限，没有则移除元素
 * - v-permission-any: 任一权限满足即显示
 * - v-permission-all: 所有权限满足才显示
 * - v-role: 角色控制
 * - v-role-any: 任一角色满足
 *
 * 用法示例：
 * <el-button v-permission="'system:user:add'">新增用户</el-button>
 * <el-button v-permission-any="['system:user:edit', 'system:user:view']">查看</el-button>
 * <el-button v-permission-all="['system:user:edit', 'system:user:view']">编辑</el-button>
 * <el-button v-role="'admin'">管理操作</el-button>
 * <el-button v-role-any="['admin', 'manager']">管理操作</el-button>
 */

import { usePermissionStore } from '@/stores/permission.store'

/**
 * 检查权限并决定是否移除元素
 * @param {HTMLElement} el DOM 元素
 * @param {boolean} hasAccess 是否有权限
 */
function _toggleElement(el, hasAccess) {
  if (hasAccess) {
    // 恢复显示
    if (el._permissionRemoved) {
      if (el._permissionParent && el._permissionNextSibling) {
        el._permissionParent.insertBefore(el, el._permissionNextSibling)
      } else if (el._permissionParent) {
        el._permissionParent.appendChild(el)
      }
      el._permissionRemoved = false
    }
    el.style.display = ''
  } else {
    // 移除元素（保存位置信息以便恢复）
    if (!el._permissionRemoved) {
      el._permissionParent = el.parentNode
      el._permissionNextSibling = el.nextSibling
      if (el.parentNode) {
        el.parentNode.removeChild(el)
      }
      el._permissionRemoved = true
    }
  }
}

/**
 * 创建权限指令的工厂函数
 * @param {Function} checkFn 权限检查函数，返回 boolean
 * @returns {object} Vue 指令对象
 */
function _createPermissionDirective(checkFn) {
  return {
    mounted(el, binding) {
      const permissionStore = usePermissionStore()
      const hasAccess = checkFn(permissionStore, binding.value)
      _toggleElement(el, hasAccess)
    },
    updated(el, binding) {
      // 值变化时重新检查
      if (binding.value !== binding.oldValue) {
        const permissionStore = usePermissionStore()
        const hasAccess = checkFn(permissionStore, binding.value)
        _toggleElement(el, hasAccess)
      }
    },
    unmounted(el) {
      // 清理存储的引用
      delete el._permissionParent
      delete el._permissionNextSibling
      delete el._permissionRemoved
    },
  }
}

// ===== 指令定义 =====

/**
 * v-permission - 单个权限控制
 * 用法：v-permission="'system:user:add'"
 */
export const permission = _createPermissionDirective((store, value) => {
  if (!value) return true
  return store.hasPermission(value)
})

/**
 * v-permission-any - 任一权限满足即显示
 * 用法：v-permission-any="['system:user:edit', 'system:user:view']"
 */
export const permissionAny = _createPermissionDirective((store, value) => {
  if (!value) return true
  const perms = Array.isArray(value) ? value : [value]
  return store.hasAnyPermission(perms)
})

/**
 * v-permission-all - 所有权限满足才显示
 * 用法：v-permission-all="['system:user:edit', 'system:user:view']"
 */
export const permissionAll = _createPermissionDirective((store, value) => {
  if (!value) return true
  const perms = Array.isArray(value) ? value : [value]
  return store.hasAllPermissions(perms)
})

/**
 * v-role - 单个角色控制
 * 用法：v-role="'admin'"
 */
export const role = _createPermissionDirective((store, value) => {
  if (!value) return true
  return store.hasRole(value)
})

/**
 * v-role-any - 任一角色满足即显示
 * 用法：v-role-any="['admin', 'manager']"
 */
export const roleAny = _createPermissionDirective((store, value) => {
  if (!value) return true
  const roles = Array.isArray(value) ? value : [value]
  return store.hasAnyRole(roles)
})

// ===== 批量注册 =====

/**
 * 安装所有权限指令
 * @param {import('vue').App} app Vue 应用实例
 */
export function setupPermissionDirectives(app) {
  app.directive('permission', permission)
  app.directive('permission-any', permissionAny)
  app.directive('permission-all', permissionAll)
  app.directive('role', role)
  app.directive('role-any', roleAny)
}

export default {
  install(app) {
    setupPermissionDirectives(app)
  },
}
