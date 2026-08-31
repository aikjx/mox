export const PLATFORM = {
  name: '璇玑算子统一系统',
  shortName: 'OUS',
  version: '3.0.0',
  mode: 'enterprise'
}

export const API_PATHS = {
  gateway: '/api',
  admin: '/api/admin',
  llm: '/api/llm',
  graph: '/api/graph',
  operators: '/api/operators',
  market: '/api/market',
  knowledge: '/api/knowledge',
  storage: '/api/storage',
  audit: '/api/audit'
}

// 端口权威来源：docs/ports/PORT-REGISTRY.md（PORT-REGISTRY-001）
// 注意：本文件当前无项目代码引用，仅作共享常量存档；若启用请以 PORT-REGISTRY-001 为准。
export const SERVICE_PORTS = {
  gateway: 8080,          // Rust 平台网关（api，唯一对外 HTTP 入口）
  nodeBackend: 0,         // Node API / sidecar 已退役（3010 停用，backend-node 已删除）
  frontendUI: 3020        // 前端（Vite Vue3 dev server）
}

export const STORAGE_TYPES = {
  LOCAL: 'local',
  S3: 's3',
  OSS: 'oss',
  MINIO: 'minio'
}

export const LLM_PROVIDERS = {
  DEEPSEEK: 'deepseek',
  VOLCENGINE: 'volcengine',
  ALIYUN_QIANWEN: 'aliyun-qianwen',
  ZHIPU: 'zhipu',
  OPENAI: 'openai',
  ANTHROPIC: 'anthropic',
  LOCAL: 'local-engine'
}

export const ROLE_KEYS = {
  SUPER_ADMIN: 'super_admin',
  ADMIN: 'admin',
  OPERATOR: 'operator',
  DEVELOPER: 'developer',
  VIEWER: 'viewer',
  GUEST: 'guest'
}

export const PERMISSION_KEYS = {
  USER_MANAGE: 'user:manage',
  ROLE_MANAGE: 'role:manage',
  LLM_CONFIG: 'llm:config',
  KNOWLEDGE_MANAGE: 'knowledge:manage',
  STORAGE_CONFIG: 'storage:config',
  SYSTEM_CONFIG: 'system:config',
  AUDIT_VIEW: 'audit:view',
  OPERATION_EXECUTE: 'operation:execute'
}
