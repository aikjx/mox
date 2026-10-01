// 当前模块归属的唯一判定：优先读路由 meta.module（模块化路由声明处即权威），
// 未迁移的旧路由回退到路径前缀表。此前 App.vue / TheSidebar.vue 等 5 处各自抄了一份前缀表并已开始漂移。
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { MODULE_SIDEBAR_CONFIG } from '@/constants'

const DEFAULT_MODULE = 'dashboard'

// 前缀 → 模块 key，顺序即优先级
const PATH_PREFIXES = [
  ['/dashboard', 'dashboard'],
  ['/projects', 'projects'],
  ['/tasks', 'tasks'],
  ['/expert-workspace', 'expert'],
  ['/expert-center', 'expert'],
  ['/expert-plaza', 'expert'],
  ['/expert-config', 'expert'],
  ['/alliance', 'expert'],
  ['/ai', 'ai'],
  ['/graph', 'graph'],
  ['/operators', 'operators'],
  ['/workflow', 'workflow'],
  ['/market', 'market'],
  ['/admin', 'admin']
]

export function moduleKeyOfPath(path) {
  const p = String(path || '')
  for (const [prefix, key] of PATH_PREFIXES) {
    if (p === prefix || p.startsWith(prefix + '/')) return key
  }
  return DEFAULT_MODULE
}

export function useActiveModule() {
  const route = useRoute()

  const moduleKey = computed(() => route.meta?.module || moduleKeyOfPath(route.path))
  const config = computed(() => MODULE_SIDEBAR_CONFIG[moduleKey.value] || MODULE_SIDEBAR_CONFIG[DEFAULT_MODULE])
  const hasSections = computed(() => (config.value.sections?.length ?? 0) > 0)

  return { route, moduleKey, config, hasSections }
}
