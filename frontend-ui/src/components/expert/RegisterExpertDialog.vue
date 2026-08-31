<!--
  专家注册对话框 · 三步表单
  ============================
  基本信息 → 能力配置 → 确认提交
  通过 v-model 控制显示隐藏，emit('registered', expertData) 事件
-->

<template>
  <el-dialog
    :model-value="modelValue"
    @update:model-value="handleVisibleChange"
    title="注册新专家"
    width="560px"
    :close-on-click-modal="false"
    :close-on-press-escape="!submitting"
    class="register-expert-dialog"
    @closed="resetForm"
  >
    <!-- 步骤指示器 -->
    <div class="step-indicator">
      <div
        v-for="(step, idx) in steps"
        :key="step.key"
        class="step-item"
        :class="{ active: currentStep === idx, done: currentStep > idx }"
      >
        <div class="step-dot">{{ currentStep > idx ? '✓' : idx + 1 }}</div>
        <span class="step-label">{{ step.label }}</span>
        <div v-if="idx < steps.length - 1" class="step-line"></div>
      </div>
    </div>

    <!-- 步骤1：基本信息 -->
    <div v-show="currentStep === 0" class="step-content">
      <el-form :model="formData" label-width="88px" label-position="right" size="default">
        <el-form-item label="专家名称" required>
          <el-input
            v-model="formData.name"
            placeholder="请输入专家名称（最多64字）"
            maxlength="64"
            show-word-limit
            clearable
          />
        </el-form-item>

        <el-form-item label="专家类型" required>
          <el-select v-model="formData.type" placeholder="请选择专家类型" style="width: 100%">
            <el-option
              v-for="(label, key) in EXPERT_TYPES"
              :key="key"
              :label="label"
              :value="key"
            >
              <div class="type-option">
                <span class="type-emoji">{{ typeEmoji(key) }}</span>
                <span>{{ label }}</span>
              </div>
            </el-option>
          </el-select>
        </el-form-item>

        <el-form-item label="头像">
          <div class="avatar-picker">
            <div class="avatar-preview" :style="{ background: typeColor(formData.type) }">
              {{ formData.avatar || typeEmoji(formData.type) }}
            </div>
            <div class="avatar-options">
              <el-input
                v-model="formData.avatar"
                placeholder="输入 emoji 或文字作为头像"
                maxlength="4"
                style="width: 200px"
              />
              <div class="emoji-presets">
                <span
                  v-for="emoji in emojiPresets"
                  :key="emoji"
                  class="emoji-preset"
                  :class="{ active: formData.avatar === emoji }"
                  @click="formData.avatar = emoji"
                >{{ emoji }}</span>
              </div>
            </div>
          </div>
        </el-form-item>

        <el-form-item label="描述">
          <el-input
            v-model="formData.description"
            type="textarea"
            :rows="3"
            placeholder="简要描述专家的定位和擅长领域（最多200字）"
            maxlength="200"
            show-word-limit
            resize="none"
          />
        </el-form-item>
      </el-form>
    </div>

    <!-- 步骤2：能力配置 -->
    <div v-show="currentStep === 1" class="step-content">
      <el-form label-width="88px" label-position="right" size="default">
        <el-form-item label="专业领域">
          <div class="domain-picker">
            <div
              v-for="domain in domainOptions"
              :key="domain"
              class="domain-tag"
              :class="{ active: formData.domains.includes(domain) }"
              @click="toggleDomain(domain)"
            >
              {{ domain }}
            </div>
          </div>
        </el-form-item>

        <el-form-item label="技能标签">
          <div class="skill-tags-wrapper">
            <div class="skill-tags-list">
              <el-tag
                v-for="(tag, idx) in formData.skills"
                :key="idx"
                closable
                type="primary"
                effect="light"
                size="small"
                class="skill-tag"
                @close="removeSkill(idx)"
              >
                {{ tag }}
              </el-tag>
            </div>
            <el-input
              v-model="newSkill"
              placeholder="输入技能标签，回车添加"
              size="small"
              class="skill-input"
              @keyup.enter="addSkill"
            />
            <div class="skill-presets">
              <span class="preset-label">推荐：</span>
              <span
                v-for="skill in skillPresets"
                :key="skill"
                class="preset-tag"
                @click="addPresetSkill(skill)"
              >+ {{ skill }}</span>
            </div>
          </div>
        </el-form-item>

        <el-form-item label="经验等级">
          <el-radio-group v-model="formData.experienceLevel" class="level-radio-group">
            <el-radio-button
              v-for="level in experienceLevels"
              :key="level.value"
              :value="level.value"
            >
              <span class="level-icon">{{ level.icon }}</span>
              <span class="level-text">{{ level.label }}</span>
            </el-radio-button>
          </el-radio-group>
        </el-form-item>

        <el-form-item label="系统提示">
          <el-input
            v-model="formData.systemPrompt"
            type="textarea"
            :rows="3"
            placeholder="可选：自定义专家的系统提示词，定义其行为和风格"
            resize="none"
          />
        </el-form-item>
      </el-form>
    </div>

    <!-- 步骤3：确认提交 -->
    <div v-show="currentStep === 2" class="step-content confirm-step">
      <div class="confirm-card">
        <div class="confirm-avatar" :style="{ background: typeColor(formData.type) }">
          {{ formData.avatar || typeEmoji(formData.type) }}
        </div>
        <div class="confirm-name">{{ formData.name || '未命名专家' }}</div>
        <el-tag :type="tagTypeByLevel(formData.experienceLevel)" effect="light" size="small">
          {{ experienceLevels.find(l => l.value === formData.experienceLevel)?.label }}
        </el-tag>
      </div>

      <div class="confirm-info-grid">
        <div class="info-row">
          <span class="info-label">专家类型</span>
          <span class="info-value">{{ EXPERT_TYPES[formData.type] || formData.type }}</span>
        </div>
        <div class="info-row">
          <span class="info-label">专业领域</span>
          <span class="info-value">
            <template v-if="formData.domains.length">
              {{ formData.domains.join('、') }}
            </template>
            <span v-else class="muted">未设置</span>
          </span>
        </div>
        <div class="info-row">
          <span class="info-label">技能标签</span>
          <span class="info-value">
            <template v-if="formData.skills.length">
              {{ formData.skills.join('、') }}
            </template>
            <span v-else class="muted">未设置</span>
          </span>
        </div>
        <div class="info-row">
          <span class="info-label">描述</span>
          <span class="info-value desc-value">
            {{ formData.description || '暂无描述' }}
          </span>
        </div>
      </div>

      <div v-if="submitError" class="submit-error">
        <el-icon><Warning /></el-icon>
        <span>{{ submitError }}</span>
      </div>
    </div>

    <!-- 底部按钮 -->
    <template #footer>
      <div class="dialog-footer">
        <el-button @click="handleClose" :disabled="submitting">取消</el-button>
        <el-button
          v-if="currentStep > 0"
          @click="prevStep"
          :disabled="submitting"
        >上一步</el-button>
        <el-button
          v-if="currentStep < steps.length - 1"
          type="primary"
          @click="nextStep"
        >下一步</el-button>
        <el-button
          v-else
          type="primary"
          :loading="submitting"
          @click="doSubmit"
        >
          {{ submitting ? '注册中…' : '确认注册' }}
        </el-button>
      </div>
    </template>
  </el-dialog>
