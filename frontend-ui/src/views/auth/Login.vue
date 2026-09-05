<template>
  <div class="login-container">
    <!-- 左侧品牌区 -->
    <div class="login-brand">
      <div class="brand-content">
        <div class="brand-logo">
          <svg viewBox="0 0 64 64" fill="none" xmlns="http://www.w3.org/2000/svg">
            <circle cx="32" cy="32" r="28" stroke="currentColor" stroke-width="2" />
            <circle cx="32" cy="32" r="18" stroke="currentColor" stroke-width="1.5" opacity="0.6" />
            <circle cx="32" cy="32" r="8" fill="currentColor" opacity="0.8" />
            <line x1="32" y1="4" x2="32" y2="14" stroke="currentColor" stroke-width="2" />
            <line x1="32" y1="50" x2="32" y2="60" stroke="currentColor" stroke-width="2" />
            <line x1="4" y1="32" x2="14" y2="32" stroke="currentColor" stroke-width="2" />
            <line x1="50" y1="32" x2="60" y2="32" stroke="currentColor" stroke-width="2" />
          </svg>
        </div>
        <h1 class="brand-title">MOX 平台</h1>
        <p class="brand-subtitle">模块化 · 正交 · 可扩展</p>
        <p class="brand-desc">多专家智能编排平台，统一管理 AI 联盟、知识库与业务流程</p>
        <div class="brand-features">
          <div class="feature-item">
            <span class="feature-icon">⚡</span>
            <span>6阶段智能管线</span>
          </div>
          <div class="feature-item">
            <span class="feature-icon">🔒</span>
            <span>企业级安全认证</span>
          </div>
          <div class="feature-item">
            <span class="feature-icon">📊</span>
            <span>全链路可观测性</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 右侧登录表单 -->
    <div class="login-form-wrapper">
      <div class="login-form">
        <h2 class="form-title">欢迎回来</h2>
        <p class="form-subtitle">请登录您的账户以继续</p>

        <el-form
          ref="loginFormRef"
          :model="loginForm"
          :rules="loginRules"
          class="login-form-content"
          @submit.prevent="handleLogin"
        >
          <el-form-item prop="username">
            <el-input
              v-model="loginForm.username"
              placeholder="用户名"
              size="large"
              :prefix-icon="User"
              clearable
            />
          </el-form-item>

          <el-form-item prop="password">
            <el-input
              v-model="loginForm.password"
              type="password"
              placeholder="密码"
              size="large"
              :prefix-icon="Lock"
              show-password
              @keyup.enter="handleLogin"
            />
          </el-form-item>

          <el-form-item prop="tenant_id" v-if="showTenant">
            <el-input
              v-model="loginForm.tenant_id"
              placeholder="租户 ID（可选）"
              size="large"
              :prefix-icon="OfficeBuilding"
              clearable
            />
          </el-form-item>

          <div class="form-options">
            <el-checkbox v-model="loginForm.remember_me">记住我</el-checkbox>
            <router-link to="/forgot-password" class="forgot-link">忘记密码？</router-link>
          </div>

          <el-form-item>
            <el-button
              type="primary"
              size="large"
              class="login-button"
              :loading="authStore.loading"
              @click="handleLogin"
            >
              {{ authStore.loading ? '登录中...' : '登 录' }}
            </el-button>
          </el-form-item>

          <el-alert
            v-if="authStore.error"
            :title="authStore.error"
            type="error"
            show-icon
            :closable="false"
            class="error-alert"
          />
        </el-form>

        <details class="token-login">
          <summary>使用已有访问令牌</summary>
          <p>适用于独立部署。令牌验证后仅在当前页面有效，刷新后需重新连接。</p>
          <el-input v-model="existingToken" type="password" placeholder="访问令牌" autocomplete="off" @keyup.enter="handleTokenLogin" />
          <el-button type="primary" :loading="authStore.loading" :disabled="!existingToken.trim()" @click="handleTokenLogin">验证并连接</el-button>
        </details>

        <div class="form-footer">
          <span>还没有账户？</span>
          <router-link to="/register" class="register-link">立即注册</router-link>
        </div>

        <div class="form-divider">
          <span>或</span>
        </div>

        <div class="social-login">
          <el-button size="large" class="social-button" @click="handleSSO('sso')">
            企业 SSO 登录
          </el-button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { User, Lock, OfficeBuilding } from '@element-plus/icons-vue'
