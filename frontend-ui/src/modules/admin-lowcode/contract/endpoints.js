// 低代码管理台端点契约：引用 system.api.js 的租户/参数配置/访问凭证端点族。
// 与后端网关 /system/*、/tenant/* 路由对齐；仅作注册表四要素中的 endpoints 台账。
export const ENDPOINTS = {
  tenant: {
    list: 'GET /tenant',
    create: 'POST /tenant',
    update: 'PUT /tenant/:id',
    remove: 'DELETE /tenant/:id',
    switch: 'GET /tenant/switch/:id',
  },
  config: {
    list: 'GET /system/config',
    create: 'POST /system/config',
    update: 'PUT /system/config/:id',
    remove: 'DELETE /system/config/:id',
  },
  security: {
    // 访问凭证（access.page.js）族，与 system.api.js 对齐
    list: 'GET /system/access',
    create: 'POST /system/access',
    update: 'PUT /system/access/:id',
    remove: 'DELETE /system/access/:id',
  },
}

export default ENDPOINTS
