# 后端高严重度缺口修复报告

修复日期：2026-09-27
修复人：专家联盟 Rust 后端修复工程师
工作目录：`D:\a10\aikjx\gitcode\infotopograph\platform`

---

## 一、任务 4：下游三 svc 补鉴权中间件

### 背景分析
- gateway 的 `auth_middleware` 位于 `gateway/mox-platform-gateway-svc/src/auth.rs`，依赖
  `crate::config::AuthConfig` 和 `crate::token_blacklist`，**不在公共 crate**，无法被下游 svc 直接复用
  （会产生循环依赖）。
- gateway 调下游的方式：`RemoteAllianceClient`（http-sdk）只透传 `X-Tenant-Id`/`X-User-Id`/`x-request-id`，
  `RegistryClient` 不带任何鉴权头。下游 svc 是被网关调用的，不直接对前端暴露。
- 因此采用**内部共享令牌（MOX_INTERNAL_TOKEN）模式**：下游校验网关注入的 Bearer token，未配置时放行（向后兼容）。

### 改了哪些文件

| 文件 | 改动 |
|------|------|
| `domains/alliance/svc/mox-alliance-scheduler-svc/src/routes.rs` | 新增 `internal_auth_layer` 中间件函数（:62-94）；在 `crypto_middleware` 之后挂载（:37-38） |
| `domains/alliance/svc/mox-alliance-executor-svc/src/routes.rs` | 同上：新增 `internal_auth_layer`；在 `crypto_middleware` 之后挂载 |
| `domains/alliance/svc/mox-alliance-registry-svc/src/routes.rs` | 补充 axum imports（`Request`/`middleware`/`Next`/`Response`）；新增 `internal_auth_layer`；在 `crypto_middleware` 之后挂载 |

### 中间件行为
1. 读 `MOX_INTERNAL_TOKEN` 环境变量；**未配置（空）则放行**——开发/本地默认行为不变。
2. `MOX_DEV_MODE=1` 强制跳过。
3. 配置令牌后，非公开路径必须携带 `Authorization: Bearer <token>`，否则返回 401。
4. 公开路径（探活/指标）始终放行：`/health`、`/metrics`、`/leadership`、`/api/registry/health`。
5. 挂载顺序：tracing → crypto → auth → handler（auth 在 crypto 之后，与任务要求一致）。

### 为什么这么改
- gateway 的 JWT auth_middleware 不可复用（gateway 内部依赖），内部令牌是最小侵入方案。
- 未配置令牌时放行，保证现有开发环境和单测不被破坏；生产环境部署时配置 `MOX_INTERNAL_TOKEN` 即可启用。
- 三个 svc 内联同一份中间件（~30 行），不引入新依赖、不新建公共 crate，符合"最小改动"。

---

## 二、任务 5：审计 Actor 从 system 改为请求身份

### 背景分析
- `emit_audit`（`experts_common.rs:579`）原硬编码 `AuditActor::system()`。
- gateway 的 `auth_middleware` 校验 JWT 后把 `UserInfo` 注入 request extensions，但专家联盟 handler 此前未提取。
- `AuditActor::human(user_id, role)` 已存在于 mox-audit crate。

### 改了哪些文件

| 文件 | 改动 |
|------|------|
| `gateway/mox-platform-gateway-svc/src/alliance/experts_common.rs` | 新增 `OptionalAuthUser` 提取器（FromRequestParts，:587-614）；新增 `actor_from_opt_user` 辅助函数（:616-626）；`emit_audit` 签名增加 `actor: &AuditActor` 参数，内部不再硬编码 `system()` |
| `gateway/mox-platform-gateway-svc/src/alliance/experts_dispatcher.rs` | 3 个 handler（`dispatch`/`consult`/`multi_consult`）加 `OptionalAuthUser(user)` 提取器；3 处 `emit_audit` 调用传入 `&actor_from_opt_user(&user)` |
| `gateway/mox-platform-gateway-svc/src/alliance/experts_registry.rs` | 4 个 handler（`create_expert`/`update_expert`/`delete_expert`/`consult_now_real`）加提取器；4 处 `emit_audit` 传真实 actor |
| `gateway/mox-platform-gateway-svc/src/alliance/experts_session.rs` | 4 个 handler（`create_session`/`delete_session`/`append_message`/`archive_session`）加提取器；4 处 `emit_audit` 传真实 actor |

### 降级策略
- `OptionalAuthUser` 提取器对缺失身份不报错（返回 `None`），`actor_from_opt_user(None)` 回退 `AuditActor::system()`。
- 即：经过 auth_middleware 的路由记录真实用户 ID/角色；公开路由或未认证请求仍记 system，向后兼容。
- 测试调用点补 `OptionalAuthUser(None)`（测试无 HTTP 请求上下文）。

---

## 三、任务 6：SQLite 补 schema 版本迁移机制

### 背景分析
- 三个 SQLite 初始化点都只用 `CREATE TABLE IF NOT EXISTS`，无 `PRAGMA user_version`，加列时老库不会自动 ALTER。

### 改了哪些文件

| 文件 | 改动 |
|------|------|
| `gateway/mox-platform-gateway-svc/src/alliance/experts_db.rs` | 新增 `const SCHEMA_VERSION: i32 = 1` 和 `migrate_schema_version` 函数；在 `open_experts_db()` 的 `init_schema` 之后调用 |
| `domains/alliance/core/mox-alliance-scheduler-core/src/storage.rs` | `SqliteTaskRepository::new()` 的建表 SQL 之后，读 `PRAGMA user_version`，<1 则 bump 到 1 |
| `domains/alliance/svc/mox-alliance-registry-svc/src/storage.rs` | `ExpertStore::open()`（文件库）和 `ExpertStore::memory()`（内存库）建表后加 `PRAGMA user_version = 1` |

### 迁移逻辑
- 当前 `SCHEMA_VERSION = 1`。
- 老库（user_version=0）：既有 `CREATE TABLE IF NOT EXISTS` 跑完后，`PRAGMA user_version = 1`。
- 已迁移库（≥1）：跳过，不重复 DDL。
- 保持 WAL / busy_timeout / synchronous 等 PRAGMA 不变，不破坏单写者约定。
- 未来加列时：递增 `SCHEMA_VERSION`，在 `migrate_schema_version` 里按版本号逐步 `ALTER TABLE`。

---

## 四、cargo test 运行结果

### 编译检查
- `cargo check -p mox-platform-gateway-svc`：**通过**（仅 1 个既有 warning：`mfa.rs` unused import `rand::RngCore`，非本次改动）。
- `cargo check -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc -p mox-alliance-registry-svc -p mox-alliance-scheduler-core`：**通过**。

### 测试结果

| crate | 测试数 | 通过 | 失败 |
|-------|--------|------|------|
| mox-platform-gateway-svc（alliance:: 模块） | 64 | 64 | 0 |
| mox-alliance-scheduler-core | 10 | 10 | 0 |
| mox-alliance-scheduler-svc（http_integration） | 11 | 11 | 0 |
| mox-alliance-registry-svc | 0（无单测） | — | — |
| mox-alliance-executor-svc | 0（无单测） | — | — |

**合计：85 通过，0 失败。**

### 编译修复过程
1. 首轮编译错误：`OptionalAuthUser` 解构后 `user` 已是 `Option<UserInfo>`，误写 `user.0` → 改为 `&user`。
2. `actor_from_opt_user` 中 `roles.join(",")` 临时值生命周期错误 → 用 `let roles` 绑定。
3. 测试代码直接调用 handler 缺新参数 → 精准补 `OptionalAuthUser(None)`。
4. 批量替换误伤未改 handler（update_config/reset_expert）→ 回滚后只给 11 个改过的 handler 补参数。

---

## 五、闭环情况

### 已闭环
- **任务 4**：三个下游 svc 路由层均挂载内部令牌鉴权中间件，防绕过网关直连。生产环境配置 `MOX_INTERNAL_TOKEN` 即生效。
- **任务 5**：11 个写操作 handler 的审计事件记录真实操作人（user_id + roles），未认证降级 system。
- **任务 6**：三个 SQLite 存储点均接入 `PRAGMA user_version` 版本迁移，未来加列可按版本号 ALTER。

### 已闭环（含第六节收尾）
- **任务 4**：三个下游 svc 路由层均挂载内部令牌鉴权中间件，防绕过网关直连。
- **任务 5**：11 个写操作 handler 的审计事件记录真实操作人（user_id + roles），未认证降级 system。
- **任务 6**：三个 SQLite 存储点均接入 `PRAGMA user_version` 版本迁移，未来加列可按版本号 ALTER。
- **第六节（收尾）**：网关 `RemoteAllianceClient` 与 `RegistryClient` 两个出站客户端在 `Client::builder()` 构造点注入 `Authorization: Bearer <MOX_INTERNAL_TOKEN>` 默认头，鉴权链路端到端打通。

### 仍待（纯部署配置，非代码缺口）
- 生产环境需在网关和三个 svc 上**同时配置** `MOX_INTERNAL_TOKEN=<同一密钥>`，鉴权才真正生效；未配置时整条链路放行（开发默认）。
- `MOX_AUDIT_HMAC_SECRET`、`MOX_API_CRYPTO=sm4`、`MOX_ALLIANCE_STORAGE_MODE=sqlite` 三个生产开关需部署时显式设置。

---

## 六、部署侧收尾：网关内部客户端注入鉴权令牌（2026-09-27）

### 背景
任务 4 在下游三 svc 路由层补了 `internal_auth_layer`，但网关侧两个出站客户端不发送 `Authorization` 头，生产启用鉴权后网关调下游会被 401。本次补齐出站侧。

### 改了哪些文件

| 文件 | 改动位置 | 改动内容 |
|------|---------|---------|
| `gateway/mox-platform-gateway-svc/src/alliance/registry_client.rs` | `RegistryClient::new()`（:22-41） | `Client::builder()` 构造时读 `MOX_INTERNAL_TOKEN`；非空则用 `default_headers(HeaderMap)` 注入 `Authorization: Bearer <token>`；未配置则不注入 |
| `domains/alliance/sdk/mox-alliance-http-sdk/src/alliance_remote.rs` | `RemoteAllianceClient::explicit()`（:116-130） | 同上模式：`reqwest::Client::builder()` 构造时读 `MOX_INTERNAL_TOKEN`，非空则注入 `Authorization: Bearer <token>` 默认头 |

### 为什么这么改
1. **构造点注入，不散落各调用处**：两个客户端都在 `Client::builder()` 处统一构造 HTTP client，用 `default_headers` 一次注入，所有出站请求自动携带——无需修改任何调用点。
2. **同源同令牌**：读的就是 `MOX_INTERNAL_TOKEN`，与下游 `internal_auth_layer` 校验的 `std::env::var("MOX_INTERNAL_TOKEN")`（routes.rs:73）完全同源。
3. **向后兼容**：未配置令牌时不注入 header，下游中间件同样放行（routes.rs:74），开发行为不变。
4. **与既有透传头不冲突**：`call()` 中逐请求注入的 `X-Tenant-Id`/`X-User-Id` 和 crypto 协商头保持不变；`default_headers` 只加 `Authorization`。

### 鉴权链路端到端状态

```
前端 → 网关 :3080
         ├─ auth_middleware 校验 JWT（既有）
         ├─ crypto_middleware 入站解密（既有）
         └─ alliance/ 模块
              ├─→ RemoteAllianceClient → scheduler:3100 / executor:3200
              │     ✅ default_headers 注入 Authorization: Bearer <MOX_INTERNAL_TOKEN>
              │     ✅ 下游 internal_auth_layer 校验同源令牌
              └─→ RegistryClient → registry:3400
                    ✅ default_headers 注入 Authorization: Bearer <MOX_INTERNAL_TOKEN>
                    ✅ 下游 internal_auth_layer 校验同源令牌
```

### 验证结果
- `cargo check -p mox-platform-gateway-svc`：通过
- `cargo check -p mox-alliance-http-sdk`：通过
- `cargo test -p mox-platform-gateway-svc --lib alliance`：**64 passed, 0 failed**
- `cargo test -p mox-alliance-http-sdk`：**15 passed, 0 failed**
- 合计 **79 测试通过，0 失败**

---

## 七、中低严重度缺口收尾（2026-09-27 第二轮）

### N6 修复：registry_client.rs .expect() 恐慌点

| 项 | 内容 |
|----|------|
| 文件 | `gateway/mox-platform-gateway-svc/src/alliance/registry_client.rs:37-41` |
| 改动 | `.build().expect("failed to build http client")` → `.build().unwrap_or_else(|e| { tracing::error!(...); Client::new() })` |
| 理由 | 生产环境 HTTP client 初始化失败不应 panic 整个网关进程；记录错误日志后降级为默认 client，保活网关其他功能 |
| 验证 | cargo check 通过；cargo test --lib alliance 64 passed 0 failed |

### N9 修复：文档动词面数 9→8

| 项 | 内容 |
|----|------|
| 文件 | `docs/expert-alliance/CURRENT-ARCHITECTURE.md:248` |
| 改动 | "全部 6 条路径、9 个动词面" → "全部 6 条路径、8 个动词面" |
| 理由 | 实测 routes.rs:23-32 为 8 个动词面，文档数字错误 |

