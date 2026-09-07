# API 注册表（权威·接口↔实现一一对应）

> 本文档为网关 8080 暴露的全部 API 的唯一权威清单，由 `platform/gateway/mox-platform-gateway-svc/src/actuator.rs` 的 `ROUTES` 静态表直接生成（生成脚本 `scripts/gen-api-registry.py`）。**声明即实现**：表中每一条都有对应源码注册与真实 handler，不存在纯占位条目。

## 1. 总览

| 指标 | 值 |
| --- | --- |
| 注册路由总数 | **222 条**（全部 ready，全部有真实实现） |
| 业务域（网关内嵌） | 13 个：actuator / platform / kg / ai / kb / alliance / system / experts / monitor / projects / workspace / notification / misc |
| 域描述符（业务规划） | 43 个：ready 7 · beta 1 · stub 35（见 §3） |
| 独立服务进程 | 6 个：kg-hub / kb-server / alliance-executor / alliance-scheduler / primiflow / melody2score（见 §4） |
| 鉴权 | 全部业务路由经 `Authorization: Bearer <dev-secret-token>`（JWT）保护；管理面 `/health /metrics /actuator` 公开 |

## 2. 逐域注册表（199 条）

按域分组，实现位置逐一标注；`ANY` 表示该方法+参数可匹配多方法（GET/POST/PUT/DELETE）。

### actuator（10 条）

实现：`platform/gateway/mox-platform-gateway-svc/src/actuator.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `actuator.index` | GET | `/actuator` | L0 | 管理面端点索引 |
| `actuator.health` | GET | `/actuator/health` | L0 | 健康检查（Spring Boot 风格） |
| `actuator.info` | GET | `/actuator/info` | L0 | 构建信息（版本/时间/Git） |
| `actuator.mappings` | GET | `/actuator/mappings` | L0 | 全部 API 注册表（?layer&domain&status&q&only_enabled） |
| `actuator.metrics` | GET | `/actuator/metrics` | L0 | 运行时指标（JVM/CPU/内存/GC/线程/请求） |
| `actuator.env` | GET | `/actuator/env` | L0 | 网关配置（密钥脱敏） |
| `actuator.loggers` | ANY | `/actuator/loggers` | L0 | 日志级别查看/动态调整 |
| `actuator.logs` | ANY | `/actuator/logs` | L0 | 在线日志查询（?level&search&limit&offset） |
| `actuator.logs_tail` | GET | `/actuator/logs/tail` | L0 | SSE 实时日志流（curl -N） |
| `actuator.api` | ANY | `/actuator/api/:id` | L0 | 按 API 启停管理（/enable|/disable） |

### platform（6 条）

实现：`lib.rs`（内联路由）+ `proxy.rs`（反向代理 :3001/:8000）

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `platform.health` | GET | `/health` | L0 | 存活探针（网关 Rust axum 版本） |
| `platform.metrics` | GET | `/metrics` | L0 | Prometheus 指标端点（o11y.rs 真实采集） |
| `platform.status` | GET | `/api/v1/status` | L0 | 网关状态（域就绪统计+认证+限流） |
| `platform.domains` | GET | `/api/v1/domains` | L0 | 43 业务域描述符列表（自描述） |
| `platform.proxy_orchestrator` | ANY | `/api/{*path}` | L6 | 业务域反向代理→编排器（默认 :3001，catch-all） |
| `platform.proxy_primiflow` | ANY | `/api/projects/{*path}` | L6 | 项目域反向代理→PrimiFlow（默认 :8000） |

### kg（6 条）

实现：`platform/domains/kg/svc/mox-kg-service-svc/src/http_adapter.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `kg.graph.neighborhood` | GET | `/kg/v1/neighborhood` | L2 | 实体邻域查询（多跳邻居+边） |
| `kg.graph.path` | GET | `/kg/v1/path` | L2 | 两实体间路径枚举（BFS） |
| `kg.graph.shortest_path` | GET | `/kg/v1/shortest-path` | L2 | 最短路径（Dijkstra 边权重） |
| `kg.graph.centrality` | GET | `/kg/v1/centrality` | L2 | 中心性分析（度/介数/接近） |
| `kg.graph.communities` | GET | `/kg/v1/communities` | L2 | 社区发现（Louvain 模块度） |
| `kg.graph.stats` | GET | `/kg/v1/stats` | L2 | 图谱统计（节点/边/标签分布） |

