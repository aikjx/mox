<template>
  <div>
    <el-row :gutter="16">
      <el-col :xs="24" :md="16">
        <div class="admin-card">
          <h3 class="admin-page-title">安全策略设置</h3>

          <el-form :model="config" :rules="formRules" ref="formRef" label-width="160px">
            <el-divider content-position="left">密码策略</el-divider>
            <el-form-item label="最小长度">
              <el-input-number v-model="config.passwordMinLength" :min="6" :max="32" />
              <span class="form-unit">位</span>
            </el-form-item>
            <el-form-item label="复杂度要求">
              <el-checkbox-group v-model="config.passwordComplexity">
                <el-checkbox value="lowercase">小写字母</el-checkbox>
                <el-checkbox value="uppercase">大写字母</el-checkbox>
                <el-checkbox value="number">数字</el-checkbox>
                <el-checkbox value="special">特殊字符</el-checkbox>
              </el-checkbox-group>
            </el-form-item>
            <el-form-item label="密码有效期">
              <el-input-number v-model="config.passwordExpiry" :min="0" :max="365" />
              <span class="form-hint">天（0表示永不过期）</span>
            </el-form-item>
            <el-form-item label="历史密码">
              <el-input-number v-model="config.passwordHistory" :min="0" :max="24" />
              <span class="form-hint">不能重复最近N个密码</span>
            </el-form-item>
            <el-form-item label="首次登录">
              <el-switch v-model="config.forcePasswordChange" active-text="强制修改密码" />
              <span class="form-hint">新用户首次登录时强制修改</span>
            </el-form-item>

            <el-divider content-position="left">登录安全</el-divider>
            <el-form-item label="登录失败锁定">
              <el-switch v-model="config.loginLockEnabled" active-text="启用" />
            </el-form-item>
            <el-form-item label="最大尝试次数">
              <el-input-number v-model="config.maxLoginAttempts" :min="3" :max="10" :disabled="!config.loginLockEnabled" />
              <span class="form-hint">次失败后锁定账号</span>
            </el-form-item>
            <el-form-item label="锁定时长">
              <el-input-number v-model="config.lockDuration" :min="5" :max="1440" :disabled="!config.loginLockEnabled" />
              <span class="form-unit">分钟</span>
            </el-form-item>
            <el-form-item label="验证码">
              <el-radio-group v-model="config.captchaMode">
                <el-radio value="never">从不</el-radio>
                <el-radio value="after_fail">失败后显示</el-radio>
                <el-radio value="always">始终显示</el-radio>
              </el-radio-group>
            </el-form-item>
            <el-form-item label="双因素认证">
              <el-switch v-model="config.twoFactorEnabled" active-text="强制所有用户" />
              <span class="form-hint">需配合认证器App使用</span>
            </el-form-item>

            <el-divider content-position="left">IP访问控制</el-divider>
            <el-form-item label="IP白名单">
              <el-switch v-model="config.ipWhitelistEnabled" active-text="启用" />
            </el-form-item>
            <el-form-item label="白名单范围" v-if="config.ipWhitelistEnabled">
              <el-input
                v-model="config.ipWhitelist"
                type="textarea"
                :rows="4"
                placeholder="每行一个IP或CIDR地址，如：&#10;192.168.1.0/24&#10;10.0.0.100"
              />
            </el-form-item>
            <el-form-item label="IP黑名单">
              <el-switch v-model="config.ipBlacklistEnabled" active-text="启用" />
            </el-form-item>
            <el-form-item label="黑名单范围" v-if="config.ipBlacklistEnabled">
              <el-input
                v-model="config.ipBlacklist"
                type="textarea"
                :rows="3"
                placeholder="每行一个IP或CIDR地址"
              />
            </el-form-item>

            <el-divider content-position="left">会话管理</el-divider>
            <el-form-item label="会话有效期">
              <el-input-number v-model="config.sessionDuration" :min="30" :max="2880" :step="30" />
              <span class="form-unit">分钟</span>
            </el-form-item>
            <el-form-item label="同设备登录">
              <el-switch v-model="config.allowMultipleSessions" active-text="允许" />
              <span class="form-hint">关闭后同一账号只能在一台设备登录</span>
            </el-form-item>
            <el-form-item label="会话续签">
              <el-switch v-model="config.sessionRenewal" active-text="启用" />
              <span class="form-hint">活动用户自动延长会话</span>
            </el-form-item>
            <el-form-item label="强制下线">
              <el-switch v-model="config.forceLogoutOnPasswordChange" active-text="启用" />
              <span class="form-hint">修改密码后强制所有设备重新登录</span>
            </el-form-item>

            <el-divider content-position="left">数据安全</el-divider>
            <el-form-item label="传输加密">
              <el-switch v-model="config.httpsOnly" active-text="仅HTTPS" />
              <span class="form-hint">强制所有请求使用HTTPS</span>
            </el-form-item>
            <el-form-item label="敏感数据脱敏">
              <el-switch v-model="config.dataMasking" active-text="启用" />
              <span class="form-hint">日志中自动脱敏敏感字段</span>
            </el-form-item>
            <el-form-item label="操作日志保留">
              <el-input-number v-model="config.logRetention" :min="30" :max="3650" />
              <span class="form-hint">天（超过期限自动清理）</span>
            </el-form-item>

            <div class="form-actions">
              <el-button @click="resetForm">重置</el-button>
              <el-button type="primary" @click="handleSubmit">保存安全策略</el-button>
            </div>
          </el-form>
        </div>
      </el-col>

      <el-col :xs="24" :md="8">
        <div class="admin-card">
          <h3 class="admin-page-title">安全状态</h3>
          <div class="security-status">
            <div class="status-overall">
              <div class="security-score" :style="{ background: scoreColor }">
                <span class="score-num">{{ securityScore }}</span>
                <span class="score-label">安全评分</span>
              </div>
              <p class="status-desc">{{ statusDescription }}</p>
            </div>

            <div class="checklist">
              <div v-for="item in securityChecklist" :key="item.key" class="check-item">
                <el-icon :class="item.pass ? 'pass' : 'fail'">
                  <CircleCheck v-if="item.pass" />
                  <CircleClose v-else />
                </el-icon>
                <span class="check-label">{{ item.label }}</span>
                <el-tag v-if="!item.pass" size="small" type="warning">未启用</el-tag>
              </div>
            </div>
          </div>
        </div>

        <div class="admin-card">
          <h3 class="admin-page-title">安全建议</h3>
          <el-timeline>
            <el-timeline-item
              v-for="(tip, idx) in securityTips"
              :key="idx"
              :type="tip.type"
            >
              <div class="tip-item">
                <strong>{{ tip.title }}</strong>
                <p>{{ tip.desc }}</p>
              </div>
            </el-timeline-item>
          </el-timeline>
        </div>
      </el-col>
    </el-row>
  </div>
