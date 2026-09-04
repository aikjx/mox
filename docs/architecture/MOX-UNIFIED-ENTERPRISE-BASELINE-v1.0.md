# MOX 统一企业级架构基线 v1.0

状态：已落地基线，作为新系统开发的唯一扩展入口。

## 1. 目标和边界

MOX 采用“统一基座 + 领域模块 + 动态配置”的开发模式。新系统不复制网关、权限、审计、SQL 执行和流程引擎，只新增三类内容：

1. 业务数据库表和索引；
2. 通过 `mox-dsql-core` 注册的参数化 SQL；
3. 通过动态流程定义声明的处理顺序、条件和结果映射。

动态配置只能引用已经发布的 SQL code，不能存储任意脚本或把用户输入拼接到 SQL 文本。复杂逻辑应沉淀为领域服务或只读 SQL，保持安全边界和可测试性。

## 2. 归一化分层

```text
L0 Foundation       错误、协议、配置、审计、可观测性
L1 Base              统一模型、存储、索引、查询、权限、生命周期
L2 Domain Core       kg / ai / flow / data / cloud / alliance 纯领域核心
L3 Domain Service    领域 HTTP、消息、外部系统适配
L4 SDK / API         对外 DTO、客户端和跨域契约
L5 Orchestrator      业务流程组合与动态处理入口
L6 Gateway            认证、租户、限流、追踪、路由和统一响应
```

依赖方向只能从上层指向下层；跨领域调用必须经过 SDK/API 或事件契约。Core 不得依赖 Service，Service 不得反向依赖另一个领域的内部 Core。

## 3. 统一运行时

### 3.1 动态 SQL

`mox-dsql-core` 提供：

- SQL 定义 Draft → Active → Deprecated 发布生命周期；
- `{{param}}` 参数绑定和条件片段 `{?if param?}...{?endif?}`；
- 参数类型、必填、范围、枚举、正则校验；
- 版本历史、版本哈希、缓存和执行审计；
- SQL 结果统一映射为 List、Map、Single、Count、Update；
- 多语句拒绝、读写类型校验、DDL 与业务 SQL 隔离。

### 3.2 动态业务流程

流程定义由 `process_code`、步骤列表和上下文映射组成：

```json
{
  "process_code": "alliance.task.dispatch",
  "steps": [
    {
      "step_code": "load_experts",
      "sql_code": "alliance.expert.list",
      "input_mapping": {"tenant_id": "$.tenant_id", "domain": "$.domain"},
      "output_key": "experts",
      "when": "exists($.domain)"
    }
  ]
}
```

执行器支持：

- 上下文输入和步骤结果回填；
- `exists(path)`、`path == literal`、`path != literal` 条件；
- 步骤跳过、失败终止、失败继续；
- 流程版本发布和流程审计。

不支持在数据库中执行任意 Rust、JavaScript、Shell 或 Python。需要执行代码时，必须实现经过权限、超时和审计封装的领域 Operator。

## 4. 专家联盟业务流程

```text
需求输入
  → 租户/权限/幂等校验
  → 需求归一化为 FlowGraph/任务上下文
  → 专家能力匹配和并行分析
  → 结果汇合、冲突消解、置信度计算
  → AI/算子/工作流动态执行
  → 硬约束、依赖、资源和预算门禁
  → 审计事件、版本和治理记录
  → 结论、图谱、代码、指标和任务状态输出
```

数据库职责只负责事实、状态和审计；动态 SQL 负责可配置读写；动态流程负责业务步骤编排；领域服务负责不可配置的算法、外部调用和安全策略。

## 5. 专家联盟数据库基线

见：

- `deploy/sql/mox-expert-alliance.sql`
- `deploy/sql/mox-expert-alliance-dsql.sql`

核心实体：

| 实体 | 责任 |
|---|---|
| tenant | 多租户边界和租户状态 |
| expert | 专家注册、能力、状态和版本 |
| task | 需求任务、幂等键、状态机和上下文 |
| assignment | 任务与专家的分配及执行结果 |
| consensus | 专家联盟最终决策和证据 |
| task_event | 追加型事件日志，支持追踪和重放 |

所有业务表必须遵守：

- `tenant_id` 强制隔离；
- 主业务实体使用软状态，不做物理删除；
- 写操作使用幂等键或版本号；
- 关键变化写追加型事件；
- JSON 字段只承载扩展属性，不替代核心查询列；
- 所有外部输入通过绑定参数进入 SQL。

## 6. 新系统接入模板

1. 在 `deploy/sql/` 增加领域 DDL，先定义租户、状态、版本、审计和索引。
2. 在 `deploy/sql/` 增加领域 DSQL 注册脚本，所有 SQL 使用唯一 `sql_code`。
3. 使用 `CreateSqlRequest` 注册/更新定义，校验后发布为 Active。
4. 使用 `CreateProcessRequest` 注册流程，步骤只引用 Active SQL。
5. 通过 `DsqlManager::execute_process` 执行动态流程。
6. 为每个 SQL、流程和状态迁移补充单元测试和租户隔离测试。
7. 通过网关暴露 API；页面只调用 API 层，不直接拼接 SQL 或访问数据库。

## 7. 验收门禁

提交前必须通过：

- `cargo test -p mox-dsql-core --all-targets`
- 领域 Core/Service 定向单测；
- 架构约束测试：无循环依赖、无 Core → Service、无跨域内部依赖；
- 动态 SQL 安全测试：缺参、类型错误、非法正则、多语句、读写错配；
- 流程测试：条件跳过、结果映射、失败终止、失败继续、审计落库；
- 网关精确路由优先于通配代理；
- 前端生产构建和核心页面冒烟测试；
- 桌面、平板、手机三种视口下的加载、空状态、错误状态和权限状态验证。

## 8. 当前基线说明

本基线把 Mox 的可复用部分固定为“契约和运行时”，不会宣称所有已有业务域已经完成。尚未迁移的领域必须显式标记为 stub/beta，不得通过统一状态接口伪装成 ready；网关、编排器和联盟调度器仍需持续拆分，避免重新形成聚合型 God Module。