### ai（4 条）

实现：`platform/domains/kg/svc/mox-kg-service-svc/src/http_adapter.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `ai.engine.process` | POST | `/ai/engine/process` | L3 | AI 引擎统一处理（多模型路由） |
| `ai.engine.analyze` | POST | `/ai/engine/analyze` | L3 | AI 深度分析（结构化输出） |
| `ai.engine.capabilities` | GET | `/ai/engine/capabilities` | L3 | AI 引擎能力清单（模型/工具/配额） |
| `ai.engine.metrics` | GET | `/ai/engine/metrics` | L3 | AI 引擎运行指标（调用量/延迟/成功率） |

### kb（17 条）

实现：`platform/domains/kg/svc/mox-kb-svc/src/handlers.rs`（nest `/api`）+ `kb_ext.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `kb.documents.list` | ANY | `/api/kb/documents` | L2 | 文档列表/上传/搜索（云盘根目录） |
| `kb.documents.detail` | ANY | `/api/kb/documents/:id` | L2 | 文档详情/下载/删除/元数据更新 |
| `kb.documents.analyze` | POST | `/api/kb/documents/:id/analyze` | L2 | 文档 AI 分析（摘要/关键词/实体） |
| `kb.documents.batch_analyze` | POST | `/api/kb/batch-analyze` | L2 | 批量文档分析（异步任务） |
| `kb.categories.list` | GET | `/api/kb/categories` | L2 | 知识库分类树 |
| `kb.tags.list` | GET | `/api/kb/tags` | L2 | 标签云/标签列表 |
| `kb.search.query` | POST | `/api/kb/search` | L2 | 全文检索（向量+关键词混合） |
| `kb.versions.list` | ANY | `/api/kb/documents/:id/versions` | L2 | 文档版本列表 |
| `kb.versions.detail` | GET | `/api/kb/documents/:id/versions/:ver` | L2 | 指定版本详情/下载 |
| `kb.versions.compare` | POST | `/api/kb/documents/:id/versions/compare` | L2 | 版本差异对比（diff） |
| `kb.versions.revert` | POST | `/api/kb/documents/:id/versions/revert` | L2 | 回滚到指定版本 |
| `kb.entities.list` | ANY | `/api/kb/documents/:id/entities` | L2 | 文档实体抽取结果/关联/解关联（handlers+ext） |
| `kb.graph.link` | ANY | `/api/kb/documents/:id/graph-link` | L2 | 文档→知识图谱关联/挂图 |
| `kb.documents.history` | GET | `/api/kb/documents/:id/history` | L2 | 文档操作历史（审计） |
| `kb.stats.summary` | GET | `/api/kb/stats` | L2 | 知识库统计（文档数/容量/活跃度） |
| `kb.history.list` | GET | `/api/kb/history` | L2 | 全局操作历史（最近活动） |
| `kb.entities.search` | GET | `/api/kb/entities/search` | L2 | 实体语义搜索（kb_ext.rs） |

### alliance（20 条）

