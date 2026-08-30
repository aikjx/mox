// Melody2Score 旋律转谱 API
import http from './http'

export const melodyHealth = () => http.get('/melody2score/health')
export const melodyStatus = () => http.get('/melody2score/status')
export const melodySamples = () => http.get('/melody2score/samples')
export const melodyRecognize = (formData) => http.post('/melody2score/recognize', formData, {
  headers: { 'Content-Type': 'multipart/form-data' },
  timeout: 120000
})
export const melodyRecognizeSample = (formData) => http.post('/melody2score/recognize-sample', formData, {
  headers: { 'Content-Type': 'multipart/form-data' },
  timeout: 120000
})
export const melodyRecognizeRecord = (payload) => http.post('/melody2score/recognize-record', payload, { timeout: 120000 })
export const melodyExportSheet = (payload) => http.post('/melody2score/export-sheet', payload, { timeout: 60000 })
export const melodySaveReport = (payload) => http.post('/melody2score/save-report', payload, { timeout: 30000 })
