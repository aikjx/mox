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

export const SERVICE_PORTS = {
  gateway: 3000,
  nodeBackend: 3002,
  frontendUI: 5174,
  frontendAdmin: 5175
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