实现：`platform/domains/alliance/sdk/mox-alliance-http-sdk/src/alliance.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `alliance.runtime` | GET | `/api/alliance/runtime` | L4 | 运行时就绪状态（远程/本地预览） |
| `alliance.tasks.list` | ANY | `/api/alliance/tasks` | L4 | 联盟任务列表/创建（InMemoryTaskRepository 真实存储） |
| `alliance.tasks.detail` | ANY | `/api/alliance/tasks/:id` | L4 | 任务详情/操作（暂停/恢复/取消） |
| `alliance.tasks.pause` | POST | `/api/alliance/tasks/:id/pause` | L4 | 暂停任务 |
| `alliance.tasks.resume` | POST | `/api/alliance/tasks/:id/resume` | L4 | 恢复任务 |
| `alliance.tasks.cancel` | POST | `/api/alliance/tasks/:id/cancel` | L4 | 取消任务 |
| `alliance.tasks.retry` | POST | `/api/alliance/tasks/:id/retry` | L4 | 重试任务 |
| `alliance.experts.search` | POST | `/api/alliance/experts/search` | L4 | 专家匹配搜索（RuleBasedExpertMatcher 真实匹配） |
| `alliance.tasks.execution_status` | GET | `/api/alliance/tasks/:id/execution-status` | L4 | 执行状态查询（真实节点统计） |
| `alliance.tasks.nodes` | GET | `/api/alliance/tasks/:id/nodes` | L4 | 执行节点列表（真实 DAG 节点） |
| `alliance.tasks.node` | ANY | `/api/alliance/tasks/:id/nodes/:node_id` | L4 | 节点详情/跳过（人工干预） |
| `alliance.tasks.logs` | GET | `/api/alliance/tasks/:id/logs` | L4 | 任务执行日志（真实存储） |
| `alliance.tasks.logs_stream` | GET | `/api/alliance/tasks/:id/logs/stream` | L4 | 任务日志流式推送 |
| `alliance.tasks.fusion` | GET | `/api/alliance/tasks/:id/fusion-result` | L4 | 融合结果（真实从节点输出融合） |
| `alliance.tasks.fusion_alias` | GET | `/api/alliance/tasks/:id/fusion` | L4 | 融合结果（兼容别名） |
| `alliance.tasks.dag` | GET | `/api/alliance/tasks/:id/dag` | L4 | DAG 节点+边（真实存储的 DAG） |
| `alliance.tasks.toggle_done` | PUT | `/api/alliance/tasks/:id/toggle-done` | L4 | 完成状态切换（真实状态流转） |
| `alliance.tasks.status_poll` | GET | `/api/alliance/tasks/:id/status` | L4 | 任务状态轮询（供前端轮询） |
| `alliance.tasks.plan` | GET | `/api/alliance/tasks/:id/plan` | L4 | 协作计划查询 |
| `alliance.stats` | GET | `/api/alliance/stats` | L4 | 联盟统计（专家/任务/成功率） |

### system（46 条）

实现：`platform/gateway/mox-platform-gateway-svc/src/system.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `system.auth.me` | GET | `/api/auth/me` | L5 | 当前登录用户信息 |
| `system.permissions.current` | GET | `/api/system/permissions` | L5 | 当前用户权限/角色/菜单 |
| `system.dept.list` | ANY | `/api/system/dept` | L5 | 部门列表/创建 |
| `system.dept.tree` | GET | `/api/system/dept/tree` | L5 | 部门树 |
| `system.dept.detail` | ANY | `/api/system/dept/:id` | L5 | 部门详情/更新/删除 |
| `system.dept.users` | GET | `/api/system/dept/:id/users` | L5 | 部门用户列表 |
| `system.post.list` | ANY | `/api/system/post` | L5 | 岗位列表/创建 |
| `system.post.by_dept` | GET | `/api/system/post/dept/:deptId` | L5 | 按部门查询岗位 |
| `system.post.detail` | ANY | `/api/system/post/:id` | L5 | 岗位详情/更新/删除 |
| `system.user.list` | ANY | `/api/system/user` | L5 | 用户列表/创建 |
| `system.user.detail` | ANY | `/api/system/user/:id` | L5 | 用户详情/更新/删除 |
| `system.user.reset_pwd` | PUT | `/api/system/user/:id/resetPwd` | L5 | 重置用户密码 |
| `system.user.change_status` | PUT | `/api/system/user/:id/changeStatus` | L5 | 用户状态切换（启用/停用） |
| `system.user.roles` | ANY | `/api/system/user/:id/roles` | L5 | 用户角色查询/分配 |
| `system.role.list` | ANY | `/api/system/role` | L5 | 角色列表/创建 |
| `system.role.detail` | ANY | `/api/system/role/:id` | L5 | 角色详情/更新/删除 |
| `system.role.menu_perms` | ANY | `/api/system/role/:id/menuPerms` | L5 | 角色菜单权限查询/设置 |
| `system.role.data_perms` | ANY | `/api/system/role/:id/dataPerms` | L5 | 角色数据权限查询/设置 |
| `system.role.users` | GET | `/api/system/role/:id/users` | L5 | 角色用户列表 |
| `system.role.copy` | POST | `/api/system/role/:id/copy` | L5 | 复制角色 |
| `system.menu.tree` | GET | `/api/system/menu/tree` | L5 | 菜单树（用户可见） |
| `system.menu.list` | ANY | `/api/system/menu` | L5 | 菜单列表/创建 |
| `system.menu.detail` | ANY | `/api/system/menu/:id` | L5 | 菜单详情/更新/删除 |
| `system.dict_type.list` | ANY | `/api/system/dict/type` | L5 | 字典类型列表/创建 |
| `system.dict_type.all` | GET | `/api/system/dict/type/all` | L5 | 全部字典类型 |
| `system.dict_type.detail` | ANY | `/api/system/dict/type/:id` | L5 | 字典类型详情/更新/删除 |
| `system.dict_data.list` | ANY | `/api/system/dict/data` | L5 | 字典数据列表/创建 |
| `system.dict_data.by_type` | GET | `/api/system/dict/data/type/:dictType` | L5 | 按类型查询字典数据 |
| `system.dict_data.detail` | ANY | `/api/system/dict/data/:id` | L5 | 字典数据详情/更新/删除 |
| `system.config.list` | ANY | `/api/system/config` | L5 | 参数配置列表/创建 |
| `system.config.refresh` | DELETE | `/api/system/config/refresh-cache` | L5 | 刷新配置缓存 |
| `system.config.detail` | ANY | `/api/system/config/:id` | L5 | 配置详情/更新/删除 |
| `system.config.by_key` | GET | `/api/system/config/key/:key` | L5 | 按键查询配置 |
| `system.operlog.list` | GET | `/api/system/operlog` | L5 | 操作日志列表 |
| `system.operlog.clean` | DELETE | `/api/system/operlog/clean` | L5 | 清空操作日志 |
| `system.operlog.detail` | ANY | `/api/system/operlog/:id` | L5 | 操作日志详情/删除 |
| `system.operlog.export` | GET | `/api/system/operlog/export` | L5 | 导出操作日志（CSV） |
| `system.loginlog.list` | GET | `/api/system/logininfor` | L5 | 登录日志列表 |
| `system.loginlog.clean` | DELETE | `/api/system/logininfor/clean` | L5 | 清空登录日志 |
| `system.loginlog.detail` | DELETE | `/api/system/logininfor/:id` | L5 | 删除登录日志 |
| `system.loginlog.export` | GET | `/api/system/logininfor/export` | L5 | 导出登录日志（CSV） |
| `system.security.status` | GET | `/api/security/status` | L5 | 安全状态（认证/限流/IAM） |
| `system.security.api_keys` | ANY | `/api/security/api-keys` | L5 | API Key 列表/创建（SQLite 持久化） |
| `system.security.api_key_revoke` | DELETE | `/api/security/api-keys/:id` | L5 | 吊销 API Key（DB+内存双删） |
| `system.security.api_key_validate` | POST | `/api/security/validate` | L5 | 校验 API Key 明文 |
| `system.security.audit_log` | GET | `/api/security/audit-log` | L5 | 审计日志（SQLite 读取） |