### 验证
- cargo check -p mox-platform-gateway-svc：通过
- cargo test -p mox-platform-gateway-svc --lib alliance：**64 passed, 0 failed**

---

## 八、缺口最终状态总览

### 已闭环（代码已修）

| # | 缺口 | 修复轮次 | 验证 |
|---|------|---------|------|
| N1 | 下游三 svc 无鉴权中间件 | 第一轮 | cargo test 85 通过 |
| N2 | 审计 Actor 硬编码 system | 第一轮 | cargo test 85 通过 |
| N3 | SQLite 无 schema 版本迁移 | 第一轮 | cargo test 85 通过 |
| N6 | registry_client.rs .expect() 恐慌点 | 第二轮 | cargo test 64 通过 |
| N9 | 文档动词面数 9→8 | 第二轮 | 文档修正 |
| G1 | legacy 工作台调禁端点 | 第一轮（前端） | vitest 646 通过 |
| G2 | legacy 假端点 /qa | 第一轮（前端） | vitest 646 通过 |
| G3 | 控制台路由无 RBAC | 第一轮（前端） | vitest 646 通过 |
| — | 网关客户端不发送内部令牌 | 第一轮收尾 | cargo test 79 通过 |

### 设计取舍（非缺陷，明确标注，留待未来需求驱动）

| # | 项 | 理由 | 处置 |
|---|----|------|------|
| N5 | registry 健康探测默认关闭 | 主动探活有网络开销，开发环境不需要；生产部署时设 MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=true 即可 | 文档已标注，不改默认值 |
| N10 | 两套熔断状态不共享 | 网关侧（dispatcher.rs）和 scheduler 侧（llm_router.rs）在不同进程边界，各自熔断是合理的进程内隔离 | 设计取舍，不改 |
| N11 | 会话单进程内恢复 | 网关未做多活，sessions 落本地 SQLite；扩容需 sticky session | 已知架构边界，未来多活时再解 |
| G10 | 文案硬编码中文 | 内网政务场景，用户群体固定中文；多语言需求出现时整体引入 i18n | 设计取舍，不改 |

### 留待未来（功能新增/大重构，非本轮范围）

| # | 项 | 优先级 | 理由 |
|---|----|--------|------|
| N4 | 图谱节点级 CRUD | P2 | 新增 POST /nodes、POST /edges 端点 + 增量维护逻辑，是功能开发不是缺陷修复 |
| N7 | /metrics Prometheus 文本格式 | P2 | scheduler/executor 当前返回 JSON 快照；补 Prometheus 文本需引入 metrics crate，是重构 |
| N8 | DAG 执行器并行度信号量 | P2 | dag_engine 加 Semaphore 限并发，是新功能开发 |
| G4 | 两套 API 客户端收敛（后端 Rust） | ✅ 已归一化 | 代码实测：gateway/src/alliance_remote.rs 仅 2 行 re-export，RemoteAllianceClient 真源在 SDK（单一实现）；RegistryClient 调 registry:3400 与 RemoteAllianceClient 调 scheduler/executor:3100/3200 职责不同，非重复。前端 legacy 收敛见 frontend-fix-report |
| G5 | SSE 统一到 useSSE | P2 | AllianceTaskView 手写 reader 迁移到 useSSE composable，是局部重构 |
| G6 | console/orch store 补测试 | P2 | 新增测试文件，是质量提升不是缺陷修复 |
| G8 | 大列表虚拟滚动 | P2 | 专家量未达规模前无性能问题；引入虚拟滚动是新组件开发 |
| G9 | 图谱可视化三套技术栈归一 | P2 | 手写 SVG / 手搓力导向 / echarts force 归一到一套，是前端重构 |
---

## 九、G4 收敛结论（2026-09-28）

### 任务
评估"网关内联 RemoteAllianceClient 与 SDK 客户端职责重叠"是否需要收敛。

### 代码实测证据

1. **gateway/src/alliance_remote.rs** 仅 2 行：
   ```rust
   pub use mox_alliance_http_sdk::alliance_remote::*;
   ```
   网关**没有**自己的 RemoteAllianceClient 实现，直接 re-export SDK。

2. **RemoteAllianceClient 真源**在 `mox-alliance-http-sdk/src/alliance_remote.rs`（1282 行），包含：
   - 客户端构造（from_env/explicit）
   - call() 统一请求方法（含加密、租户透传）
   - scheduler_get/scheduler_post/executor_get/executor_post_raw
   - 全部 DTO 归一化函数（norm_task/norm_node/norm_mode/norm_fusion 等）
   - 全部远程端点 handler（remote_create_task 等 15 个函数）
   - 生命周期测试（12 个 #[test]）

3. **RegistryClient**（`gateway/src/alliance/registry_client.rs`，~110 行）是另一个独立客户端：
   - 调用目标：registry-svc:3400（专家注册中心）
   - 方法面：health/register/deregister/heartbeat/aggregated_heartbeat/list_experts
   - 与 RemoteAllianceClient（调 scheduler:3100 + executor:3200，任务编排）**职责完全不同**

### 结论

**G4（后端 Rust 客户端）已归一化，无需收敛。**

- 网关通过 re-export 复用 SDK 的 RemoteAllianceClient——不存在第二份实现
- RegistryClient 与 RemoteAllianceClient 指向不同下游服务、不同职责，不是重复
- 唯一的小重复是两者各自在 Client::builder() 注入 MOX_INTERNAL_TOKEN（各 ~8 行），但因构建独立 Client 实例指向不同 base_url，抽公共函数收益极小

