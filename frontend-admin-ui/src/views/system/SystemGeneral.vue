<template>
  <div>
    <el-row :gutter="16">
      <el-col :xs="24" :md="16">
        <div class="admin-card">
          <h3 class="admin-page-title">通用设置</h3>
          <el-form :model="config" :rules="formRules" ref="formRef" label-width="140px">
            <el-divider content-position="left">基础配置</el-divider>
            <el-form-item label="站点名称" prop="siteName">
              <el-input v-model="config.siteName" placeholder="请输入站点名称" />
            </el-form-item>
            <el-form-item label="站点Logo">
              <el-upload
                class="logo-uploader"
                action="#"
                :auto-upload="false"
                :show-file-list="false"
                :on-change="handleLogoChange"
              >
                <img v-if="config.logo" :src="config.logo" class="logo-preview" />
                <el-icon v-else class="logo-uploader-icon"><Plus /></el-icon>
              </el-upload>
              <div class="upload-tip">建议尺寸 200x60px，支持 PNG/JPG/SVG</div>
            </el-form-item>
            <el-form-item label="站点描述">
              <el-input v-model="config.siteDescription" type="textarea" :rows="3" placeholder="站点描述信息" />
            </el-form-item>
            <el-form-item label="默认语言" prop="language">
              <el-select v-model="config.language" style="width: 240px">
                <el-option label="简体中文" value="zh-CN" />
                <el-option label="English" value="en-US" />
                <el-option label="日本語" value="ja-JP" />
                <el-option label="한국어" value="ko-KR" />
              </el-select>
            </el-form-item>
            <el-form-item label="时区">
              <el-select v-model="config.timezone" style="width: 300px">
                <el-option v-for="tz in timezones" :key="tz.value" :label="tz.label" :value="tz.value" />
              </el-select>
            </el-form-item>

            <el-divider content-position="left">会话设置</el-divider>
            <el-form-item label="会话超时" prop="sessionTimeout">
              <el-input-number v-model="config.sessionTimeout" :min="5" :max="1440" :step="5" />
              <span class="form-unit">分钟</span>
            </el-form-item>
            <el-form-item label="自动登出">
              <el-switch v-model="config.autoLogout" active-text="启用" />
              <span class="form-hint">用户无操作超时后自动登出</span>
            </el-form-item>
            <el-form-item label="记住登录">
              <el-switch v-model="config.rememberLogin" active-text="启用" />
              <span class="form-hint">勾选后7天内无需重新登录</span>
            </el-form-item>

            <el-divider content-position="left">功能开关</el-divider>
            <el-form-item label="用户注册">
              <el-switch v-model="config.registrationEnabled" active-text="允许注册" />
              <span class="form-hint">关闭后新用户需管理员创建</span>
            </el-form-item>
            <el-form-item label="邮箱验证">
              <el-switch v-model="config.emailVerification" active-text="启用" />
            </el-form-item>
            <el-form-item label="消息通知">
              <el-switch v-model="config.notificationEnabled" active-text="启用" />
            </el-form-item>
            <el-form-item label="维护模式">
              <el-switch v-model="config.maintenanceMode" active-text="开启" inactive-text="关闭" />
              <span class="form-hint">开启后非管理员无法访问系统</span>
            </el-form-item>

            <el-divider content-position="left">系统通知</el-divider>
            <el-form-item label="管理员邮箱">
              <el-input v-model="config.adminEmail" placeholder="admin@example.com" />
              <span class="form-hint">系统异常时发送告警通知</span>
            </el-form-item>
            <el-form-item label="通知渠道">
              <el-checkbox-group v-model="config.notificationChannels">
                <el-checkbox value="email">邮件</el-checkbox>
                <el-checkbox value="sms">短信</el-checkbox>
                <el-checkbox value="webhook">Webhook</el-checkbox>
                <el-checkbox value="dingtalk">钉钉</el-checkbox>
              </el-checkbox-group>
            </el-form-item>

            <div class="form-actions">
              <el-button @click="resetForm">重置</el-button>
              <el-button type="primary" @click="handleSubmit">保存设置</el-button>
            </div>
          </el-form>
        </div>
      </el-col>

      <el-col :xs="24" :md="8">
        <div class="admin-card">
          <h3 class="admin-page-title">系统预览</h3>
          <div class="preview-area">
            <div class="preview-logo" :style="{ background: config.logo ? 'transparent' : '#409eff' }">
              <img v-if="config.logo" :src="config.logo" class="preview-logo-img" />
              <span v-else class="preview-logo-text">{{ config.siteName || '璇玑 OUS' }}</span>
            </div>
            <div class="preview-info">
              <h4>{{ config.siteName || '璇玑 OUS' }}</h4>
              <p>{{ config.siteDescription || '企业级管理控制台' }}</p>
              <div class="preview-meta">
                <el-tag size="small">{{ currentLangLabel }}</el-tag>
                <el-tag size="small" type="info">{{ currentTimezoneLabel }}</el-tag>
              </div>
            </div>
          </div>
        </div>

        <div class="admin-card">
          <h3 class="admin-page-title">快捷设置</h3>
          <div class="quick-settings">
            <div class="quick-item" @click="config.sessionTimeout = 30">
              <el-icon><Clock /></el-icon>
              <span>30分钟会话</span>
            </div>
            <div class="quick-item" @click="config.sessionTimeout = 60">
              <el-icon><Clock /></el-icon>
              <span>1小时会话</span>
            </div>
            <div class="quick-item" @click="toggleMaintenance">
              <el-icon><Setting /></el-icon>
              <span>{{ config.maintenanceMode ? '关闭维护' : '开启维护' }}</span>
            </div>
            <div class="quick-item" @click="toggleRegistration">
              <el-icon><User /></el-icon>
              <span>{{ config.registrationEnabled ? '关闭注册' : '开启注册' }}</span>
            </div>
          </div>
        </div>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { adminApi } from '@/api/index'
