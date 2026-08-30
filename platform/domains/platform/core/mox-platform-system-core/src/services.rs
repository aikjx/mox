// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 业务服务层
//!
//! 每个服务封装一类领域能力与对应的写后事件发布。
//! 鉴权（RBAC）由 `PermissionService` 统一提供，服务方法本身只负责领域逻辑，
//! 调用方（编排层 / HTTP 层）负责在调用前做 `require(...)` 鉴权。

pub mod comm;
pub mod member;
pub mod permission;
pub mod task;

// ===== 重导出所有公开服务（保持 API 兼容）=====
pub use comm::CommService;
pub use member::MemberService;
pub use permission::PermissionService;
pub use task::TaskService;

// ---------- DIP Trait 实现：把 L3 具体服务绑定到 L2 依赖的抽象（AIS DIP）----------
// 注意：`use async_trait` 为了在 trait 内使用 async fn；具体 impl 保留对 crate 内部
// 结构的自由读写，而对外暴露抽象接口。编排层 orchestrator 只依赖 trait，
// 不再 `use crate::services::*`。

#[allow(unused_imports)]
use crate::domain_traits::{
    CommServiceTrait, MemberServiceTrait, PermissionServiceTrait, TaskServiceTrait,
};
use crate::error::Result;
use crate::model::*;
use crate::rbac::*;

#[async_trait::async_trait]
impl MemberServiceTrait for MemberService {
    async fn invite(&self, by: &str, input: &InviteInput) -> Result<Member> {
        MemberService::invite(self, by, input).await
    }
    async fn list(&self, mox_id: &str) -> Vec<Member> {
        MemberService::list(self, mox_id).await
    }
    async fn get(&self, id: &str) -> Result<Member> {
        MemberService::get(self, id).await
    }
    async fn activate(&self, member_id: &str, by: &str) -> Result<Member> {
        MemberService::activate(self, member_id, by).await
    }
    async fn set_status(&self, member_id: &str, status: MemberStatus) -> Result<Member> {
        MemberService::set_status(self, member_id, status).await
    }
}

#[async_trait::async_trait]
impl PermissionServiceTrait for PermissionService {
    async fn authorize(&self, member_id: &str, perm: Permission, ctx: &ResourceCtx) -> Result<()> {
        PermissionService::authorize(self, member_id, perm, ctx).await
    }
    async fn assign_role(&self, binding: RoleBinding) {
        let _ = PermissionService::assign_role(self, binding).await;
    }
    async fn bindings_of(&self, member_id: &str) -> Vec<RoleBinding> {
        PermissionService::bindings_of(self, member_id).await
    }
    async fn effective_permissions(&self, member_id: &str) -> Vec<Permission> {
        PermissionService::effective_permissions(self, member_id).await
    }
}

#[async_trait::async_trait]
impl TaskServiceTrait for TaskService {
    async fn create(
        &self,
        mox_id: &str,
        actor: &str,
        title: &str,
        description: &str,
        priority: Priority,
    ) -> Result<Task> {
        TaskService::create(self, mox_id, actor, title, description, priority).await
    }
    async fn list(&self, mox_id: &str) -> Vec<Task> {
        TaskService::list(self, mox_id).await
    }
    async fn get(&self, id: &str) -> Result<Task> {
        TaskService::get(self, id).await
    }
    async fn assign(&self, task_id: &str, actor: &str, assignees: Vec<String>) -> Result<Task> {
        TaskService::assign(self, task_id, actor, assignees).await
    }
    async fn transition(&self, task_id: &str, by: &str, to: TaskStatus) -> Result<Task> {
        TaskService::transition(self, task_id, by, to).await
    }
    async fn comment(&self, task_id: &str, by: &str, body: &str) -> Result<Message> {
        TaskService::comment(self, task_id, by, body).await
    }
    async fn watch(&self, task_id: &str, actor: &str) -> Result<Task> {
        TaskService::watch(self, task_id, actor).await
    }
    async fn add_subtask(&self, task_id: &str, title: &str) -> Result<Task> {
        TaskService::add_subtask(self, task_id, title).await
    }
    async fn add_dependency(&self, task_id: &str, dep_id: &str) -> Result<Task> {
        TaskService::add_dependency(self, task_id, dep_id).await
    }
    async fn toggle_subtask(&self, task_id: &str, sub_id: &str) -> Result<Task> {
        TaskService::toggle_subtask(self, task_id, sub_id).await
    }
}

#[async_trait::async_trait]
impl CommServiceTrait for CommService {
    async fn create_channel(
        &self,
        mox_id: &str,
        kind: ChannelKind,
        name: &str,
        members: Vec<String>,
    ) -> Channel {
        CommService::create_channel(self, mox_id, kind, name, members).await
    }
    async fn send_message(
        &self,
        channel_id: &str,
        actor: &str,
        body: &str,
        kind: MessageKind,
    ) -> Result<Message> {
        CommService::send_message(self, channel_id, actor, body, kind).await
    }
    async fn list_messages(&self, channel_id: &str) -> Vec<Message> {
        CommService::list_messages(self, channel_id).await
    }
    async fn notify(&self, member_id: &str, title: &str, body: &str, related_task: Option<&str>) {
        CommService::notify(self, member_id, title, body, related_task).await
    }
    async fn list_notifications(&self, member_id: &str) -> Vec<Notification> {
        CommService::list_notifications(self, member_id).await
    }
    async fn mark_read(&self, id: &str, member_id: &str) -> Result<()> {
        CommService::mark_read(self, id, member_id).await
    }
}
