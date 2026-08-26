'use strict';

/**
 * MOX Enterprise · RBAC 权限控制中间件
 * ========================================
 * 基于角色的访问控制（Role-Based Access Control）
 *
 * 权限模型：
 *   - Subject: 用户/服务账号
 *   - Role: 角色（admin / operator / viewer / tenant-admin / tenant-user）
 *   - Permission: 权限（resource:action 格式）
 *   - Resource: 资源（file / chunk / metadata / system / audit）
 *   - Action: 操作（read / write / delete / admin / verify / gc）
 *
 * 用法：
 *   const { rbacMiddleware, requirePermission } = require('./security/rbac-middleware');
 *   app.use(rbacMiddleware);
 *   app.delete('/api/file/:id', requirePermission('file:delete'), handler);
 */

const crypto = require('crypto');

// ─── 角色定义 ───
const ROLES = {
  // 系统超级管理员：全部权限
  super_admin: {
    description: '系统超级管理员',
    permissions: ['*:*'],
    scope: 'global',
  },
  // 系统运维：除用户管理外的全部操作
  operator: {
    description: '系统运维工程师',
    permissions: [
      'file:read', 'file:write', 'file:delete',
      'chunk:read', 'chunk:write', 'chunk:delete',
      'metadata:read', 'metadata:write',
      'system:read', 'system:verify', 'system:gc',
      'audit:read',
    ],
    scope: 'global',
  },
  // 只读审计员
  auditor: {
    description: '审计员（只读）',
    permissions: ['file:read', 'metadata:read', 'audit:read', 'system:read'],
    scope: 'global',
  },
  // 租户管理员：本租户内全部权限
  tenant_admin: {
    description: '租户管理员',
    permissions: [
      'file:read', 'file:write', 'file:delete',
      'chunk:read', 'chunk:write',
      'metadata:read', 'metadata:write',
    ],
    scope: 'tenant',
  },
  // 租户普通用户：本租户内读写
  tenant_user: {
    description: '租户普通用户',
    permissions: ['file:read', 'file:write', 'chunk:read', 'metadata:read'],
    scope: 'tenant',
  },
  // 只读用户
  viewer: {
    description: '只读用户',
    permissions: ['file:read', 'metadata:read'],
    scope: 'tenant',
  },
  // 服务账号：API 调用专用
  service_account: {
    description: '服务账号',
    permissions: ['file:read', 'file:write', 'chunk:read', 'chunk:write', 'metadata:read'],
    scope: 'tenant',
  },
};

// ─── 内存角色绑定（生产环境应从数据库/配置中心加载） ───
const roleBindings = new Map(); // subjectId -> { roles: string[], tenantId?: string }

/**
 * 绑定角色到主体
 */
function assignRole(subjectId, role, tenantId = null) {
  if (!ROLES[role]) throw new Error(`未知角色: ${role}`);
  const existing = roleBindings.get(subjectId) || { roles: [], tenantId };
  if (!existing.roles.includes(role)) existing.roles.push(role);
  if (tenantId) existing.tenantId = tenantId;
  roleBindings.set(subjectId, existing);
}

/**
 * 移除角色
 */
function removeRole(subjectId, role) {
  const existing = roleBindings.get(subjectId);
  if (existing) {
    existing.roles = existing.roles.filter(r => r !== role);
    if (existing.roles.length === 0) roleBindings.delete(subjectId);
  }
}

/**
 * 获取主体的所有权限（合并所有角色）
 */
function getSubjectPermissions(subjectId) {
  const binding = roleBindings.get(subjectId);
  if (!binding) return { permissions: [], scope: null, tenantId: null };

  const permissions = new Set();
  let scope = null;
  for (const role of binding.roles) {
    const roleDef = ROLES[role];
    if (!roleDef) continue;
    roleDef.permissions.forEach(p => permissions.add(p));
    if (roleDef.scope === 'global') scope = 'global';
    else if (!scope) scope = 'tenant';
  }
  return {
    permissions: Array.from(permissions),
    scope,
    tenantId: binding.tenantId,
    roles: binding.roles,
  };
}

/**
 * 检查主体是否拥有指定权限
 * 支持通配符：*:* 表示全部权限，file:* 表示 file 资源全部操作
 */
function hasPermission(subjectId, resource, action) {
  const { permissions } = getSubjectPermissions(subjectId);
  const required = `${resource}:${action}`;

  for (const perm of permissions) {
    if (perm === '*:*' || perm === required) return true;
    // 通配符匹配：file:* 匹配 file:read, file:write 等
    if (perm.endsWith(':*')) {
      const permResource = perm.slice(0, -2);
      if (resource === permResource) return true;
    }
  }
  return false;
}

