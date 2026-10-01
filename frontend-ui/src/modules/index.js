// 前端模块清单：新模块只在此追加一行，其路由与导航由内核自动派生。
// 顺序即优先级，兜底路由（fallback）必须排在本清单之后。
import './expert-alliance/index.js'
import './governance/index.js'
import './admin-lowcode/index.js'
import './project/index.js'
import './ai/index.js'
import './graph/index.js'
import './workflow/index.js'
import './market/index.js'
import './operators/index.js'
import './system/index.js'

export { listModules, getModule, collectRoutes, collectNav } from './_kernel/module-registry.js'
export { NAV_ICONS, navIcon, navIconNames, registerNavIcons } from './_kernel/nav-icons.js'