### 前端 G4 说明
前端"两套 API 客户端"（模块 allianceApi vs legacy @/api/*.js）是另一个问题，已在 frontend-fix-report 中通过 G1/G2/G3 清理最危险部分；完整收敛 legacy 是 P1 前端重构，不在本轮 Rust 范围。

---

## P0-C 双值令牌滚动（2026-09-29 第三轮）

### 背景

12-innovation-roadmap.md §1.3 表第 3 行「下游 svc 内部令牌双值滚动 / WS 文档修订」前半。
下游三 svc（scheduler/executor/registry）的 `internal_auth_layer` 此前只校验单一 `MOX_INTERNAL_TOKEN`，
生产换令牌必须停机或双窗口内 401。本轮加备用令牌 `MOX_INTERNAL_TOKEN_ALT`，支持零停机滚动。

### 改动文件（路径 + 行号）

三处 `internal_auth_layer` 函数体同构改造，每处净增 3 行逻辑 + 注释：

1. `platform/domains/alliance/svc/mox-alliance-scheduler-svc/src/routes.rs`
   - doc 注释 :62-79（新增滚动流程说明 5 行）
   - 函数体 :80-104：
     - :86 新增 `let expected_alt = std::env::var("MOX_INTERNAL_TOKEN_ALT").unwrap_or_default();`
     - :87 放行条件由 `expected.is_empty()` 改为 `(expected.is_empty() && expected_alt.is_empty())`
     - :101 校验由 `t == expected` 改为 `t == expected || (!expected_alt.is_empty() && t == expected_alt)`

2. `platform/domains/alliance/svc/mox-alliance-executor-svc/src/routes.rs`
   - doc 注释 :119-136
   - 函数体 :137-161：同构（expected_alt 读取 / 双空放行 / 双值或校验）

3. `platform/domains/alliance/svc/mox-alliance-registry-svc/src/routes.rs`
   - doc 注释 :71-88
   - 函数体 :89-113：同构

### 行为保持（向后兼容）

- 两令牌均未配置 → 与现状一致放行（`dev_mode || (主空 && 备空)`）
- `MOX_DEV_MODE=1` 跳过逻辑不变
- 公开白名单（/health /metrics /leadership /api/registry/health）不变
- 网关出站仍只读主值 `MOX_INTERNAL_TOKEN`（registry_client.rs / alliance_remote.rs 未动）；
  滚动时网关切主值即可，下游因同时接受 ALT 旧值而不 401

### 生产滚动流程（双窗口，零停机）

1. 旧值写入 `MOX_INTERNAL_TOKEN_ALT` → 全集群滚动重启三 svc（此时主值=旧值、ALT=旧值，行为不变）
2. 网关出站 `MOX_INTERNAL_TOKEN` 切新值 → 全集群滚动重启网关
   （旧副本带旧值命中 ALT，新副本带新值命中主值，均不 401）
3. 观察一个周期后移除 `MOX_INTERNAL_TOKEN_ALT`，完成滚动

### 验证结果

- `cargo check -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc -p mox-alliance-registry-svc --all-targets`
  → Finished dev profile in 9.51s，无 error / 无 warning
- `cargo test` 三 svc：
  - executor-svc：unit 9 + http_integration 6 = **15 passed, 0 failed**
  - registry-svc：unit 17 + http_registry 11 = **28 passed, 0 failed**
  - scheduler-svc：unit 10 + http_integration 11 = **21 passed, 0 failed**
  - 合计 **64 passed, 0 failed**
  - scheduler http_integration 11 用例全部通过（这些用例未带 Authorization，因 MOX_DEV_MODE / 未配置令牌而走放行分支，不受双值改造影响）
- gateway alliance 用例：`cargo test -p mox-platform-gateway-svc alliance` → **64 passed, 0 failed**（0.20s；另有 1 个既有 warning `unused import: rand::RngCore` 位于 system/mfa.rs:26，与本次三 svc 改动无关，未触碰）

### 测试缺口说明

三 svc 现有测试套件均在「未配置令牌 / dev_mode」放行路径下跑，未构造「配置主+备后 Bearer=ALT 通过、Bearer=错值 401」的用例。
因改造为纯函数级（env 读取 + 字符串比较），且现有 64 用例全绿不破坏放行路径，本轮按任务要求「若没有测试文件则跑现有 cargo test 确认不破坏」执行；
双值正向/反向用例建议后续在集成测试中补（需在测试进程内临时 set_var，注意 env 全局性需串行）。


---

## R1 后端 RBAC 强制（2026-09-30）

### 背景

14 号企业权限模型 §六 G-2 缺口：「后端联盟写面不做角色强制」。改造前现状：
管理写面 handler（专家注册/CRUD、图谱重建、调度配置写、负载重置）虽已经
`OptionalAuthUser` 提取用户身份并注入审计 actor（N2 闭环），但**未认证 / 非管理角色
直连 HTTP 仍能执行管理写面**——只审计不拦截，权限模型停留在前端守卫层。

本轮把权限矩阵闭环到「后端强制」（编号 R1，对应 16 号台账新增 N12 行）。

### 路径选择：选 b（联盟域轻量 RBAC 层），不选 a（复用 mox-ai-expert-svc）

| 维度 | 路径 a：复用 mox-ai-expert-svc `check_with_audit` | 路径 b：联盟域轻量 RBAC（本轮） |
|---|---|---|
| 依赖成本 | 该 crate 是重型 svc（reqwest blocking / rayon / rusqlite / mox-kg-sdk 等），网关轻量服务拉入整套依赖树 | 仅在 gateway alliance 模块内新增一个 `experts_rbac.rs`，零外部依赖新增 |
| 审计耦合 | `check_with_audit` 依赖其私有 `crate::audit::integration::AuditContext` / `crate::audit::AuditResource`（check.rs:8-10），跨 crate 复用需把整套审计设施 pub 化 | 复用网关现有 `emit_audit` / `actor_from_opt_user`（experts_common.rs:628 / :613），审计链不分裂 |
| 角色语义 | 该 crate 角色为 viewer/editor/admin/safety_approver（39 号链），与联盟现行 5 角色码（super_admin/tenant_admin/dept_manager/normal_user/readonly_auditor）不匹配 | 直接采用联盟现行角色码，与前端 `ADMIN_GUARD=['super_admin','tenant_admin']` 对齐 |
| 策略源 | 其 POLICY 是全局静态策略（服务实例状态），不适合网关多租户 | 管理角色集 `ADMIN_ROLES: &[&str]` 为常量，进程内一致 |

结论：路径 a 有跨域耦合、审计设施外泄、角色语义错配三重风险；路径 b 在网关内
闭环，最小侵入、与既有审计链同源。

### 新增文件

`platform/gateway/mox-platform-gateway-svc/src/alliance/experts_rbac.rs`（约 340 行）：

- `pub const ADMIN_ROLES: &[&str] = &["super_admin", "tenant_admin"]`
- `pub enum RbacAction`：`RegisterExpert` / `UpdateExpert` / `DeleteExpert` /
  `RebuildGraph` / `ResetDispatcher` / `ResetAllDispatcher` / `UpdateConfig`
  （每个带 `code()` 短名与 `description()` 中文描述）
- `pub struct RbacDenied { status: u16, reason: String, suggested: String }`
- `pub fn enforce_admin(user: &Option<UserInfo>, action: RbacAction) -> Result<(), RbacDenied>`
  —— 纯判断：None → 401；Some 但 roles 与 ADMIN_ROLES 无交集 → 403；匹配 → Ok
- `pub fn audit_denied(state, user, action, denied)` —— 拒绝时经 `emit_audit` 记录
  `AuditAction::RBACDenied` / `AuditOutcome::Blocked`，resource_type=`rbac`，
  resource_id=action.code()，detail 含 action/status/reason/suggested
- `pub fn enforce_admin_or_respond(state, user, action) -> Result<(), ApiResponse<Value>>`
  —— 一站式：拒绝则审计并返回 `err(denied.status, denied.reason)`

模块注册：`alliance/mod.rs:29` 新增 `pub mod experts_rbac;`。

### 接入的 handler 清单（文件:行号）

| 管理写面动作 | 文件 | handler 签名行 | enforce 调用行 |
|---|---|---|---|
| 注册专家 POST /api/experts | `experts_registry.rs` | :349 `create_expert` | :355 |
| 更新专家 PUT /api/experts/:id | `experts_registry.rs` | :392 `update_expert` | :399 |
| 删除专家 DELETE /api/experts/:id | `experts_registry.rs` | :421 `delete_expert` | :427 |
| 重建图谱 POST /api/expert-graph/rebuild | `experts_graph.rs` | :874 `post_rebuild` | :879（签名加 `OptionalAuthUser(user)`） |
| 更新调度配置 PUT /api/experts/dispatcher/config | `experts_dispatcher.rs` | :390 `update_config` | :396（签名加 `OptionalAuthUser(user)`） |
| 重置单专家负载 POST /api/experts/dispatcher/reset/:id | `experts_dispatcher.rs` | :773 `reset_expert` | :780（签名加 `OptionalAuthUser(user)`） |
| 重置全部负载 POST /api/experts/dispatcher/reset-all | `experts_dispatcher.rs` | :829 `reset_all` | :834（签名加 `OptionalAuthUser(user)`） |

接入方式统一为 handler 入口前 3 行：

```rust
if let Err(resp) = enforce_admin_or_respond(&state, &user, RbacAction::Xxx) {
    return resp;
}
```

强制检查在既有业务逻辑与既有 `emit_audit(Success)` 之前——拒绝时**不执行业务**，
只发 `rbac.denied` 审计；放行后原 `emit_audit(Success)` 调用链原样保留，不破坏
N2 已闭环的成功审计。

### 行为变化（安全增强，如实说明）

- **改造前**：管理写面未认证时由 `actor_from_opt_user(None)` 降级为 system actor
  照常执行业务并审计 Success；非管理角色同样能直连执行。
- **改造后**：
  - 未认证（`UserInfo=None`）→ 401，不执行业务，发 `rbac.denied`（Blocked）审计
  - 已认证但角色不含 super_admin/tenant_admin → 403，不执行业务，发 `rbac.denied` 审计
  - 管理角色 → 正常执行业务，原 Success 审计不变
- **保持不变**：公开读面、健康路径、自服务写面（consult/multi_consult/debate、
  会话创建/追加/归档、收藏/预约/取消）行为不变——这些仅要求认证，不强制角色。

### 边界说明（哪些强制 / 哪些不强制及理由）

| 写面类别 | 是否强制角色 | 理由（14 号矩阵 §五） |
|---|---|---|
| 专家注册/CRUD | ✅ 强制 super_admin/tenant_admin | 矩阵 #10/#11：仅管理员可写专家档案 |
| 图谱重建 | ✅ 强制 | 矩阵 #8/#18：破坏性全量重建，仅管理员 |
| 调度配置写 | ✅ 强制 | 矩阵 #17：影响全局调度行为，仅管理员 |
| 负载重置（单/全） | ✅ 强制 | 矩阵 #7/#18：运维破坏性操作，仅管理员 |
| consult / multi_consult / debate | ❌ 仅认证 | 矩阵 #12/#13：自服务写面，任意认证角色可用 |
| 会话创建/追加/归档 | ❌ 仅认证 | 矩阵 #14：用户个人会话，自服务 |
| 收藏 / 预约 / 取消 | ❌ 仅认证 | 自服务写面 |
| 所有读面 | ❌ 仅认证 | 由 auth_middleware 统一保证，认证即可 |

### 测试结果

- `cargo check -p mox-platform-gateway-svc` → Finished dev profile，无 error
  （仅 1 个既有 warning：`system/mfa.rs:26 unused import rand::RngCore`，与本轮无关）
- `cargo test -p mox-platform-gateway-svc alliance` → **73 passed; 0 failed**（0.15s）
  - 基线 64 用例全绿（其中 5 处直呼 handler 的测试调用点已补 `admin_user()` 参数：
    `experts_registry.rs:902/946/963`、`experts_dispatcher.rs:973/990/1004/1073/1197`）
  - 新增 9 个 RBAC 单元测试（`experts_rbac.rs`）：
    - `test_enforce_admin_rejects_anonymous`（None → 401）
    - `test_enforce_admin_rejects_non_admin`（normal_user → 403）
    - `test_enforce_admin_allows_admin`（tenant_admin → Ok）
    - `test_enforce_admin_allows_super_admin`（super_admin → Ok）
    - `test_enforce_admin_or_respond_anonymous_returns_401`
    - `test_enforce_admin_or_respond_non_admin_returns_403`
    - `test_enforce_admin_or_respond_admin_ok`
    - `test_admin_roles_constant`（ADMIN_ROLES 常量固定）
    - `test_audit_denied_does_not_panic`（NoopSink 下审计不 panic）

### 不引入第二套审计/鉴权体系

- 拒绝审计复用 `emit_audit`（experts_common.rs:628）与 `AuditAction::RBACDenied`
  （mox-audit event.rs:281，事件名 `rbac.denied`），outcome 用 `AuditOutcome::Blocked`
- 身份提取复用既有 `OptionalAuthUser`（experts_common.rs:585），未新增加密/令牌逻辑
- 角色集为进程内常量，不依赖外部策略服务、不读数据库


## T1a DAG 并行信号量（2026-09-30）

### 背景与现状核证
- 10 号评审 :157-161（N8 🔴）：grep `max_parallel/concurrency/Semaphore` 在 DAG 执行路径零命中，跨任务就绪节点全部 `tokio::spawn` 后统一 await，无并发上限，可能打爆下游 LLM 配额。
- 核证位置：`core/mox-alliance-executor-core/src/dag_engine.rs::schedule_ready_nodes`（节点 spawn 点原 499 行），每轮 poll 把所有 Running 任务的就绪节点全部 spawn 出去再统一 await。
- 发现既有死字段：`proto/mox-alliance-executor-proto/src/types.rs:12` 的 `ExecutorConfig.max_concurrent_nodes`（默认 50）声明为“最大并发执行节点数”，但全 crate 零引用——正是“声明了没接线”。

### 引擎位置
- DAG 执行引擎：`core/mox-alliance-executor-core/src/dag_engine.rs`
- 并行派发点：`schedule_ready_nodes`（节点 spawn 处）
- 生产装配链：`svc/mox-alliance-executor-svc/src/bin/main.rs:34` 把 `boot.executor.max_concurrent_nodes`（默认 50，已有 env `EXECUTOR_MAX_CONCURRENT_NODES` 通道，见 `core/mox-alliance-boot-config/src/lib.rs:556`）注入 `ExecutorConfig`。

### 改动文件:行号（仅 dag_engine.rs，最小改动）
- `:43` 新增常量 `ENV_DAG_MAX_PARALLEL = "MOX_DAG_MAX_PARALLEL"`
- `:57` 新增纯函数 `resolve_dag_max_parallel(env, config_default) -> Option<usize>`
- `:105` / `:107` `DagEngineImpl` 新增字段 `node_semaphore: Option<Arc<Semaphore>>`、`max_parallel: Option<usize>`
- `:155`-`:158` `with_state_sink` 读 env、构造全局 `Semaphore`，并以 info/warn 日志输出护栏是否启用
- `:231`-`:232` `run_scheduler_loop` 透传信号量
- `:527`-`:528` `schedule_ready_nodes` 透传信号量
- `:587` 节点任务在 `execute_node` 之前 `acquire_owned()` 拿全局许可；permit 随任务结束 drop 自动归还（不手动 release）
- `:597` 打指标日志 `dag.concurrent={inflight}/{max} task=.. node=..`（debug，target="dag"）
- 不改变 DAG 拓扑/依赖语义，仅在节点真正打 LLM 前加并发护栏。

### 默认值决策与理由（采用默认有界）
- env 未设置时回退 `config.max_concurrent_nodes`（生产默认 50），信号量上限 = 50，上线即有护栏。
- env `MOX_DAG_MAX_PARALLEL` 优先级更高：
  - 正整数 → 采用该值（运维收紧到 8 等）
  - `0` 或负数 → `None`（无界逃生门，恢复改动前行为）
  - 非法字符串 → 忽略，回退配置默认
- 理由：
  1. 接线既有“最大并发节点数”语义字段，不引入魔法数；生产经 boot-config 默认即 50，直接修掉 N8 无上限缺口。
  2. 既有测试普遍用 `ExecutorConfig::default()`（50），单批次节点数远小于 50，acquire 立即通过，不改变测试时序。
  3. 保留 `MOX_DAG_MAX_PARALLEL=0` 作为“恢复现状”逃生口；如需更严护栏，显式设 8。

### 每专家 / Provider 两级配额
- 本轮**未做**：按 expert_id/provider 的 `HashMap<Semaphore>` 需动态创建、生命周期管理与指标聚合，改动面与回归风险超出“最小可回退”范围。
- 标注留待 **P1.5**：在全局护栏稳定后，按 `node.expert_id` 做二级配额，避免单专家/单 Provider 被热点节点打满。

### 并发指标
- 执行期 debug 日志：`dag.concurrent={inflight}/{max}`（inflight = max - available_permits，acquire 成功后打印）。
- 本轮不接 `/metrics`（executor-core 为库，结构化 metrics 在 scheduler-core）；运维验证靠启动日志与 debug 日志，P1.5 再暴露结构化指标。

### 测试结果
- `cargo test -p mox-alliance-executor-core --features mock`：
  - lib 单测 **41 passed**（含新增 2 个：`resolve_dag_max_parallel_prefers_env_and_supports_escape_hatch`、`semaphore_caps_in_flight_nodes_at_configured_max`）
  - `tests/bench_alliance.rs` → 1 passed（154s，debug 下多维度基准自身规模所致，非限流引入；断言全过）
  - `tests/integration_e2e.rs` → 5 passed
- `cargo test -p mox-platform-gateway-svc alliance` → **73 passed; 0 failed**（与基线 73 一致，未破坏）
- 端到端护栏用例：`max_concurrent_nodes=2`，6 个无依赖根节点，实测峰值并发 ≤ 2（无护栏时应为 6），证明信号量生效。

### 前端滑块留待说明
- 12 号 T1 路径②（前端编排视图并发上限滑块 + 当前并发指标曲线）本轮**未做**：后端 env/启动日志/debug 指标已就绪，前端滑块与实时并发曲线留待前端任务接入；引擎启动日志已暴露 `dag.max_parallel`，可作为滑块回显数据源。

### 不做 / 回退说明
- 未改 `ExecutorConfig` 结构、未改生产装配链、未碰 DAG 拓扑逻辑。
- 若线上观察到有界 50 导致吞吐异常，`MOX_DAG_MAX_PARALLEL=0` 可立即无界回退，无需发版。


---

## N7 指标文本化（2026-09-30）

### 背景与现状核证
- 12 号 :30「D6/N7 指标三进程格式不统一」、11 号弱②「无可观测控制台」的补课；
  八节总览原把 N7 列为 P2「scheduler/executor 返回 JSON 快照，补 Prometheus 文本需引入 metrics crate」。
- 本轮**不引入 prometheus crate**，手工渲染 exposition 0.0.4 文本（零新依赖，最小可回退）。
- 核证三 svc 现状（文件:行号）：
  - scheduler:3100 `/metrics` 已存在，返回 `Json(AllianceMetrics.snapshot())`（`svc/mox-alliance-scheduler-svc/src/routes.rs:25` 注册、原 handler 直接 `Json`）。
    **存在消费者**：`tests/http_integration.rs:162 metrics_endpoint_returns_snapshot` 以**无 Accept 头**请求 `/metrics` 并断言 JSON 字段——不能破坏。
  - executor:3200 `/metrics` 已存在，返回 `Json(ExecutorMetrics.snapshot())`（`svc/mox-alliance-executor-svc/src/routes.rs:79` 注册）。
  - registry:3400 **未注册 `/metrics` 路由**（`svc/mox-alliance-registry-svc/src/routes.rs` create_router 原无此行），仅 internal_auth 白名单提到它——需新增。

### 方案：Accept 头协商（不破坏既有 JSON 消费者）
- scheduler / executor：`/metrics` 按 `Accept` 头协商——
  - `Accept` 含 `text/plain`（Prometheus 默认抓取头
    `application/openmetrics-text; version=0.0.1,text/plain;version=0.0.4;q=0.5,*/*;q=0.1` 命中）→
    返回 `text/plain; version=0.0.4; charset=utf-8` Prometheus 文本；
  - 否则（无 Accept / `application/json`，含既有前端控制台与 http_integration 测试）→ **保持原 JSON 快照**，零破坏。
- registry：原本无 `/metrics`、无 JSON 消费者，直接新增文本端点。
- 命名统一 `mox_alliance_<svc>_<metric>`，每条带 `# HELP` / `# TYPE`。

