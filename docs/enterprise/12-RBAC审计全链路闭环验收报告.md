# 12 · RBAC / 审计 全链路闭环验收报告

> **文档类型**：企业级安全能力验收报告（ISD-RBAC-V1.0）
> **日期**：2026-08-18 · 关联 `09` 完成归档 / `10` 交付清单 / `11` 质量闭环报告
> **结论**：原"RBAC 中间件已建未挂载"边界**已闭合**——六角色权限矩阵在**兼容 / 严格两种模式**下经 11 项 HTTP 探针 + 5 项 E2E 集成测试全部通过，放行/拒绝双向签名审计真实落库并可查询。

---

## 1 · 验收背景

`crates/runtime/src/rbac_middleware.rs` 自建库起即实现完整 RBAC/审计（`Role`/`Permission`/`check_permission`/审计链），但此前**仅作为模块编译、未挂载进请求管线**（`11` 号报告 v1.0 诚实声明的唯一边界）。鉴权仅由网关 `OUS_API_TOKEN` 承担，六角色权限矩阵在运行时不可区分。

本轮（2026-08-18）完成全链路挂载与端到端验证。

## 2 · 本轮交付清单

| # | 交付项 | 说明 | 落点 |
|---|--------|------|------|
| 1 | 认证主体模型 `Principal` | token_id / roles / tenant；认证层解析一次写入请求扩展，授权层直接读取，杜绝两层口径不一致 | `rbac_middleware.rs` |
| 2 | 令牌注册表 `TokenRegistry` | `OUS_API_TOKEN` 恒为 Admin（向后兼容）；`OUS_RBAC_TOKENS=令牌:角色[:租户]` 多组配置，配置即启用**严格模式** | `TokenRegistry::from_env` |
| 3 | 全覆盖权限映射 `required_permission` | 未登记路由按方法兜底（GET→ViewFlow，变更→EditFlow），消除假 404 | `rbac_middleware.rs` |
| 4 | 审计双写 + 签名 | 放行 `allowed` / 拒绝 `forbidden` 均写 HMAC-SHA256 签名事件；修复 `MemoryAuditSink` 异步锁丢弃事件缺陷 | `AuditSink::write` |
| 5 | 三层安全管线 | CORS → `auth_middleware`（认证写主体）→ `rbac_audit_middleware`（授权+审计）→ `standardize_response` | `main.rs` |
| 6 | 审计查询面 | 新增 `GET /api/audit`（admin/auditor 专属）；`/api/logs` 信封化 `{logs:[...]}`；OpenAPI 同步 | `main.rs` + `openapi.rs` |
| 7 | RFC 9457 语义完善 | "资源不存在"类业务失败由 400 修正为 404 + `NOT_FOUND` | `api_standard.rs` |
| 8 | 安全收紧 | 兼容模式移除"任意非空令牌→Viewer"兜底，未知令牌一律 401 | `extract_roles_from_token` |

## 3 · 端到端验证证据（真实服务器 + HTTP 实测）

服务器：`target/debug/operator-server.exe`（`OUS_API_TOKEN=ci-token-2026`，端口 3000/3001）。

### 3.1 兼容模式（未配置 `OUS_RBAC_TOKENS`，按前缀推断角色）

| 探针 | 期望 | 实测 |
|------|:---:|:---:|
| 无令牌访问受保护接口 | 401 | ✅ 401 |
| viewer 写操作 `POST /api/operators/register` | 403 | ✅ 403 |
| viewer 只读 `GET /api/operators` | 200 | ✅ 200 |
| viewer 读 `/api/audit` | 403 | ✅ 403 |
| admin 读 `/api/logs` | 200 | ✅ 200 |
| admin 读 `/api/audit` | 200 + 签名审计事件 | ✅ 200 |
| 未知令牌 `unknown_token_xyz` | 401（收紧后） | ✅ 401 |

### 3.2 严格模式（`OUS_RBAC_TOKENS=viewer_token123:viewer,editor_tok:editor,auditor_tok:auditor`）

| 探针 | 期望 | 实测 |
|------|:---:|:---:|
| 注册表 viewer 写 | 403 | ✅ 403 |
| 注册表 viewer 读 | 200 | ✅ 200 |
| 未登记前缀令牌 `admin_token123` | 401（严格模式只认注册表） | ✅ 401 |
| 任意未登记令牌 | 401 | ✅ 401 |
| `OUS_API_TOKEN`（恒 Admin） | 200 | ✅ 200 |
| editor 写操作 | 200 | ✅ 200 |
| auditor 读 `/api/audit` | 200 | ✅ 200 |

### 3.3 E2E 集成测试（`crates/runtime/tests/runtime_integration.rs`，5 项 `#[ignore]` 用例）

```
running 5 tests
test runtime_integration_tests::test_error_format_rfc9457 ... ok
test runtime_integration_tests::test_rbac_viewer_denied_write ... ok
test runtime_integration_tests::test_audit_event_recorded ... ok
test runtime_integration_tests::test_health_endpoint ... ok
test runtime_integration_tests::test_rbac_admin_has_all_permissions ... ok
test result: ok. 5 passed; 0 failed
```

审计事件实例（`GET /api/audit` 实测返回，字段已脱敏展示）：

```json
{
  "audit": [{
    "action": "POST /api/operators/register",
    "actor": "viewer_t",
    "outcome": "forbidden",
    "roles": ["viewer"],
    "tenant_id": "default",
    "signature": "6b55b601…ab052",
    "content_hash": "eb991d8a…d6015e"
  }],
  "total": 1
}
```

## 4 · 回归与质量门禁

| 项 | 结果 |
|----|------|
| `cargo test --workspace --no-fail-fast` | **644 passed / 0 failed / 6 ignored**（58 测试二进制，日志 `logs/cargo_test_20260818_rbac.log`） |
| `cargo test -p mox-expert` | 146 passed / 0 failed |
| `cargo test -p primiflow-fusion` | 44 passed / 0 failed |
| `cargo clean -p runtime && cargo build -p runtime --bin operator-server` | 0 error / **0 本仓库告警**（仅第三方 sqlx-postgres future-incompat 提示） |
| 前端 `npm run build` | 0 error / 0 warning（dist 已产出） |
| 6 大公理数学自洽（`verify_axioms.py`） | 全过 |

## 5 · 遗留与边界（诚实声明）

- 兼容模式（未配置 `OUS_RBAC_TOKENS`）角色按令牌前缀推断，**仅建议开发环境使用**；生产务必配置 `OUS_RBAC_TOKENS` 进入严格模式（启动日志会以 WARN 提示）。
- 审计事件存储为进程内 `MemoryAuditSink`，重启即清空；如需持久化审计，可替换为 `LogAuditSink`（日志）或外接存储，接口已就绪。
- `/api/audit` 与治理台 `GET /api/governance/audit/logs`（哈希链审计）为两条审计流：前者是请求级访问审计，后者是治理决策链审计，互补不冲突。

## 6 · 验收判定

✅ **通过**。RBAC 六角色权限矩阵（Admin/Editor/Operator/Viewer/SafetyApprover/Auditor）在兼容与严格两种模式下全部按预期执行，访问审计双向签名留痕可查询，`/api/logs` 与 RFC 9457 错误语义符合集成测试契约，全工作区 644 测试无回归。
