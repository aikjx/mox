# API 层规范（Enterprise Standard）

> 本目录是所有前端网络请求的**唯一出口**。新代码一律从 `@/api`（或按域模块）导入，禁止在组件/Store 中直接调用 `axios`。

## 1. 模块划分

| 文件 | 业务域 |
|---|---|
| `system.api.js` | 系统状态 / 安全凭证 / 权限 / 部门 / 岗位 / 用户 / 角色 / 菜单 / 字典 / 参数配置 / 操作日志 / 登录日志 |
| `ai.api.js` | AI 对话 / 联网搜索 / 无穷维度优化 / 制品引擎 / 全维智能分析 / 16 模块 AI 增强 |
| `experts.api.js` | 专家联盟 / 会话持久化 / 调度引擎 / 专家图谱 / V2 编排 |
| `workflow.api.js` | 工作流 / 流程图 / AI 插件 / MCP / 自动化 / 浏览器自动化 |
| `projects.api.js` | 项目 / 任务 / 资源 |
| `graph.api.js` / `kb.api.js` | 知识图谱 / 知识库 |
| `llm.api.js` / `melody.api.js` / `caomei.api.js` / `mox.api.js` / `operators.api.js` | LLM / 乐谱 / 草莓 / MOX / 算子 |
| `alliance.ts` | 专家联盟（TypeScript 实现，见 §6 已知差异） |
| `http.js` | axios 实例与拦截器（鉴权 / 重试 / 项目注入 / 错误规范化） |
| `index.js` | 统一再导出入口（向后兼容） |

- **新增域**：新建 `xxx.api.js`，在 `index.js` 增加一行 `export * from './xxx.api'`。
- **禁止**在 `index.js` 中直接定义接口；`index.js` 只做再导出。

## 2. 命名规范

| 语义 | 命名 | 示例 |
|---|---|---|
| 分页/列表 | `get<Resource>List(params)` | `getUserList`, `getRoleList` |
| 详情 | `get<Resource>Detail(id)` | `getDeptDetail` |
| 树 | `get<Resource>Tree(params)` | `getMenuTree`, `getDeptTree` |
| 全量 | `get<Resource>All()` | `getDictTypeAll` |
| 创建 / 更新 / 删除 | `create<Resource>` / `update<Resource>` / `delete<Resource>` | `createRole` |
| 变更状态 | `change<Resource>Status(id, status)` | `changeUserStatus` |
| 重置 / 清理 | `reset<Resource>` / `clean<Resource>` | `resetUserPwd`, `cleanOperLog` |
| 授权/分配 | `assign<Resource><Target>(id, data)` | `assignRoleMenuPerms` |
| 子资源读取 | `get<Resource><Sub>(id, params)` | `getDeptUserList`, `getRoleUsers` |
| 动作型（非 CRUD） | `<Verb><Object>(payload)` | `consultExpert`, `validateFlow` |
| 导出（blob） | `export<Resource>(params)` | `exportOperLog` |

**硬规则**：
- 同一资源只能有一个"列表"入口，禁止 `getRoles`（旧）与 `getRoleList`（规范）并存。
- 动词后置不统一（`listArtifacts` / `automationList`）属历史遗留，新代码一律按上表命名；如需复用旧名请显式加别名并标注 `@deprecated`。

## 3. REST 路径规范

- 资源统一**单数**路径 + 语义动作：`/system/<resource>`、`/system/<resource>/{id}`、`/system/<resource>/tree`、`/system/<resource>/{id}/<action>`。
  - ✅ `/system/role`、`/system/menu/tree`、`/system/dept/{id}/users`
  - ❌ `/system/roles`（复数，与 `/system/role` 冲突）
- 路径变量一律 `encodeURIComponent(id)`，防止注入与编码错误。
- `GET` 用 `{ params }` 传查询参数；写操作传 JSON body。
- blob 下载：`http.get(url, { params, responseType: 'blob' })`。

## 4. 请求契约（http.js 统一承担）

| 能力 | 说明 |
|---|---|
| 鉴权 | `Authorization: Bearer <token>`，401 触发全局登出事件 `mox:auth-failed` |
| 自动重试 | 网络错误 / 502-504 指数退避重试（GET/HEAD/OPTIONS） |
| 项目注入 | 自动附加当前 `project_id`（`registerProjectIdGetter` 注册） |
| 信封解包 | `{success, data}` 自动解包；失败统一抛 `Error(code + message)` |
| 错误提示 | 按状态码分类 `ElMessage` 提示 |

## 5. 向后兼容

- `index.js` 的 `export *` 要求所有导出名**全局唯一**（跨文件重名会静默歧义）。
- 改名规范：保留旧名 1 个版本周期作为 `@deprecated` 别名，再删除。
- 已删除的历史死代码：`getMenuTree`（重复声明，保留 `/system/menu/tree`）、`getRoles`（与 `getRoleList` 重复且路径冲突）。

## 6. 已知差异（暂不改造）

- `alliance.ts` 为 TypeScript，其余为 JS：当前 Vite 可正常解析，建议后续统一。
- 后端 Rust 网关（`:8080`）尚未挂接 `/system/*` 与 `/security/*` 等路由，管理面板请求会失败并回退 mock 数据；路径定义以本规范为准，待后端挂接。

## 7. 检查命令

```bash
# 全量语法校验（捕获重复声明等解析期错误）
cd frontend-ui
Get-ChildItem src/api/*.js | ForEach-Object { node --check $_.FullName }

# 跨文件重名导出审计（export * 静默歧义风险）
# 见仓库 scripts/api-duplicate-check.ps1
```