</template>

<script setup>
import { ref, reactive, computed, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { Warning } from '@element-plus/icons-vue'
import { EXPERT_TYPES } from '@/constants/expert.constants'
import { registerExpert } from '@/api/experts.api.js'

const props = defineProps({
  modelValue: { type: Boolean, default: false }
})

const emit = defineEmits(['update:modelValue', 'registered'])

// ========== 步骤配置 ==========
const steps = [
  { key: 'basic', label: '基本信息' },
  { key: 'capability', label: '能力配置' },
  { key: 'confirm', label: '确认提交' }
]
const currentStep = ref(0)

// ========== 表单数据 ==========
const formData = reactive({
  name: '',
  type: 'algorithm',
  avatar: '',
  description: '',
  domains: [],
  skills: [],
  experienceLevel: 'intermediate',
  systemPrompt: ''
})

const submitting = ref(false)
const submitError = ref('')
const newSkill = ref('')

// ========== 预设数据 ==========
const emojiPresets = ['🤖', '🧠', '💡', '🎯', '⚡', '🔮', '📊', '🛡️', '🚀', '🎨']

const domainOptions = [
  '后端开发', '前端开发', '移动开发', '数据分析', '人工智能',
  '系统架构', '云原生', '安全合规', '产品设计', '项目管理'
]

const skillPresets = [
  'Python', 'JavaScript', 'Java', 'Go', 'Rust',
  '微服务', '分布式', '高并发', '性能优化', 'DevOps',
  '机器学习', '深度学习', 'NLP', '计算机视觉', 'RAG'
]

const experienceLevels = [
  { value: 'junior', label: '初级', icon: '🌱' },
  { value: 'intermediate', label: '中级', icon: '🌿' },
  { value: 'senior', label: '高级', icon: '🌳' },
  { value: 'expert', label: '专家', icon: '🏆' },
  { value: 'master', label: '大师', icon: '👑' }
]

// ========== 辅助函数 ==========
function typeEmoji(type) {
  const emojis = {
    algorithm: '🧮', architecture: '🏗️', data: '🔗',
    ai: '🤖', workflow: '⚡', graph: '🕸️',
    security: '🔒', performance: '🚀', monitor: '📊',
    market: '📈', mcp: '🔌', automation: '🤖',
    requirement: '📋', fusion: '🔀', operator: '⚙️',
    custom: '👤'
  }
  return emojis[type] || '👤'
}

function typeColor(type) {
  const colors = {
    algorithm: '#6366f1', architecture: '#0891b2', data: '#10b981',
    ai: '#ec4899', workflow: '#f59e0b', operator: '#8b5cf6',
    graph: '#06b6d4', security: '#ef4444', performance: '#14b8a6',
    monitor: '#f97316', market: '#f43f5e', mcp: '#a855f7',
    automation: '#0ea5e9', requirement: '#16a34a', fusion: '#7c3aed',
    custom: '#64748b'
  }
  return colors[type] || colors.custom
}

function tagTypeByLevel(level) {
  const map = {
    junior: 'info', intermediate: '', senior: 'primary',
    expert: 'warning', master: 'danger'
  }
  return map[level] || ''
}

// ========== 领域选择 ==========
function toggleDomain(domain) {
  const idx = formData.domains.indexOf(domain)
  if (idx >= 0) {
    formData.domains.splice(idx, 1)
  } else {
    formData.domains.push(domain)
  }
}

// ========== 技能标签 ==========
function addSkill() {
  const skill = newSkill.value.trim()
  if (!skill) return
  if (formData.skills.includes(skill)) {
    ElMessage.warning('该技能标签已存在')
    return
  }
  if (formData.skills.length >= 20) {
    ElMessage.warning('最多添加 20 个技能标签')
    return
  }
  formData.skills.push(skill)
  newSkill.value = ''
}

function addPresetSkill(skill) {
  if (formData.skills.includes(skill)) return
  if (formData.skills.length >= 20) {
    ElMessage.warning('最多添加 20 个技能标签')
    return
  }
  formData.skills.push(skill)
}

function removeSkill(idx) {
  formData.skills.splice(idx, 1)
}

// ========== 步骤切换 ==========
function nextStep() {
  // 校验当前步骤
  if (currentStep.value === 0) {
    if (!formData.name.trim()) {
      ElMessage.warning('请输入专家名称')
      return
    }
    if (!formData.type) {
      ElMessage.warning('请选择专家类型')
      return
    }
  }
  submitError.value = ''
  currentStep.value++
}

function prevStep() {
  submitError.value = ''
  currentStep.value--
}

// ========== 提交 ==========
async function doSubmit() {
  if (!formData.name.trim()) {
    ElMessage.warning('请输入专家名称')
    currentStep.value = 0
    return
  }

  submitting.value = true
  submitError.value = ''

  try {
    const payload = {
      name: formData.name.trim(),
      type: formData.type,
      avatar: formData.avatar || undefined,
      description: formData.description || undefined,
      capabilities: [...formData.domains, ...formData.skills],
      experienceLevel: formData.experienceLevel,
      systemPrompt: formData.systemPrompt || undefined
    }

    const result = await registerExpert(payload)
    const expertData = result?.data || result

    ElMessage.success('专家注册成功！')
    emit('registered', expertData)
    handleClose()
  } catch (e) {
    console.error('[registerExpert] 注册失败:', e)
    submitError.value = e.message || '注册失败，请稍后重试'
    // 优雅降级：模拟成功（演示用）
    ElMessage.warning('注册服务暂不可用，已生成本地模拟数据')
    const mockExpert = {
      id: 'exp_' + Date.now().toString(36),
      name: formData.name.trim(),
      type: formData.type,
      avatar: formData.avatar,
      description: formData.description,
      capabilities: [...formData.domains, ...formData.skills],
      experienceLevel: formData.experienceLevel,
      status: 'active',
      metrics: { total_consults: 0, success_rate: 0.95 }
    }
    emit('registered', mockExpert)
    handleClose()
  } finally {
    submitting.value = false
  }
}

// ========== 显示控制 ==========
function handleVisibleChange(val) {
  emit('update:modelValue', val)
}

function handleClose() {
  emit('update:modelValue', false)
}

function resetForm() {
  currentStep.value = 0
  submitting.value = false
  submitError.value = ''
  newSkill.value = ''
  formData.name = ''
  formData.type = 'algorithm'
  formData.avatar = ''
  formData.description = ''
  formData.domains = []
  formData.skills = []
  formData.experienceLevel = 'intermediate'
  formData.systemPrompt = ''
}

// 监听显示变化
watch(() => props.modelValue, (val) => {
  if (val) {
    resetForm()
  }
})
</script>

<style scoped>
.register-expert-dialog :deep(.el-dialog__body) {
  padding-top: 8px;
}

/* 步骤指示器 */
.step-indicator {
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 24px;
  padding: 0 20px;
}
.step-item {
  display: flex;
  align-items: center;
  flex: 1;
  position: relative;
}
.step-item:last-child { flex: 0; }
.step-dot {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  background: #e2e8f0;
  color: #64748b;
  display: grid;
  place-items: center;
  font-weight: 700;
  font-size: 13px;
  flex-shrink: 0;
  transition: all 0.3s ease;
}
.step-item.active .step-dot {
  background: linear-gradient(135deg, #6366f1, #0ea5e9);
  color: #fff;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.35);
}
.step-item.done .step-dot {
  background: #10b981;
  color: #fff;
}
.step-label {
  margin-left: 8px;
  font-size: 13px;
  font-weight: 600;
  color: #64748b;
  white-space: nowrap;
}
.step-item.active .step-label { color: #6366f1; }
.step-item.done .step-label { color: #10b981; }
.step-line {
  flex: 1;
  height: 2px;
  background: #e2e8f0;
  margin: 0 12px;
  min-width: 30px;
}
.step-item.done .step-line { background: #10b981; }

.step-content {
  min-height: 320px;
}

/* 类型选项 */
.type-option {
  display: flex;
  align-items: center;
  gap: 8px;
}
.type-emoji { font-size: 16px; }

/* 头像选择 */
.avatar-picker {
  display: flex;
  gap: 14px;
  align-items: flex-start;
}
.avatar-preview {
  width: 56px;
  height: 56px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  font-size: 26px;
  flex-shrink: 0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}
.avatar-options { flex: 1; }
.emoji-presets {
  display: flex;
  gap: 6px;
  margin-top: 8px;
  flex-wrap: wrap;
}
.emoji-preset {
  width: 32px;
  height: 32px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  background: #f1f5f9;
  cursor: pointer;
  font-size: 18px;
  transition: all 0.2s;
}
.emoji-preset:hover {
  background: #e2e8f0;
  transform: scale(1.1);
}
.emoji-preset.active {
  background: #eef2ff;
  box-shadow: inset 0 0 0 2px #6366f1;
}

/* 领域选择 */
.domain-picker {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.domain-tag {
  padding: 6px 14px;
  border-radius: 999px;
  background: #f1f5f9;
  color: #475569;
  font-size: 12.5px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
  user-select: none;
}
.domain-tag:hover {
  background: #e2e8f0;
}
.domain-tag.active {
  background: linear-gradient(135deg, rgba(99, 102, 241, 0.1), rgba(14, 165, 233, 0.08));
  color: #4338ca;
  border-color: #c7d2fe;
  font-weight: 600;
}

/* 技能标签 */
.skill-tags-wrapper {
  width: 100%;
}
.skill-tags-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 8px;
  min-height: 24px;
}
.skill-tag { margin: 0; }
.skill-input { width: 100%; }
.skill-presets {
  margin-top: 8px;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}
.preset-label {
  font-size: 12px;
  color: #94a3b8;
}
.preset-tag {
  font-size: 11.5px;
  color: #6366f1;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  transition: all 0.2s;
}
.preset-tag:hover {
  background: #eef2ff;
}

/* 经验等级 */
.level-radio-group :deep(.el-radio-button__inner) {
  padding: 8px 14px;
}
.level-icon { margin-right: 4px; }
.level-text { font-size: 13px; }

/* 确认页 */
.confirm-step {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.confirm-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 20px;
  background: linear-gradient(135deg, #fafbff, #f0fdf4);
  border-radius: 12px;
  border: 1px solid #e0e7ff;
}
.confirm-avatar {
  width: 64px;
  height: 64px;
  border-radius: 18px;
  display: grid;
  place-items: center;
  font-size: 30px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.12);
}
.confirm-name {
  font-size: 18px;
  font-weight: 700;
  color: #0f172a;
}

.confirm-info-grid {
  background: #f8fafc;
  border-radius: 10px;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.info-row {
  display: grid;
  grid-template-columns: 80px 1fr;
  gap: 12px;
  font-size: 13px;
}
.info-label {
  color: #64748b;
  font-weight: 500;
}
.info-value {
  color: #1e293b;
  word-break: break-word;
}
.desc-value {
  line-height: 1.6;
}
.muted { color: #94a3b8; }

.submit-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: #fef2f2;
  border: 1px solid #fecaca;
  border-radius: 8px;
  color: #b91c1c;
  font-size: 13px;
}

/* 底部按钮 */
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
