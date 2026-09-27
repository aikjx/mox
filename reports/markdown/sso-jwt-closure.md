# SSO 最终闭环：callback 换发平台 JWT + 前端回跳登录

## 1. 本轮目标
上一轮 callback 已真实做 OAuth2/OIDC 授权码交换，但只返回 SsoSession 占位（session_id/外部 access_token），**未对接 IAM 签平台 JWT**。本轮补齐：
- callback 成功后复用 `auth_session::issue_tokens` 为映射到的 IAM 用户签 `{access_token, refresh_token, user}`，与 `/api/auth/login` 同形状、不绕 RBAC；
- 前端 Login.vue 回跳后用同一套 authStore 持久化登录；
- pending map 加 10 分钟惰性 TTL 清扫。

## 2. 用户映射决策（按现有 IAM 数据结构定，不造新库）
- 查不到 `find_by_email`，用 `g.iam.list_users(DEFAULT_TENANT)` 遍历按 `email` 匹配；
- 无匹配 → **409**（不自动建号、不发任意 token）；
- 命中但 `user_status != active` → **403**；
- 身份源未返回 email → **401**；
- roles 复用 `roles_of(g, tenant_id, user)`，claims（sub/tenant_id/roles）与密码登录一致，不绕过 RBAC。

## 3. 后端改动
### sso/api.rs
- `callback_handler` 签名由 `State<Arc<SsoState>>` 改为 `State<GatewayState>`，经 `g.enterprise.sso` 访问 providers/pending，经 `g.iam.list_users` + `crate::system::auth_session::{roles_of,issue_tokens,user_json}` 签 JWT。
- pending 惰性清扫：每次回调先 `retain` 掉 `created_at` 超过 600s 的项，再 `remove` 消费。
- `exchange_oauth2_code` 改为无状态 `Result<SsoExchange{ext_sub,email}, Response>`（不再落 SsoSession map、不再返回 session_id），便于单测。
- 返回 `{access_token, refresh_token, token_type:"Bearer", user, external_sub}`。
- `build_sso_router()` 由泛型收敛为 `Router<GatewayState>`（callback 依赖 IAM）；`deployment.rs` / `enterprise_features.rs` 两处调用去掉 turbofish。
- 测试重写：旧测试直接 `State(Arc<SsoState>)` 调 callback 已不成立；改为测 `exchange_oauth2_code` 缺凭据 422 + pending 一次性消费。

### system/auth_session.rs
- `roles_of` / `user_json` / `issue_tokens` 改为 `pub(crate)`，供 sso 模块复用，不暴露给外部。

## 4. 前端改动（views/auth/Login.vue）
`handleSsoCallbackReturn` 收到 callback 返回后：
- 校验 `access_token` 存在；
- 写 `authStore.accessToken/refreshToken/userInfo` 同步状态；
- 写 `localStorage` 同密码登录键（`mox_access_token` / `mox_refresh_token` / `mox_user_info`）；
- 成功提示后跳 `redirect` 或 `/dashboard`；
- 409（未映射）/403（停用）等错误经 axios 拦截器原样 `ElMessage.error`。

## 5. 验证结果
- `cargo test -p mox-platform-gateway-svc sso`：2 passed（缺凭据 422、pending 生命周期）。
- `cargo build -p mox-platform-gateway-svc`：Finished（16.7s）。
- `npm run build`（Vite）：✓ 4352 modules transformed，built in 4m31s。
- curl E2E：**如实披露**——授权码交换需真实外部 IdP（client_id/secret/token_endpoint + 浏览器跳转身份源），本地无 IdP 无法端到端跑通成功路径；可验证的边界（422 缺凭据、400 state 失效、501 SAML/CAS/LDAP）已由 Rust 单测覆盖。

## 6. 遗留缺口
- SAML/CAS/LDAP 仍 501（如实）。
- email 映射是默认租户内全表扫描，用户量大时需补 `find_by_email` 索引查询（当前种子规模可接受）。
- 未在浏览器 GUI 真实登录（需真实 IAM + IdP），以代码审查 + 构建 + 单测为准。
