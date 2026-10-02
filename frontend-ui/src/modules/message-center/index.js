import { defineModule } from '@/modules/_kernel/module-registry.js'

export const MESSAGE_CENTER_MODULE = defineModule({
  name: 'message-center', title: '消息中心', version: '1.0.0',
  endpoints: { send: { method: 'POST', path: '/enterprise/message/send' } },
  routes: [{
    path: '/messages/send', name: 'MessageSend', component: () => import('./SendMessagePage.vue'),
    meta: { title: '发送站内消息', module: 'message-center', layout: 'default', requiresAuth: true },
  }],
  nav: [{ key: 'message-send', label: '发送站内消息', icon: 'Message', path: '/messages/send', section: '工作空间', module: 'message-center' }],
}, import.meta.url)

export default MESSAGE_CENTER_MODULE
