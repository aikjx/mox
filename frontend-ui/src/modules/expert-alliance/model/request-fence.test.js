import { describe, it, expect } from 'vitest'
import { createRequestFence } from './request-fence'

describe('request ownership', () => {
  it('a late read cannot overwrite a newer successful read', async () => {
    const fence = createRequestFence()
    let value, release
    const first = fence.begin('stats')
    const pending = new Promise(resolve => { release = resolve }).then(next => { if (first()) value = next })
    const second = fence.begin('stats')
    if (second()) value = 'new'
    release('old'); await pending
    expect(value).toBe('new')
  })
  it('identity invalidation rejects prior results even if the new request uses the same key', () => {
    const fence = createRequestFence()
    const old = fence.begin('history')
    fence.invalidate()
    const fresh = fence.begin('history')
    expect(old()).toBe(false)
    expect(fresh()).toBe(true)
  })
  it('independent outlets do not cancel each other', () => {
    const fence = createRequestFence()
    const list = fence.begin('list'), stats = fence.begin('stats')
    expect(list()).toBe(true)
    expect(stats()).toBe(true)
  })
})
