// 消息操作 composable - 从 MessageBubble 提取的通用业务逻辑
import { ref, watch, onBeforeUnmount } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  stableMsgId,
  copyTextUniversal,
  mdToPlainText,
  formatTime,
} from '@/utils/message.utils'

/**
 * 消息评分（喜欢/不喜欢）
 */
export function useMessageRating(props, emit) {
  const rating = ref(null)

  function persistRating(val) {
    const sid = stableMsgId(props.msg)
    try {
      if (val) localStorage.setItem('ous_msg_rating_' + sid, val)
      else localStorage.removeItem('ous_msg_rating_' + sid)
    } catch (_) {}
  }

  function loadRating() {
    const sid = stableMsgId(props.msg)
    try {
      rating.value = localStorage.getItem('ous_msg_rating_' + sid) || null
    } catch (_) {
      rating.value = null
    }
  }

  function toggleRating(kind) {
    if (rating.value === kind) {
      rating.value = null
      persistRating(null)
    } else {
      rating.value = kind
      persistRating(kind)
    }
    emit?.('rate', props.msg, rating.value)
  }

  return { rating, loadRating, toggleRating }
}

/**
 * 消息收藏
 */
export function useMessageFavorite(props, emit) {
  const favorited = ref(false)

  function loadFavorite() {
    const sid = stableMsgId(props.msg)
    try {
      favorited.value = localStorage.getItem('ous_msg_fav_' + sid) === '1'
    } catch (_) {
      favorited.value = false
    }
  }

  function toggleFavorite() {
    const sid = stableMsgId(props.msg)
    favorited.value = !favorited.value
    try {
      if (favorited.value) {
        localStorage.setItem('ous_msg_fav_' + sid, '1')
        ElMessage.success({ message: '已收藏', duration: 1200 })
      } else {
        localStorage.removeItem('ous_msg_fav_' + sid)
        ElMessage.info({ message: '已取消收藏', duration: 1200 })
      }
    } catch (_) {}
    emit?.('favorite', props.msg, favorited.value)
  }

  return { favorited, loadFavorite, toggleFavorite }
}

/**
 * 消息分享
 */
export function useMessageShare(props, senderName) {
  async function doShare() {
    const msg = props.msg
    const sender = senderName.value
    const time = formatTime(msg?.timestamp)
    const plain = mdToPlainText(msg?.content || '')
    const snippet = plain.length > 80 ? plain.slice(0, 80) + '…' : plain
    const opCount = (msg?.referenced_operators?.length || 0)
    const url = typeof location !== 'undefined' ? location.href : ''
    const title = `来自 ${sender} 的璇玑助手消息`
    const text = `[璇玑助手] ${sender} · ${time}${opCount ? ` · 算子${opCount}枚` : ''}\n${snippet}\n打开链接：${url}`

    try {
      if (typeof navigator.share === 'function') {
        await navigator.share({ title, text, url })
        ElMessage.success({ message: '已分享', duration: 1500 })
        return true
      }
    } catch (_) { /* 用户取消或失败，fallback */ }

    const ok = await copyTextUniversal(text + (url && !text.includes(url) ? '\n' + url : ''), '分享卡片已复制', true)
    if (ok) {
      ElMessage.success({ message: '分享卡片已复制到剪贴板', duration: 1800 })
      return true
    }
    ElMessage.error('分享失败，请手动复制')
    return false
  }

  return { doShare }
}

/**
 * 语音朗读（三层回退：流式 TTS → 浏览器 TTS → 复制到剪贴板）
 */
