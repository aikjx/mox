export const USER_ROLES = [
  { key: 'super_admin', label: '超级管理员', description: '拥有系统全部权限' },
  { key: 'admin', label: '管理员', description: '拥有大部分管理权限' },
  { key: 'operator', label: '运营人员', description: '可管理内容和知识库' },
  { key: 'developer', label: '开发者', description: '可使用API和开发功能' },
  { key: 'viewer', label: '查看者', description: '只能查看内容' },
  { key: 'guest', label: '访客', description: '基础访问权限' }
]

export const LLM_PROVIDER_TEMPLATES = [
  {
    id: 'deepseek',
    name: 'DeepSeek',
    endpoint: 'https://api.deepseek.com/v1',
    defaultModel: 'deepseek-chat',
    models: ['deepseek-chat', 'deepseek-reasoner'],
    features: ['chat', 'reasoning', 'function-calling']
  },
  {
    id: 'volcengine',
    name: '火山引擎',
    endpoint: 'https://ark.cn-beijing.volces.com/api/v3',
    defaultModel: 'doubao-pro-32k',
    models: ['doubao-pro-32k', 'doubao-pro-128k', 'doubao-lite-32k'],
    features: ['chat', 'embedding', 'function-calling']
  },
  {
    id: 'aliyun-qianwen',
    name: '阿里云千问',
    endpoint: 'https://dashscope.aliyuncs.com/api/v1',
    defaultModel: 'qwen-max',
    models: ['qwen-max', 'qwen-plus', 'qwen-turbo'],
    features: ['chat', 'reasoning', 'image', 'function-calling']
  },
  {
    id: 'zhipu',
    name: '智谱AI',
    endpoint: 'https://open.bigmodel.cn/api/paas/v4',
    defaultModel: 'glm-4',
    models: ['glm-4', 'glm-4-flash', 'glm-3-turbo'],
    features: ['chat', 'reasoning', 'function-calling']
  },
  {
    id: 'local-engine',
    name: '本地智能引擎',
    endpoint: 'http://localhost:3010/api/local',
    defaultModel: 'local-default',
    models: ['local-default'],
    features: ['chat', 'offline'],
    isLocal: true
  }
]

export const AUDIT_ACTION_TYPES = [
  { key: 'login', label: '登录' },
  { key: 'logout', label: '登出' },
  { key: 'create', label: '创建' },
  { key: 'update', label: '更新' },
  { key: 'delete', label: '删除' },
  { key: 'export', label: '导出' },
  { key: 'config_change', label: '配置变更' },
  { key: 'permission_change', label: '权限变更' },
  { key: 'api_call', label: 'API 调用' }
]

export const SYSTEM_CONFIG = {
  sessionTimeout: 3600,
  maxLoginAttempts: 5,
  passwordMinLength: 8,
  requireUppercase: true,
  requireNumber: true,
  requireSpecialChar: false,
  defaultLanguage: 'zh-CN',
  supportedLanguages: ['zh-CN', 'en-US'],
  timezone: 'Asia/Shanghai',
  dateFormat: 'YYYY-MM-DD',
  timeFormat: '24h'
}
