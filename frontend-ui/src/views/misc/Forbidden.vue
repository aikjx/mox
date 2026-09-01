<template>
  <div class="forbidden-page">
    <div class="forbidden-card">
      <!-- 大图标 + 403 -->
      <div class="forbidden-visual">
        <div class="icon-wrapper">
          <el-icon class="lock-icon"><Lock /></el-icon>
        </div>
        <div class="code-403">403</div>
      </div>

      <!-- 标题和描述 -->
      <h1 class="forbidden-title">抱歉，您没有访问权限</h1>
      <p class="forbidden-desc">
        该页面需要特定的权限或角色才能访问。
        <br />
        如需访问，请联系系统管理员申请相应权限。
      </p>

      <!-- 操作按钮 -->
      <div class="forbidden-actions">
        <el-button type="primary" @click="goHome">
          <el-icon><House /></el-icon>
          <span>返回首页</span>
        </el-button>
        <el-button @click="showPermDialog = true">
          <el-icon><Key /></el-icon>
          <span>查看我的权限</span>
        </el-button>
        <el-button @click="goBack">
          <el-icon><Back /></el-icon>
          <span>返回上一页</span>
        </el-button>
      </div>

      <!-- 联系管理员提示 -->
      <div class="admin-hint">
        <el-icon class="hint-icon"><InfoFilled /></el-icon>
        <span>如需申请权限，请联系管理员或发送邮件至 <a href="mailto:admin@mox.local">admin@mox.local</a></span>
      </div>
    </div>

    <!-- 我的权限弹窗 -->
    <el-dialog
      v-model="showPermDialog"
      title="我的权限信息"
      width="600px"
      :close-on-click-modal="false"
      class="perm-dialog"
    >
      <el-tabs v-model="activeTab">
        <!-- 角色列表 -->
        <el-tab-pane label="角色" name="roles">
          <div class="perm-section">
            <div class="perm-count">共 {{ permissionStore.roles.length }} 个角色</div>
            <div class="role-list" v-if="permissionStore.roles.length">
              <el-tag
                v-for="role in permissionStore.roles"
                :key="role"
                :type="isAdminRole(role) ? 'danger' : 'primary'"
                effect="light"
                size="large"
                class="role-tag"
              >
                <el-icon><UserFilled /></el-icon>
                <span>{{ roleLabel(role) }}</span>
              </el-tag>
            </div>
            <el-empty v-else description="暂无角色" :image-size="80" />
          </div>
        </el-tab-pane>

        <!-- 权限列表 -->
        <el-tab-pane label="权限" name="permissions">
          <div class="perm-section">
            <div class="perm-count">
              共 {{ permissionStore.permissions.length }} 项权限
              <span v-if="permissionStore.isAdmin" class="admin-badge">
                <el-icon><Trophy /></el-icon>
                超级管理员拥有全部权限
              </span>
            </div>
            <div class="perm-search" v-if="!permissionStore.isAdmin">
              <el-input
                v-model="permSearch"
                placeholder="搜索权限标识..."
                clearable
                size="default"
              >
                <template #prefix>
                  <el-icon><Search /></el-icon>
                </template>
              </el-input>
            </div>
            <div class="perm-list" v-if="filteredPermissions.length">
              <div
                v-for="perm in filteredPermissions"
                :key="perm"
                class="perm-item"
              >
                <el-icon class="perm-icon"><Check /></el-icon>
                <span class="perm-code">{{ perm }}</span>
              </div>
            </div>
            <el-empty v-else description="暂无权限数据" :image-size="80" />
          </div>
        </el-tab-pane>

        <!-- 数据权限 -->
        <el-tab-pane label="数据权限" name="dataScope">
          <div class="perm-section">
            <el-descriptions :column="1" border size="default">
              <el-descriptions-item label="数据权限范围">
                <el-tag :type="dataScopeType" effect="light">
                  {{ permissionStore.dataScopeLabel }}
                </el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="所属部门">
                {{ permissionStore.deptId || '未设置' }}
              </el-descriptions-item>
              <el-descriptions-item label="自定义部门" v-if="permissionStore.dataScope === 'custom'">
                <div v-if="permissionStore.customDeptIds.length" class="custom-dept-list">
                  <el-tag
                    v-for="id in permissionStore.customDeptIds"
                    :key="id"
                    size="small"
                    type="info"
                  >
                    {{ id }}
                  </el-tag>
                </div>
                <span v-else>无</span>
              </el-descriptions-item>
            </el-descriptions>
          </div>
        </el-tab-pane>
      </el-tabs>

      <template #footer>
        <el-button type="primary" @click="showPermDialog = false">我知道了</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  Lock,
  House,
  Key,
  Back,
  InfoFilled,
  UserFilled,
  Trophy,
  Search,
  Check,
} from '@element-plus/icons-vue'
import { usePermissionStore } from '@/stores/permission.store'

const router = useRouter()
const route = useRoute()
const permissionStore = usePermissionStore()

const showPermDialog = ref(false)
const activeTab = ref('roles')
const permSearch = ref('')

// 过滤后的权限列表
const filteredPermissions = computed(() => {
  if (!permSearch.value) return permissionStore.permissions
  const keyword = permSearch.value.toLowerCase()
  return permissionStore.permissions.filter(p => p.toLowerCase().includes(keyword))
})

