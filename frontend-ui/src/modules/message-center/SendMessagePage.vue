<script setup>
import { reactive, ref, shallowRef, watch, onBeforeUnmount } from 'vue'
import { useAuthStore } from '@/stores'
import { sendInAppMessage } from '@/api'
import { createSendAttempt, readSendReceipt, isDefinitiveSendRejection } from './contract/send.js'

const auth = useAuthStore()
const draft = reactive({ receivers: '', title: '', content: '', type: 'custom', priority: 'normal' })
const phase = ref('idle')
const error = ref('')
const receipt = ref('')
const attempt = shallowRef(null)
let generation = 0

function reset() {
  generation++
  Object.assign(draft, { receivers: '', title: '', content: '', type: 'custom', priority: 'normal' })
  attempt.value = null
  error.value = ''
  receipt.value = ''
  phase.value = 'idle'
}

async function send(retry = false) {
  if (phase.value === 'sending' || phase.value === 'sent' || (phase.value === 'unknown' && !retry)) return
  const scope = generation
  try {
    if (!retry) attempt.value = createSendAttempt(draft)
    if (!attempt.value) return
  } catch (e) { error.value = e.message; return }
  phase.value = 'sending'
  error.value = ''
  try {
    const data = await sendInAppMessage(attempt.value)
    if (scope !== generation) return
    receipt.value = readSendReceipt(data)
    phase.value = 'sent'
  } catch (e) {
    if (scope !== generation) return
    phase.value = isDefinitiveSendRejection(e) ? 'rejected' : 'unknown'
    error.value = e.message || '未确认发送结果'
  }
}

watch(() => [auth.accessToken, auth.userId, auth.tenantId], reset, { flush: 'sync' })
onBeforeUnmount(() => { generation++ })
</script>

<template>
  <main class="message-send">
    <h1>发送站内消息</h1>
    <p>发送至当前租户的真实用户。跨用户发送权限由服务端核验，已提交消息可在收件人的通知中心阅读。</p>
    <el-alert v-if="phase === 'sent'" title="全部收件人站内箱已提交" type="success" :closable="false">
      <p>消息 ID：{{ receipt }}</p>
      <p>此回执仅代表站内提交。</p>
    </el-alert>
    <el-alert v-if="error" :title="error" type="error" :closable="false" role="alert" />
    <el-alert v-if="phase === 'unknown'" title="发送结果尚未确认" type="warning" :closable="false">
      <p>保留当前内容，使用“确认原请求”重试。服务端会按原发送标识去重。离开或刷新页面将丢失本地重试信息，请先确认结果。</p>
    </el-alert>
    <fieldset :disabled="['sending', 'sent', 'unknown'].includes(phase)">
      <legend>站内消息内容</legend>
      <el-form label-position="top" :disabled="['sending', 'sent', 'unknown'].includes(phase)" @submit.prevent="send()">
        <el-form-item label="收件人 ID（逗号或换行分隔）">
          <el-input v-model="draft.receivers" type="textarea" :rows="2" aria-label="收件人 ID" placeholder="输入实际用户 ID" />
          <el-button text type="primary" :disabled="!auth.userId" @click="draft.receivers = auth.userId">发送给自己</el-button>
        </el-form-item>
        <el-form-item label="标题"><el-input v-model="draft.title" maxlength="200" show-word-limit aria-label="消息标题" /></el-form-item>
        <el-form-item label="正文"><el-input v-model="draft.content" type="textarea" :rows="8" maxlength="10000" show-word-limit aria-label="消息正文" /></el-form-item>
        <div class="options">
          <el-form-item label="类型">
            <el-select v-model="draft.type" aria-label="消息类型">
              <el-option label="普通消息" value="custom" /><el-option label="任务" value="task" />
              <el-option label="告警" value="alert" /><el-option label="系统通知" value="system" />
            </el-select>
          </el-form-item>
          <el-form-item label="优先级">
            <el-select v-model="draft.priority" aria-label="优先级">
              <el-option label="低" value="low" /><el-option label="普通" value="normal" />
              <el-option label="高" value="high" /><el-option label="紧急" value="urgent" />
            </el-select>
          </el-form-item>
        </div>
        <el-button type="primary" native-type="submit">提交站内消息</el-button>
      </el-form>
    </fieldset>
    <p v-if="phase === 'sending'" aria-live="polite">正在确认发送结果…</p>
    <el-button v-if="phase === 'unknown'" type="primary" @click="send(true)">确认原请求</el-button>
    <el-button v-if="phase === 'sent'" @click="reset">新建消息</el-button>
    <p>当前只开放站内渠道。消息正文以文本展示；邮件、短信、定时发送尚未开放。</p>
  </main>
</template>

<style scoped>
.message-send { max-width: 900px; margin: 0 auto; padding: 24px; color: var(--text-primary); }
fieldset { margin: 20px 0; padding: 20px; border: 1px solid var(--border); border-radius: 8px; min-width: 0; }
.options { display: flex; flex-wrap: wrap; gap: 24px; }
.options > * { flex: 1 1 200px; }
.el-alert { margin-top: 12px; }
p { overflow-wrap: anywhere; }
@media (max-width: 600px) { .message-send { padding: 12px; } fieldset { padding: 12px; } }
</style>
