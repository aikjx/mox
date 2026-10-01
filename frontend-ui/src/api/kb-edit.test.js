import { describe, it, expect, vi, beforeEach } from 'vitest'
vi.mock('./http', () => ({ default: { put: vi.fn(), post: vi.fn() } }))
import http from './http'
import { kbSaveDocumentEdit } from './kb.api'
describe('atomic KB editing', () => {
  beforeEach(() => vi.clearAllMocks())
  it('sends one update with the captured version, content and note', async () => {
    http.put.mockResolvedValue({ document: { current_version: 'v3' } })
    const result = await kbSaveDocumentEdit({ id: 'kb/a', current_version: 'v2', title: 'T', content: 'body', tags: ['t'], version_note: ' reason ' })
    expect(http.put).toHaveBeenCalledTimes(1)
    expect(http.post).not.toHaveBeenCalled()
    expect(http.put).toHaveBeenCalledWith('/kb/documents/kb%2Fa', { title: 'T', content: 'body', category: undefined, tags: ['t'], expected_current_version: 'v2', version_note: 'reason' })
    expect(result.document.current_version).toBe('v3')
  })
  it('rejects summary-only rows without sending a request', () => {
    expect(() => kbSaveDocumentEdit({ id: 'kb-1', title: 'summary' })).toThrow()
    expect(http.put).not.toHaveBeenCalled()
  })
  it('propagates conflicts so the editor can keep unsaved content', async () => {
    const conflict = new Error('409 conflict')
    http.put.mockRejectedValue(conflict)
    await expect(kbSaveDocumentEdit({ id: 'kb-1', current_version: 'v1' })).rejects.toBe(conflict)
    expect(http.post).not.toHaveBeenCalled()
  })
})