export function useMessageSpeech(getContentFn) {
  const speechState = ref('idle') // idle | playing | paused
  let speechUtterance = null

  async function handleSpeakThreeLayer(text) {
    if (!text) return
    speechState.value = 'playing'

    // 第一层：流式 TTS API（预留，当前使用浏览器 TTS）
    try {
      if ('speechSynthesis' in window) {
        const utterance = new SpeechSynthesisUtterance(text)
        utterance.lang = 'zh-CN'
        utterance.rate = 1
        utterance.pitch = 1
        utterance.onend = () => { speechState.value = 'idle' }
        utterance.onerror = () => { speechState.value = 'idle' }
        speechUtterance = utterance
        speechSynthesis.speak(utterance)
        return
      }
    } catch (_) { /* fall through */ }

    // 第三层：复制到剪贴板
    try {
      await copyTextUniversal(text, '文本已复制到剪贴板', true)
      speechState.value = 'idle'
      return
    } catch (_) {}

    speechState.value = 'idle'
    ElMessage.warning({
      type: 'warning',
      showClose: true,
      duration: 0,
      message: '🔈 无可用语音路径。请手动复制消息文本。',
    })
  }

  function toggleSpeak() {
    if (speechState.value === 'playing') {
      try {
        const a = window.__mbAudio
        if (a && !a.paused) { a.pause(); speechState.value = 'paused'; return }
      } catch (_) {}
      try {
        if (typeof speechSynthesis !== 'undefined') { speechSynthesis.pause(); speechState.value = 'paused'; return }
      } catch (_) {}
      speechState.value = 'paused'
      return
    }
    if (speechState.value === 'paused') {
      try {
        const a = window.__mbAudio
        if (a && a.paused && !a.ended) { a.play(); speechState.value = 'playing'; return }
      } catch (_) {}
      try {
        if (typeof speechSynthesis !== 'undefined') { speechSynthesis.resume(); speechState.value = 'playing'; return }
      } catch (_) {}
      speechState.value = 'playing'
      return
    }
    // idle → 新播放
    try { if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel() } catch (_) {}
    try {
      const a = window.__mbAudio
      if (a) { try { a.pause() } catch (_) {} ; window.__mbAudio = null }
    } catch (_) {}
    const text = mdToPlainText(getContentFn?.() || '')
    handleSpeakThreeLayer(text)
  }

  function cancelSpeak() {
    try { if (typeof speechSynthesis !== 'undefined') speechSynthesis.cancel() } catch (_) {}
    try {
      const a = window.__mbAudio
      if (a) { try { a.pause() } catch (_) {} ; window.__mbAudio = null }
    } catch (_) {}
    speechState.value = 'idle'
    speechUtterance = null
  }

  return { speechState, toggleSpeak, cancelSpeak, handleSpeakThreeLayer }
}

/**
 * 消息反馈
 */
export function useMessageFeedback(props, emit) {
  const fbForm = ref({ category: 'bug', content: '', contact: '' })
  const fbSubmitting = ref(false)
  const fbDialog = ref(false)

  function resetFbForm() {
    fbForm.value = { category: 'bug', content: '', contact: '' }
  }

  async function submitFeedback() {
    if (!fbForm.value.content.trim()) {
      ElMessage.warning('请填写反馈内容')
      return
    }
    fbSubmitting.value = true
    try {
      // TODO: 调用真实反馈 API
      await new Promise(r => setTimeout(r, 500))
      ElMessage.success('反馈已提交，感谢您的建议！')
      fbDialog.value = false
      resetFbForm()
      emit?.('feedback', props.msg, fbForm.value)
    } catch (e) {
      ElMessage.error('提交失败：' + (e.message || '未知错误'))
    } finally {
      fbSubmitting.value = false
    }
  }

  return { fbForm, fbSubmitting, fbDialog, resetFbForm, submitFeedback }
}

/**
 * 导出 Markdown
 */
export function useMessageExport(props) {
  async function exportMarkdown(format = 'md') {
    const content = props.msg?.content || ''
    const title = props.msg?.title || '消息导出'
    const filename = `${title}_${Date.now()}.${format}`

    try {
      const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = filename
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      ElMessage.success('导出成功')
      return true
    } catch (e) {
      ElMessage.error('导出失败：' + (e.message || '未知错误'))
      return false
    }
  }

  return { exportMarkdown }
}
