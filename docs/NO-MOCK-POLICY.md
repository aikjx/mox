# 禁止使用 Mock 规范（NO-MOCK-POLICY）

> 版本：V1.0  
> 日期：2026-09-03  
> 适用范围：全仓库所有生产代码（后端 Rust + 前端 Vue3）

---

## 1. 核心原则

**本项目所有生产 API 接口、前端接口、基础设施层，禁止使用任何形式的 mock / stub / fake / 硬编码数据。**

所有接口必须返回真实业务数据或真实错误状态。降级策略只允许：显示空状态 + 错误提示 + 重试机制，**不允许返回假数据冒充真实数据**。

---

## 2. 禁止清单

### 2.1 后端禁止项

| 禁止类型 | 示例 | 替代方案 |
|---------|------|---------|
| 硬编码 JSON 响应 | `Json(json!({"ok": true, "data": [...]}))` | 调用真实业务逻辑/数据库/算法 |
| Stub 路由桩 | 返回 `"stub": true` 的端点 | 桥接真实领域服务或开发真实功能 |
| Fake 内存存储 | `HashMap` 冒充 Redis/S3 | 使用真实 Redis 客户端 / S3 HTTP 客户端 |
| Demo 数据生成 | `vec![demo_item_1, demo_item_2]` | 从真实数据源读取 |
| Mock 依赖库 | `mockall` / `mockito` / `wiremock` / `httpmock` | 生产依赖中禁止出现（测试依赖除外） |

### 2.2 前端禁止项

| 禁止类型 | 示例 | 替代方案 |
|---------|------|---------|
| Mock 服务工作者 | `msw` / `mockjs` / `axios-mock-adapter` | 调用真实后端 API |
| 硬编码数据文件 | `mockData.js` / `mockData.ts` | 删除，改为 API 调用 |
| Demo 登录模式 | `LOGIN_MODES.DEMO` 生成假 token | 默认 JWT 真实登录 |
| Mock 兜底函数 | `_getMockPermissions()` / `MOCK_KPI` | API 失败时显示空状态+错误提示 |
| `Promise.resolve(硬编码)` | `return Promise.resolve([...])` | `return http.get('/api/...')` |

### 2.3 测试中的 Mock

- **单元测试**：允许使用 mock 框架隔离外部依赖，但必须在代码注释中说明理由
- **集成测试**：优先使用真实服务/内存数据库，禁止用 mock 替代核心业务逻辑
- **E2E 测试**：必须连接真实后端，禁止使用 mock 服务

---

## 3. 已移除的 Mock 清单（历史记录）

### 3.1 后端（已移除）

| 位置 | 原 Mock 类型 | 替换方案 | 完成日期 |
|------|-------------|---------|---------|
| `gateway/.../alliance.rs` | 13 个端点 stub（`"stub": true`） | 真实 InMemoryTaskRepository + RuleBasedExpertMatcher + 进程内执行状态机 | 2026-09-03 |
| `gateway/.../routes.rs` | KG 6 stub + AI 4 stub + 25 域 stub_handler（死代码） | 移除死代码，真实路由在 lib.rs 装配 | 2026-09-03 |
| `ai/expert-svc/server.rs:708` | `{"ok": true}` 硬编码 | 返回真实图元数据（flow_id/nodes/edges/ingested_at） | 2026-09-03 |
| `platform/orchestrator-svc/automation.rs:833` | `{"ok": true, "id": id}` 硬编码 | 返回真实资产元数据（name/updated_at/feature_count/run_count） | 2026-09-03 |
| `platform/plugin-core/host_api.rs:177` | `{"ok": true}` 硬编码 | 返回真实事件元数据（event_id/event_type/published_at） | 2026-09-03 |
| `cloud/filer-svc/meta_redis.rs` | Redis Mock 后端（fake_get/fake_expire/fake_smembers） | 真实 Redis 客户端（MultiplexedConnection + AsyncCommands） | 2026-09-03 |
| `cloud/filer-svc/filer_server.rs` | In-memory S3 mock | 真实 S3 桥接（mox-cloud-store-core::S3Client，自研 SigV4） | 2026-09-03 |
| `kg/service-svc/http_adapter.rs` | 10 个端点固定 6 节点 demo 图 | 真实算法桥接（687节点/776边 seed 图谱） | 2026-09-02 |