/**
 * 检查租户范围权限
 * 全局 scope 的主体可以访问任何租户
 * 租户 scope 的主体只能访问自己的租户
 */
function hasTenantAccess(subjectId, targetTenantId) {
  const { scope, tenantId } = getSubjectPermissions(subjectId);
  if (scope === 'global') return true;
  if (scope === 'tenant' && tenantId === targetTenantId) return true;
  return false;
}

// ─── Express 中间件 ───

/**
 * RBAC 中间件：从请求中提取 subjectId，挂载权限信息到 req.user
 */
function rbacMiddleware(req, res, next) {
  // 从认证中间件设置的 req.auth 中提取 subjectId
  // 认证中间件应在 RBAC 之前执行
  const subjectId = req.auth?.subjectId || req.user?.id || req.headers['x-subject-id'];
  const tenantId = req.auth?.tenantId || req.headers['x-tenant-id'];

  if (!subjectId) {
    return res.status(401).json({ error: 'unauthorized', message: '未认证的请求' });
  }

  const perms = getSubjectPermissions(subjectId);
  req.user = {
    ...(req.user || {}),
    subjectId,
    tenantId: tenantId || perms.tenantId,
    roles: perms.roles,
    scope: perms.scope,
    permissions: perms.permissions,
  };

  next();
}

/**
 * 权限守卫：要求指定权限才能访问
 * 用法：app.delete('/api/file/:id', requirePermission('file', 'delete'), handler)
 */
function requirePermission(resource, action) {
  return (req, res, next) => {
    const subjectId = req.user?.subjectId;
    if (!subjectId) {
      return res.status(401).json({ error: 'unauthorized' });
    }

    if (!hasPermission(subjectId, resource, action)) {
      return res.status(403).json({
        error: 'forbidden',
        message: `权限不足：需要 ${resource}:${action}`,
        required: `${resource}:${action}`,
        subjectId,
      });
    }

    // 租户范围检查（如果请求路径中包含 tenantId）
    const targetTenantId = req.params.tenantId || req.body.tenantId || req.query.tenantId;
    if (targetTenantId && !hasTenantAccess(subjectId, targetTenantId)) {
      return res.status(403).json({
        error: 'forbidden',
        message: '跨租户访问被拒绝',
      });
    }

    next();
  };
}

/**
 * 要求全局 scope（系统级操作）
 */
function requireGlobalScope(req, res, next) {
  if (req.user?.scope !== 'global') {
    return res.status(403).json({ error: 'forbidden', message: '需要全局 scope 权限' });
  }
  next();
}

// ─── API Key 认证（服务账号用） ───
const apiKeys = new Map(); // apiKeyHash -> { subjectId, tenantId, roles, expiresAt }

/**
 * 注册 API Key
 */
function registerApiKey(subjectId, apiKey, options = {}) {
  const hash = crypto.createHash('sha256').update(apiKey).digest('hex');
  apiKeys.set(hash, {
    subjectId,
    tenantId: options.tenantId || null,
    roles: options.roles || ['service_account'],
    expiresAt: options.expiresAt || null,
    createdAt: new Date(),
    lastUsedAt: null,
  });
  // 绑定角色
  for (const role of (options.roles || ['service_account'])) {
    assignRole(subjectId, role, options.tenantId);
  }
}

/**
 * API Key 认证中间件
 */
function apiKeyAuth(req, res, next) {
  const apiKey = req.headers['x-api-key'] || req.query.api_key;
  if (!apiKey) return next(); // 跳过，让其他认证方式处理

  const hash = crypto.createHash('sha256').update(apiKey).digest('hex');
  const record = apiKeys.get(hash);

  if (!record) {
    return res.status(401).json({ error: 'invalid_api_key' });
  }
  if (record.expiresAt && new Date(record.expiresAt) < new Date()) {
    return res.status(401).json({ error: 'api_key_expired' });
  }

  record.lastUsedAt = new Date();
  req.auth = {
    subjectId: record.subjectId,
    tenantId: record.tenantId,
    method: 'api_key',
  };
  next();
}

module.exports = {
  ROLES,
  rbacMiddleware,
  requirePermission,
  requireGlobalScope,
  apiKeyAuth,
  registerApiKey,
  assignRole,
  removeRole,
  hasPermission,
  hasTenantAccess,
  getSubjectPermissions,
};