import { useAuthStore } from '../../stores/auth.store'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const loginFormRef = ref(null)
const showTenant = ref(false)
const existingToken = ref('')

async function handleTokenLogin() {
  try {
    await authStore.loginWithToken(existingToken.value)
    existingToken.value = ''
    const redirect = route.query.redirect
    await router.push(typeof redirect === 'string' && redirect.startsWith('/') && !redirect.startsWith('//') ? redirect : '/expert-center/tasks')
  } catch { /* 错误由认证状态展示 */ }
}

const loginForm = reactive({
  username: '',
  password: '',
  tenant_id: '',
  remember_me: true
})

const loginRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 50, message: '用户名长度在 3 到 50 个字符', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 8, message: '密码长度至少 8 个字符', trigger: 'blur' }
  ]
}

async function handleLogin() {
  if (!loginFormRef.value) return

  try {
    await loginFormRef.value.validate()
  } catch {
    return
  }

  try {
    await authStore.login(
      loginForm.username,
      loginForm.password,
      loginForm.tenant_id || 'default'
    )

    ElMessage.success('登录成功')

    // 跳转到原目标页面或首页
    const redirect = route.query.redirect || '/dashboard'
    router.push(redirect)
  } catch (err) {
    // 错误已在 store 中设置
    console.error('登录失败:', err)
  }
}

function handleSSO(provider) {
  ElMessage.info(`${provider} 登录功能开发中`)
}
</script>

<style scoped>
.token-login { margin-top: 20px; font-size: 13px; }
.token-login summary { cursor: pointer; color: #5264bf; }
.token-login p { color: #606266; line-height: 1.6; }
.token-login .el-button { margin-top: 12px; width: 100%; }
.login-container {
  display: flex;
  min-height: 100vh;
  background: #f5f7fa;
}

/* 左侧品牌区 */
.login-brand {
  flex: 1;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
}

.brand-content {
  max-width: 480px;
}

.brand-logo {
  width: 80px;
  height: 80px;
  margin-bottom: 24px;
  color: white;
}

.brand-title {
  font-size: 36px;
  font-weight: 700;
  margin: 0 0 8px 0;
}

.brand-subtitle {
  font-size: 18px;
  opacity: 0.9;
  margin: 0 0 24px 0;
}

.brand-desc {
  font-size: 14px;
  line-height: 1.6;
  opacity: 0.85;
  margin: 0 0 32px 0;
}

.brand-features {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.feature-item {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 14px;
  opacity: 0.9;
}

.feature-icon {
  font-size: 18px;
}

/* 右侧登录表单 */
.login-form-wrapper {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 40px;
}

.login-form {
  width: 100%;
  max-width: 420px;
  background: white;
  padding: 40px;
  border-radius: 16px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.08);
}

.form-title {
  font-size: 28px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: #1a1b1c;
}

.form-subtitle {
  font-size: 14px;
  color: #6b7280;
  margin: 0 0 32px 0;
}

.login-form-content {
  margin-top: 0;
}

.form-options {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.forgot-link {
  color: #667eea;
  text-decoration: none;
  font-size: 14px;
}

.forgot-link:hover {
  text-decoration: underline;
}

.login-button {
  width: 100%;
  font-size: 16px;
  font-weight: 500;
}

.error-alert {
  margin-bottom: 16px;
}

.form-footer {
  text-align: center;
  margin-top: 24px;
  font-size: 14px;
  color: #6b7280;
}

.register-link {
  color: #667eea;
  text-decoration: none;
  font-weight: 500;
  margin-left: 4px;
}

.register-link:hover {
  text-decoration: underline;
}

.form-divider {
  display: flex;
  align-items: center;
  margin: 24px 0;
  color: #d1d5db;
  font-size: 12px;
}

.form-divider::before,
.form-divider::after {
  content: '';
  flex: 1;
  height: 1px;
  background: #e5e7eb;
}

.form-divider span {
  padding: 0 16px;
}

.social-login {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.social-button {
  width: 100%;
}

/* 响应式 */
@media (max-width: 768px) {
  .login-container {
    flex-direction: column;
  }

  .login-brand {
    padding: 32px 24px;
  }

  .brand-title {
    font-size: 28px;
  }

  .login-form-wrapper {
    padding: 24px;
  }

  .login-form {
    padding: 24px;
  }
}
</style>
