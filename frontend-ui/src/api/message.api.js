import http from './http'

// A retry must reuse the frozen command and its key; transport retries are explicit.
export const sendInAppMessage = (attempt) => http.post('/enterprise/message/send', attempt.payload, {
  headers: { 'Idempotency-Key': attempt.key }, silent: true, _retry: 0,
})