### 3.2 前端（已移除）

| 位置 | 原 Mock 类型 | 替换方案 | 完成日期 |
|------|-------------|---------|---------|
| `stores/auth.store.js` | Demo 登录模式（`demo-` token + 硬编码用户） | 默认 JWT 真实登录 | 2026-09-03 |
| `views/misc/Login.vue` | 演示模式 Tab/警告 | 移除，默认 JWT 登录界面 | 2026-09-03 |
| `stores/permission.store.js` | `_getMockPermissions()` mock 兜底 | API 失败设空权限+真实错误 | 2026-09-03 |
| `composables/workspace/useWorkspaceData.js` | MOCK_KPI/MOCK_MEMBERS/MOCK_PHASES/MOCK_FILES/MOCK_HISTORY | 移除，失败设空数组+error 状态 | 2026-09-03 |
| `views/workspace/mockData.js` | 独立 mock 数据文件 | 删除文件 | 2026-09-03 |
| `views/workspace/ExpertWorkspaceView.vue` | mockData import + simulateDebate | 移除，6 个 load 函数改空数组兜底 | 2026-09-03 |
| `package.json` | `msw` devDependency | 移除（未在代码中使用） | 2026-09-03 |

---

## 4. 新代码评审 Checklist

提交新代码前，必须逐项确认：

- [ ] 生产代码中无 `mock` / `stub` / `fake` / `demo` 命名的变量、函数、文件
- [ ] HTTP handler 不直接返回 `Json(json!({...}))` 硬编码数据
- [ ] 前端 API 封装不返回 `Promise.resolve(硬编码数据)`
- [ ] Cargo.toml 生产依赖中无 `mockall` / `mockito` / `wiremock` / `httpmock`
- [ ] package.json 中无 `msw` / `mockjs` / `axios-mock-adapter`
- [ ] API 失败时显示空状态 + 错误提示，不返回假数据
- [ ] 测试中使用 mock 时，代码注释说明理由

---

## 5. 降级策略规范

当后端 API 不可用时，前端只允许以下降级行为：

1. **空状态**：显示空列表/空图表 + "暂无数据" 提示
2. **错误提示**：Toast/Notification 显示真实错误信息（HTTP 状态码 + 错误消息）
3. **重试机制**：提供"重新加载"按钮，用户可手动重试
4. **加载状态**：请求中显示 Loading skeleton/spinner

**绝对禁止**：API 失败时用硬编码假数据填充页面，让用户误以为是真实数据。

---

## 6. 验证命令

```bash
# 后端：检查生产依赖中无 mock 库
grep -ri "mockall\|mockito\|wiremock\|httpmock" --include="Cargo.toml" .

# 前端：检查 package.json 无 mock 库
grep -i "msw\|mockjs\|axios-mock" frontend-ui/package.json

# 后端：检查生产代码无硬编码 JSON 响应（排除 tests/）
grep -rn "Json(json!" --include="*.rs" platform/ | grep -v "tests/"

# 前端：检查无 mockData 文件引用
grep -rn "mockData" --include="*.{js,vue,ts}" frontend-ui/src/
```

---

## 7. 例外审批

如确需在生产代码中使用 mock（如第三方服务未就绪的临时过渡），必须：

1. 在代码中添加 `// TEMP-MOCK: <原因> <预计移除日期>` 注释
2. 在项目 Issue 中创建对应任务跟踪
3. 超过 30 天未移除的，必须在架构评审会上说明理由

---

**本规范自 2026-09-03 起强制执行。**  
**违反者：代码评审驳回，不得合并。**
