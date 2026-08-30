// ⚠️ API 统一入口
// 所有 API 已按领域拆分到 *.api.js 文件
// 本文件重新导出所有 API，保持向后兼容
// 新代码建议按需从对应模块导入，如：import { getGraph } from '@/api/graph.api'

export { default as http, registerProjectIdGetter } from './http'

export * from './system.api'
export * from './graph.api'
export * from './ai.api'
export * from './operators.api'
export * from './workflow.api'
export * from './experts.api'
export * from './projects.api'
export * from './llm.api'
export * from './kb.api'
export * from './caomei.api'
export * from './mox.api'
export * from './melody.api'
export * from './alliance'
