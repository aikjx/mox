<template>
  <div class="register-container">
    <div class="register-form-wrapper">
      <div class="register-form">
        <div class="form-header">
          <router-link to="/login" class="back-link">
            <el-icon><ArrowLeft /></el-icon>
            返回登录
          </router-link>
          <h2 class="form-title">创建账户</h2>
          <p class="form-subtitle">填写以下信息以注册新账户</p>
        </div>

        <el-form
          ref="registerFormRef"
          :model="registerForm"
          :rules="registerRules"
          class="register-form-content"
          @submit.prevent="handleRegister"
        >
          <el-form-item prop="username">
            <el-input
              v-model="registerForm.username"
              placeholder="用户名"
              size="large"
              :prefix-icon="User"
              clearable
            />
          </el-form-item>

          <el-form-item prop="email">
            <el-input
              v-model="registerForm.email"
              placeholder="邮箱地址"
              size="large"
              :prefix-icon="Message"
              clearable
            />
          </el-form-item>

          <el-form-item prop="password">
            <el-input
              v-model="registerForm.password"
              type="password"
              placeholder="密码"
              size="large"
              :prefix-icon="Lock"
              show-password
            />
          </el-form-item>

          <el-form-item prop="confirm_password">
            <el-input
              v-model="registerForm.confirm_password"
              type="password"
              placeholder="确认密码"
              size="large"
              :prefix-icon="Lock"
              show-password
              @keyup.enter="handleRegister"
            />
          </el-form-item>

          <el-form-item prop="tenant_id">
            <el-input
              v-model="registerForm.tenant_id"
              placeholder="租户 ID（可选，留空使用默认租户）"
              size="large"
              :prefix-icon="OfficeBuilding"
              clearable
            />
          </el-form-item>

          <el-form-item prop="agree_terms">
            <el-checkbox v-model="registerForm.agree_terms">
              我已阅读并同意
              <a href="#" class="terms-link">《服务条款》</a>
              和
              <a href="#" class="terms-link">《隐私政策》</a>
            </el-checkbox>
          </el-form-item>

          <el-form-item>
            <el-button
              type="primary"
              size="large"
              class="register-button"
              :loading="loading"
              @click="handleRegister"
            >
              {{ loading ? '注册中...' : '注 册' }}
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

        <div class="form-footer">
          <span>已有账户？</span>
          <router-link to="/login" class="login-link">立即登录</router-link>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { User, Lock, Message, OfficeBuilding, ArrowLeft } from '@element-plus/icons-vue'
import authApi from '../../api/auth'

const router = useRouter()

const registerFormRef = ref(null)
const loading = ref(false)
const error = ref('')

const registerForm = reactive({
  username: '',
  email: '',
  password: '',
  confirm_password: '',
  tenant_id: '',
  agree_terms: false
})

const validateConfirmPassword = (rule, value, callback) => {
  if (value !== registerForm.password) {
    callback(new Error('两次输入的密码不一致'))
  } else {
    callback()
  }
}

const registerRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 50, message: '用户名长度在 3 到 50 个字符', trigger: 'blur' }
  ],
  email: [
    { required: true, message: '请输入邮箱地址', trigger: 'blur' },
    { type: 'email', message: '请输入正确的邮箱地址', trigger: 'blur' }
  ],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 8, message: '密码长度至少 8 个字符', trigger: 'blur' }
  ],
  confirm_password: [
    { required: true, message: '请确认密码', trigger: 'blur' },
    { validator: validateConfirmPassword, trigger: 'blur' }
  ],
  agree_terms: [
    {
      validator: (rule, value, callback) => {
        if (!value) {
          callback(new Error('请阅读并同意服务条款'))
        } else {
          callback()
        }
      },
      trigger: 'change'
    }
  ]
}

async function handleRegister() {
  if (!registerFormRef.value) return

  try {
    await registerFormRef.value.validate()
  } catch {
    return
  }

  loading.value = true
  error.value = ''

  try {
    await authApi.register({
      username: registerForm.username,
      email: registerForm.email,
      password: registerForm.password,
      tenant_id: registerForm.tenant_id || 'default'
    })

    ElMessage.success('注册成功，请登录')
    router.push('/login')
  } catch (err) {
    error.value = err.message || '注册失败，请稍后重试'
  } finally {
    loading.value = false
  }
}
</script>

<style scoped>
.register-container {
  display: flex;
  min-height: 100vh;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
}

.register-form-wrapper {
  width: 100%;
  max-width: 480px;
}

.register-form {
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
}

.register-button {
  width: 100%;
  font-size: 16px;
  font-weight: 500;
}

.error-alert {
  margin-bottom: 16px;
}

.terms-link {
  color: #667eea;
  text-decoration: none;
}

.terms-link:hover {
  text-decoration: underline;
}

.form-footer {
  text-align: center;
  margin-top: 24px;
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
  .register-form {
    padding: 24px;
  }
}
</style>
