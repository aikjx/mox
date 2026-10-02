// Auth store owns identity; the gateway owns tenant isolation.
import { watch, onScopeDispose, getCurrentScope } from 'vue'
import { useAuthStore } from '@/stores'
import { ENDPOINTS, createAllianceEventStream } from '@/modules/expert-alliance/contract'

// Caller refreshes authoritative lists on onOpen/onGap. No replay or auto-subscription.
export function useAllianceEventStream(options = {}) {
  const auth = useAuthStore()
  const endpointName = 'allianceEventStream'
  const stream = createAllianceEventStream({
    ...options,
    url: ENDPOINTS[endpointName].path,
    getToken: () => auth.accessToken
  })
  const unwatch = watch(
    () => [auth.accessToken, auth.userInfo?.id, auth.userInfo?.tenant_id],
    () => stream.stop(), { flush: 'sync' }
  )
  if (getCurrentScope()) onScopeDispose(() => { unwatch(); stream.stop() })
  return stream
}

export const ALLIANCE_EVENT_STREAM_ENDPOINT = ENDPOINTS.allianceEventStream
