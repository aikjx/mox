import { describe, expect, it } from 'vitest'
import { reportedPhaseProgress } from './phase-progress.js'

const phases = [{ key: 'requirement' }, { key: 'develop' }]
describe('reported project progress', () => {
  it('keeps missing data unknown', () => {
    expect(reportedPhaseProgress(phases, null).map(item => item.progress)).toEqual([null, null])
  })
  it('preserves actual zero and reported completion', () => {
    const result = reportedPhaseProgress(phases, { phases: [
      { key: 'requirement', progress: 100, status: 'done' },
      { key: 'develop', progress: 0, status: 'active' }
    ] })
    expect(result.map(item => item.progress)).toEqual([100, 0])
  })
  it('does not invent progress for an unreported phase', () => {
    expect(reportedPhaseProgress(phases, { phases: [{ key: 'develop', progress: 20 }] })[0].progress).toBeNull()
  })
  it('rejects malformed and out of range progress', () => {
    for (const progress of [-1, 101, NaN, '65']) {
      expect(reportedPhaseProgress(phases, { phases: [{ key: 'develop', progress }] })[1].progress).toBeNull()
    }
  })
})
