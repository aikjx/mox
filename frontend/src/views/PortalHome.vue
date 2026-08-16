<template>
  <div class="portal">
    <!-- 顶部导航 -->
    <header class="nav">
      <div class="brand">
        <span class="logo">🧠</span>
        <span class="name">智算企业门户</span>
        <span class="sub">Powered by 算子统一系统 (OUS)</span>
      </div>
      <nav class="menu">
        <a @click="go('/')">首页</a>
        <a @click="go('/hall')">业务大厅</a>
        <a @click="go('/workbench')">智能工作台</a>
        <a @click="go('/login')" class="login">登录</a>
      </nav>
    </header>

    <!-- 首屏 -->
    <section class="hero">
      <div class="hero-text">
        <h1>用统一算子引擎，<br />驱动企业全部业务流程</h1>
        <p>六大公理内核 · 专家联盟全维处理 · 一次编排、处处运行（云/本地、云/本地 LLM、浏览器/桌面）</p>
        <div class="hero-actions">
          <el-button type="primary" size="large" round @click="go('/hall')">进入业务大厅</el-button>
          <el-button size="large" round @click="go('/workbench')">打开智能工作台</el-button>
        </div>
        <div class="hero-stats">
          <div><b>6</b><span>数学公理内核</span></div>
          <div><b>13+</b><span>业务处理流程</span></div>
          <div><b>4</b><span>全形态运行</span></div>
        </div>
      </div>
      <div class="hero-card">
        <div class="hc-head">⚡ 实时能力</div>
        <ul>
          <li>AI 智能对话（流式）</li>
          <li>业务流程可视化编排</li>
          <li>知识图谱分析</li>
          <li>浏览器自动化 / 算子执行</li>
        </ul>
        <el-button text type="primary" @click="openChat">向 AI 助手提问 →</el-button>
      </div>
    </section>

    <!-- 能力卡片 -->
    <section class="features">
      <div class="feat" v-for="f in features" :key="f.t" @click="go(f.to)">
        <el-icon><component :is="f.icon" /></el-icon>
        <h3>{{ f.t }}</h3>
        <p>{{ f.d }}</p>
      </div>
    </section>

    <!-- AI 客服浮窗 -->
    <transition name="fade">
      <div class="chat-fab" v-if="!chatOpen" @click="chatOpen = true">💬</div>
    </transition>
    <transition name="slide">
      <div class="chat-panel" v-if="chatOpen">
        <div class="cp-head">
          <span>AI 企业助手</span>
          <el-icon @click="chatOpen = false"><Close /></el-icon>
        </div>
        <div class="cp-body" ref="cpBody">
          <div v-for="m in chat" :key="m.k" :class="['b', m.role]">
            {{ m.text }}
          </div>
          <div v-if="chatLoading" class="b ai typing">推理中…</div>
        </div>
        <div class="cp-input">
          <el-input v-model="draft" placeholder="咨询业务、查算子、办流程…" @keyup.enter="send" />
          <el-button type="primary" @click="send">发送</el-button>
        </div>
      </div>
    </transition>

    <footer class="foot">
      算子统一系统 (OUS) · 企业级 AI 门户演示 · 当前形态：{{ runMode }}
    </footer>
  </div>
</template>

