import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

// These local normalization actions need neither a request nor an HTTP replacement.

import { usePermissionStore } from './permission.store'

describe('permission store 角色归一化与守卫判定', () => {
  beforeEach(() => {
    localStorage.clear()
    setActivePinia(createPinia())
  })

  it('后端 roles 是 [{id,code,name}] 对象数组时，归一化为 code 字符串数组', () => {
    const store = usePermissionStore()
    store.setRoles([
      { id: 'r1', code: 'tenant_admin', name: '租户管理员' },
      { id: 'r2', code: 'normal_user', name: '普通员工' }
    ])
    expect(store.roles).toEqual(['tenant_admin', 'normal_user'])
  })

  it('兼容历史/旧缓存里直接给字符串数组', () => {
    const store = usePermissionStore()
    store.setRoles(['super_admin', 'viewer'])
    expect(store.roles).toEqual(['super_admin', 'viewer'])
  })

  it('对象里缺 code 时被过滤，不把对象塞进角色列表', () => {
    const store = usePermissionStore()
    store.setRoles([{ id: 'x', name: '无码角色' }, 'admin'])
    expect(store.roles).toEqual(['admin'])
  })

  it('tenant_admin / super_admin 都判为 isAdmin（与后端 role_code 对齐）', () => {
    const s1 = usePermissionStore(); s1.setRoles(['tenant_admin'])
    expect(s1.isAdmin).toBe(true)
    const s2 = usePermissionStore(); s2.setRoles(['super_admin'])
    expect(s2.isAdmin).toBe(true)
  })

  it('普通用户不判 isAdmin，且 hasAnyRole 对管理角色码为假', () => {
    const store = usePermissionStore()
    store.setRoles(['normal_user'])
    expect(store.isAdmin).toBe(false)
    expect(store.hasAnyRole(['super_admin', 'tenant_admin'])).toBe(false)
  })

  it('归一化后 hasAnyRole 能命中真实角色码（路由守卫 requiresRole 生效）', () => {
    const store = usePermissionStore()
    store.setRoles([{ id: 'r1', code: 'tenant_admin', name: '租户管理员' }])
    expect(store.hasAnyRole(['super_admin', 'tenant_admin'])).toBe(true)
  })
})
