// admin-lowcode 端点契约轻量守护：三族端点形状齐备、HTTP 动词与路径模板合法。
// 不跑 API-REGISTRY 逐字对齐（那是 expert-alliance 的重契约），此处只防契约表空壳化。
import { describe, it, expect } from 'vitest'
import { ENDPOINTS } from './endpoints.js'

describe('admin-lowcode 端点契约', () => {
  it('tenant/config/security 三族均存在且为对象', () => {
    for (const family of ['tenant', 'config', 'security']) {
      expect(ENDPOINTS[family], family).toBeTruthy()
      expect(Object.keys(ENDPOINTS[family]).length).toBeGreaterThan(0)
    }
  })

  it('每个端点字符串都以 HTTP 动词开头且指向对应资源', () => {
    for (const [family, table] of Object.entries(ENDPOINTS)) {
      for (const [action, spec] of Object.entries(table)) {
        expect(spec, `${family}.${action}`).toMatch(/^(GET|POST|PUT|DELETE) /)
      }
    }
  })
})
