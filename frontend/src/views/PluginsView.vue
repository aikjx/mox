<template>
  <div class="pv">
    <div class="head">
      <div>
        <h2 class="page-title">AI 插件</h2>
        <p class="page-subtitle">插件互通总线 · 注册 / 发现 / 消息互通的插件生态</p>
      </div>
      <el-button type="primary" @click="showReg = true">
        <el-icon><Plus /></el-icon> 注册插件
      </el-button>
    </div>

    <div class="grid grid-3 plugin-grid">
      <div class="panel plugin-card" v-for="p in plugins" :key="p.id || p.name">
        <div class="plugin-head">
          <div class="plugin-avatar" :style="{ background: colorOf(p) }">
            <el-icon><Connection /></el-icon>
          </div>
          <div class="plugin-id">{{ p.name || p.id }}</div>
          <span class="badge success">在线</span>
        </div>
        <div class="plugin-desc">{{ p.description || p.capabilities?.join(' · ') || 'AI 能力插件' }}</div>
        <div class="plugin-actions">
          <el-button size="small" @click="sendMsg(p)">
            <el-icon><Promotion /></el-icon> 发送消息
          </el-button>
        </div>
      </div>
      <el-empty v-if="!plugins.length" description="暂无插件，请注册" :image-size="70" />
    </div>

    <!-- 消息互通台 -->
    <div class="panel card-pad">
      <h3 class="section-title">插件消息互通</h3>
      <div class="msg-box">
        <div class="msg-log" ref="logEl">
          <div v-for="(m, i) in messages" :key="i" class="msg" :class="m.dir">
            <div class="msg-meta">{{ m.from }} → {{ m.to }}</div>
            <div class="msg-content">{{ m.content }}</div>
          </div>
          <el-empty v-if="!messages.length" description="尚无消息往来" :image-size="50" />
        </div>
        <div class="msg-input">
          <el-select v-model="msgForm.from" placeholder="来源" style="width: 160px">
            <el-option v-for="p in plugins" :key="p.id" :label="p.name || p.id" :value="p.id || p.name" />
          </el-select>
          <el-select v-model="msgForm.to" placeholder="目标" style="width: 160px">
            <el-option v-for="p in plugins" :key="p.id" :label="p.name || p.id" :value="p.id || p.name" />
          </el-select>
          <el-input v-model="msgForm.content" placeholder="消息内容" style="flex: 1" @keyup.enter="pushMsg" />
          <el-button type="primary" :loading="sending" @click="pushMsg">发送</el-button>
        </div>
      </div>
    </div>

    <el-dialog v-model="showReg" title="注册 AI 插件" width="460px">
      <el-form label-width="80px">
        <el-form-item label="插件 ID">
          <el-input v-model="reg.id" placeholder="唯一标识" />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="reg.name" />
        </el-form-item>
        <el-form-item label="能力">
          <el-input v-model="reg.capabilities" placeholder="逗号分隔，如 chat,search" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showReg = false">取消</el-button>
        <el-button type="primary" :loading="reging" @click="doReg">注册</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, onMounted, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import { getAiPlugins, registerAiPlugin, sendPluginMessage } from '@/api'

const plugins = ref([])
const messages = ref([])
const logEl = ref(null)
const sending = ref(false)

const showReg = ref(false)
const reging = ref(false)
const reg = ref({ id: '', name: '', capabilities: '' })

const msgForm = ref({ from: '', to: '', content: '' })

function colorOf(p) {
  const colors = ['#4f46e5', '#06b6d4', '#10b981', '#f59e0b', '#ec4899']
  const s = (p.id || p.name || '').toString()
  let h = 0
  for (const c of s) h = (h + c.charCodeAt(0)) % colors.length
  return colors[h]
}

async function load() {
  try {
    const r = await getAiPlugins()
    plugins.value = r.plugins || r.data || r || []
  } catch (e) {
    plugins.value = []
  }
}

function sendMsg(p) {
  msgForm.value.from = msgForm.value.from || (p.id || p.name)
  msgForm.value.to = msgForm.value.to || (plugins.value[1] && (plugins.value[1].id || plugins.value[1].name)) || ''
}

async function pushMsg() {
  if (!msgForm.value.from || !msgForm.value.to || !msgForm.value.content) {
    ElMessage.warning('请填写完整消息信息')
    return
  }
  sending.value = true
  try {
    const r = await sendPluginMessage({
      from: msgForm.value.from,
      to: msgForm.value.to,
      message: msgForm.value.content
    })
    messages.value.push({
      dir: 'out',
      from: msgForm.value.from,
      to: msgForm.value.to,
      content: msgForm.value.content
    })
    const reply = r.response || r.message || r.reply || '（已接收）'
    messages.value.push({ dir: 'in', from: msgForm.value.to, to: msgForm.value.from, content: reply })
    msgForm.value.content = ''
    await nextTick()
    if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    sending.value = false
  }
}

async function doReg() {
  if (!reg.value.id || !reg.value.name) {
    ElMessage.warning('请填写 ID 与名称')
    return
  }
  reging.value = true
  try {
    await registerAiPlugin({
      id: reg.value.id,
      name: reg.value.name,
      capabilities: reg.value.capabilities.split(',').map((s) => s.trim()).filter(Boolean)
    })
    ElMessage.success('注册成功')
    showReg.value = false
    reg.value = { id: '', name: '', capabilities: '' }
    await load()
  } catch (e) {
    ElMessage.error(e.message)
  } finally {
    reging.value = false
  }
}

onMounted(load)
</script>

<style scoped>
.pv {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.plugin-grid {
  align-items: start;
}
.plugin-card {
  padding: 16px 18px;
}
.plugin-head {
  display: flex;
  align-items: center;
  gap: 10px;
}
.plugin-avatar {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  color: #fff;
  font-size: 18px;
}
.plugin-id {
  font-weight: 700;
  font-size: 15px;
  flex: 1;
}
.plugin-desc {
  font-size: 13px;
  color: var(--text-3);
  margin: 12px 0;
  min-height: 36px;
}
.plugin-actions {
  display: flex;
  gap: 8px;
}
.card-pad {
  padding: 18px 20px;
}
.msg-box {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.msg-log {
  height: 220px;
  overflow-y: auto;
  background: var(--bg-page);
  border-radius: 10px;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.msg {
  max-width: 75%;
  padding: 8px 12px;
  border-radius: 10px;
}
.msg.out {
  align-self: flex-end;
  background: var(--brand);
  color: #fff;
}
.msg.in {
  align-self: flex-start;
  background: #fff;
  border: 1px solid var(--border);
}
.msg-meta {
  font-size: 11px;
  opacity: 0.7;
  margin-bottom: 3px;
}
.msg-content {
  font-size: 13px;
}
.msg-input {
  display: flex;
  gap: 8px;
}
</style>