### experts（49 条）

实现：`experts_registry/collaboration/dispatcher/graph/orchestration/session/ext.rs` 七模块

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `experts.registry.list` | GET | `/api/experts` | L3 | 专家列表（注册中心） |
| `experts.registry.capabilities` | GET | `/api/experts/capabilities` | L3 | 专家能力清单 |
| `experts.registry.metrics` | GET | `/api/experts/metrics` | L3 | 专家运行指标 |
| `experts.registry.overview` | GET | `/api/experts/overview` | L3 | 专家体系总览 |
| `experts.registry.stats` | GET | `/api/experts/stats` | L3 | 专家统计 |
| `experts.registry.detail` | ANY | `/api/experts/:id` | L3 | 专家详情/维护 |
| `experts.registry.detail_metrics` | GET | `/api/experts/:id/metrics` | L3 | 单个专家指标 |
| `experts.registry.consult_room` | GET | `/api/experts/bookings/:id/consult-room` | L3 | 咨询室接入（真实房间） |
| `experts.registry.team` | POST | `/api/experts/team` | L3 | 组建专家团队 |
| `experts.registry.consult_now` | POST | `/api/experts/:id/consult-now` | L3 | 立即咨询专家 |
| `experts.collab.consult` | POST | `/api/experts/:id/consult` | L3 | 单专家咨询 |
| `experts.collab.multi_consult` | POST | `/api/experts/multi-consult` | L3 | 多专家协同咨询 |
| `experts.collab.debate` | POST | `/api/experts/debate` | L3 | 专家辩论 |
| `experts.collab.route` | POST | `/api/experts/route` | L3 | 智能路由 |
| `experts.collab.intelligent_consult` | POST | `/api/experts/intelligent-consult` | L3 | 智能咨询 |
| `experts.collab.algorithm_analysis` | POST | `/api/experts/algorithm-analysis` | L3 | 算法分析 |
| `experts.collab.enterprise_consult` | POST | `/api/experts/enterprise/consult` | L3 | 企业级咨询 |
| `experts.collab.enterprise_analyze` | POST | `/api/experts/enterprise/analyze` | L3 | 企业级分析 |
| `experts.dispatch.status` | GET | `/api/experts/dispatcher/status` | L3 | 调度器状态 |
| `experts.dispatch.dispatch` | POST | `/api/experts/dispatcher/dispatch` | L3 | 任务分发 |
| `experts.dispatch.consult` | POST | `/api/experts/dispatcher/consult` | L3 | 调度咨询 |
| `experts.dispatch.multi_consult` | POST | `/api/experts/dispatcher/multi-consult` | L3 | 调度多专家咨询 |
| `experts.dispatch.reset` | POST | `/api/experts/dispatcher/reset/:id` | L3 | 重置调度状态 |
| `experts.dispatch.reset_all` | POST | `/api/experts/dispatcher/reset-all` | L3 | 全量重置调度 |
| `experts.graph.overview` | GET | `/api/expert-graph` | L3 | 专家协作图总览 |
| `experts.graph.stats` | GET | `/api/expert-graph/stats` | L3 | 协作图统计 |
| `experts.graph.neighbors` | GET | `/api/expert-graph/neighbors/:id` | L3 | 专家邻域 |
| `experts.graph.collaborators` | GET | `/api/expert-graph/collaborators/:id` | L3 | 协作伙伴 |
| `experts.graph.path` | GET | `/api/expert-graph/path/:source/:target` | L3 | 专家间路径 |
| `experts.graph.communities` | GET | `/api/expert-graph/communities` | L3 | 协作社区发现 |
| `experts.graph.optimal_team` | POST | `/api/expert-graph/optimal-team` | L3 | 最优团队推荐 |
| `experts.graph.rebuild` | POST | `/api/expert-graph/rebuild` | L3 | 重建协作图 |
| `experts.orch.orchestrate` | POST | `/api/experts/orchestrate` | L3 | 专家编排执行 |
| `experts.orch.plan_generate` | POST | `/api/experts/plan/generate` | L3 | 生成协作计划 |
| `experts.orch.plan_execute` | POST | `/api/experts/plan/execute` | L3 | 执行协作计划 |
| `experts.orch.stats` | GET | `/api/experts/orchestration/stats` | L3 | 编排统计 |
| `experts.orch.plugins` | GET | `/api/experts/orchestration/plugins` | L3 | 编排插件清单 |
| `experts.orch.history` | GET | `/api/experts/orchestration/history` | L3 | 编排历史 |
| `experts.session.stats` | GET | `/api/experts/sessions/stats` | L3 | 会话统计 |
| `experts.session.list` | GET | `/api/experts/sessions` | L3 | 会话列表（分页+状态/类型/专家/用户过滤+搜索） |
| `experts.session.messages` | POST | `/api/experts/sessions/:id/messages` | L3 | 发送会话消息 |
| `experts.session.similar_search` | POST | `/api/experts/sessions/:id/similar-search` | L3 | 会话相似检索 |
| `experts.session.export` | GET | `/api/experts/sessions/:id/export` | L3 | 导出会话 |
| `experts.session.archive` | POST | `/api/experts/sessions/:id/archive` | L3 | 归档会话 |
| `experts.session.semantic_search` | POST | `/api/experts/semantic-search` | L3 | 全局语义搜索 |
| `experts.ext.bookings_mine` | GET | `/api/experts/bookings/mine` | L3 | 我的预约 |
| `experts.ext.favorite` | POST | `/api/experts/:id/favorite` | L3 | 收藏专家 |
| `experts.ext.bookings_create` | POST | `/api/experts/bookings` | L3 | 创建预约 |
| `experts.ext.bookings_cancel` | PUT | `/api/experts/bookings/:id/cancel` | L3 | 取消预约 |

