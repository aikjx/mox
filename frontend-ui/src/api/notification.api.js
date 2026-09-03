// 通知中心 API - 通知列表、未读数、已读标记
import http from './http'

// 通知列表
export const getNotifications = (params) => http.get('/notifications', { params })

// 通知未读数
export const getNotificationUnreadCount = () => http.get('/notifications/unread-count')

// 标记单条通知已读
export const markNotificationRead = (id) =>
  http.put(`/notifications/${encodeURIComponent(id)}/read`)

// 标记全部通知已读
export const markAllNotificationsRead = () => http.put('/notifications/read-all')
