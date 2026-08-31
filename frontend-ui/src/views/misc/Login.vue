<template>
  <div class="login">
    <div class="card">
      <div class="brand">🧠 智算企业门户 · 统一登录</div>

      <!-- 演示模式红色警告 -->
      <el-alert
        v-if="authMode === 'demo'"
        type="error"
        :closable="false"
        show-icon
        title="演示模式"
        description="当前为演示模式，仅用于功能展示。企业部署请对接 JWT/OAuth2 认证网关。"
        style="margin-bottom:16px"
      />

      <!-- 认证模式切换 -->
      <div class="mode-tabs">
        <div
          v-for="mode in authModes"
          :key="mode.value"
          :class="['mode-tab', { active: authMode === mode.value }]"
          @click="switchMode(mode.value)"
        >
          {{ mode.label }}
        </div>
      </div>

      <!-- 账号密码登录表单（demo / jwt 模式） -->
      <template v-if="authMode !== 'oauth2'">
        <el-form
          ref="loginFormRef"
          :model="form"
          :rules="loginRules"
          label-width="90px"
          @submit.prevent="handleLogin"
        >
          <el-form-item label="账号" prop="username">
            <el-input
              v-model="form.username"
              placeholder="请输入账号"
              :disabled="isLocked || loggingIn"
              @keyup.enter="handleLogin"
            />
          </el-form-item>

          <el-form-item label="密码" prop="password">
            <el-input
              v-model="form.password"
              type="password"
              show-password
              :placeholder="authMode === 'demo' ? '演示模式：至少6位' : '请输入密码'"
              :disabled="isLocked || loggingIn"
              @keyup.enter="handleLogin"
            />
            <!-- 密码强度提示 -->
            <div class="pwd-strength" v-if="form.password">
              <div
                :class="['strength-bar', strengthLevel >= 1 ? 'weak' : '']"
              ></div>
              <div
                :class="['strength-bar', strengthLevel >= 2 ? 'medium' : '']"
              ></div>
              <div
                :class="['strength-bar', strengthLevel >= 3 ? 'strong' : '']"
              ></div>
              <span class="strength-text">{{ strengthText }}</span>
            </div>
          </el-form-item>

          <!-- 图形验证码（可配置开关） -->
          <el-form-item
            v-if="captchaEnabled"
            label="验证码"
            prop="captcha"
          >
            <div class="captcha-row">
              <el-input
                v-model="form.captcha"
                placeholder="请输入验证码"
                :disabled="isLocked || loggingIn"
                maxlength="4"
                @keyup.enter="handleLogin"
              />
              <div class="captcha-img" @click="refreshCaptcha" title="点击刷新">
                {{ captchaCode }}
              </div>
            </div>
          </el-form-item>

          <!-- 运行形态 & LLM 来源（保留原有配置） -->
          <el-form-item label="运行形态">
            <el-select v-model="runMode" style="width:100%" :disabled="isLocked || loggingIn">
              <el-option label="本地电脑 + 本地 LLM" value="local" />
              <el-option label="云电脑 + 云 LLM" value="cloud" />
            </el-select>
          </el-form-item>

          <el-form-item label="LLM 来源">
            <el-select v-model="llmMode" style="width:100%" :disabled="isLocked || loggingIn">
              <el-option label="本地 (Ollama/vLLM)" value="local" />
              <el-option label="云端 (DeepSeek/OpenAI)" value="cloud" />
            </el-select>
          </el-form-item>

          <!-- 登录按钮 -->
          <el-button
            type="primary"
            style="width:100%"
            :loading="loggingIn"
            :disabled="isLocked"
            @click="handleLogin"
          >
            <span v-if="isLocked">账号锁定中 ({{ lockCountdown }}s)</span>
            <span v-else-if="loggingIn">登录中...</span>
            <span v-else>{{ authMode === 'demo' ? '演示登录' : '登录并进入工作台' }}</span>
          </el-button>

          <!-- 失败次数提示 -->
          <div v-if="failedAttempts > 0 && !isLocked" class="fail-hint">
            已失败 {{ failedAttempts }} 次，连续失败 5 次将锁定 30 秒
          </div>
        </el-form>
      </template>

      <!-- OAuth2 SSO 登录（oauth2 模式） -->
      <template v-else>
        <div class="sso-section">
          <div class="sso-description">
            通过企业统一身份认证（SSO）登录系统
          </div>

          <el-button
            type="primary"
            size="large"
            style="width:100%; margin-top: 16px;"
            :loading="loggingIn"
            @click="handleOAuth2Login"
          >
            <el-icon style="margin-right: 8px"><User /></el-icon>
            企业 SSO 登录
          </el-button>

          <div class="sso-divider">
            <span>或</span>
          </div>

          <el-button
            style="width:100%"
            @click="switchMode('jwt')"
          >
            使用账号密码登录
          </el-button>

          <!-- OAuth2 配置说明 -->
          <div class="sso-hint">
            <el-icon><InfoFilled /></el-icon>
            <span>OAuth2 配置请在环境变量中设置 VITE_OAUTH2_AUTH_URL</span>
          </div>
        </div>
      </template>

      <div class="hint">登录后将以所选形态运行（§13 全形态产品矩阵）</div>
    </div>
  </div>