### monitor（12 条）

实现：`platform/gateway/mox-platform-gateway-svc/src/monitor.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `monitor.metrics_detail` | GET | `/api/monitor/metrics/detail` | L5 | 指标详情 |
| `monitor.quality` | GET | `/api/monitor/quality` | L5 | 质量评估 |
| `monitor.business` | GET | `/api/monitor/business` | L5 | 业务监控 |
| `monitor.alerts_summary` | GET | `/api/monitor/alerts/summary` | L5 | 告警摘要 |
| `monitor.nodes` | GET | `/api/monitor/nodes` | L5 | 节点列表 |
| `monitor.node_logs` | GET | `/api/monitor/nodes/:name/logs` | L5 | 节点日志 |
| `monitor.node_trace` | GET | `/api/monitor/nodes/:name/trace` | L5 | 节点链路追踪 |
| `monitor.alert_rules` | ANY | `/api/monitor/alert-rules` | L5 | 告警规则列表/创建 |
| `monitor.alert_rule_detail` | ANY | `/api/monitor/alert-rules/:id` | L5 | 告警规则详情/更新/删除 |
| `monitor.alert_rule_toggle` | PUT | `/api/monitor/alert-rules/:id/toggle` | L5 | 启停告警规则 |
| `monitor.timeseries` | GET | `/api/monitor/timeseries` | L5 | 时序数据 |
| `monitor.business_timeseries` | GET | `/api/monitor/business/timeseries` | L5 | 业务时序数据 |

### projects（14 条）

实现：`platform/gateway/mox-platform-gateway-svc/src/projects_ext.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `projects.ai_recommend` | POST | `/api/projects/ai-recommend` | L6 | AI 项目推荐 |
| `projects.members` | ANY | `/api/projects/:id/members` | L6 | 成员列表/添加 |
| `projects.member_detail` | ANY | `/api/projects/:id/members/:memberId` | L6 | 成员更新/移除 |
| `projects.phases` | GET | `/api/projects/:id/phases` | L6 | 项目阶段 |
| `projects.files` | GET | `/api/projects/:id/files` | L6 | 项目文件列表 |
| `projects.files_upload` | POST | `/api/projects/:id/files/upload` | L6 | 项目文件上传 |
| `projects.activities` | GET | `/api/projects/:id/activities` | L6 | 项目动态 |
| `projects.documents` | GET | `/api/projects/:id/documents` | L6 | 项目文档列表 |
| `projects.advance_phase` | PUT | `/api/projects/:id/advance-phase` | L6 | 推进阶段 |
| `projects.phase_progress` | GET | `/api/projects/:id/phase-progress` | L6 | 阶段进度 |
| `projects.favorite` | POST | `/api/projects/:id/favorite` | L6 | 收藏项目 |
| `projects.share` | POST | `/api/projects/:id/share` | L6 | 分享项目 |
| `projects.document_download` | GET | `/api/projects/:id/documents/:docId/download` | L6 | 项目文档下载 |
| `projects.requirements_graph` | GET | `/api/projects/:id/requirements-graph` | L6 | 需求关系图 |

