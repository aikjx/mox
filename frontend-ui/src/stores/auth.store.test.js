import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
// 拆 `../api/auth` → `../api/auth.api` 那次改造把 `src/api/index.js:27` 的 `export * from './auth.api'`
// 变成了 store 的实际入口（`auth.store.js:8` 取的是**命名**导出），替身只给 `default` 会让
// `authApi.getCurrentUser` 成 undefined ⇒ 三条用例全报 "is not a function"，断言根本没在演 store。
// 同一个 vi.fn 同时挂到命名与 default 两个口：store 走命名、本文件走 default，指的才是同一个假象。
const authApiMock = vi.hoisted(() => {
  const getCurrentUser = vi.fn()
  return { getCurrentUser, default: { getCurrentUser } }
})
vi.mock('../api/auth.api', () => authApiMock)
vi.mock('../api/http', () => ({ registerAuthTokenGetter: vi.fn() }))
import authApi from '../api/auth.api'
import { registerAuthTokenGetter } from '../api/http'
import { useAuthStore } from './auth.store'

describe('verified token session', () => {
  beforeEach(() => { localStorage.clear(); setActivePinia(createPinia()); vi.clearAllMocks() })
  it('verifies identity before establishing an in-memory session', async () => {
    authApi.getCurrentUser.mockResolvedValue({ data: { id: 'u1', enabled: true, username: 'tester' } })
    const store = useAuthStore()
    await store.loginWithToken(' test-token ')
    expect(authApi.getCurrentUser).toHaveBeenCalledWith('test-token')
    expect(store.isLoggedIn).toBe(true)
    expect(registerAuthTokenGetter.mock.calls[0][0]()).toBe('test-token')
    expect(localStorage.getItem('mox_access_token')).toBeNull()
    store.clearAuth()
    expect(registerAuthTokenGetter.mock.calls[0][0]()).toBe('')
  })
  it('does not log in when verification is rejected', async () => {
    authApi.getCurrentUser.mockRejectedValue(new Error('Unauthorized'))
    const store = useAuthStore()
    await expect(store.loginWithToken('invalid')).rejects.toThrow('Unauthorized')
    expect(store.isLoggedIn).toBe(false)
  })
  it('rejects an unavailable identity', async () => {
    authApi.getCurrentUser.mockResolvedValue({ id: 'u1', enabled: false })
    const store = useAuthStore()
    await expect(store.loginWithToken('disabled')).rejects.toThrow('当前身份不可用')
    expect(store.isLoggedIn).toBe(false)
  })
})