</template>

<script setup>
import { reactive, ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { User, InfoFilled } from '@element-plus/icons-vue'
import { useAuthStore, LOGIN_MODES } from '@/stores/auth.store'
import { secureSetItem } from '@/utils/secureStorage'

const route = useRoute()
const router = useRouter()
const authStore = useAuthStore()

// ===== 认证模式配置 =====
const authModes = [
  { value: LOGIN_MODES.DEMO, label: '演示模式' },
  { value: LOGIN_MODES.JWT, label: '账号密码' },
  { value: LOGIN_MODES.OAUTH2, label: 'SSO 登录' },
]

// 从 auth store 读取当前模式（已自动从 localStorage / 环境变量初始化）
const authMode = ref(authStore.loginMode || LOGIN_MODES.DEMO)

// 验证码开关（可通过环境变量或 localStorage 配置）
const captchaEnabled = ref(_getCaptchaConfig())

function _getCaptchaConfig() {
  // 优先级：localStorage > 环境变量 > 默认关闭
  const stored = typeof localStorage !== 'undefined'
    ? localStorage.getItem('mox_captcha_enabled')
    : null
  if (stored !== null) {
    return stored === 'true' || stored === '1'
  }
  const envVal =
    typeof import.meta !== 'undefined' &&
    import.meta.env &&
    import.meta.env.VITE_CAPTCHA_ENABLED
  if (envVal !== undefined) {
    return envVal === 'true' || envVal === '1'
  }
  return false
}

// ===== 表单数据 =====
const loginFormRef = ref(null)
const form = reactive({
  username: 'admin',
  password: '',
  captcha: '',
})

const runMode = ref(localStorage.getItem('OUS_RUN_MODE') || 'local')
const llmMode = ref(localStorage.getItem('OUS_LLM_MODE') || 'local')

// ===== 登录失败限流 =====
const MAX_FAILED_ATTEMPTS = 5
const LOCK_DURATION_SECONDS = 30

const failedAttempts = ref(0)
const isLocked = ref(false)
const lockCountdown = ref(0)
let lockTimer = null

function _loadFailedState() {
  try {
    const stored = localStorage.getItem('mox_login_failed')
    if (stored) {
      const data = JSON.parse(stored)
      failedAttempts.value = data.count || 0
      // 检查是否还在锁定中
      if (data.lockUntil && Date.now() < data.lockUntil) {
        isLocked.value = true
        lockCountdown.value = Math.ceil((data.lockUntil - Date.now()) / 1000)
        _startLockCountdown()
      }
    }
  } catch {}
}

function _saveFailedState() {
  try {
    const data = {
      count: failedAttempts.value,
      lockUntil: isLocked.value
        ? Date.now() + LOCK_DURATION_SECONDS * 1000
        : 0,
    }
    localStorage.setItem('mox_login_failed', JSON.stringify(data))
  } catch {}
}

function _startLockCountdown() {
  if (lockTimer) clearInterval(lockTimer)
  lockTimer = setInterval(() => {
    lockCountdown.value--
    if (lockCountdown.value <= 0) {
      clearInterval(lockTimer)
      lockTimer = null
      isLocked.value = false
      failedAttempts.value = 0
      _saveFailedState()
    }
  }, 1000)
}

function _recordFailedAttempt() {
  failedAttempts.value++
  if (failedAttempts.value >= MAX_FAILED_ATTEMPTS) {
    isLocked.value = true
    lockCountdown.value = LOCK_DURATION_SECONDS
    _startLockCountdown()
    ElMessage.error(`登录失败次数过多，账号已锁定 ${LOCK_DURATION_SECONDS} 秒`)
  } else {
    const remaining = MAX_FAILED_ATTEMPTS - failedAttempts.value
    ElMessage.warning(`登录失败，还剩 ${remaining} 次机会`)
  }
  _saveFailedState()
}

function _resetFailedAttempts() {
  failedAttempts.value = 0
  isLocked.value = false
  lockCountdown.value = 0
  if (lockTimer) {
    clearInterval(lockTimer)
    lockTimer = null
  }
  try {
    localStorage.removeItem('mox_login_failed')
  } catch {}
}

// ===== 密码强度校验 =====
const loginRules = {
  username: [
    { required: true, message: '请输入账号', trigger: 'blur' },
    { min: 2, max: 50, message: '账号长度在 2 到 50 个字符', trigger: 'blur' },
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '密码长度不能少于 6 位', trigger: 'blur' },
  ],
  captcha: captchaEnabled.value
    ? [{ required: true, message: '请输入验证码', trigger: 'blur' }]
    : [],
}

