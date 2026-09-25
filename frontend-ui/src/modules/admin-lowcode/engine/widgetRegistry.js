/**
 * 列渲染控件注册表：schema 里的 widget 名 → h() 渲染函数。
 * 禁止运行时字符串模板编译；新增列类型只在这里注册一次，所有页面共享。
 */
import { h } from 'vue'
import { ElTag } from 'element-plus'

const registry = {
  // 纯文本：空值渲染占位符
  text: ({ value }) => (value === null || value === undefined || value === '') ? '-' : String(value),

  // 标签：支持 row 级 tagTypeOf(row) 与 labelOf(row)，兼容静态 map/tagType 表
  tag: ({ value, row, col }) => h(ElTag, {
    type: typeof col.tagTypeOf === 'function' ? col.tagTypeOf(row) : (col.tagType?.[value] || 'info'),
    size: 'small',
    effect: col.tagEffect || 'plain',
  }, () => typeof col.labelOf === 'function' ? col.labelOf(row) : (col.map?.[value] ?? value)),
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
