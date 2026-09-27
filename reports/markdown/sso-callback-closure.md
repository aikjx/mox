# SSO 回调闭环：OAuth2/OIDC 授权码交换真实实现

> 日期：2026-09-27
> 范围：`platform/gateway/mox-platform-gateway-svc/src/sso/` + 前端 Login.vue

---

## 1. 实现逻辑

### state 校验流程（防 CSRF）
1. `POST /sso/login` 生成/接收 `state`，写入内存 `pending: HashMap<state, PendingAuth{provider_id, redirect_uri, created_at}>`。
2. `POST /sso/callback {provider_id, code, state}`：
   - provider 必须存在且 enabled
   - `pending.remove(state)` 一次性消费；不存在 → **400 state 无效或已过期**；归属 provider 不一致 → **400 state 与提供商不匹配**
3. state 校验通过才进入协议分支。

### 协议分支
- **oauth2/oidc**（真实实现）：
  - 缺 `client_id/client_secret/token_endpoint` → **422**，不造桩成功
  - `reqwest::Client`（15s 超时）POST `token_endpoint`，Basic 认证 + form body `grant_type=authorization_code&code&redirect_uri`（RFC 6749 §4.1.3）
  - token 端点网络失败/非 2xx → **502** 透传错误；无 access_token → **502**
  - OIDC 再 GET `userinfo_endpoint`（Bearer）取 sub/email/name
  - 建立 `SsoSession{session_id, provider_id, user_id, external_user_id, access_token, status:active}`，返回 `{session_id, access_token, expires_in, user}`
- **saml/cas/ldap** → **501**，message 如实说明"仅 OAuth2/OIDC 已接通外部身份源"
- 路径与信封 `/api/enterprise/sso/callback`、`{code,message,data}` 不变。

### 前端回跳接入
- `chooseProvider` 把 `provider_id` 存 sessionStorage
- Login.vue `onMounted`：检测 URL `?code&state` → 调 `ssoCallback({provider_id, code, state})` → 成功提示并跳 redirect；失败透传后端 message
- 不改用户名/密码主链路。

## 2. 单测（3 个新用例，覆盖边界）

| 用例 | 断言 |
|---|---|
| `forged_state_rejected_and_no_session` | 未经过 /login 的伪造 state → 400，sessions 空 |
| `missing_credentials_returns_422_not_mock_success` | 走 /login 拿合法 state，但预置模板 client_id/secret 为空 → 422，sessions 空（不访问外网、不造桩） |
| `saml_protocol_still_not_implemented` | saml provider → 501 |

原 `external_callback_never_creates_a_mock_session` 已被更严格的新用例取代（旧用例断言固定 501，与新真实行为冲突）。

## 3. 验证结果

- `cargo test -p mox-platform-gateway-svc sso`：**3 passed**
- `cargo test -p mox-platform-gateway-svc` 全包：lib 117 + 集成 13+7+8 全 ok
- `cargo build -p mox-platform-gateway-svc`：见回报
- 前端：Login.vue 改动，vite build 见回报
- **curl E2E**：⚠️ 真实授权码交换需要外部可用 IdP（公网 OAuth2 服务），本地无 mock IdP 可起，未做端到端真交换；已用单测覆盖"缺字段 422 / 伪造 state 400 / saml 501"三条错误路径，成功路径需接真实 IdP 联调。

## 4. 遗留缺口

1. **平台 principal 供给**：callback 建了 SSO session，但还没把外部用户映射到本地用户/发平台 JWT——前端拿到 session_id 后尚未接 `authStore.loginWithToken`（因为 session_id 不是 /api/auth/current-user 认可的票据）。下一步需把 SSO session 对接到 IAM 的 token 签发。
2. **state 过期清理**：pending map 无 TTL 清扫，长期会堆积；需加定时清理。
3. **refresh_token 落库**：拿到 refresh_token 暂存 sessions map 但未实现刷新逻辑。
4. **真实 IdP E2E**：需配置飞书/钉钉真实 client_id+secret 后联调。