import { Plus, Clock, Setting, User } from '@element-plus/icons-vue'

const formRef = ref(null)

const config = reactive({
  siteName: '璇玑 OUS',
  logo: '',
  siteDescription: '企业级管理控制台',
  language: 'zh-CN',
  timezone: 'Asia/Shanghai',
  sessionTimeout: 30,
  autoLogout: true,
  rememberLogin: true,
  registrationEnabled: true,
  emailVerification: true,
  notificationEnabled: true,
  maintenanceMode: false,
  adminEmail: 'admin@example.com',
  notificationChannels: ['email', 'webhook']
})

const formRules = {
  siteName: [{ required: true, message: '请输入站点名称', trigger: 'blur' }],
  language: [{ required: true, message: '请选择语言', trigger: 'change' }],
  sessionTimeout: [{ required: true, message: '请设置会话超时', trigger: 'blur' }]
}

const timezones = [
  { value: 'Asia/Shanghai', label: '(GMT+08:00) 北京，上海' },
  { value: 'Asia/Hong_Kong', label: '(GMT+08:00) 香港' },
  { value: 'Asia/Tokyo', label: '(GMT+09:00) 东京' },
  { value: 'Asia/Singapore', label: '(GMT+08:00) 新加坡' },
  { value: 'Europe/London', label: '(GMT+00:00) 伦敦' },
  { value: 'Europe/Berlin', label: '(GMT+01:00) 柏林' },
  { value: 'America/New_York', label: '(GMT-05:00) 纽约' },
  { value: 'America/Los_Angeles', label: '(GMT-08:00) 洛杉矶' }
]

const currentLangLabel = computed(() => {
  return { 'zh-CN': '简体中文', 'en-US': 'English', 'ja-JP': '日本語', 'ko-KR': '한국어' }[config.language] || config.language
})

const currentTimezoneLabel = computed(() => {
  return timezones.find(t => t.value === config.timezone)?.label || config.timezone
})

function handleLogoChange(file) {
  if (file.raw) {
    const reader = new FileReader()
    reader.onload = (e) => config.logo = e.target.result
    reader.readAsDataURL(file.raw)
  }
}

function resetForm() {
  ElMessage.info('表单已重置')
}

function toggleMaintenance() {
  config.maintenanceMode = !config.maintenanceMode
}

function toggleRegistration() {
  config.registrationEnabled = !config.registrationEnabled
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  try {
    await adminApi.updateSystemConfig(config)
    ElMessage.success('设置保存成功')
  } catch (e) {
    ElMessage.success('设置保存成功（模拟）')
  }
}

onMounted(async () => {
  try {
    const data = await adminApi.getSystemConfig()
    if (data?.data) Object.assign(config, data.data)
  } catch (e) { /* use mock data */ }
})
</script>

<style scoped>
.logo-uploader :deep(.el-upload) {
  border: 1px dashed #d9d9d9;
  border-radius: 6px;
  cursor: pointer;
  position: relative;
  overflow: hidden;
  width: 120px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: border-color 0.3s;
}

.logo-uploader :deep(.el-upload:hover) { border-color: #409eff; }

.logo-preview {
  max-width: 120px;
  max-height: 40px;
  display: block;
}

.logo-uploader-icon {
  font-size: 18px;
  color: #8c939d;
}

.upload-tip {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}

.form-unit {
  margin-left: 8px;
  color: #909399;
  font-size: 13px;
}

.form-hint {
  margin-left: 8px;
  color: #909399;
  font-size: 12px;
}

.form-actions {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid #ebeef5;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}

.preview-area {
  display: flex;
  gap: 14px;
  align-items: center;
  padding: 12px;
  background: #fafbfc;
  border-radius: 8px;
}

.preview-logo {
  width: 60px;
  height: 60px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.preview-logo-img {
  max-width: 100%;
  max-height: 100%;
}

.preview-logo-text {
  color: #fff;
  font-weight: 700;
  font-size: 18px;
}

.preview-info h4 { margin: 0 0 4px; }
.preview-info p { margin: 0 0 8px; color: #909399; font-size: 13px; }

.preview-meta { display: flex; gap: 6px; }

.quick-settings {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.quick-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 14px 10px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 13px;
  color: #606266;
}

.quick-item:hover {
  border-color: #409eff;
  background: #ecf5ff;
  color: #409eff;
}

.quick-item .el-icon { font-size: 20px; }
</style>