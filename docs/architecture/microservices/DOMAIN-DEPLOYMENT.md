# KG、Cloud、KB、IAM 的部署契约

默认选择融合部署。只有某个域需要独立发布、隔离资源或独立运维时再拆进程。
域边界是代码职责边界，不要求每个域都有单独的进程。IAM 没有必须最先启动的硬依赖：
现有有效 JWT 可由业务进程本地验签，IAM 不可用时登录与刷新受影响。

## 两种可执行形态

两种模式都使用 `mox-platform-gateway-svc` 的 `mox-server` 二进制，
通过 `MOX_HOST_ROLE=all|kg|cloud|kb|iam` 选择路由装配。错误角色值启动即失败。
独立角色只挂载本域业务路由，并复用融合模式的认证、限流、健康检查与日志中间件。
这是运行时装配，不是二进制裁剪：共享宿主仍包含其他域代码与基础状态初始化。

| 模式 | 入口 | 业务进程 | 数据 |
|---|---|---|---|
| fused | mox-server:3080 | 一个融合进程 | fused-data |
| split | Nginx:3080 | kg:3411、cloud:3412、iam:3413、kb:3414 | 每域独立命名卷 |

独立模式的 Nginx 保留原始 URI、查询、Authorization、Cookie；每个后端仍执行 JWT 验签，
未知路径返回 404，不自动降级到另一个拥有不同数据的进程。后端宿主端口仅发布到 loopback。
`/health` 在入口只表示代理存活，域健康需分别探测各域 `/health`；不代表全部依赖就绪。
此 split profile 覆盖四个指定域，其他平台功能仍由原企业部署承载，不宣称完整平台等价。

## 启动

在仓库根目录设置强随机 `JWT_SECRET`。四个宿主使用相同密钥、签发者和现有 JWT 格式；
密钥相同意味着这些宿主处于同一信任域。需要不同安全信任域时应改为非对称签名与公钥验证。

```sh
docker compose -f docker-compose.domains.yml --profile fused up -d --build
# 或选择独立模式；同一主机不能同时启用两个 profile，入口端口相同。
docker compose -f docker-compose.domains.yml --profile split up -d --build
```

独立部署也可直接运行同一二进制：设置 `MOX_HOST_ROLE=cloud` 后使用
`mox-server --bind 127.0.0.1 --port 3412`。各进程必须使用独立工作目录，
数据默认写入工作目录下 `data/`。跨机器部署应使用私有网络、TLS 与受控入口。

IAM 注册与登录沿用 `/api/auth/register`、`/api/auth/login`，用户存入 SQLite。
同一 IAM access token 可用于 `/kg/v1/*`、`/cloud/v1/*`、`/api/kb/*`。
IAM 宿主还承载 `/api/tenant/*`、`/api/system/*`、`/api/security/*`、`/rbac/v1/*`。
KG 现有适配器同时包含 `/graph/v1/*` 和 `/ai/engine/*`，拆分时保持兼容。

## 数据与能力边界

- SQLite 与本地对象文件部署保持单副本。不能仅增加 replica 数来实现写入扩容。
- 融合卷与独立卷互不相通；切换模式不会迁移旧数据。已有业务数据必须先备份、停止写入，
  按 IAM 数据库、Cloud 文件、KB 存储分别迁移并验收。不要把同一个 SQLite 卷挂给多个进程。
- KG 当前查询适配器加载共享种子图；这次部署切换没有增加分布式图数据库。
- KB 文档与摘要索引已做重启恢复验证；修复了将物理哈希文件名误当逻辑对象 key 的索引扫描。
  当前按存储元数据重建索引，读列表成本为 O(n)，大规模文档应改为事务索引。
  KB 域内图关系的持久性仍取决于现有组件，不能用挂卷代替验证。
- JWT 验签不等于完整的租户数据隔离；现有 Cloud 桶与 KB API 的跨租户授权仍需专项验收，
  不能据本部署验证声称已达到多租户生产安全标准。
- 外部 OAuth/OIDC/SAML 等身份源尚未完成真实协议对接。原 SSO 回调的模拟成功已删除，
  未实现的回调返回 501，不产生伪造会话。当前可用的是平台 IAM 登录与跨域 JWT 认证。
- 旧 `mox-kg-server` 等四个独立二进制实现与这些 API 不等价；不得用旧演示宿主直接替换新宿主。

## 可重现验证

```sh
cargo build -p mox-platform-gateway-svc --bin mox-server
python scripts/tests/verify_domain_hosts.py target/debug/mox-server.exe
python -m unittest scripts.tests.test_deployment_architecture -v
python scripts/gate/verify-ports.py
```

Linux 使用 `target/debug/mox-server`。进程验证使用临时目录和动态空闲测试端口，
检查注册、登录、重启后的用户持久化、跨域 token、匿名拒绝、dev token 拒绝及域路由隔离。
生产端口的唯一权威仍为 `docs/api/PORT-REGISTRY.md`。
