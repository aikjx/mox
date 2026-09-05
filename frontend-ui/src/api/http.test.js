import { afterEach, expect, it, vi } from 'vitest'
vi.mock('element-plus', () => ({ ElMessage: { error: vi.fn() } }))
import http, { registerAuthTokenGetter } from './http'

afterEach(() => registerAuthTokenGetter(null))
const capture = async config => ({ data: config.headers.toJSON(), status: 200, statusText: 'OK', headers: {}, config })
it('preserves the explicit token being verified', async () => {
  registerAuthTokenGetter(() => 'old-token')
  const headers = await http.get('/auth/me', { headers: { Authorization: 'Bearer candidate' }, adapter: capture })
  expect(headers.Authorization).toBe('Bearer candidate')
})
it('uses the active session and does not fall back after logout', async () => {
  let token = 'session-token'
  registerAuthTokenGetter(() => token)
  expect((await http.get('/test', { adapter: capture })).Authorization).toBe('Bearer session-token')
  token = ''
  expect((await http.get('/test', { adapter: capture })).Authorization).toBeUndefined()
})
