//! 领域层 Trait 抽象（DIP 依赖反转）。
//!
//! 供 L2 编排层（orchestrator）依赖的**稳定服务抽象契约**，禁止反向依赖具体服务实现。
//! 所有具体实现（services.rs 中的 MemberService/PermissionService/TaskService/CommService）
//! 都在 L3 层 **impl 本文件定义的 trait**，从而满足 AIS DIP："L2→抽象、L3→实现抽象"。

use async_trait::async_trait;

use crate::error::Result;
use crate::model::*;
use crate::rbac::{Permission, ResourceCtx, RoleBinding};

// ===================== 成员管理（Member） =====================

/// 成员服务抽象：璇玑与成员的增删改查/邀请/列表。
#[async_trait]
pub trait MemberServiceTrait: Send + Sync {
    /// 邀请新成员（执行唯一性/配额校验）。
    async fn invite(&self, by: &str, input: &InviteInput) -> Result<Member>;
    /// 列出指定璇玑下的所有成员。
    async fn list(&self, xuanji_id: &str) -> Result<Vec<Member>>;
}

// ===================== 权限（Permission） =====================

/// 权限服务抽象：RBAC authorize 与角色绑定写入。
#[async_trait]
pub trait PermissionServiceTrait: Send + Sync {
    /// 判定某主体是否具备特定权限（resource 作用域下）。
    async fn authorize(&self, member_id: &str, perm: Permission, ctx: &ResourceCtx) -> Result<()>;
    /// 授予角色（写入持久化）。
    async fn assign_role(&self, binding: RoleBinding);
}

// ===================== 任务（Task） ==========================

/// 任务服务抽象：生命周期流转、评论、关注、分配、读取。
#[async_trait]
pub trait TaskServiceTrait: Send + Sync {
    /// 分配任务给指定处理人。
    async fn assign(&self, task_id: &str, actor: &str, assignees: Vec<String>) -> Result<Task>;
    /// 任务状态迁移（状态机校验、事件发布）。
    async fn transition(&self, task_id: &str, by: &str, to: TaskStatus) -> Result<Task>;
    /// 任务评论（写入通信通道）。
    async fn comment(&self, task_id: &str, by: &str, body: &str) -> Result<Message>;
    /// 关注/订阅任务变更通知（返回订阅后的 Task）。
    async fn watch(&self, task_id: &str, actor: &str) -> Result<Task>;
}

// ===================== 通信（Comm） =========================

/// 通信服务抽象：在璇玑 / 任务通道里发送消息。
#[async_trait]
pub trait CommServiceTrait: Send + Sync {
    /// 发送文本消息到指定 channel（默认 Chat 类型）。
    async fn send_message(
        &self,
        channel_id: &str,
        actor: &str,
        body: &str,
        kind: MessageKind,
    ) -> Result<Message>;
}
