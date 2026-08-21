<template>
  <el-container class="admin-layout">
    <el-aside width="240px" class="admin-aside">
      <div class="logo-area">
        <h2>璇玑 OUS</h2>
        <span>企业级管理控制台</span>
      </div>
      <el-menu :default-active="$route.path" router class="admin-menu">
        <el-menu-item index="/dashboard">
          <el-icon><Monitor /></el-icon>
          <span>管理仪表盘</span>
        </el-menu-item>
        <el-sub-menu index="/users">
          <template #title>
            <el-icon><User /></el-icon>
            <span>用户与权限</span>
          </template>
          <el-menu-item index="/users/list">用户管理</el-menu-item>
          <el-menu-item index="/users/roles">角色权限</el-menu-item>
          <el-menu-item index="/users/audit">审计日志</el-menu-item>
        </el-sub-menu>
        <el-sub-menu index="/llm">
          <template #title>
            <el-icon><Cpu /></el-icon>
            <span>大模型配置</span>
          </template>
          <el-menu-item index="/llm/providers">模型供应商</el-menu-item>
          <el-menu-item index="/llm/routing">智能路由</el-menu-item>
          <el-menu-item index="/llm/usage">用量统计</el-menu-item>
        </el-sub-menu>
        <el-sub-menu index="/knowledge">
          <template #title>
            <el-icon><Collection /></el-icon>
            <span>知识库管理</span>
          </template>
          <el-menu-item index="/knowledge/list">知识库列表</el-menu-item>
          <el-menu-item index="/knowledge/categories">分类管理</el-menu-item>
          <el-menu-item index="/knowledge/permissions">访问权限</el-menu-item>
        </el-sub-menu>
        <el-sub-menu index="/storage">
          <template #title>
            <el-icon><FolderOpened /></el-icon>
            <span>云盘与存储</span>
          </template>
          <el-menu-item index="/storage/paths">存储路径配置</el-menu-item>
          <el-menu-item index="/storage/permissions">访问权限</el-menu-item>
        </el-sub-menu>
        <el-sub-menu index="/system">
          <template #title>
            <el-icon><Setting /></el-icon>
            <span>系统设置</span>
          </template>
          <el-menu-item index="/system/general">通用设置</el-menu-item>
          <el-menu-item index="/system/security">安全策略</el-menu-item>
          <el-menu-item index="/system/about">系统信息</el-menu-item>
        </el-sub-menu>
      </el-menu>
    </el-aside>
    <el-container>
      <el-header class="admin-header">
        <div class="header-left">
          <el-breadcrumb separator="/">
            <el-breadcrumb-item :to="{ path: '/dashboard' }">首页</el-breadcrumb-item>
            <el-breadcrumb-item>{{ $route.meta?.title || '管理控制台' }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>
        <div class="header-right">
          <el-dropdown>
            <span class="user-info">
              <el-avatar :size="32" :icon="UserFilled" />
              <span class="username">{{ currentUser }}</span>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item>个人设置</el-dropdown-item>
                <el-dropdown-item divided>退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>
      <el-main class="admin-main">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup>
import { ref } from 'vue'
import { useUserStore } from '@/stores/user'

const userStore = useUserStore()
const currentUser = ref(userStore.username || 'admin')
</script>

<style scoped>
.admin-layout { height: 100vh; }
.admin-aside { background: #1a1a2e; display: flex; flex-direction: column; }
.logo-area { padding: 20px; text-align: center; border-bottom: 1px solid #2a2a4a; }
.logo-area h2 { color: #fff; font-size: 18px; margin: 0 0 4px; }
.logo-area span { color: #888; font-size: 12px; }
.admin-menu { flex: 1; border-right: none; }
.admin-menu :deep(.el-menu) { background: transparent; }
.admin-menu :deep(.el-menu-item),
.admin-menu :deep(.el-sub-menu__title) { color: #ccc; }
.admin-menu :deep(.el-menu-item:hover),
.admin-menu :deep(.el-sub-menu__title:hover) { background: rgba(255,255,255,0.08); }
.admin-menu :deep(.el-menu-item.is-active) { background: #409eff; color: #fff; }
.admin-header {
  background: #fff;
  border-bottom: 1px solid #e4e7ed;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 20px;
}
.user-info { display: flex; align-items: center; cursor: pointer; gap: 8px; }
.username { font-size: 14px; font-weight: 500; }
.admin-main { background: #f5f7fa; padding: 20px; }
</style>