// 数据权限类型映射
const dataScopeType = computed(() => {
  const map = {
    all: 'success',
    dept: 'primary',
    deptAndBelow: 'warning',
    self: 'info',
    custom: 'danger',
  }
  return map[permissionStore.dataScope] || 'info'
})

// 是否管理员角色
function isAdminRole(role) {
  return role === 'admin' || role === 'super_admin'
}

// 角色中文名（简单映射）
function roleLabel(role) {
  const map = {
    admin: '系统管理员',
    super_admin: '超级管理员',
    developer: '开发者',
    user: '普通用户',
    manager: '部门经理',
    auditor: '审计员',
    operator: '运维人员',
  }
  return map[role] || role
}

// 返回首页
function goHome() {
  router.push('/dashboard')
}

// 返回上一页
function goBack() {
  if (window.history.length > 1) {
    router.back()
  } else {
    goHome()
  }
}
</script>

<style scoped>
.forbidden-page {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #0f172a 0%, #1e293b 50%, #0f172a 100%);
  padding: 20px;
  position: relative;
  overflow: hidden;
}

/* 背景装饰 */
.forbidden-page::before {
  content: '';
  position: absolute;
  width: 600px;
  height: 600px;
  background: radial-gradient(circle, rgba(99, 102, 241, 0.1) 0%, transparent 70%);
  top: -200px;
  right: -200px;
  pointer-events: none;
}

.forbidden-page::after {
  content: '';
  position: absolute;
  width: 500px;
  height: 500px;
  background: radial-gradient(circle, rgba(236, 72, 153, 0.08) 0%, transparent 70%);
  bottom: -150px;
  left: -150px;
  pointer-events: none;
}

.forbidden-card {
  background: rgba(30, 41, 59, 0.8);
  backdrop-filter: blur(20px);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 20px;
  padding: 60px 50px;
  text-align: center;
  max-width: 560px;
  width: 100%;
  position: relative;
  z-index: 1;
  box-shadow:
    0 20px 60px rgba(0, 0, 0, 0.4),
    0 0 0 1px rgba(255, 255, 255, 0.04) inset;
}

/* 视觉区 */
.forbidden-visual {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 24px;
  margin-bottom: 32px;
}

.icon-wrapper {
  width: 80px;
  height: 80px;
  border-radius: 20px;
  background: linear-gradient(135deg, #ef4444 0%, #dc2626 100%);
  display: grid;
  place-items: center;
  box-shadow: 0 8px 24px rgba(239, 68, 68, 0.35);
}

.lock-icon {
  font-size: 40px;
  color: #fff;
}

.code-403 {
  font-size: 72px;
  font-weight: 800;
  background: linear-gradient(135deg, #ef4444, #f97316);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  letter-spacing: -2px;
  line-height: 1;
}

/* 标题描述 */
.forbidden-title {
  font-size: 24px;
  font-weight: 700;
  color: #f1f5f9;
  margin: 0 0 12px;
}

.forbidden-desc {
  font-size: 14px;
  color: #94a3b8;
  line-height: 1.7;
  margin: 0 0 32px;
}

/* 操作按钮 */
.forbidden-actions {
  display: flex;
  gap: 12px;
  justify-content: center;
  flex-wrap: wrap;
  margin-bottom: 28px;
}

.forbidden-actions .el-button {
  height: 42px;
  padding: 0 20px;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 500;
}

.forbidden-actions .el-button .el-icon {
  margin-right: 6px;
}

/* 管理员提示 */
.admin-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  font-size: 13px;
  color: #64748b;
  padding-top: 20px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

.hint-icon {
  color: #3b82f6;
  font-size: 16px;
}

.admin-hint a {
  color: #60a5fa;
  text-decoration: none;
  transition: color 0.2s;
}

.admin-hint a:hover {
  color: #93c5fd;
}

/* ===== 弹窗内样式 ===== */
.perm-section {
  padding: 8px 0;
}

.perm-count {
  font-size: 13px;
  color: #64748b;
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 10px;
}

.admin-badge {
  color: #f59e0b;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-weight: 500;
}

.perm-search {
  margin-bottom: 16px;
}

.role-list {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.role-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  font-size: 13px;
}

.perm-list {
  max-height: 320px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.perm-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  transition: background 0.15s;
}

.perm-item:hover {
  background: rgba(99, 102, 241, 0.08);
}

.perm-icon {
  color: #10b981;
  flex-shrink: 0;
}

.perm-code {
  color: #cbd5e1;
  font-family: 'SF Mono', 'Fira Code', Consolas, monospace;
  word-break: break-all;
}

.custom-dept-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

/* 响应式 */
@media (max-width: 640px) {
  .forbidden-card {
    padding: 40px 24px;
  }

  .forbidden-visual {
    gap: 16px;
  }

  .icon-wrapper {
    width: 60px;
    height: 60px;
    border-radius: 16px;
  }

  .lock-icon {
    font-size: 30px;
  }

  .code-403 {
    font-size: 52px;
  }

  .forbidden-title {
    font-size: 20px;
  }

  .forbidden-actions .el-button {
    flex: 1;
    min-width: 120px;
  }
}
</style>