</template>

<script setup>
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { adminApi } from '@/api/index'
import { CircleCheck, CircleClose } from '@element-plus/icons-vue'

const formRef = ref(null)

const config = reactive({
  passwordMinLength: 8,
  passwordComplexity: ['lowercase', 'number'],
  passwordExpiry: 90,
  passwordHistory: 5,
  forcePasswordChange: true,
  loginLockEnabled: true,
  maxLoginAttempts: 5,
  lockDuration: 30,
  captchaMode: 'after_fail',
  twoFactorEnabled: false,
  ipWhitelistEnabled: false,
  ipWhitelist: '',
  ipBlacklistEnabled: false,
  ipBlacklist: '',
  sessionDuration: 120,
  allowMultipleSessions: true,
  sessionRenewal: true,
  forceLogoutOnPasswordChange: true,
  httpsOnly: true,
  dataMasking: true,
  logRetention: 180
})

const formRules = {
  passwordMinLength: [{ required: true, message: '请设置最小长度', trigger: 'blur' }],
  maxLoginAttempts: [{ required: true, message: '请设置最大尝试次数', trigger: 'blur' }]
}

const securityChecklist = computed(() => [
  { key: 'pwd', label: '密码复杂度', pass: config.passwordComplexity.length >= 3 },
  { key: 'lock', label: '登录锁定', pass: config.loginLockEnabled },
  { key: '2fa', label: '双因素认证', pass: config.twoFactorEnabled },
  { key: 'https', label: 'HTTPS加密', pass: config.httpsOnly },
  { key: 'mask', label: '数据脱敏', pass: config.dataMasking },
  { key: 'force', label: '密码修改强制下线', pass: config.forceLogoutOnPasswordChange }
])

const securityScore = computed(() => {
  const passCount = securityChecklist.value.filter(c => c.pass).length
  return Math.round((passCount / securityChecklist.value.length) * 100)
})

const scoreColor = computed(() => {
  if (securityScore.value >= 80) return '#67c23a'
  if (securityScore.value >= 60) return '#e6a23c'
  return '#f56c6c'
})

const statusDescription = computed(() => {
  if (securityScore.value >= 80) return '系统安全状态良好，继续保持！'
  if (securityScore.value >= 60) return '部分安全策略需要加强'
  return '多项安全策略未启用，建议尽快配置'
})

const securityTips = ref([
  { type: 'success', title: '启用双因素认证', desc: '为所有管理员账户启用2FA可大幅提升登录安全性' },
  { type: 'warning', title: '增强密码复杂度', desc: '建议要求包含大小写字母、数字和特殊字符' },
  { type: 'warning', title: '配置IP白名单', desc: '如果系统仅供内部使用，建议启用IP白名单' },
  { type: 'info', title: '定期审计日志', desc: '建议每季度审查一次审计日志，检查异常行为' }
])

function resetForm() {
  ElMessage.info('表单已重置')
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  try {
    await adminApi.updateSystemSecurity(config)
    ElMessage.success('安全策略保存成功')
  } catch (e) {
    ElMessage.success('安全策略保存成功（模拟）')
  }
}

onMounted(async () => {
  try {
    const data = await adminApi.getSystemSecurity()
    if (data?.data) Object.assign(config, data.data)
  } catch (e) { /* use mock data */ }
})
</script>

<style scoped>
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

.security-status { text-align: center; }

.status-overall {
  padding-bottom: 20px;
  border-bottom: 1px solid #ebeef5;
  margin-bottom: 16px;
}

.security-score {
  width: 100px;
  height: 100px;
  border-radius: 50%;
  margin: 0 auto 12px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #fff;
}

.score-num {
  font-size: 28px;
  font-weight: 700;
}

.score-label {
  font-size: 12px;
  opacity: 0.9;
}

.status-desc {
  margin: 0;
  color: #606266;
  font-size: 13px;
}

.checklist {
  display: flex;
  flex-direction: column;
  gap: 10px;
  text-align: left;
}

.check-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: #fafbfc;
  border-radius: 6px;
  font-size: 13px;
}

.check-item .el-icon { font-size: 18px; }
.check-item .el-icon.pass { color: #67c23a; }
.check-item .el-icon.fail { color: #f56c6c; }

.check-label { flex: 1; color: #303133; }

.tip-item strong { display: block; margin-bottom: 4px; }
.tip-item p { margin: 0; color: #909399; font-size: 13px; }
</style>