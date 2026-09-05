f = "d:/a10/aikjx/gitcode/infotopograph/reports/markdown/网关架构模块逻辑处理流程整理分析与优化报告.md"
c = open(f, encoding="utf-8").read()
nl = "\n"
anchor = '| `src/experts_common.rs` | 删除无调用的空路由占位 `build_experts_common_router` |'
sec8_7 = '''### 8.7 全量域状态纳管（状态注册中心扩展）

装配层归一化（§8.2）只纳管了跨模块共享的专家联盟状态。本轮把其余 6 套**模块私有状态**也收口到 `ModuleStates`：

| 域状态 | 原构造方式 | 归一化后 |
|--------|-----------|----------|
| `MonitorState` | `build_monitor_router(runtime, logs)` 内部 `new` | 注册中心 `new(runtime, logs)`，`build_monitor_router(Arc<MonitorState>)` |
| `WorkspaceState` | `build_workspace_router()` 内部 `new` | `build_workspace_router(Arc<WorkspaceState>)` |
| `ProjectsState` | `build_projects_ext_router()` 内部 `new` | `build_projects_ext_router(Arc<ProjectsState>)` |
| `MiscState` | `build_misc_router()` 内部 `new` | `build_misc_router(Arc<MiscState>)` |
| `KbExtState` | `build_kb_ext_router()` 内部 `new` | `build_kb_ext_router(Arc<KbExtState>)` |
| `NotificationState` | `build_notification_router()` 内部 `new` | `build_notification_router(Arc<NotificationState>)` |

**契约变更**：`build_*_router()` 由「自包含 `Router<()>`」（内部创建状态）改为「接收 `Arc<State>`」——状态创建职责从路由模块上移到注册中心，编译期保证每个状态只有一份 `Arc` 实例。

**验证强化**：新增 `test_module_states_owns_all_domain_states`，对全部 7 套域状态逐一 `Arc::ptr_eq` 校验克隆后仍指向同一实例（唯一真源）；测试用 `LogStore::new(16)` 最小容量实例，不触碰生产数据。

**收益**：状态创建时机全部集中到 `ModuleStates::new()`，为「统一健康检查 / 统一生命周期 / 统一观测」铺平了最后一段路；`build_*_router` 签名现在与专家域（`build_experts_*_router(Arc<ExpertsSharedState>)`）完全一致，9 套状态的构造风格终结于两套（域状态 `Arc<T>` + 框架状态 `GatewayState`/`AllianceGatewayState`）。'''

c = c.replace(anchor + nl + nl + '---', anchor + nl + nl + sec8_7 + nl + '---')
open(f, "w", encoding="utf-8").write(c)
print("done87")
