// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 领域层 Trait 抽象（DIP 依赖反转）。
//!
//! 供 L2 编排层（orchestrator/server）依赖的**稳定服务抽象契约**，禁止反向依赖具体服务实现。
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
    /// 按 id 获取单个成员（不存在时 NotFound）。
    async fn get(&self, id: &str) -> Result<Member>;
    /// 激活成员（Invited → Active 状态机迁移）。
    async fn activate(&self, member_id: &str, by: &str) -> Result<Member>;
    /// 通用状态迁移（BR-21 状态机校验）。
    async fn set_status(&self, member_id: &str, status: MemberStatus) -> Result<Member>;
    /// 列出指定璇玑下的所有成员（空列表而非 NotFound）。
    async fn list(&self, mox_id: &str) -> Vec<Member>;
}

// ===================== 权限（Permission） =====================

/// 权限服务抽象：RBAC authorize 与角色绑定写入。
#[async_trait]
pub trait PermissionServiceTrait: Send + Sync {
    /// 判定某主体是否具备特定权限（resource 作用域下）。
    async fn authorize(&self, member_id: &str, perm: Permission, ctx: &ResourceCtx) -> Result<()>;
    /// 授予角色（写入持久化）。
    async fn assign_role(&self, binding: RoleBinding);
    /// 查询某成员当前的角色绑定列表。
    async fn bindings_of(&self, member_id: &str) -> Vec<RoleBinding>;
    /// 计算成员的有效权限：角色 → 权限展开（默认继承，企业级审计友好）。
    async fn effective_permissions(&self, member_id: &str) -> Vec<Permission>;
}

// ===================== 任务（Task） ==========================

/// 任务服务抽象：生命周期流转、评论、关注、分配、读取、创建。
#[async_trait]
pub trait TaskServiceTrait: Send + Sync {
    /// 创建任务（写入仓库 + 事件发布）。
    async fn create(
        &self,
        mox_id: &str,
        actor: &str,
        title: &str,
        description: &str,
        priority: Priority,
    ) -> Result<Task>;
    /// 按 id 读取任务（NotFound 时 Err）。
    async fn get(&self, id: &str) -> Result<Task>;
    /// 列出指定璇玑下的所有任务。
    async fn list(&self, mox_id: &str) -> Vec<Task>;
    /// 分配任务给指定处理人。
    async fn assign(&self, task_id: &str, actor: &str, assignees: Vec<String>) -> Result<Task>;
    /// 任务状态迁移（状态机校验、事件发布）。
    async fn transition(&self, task_id: &str, by: &str, to: TaskStatus) -> Result<Task>;
    /// 任务评论（写入通信通道）。
    async fn comment(&self, task_id: &str, by: &str, body: &str) -> Result<Message>;
    /// 关注/订阅任务变更通知（返回订阅后的 Task）。
    async fn watch(&self, task_id: &str, actor: &str) -> Result<Task>;
    /// 登记子任务（task → sub，子任务继承父任务的璇玑 id 与优先级）。
    async fn add_subtask(&self, task_id: &str, title: &str) -> Result<Task>;
    /// 登记依赖（task → dep，前置 dep 完成后 task 才能推进；检测环并 Err）。
    async fn add_dependency(&self, task_id: &str, dep_id: &str) -> Result<Task>;
    /// 切换子任务完成状态（子任务完成/未完成切换；DoD 门禁依赖此状态）。
    async fn toggle_subtask(&self, task_id: &str, sub_id: &str) -> Result<Task>;
}

// ===================== 通信（Comm） =========================

/// 通信服务抽象：频道、消息、通知三位一体（DIP：trait 暴露所有 server 需要的能力）。
#[async_trait]
pub trait CommServiceTrait: Send + Sync {
    /// 创建频道（璇玑 / 任务 / 私信等），返回创建后的 Channel。
    async fn create_channel(
        &self,
        mox_id: &str,
        kind: ChannelKind,
        name: &str,
        members: Vec<String>,
    ) -> Channel;
    /// 发送文本消息到指定 channel（默认 Chat 类型）。
    async fn send_message(
        &self,
        channel_id: &str,
        actor: &str,
        body: &str,
        kind: MessageKind,
    ) -> Result<Message>;
    /// 列出指定频道所有消息（按时间升序）。
    async fn list_messages(&self, channel_id: &str) -> Vec<Message>;
    /// 向单个成员推送通知（related_task 关联任务 id，用于点击跳转）。
    async fn notify(&self, member_id: &str, title: &str, body: &str, related_task: Option<&str>);
    /// 列出某成员的全部通知（含已读/未读）。
    async fn list_notifications(&self, member_id: &str) -> Vec<Notification>;
    /// 把一条通知标记为已读（id + member_id 校验归属）。
    async fn mark_read(&self, id: &str, member_id: &str) -> Result<()>;
}
