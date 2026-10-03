// Auth store owns identity; the gateway owns tenant isolation.
import { reactive, watch, onScopeDispose, getCurrentScope } from 'vue'
import { useAuthStore } from '@/stores'
import { ENDPOINTS, EVENT_CONNECTION, createAllianceEventRecovery } from '@/modules/expert-alliance/contract'

// Pages own authoritative refresh; identity changes invalidate both stream and cursor.
export function useAllianceEventStream(options = {}) {
  const auth = useAuthStore()
  const endpointName = 'allianceEventStream'
  const connection = reactive({ state: EVENT_CONNECTION.IDLE, message: '' })
  const stream = createAllianceEventRecovery({
    ...options,
    url: ENDPOINTS[endpointName].path,
    getToken: () => auth.accessToken,
    onState: next => Object.assign(connection, next)
  })
  const unwatch = watch(
    () => [auth.accessToken, auth.userInfo?.id, auth.userInfo?.tenant_id],
    () => stream.stop(), { flush: 'sync' }
  )
  if (getCurrentScope()) onScopeDispose(() => { unwatch(); stream.stop() })
  return { ...stream, connection }
}

export const ALLIANCE_EVENT_STREAM_ENDPOINT = ENDPOINTS.allianceEventStream
