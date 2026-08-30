<template>
  <div class="login">
    <div class="card">
      <div class="brand">🧠 智算企业门户 · 统一登录</div>
      <el-alert type="info" :closable="false" show-icon title="演示登录" description="OUS 当前以门户壳演示鉴权；企业部署请对接 §7.1 的 JWT/OAuth2 网关。" style="margin-bottom:16px" />
      <el-form label-width="90px">
        <el-form-item label="账号">
          <el-input v-model="form.user" placeholder="如 admin" />
        </el-form-item>
        <el-form-item label="密码">
          <el-input v-model="form.pass" type="password" show-password placeholder="演示任意非空" />
        </el-form-item>
        <el-form-item label="运行形态">
          <el-select v-model="runMode" style="width:100%">
            <el-option label="本地电脑 + 本地 LLM" value="local" />
            <el-option label="云电脑 + 云 LLM" value="cloud" />
          </el-select>
        </el-form-item>
        <el-form-item label="LLM 来源">
          <el-select v-model="llmMode" style="width:100%">
            <el-option label="本地 (Ollama/vLLM)" value="local" />
            <el-option label="云端 (DeepSeek/OpenAI)" value="cloud" />
          </el-select>
        </el-form-item>
        <el-button type="primary" style="width:100%" @click="login">登录并进入工作台</el-button>
      </el-form>
      <div class="hint">登录后将以所选形态运行（§13 全形态产品矩阵）</div>
    </div>
  </div>
</template>

<script setup>
import { reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'

const route = useRoute()
const router = useRouter()
const form = reactive({ user: 'admin', pass: '' })
const runMode = ref(localStorage.getItem('OUS_RUN_MODE') || 'local')
const llmMode = ref(localStorage.getItem('OUS_LLM_MODE') || 'local')

function login() {
  if (!form.pass) {
    ElMessage.warning('请输入密码（演示任意非空）')
    return
  }
  const token = 'demo-' + Date.now()
  localStorage.setItem('ous_api_token', token)
  localStorage.setItem('ous_token', token)
  localStorage.setItem('OUS_RUN_MODE', runMode.value)
  localStorage.setItem('OUS_LLM_MODE', llmMode.value)
  ElMessage.success('登录成功')
  router.replace(route.query.redirect || '/workbench')
}
</script>

<style scoped>
.login { min-height: 100vh; display: flex; align-items: center; justify-content: center; background: linear-gradient(160deg, #0b1020, #131a33); }
.card { width: 380px; background: #0e1428; border: 1px solid rgba(255,255,255,.1); border-radius: 16px; padding: 28px; color: #e6ebf5; }
.brand { font-size: 18px; font-weight: 700; margin-bottom: 18px; text-align: center; }
.hint { font-size: 12px; color: #8b9bc0; margin-top: 12px; text-align: center; }
</style>
