# SSO 全维度前端接入报告

> 日期：2026-09-27
> 后端：`platform/gateway/mox-platform-gateway-svc/src/sso/{api,mod}.rs`（嵌套挂载 `/api/enterprise/sso`）

---

## 1. 后端契约清单（提取自源码）

信封 `{code, message|msg, data, total}`，由 `http.js` 统一解包（code===0 返回 data 本体，否则 reject 带 `[code]` 前缀）。

| 方法 | 路径 | 说明 | 响应 data |
|---|---|---|---|
| GET | `/protocols` | 五协议清单 | `[{code,name,description}]`：oauth2/oidc/saml/cas/ldap |
| GET | `/providers?status=&protocol=` | 提供商列表（sort_order 升序） | `[SsoProvider]` |
| GET | `/providers/:id` | 详情 | `SsoProvider`；404 `{code:404,message:"SSO提供商不存在"}` |
| POST | `/providers` | 创建（**status 强制 disabled**） | `{provider_id}` |
| PUT | `/providers/:id` | 部分更新 | `{code:0,message}` |
| DELETE | `/providers/:id` | 删除 | `{code:0,message}` |
| POST | `/login` | 发起登录 `{provider_id,redirect_uri?,state?}` | `{auth_url,state}` |
| POST | `/callback` | 授权码交换 | **固定 501 NOT_IMPLEMENTED** |
| POST | `/logout` | `{session_id}` | `{code:0,message}` |

**SsoProvider 字段**：provider_id, tenant_id, name, protocol, status(enabled/disabled), client_id, client_secret, auth_endpoint, token_endpoint, userinfo_endpoint(Option), logout_endpoint(Option), redirect_uri, scopes[], field_mapping{}, extra_config{}, is_default, sort_order, created_at, updated_at。

**预置 3 模板**：tpl_feishu / tpl_dingtalk / tpl_wecom，全部 disabled，端点已填好。

### 关键边界（如实披露）

`POST /callback` 在后端**固定返回 501**，message: "External SSO provider integration is not implemented; use /api/auth/login"，且有 Rust 单测 `external_callback_never_creates_a_mock_session` 守护——**外部身份源授权码/state 校验未实现，永远不会创建会话**。因此前端 SSO 登录流程只做到"**发起跳转**"（POST /login → window.location 跳转 auth_url），**不伪造 loginWithToken 平台登录态**。真正回派发 token 待后端 callback 落地后再接。

## 2. 登录页改动（views/auth/Login.vue）

**零改动用户名/密码主链路**（`handleLogin`/`loginWithToken` 原样保留）。

- 原占位 `handleSSO` 替换为真实流程：
  1. 点击「企业 SSO 登录」→ `GET /providers?status=enabled`
  2. 空态：ElMessage 提示"暂无可登录提供商"；单提供商直接发起；多提供商弹 `el-dialog` 列表选择（名称+协议 tag）
  3. 选择后 `POST /login {provider_id, redirect_uri}` → 拿 `auth_url` → `window.location.href = auth_url`
- 失败态：try/catch + ElMessage.error 透传后端 message；ssoLoading 按钮 loading。

## 3. AdminSso 管理面板（views/admin/panels/AdminSso.vue）

**手写，未 schema 化**——理由：表单按 protocol 分支（OAuth2/OIDC 才显示 4 个端点字段、ldap/saml 字段完全不同），且未来要加 field_mapping/extra_config 键值对编辑器；标准 CRUD 列表+对话框部分仍复用公共 `FormDialog`（含 `visible(formData)` 协议分支、`disabled(isEdit)` 锁协议）。

功能：
- 顶部协议卡片：5 协议 chips（GET /protocols）
- 提供商表格：名称 / 协议 tag / **状态 switch（PUT 启停）** / Client ID / 排序 / 更新时间 / 操作
- 新增/编辑对话框：名称、协议类型、client_id/secret、端点组（按协议显隐）、scopes、排序、默认开关
- 权限码：`v-permission="'sso:create'/'sso:update'/'sso:delete'"` 控制新增/启停/删除按钮
- 创建后提示"默认停用，配置后再启用"（对齐后端强制 disabled）

## 4. 新增/修改文件

| 文件 | 变更 |
|---|---|
| `src/api/sso.api.js` | 新增（9 个 API 函数） |
| `src/api/index.js` | +1 行 re-export |
| `src/views/auth/Login.vue` | SSO 流程替换占位 + 选择弹窗 + 样式 |
| `src/views/admin/panels/AdminSso.vue` | 新增面板 |
| `src/modules/system/index.js` | +`sso` 路由懒加载 |
| `src/views/admin/AdminView.vue` | TABS +「企业 SSO」入口 |

## 5. 验证结果

- **vitest**：见下方回报（基线 30 文件/680 用例）。
- **vite build**：见下方回报（新 SFC 编译入 chunk）。
- **curl E2E**：⚠️ **dev server 未启动**（127.0.0.1:3020 连接拒绝，浏览器亦 chrome-error），本轮无法实测 `/api/enterprise/sso/*`。端点存在性依据后端 `build_sso_router` 挂载确认，运行时 E2E 留待服务起后验证。
- **GUI**：登录需真实 IAM，受限；以代码审查 + 构建为准。

## 6. 遗留缺口

1. **callback 501**：SSO 真登录闭环（回派发平台 token）阻塞在后端，前端已预留 `ssoCallback()` 契约函数，后端落地后接 Login.vue 回跳处理即可。
2. **field_mapping / extra_config**：面板暂未暴露键值对编辑器，后续按需加 slot。
3. **curl E2E 未跑**：服务未起，下次联调补 protocols/providers CRUD/login 边界实测。
4. sso 路由未在菜单权限表登记权限码可见性（依赖登录后 permission store 注入 sso:* 权限）。