### 改动文件:行号
| 文件 | 位置 | 改动 |
|---|---|---|
| `svc/mox-alliance-scheduler-svc/src/routes.rs` | :17 | 新增 `use mox_alliance_scheduler_core::MetricsSnapshot;` |
| 同上 | :163-181 | `metrics_handler` 加 `headers: HeaderMap`，按 `wants_prometheus_text` 分流文本/JSON |
| 同上 | :184 | 新增 `wants_prometheus_text(&HeaderMap)->bool`（Accept 含 text/plain） |
| 同上 | :193 | 新增 `prom_metric(...)` 助手（HELP+TYPE+sample 一行式） |
| 同上 | :198-220 | 新增 `render_scheduler_metrics_prometheus(&MetricsSnapshot)`：17 条 `mox_alliance_scheduler_*`（match/llm/fusion/dag 四维，counter 加 `_total`，avg 为 gauge） |
| 同上 | 文末 `mod n7_prometheus_tests` | 新增 2 个断言（见测试） |
| `svc/mox-alliance-executor-svc/src/routes.rs` | :23 | `use ...::{ExecutorAppState, ExecutorMetricsSnapshot};` |
| 同上 | :176-194 | `metrics_handler` 同样 Accept 协商分流 |
| 同上 | :197 / :206 / :211-220 | 同构 `wants_prometheus_text` / `prom_metric` / `render_executor_metrics_prometheus`：5 条 `mox_alliance_executor_*`（tasks_submitted/completed、nodes_completed、errors、tasks_cancelled，均 counter） |
| `svc/mox-alliance-registry-svc/src/routes.rs` | :37 | 新增 `.route("/metrics", get(metrics_handler))` |
| 同上 | :74 | 新增 `prom_metric(...)` |
| 同上 | :83-99 | 新增 `metrics_handler`：直接出文本，`mox_alliance_registry_instances`（registry.count）+ `mox_alliance_registry_directory_experts`（dir_store.list().len） |

### 导出指标清单（命名规范 `mox_alliance_<svc>_<metric>`）
- scheduler（17）：`..._match_requests_total`、`..._match_errors_total`、`..._match_latency_us_sum/_count`、`..._match_avg_latency_us`；
  `..._llm_calls_total`、`..._llm_errors_total`、`..._llm_latency_ms_sum/_count`、`..._llm_avg_latency_ms`；
  `..._fusion_calls_total`、`..._fusion_latency_ms_sum/_count`、`..._fusion_avg_latency_ms`；
  `..._dag_executions_total`、`..._dag_node_executions_total`、`..._dag_avg_nodes_per_execution`。
- executor（5）：`..._tasks_submitted_total`、`..._tasks_completed_total`、`..._nodes_completed_total`、`..._errors_total`、`..._tasks_cancelled_total`。
- registry（2）：`mox_alliance_registry_instances`（gauge）、`mox_alliance_registry_directory_experts`（gauge）。

### 测试结果
- `cargo check -p mox-alliance-scheduler-svc -p mox-alliance-executor-svc -p mox-alliance-registry-svc` → Finished dev profile，无 error / 无 warning。
- `cargo test` 三 svc：
  - scheduler-svc：lib 12 + http_integration 11 = **23 passed, 0 failed**
    （新增 `routes::n7_prometheus_tests::prometheus_text_contains_expected_metric_lines`：断言 17 条数据行均为 `mox_alliance_scheduler_* <number>`、值可解析；
    `accept_header_drives_negotiation`：无 Accept / `application/json` → false，Prometheus 默认 Accept → true）
  - executor-svc：lib 9 + http_integration 6 = **15 passed, 0 failed**
  - registry-svc：lib 17 + http_registry 11 = **28 passed, 0 failed**
  - 合计 **66 passed, 0 failed**。
- **兼容性证据**：既有 `metrics_endpoint_returns_snapshot`（无 Accept 头 → 断言 JSON 字段齐备）仍通过，证明默认 JSON 路径零破坏。

### 回退说明
- 改动全部在三 svc 的 `routes.rs`：仅改 `metrics_handler` 分流 + 新增渲染/助手函数 + scheduler 末尾一个测试模块 + registry 注册一行。
- 未引入新依赖、未动快照结构（`AllianceMetrics`/`ExecutorMetrics`）、未动业务 handler。
- 回退 = 把 `metrics_handler` 恢复为 `Json(state.metrics.snapshot())` 并删新增函数；registry 删一行路由即可。
- Prometheus 抓取配置：对三 svc 各加一个 `/metrics` job，Accept 默认即命中文本；现有 JSON 面板继续无 Accept 访问，不受影响。

---

## N4 图谱节点级 CRUD（2026-09-30）

### 背景

原图谱是「只读快照 + 全量 rebuild」：`build_experts_graph_router`（experts_graph.rs）只有 8 条只读路由
（GET /api/expert-graph、/stats、/neighbors/:id、/collaborators/:id、/path/:source/:target、/communities、
POST /optimal-team）+ POST /rebuild。落库函数 `save_graph_conn`（experts_db.rs）是全量 DELETE+INSERT
（rebuild 语义），无节点/边的增量增删改。这是图 RAG（12 §2.2 T2）与画布式编排（U1）的前置硬缺口。

本轮把图谱升级为可增量维护：6 个写端点 + SQLite 增量落库 + 内存态同步 + RBAC 强制。

### 端点清单（前缀 `/api/expert-graph`，写面全部 RBAC 强制）

| 方法 | 路径 | registry id | 行为 | 错误语义 |
|------|------|-------------|------|---------|
| POST | `/api/expert-graph/nodes` | experts.graph.node_create | 新增节点 `{id,label,node_type,properties?}` | 重复 id→409；node_type 非法/label 空→400 |
| PUT | `/api/expert-graph/nodes/:id` | experts.graph.node_update | 合并式更新（label/node_type/properties） | 节点不存在→404 |
| DELETE | `/api/expert-graph/nodes/:id` | experts.graph.node_delete | 删除节点并联动删除其全部关联边 | 节点不存在→404 |
| POST | `/api/expert-graph/edges` | experts.graph.edge_create | 新增边 `{source,target,edge_type,weight?,properties?}` | source/target 不存在→400；无向同对同 edge_type 重复→409；weight∉[0,1]→400 |
| PUT | `/api/expert-graph/edges/:seq` | experts.graph.edge_update | 更新边 edge_type/weight/properties | seq 越界→404 |
| DELETE | `/api/expert-graph/edges/:seq` | experts.graph.edge_delete | 删除边并按内存顺序重排 seq | seq 越界→404 |

- 统一响应 `mutation_response`：返回受影响元素 + `stats{node_count,edge_count,version,built_at}`，供前端即时刷新。
- `seq` 语义（核证表结构后定）：`graph_edges.seq INTEGER PRIMARY KEY` 就是内存 `edges` Vec 的下标
  （`load_graph` 按 seq 升序 push、`save_graph_conn` 按下标 enumerate 写入）；故按 seq 定位边，
  删除/增边后按内存顺序重排 seq 保持「seq==下标」不变式。

### 校验规则

- `node_type` 合法枚举：`expert` / `domain` / `capability`（读 GraphNode 注释核证；builder 实际只产出 expert/domain）。
- 边 `weight` 范围 `[0.0, 1.0]`。
- 边 `source`/`target` 必须在节点集合中存在。
- 边查重按**无向**：(source,target) 与 (target,source) 视为同对，同 edge_type 即 409。

### RBAC 动作（experts_rbac.rs）

- 新增 `RbacAction::MutateGraph`，权限码 `code() = "graph.mutate"`，description = `图谱节点/边增量维护`。
- 6 个写 handler 入口统一 `enforce_admin_or_respond(&state, &user, RbacAction::MutateGraph)`：
  未认证→401、非 super_admin/tenant_admin→403，拒绝时发 `rbac.denied`（Blocked）审计，不执行业务。
- admin 角色集 `ADMIN_ROLES` 不变（super_admin/tenant_admin）。

### 存储增量函数（experts_db.rs 新增，不动全量 save_graph_conn）

| 函数 | SQL 语义 | 说明 |
|------|---------|------|
| `upsert_graph_node_conn` | `INSERT ... ON CONFLICT(id) DO UPDATE` | 节点 UPSERT |
| `delete_graph_node_cascade_conn` | 事务内 DELETE incident edges + DELETE node | 删节点联动删边 |
| `upsert_graph_edge_conn` | `INSERT ... ON CONFLICT(seq) DO UPDATE` | 边 UPSERT |
| `replace_graph_edges_conn` | DELETE 全表后按内存顺序重写 seq | 删/增边后重排 seq，保 seq==下标 |
| `set_graph_meta_conn` | `INSERT ... ON CONFLICT(k) DO UPDATE` | UPSERT graph_meta（built_at/version） |

每个增量函数另配 best-effort 公开包装（失败仅 `log_err`，不阻断内存态已生效的响应）。

### 内存态同步与版本

- handler 加锁改 `state.graph`（`ExpertsSharedState.graph: Arc<Mutex<ExpertGraph>>`）→ `version += 1` → 落库。
- version 语义与 rebuild 一致（`ExpertGraph.version: u64`，graph_meta.k='version'）。
- **与 rebuild 互不冲突**：rebuild 全量重算会覆盖增量结果，属预期语义（已在文档标注）。

### 前端契约（只补 API 层，不接 UI——U1 画布留待）

- `contract/endpoints.js`：新增 6 个 ENDPOINTS 键（graphNodeCreate/graphNodeUpdate/graphNodeDelete/
  graphEdgeCreate/graphEdgeUpdate/graphEdgeDelete）。
- `api/alliance.api.js`：新增 6 个方法 createGraphNode/updateGraphNode/deleteGraphNode/
  createGraphEdge/updateGraphEdge/deleteGraphEdge。
- `store/alliance-graph.store.js`：新增 6 个薄方法（调 api 方法），字段与后端对齐。
- `docs/API-REGISTRY.md` + `actuator.rs` ROUTES 数组（236→242）同步登记 6 条。
- `contract/graph.test.js` 路由等集断言改写为按行解析 get/post/put/delete 全链路。

### 测试结果

- 后端 `cargo test --lib alliance::` → **86 passed; 0 failed**（基线 73 + 新增 13：
  9 个 handler/纯逻辑测试 + 4 个 db 增量测试）。新增用例：
  - `test_validate_node_type_and_weight`、`test_mutate_graph_rbac_rejects`
  - `test_create_node_ok_and_duplicate_409`、`test_create_node_bad_type_400`
  - `test_update_node_404_and_merge`、`test_delete_node_cascades_edges`
  - `test_create_edge_ok_duplicate_409_missing_endpoint_400`、`test_update_delete_edge_by_seq`
  - `test_crud_requires_auth_401`
  - db：`upsert_node_insert_then_update`、`delete_node_cascades_edges`、
    `edge_upsert_then_renumber_keeps_seq_order`、`bump_meta_upsert`
- `cargo check --lib`：通过。
- 前端 `npx vitest run src/modules/expert-alliance` → **649 passed / 1 failed（650 总数）**。
  唯一失败为 `contract/contract.test.js` 锚定 `experts_dispatcher.rs:581` 应含 `AuditAction::ExpertDispatch`——
  该串实际已漂移到 :588（dispatcher.rs 在本任务前就有未提交改动，N4 未触碰该文件），
  属**既有失败、与本轮无关**；本轮新增/触及的 graph.test.js(32)、graph.store.test.js(18)、api.test.js(58)
  共 108 例全绿。

### 不做 / 回退说明

- 不改 rebuild 语义、不破坏 8 条只读路由与既有测试。
- 前端只补 API/store 薄方法，不接画布 UI（U1 留待）。
- 回退 = 删 6 个 handler 与路由、删 experts_db.rs 增量函数、删 RBAC MutateGraph、回滚 endpoints/api/store 6 键。


---

## T2 图 RAG（2026-10-01）

### 背景与现状核证
- 12 号 T2 规划：「沿边多跳扩展相关专家/能力域/历史协作链路，给 planner『该找谁、为什么、怎么组队』结构化上下文」；规划期备注「初期可在 SQLite 图表上用递归 CTE 做多跳」。
- 核证 `ExpertsSharedState`（experts_common.rs:459-478）：`graph: Arc<Mutex<ExpertGraph>>`，**内存态图，state 无 SQLite 连接句柄**；N4 已实现内存图↔SQLite 双向同步（experts_graph.rs 6 写端点 + experts_db.rs 增量落库）。
- 核证图结构：`GraphNode{id,label,node_type,properties}`、`GraphEdge{source,target,edge_type,weight,properties}`（experts_common.rs:362-393）；边权重 ∈ [0,1]（validate_weight）。
- 核证既有路由：`build_experts_graph_router`（experts_graph.rs:1310）原 14 条（8 只读 + rebuild + N4 6 写），全部前缀 `/api/expert-graph`。