### workspace（7 条）

实现：`platform/gateway/mox-platform-gateway-svc/src/workspace.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `workspace.kpi` | GET | `/api/workspace/kpi` | L5 | 工作台 KPI |
| `workspace.file_preview` | GET | `/api/files/:id/preview` | L5 | 文件预览 |
| `workspace.file_download` | GET | `/api/files/:id/download` | L5 | 文件下载 |
| `workspace.whiteboard_save` | POST | `/api/whiteboard/:sessionId/save` | L5 | 白板保存 |
| `workspace.history` | GET | `/api/workspace/history` | L5 | 工作台历史 |
| `workspace.tasks_decompose` | POST | `/api/tasks/decompose` | L5 | 任务分解 |
| `workspace.tasks_execute` | POST | `/api/tasks/:id/execute` | L5 | 任务执行 |

### notification（4 条）

实现：`platform/gateway/mox-platform-gateway-svc/src/notification.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `notification.list` | GET | `/api/notifications` | L5 | 通知列表 |
| `notification.unread_count` | GET | `/api/notifications/unread-count` | L5 | 未读数量 |
| `notification.read` | PUT | `/api/notifications/:id/read` | L5 | 标记已读 |
| `notification.read_all` | PUT | `/api/notifications/read-all` | L5 | 全部已读 |