const strengthLevel = computed(() => {
  const pwd = form.password
  if (!pwd) return 0
  let level = 0
  // 长度 >= 6
  if (pwd.length >= 6) level++
  // 包含数字和字母
  if (/[a-zA-Z]/.test(pwd) && /[0-9]/.test(pwd)) level++
  // 包含特殊字符
  if (/[^a-zA-Z0-9]/.test(pwd)) level++
  // 长度 >= 10 额外加分
  if (pwd.length >= 10 && level < 3) level++
  return Math.min(level, 3)
})

const strengthText = computed(() => {
  const texts = ['', '弱', '中', '强']
  return texts[strengthLevel.value] || ''
})

// ===== 图形验证码 =====
const captchaCode = ref('')

function refreshCaptcha() {
  // 前端生成随机验证码（生产环境应由后端生成图片）
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'
  let code = ''
  for (let i = 0; i < 4; i++) {
    code += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  captchaCode.value = code
}

function _validateCaptcha() {
  if (!captchaEnabled.value) return true
  if (!form.captcha) {
    ElMessage.warning('请输入验证码')
    return false
  }
  if (form.captcha.toUpperCase() !== captchaCode.value) {
    ElMessage.warning('验证码错误')
    refreshCaptcha()
    form.captcha = ''
    return false
  }
  return true
}

// ===== 登录处理 =====
const loggingIn = ref(false)

async function handleLogin() {
  if (isLocked.value) {
    ElMessage.warning('账号已锁定，请稍后再试')
    return
  }

  // 表单校验
  if (loginFormRef.value) {
    try {
      await loginFormRef.value.validate()
    } catch {
      return
    }
  } else {
    if (!form.username || !form.password) {
      ElMessage.warning('请输入账号和密码')
      return
    }
    if (form.password.length < 6) {
      ElMessage.warning('密码长度不能少于 6 位')
      return
    }
  }

  // 验证码校验
  if (!_validateCaptcha()) {
    return
  }

  loggingIn.value = true

  try {
    // 调用 auth store 的 login 方法
    const result = await authStore.login(
      {
        username: form.username,
        password: form.password,
      },
      authMode.value
    )

    if (result && result.token) {
      // 保存运行配置
      localStorage.setItem('OUS_RUN_MODE', runMode.value)
      localStorage.setItem('OUS_LLM_MODE', llmMode.value)

      // 重置失败计数
      _resetFailedAttempts()

      ElMessage.success('登录成功')

      // 跳转
      const redirect = route.query.redirect || '/workbench'
      router.replace(redirect)
    } else {
      throw new Error('登录失败：未获取到有效 token')
    }
  } catch (e) {
    console.error('[Login] 登录失败:', e)
    _recordFailedAttempt()

    // 刷新验证码
    if (captchaEnabled.value) {
      refreshCaptcha()
      form.captcha = ''
    }

    // 错误消息已在 auth store / http 拦截器中处理
    // 这里补充兜底提示
    if (e && e.message && !e.message.includes('OAUTH2')) {
      // 避免重复提示（ElMessage 可能已在拦截器中弹过）
      // 这里只在没有状态码的情况下弹
      if (!e.status) {
        ElMessage.error(e.message || '登录失败')
      }
    }
  } finally {
    loggingIn.value = false
  }
}

// ===== OAuth2 SSO 登录 =====
function handleOAuth2Login() {
  loggingIn.value = true

  // 构造 OAuth2 授权 URL
  const authUrl =
    (typeof import.meta !== 'undefined' &&
      import.meta.env &&
      import.meta.env.VITE_OAUTH2_AUTH_URL) ||
    ''

  if (!authUrl) {
    ElMessage.error('OAuth2 配置缺失：请设置 VITE_OAUTH2_AUTH_URL')
    loggingIn.value = false
    return
  }

  try {
    // 保存当前状态（用于回调后恢复）
    secureSetItem('oauth2_state', {
      redirect: route.query.redirect || '/workbench',
      timestamp: Date.now(),
    })

    // 跳转到授权服务器
    const redirectUri = encodeURIComponent(
      window.location.origin + '/#/oauth2/callback'
    )
    const fullUrl = `${authUrl}?client_id=${
      import.meta.env.VITE_OAUTH2_CLIENT_ID || 'mox-client'
    }&redirect_uri=${redirectUri}&response_type=code&scope=openid profile`

    window.location.href = fullUrl
  } catch (e) {
    console.error('[Login] OAuth2 跳转失败:', e)
    ElMessage.error('SSO 登录跳转失败')
    loggingIn.value = false
  }
}

// ===== 模式切换 =====
function switchMode(mode) {
  authMode.value = mode
  authStore.setLoginMode(mode)
  // 切换模式时重置表单状态
  form.password = ''
  form.captcha = ''
  if (captchaEnabled.value) {
    refreshCaptcha()
  }
}

// ===== 生命周期 =====
onMounted(() => {
  _loadFailedState()
  if (captchaEnabled.value) {
    refreshCaptcha()
  }
})

onUnmounted(() => {
  if (lockTimer) {
    clearInterval(lockTimer)
    lockTimer = null
  }
})
</script>

<style scoped>
.login {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(160deg, #0b1020, #131a33);
}
.card {
  width: 400px;
  background: #0e1428;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 16px;
  padding: 28px;
  color: #e6ebf5;
}
.brand {
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 18px;
  text-align: center;
}
.hint {
  font-size: 12px;
  color: #8b9bc0;
  margin-top: 12px;
  text-align: center;
}

/* 模式切换标签 */
.mode-tabs {
  display: flex;
  background: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  padding: 4px;
  margin-bottom: 20px;
}
.mode-tab {
  flex: 1;
  text-align: center;
  padding: 8px 12px;
  font-size: 13px;
  color: #8b9bc0;
  cursor: pointer;
  border-radius: 6px;
  transition: all 0.2s;
}
.mode-tab:hover {
  color: #e6ebf5;
}
.mode-tab.active {
  background: rgba(64, 158, 255, 0.2);
  color: #409eff;
  font-weight: 500;
}

/* 密码强度 */
.pwd-strength {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 6px;
}
.strength-bar {
  flex: 1;
  height: 4px;
  background: rgba(255, 255, 255, 0.1);
  border-radius: 2px;
  transition: background 0.3s;
}
.strength-bar.weak {
  background: #f56c6c;
}
.strength-bar.medium {
  background: #e6a23c;
}
.strength-bar.strong {
  background: #67c23a;
}
.strength-text {
  font-size: 11px;
  color: #8b9bc0;
  min-width: 24px;
  text-align: right;
}

/* 验证码 */
.captcha-row {
  display: flex;
  gap: 10px;
  align-items: center;
}
.captcha-row .el-input {
  flex: 1;
}
.captcha-img {
  width: 90px;
  height: 36px;
  background: linear-gradient(135deg, #1e3a5f, #2d4a6f);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: 'Courier New', monospace;
  font-size: 18px;
  font-weight: bold;
  color: #409eff;
  letter-spacing: 2px;
  cursor: pointer;
  user-select: none;
  transition: transform 0.2s;
}
.captcha-img:hover {
  transform: scale(1.02);
}

/* 失败提示 */
.fail-hint {
  font-size: 12px;
  color: #e6a23c;
  text-align: center;
  margin-top: 8px;
}

/* SSO 区域 */
.sso-section {
  padding: 20px 0;
}
.sso-description {
  font-size: 13px;
  color: #8b9bc0;
  text-align: center;
  line-height: 1.6;
}
.sso-divider {
  display: flex;
  align-items: center;
  margin: 20px 0;
  color: #8b9bc0;
  font-size: 12px;
}
.sso-divider::before,
.sso-divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: rgba(255, 255, 255, 0.1);
}
.sso-divider span {
  padding: 0 12px;
}
.sso-hint {
  margin-top: 20px;
  padding: 10px;
  background: rgba(64, 158, 255, 0.1);
  border-radius: 6px;
  font-size: 12px;
  color: #8b9bc0;
  display: flex;
  align-items: flex-start;
  gap: 6px;
}
.sso-hint .el-icon {
  flex-shrink: 0;
  margin-top: 2px;
  color: #409eff;
}
</style>