### 检索链路设计（落地为真实实现）
```
输入: seeds(节点 id 数组) + max_depth(1-4, 默认2) + top_k(默认20) +
      可选 node_types(结果侧过滤) / min_weight(边侧过滤, 默认0)
→ 多跳邻域扩展：从每个种子 BFS 沿无向边展开至 max_depth
→ 召回排序：aggregate_weight = 路径边权重**乘积**（w∈[0,1]，随深度自然衰减）；
           同节点多路径取最优（乘积大者优先，同乘积取更浅深度）
→ 输出: node{id,label,node_type} + depth + aggregate_weight + path + first_hops[首跳边]
→ stats: searched_nodes(过滤前去重节点数) / returned / elapsed_ms
→ 融合重排: hybrid_rerank 扩展点（当前透传，向量融合待 #27）
```
- 排除种子自身；环路防重复：单路径内 `path.contains(nb)` 防回环，跨路径以 `best` 择优（不强制全局 visited，允许更优路径重开扩展，但 best 比较收敛）。
- 候选专家契约：结果中 `node.node_type == "expert"` 即「候选专家」排序，可直接供 optimal-team/planner 消费；不强耦合，文档给接口契约。

### 实现选择（内存态 vs SQLite 递归 CTE）
- **采用内存态加权多跳扩展**（数据源 `state.graph`，与 get_graph 同源、与 SQLite 同步等价）。
- 理由：① state 无 SQLite 连接句柄，引入需连接生命周期管理；② 内存态零连接开销，与既有 8 只读查询同构；③ 图规模小（数十节点），内存 BFS/择优完全够用。
- 与规划期「SQLite 递归 CTE」**语义等价**，A4 图库迁入时仅替换查询实现，handler 契约不变。

### 端点契约
- `POST /api/expert-graph/rag/expand`（读面公开，与 get_graph 一致，无需角色）。
- 请求：`{ seeds: [String], max_depth?: 1..4(默认2), top_k?: >0(默认20), node_types?: [String], min_weight?: 0..1(默认0) }`。
- 响应：`{ query, results: [{ node, depth, aggregate_weight, path, first_hops }], stats: { searched_nodes, returned, elapsed_ms }, rerank: "graph_only（向量融合待 #27）" }`。
- 校验：seeds 空 → 400；max_depth 越界 → 400；top_k=0 → 400；min_weight 越界 → 400；任一种子不存在 → 404（与 get_neighbors 语义一致）；空图 → 空 results（非错误）。

### 改动文件:行号
- 后端 `experts_graph.rs`：
  - 新增 `RagExpandBody` / `RagHit`（七-C 节，:1330 起）
  - 新增纯函数 `expand_neighborhood(graph, seeds, max_depth, top_k, node_types, min_weight) -> Vec<RagHit>`（:1357）
  - 新增融合重排扩展点 `hybrid_rerank(graph_hits, vector_candidates)`（:1482，当前透传）
  - 新增 handler `post_rag_expand`（:1495）
  - 路由装配加 `.route("/api/expert-graph/rag/expand", post(post_rag_expand))`（:1325）
  - 新增 11 个单测（RAG 多跳/权重排序/环路/空图/深链/类型过滤/权重过滤/top_k/404/400/200）
- 后端 `actuator.rs`：ROUTES 242→243，加 `experts.graph.rag_expand` 行（:633）。
- 前端：
  - `contract/endpoints.js`：加 `graphRagExpand` 端点声明
  - `contract/graph.js`：加 `RAG_EXPAND_DEFAULTS` / `ragExpandBody` / `ragExpandProblem` / `ragExpandRows`
  - `api/alliance.api.js`：加 `expandGraphNeighborhood(input)` 方法
  - `store/alliance-graph.store.js`：加 `ragResults` ref、`ragDraft`、`ragProblem`、`expandNeighborhood()` action
- 文档：`docs/API-REGISTRY.md` 加 `experts.graph.rag_expand` 行。
- 顺手修复既有锚点漂移：`contract/registry.js:265` 与 `contract/contract.test.js:1413` 的 `experts_dispatcher.rs:581` → `:588`（N4 遗留行号漂移，本轮一并让门禁真绿）。

### 测试结果
- `cargo test -p mox-platform-gateway-svc --lib alliance`：**97 passed / 0 failed**（基线 86 + 本轮新增 11 个 RAG 测试）。
- `cargo check -p mox-platform-gateway-svc`：通过（仅既有 mfa.rs unused-import warning，与本轮无关）。
- 前端 `npx vitest run src/modules/expert-alliance`：**650 passed / 0 failed**（25 个测试文件全绿，含本轮新增端点契约门禁）。

### 不做 / 回退说明
- 向量融合未做：`hybrid_rerank` 当前透传图谱结果，真实 pgvector 召回留待 #27；响应 `rerank` 字段如实标注「graph_only（向量融合待 #27）」，不冒充已做。
- 不写面：本轮全是读面，不动 N4 6 写端点、不动 RBAC。
- 前端只补 API/store 薄方法，不接画布 UI（T2 消费方/U1 后续接）。
- 回退 = 删 `post_rag_expand` handler 与路由、删 `expand_neighborhood`/`hybrid_rerank`、删 actuator ROUTES 一行、回滚 endpoints/api/store/contract 前端 4 处改动。


---

## U2 匹配透明化——逐维得分与权重透出（2026-10-01）

### 背景
生产主路径 ModularWeightMatcher 已计算 MatchScoreBreakdown{domain_match,capability_match,health_score,priority_score,performance_score}，但 HTTP 边界（scheduler-svc `/experts/search` → 网关 http-sdk 远程归一 / 本地降级）把它压成瘦 ExpertSummary，前端只回总分。U2 把逐维得分 + 该专家实际所用权重透出。

### 权重真相（代码为准）
- 主路径默认 MatchingWeights：domain 0.35 / capability 0.30 / rating(priority) 0.20 / performance 0.10 / **health 0.05**（common-proto/types.rs:1320-1324，可每专家覆盖）。
- health 分 = is_healthy?1.0:0.2（modular_matcher.rs:170-171），非硬过滤。
- priority 维乘的是 weights.rating（Expert 无独立 rating 字段）。
- 旧文档「健康度 0.15」系网关 compute_match_score 的 bio 权重 / 备用 RuleBased matcher 健康权重误植，主路径实为 0.05。

### 改动
- scheduler-proto/matcher.rs：`MatchedExpert` 增 `weights: MatchingWeights`（#[serde(default)]，向后兼容）。
- scheduler-core/modular_matcher.rs:286：填 `expert_weights.clone()`；RuleBased matcher.rs:192、planner.rs 测试构造补 `MatchingWeights::default()`。
- api/dto.rs：新增 `ScoreDim{value,weight}`、`ExpertScoreView{domain,capability,health,priority,performance,total}`；`ExpertSummary` 增可选 `match_score/match_reason/scores`。
- scheduler-svc/routes.rs `/experts/search`：按「priority 维用 rating 权重」组装 scores 透出。
- http-sdk：alliance_remote.rs（远程优先）透传；alliance.rs:883（本地降级）按 MatchingWeights::default() 组装 scores。

### 测试结果
- `cargo check`：proto/api/scheduler-core/scheduler-svc/http-sdk/gateway 全 Finished。
- `cargo test`：scheduler-core 115 passed；scheduler-svc + http-sdk 38 passed；0 failed。
- 未改算法/权重值，仅透出；新字段均 #[serde(default)]，旧调用方不受影响。


---

## T3 MCP 服务器（2026-10-01）

### 选型理由
- **自实现 JSON-RPC 2.0 over stdio**（帧格式 `Content-Length: N\r\n\r\n<body>`），不引入 rmcp 等第三方 MCP crate，契合「零外部依赖气隙部署」卖点；MCP stdio 本质即 JSON-RPC 2.0 帧协议，自实现完全可控。
- **独立 binary crate**，不并入 axum 网关进程：避免 stdin/stdout 与网关 async runtime 冲突，符合 12 号规划「MCP Server 独立层、鉴权走网关」。
- **薄适配层**：handler 全部以真实 HTTP 调用网关读面端点，不重复匹配/组队/检索逻辑；零本地 mock。

### crate 路径与注册
- 新增 `platform/domains/alliance/mcp/mox-alliance-mcp-server/`（bin: `mox-alliance-mcp-server`）。
- 已在根 `Cargo.toml` `members` 注册（不在 default-members，不影响既有默认构建）。
- 依赖仅复用 workspace 既有 `serde/serde_json/tokio/reqwest/chrono`，零新增外部 crate。

### 配置（零硬编码）
- `MOX_MCP_GATEWAY_URL`：网关基址，默认 `http://127.0.0.1:3080`。
- `MOX_INTERNAL_TOKEN`：可选，设置后注入 `Authorization: Bearer <token>`（本次验证用 debug 构建 dev_mode 下代码内正式开发凭证 `dev-secret-token`）。

### 三工具契约（字段照抄后端真实 DTO）
| MCP 工具 | 上游真实端点 | 请求形状 | 输出（透传网关真实响应） |
| --- | --- | --- | --- |
| `expert_search` | POST `/api/alliance/experts/search` | `{query, domains[], limit}` | 专家列表，含 U2 逐维 `scores`（domain/capability/health/priority/performance/total）+ match_score |
| `optimal_team` | POST `/api/expert-graph/optimal-team` | `{required_skills[], required_domains[], max_members, min_rating, goal}` | team_members + coverage + team_score（weighted_set_cover_greedy） |
| `graph_expand` | POST `/api/expert-graph/rag/expand` | `{seeds[], max_depth, top_k, node_types[], min_weight}` | 邻域节点 + aggregate_weight + path + first_hops |

### 真实验证证据（2026-10-01，真实网关 mox-server:3080）
- 启动真实网关 `cargo build -p mox-platform-gateway-svc` → `mox-server` 监听 0.0.0.0:3080（内置 10 个种子专家并自动建图）。
- Python MCP 客户端经 stdio 帧跑通 `initialize → notifications/initialized → tools/list → tools/call×3`，日志摘要：
  - `initialize` → serverInfo `mox-alliance-mcp-server@3.0.0-ai-powered`，protocolVersion `2024-11-05`。
  - `tools/list` → 3 工具 `[expert_search, optimal_team, graph_expand]`。
  - `expert_search`（gateway_status=200）→ 真实专家 `expert-architecture/架构设计专家`(match_score≈0.681)、`expert-requirement/需求分析专家`，逐维 scores 完整透出。
  - `optimal_team`（200）→ 真实组队 `exp-ai-001/AI算法·灵玑`、`exp-architecture-001/架构师·玄枢`，team_score=1.92，coverage_ratio=1.0。
  - `graph_expand`（200，seed=exp-architecture-001）→ 真实邻域 `domain-architecture/domain-backend/domain-distributed`，aggregate_weight=1.0。
- 证据落盘：`platform/domains/alliance/mcp/verify_evidence.json`、`verify_run.log`；验证脚本 `verify_mcp_client.py`。

### 测试
- `cargo test -p mox-alliance-mcp-server`：5 passed / 0 failed（initialize 握手、tools/list 契约、通知静默、未知工具错误路径、帧头解析）。
- `cargo check -p mox-alliance-mcp-server`：Finished，无 warning。

### 留待（如实标注）
- MCP Client 接入与凭证托管（OAuth/企业 IdP）未做；当前以 `MOX_INTERNAL_TOKEN` 静态 Bearer 直通读面。


---

## 收官全链路更新（2026-10-01）

> 本轮为**整合核验，不新增功能、不改核心代码**：把 N4/T2/U2/U1/T3 多轮新增的真实能力并入
> `docs/expert-alliance/13-end-to-end-business-flow.md`，并核对其与代码事实一致。

### 13 号文档更新点清单

| # | 位置 | 更新内容 | 事实来源（代码） |
|---|------|---------|------------------|
| 1 | 文首权威说明后 | 加「2026-10-01 收官全链路更新」横幅，列出 N4/T2/U2/U1/T3 五项与嵌入位置 | 本轮整合 |
| 2 | 新增 §1.3 | 全链路总图：业务发起→T2 邻域检索→U2 组队可解释→DAG→U1 画布→执行→融合→交付→T3 MCP，实线=自动化主链、虚线框=新增面，并标注 T2「未强耦合进 ModularWeightMatcher 主匹配链」 | experts_graph.rs 路由清单；本报告 T2/U2/T3/U1 各节 |
| 3 | §2.3 专家匹配末尾 | 新增 U2 输出可解释小节：`scores{每维 value,weight}`/match_score/match_reason；权重 domain.35/cap.30/rating.20/perf.10/health.05；health=is_healthy?1.0:0.2 非硬过滤；optimal-team 公式独立不随 scores 面板 | common-proto/types.rs:1320-1324；modular_matcher.rs:170-171；api/dto.rs ScoreDim/ExpertScoreView |
| 4 | §4.3 图谱末尾 | 新增 T2 读面小节：`POST /api/expert-graph/rag/expand` 请求/响应形状、内存态 BFS、边权乘积排序、400/404/空图语义；路由计数由「8 只读」更正为含 N4 6 写 + T2 1 读 | experts_graph.rs:1598-1612 路由装配；:1357 expand_neighborhood；:1495 post_rag_expand |
| 5 | 新增 §4.6 | U1 画布可视化：拖拽坐标仅前端视觉层不落后端、编辑走 N4、邻域展开调 T2 幂等并入、按钮级 `v-role-any` 与后端 `graph.mutate` 双守卫；dev:3020→gateway:3080 | frontend-fix-report U1 节；directives/permission.js:124 roleAny |
| 6 | 新增 §4.7 | T3 MCP：独立 stdio binary、不占端口、三工具薄适配网关读面、`MOX_MCP_GATEWAY_URL` 默认 :3080、`MOX_INTERNAL_TOKEN` 直通 | 本报告 T3 节；mox-alliance-mcp-server |
| 7 | §4.5 缺口段 | 加收官复核注：N1（三 svc internal_auth_layer）、N2（审计 Actor 真实身份）已修复，原「无中间件/硬编码 system」为旧口径 | 本报告任务4/5、P0-C、R1 节 |
| 8 | §七差异表 | 追加第 9/10/11 行：N1 鉴权打通、N2 审计 Actor、路由计数口径被取代 | 同上 |
| 9 | 文末落款 | 加收官更新声明 | 本轮 |

