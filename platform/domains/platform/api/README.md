# mox-platform-api

MOX 平台域 API 契约层 — 定义 IAM、租户、工作流编排、审计日志等平台核心能力的 trait 契约与数据结构。

## 功能特性

- **统一身份认证** — 提供 `IdentityProvider` trait，支持用户名密码认证、Token 校验与刷新、登出等完整身份生命周期管理
- **用户与角色管理** — `UserManager` trait 定义用户 CRUD、角色分配等用户管理能力
- **多租户管理** — `TenantManager` trait 提供租户创建、查询、更新与列表能力，支持套餐计划与元数据扩展
- **工作流编排** — `WorkflowOrchestrator` trait 定义工作流提交、状态查询、取消与列表，支持任务依赖图
- **企业审计日志** — `AuditLogger` trait 提供审计日志写入与查询，满足合规性要求
- **标准化错误体系** — `PlatformApiError` 统一错误类型（NotFound / Unauthorized / Forbidden / Conflict / Validation / Internal）

## 架构定位

本 crate 属于 MOX 平台 **L5 Domain 抽象层**，定义平台域的核心服务契约：

```text
L1 Gateway (mox-platform-gateway-svc)
    │
L2/L3 Service (mox-platform-enterprise-svc, mox-platform-orchestrator-svc)
    │  impl
L5 API Traits ← 本 crate（IdentityProvider / UserManager / TenantManager / WorkflowOrchestrator / AuditLogger）
    │
L4 Core Implementation (mox-platform-iam-core, mox-platform-meta-core, ...)
```

所有平台服务的具体实现均需实现本 crate 定义的 trait，确保上层调用与下层实现解耦。

## 快速开始

### 添加依赖

```toml
[dependencies]
mox-platform-api = { path = "../../api" }
```

### 基本用法示例

实现一个内存版的身份认证提供者：

```rust
use mox_platform_api::{IdentityProvider, AuthToken, UserInfo, PlatformApiResult};
use async_trait::async_trait;

pub struct MemoryIdentityProvider {
    // 内存存储...
}

#[async_trait]
impl IdentityProvider for MemoryIdentityProvider {
    async fn authenticate(&self, username: &str, password: &str) -> PlatformApiResult<AuthToken> {
        // 认证逻辑...
    }

    async fn validate_token(&self, token: &str) -> PlatformApiResult<UserInfo> {
        // Token 校验逻辑...
    }

    async fn refresh_token(&self, refresh_token: &str) -> PlatformApiResult<AuthToken> {
        // Token 刷新逻辑...
    }

    async fn logout(&self, token: &str) -> PlatformApiResult<()> {
        // 登出逻辑...
    }
}
```

提交工作流：

```rust
use mox_platform_api::{WorkflowOrchestrator, WorkflowInfo, TaskInfo, TaskStatus};

async fn submit_workflow(orchestrator: &dyn WorkflowOrchestrator) -> PlatformApiResult<String> {
    let workflow = WorkflowInfo {
        id: "wf-001".to_string(),
        name: "数据同步工作流".to_string(),
        tasks: vec![
            TaskInfo {
                id: "task-1".to_string(),
                name: "数据抽取".to_string(),
                workflow_id: "wf-001".to_string(),
                status: TaskStatus::Pending,
                depends_on: vec![],
                result: None,
                error: None,
                started_at: None,
                completed_at: None,
            },
        ],
        status: TaskStatus::Pending,
        created_at: "2026-01-01T00:00:00Z".to_string(),
    };

    orchestrator.submit(workflow).await
}
```

## 核心模块/类型列表

### 错误与结果类型
- `PlatformApiError` — 平台 API 统一错误枚举
- `PlatformApiResult<T>` — 统一结果类型别名

### IAM 模块
- `UserInfo` — 用户信息结构体（id / username / email / tenant_id / roles / enabled / created_at）
- `AuthToken` — 认证令牌结构体（access_token / refresh_token / expires_in / token_type）
- `IdentityProvider` — 身份认证 trait（authenticate / validate_token / refresh_token / logout）
- `UserManager` — 用户管理 trait（create_user / get_user / update_user / delete_user / list_users / assign_role）

### 租户模块
- `TenantInfo` — 租户信息结构体（id / name / plan / status / metadata / created_at）
- `TenantManager` — 租户管理 trait（create_tenant / get_tenant / update_tenant / list_tenants）

### 编排模块
- `TaskStatus` — 任务状态枚举（Pending / Running / Completed / Failed / Cancelled / Skipped）
- `TaskInfo` — 任务信息结构体
- `WorkflowInfo` — 工作流信息结构体
- `WorkflowOrchestrator` — 工作流编排 trait（submit / get_status / cancel / list）

### 审计模块
- `AuditLogEntry` — 审计日志条目结构体（id / tenant_id / user_id / action / resource_type / resource_id / timestamp / details）
- `AuditLogger` — 审计日志 trait（log / query）

## License

Licensed under the MIT License.

See the LICENSE file in the workspace root for details.
