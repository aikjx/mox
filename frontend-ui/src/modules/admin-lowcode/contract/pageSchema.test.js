/**
 * PageSchema DSL 校验单测：纯 contract 层，不渲染 DOM。
 */
import { describe, it, expect } from 'vitest'
import { validatePageSchema, assertPageSchema, REQUIRED_API_KEYS } from './pageSchema.js'
import { tenantPage } from '../pages/tenant.page.js'
import { configPage } from '../pages/config.page.js'
import { accessPage } from '../pages/access.page.js'

describe('PageSchema DSL 校验', () => {
  it('tenantPage 应通过完整校验', () => {
    expect(validatePageSchema(tenantPage)).toEqual([])
  })

  it('必填 api 键齐全', () => {
    for (const k of REQUIRED_API_KEYS) {
      expect(typeof tenantPage.api[k]).toBe('function')
    }
  })

  it('每列都有 prop', () => {
    for (const c of tenantPage.list.columns) {
      expect(c.prop).toBeTruthy()
    }
  })

  it('缺少 api.list 应报错', () => {
    const bad = { ...tenantPage, api: {} }
    expect(validatePageSchema(bad).join('\n')).toContain('api.list')
  })

  it('列缺 prop 应报出列下标', () => {
    const bad = { ...tenantPage, list: { columns: [{ label: '无 prop' }] } }
    expect(validatePageSchema(bad).join('\n')).toContain('columns[0]')
  })

  it('非法表单控件类型应报错', () => {
    const bad = { ...tenantPage, form: { fields: [{ prop: 'x', type: 'not-a-widget' }] } }
    expect(validatePageSchema(bad).join('\n')).toContain('type=')
  })

  it('assertPageSchema 通过时原样返回', () => {
    expect(assertPageSchema(tenantPage)).toBe(tenantPage)
  })

  it('configPage（服务端分页 CRUD）应通过校验', () => {
    expect(validatePageSchema(configPage)).toEqual([])
    expect(configPage.list.serverPagination).toBe(true)
  })

  it('accessPage（凭证列表+吊销）应通过校验', () => {
    expect(validatePageSchema(accessPage)).toEqual([])
  })

  it('checkboxGroup 是合法表单控件', () => {
    const ok = { ...accessPage, form: { fields: [{ prop: 'p', type: 'checkboxGroup' }] } }
    expect(validatePageSchema(ok)).toEqual([])
  })
})