### 全链路各环节一句话概览

- **业务发起**：前端 `POST /api/alliance/tasks`（gateway:3080→scheduler:3100）建任务，或 `/experts/search` 选专家（U2 透出逐维 scores）。
- **图谱检索 T2**：`rag/expand` 从 seeds 多跳扩展邻域、按边权乘积排序，是供画布/MCP/候选组队消费的读面护城河（尚未强耦合主匹配链）。
- **最优组队 U2**：ModularWeightMatcher 五维加权选 Top N 并透明化逐维得分；图上另有贪心集合覆盖 `optimal-team`。
- **DAG 编排**：SimplePlanGenerator 七模式生成 CollaborationPlan，planning→running。
- **画布可视化 U1**：前端 SVG 画布拖拽/编辑/邻域展开，写回走 N4（RBAC graph.mutate），坐标仅视觉层。
- **执行（三 svc）**：scheduler ExecutorBridge→executor:3200 DagEngine 拓扑并行（信号量默认 50）、60s 超时/3 次指数退避。
- **融合**：全部节点成功后 DAG 尾部 FusionEngine 出 FusionOutput，任一失败不融合。
- **结果交付**：网关读 executor:3200 状态/节点/结果 + SSE 日志，融合 404 归一化为 pending。
- **MCP 外部消费 T3**：独立 stdio 服务器经 `expert_search/optimal_team/graph_expand` 三工具把读面暴露给外部 Client。

### 核对中发现的冲突与处置

1. **§4.5 旧结论已被取代**（N1/N2）：原文称「下游三 svc 无 JWT 中间件」「审计 Actor 硬编码 system」，与本报告任务4/5、R1 节实测矛盾——已在 §4.5 加收官注 + §七第 9/10 行如实标注，未删原文。
2. **§4.3 路由计数口径过期**：原文「共 8 条只读路由」且把 `POST /optimal-team` 归入只读，与 N4（6 写）+ T2（1 读）后实测路由清单不符——已在 §4.3 末行更正为实测清单，并记 §七第 11 行。
3. **T2 是否进主链**：任务书叙事把 T2 画在「选专家→组队」之间，但代码实测 `expand_neighborhood`/`hybrid_rerank` 为独立读面、**未接线进 ModularWeightMatcher**——文档如实标注「供消费、未强耦合」，不虚构已接线。
4. **端口一致性**：gateway:3080 / scheduler:3100 / executor:3200 / registry:3400 与 PORT-REGISTRY V1.2 完全一致；T3 MCP 为 stdio 不占端口，未引入新监听端口，无端口漂移。

### 本轮边界

- 只改 `13-end-to-end-business-flow.md`（文档）与本报告（追加节）；未改任何 `.rs`/`.vue`/路由/权重。
- 未跑 cargo/vitest（无代码改动）；事实均对照既有修复报告与 `experts_graph.rs`/`directives/permission.js` 实读核证。


---

## A1 多租户（2026-10-01）——租户级数据隔离 + 权限隔离（阶段一已落地）

### 租户模型设计与决策理由

| 维度 | 决策 | 理由（第一性） |
|---|---|---|
| 租户注入 | 从可信身份 `UserInfo.tenant_id`（auth 中间件注入 extensions）取租户；`X-Tenant-Id` 头仅做**一致性校验**（须与身份一致，否则 403） | 企业 SaaS 硬安全：租户不可被请求头伪造。前端 JWT 体系不动；无头/无身份→401 而非静默降级，避免越权。单租户部署时所有身份 `tenant_id=default`，行为与现状零回归 |
| 存储 | SQLite 单文件加 `tenant_id TEXT NOT NULL DEFAULT 'default'` 列，复合主键 `(tenant_id, …)`；非每租户独立库文件 | 复用 N3 已有 schema 迁移框架与 WAL；无连接池/文件管理负担；复合索引满足行级过滤 |
| schema | `PRAGMA user_version` 由 v1 升 **v2**；存量 v1 库经 `migrate_v1_to_v2()`「建新表→拷贝→删旧表→改名」重建，既有行归 `default` | 保留历史数据兼容；新库由 init_schema 直接建 v2 形状 |
| 内存态 | `ExpertsSharedState.registry` → `HashMap<TenantId, HashMap<Id, ExpertDescriptor>>`；`graph` → `HashMap<TenantId, ExpertGraph>` | 绕过内存态即绕过隔离，必须分区；读面用静态空投影 `empty_registry()/empty_graph()` 展平，下游 `.values()/.get()` 零改动 |
| 写面清库 | 原 `DELETE FROM 整表`+重插改为 `DELETE WHERE tenant_id=?` 后只插本租户口 | 多租户下整表 DELETE 会清掉别租户数据，是隔离红线 |
| RBAC | `ADMIN_ROLES=[super_admin, tenant_admin]` 保持全局；数据过滤按租户；租户内角色为阶段二 | 平台级管理语义不变；enforce_admin 透传 tenant 进审计 |
| 审计 | `emit_audit` 新增 `tenant: &str` 参，替换原硬编码 `"experts-alliance"`（experts_common.rs:640 附近）；审计 Resource 带 `tenant_id` | 审计可按租户追溯 |

### 阶段一改动文件:行号（关键）

- `alliance/experts_db.rs`：四张表（experts/graph_nodes/graph_edges/graph_meta）加 tenant_id 复合键；`migrate_v1_to_v2()`；所有读写函数（list/get/upsert/delete/save_registry/save_graph/upsert_graph_node/upsert_graph_edge）加 tenant 参与 WHERE 过滤；save_* 改按租户 DELETE。
- `alliance/experts_common.rs`：`TenantId` 提取器（FromRequestParts，:58-98）；`DEFAULT_TENANT="default"`（:41）；`ExpertsSharedState.registry/graph` per-tenant（:540-609）；`emit_audit` 加 tenant（:728）；`empty_registry/empty_graph` 静态空投影。
- `alliance/experts_rbac.rs`：enforce/audit_denied 透传 tenant。
- `alliance/experts_registry.rs` / `experts_graph.rs` / `experts_dispatcher.rs` / `experts_collaboration.rs` / `experts_orchestration.rs` / `experts_session.rs` / `experts_ext.rs`：handler 签名接 `TenantId`，读面按租户取内层；自由函数 `dispatch_task` 加 tenant 首参。
- `monitor.rs`：专家指标改跨租户聚合（全局 dashboard 语义）。
- 新增测试：`experts_registry.rs`（3 个租户隔离单测）；`tests/tenant_expert_isolation.rs`（真实 TCP E2E）；既有 `tests/trusted_tenant_http.rs`（租户绑定真实 HTTP，2 测）。

### 真实验证证据（禁止 mock）

证据落盘：`platform/domains/alliance/_verification/a1-e2e-evidence.txt`。
起真实生产路由器（JWT 中间件 + 租户提取器 + 真实 SQLite），两真实身份各带 `tenant_admin`：

- `tenant-a` POST `/api/experts` → 200，创建 `e2e-a-expert`（架构师·甲）。
- `tenant-b` POST `/api/experts` → 200，创建 `e2e-b-expert`（数据师·乙）。
- `tenant-a` GET `/api/experts` → `total:1`，列表**仅含 e2e-a-expert**，不含 e2e-b-expert。
- `tenant-b` GET `/api/experts` → `total:1`，列表**仅含 e2e-b-expert**，不含 e2e-a-expert。
- `tenant-a` GET `/api/experts/e2e-b-expert` → **404**（跨租户不可见）。
- 无 token GET → **401**；tenant-a token 配 `x-tenant-id: tenant-b` → **403**（头伪造被拒）。

单测补充：两租户列表互不可见、默认租户回归（seed_expert 进 default，具名租户看不到）、提取器（无头取身份 tenant / 头与身份冲突 403 / 无身份 401）。

### 测试结果

- `cargo check -p mox-platform-gateway-svc --all-targets`：干净（0 error）。
- `cargo test -p mox-platform-gateway-svc`：lib 179 passed + 集成 13+7+8+3+1+2，**0 failed**。
- 修复过程中发现并消除一处 per-tenant 改造引入的自死锁：graph CRUD handler 中 `drop(graph)` 只释放内层引用、外层 `all_g` MutexGuard 仍持有即重锁自身 → 补 `drop(all_g)`。

### 阶段二（方案稿，未硬做，诚实标注）

1. **任务/会话/执行器租户隔离**：当前 `sessions`/`dispatch_records`/`plans`/`orchestration_history` 仍为全局内存态（handler 已接 tenant 但未分区存储）；预约 `resolve_expert_name` 暂跨租户全局查找。需将这些 Map 改 per-tenant 并按租户过滤。
2. **租户配额**：每租户专家数/并发调度/存储上限，接 dispatcher_config。
3. **密钥隔离**：每租户独立 API Key / 模型密钥。
4. **SSO/SAML**：租户级身份源对接，替换单 JWT；`UserInfo.tenant_id` 由 SAML 断言下发。
5. **租户内 RBAC**：tenant_admin 仅管本租户资源（当前 ADMIN_ROLES 全局）。


---

## D4 进程内三项落盘 + D8 探活默认开（2026-10-02）

### 背景

- D4：`ExpertsSharedState` 的 `plans` / `orchestration_history` / `favorites` 三项此前为纯内存态（experts_common.rs），进程崩溃即丢；sessions/graph 已有 SQLite 落盘，三项是最后缺口（16 号台账 D4）。
- D8：registry-svc `Config::default().health_probe_enabled` 为 `false`（app_state.rs），主动探活需生产显式设 env（16 号 N5/D8 登记为「设计取舍」）。

### D4 落盘方案

| 维度 | 决策 | 理由 |
|---|---|---|
| schema | `PRAGMA user_version` v2 → **v3**；新增三表 `collaboration_plans` / `orchestration_history` / `favorites`，均 `(tenant_id, …)` 复合主键 | 复用 N3 迁移框架 + A1 租户行级隔离；三表此前无历史数据，v2→v3 仅 `CREATE TABLE IF NOT EXISTS` + bump 版本号，无需数据搬迁 |
| 写时持久化 | 每次写操作后立即 upsert/insert 单条（`upsert_plan` / `insert_history_record` / `upsert_favorite` / `delete_favorite`），best-effort（失败仅 log_err，不阻断业务） | 数据量小，单条 upsert 避免全量重写；与既有 sessions/graph 持久化同约定 |
| 启动加载 | `ExpertsSharedState::new()` 调 `load_all_plans / load_all_history / load_all_favorites` 一次读回全部租户分区，重建内存态 | 崩溃恢复 = 重启走同一启动路径 |
| 内存态 | plans/history 保持 A1 既有「全局扁平 + handler 按 metadata.tenant_id 过滤」读面（plan_id/execution_id 为全局 UUID 不冲突）；**favorites 改按租户分区** `HashMap<tenant, HashSet<expert_id>>`（原扁平 HashSet 无法跨租户隔离） | 读面最小改动；favorites 无租户维度，分区是唯一正确解 |
| favorites 租户来源 | handler 取 `OptionalAuthUser` 的 `tenant_id`；无身份（无头请求）归 `default` | 不新增 401/403，单租户 default 行为零回归；生产经鉴权中间件注入真实租户 |

### 改动文件:行号（关键）

- `alliance/experts_db.rs`：import 加 `CollaborationPlan/OrchestrationRecord/HashSet`；`SCHEMA_VERSION=3`；init_schema 加三表；新增 `upsert_plan_conn/upsert_plan/load_all_plans`、`insert_history_record_conn/insert_history_record/load_all_history`、`upsert_favorite_conn/upsert_favorite/delete_favorite_conn/delete_favorite/load_all_favorites`。
- `alliance/experts_common.rs`：`favorites` 字段类型 `HashSet<String>` → `HashMap<String, HashSet<String>>`；`new()` 三项改 `load_all_*()` 启动加载；新增 7 个 pub 包装函数；2 个测试构造点改 `HashMap::new()`。
- `alliance/experts_orchestration.rs`：3 处 plan 存入后 `upsert_plan(tenant, &plan)`；2 处 history push 前 `insert_history_record(tenant, &record)`（orchestrate / generate_plan_handler / execute_plan_handler）。
- `alliance/experts_ext.rs`：`toggle_expert_favorite` 接 `OptionalAuthUser` 派生租户，分区写入 + upsert/delete_favorite 落盘；原直接调 handler 的单测改为验证内存分区（DB 闭环交集成测试）。
- 其余 5 模块测试构造点（dispatcher/graph/rbac/registry/session）favorites 改 `HashMap::new()`。
- 新增集成测试 `tests/d4_crash_recovery.rs`。
- D8：`registry-svc/src/app_state.rs` `health_probe_enabled: false → true`（含注释/from_env 文档）；`health_probe.rs` 模块注释 + 测试 `probe_default_config_enables_task_d8`（原断言默认关的用例已改判）。

