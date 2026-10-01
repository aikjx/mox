/**
 * 列渲染控件注册表：schema 里的 widget 名 → h() 渲染函数。
 * 禁止运行时字符串模板编译；新增列类型只在这里注册一次，所有页面共享。
 */
import { h } from 'vue'
import { ElTag } from 'element-plus/es/components/tag/index'

function fmtTime(t) {
  if (!t) return '-'
  const d = new Date(t)
  if (Number.isNaN(d.getTime())) return String(t)
  return d.toLocaleString()
}

const registry = {
  // 纯文本：空值渲染占位符
  text: ({ value }) => (value === null || value === undefined || value === '') ? '-' : String(value),

  // 行级标签：支持 tagTypeOf(row)/labelOf(row)，兼容静态 map/tagType
  tag: ({ value, row, col }) => h(ElTag, {
    type: typeof col.tagTypeOf === 'function' ? col.tagTypeOf(row) : (col.tagType?.[value] || 'info'),
    size: 'small',
    effect: col.tagEffect || 'plain',
  }, () => typeof col.labelOf === 'function' ? col.labelOf(row) : (col.map?.[value] ?? value)),

  // 静态字典回显：col.map {值:中文} + col.tagType {值:类型}，无 row 函数
  dict: ({ value, col }) => h(ElTag, {
    type: col.tagType?.[value] || 'info',
    size: 'small',
  }, () => col.map?.[value] ?? value),

  // 时间格式化：ISO/时间戳 → toLocaleString，空值占位（可用 col.emptyText 自定义）
  date: ({ value, col }) => (value === null || value === undefined || value === '')
    ? (col?.emptyText || '-')
    : fmtTime(value),

  // 数组 join：权限/permissions 等多值标签
  arrayTags: ({ row, col }) => {
    const arr = row?.[col.prop] || []
    return arr.length
      ? arr.map((p, i) => h(ElTag, { key: p + i, size: 'small', style: 'margin-right:6px' }, () => p))
      : '-'
  },
}

/** 注册自定义列控件（逃生舱：复杂列可注册自己的渲染函数）。 */
export function registerWidget(name, render) {
  registry[name] = render
}

export function getWidget(name) {
  return registry[name] || registry.text
}

/** 渲染某列在某行的 vnode。col 来自 schema.list.columns。 */
export function renderCell(col, row) {
  const fn = getWidget(col.widget)
  return fn({ value: row?.[col.prop], row, col })
}
