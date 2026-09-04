<template>
  <div class="forgot-container">
    <div class="forgot-form-wrapper">
      <div class="forgot-form">
        <div class="form-header">
          <router-link to="/login" class="back-link">
            <el-icon><ArrowLeft /></el-icon>
            返回登录
          </router-link>
          <h2 class="form-title">忘记密码</h2>
          <p class="form-subtitle">输入您的邮箱地址，我们将发送重置密码链接</p>
        </div>

        <!-- 步骤1：输入邮箱 -->
        <el-form
          v-if="step === 1"
          ref="emailFormRef"
          :model="emailForm"
          :rules="emailRules"
          class="forgot-form-content"
          @submit.prevent="handleSendEmail"
        >
          <el-form-item prop="email">
            <el-input
              v-model="emailForm.email"
              placeholder="邮箱地址"
              size="large"
              :prefix-icon="Message"
              clearable
              @keyup.enter="handleSendEmail"
            />
          </el-form-item>

          <el-form-item>
            <el-button
              type="primary"
              size="large"
              class="submit-button"
              :loading="loading"
              @click="handleSendEmail"
            >
              {{ loading ? '发送中...' : '发送重置链接' }}
            </el-button>
          </el-form-item>

          <el-alert
            v-if="error"
            :title="error"
            type="error"
            show-icon
            :closable="false"
            class="error-alert"
          />
        </el-form>

        <!-- 步骤2：发送成功 -->
        <div v-if="step === 2" class="success-content">
          <div class="success-icon">
            <el-icon :size="64" color="#52c41a"><CircleCheckFilled /></el-icon>
          </div>
          <h3 class="success-title">邮件已发送</h3>
          <p class="success-desc">
            重置密码链接已发送至 <strong>{{ emailForm.email }}</strong>，
            请在 30 分钟内点击链接完成密码重置。
          </p>
          <el-alert
            title="如果没有收到邮件，请检查垃圾邮件文件夹，或确认邮箱地址是否正确。"
            type="info"
            :closable="false"
            class="info-alert"
          />
          <div class="success-actions">
            <el-button size="large" @click="step = 1">重新发送</el-button>
            <el-button type="primary" size="large" @click="$router.push('/login')">
              返回登录
            </el-button>
          </div>
        </div>

        <div class="form-footer">
          <span>想起密码了？</span>
          <router-link to="/login" class="login-link">立即登录</router-link>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { Message, ArrowLeft, CircleCheckFilled } from '@element-plus/icons-vue'
import authApi from '../../api/auth'

const emailFormRef = ref(null)
const loading = ref(false)
const error = ref('')
const step = ref(1)

const emailForm = reactive({
  email: ''
})

const emailRules = {
  email: [
    { required: true, message: '请输入邮箱地址', trigger: 'blur' },
    { type: 'email', message: '请输入正确的邮箱地址', trigger: 'blur' }
  ]
}

async function handleSendEmail() {
  if (!emailFormRef.value) return

  try {
    await emailFormRef.value.validate()
  } catch {
    return
  }

  loading.value = true
  error.value = ''

  try {
    await authApi.forgotPassword(emailForm.email)
    ElMessage.success('重置密码邮件已发送')
    step.value = 2
  } catch (err) {
    error.value = err.message || '发送失败，请稍后重试'
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.forgot-container {
  display: flex;
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}

.forgot-form-wrapper {
  width: 100%;
  max-width: 440px;
}

.forgot-form {
  background: white;
  padding: 40px;
  border-radius: 16px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.12);
}

.form-header {
  margin-bottom: 32px;
}

.back-link {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #6b7280;
  text-decoration: none;
  font-size: 14px;
  margin-bottom: 16px;
}

.back-link:hover {
  color: #667eea;
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
  margin: 0;
  line-height: 1.6;
}

.submit-button {
  width: 100%;
  font-size: 16px;
  font-weight: 500;
}

.error-alert {
  margin-bottom: 16px;
}

/* 成功状态 */
.success-content {
  text-align: center;
}

.success-icon {
  margin-bottom: 24px;
}

.success-title {
  font-size: 24px;
  font-weight: 600;
  color: #1a1b1c;
  margin: 0 0 12px 0;
}

.success-desc {
  font-size: 14px;
  color: #6b7280;
  line-height: 1.6;
  margin: 0 0 24px 0;
}

.info-alert {
  margin-bottom: 24px;
  text-align: left;
}

.success-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
}

.form-footer {
  text-align: center;
  margin-top: 32px;
  font-size: 14px;
  color: #6b7280;
}

.login-link {
  color: #667eea;
  text-decoration: none;
  font-weight: 500;
  margin-left: 4px;
}

.login-link:hover {
  text-decoration: underline;
}

@media (max-width: 480px) {
  .forgot-form {
    padding: 24px;
  }
}
</style>