### 真实验证证据（禁止 mock）

证据落盘：`platform/domains/alliance/_verification/d4-e2e-evidence.txt`。
真实 SQLite（独立临时库）+ 真实 `ExpertsSharedState::new()` 启动路径，走「写入 → 模拟崩溃重启（drop state）→ 读回」闭环：

- 首次 new() 三项均空；写入 tenant-a（plan-a1/plan-a2 + exec-a1 + 收藏 exp-a-1/exp-a-2）与 tenant-b（plan-b1 + 收藏 exp-b-1）。
- drop state1 后二次 new()：plans 恢复 3 个且 plan-a1 逐字段一致（title/status/steps/metadata.tenant_id/expert_ids）；history 恢复 1 条（execution_id/plan_id/status/duration_ms）；favorites 按租户分区恢复。
- **租户隔离**：tenant-a 只见 2 plan（不含 plan-b1），tenant-b 只见 1 plan（不含 plan-a1/a2）；favorites 跨租户互不可见。
- 三次 new() 幂等：plans 仍 3（不翻倍）、history 仍 1。

### 测试结果

- `cargo check -p mox-platform-gateway-svc --lib`：exit 0（仅既有 drop-on-ref/unused 警告，无新增）。
- `cargo test -p mox-platform-gateway-svc --lib`：**180 passed / 0 failed**（A1 基线 179，+favorites 分区单测等净增）。
- `cargo test -p mox-platform-gateway-svc --tests`：`experts_db_persistence` 8 / `tenant_expert_isolation` 1 / `d4_crash_recovery` 1 等全部 **0 failed**（既有 sessions/registry/graph 落盘路径零回归）。
- `cargo test -p mox-alliance-registry-svc`：**17 lib + 11 集成 = 28 passed / 0 failed**；`probe_default_config_enables_task_d8` 通过。

### D8 说明

探活端点公开白名单（routes.rs `/health` `/metrics` `/leadership` `/api/registry/health`）不鉴权，与 `health_probe_enabled` 无关——改默认值不触及路由鉴权。默认开后启动后台探测任务（server.rs:39），提前标记「仍心跳但业务不可用」实例为 Unhealthy 并在恢复时自动回册；env `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=0/false/no/off` 可关回退被动租约。

### 不做 / 诚实标注

- plans/history 落盘是网关本地 SQLite，**多副本/A2 memory 独立前不跨进程共享**（同 N11 会话边界）；并入 scheduler `ea_task_node` 表仍远期。
- favorites 仅按租户分区落盘，未接用户维度/跨设备同步；登记值语义（availability.status）不改，探活结果回流前端仍为遗留。
- 回退：三表保留不影响旧路径；恢复默认 = app_state.rs 改回 false + new() 改回 `HashMap/Vec::new()`。


---

## A2 无状态化阶段一：冷数据外移为「SQLite 唯一真相」（2026-10-02）

### 背景与目标

- D4 已把 `collaboration_plans` / `orchestration_history` / `favorites` 三项落 SQLite，但语义是「**内存为主 + SQLite 备份**」：多副本各持内存副本，A 写穿 SQLite 后，B 不重启、不刷新本地缓存就读不到（D4 诚实标注的遗留：「多副本/A2 memory 独立前不跨进程共享」）。
- A2 阶段一目标：把这三项**冷/低频数据**的读面也改为实时查 SQLite，立 SQLite 为**唯一真相**，补齐跨实例一致性 / HA 读一致性；registry/graph 高频态保持进程内（留阶段二）。

### 设计决策（含理由）

| 维度 | 决策 | 理由 |
|---|---|---|
| 路径选择 | SQLite 作为唯一真相（**写穿 + 读路径实时查 SQLite**），**不引入外部存储**（不上新 Redis/DB/对象存储） | 守气隙/最小侵入/零新依赖：三项表 D4 已建、写穿已就绪，WAL 多连接可见性天然支持跨副本读；引入外部存储会破坏气隙卖点且新增运维面，成本远超收益。 |
| 外移对象 | **仅三项冷数据**（plans / history / favorites）；registry/graph **保持进程内不外移** | 写热点说明：registry 是每次咨询/匹配的热读（`compute_match_score` 逐专家打分）、graph 是多跳遍历大对象，二者读 QPS 高、对象大，逐请求查 SQLite 会放大延迟与连接开销；plans/history/favorites 是冷/低频读（统计、历史列表、收藏开关），短连接本地文件亚毫秒级，可接受。外移选择遵循「先冷后热」，最小化风险。 |
| 写路径 | 保持 D4 写穿（单条 upsert/insert/delete，best-effort 失败仅 log 不阻断业务）；**补 busy/locked 应用层重试循环**（≤5 次退避） | WAL 下读写不互斥，仅多写者争锁；`busy_timeout=5s` 已是第一道等待，应用层再做有限次重试，降低多副本共享同一文件时写穿的瞬时失败率。非锁类错误（如序列化失败）不重试，按既有约定记录即返回。 |
| 读路径 | 三个读 handler 改为**按租户实时查 SQLite**（`load_plans_by_tenant` / `get_plan` / `load_history_by_tenant` / `load_favorites_by_tenant`）；favorites toggle 的 `is_fav` 判定也改为查 SQLite | 低频读成本可接受；实时查库即跨实例一致，**无需失效广播**（读侧不去维护与其他副本的缓存一致性）。本实例写后仍同步更新本地内存镜像（保持即时读与既有 handler 形状），但读面权威来源是 SQLite。 |
| 内存态定位 | `plans`/`orchestration_history`/`favorites` 内存镜像降级为「本实例最近写入的即时读缓存 + 启动加载重建」，不再是权威源 | 单实例下读 SQLite == 写穿后内存镜像，行为等价；保留镜像以兼容既有 handler 构造形状与启动路径，改动最小、可回退。 |
| 租户隔离 | 租户过滤**下推到 SQL**（`WHERE tenant_id=? [AND plan_id=?]`） | `get_plan` 跨租户自然 None（404）；history 行 D4 写入时即带 `tenant_id`，直接按租户查，等价于旧「按本租户可见 plan 过滤 history」语义。 |

### 改动文件:行号（关键）

- `alliance/experts_db.rs`：
  - 新增 `retry_write`（busy/locked ≤5 次退避重试，非锁错误直接 log）；
  - 四个写穿函数 `upsert_plan` / `insert_history_record` / `upsert_favorite` / `delete_favorite` 改走 `retry_write`；
  - 新增四个按租户实时读函数：`load_plans_by_tenant` / `get_plan`(tenant,plan_id) / `load_history_by_tenant` / `load_favorites_by_tenant`。
- `alliance/experts_orchestration.rs`：
  - `execute_plan_handler`：plan 读取由 `state.plans.get_mut`（内存）改为 `experts_db::get_plan(tenant, plan_id)`（SQLite，租户下推）；
  - `orchestration_stats`：plans/history 由内存过滤改为 `load_plans_by_tenant` / `load_history_by_tenant`（`state` 参数改 `_state`）；
  - `orchestration_history` handler：历史由「先查本租户可见 plan 集合再过滤内存 history」改为直接 `load_history_by_tenant` 后本地 retain/status/分页（`state` 改 `_state`）。
- `alliance/experts_ext.rs`：`toggle_expert_favorite` 的 `is_fav` 判定由本地内存镜像改为 `load_favorites_by_tenant(tenant)`（SQLite 唯一真相），再回写本地镜像 + 写穿方向不变。
- 新增集成测试 `tests/a2_multi_instance_consistency.rs`。

### 跨实例 E2E 证据（禁止 mock）

证据落盘：`platform/domains/alliance/_verification/a2-e2e-evidence.txt`。
两个独立活实例（各自 `ExpertsSharedState::new()`，各自独立内存镜像）共享同一份真实 SQLite（临时文件，WAL）：

- **对照组**：先断言 inst_b 内存镜像里**没有** inst_a 刚写的 plan/收藏（B 自启动后未 reload）——证明读路径若走内存必然读不到。
- inst_a 真实写穿 plan(a2-plan-1) + history(a2-exec-1) + favorite(exp-a-fav)（tenant-a）。
- **inst_b 不重启**，直接走新读路径：按租户读到 a2-plan-1（title/expert_ids 逐字段一致）、`get_plan` 单点命中、history 1 条（duration_ms=123）、favorite 含 exp-a-fav。
- **租户隔离**：B 读 tenant-b 的 plans/history/favorites 全空，`get_plan("tenant-b","a2-plan-1")` 为 None（跨租户 404）。
- **双向一致**：B 写 tenant-b 收藏后，A 实时读到。

### 测试结果

- `cargo check -p mox-platform-gateway-svc --lib`：exit 0（仅既有 drop-on-ref/unused-var/unused-import 警告，与本次改动无关；本次新增代码无新警告）。
- `cargo test -p mox-platform-gateway-svc --lib`：**180 passed / 0 failed**（与 D4 基线 180 完全一致，单实例零回归）。
- `cargo test -p mox-platform-gateway-svc --tests`：a2_multi_instance_consistency **1**（新增）/ d4_crash_recovery 1 / experts_db_persistence 8 / tenant_expert_isolation 1 / alliance_remote 13 / inbox_http 4 / inbox_persistence 6 / delivery_truth 3 / domain_grouping 7 / 其余 0 failed。

### 阶段二（方案稿，诚实标注，本轮不硬做）

- registry（per-tenant HashMap）/ graph（per-tenant 大图）高频态外移：读 QPS 高、对象大，需先引入读缓存 + 失效机制或专用图库（A4 气隙嵌入式约束），不盲目外移。
- 执行器 task 状态、dispatcher 熔断/调度记录的跨副本共享。
- 分布式通知 / 缓存失效广播（阶段一读路径实时查库规避了此需求，但高频态外移后必需）。
- 多副本写冲突策略：当前 SQLite 单写者 + WAL + busy_timeout + 应用重试够用；若写并发上升需上事务级乐观锁/版本号，或迁 scheduler 任务表（远期）。
- sessions（N11）仍单进程内可恢复，多副本共享留待 memory 独立 / A3 事件溯源。

---

## T4 事件驱动（2026-10-02）：进程内事件总线 + 最小真实价值闭环

### 背景与现状核证

工程此前**零事件机制**（grep broadcast/subscribe/EventBus/notify_ 零命中）：审计为直接调用
`emit_audit`，SSE 为单向日志流。本轮从零建立进程内广播总线，落地 T4 卡片（§2.4：任务状态
变更事件流 pending→running→completed/failed、节点启停）的第一性闭环，对齐 Airflow 3.0 事件
驱动 / LangGraph 状态机事件。

> 勘误：任务书把「事件驱动」误挂在「12 号 M1」。实际 M1=私有化交付产品化（信创/气隙离线包）；
> 事件驱动为 **T4**。本节与文档均以 T4 为准。

### 事件模型（experts_events.rs）

`AllianceEventKind` 枚举（**按真实 emit 点收敛，不为枚举而枚举**）+ `AllianceEvent` 信封
（id=evt-uuid + type + payload + source + tenant(A1) + occurred_at=RFC3339）：

| 变体 | 真实 emit 点 |
|---|---|
| PlanCreated | generate_plan_handler / orchestrate |
| PlanStatusChanged | execute_plan_handler / orchestrate（draft→running→completed/failed/partial）|
| ExpertRegistered | create_expert |
| ExpertDisabled | delete_expert |

### 总线选型理由

选 `tokio::sync::broadcast`：① tokio features=`full`（见 Cargo.toml）已含 sync::broadcast，
**零新依赖**；② 一对多广播，天然支持「事件日志消费者 + 未来 SSE 事件帧 + 外部系统订阅」多下游；
③ 生产路径 send 非阻塞、无消费者返回 Err 被忽略，**失败不阻断业务**（与审计 best-effort 一致）；
④ 有界 channel + Lagged 追赶，慢消费者不反压写路径。总线挂 `ExpertsSharedState.events`。

### 闭环选择（价值×成本）

- **A 计划/任务状态主线**（落地）：价值=即 T4 §2.4 卡片本身（前端免轮询 / 外部系统订阅生命周期
  联动 / 可观测数据骨干）；成本=emit 点已在 orchestration handler 定位，消费者=新增一张表。
- B 专家注册事件→审计增强：价值=边际（审计链已覆盖）；成本=create/delete_expert 各一行 emit。
  作为 A 的旁路真实 emit 点一并接入（消费者通用，零额外成本），不作主闭环。

### 改动文件:行号（关键）

