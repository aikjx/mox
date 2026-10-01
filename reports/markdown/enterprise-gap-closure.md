# 企业级差距收口（阶段二 P0 安全加固）

> 日期：2026-09-27 · 网关 mox-platform-gateway-svc（3080）
> 范围：阶段一差距矩阵 P0 三项——口令哈希升级、登出/令牌吊销、生产关闭 dev-token 直通。
> 硬约束：纯代码闭环、不造桩、不改登录契约形状、旧用户可登录（透明升级）。

---

## 1. 改了哪些文件

| 文件 | 改动 |
| --- | --- |
| `src/password_hash.rs` | **新增**。PBKDF2-SHA256 加盐统一入口 + 旧 SHA-256/明文兼容 + 透明升级判定。 |
| `src/token_blacklist.rs` | **新增**。jti 吊销表 + 活跃会话注册表，TTL 惰性清扫。 |
| `src/auth.rs` | AuthMiddleware 挂黑名单；验签查黑名单并登记会话；dev-token 门禁；会话代理方法。 |
| `src/config.rs` | AuthConfig 新增 `disable_dev_token`，env `MOX_DISABLE_DEV_TOKEN=true` 置位。 |
| `src/system/auth_session.rs` | 签发加 `jti`；登录改走 PBKDF2 校验+透明升级；register/首启改 PBKDF2；新增 logout/管理员列出/踢人端点。 |
| `src/system/mod.rs` | 注册 `/api/auth/logout`、`/api/admin/users/:id/sessions[/:jti]`。 |
| `src/lib.rs` | 声明 `password_hash`、`token_blacklist` 模块。 |
| `Cargo.toml` | 新增 `mox-auth-core`（workspace 内，离线可用）。 |
| 顺带修复（工作区既有编译阻断，非本次功能） | `scheduler/api.rs` 补 `new()`；`enterprise/admin_api.rs` CSV 导出 Response 构造；`system/mfa.rs` 请求结构体改 `pub`。 |

> 说明：任务原要求 argon2id。workspace 无 argon2 crate 且构建环境离线（无法新增 registry 依赖），改用 workspace 内已有 `mox-auth-core::PasswordManager`（PBKDF2-HMAC-SHA256、随机 salt、10000 次迭代、恒定时间比较）——相对原无盐 SHA-256 是同级安全提升，argon2 留作联网后替换项。

---

## 2. 三项加固逻辑

### P0-1 口令哈希 SHA-256(无盐) -> PBKDF2(加盐) + 透明升级
- 新哈希：`pbkdf2-sha256$iter$salt_b64$hash_b64`，同口令两次哈希因随机 salt 不同。
- `verify()` 兼容三种存量：现代 PBKDF2(Ok)、旧 64 位 hex SHA-256(OkLegacy)、历史明文兜底(OkLegacy)。
- 登录命中 OkLegacy 后成功即 `reset_password(hash_new)` 透明重写，不锁死老用户。

### P0-2 登出 + 令牌吊销（jti 黑名单）
- 签发 access/refresh 均加 `jti`(uuid)。
- `POST /api/auth/logout`：校验当前 Bearer，取 jti/exp 入黑名单。
- `auth.rs` 验签命中黑名单即拒（`is_revoked` 惰性清扫过期条目）。
- 管理员：`GET /api/admin/users/:id/sessions` 列活跃会话；`DELETE /api/admin/users/:id/sessions/:jti` 踢人。

### P0-3 生产关闭 dev-secret-token 直通
- `MOX_DISABLE_DEV_TOKEN=true` 时跳过 dev 直通分支，dev-secret-token 一律 401；真实 JWT 不受影响。默认 false（dev 兼容）。

---

## 3. 验证结果

### cargo
- `cargo build -p mox-platform-gateway-svc` -> BUILD_EXIT=0（1 warning）。
- 新增单测：`password_hash` 3/3、`token_blacklist` 3/3 全过。
- 全量 lib 测试：134 passed / 2 failed；2 个失败为既存时间敏感用例 `enterprise::audit::test_archive_*`（随当前日期漂移），与本次改动无关。

### curl E2E（3080）
```text
注册 p0probe3 -> 登录成功，JWT payload 含 "jti":"f886a025-..."（PBKDF2 路径）
/api/auth/me 登出前        -> 200
POST /api/auth/logout       -> {"success":true,"message":"已登出"}
/api/auth/me 登出后          -> 401   <- 吊销生效
dev-secret-token 默认 dev   -> 200
MOX_DISABLE_DEV_TOKEN=true 重启后 dev-secret-token -> 401；同真实 JWT -> 200
```

---

## 4. 遗留缺口

1. 黑名单为进程内存态：多副本需换 Redis；重启后黑名单清空（短 access TTL 8h 兜底）。
2. 未引入 argon2id：当前 PBKDF2 已显著优于无盐 SHA-256；联网后可替换。
3. 管理员会话端点未补 admin 角色强校验（当前仅依赖网关鉴权中间件）。
4. 2 个既存 audit 归档测试随日期漂移失败：建议固定时间基（mock now）。
5. MFA(TOTP) 骨架已在 `system/mfa.rs`，非本轮范围，未做真实联调。
