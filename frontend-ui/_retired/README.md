# 退役页面（不在构建与路由内）

此处文件已从 `src/` 移出：不被路由挂载、不被任何模块 import、`vite build` 与 `vitest` 均不覆盖。

| 文件 | 原路径 | 退役原因 | 待裁决 |
|------|--------|----------|--------|
| `misc-Login.vue` | `src/views/misc/Login.vue` | 671 行、零 importer 的第二套登录页；`/login` 实际挂 `src/views/auth/Login.vue`（411 行） | 它的"多认证模式切换 mode-tabs"是活登录页没有的能力。若确认要支持多认证方式，应在 `views/auth/Login.vue` 上重做，而不是复活本文件 |
| `ForgotPassword.vue` | `src/views/auth/ForgotPassword.vue` | 指向不存在的后端：网关未提供 `/api/auth/forgot-password`、`/api/auth/reset-password`，也不在 `public_paths` 白名单；前端 `Login.vue` 的"忘记密码？"入口已一并移除 | 若将来实现找回密码，需要：网关端点 + `public_paths` 登记 + 本页回归 + 路由 + 白名单，四处同步 |

恢复方式：`git log --diff-filter=D -- <原路径>` 找到删除它的提交后 `git checkout <sha>^ -- <原路径>`；未提交期间直接 `git checkout -- <原路径>` 即可。