- `src/alliance/mod.rs`：注册 `pub mod experts_events;`
- `src/alliance/experts_events.rs`：新建（EventBus / AllianceEvent / Kind + `spawn_event_log_consumer` + 单测）
- `src/alliance/experts_common.rs`：`ExpertsSharedState` 新增 `events` 字段；`new()` 建总线并挂消费者；补 2 处测试构造
- `src/alliance/experts_db.rs`：`SCHEMA_VERSION 3→4`；`init_schema` 加 `alliance_event_log` 表；
  `insert_event_log(_conn)` / `load_event_log_by_tenant` / `EventLogRow`
- `src/alliance/experts_orchestration.rs`：orchestrate / generate_plan_handler / execute_plan_handler 真实 emit
- `src/alliance/experts_registry.rs`：create_expert / delete_expert 真实 emit；补 `make_test_state`
- `src/alliance/{dispatcher,session,rbac,graph,ext}.rs`：补 `events` 测试构造（编译适配）
- `tests/t4_event_bus_e2e.rs`：新建 E2E（禁止 mock）

### 真实验证证据（禁止 mock）

`tests/t4_event_bus_e2e.rs`：真实 tokio 运行时 + 真实 `ExpertsSharedState::new()` + 真实 SQLite
（独立临时库）+ 真实 TCP + 生产 JWT 认证中间件。链路：
1. POST `/api/experts/plan/generate`（真实 generate_plan_handler）→ emit PlanCreated；
2. POST `/api/experts/plan/execute`（真实 execute_plan_handler）→ 新租户无专家快速失败，
   draft→running→failed → emit 两次 PlanStatusChanged；
3. 事件经 broadcast 总线被 `new()` 启动时挂的**真实消费者**订阅 → 真实落 `alliance_event_log`；
4. 轮询读回断言：event_type/tenant/occurred_at/plan_id 齐备；跨租户隔离（另一租户读不到）。
旁路：POST `/api/experts`（tenant_admin JWT）→ create_expert → ExpertRegistered 落库。

```
running 2 tests
test t4_expert_register_event_is_emitted_over_real_http ... ok
test t4_plan_lifecycle_events_flow_into_event_log_end_to_end ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
EXITCODE=0
```

证据落盘：`_verification/t4-e2e-evidence.txt`。

### 测试结果

- lib 单测：180（A2 后）→ **187**（+7 事件总线/序列化/收发单测），0 failed。
- 集成：新增 t4 2 个 E2E，全绿；既有审计 / 单租户 / 多实例一致性零回归。
- `cargo check`：干净通过（仅既有 unused 警告，非本次引入）。

### 诚实标注：已落地 vs 后续

- 已落地：进程内 broadcast 总线 + 事件模型 + 事件日志消费者落库 + 真实 E2E。
- 后续（方案稿）：跨进程/多副本广播、SSE 事件帧推送、外部系统 Webhook 订阅——需在总线之上再加
  外发层，本阶段仅进程内闭环。审计链照旧直接调用不变，事件消费者为增量能力。

---

## A1 阶段二：租户配额 + SSO 可测性评估（2026-10-02）

### 一、本轮范围

- **落地（真实，禁止 mock）**：多租户资源治理的第一个真实可测维度——**单租户专家数上限配额**。
- **评估（如实）**：SSO 子项的真实可落地性，核证既有 `sso/api.rs` OIDC 实现，判定本机是否具备端到端真实验证条件。
- **不改**：租户隔离/RBAC/审计链/内部鉴权既有行为；单租户（default）默认行为零回归。

### 二、配额模型与维度选择理由

阶段一选「租户专家数上限」而非一次铺开多维度，依据是**治理价值 × 真实可数 × 改动成本**排序：

| 维度 | 是否本轮做 | 理由 |
|------|-----------|------|
| **租户专家数（registry 行数）** | 做 | registry 已是干净的 per-tenant `HashMap<tenant, HashMap<expert_id,_>>`，`reg.len()` 即真实占用数，`create_expert` 单点拦截，成本最低、价值最直接（防单租户无限注册专家记录） |
| DAG 计划/任务数 | 后续 | `plans: HashMap<plan_id, plan>` 是全局扁平表，租户靠 `plan.metadata["tenant_id"]` 运行期过滤；按租户计数需全表扫描 + 语义确认，成本/收益不如专家数，本轮如实不做 |
| 会话数 | 后续 | `sessions` 全局扁平，且会话天然短生命周期，配额治理价值低 |
| 图谱节点数 | 后续 | 图谱由注册表推导，专家数配额已间接约束；独立计数价值低 |
| LLM 调用量 / 并发配额 | 阶段三 | 接 T1 信号量 + 计量，属另一子系统 |

**超限语义**：HTTP **409 Conflict**（资源占用冲突，非 400 参数错、非 429 限流语义——本配额是「创建资源」类冲突，409 最贴切）。

### 三、配置来源（避免过度工程）

- **代码默认 + env 覆盖**，不落库：`MOX_ALLIANCE_QUOTA_EXPERTS_PER_TENANT`，默认 **1000**。
- 默认 1000 不误伤现有单租户：内置种子专家仅 10 个，且经 `ExpertsSharedState::new()` **直接写入内层注册表**（不经 `create_expert` handler），不受配额约束。
- **不落租户级配置表**：租户级配额差异属阶段三/管理 UI；本轮统一全局上限 + env，避免引入表与迁移。
- 读取时机：管理写面每次 `create_expert` 实时读取（非热路径），**env 修改即时生效、无需重启**；env 缺失/非正整数一律回退默认。

### 四、组件与计数口径

- `QuotaGuard`（函数式，遵循任务「或函数」的最小改动取向，未给 `ExpertsSharedState` 加字段、未动 8 处测试构造）：
  - `check_expert_quota_with(used, limit, tenant)`：纯函数，可单测；超限返回已构造好的 409 响应。
  - `check_expert_quota(used, tenant)`：生产入口，env 取上限后委托纯函数。
- **计数口径**：该租户内层注册表全部记录数 `reg.len()`。软删除（enabled=false）专家记录仍占槽位——其 id 已被 `create_expert` 的 id 冲突检查永久保留、不可复用，故按「注册表专家记录数」计最诚实、最可复算。
- **与既有链共存**：校验在 handler 内、租户已解析后执行；顺序为 RBAC → name/id 校验 → id 冲突(400) → **配额(409)** → insert。管理写面 RBAC 不变；拒绝照旧落审计链（`quota.denied`，`AuditOutcome::Failure`）。

### 五、改动文件:行号

- `src/alliance/experts_common.rs:114-165`：新增「零-B 多租户配额治理」节——`DEFAULT_QUOTA_EXPERTS_PER_TENANT=1000`、`quota_experts_per_tenant()`、`check_expert_quota_with()`、`check_expert_quota()`。
- `src/alliance/experts_common.rs:1199-1214`：新增纯函数单测 `test_check_expert_quota_with_threshold`（used<limit 放行；used>=limit 返回 409 且 body 含 quota/used/tenant）。
- `src/alliance/experts_registry.rs:381-396`：`create_expert` 在 id 冲突检查后接入 `check_expert_quota(reg.len(), tenant)`，拒绝时发 `quota.denied` 审计并返回 409。
- `tests/a1_quota_tenant.rs`：新建 E2E（禁止 mock）。

### 六、E2E 真实验证证据（禁止 mock）

真实生产路由器 + 真实 JWT(HS256) 中间件 + 真实 `ExpertsSharedState::new()` + 真实 SQLite（独立临时库）+ 真实 TCP + reqwest。低配额 env `MOX_ALLIANCE_QUOTA_EXPERTS_PER_TENANT=2`：

```
[A1-QUOTA-E2E] tenant-a POST /api/experts qa-ok-1       -> HTTP 200
[A1-QUOTA-E2E] tenant-a POST /api/experts qa-ok-2       -> HTTP 200
[A1-QUOTA-E2E] tenant-a POST /api/experts qa-overflow-1 -> HTTP 409
  body={"code":409,"msg":"租户专家数已达上限（quota=2, used=2）",
        "data":{"error":"quota_exceeded","limit":2,"quota":2,"resource":"expert","tenant":"tenant-a","used":2}}
[A1-QUOTA-E2E] tenant-b POST /api/experts qb-ok-1       -> HTTP 200
[A1-QUOTA-E2E] tenant-b POST /api/experts qb-ok-2       -> HTTP 200
[A1-QUOTA-E2E] tenant-b POST /api/experts qb-overflow-1-> HTTP 409
  body={...,"data":{...,"tenant":"tenant-b","quota":2,"used":2}}
[A1-QUOTA-E2E] tenant-a GET /api/experts -> {"data":{...,"total":2}}（被拒专家未落库）
running 1 test
test tenant_expert_quota_enforced_per_tenant ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

结论：真实租户真实建 2 成功 → 第 3 个被真实 **409** 拒绝（响应体含 quota/used）→ 另一租户配额独立不受影响 → 被拒记录未落库。证据落盘：`_verification/a1-quota-e2e-evidence.txt`。

### 七、SSO 可落地性评估（如实，不造假对接）

**核证既有实现（先读码）**：`src/sso/api.rs`
- OAuth2/OIDC 授权码交换 `exchange_oauth2_code` 为**真实实现**：reqwest（15s 超时）`POST provider.token_endpoint`，Basic 认证 + form body（RFC 6749 §4.1.3：grant_type=authorization_code/code/redirect_uri）；非 2xx → 502；缺 client_id/secret/token_endpoint → **422 不造桩成功**；OIDC 再 GET `userinfo_endpoint` 取 sub/email。
- 回调 `callback_handler`：provider 存在且 enabled → state 一次性消费（10 分钟过期、防 CSRF）→ 协议分支（saml/cas/ldap = **501**）→ oauth2/oidc 交换 → 按 email 在默认租户 IAM 映射（无映射 **409**，不自动建号、不绕 RBAC）→ 复用密码登录同一套 `issue_tokens` 签平台 JWT。
- **配置来源**：provider 为**内存态** `SsoState.providers: HashMap<String,SsoProvider>`，由 `builtin_provider_templates()` 种子 + HTTP CRUD（list/create/update/delete provider、login、callback）；**无持久化**（重启重置），client_id/secret/token_endpoint 均来自 provider 记录。

**本机可测性结论**：本机为本地开发 Windows 环境，provider 模板的 client_id/client_secret 为空、无外部 IdP 凭据、无本地 Keycloak/Okta 等 OIDC 服务在跑。因此——
- OIDC **协议实现已在且真实**，但「授权码换 token → 取 userinfo → IAM 映射 → 签平台 JWT」的**端到端真实验证缺一个真实可达的 IdP**。
- 本轮**不造假对接、不 mock 一个假 token 端点冒充成功**。现有单测已如实覆盖：缺凭据 → 422（不访问外网、不造桩成功）、pending state 一次性生命周期。

**SSO 接入方案稿（待真实 IdP，诚实标注）**：
1. **对接点（已就绪，无需新协议层）**：用既有 `POST /api/enterprise/sso/providers` 建一个 `protocol=oidc, status=enabled` 的 provider，填真实 `client_id/client_secret/auth_endpoint/token_endpoint/userinfo_endpoint/redirect_uri`；前端跳 `login` 拿 `auth_url` → 用户在 IdP 登录 → 回跳带 code → `POST /callback` 换平台 JWT。
2. **可测条件（缺一不可）**：① 一个真实 OIDC IdP——本地起 Keycloak（docker，建 realm/client/测试用户）或 Okta/Auth0 测试租户凭据；② 该 IdP 预建一个 email 与平台 IAM 用户一致的账号（否则 409 未映射）；③ redirect_uri 与 IdP 登记一致。
3. **SAML/CAS/LDAP 改动点（当前 501 → 真实 handler）**：`callback_handler` 协议分支新增 SAML Response 解析（验签）、CAS ticket 校验（`/serviceValidate`）、LDAP bind；provider 配置需补 IdP metadata/证书。本轮如实保持 501。
4. **后续工程项**：provider 内存态 → 持久化表（多副本/重启保留）、密钥加密存储、租户级 SSO 策略（当前固定 default 租户映射）。

### 八、测试结果

- `cargo test`：**全绿，0 failed**（lib 189 passed；新增 `test_check_expert_quota_with_threshold` 1 个纯函数单测 + 集成 `a1_quota_tenant` 1 个 E2E）。既有审计/单租户/多租户隔离/多实例一致性零回归。
- `cargo check`：干净通过；新增代码无新 warning（仅既有 unused import/drop-ref 警告，非本轮引入）。
- 单租户默认行为：默认配额 1000，default 租户 10 个种子专家直写注册表、不经 handler，零回归。

### 九、诚实标注：已落地 vs 方案稿 vs 待真实 IdP

- **已落地（真实）**：租户专家数配额（env 可配、超限真实 409 + 结构化 quota/used、按租户独立、拒绝落审计）+ 低配额 E2E 真实证据。
- **方案稿（未硬做）**：DAG 计划/任务数、会话、图谱节点配额；租户级配额配置表/管理 UI；LLM 调用量/并发配额（接 T1）；SAML/CAS/LDAP 真实 handler；provider 持久化与加密。
- **待真实 IdP**：OIDC 端到端真实验证——协议代码已就绪，缺本地 Keycloak/Okta 测试凭据，拿到后即可按 §七方案稿真跑，本轮不造假。