<script setup>
import { ref, nextTick, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { aiChat } from '@/api'

const router = useRouter()
const go = (p) => router.push(p)

const features = [
  { t: 'AI 对话', d: '基于会话日志溯源的统一智能体', icon: 'ChatDotRound', to: '/workbench' },
  { t: '业务大厅', d: '调用 / 执行业务流程与算子', icon: 'Files', to: '/hall' },
  { t: '流程编排', d: '拖拽式 DAG + 实时类型校验', icon: 'Share', to: '/workbench' },
  { t: '知识图谱', d: 'PageRank / 社群发现可视化', icon: 'Connection', to: '/workbench' },
  { t: '专家联盟', d: '最高权限全维处理模式', icon: 'UserFilled', to: '/workbench' },
  { t: '全形态', d: '云/本地 · 浏览器/桌面', icon: 'Monitor', to: '/' },
]

const chatOpen = ref(false)
const chat = ref([])
const draft = ref('')
const chatLoading = ref(false)
const cpBody = ref(null)
let k = 0

const runMode =
  (typeof window !== 'undefined' && (localStorage.getItem('OUS_RUN_MODE') || 'local')) + ' + ' +
  (localStorage.getItem('OUS_LLM_MODE') || 'local')

async function send() {
  const text = draft.value.trim()
  if (!text || chatLoading.value) return
  chat.value.push({ k: ++k, role: 'user', text })
  draft.value = ''
  chatLoading.value = true
  await nextTick()
  cpBody.value?.scrollTo({ top: 1e9 })
  try {
    const resp = await aiChat({ message: text })
    const out = resp?.reply || resp?.response || JSON.stringify(resp)
    chat.value.push({ k: ++k, role: 'ai', text: out })
  } catch (e) {
    chat.value.push({ k: ++k, role: 'ai', text: '（助手暂不可用：' + e.message + '）' })
  } finally {
    chatLoading.value = false
    await nextTick()
    cpBody.value?.scrollTo({ top: 1e9 })
  }
}

function openChat() {
  chatOpen.value = true
  if (!chat.value.length) {
    chat.value.push({ k: ++k, role: 'ai', text: '您好，我是企业 AI 助手。可以帮您查算子、办流程、答业务问题。' })
  }
}

onMounted(() => {
  if (!localStorage.getItem('OUS_RUN_MODE')) localStorage.setItem('OUS_RUN_MODE', 'local')
  if (!localStorage.getItem('OUS_LLM_MODE')) localStorage.setItem('OUS_LLM_MODE', 'local')
})
</script>

<style scoped>
.portal { min-height: 100vh; background: linear-gradient(160deg, #0b1020, #131a33); color: #e6ebf5; }
.nav { display: flex; justify-content: space-between; align-items: center; padding: 16px 40px; }
.brand { display: flex; align-items: center; gap: 10px; }
.logo { font-size: 26px; }
.name { font-size: 20px; font-weight: 700; }
.sub { font-size: 12px; color: #8b9bc0; margin-left: 6px; }
.menu a { margin-left: 22px; cursor: pointer; color: #c4cdec; font-size: 14px; }
.menu a.login { color: #6ea8ff; font-weight: 600; }
.hero { display: flex; gap: 40px; padding: 60px 40px; align-items: center; }
.hero-text h1 { font-size: 38px; line-height: 1.3; margin: 0 0 16px; }
.hero-text p { color: #9fb0d6; max-width: 520px; }
.hero-actions { margin: 24px 0; }
.hero-stats { display: flex; gap: 30px; margin-top: 30px; }
.hero-stats div { display: flex; flex-direction: column; }
.hero-stats b { font-size: 28px; color: #6ea8ff; }
.hero-stats span { font-size: 12px; color: #8b9bc0; }
.hero-card { background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); border-radius: 16px; padding: 24px; width: 320px; }
.hc-head { font-weight: 600; margin-bottom: 12px; }
.hero-card ul { padding-left: 18px; color: #c4cdec; line-height: 2; }
.features { display: grid; grid-template-columns: repeat(3, 1fr); gap: 18px; padding: 20px 40px 50px; }
.feat { background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.07); border-radius: 14px; padding: 22px; cursor: pointer; transition: .2s; }
.feat:hover { border-color: #6ea8ff; transform: translateY(-3px); }
.feat .el-icon { font-size: 26px; color: #6ea8ff; }
.feat h3 { margin: 10px 0 6px; }
.feat p { color: #9fb0d6; font-size: 13px; }
.chat-fab { position: fixed; right: 24px; bottom: 24px; width: 56px; height: 56px; border-radius: 50%; background: #6ea8ff; display: flex; align-items: center; justify-content: center; font-size: 26px; cursor: pointer; box-shadow: 0 8px 24px rgba(0,0,0,.4); }
.chat-panel { position: fixed; right: 24px; bottom: 24px; width: 340px; height: 460px; background: #0e1428; border: 1px solid rgba(255,255,255,.1); border-radius: 14px; display: flex; flex-direction: column; overflow: hidden; }
.cp-head { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid rgba(255,255,255,.08); font-weight: 600; }
.cp-body { flex: 1; overflow: auto; padding: 12px; }
.b { margin-bottom: 10px; padding: 8px 12px; border-radius: 10px; font-size: 13px; line-height: 1.5; max-width: 85%; }
.b.user { background: #2a3a66; margin-left: auto; }
.b.ai { background: rgba(255,255,255,.06); }
.b.typing { color: #8b9bc0; font-style: italic; }
.cp-input { display: flex; gap: 8px; padding: 10px; border-top: 1px solid rgba(255,255,255,.08); }
.foot { text-align: center; padding: 24px; color: #6b7aa0; font-size: 12px; }
.fade-enter-active, .fade-leave-active { transition: opacity .2s; }
.slide-enter-active, .slide-leave-active { transition: all .25s; }
</style>