### misc（5 条）

实现：`platform/gateway/mox-platform-gateway-svc/src/misc.rs`

| ID | 方法 | 路径 | 层 | 说明 |
| --- | --- | --- | --- | --- |
| `misc.avatar` | POST | `/api/users/:id/avatar` | L5 | 用户头像上传 |
| `misc.market_review` | POST | `/api/market/:id/review` | L5 | 市场评论 |
| `misc.ai_flow_update` | PUT | `/api/ai/flows/:id` | L5 | AI 流程更新 |
| `misc.tasks` | GET | `/api/tasks` | L5 | 任务列表（通用） |
| `misc.projects` | GET | `/api/projects` | L5 | 项目列表（通用） |

## 3. 业务域描述符（43 域·routes.rs）

| 状态 | 数量 | 域 |
| --- | --- | --- |
| ready | 7 | Health·Metrics·KG·KB·AIEngine·Alliance·Expert |
| beta | 1 | IAM（依赖编排器 :3001，未启动时 502 ORCHESTRATOR_UNREACHABLE） |
| stub | 35 | Auth·Tenant·RBAC·Graph·Cypher·nGQL·AI-Core·Intent·Flow·Workflow·BPM·Pipeline·Cloud·S3·Volume·FS·Data·ETL·Norm·Standard·Voice·MIDI·Melody·TTS·Market·Shop·Order·Billing·Streams·Kafka·WebSocket·Event·Enterprise·Platform·Audit |

> stub 仅为规划声明，不对外承诺；S3 曾标 ready 但无实现，已如实降为 stub。

## 4. 独立服务进程（网关之外）

| 服务 | 二进制 | 路由前缀 | 说明 |
| --- | --- | --- | --- |
| kg-hub | `mox-kg-hub-svc` | `/api/kg/*`（15 条） | 知识图谱枢纽：检索/影响/治理/闭环 |
| kb-server | `mox-kb-server` | `/api/v1/kb/*`（4 条） | 知识库独立服务（与网关内嵌 kb 并存） |
| alliance-executor | `mox-alliance-executor` | `/health` `/tasks/:id/*` `/internal/*`（8 条） | 联盟任务执行器 |
| alliance-scheduler | `mox-alliance-scheduler` | `/tasks` `/experts/search`（5 条） | 联盟调度器 |
| primiflow | 前端子项目 | — | `:8000`，经 `/api/projects/{*path}` 代理 |
| melody2score | 前端子项目 | — | `:8012`，简谱转谱 |

### 4.1 全链路启动命令（2026-09-07 实测通过）

五个进程按依赖顺序启动（Windows PowerShell，均为 debug 构建）：

