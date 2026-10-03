// Invalidate an identity or reset; within it only the latest request for a key may commit.
export function createRequestFence() {
  let epoch = 0
  const versions = new Map()
  return {
    invalidate() { epoch++; versions.clear() },
    begin(key) {
      const owner = epoch
      const version = (versions.get(key) || 0) + 1
      versions.set(key, version)
      return () => owner === epoch && versions.get(key) === version
    }
  }
}
