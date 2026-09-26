/**
 * PageSchema DSL 规范与校验（纯对象，无 Vue 运行时依赖，可在 node 单测）。
 *
 * 一个 PageSchema 描述一页标准 CRUD：
 * {
 *   key,                                  // 页面唯一标识
 *   api: { list, create, update, remove },// 直接绑定 src/api 既有函数
 *   search: { showKeyword, keywordPlaceholder, fields: [{prop,label,type,options,defaultValue}] },
 *   list: { rowKey, serverPagination, showPagination, columns: [{prop,label,width,widget,map,tagTypeOf,labelOf}], rowActions: [{label,type,action,show,confirm,handler}] },
 *   toolbar: [{label,type,icon,action,handler}],
 *   stats: [{label,icon,color,value}],
 *   form: { width, createTitle, editTitle, fields: [{prop,label,type,options,rules,disabledOnEdit,visibleOnEdit}], buildPayload }
 * }
 */

export const REQUIRED_API_KEYS = Object.freeze(['list', 'create', 'update', 'remove'])

export const VALID_FIELD_WIDGETS = Object.freeze([
  'input', 'textarea', 'number', 'select', 'radio', 'checkboxGroup', 'switch', 'date', 'treeSelect', 'slot'
])

/**
 * 校验 pageSchema，返回错误字符串数组（空数组 = 通过）。
 * 纯函数，便于 contract 层单测与 dev 期快速失败。
 */
export function validatePageSchema(schema) {
  const errors = []
  if (!schema || typeof schema !== 'object') {
    return ['pageSchema 必须是对象']
  }
  if (!schema.key) errors.push('缺少 key')

  if (!schema.api || typeof schema.api !== 'object') {
    errors.push('缺少 api 绑定')
  } else {
    for (const k of REQUIRED_API_KEYS) {
      if (typeof schema.api[k] !== 'function') errors.push(`api.${k} 必须是函数`)
    }
  }

  if (!schema.list || !Array.isArray(schema.list.columns) || schema.list.columns.length === 0) {
    errors.push('list.columns 必须是非空数组')
  } else {
    schema.list.columns.forEach((c, i) => {
      if (!c.prop) errors.push(`list.columns[${i}] 缺少 prop`)
    })
  }

  if (schema.search && !Array.isArray(schema.search.fields)) {
    errors.push('search.fields 必须是数组')
  }

  if (schema.form) {
    if (!Array.isArray(schema.form.fields)) {
      errors.push('form.fields 必须是数组')
    } else {
      schema.form.fields.forEach((f, i) => {
        if (!f.prop) errors.push(`form.fields[${i}] 缺少 prop`)
        if (f.type && !VALID_FIELD_WIDGETS.includes(f.type)) {
          errors.push(`form.fields[${i}].type="${f.type}" 非法，允许: ${VALID_FIELD_WIDGETS.join('/')}`)
        }
      })
    }
  }

  if (schema.list && Array.isArray(schema.list.rowActions)) {
    schema.list.rowActions.forEach((a, i) => {
      if (!a.label && typeof a.label !== 'function') errors.push(`list.rowActions[${i}] 缺少 label`)
    })
  }

  return errors
}

/** 校验失败直接抛错（dev 期快速失败），通过则原样返回。 */
export function assertPageSchema(schema) {
  const errors = validatePageSchema(schema)
  if (errors.length) {
    throw new Error(`PageSchema 校验失败:\n - ${errors.join('\n - ')}`)
  }
  return schema
}