```powershell
# 1) 联盟调度器 :3100 / 执行器 :3200（独立二进制，配置读 config/alliance-*.yml）
Start-Process target\debug\mox-alliance-scheduler.exe -ArgumentList @("--port","3100") -WindowStyle Hidden
Start-Process target\debug\mox-alliance-executor.exe -ArgumentList @("--port","3200") -WindowStyle Hidden

# 2) 知识库独立服务 :8104
Start-Process target\debug\mox-kb-server.exe -ArgumentList @("--port","8104") -WindowStyle Hidden

# 3) 编排器 :3001（OUS_ENABLE_MOX_SYSTEM=0 跳过未 bootstrap 的 mox-system 模块；OUS_API_TOKEN 与网关对齐）
$env:OUS_ENABLE_MOX_SYSTEM="0"; $env:OUS_API_TOKEN="dev-secret-token"
Start-Process target\debug\operator-server.exe -ArgumentList @("--port","3001") -WindowStyle Hidden

# 4) 网关 :8080（MOX_ALLIANCE_*_URL 激活联盟远程模式）
$env:MOX_ALLIANCE_SCHEDULER_URL="http://127.0.0.1:3100"; $env:MOX_ALLIANCE_EXECUTOR_URL="http://127.0.0.1:3200"
Start-Process target\debug\mox-server.exe -ArgumentList @("--port","8080") -WindowStyle Hidden
```

验证要点：网关 `/api/v1/status` → `iam: ready`；`/api/alliance/runtime` → `mode: remote, execution_ready: true`；联盟任务创建后经调度器真实执行（DAG 节点流转）；编排器 `/api/graph/export`、`/api/status` 需带 `Authorization: Bearer dev-secret-token`。

**无外部模型 key 的完整闭环验证（实测 2026-09-07）**：启动前设 `$env:EXECUTOR_MODE="mock"` 再运行上述脚本 → 执行器以 Mock 节点执行器启动（`/health` 返回 `execution_mode: mock`）；创建联盟任务后经 网关→调度器→执行器 全链路真实流转，DAG 5/5 节点完成，任务终态 `completed`。生产（expert）模式需配置 LLM provider key（`strict_llm_consultant_from_env`），未配置时 readiness 如实报告未就绪。

## 5. 治理规则（新增/修改 API 必须遵守）

1. **单一权威源**：所有对外路由必须先登记到 `actuator.rs` `ROUTES`，再写 handler；`/actuator/mappings` 是唯一注册表视图。
2. **前缀权威**：kg=`/kg/v1/*`；ai=`/ai/engine/*`；kb=`/api/kb/*`；alliance=`/api/alliance/*`；experts=`/api/experts*`；system/security=`/api/system/*`、`/api/security/*`；其余模块=`/api/<module>/*`。历史前缀（`/ai/v1`、`/kb/v1`、`/alliance/v1`）已废弃，一律 404。
3. **新增路由闭环**：改 `actuator.rs` → 更新本文档（重跑 `scripts/gen-api-registry.py`）→ `cargo check -p mox-platform-gateway-svc` → 启动验证 `/actuator/mappings` 计数与新增路径 200。
4. **状态语义**：`ready`=有真实 handler 且已接线路由；`stub`=仅规划；`beta`=可用但依赖外部进程。不允许出现“声明 ready 但无路由”的条目。

## 6. 变更记录

| 日期 | 变更 |
| --- | --- |
| 2026-09-06 | **注册表归一化（98→199）**：修正 ai/kb/alliance 三域前缀漂移（`/ai/engine`、`/api/kb`、`/api/alliance`）；补齐漏声明的 experts 48 / monitor 12 / projects 14 / workspace 7 / notification 4 / misc 5 / kb_ext 2 / auth 1；canonical 映射修正；S3 如实降 stub、Expert 如实升 ready；生成脚本与治理规则落地 |
| 2026-09-07 | **运行验证与语义修复**：5 进程全链路实测（编排器3001/kb8104/调度3100/执行3200/网关8080）；联盟远程模式激活（`MOX_ALLIANCE_*_URL`）；Mock 执行器全链路任务闭环 completed（5/5 节点）；readiness 语义修复（mock 模式如实就绪）；一键启停脚本落地 |
| 2026-09-07 | **Phase 0 落地（199→208）**：RBAC 域 3 条（IAM 真实仓储：角色/权限/当前用户）、Graph 域 3 条（与 kg 同源真实算法：总览/统计/社区）、Voice 域 3 条（桥接 melody2score :8012：健康/样例/识别）；三域描述符 stub→ready，全链路实测 200 |