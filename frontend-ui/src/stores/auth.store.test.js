import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
vi.mock('../api/auth', () => ({ default: { getCurrentUser: vi.fn() } }))
vi.mock('../api/http', () => ({ registerAuthTokenGetter: vi.fn() }))
import authApi from '../api/auth'
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
