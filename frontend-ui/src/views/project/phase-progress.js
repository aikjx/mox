// Only reported progress is displayed. Missing data is unknown, not an estimate.
export function reportedPhaseProgress(phases, snapshot) {
  return phases.map(phase => {
    const reported = Array.isArray(snapshot?.phases)
      ? snapshot.phases.find(item => item.key === phase.key || item.name === phase.key)
      : null
    const valid = typeof reported?.progress === 'number'
      && Number.isFinite(reported.progress) && reported.progress >= 0 && reported.progress <= 100
    return {
      ...phase,
      progress: valid ? reported.progress : null,
      status: reported?.status || 'unknown'
    }
  })
}
