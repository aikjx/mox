// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 成员管理服务
//!
//! 负责成员邀请、激活、状态迁移、资料更新等领域逻辑。

use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::*;
use crate::event::{DomainEvent, EventBus};
use crate::model::*;
use crate::rbac::*;
use crate::store::Store;

#[derive(Clone)]
pub struct MemberService {
    pub store: Arc<Store>,
    pub bus: EventBus,
    pub config: Arc<AppConfig>,
}

impl MemberService {
    pub fn new(store: Arc<Store>, bus: EventBus, config: Arc<AppConfig>) -> Self {
        Self { store, bus, config }
    }

    pub async fn invite(&self, by: &str, input: &InviteInput) -> Result<Member> {
        if self.store.get_mox(&input.mox_id).await.is_none() {
            return Err(AppError::BadRequest(format!(
                "璇玑 {} 不存在",
                input.mox_id
            )));
        }
        // BR-04 输入校验：邮箱格式 + 名称非空（防脏数据进入协作网络）
        if input.name.trim().is_empty() {
            return Err(AppError::BadRequest("成员名称不能为空".into()));
        }
        if !input.email.contains('@') {
            return Err(AppError::BadRequest(format!(
                "邮箱格式非法: {}",
                input.email
            )));
        }
        // 配额（I-03）：单璇玑成员数上限
        let count = self.store.list_members(&input.mox_id).await.len();
        if count >= self.config.quotas.max_members {
            return Err(AppError::Conflict(format!(
                "璇玑 {} 已达成员上限（{}）",
                input.mox_id, self.config.quotas.max_members
            )));
        }
        // BR-04 邀请幂等：同一璇玑内 email 唯一（大小写不敏感），
        // 避免重复邀请产生多个成员实体导致权限绑定与通知分裂。
        let email_key = input.email.trim().to_lowercase();
        if let Some(existing) = self
            .store
            .list_members(&input.mox_id)
            .await
            .into_iter()
            .find(|m| m.email.trim().to_lowercase() == email_key)
        {
            return Err(AppError::Conflict(format!(
                "邮箱 {} 已在璇玑 {} 中存在成员 {}（状态: {}）",
                input.email,
                input.mox_id,
                existing.id,
                existing.status.label()
            )));
        }
        let m = Member {
            id: new_id("mem"),
            mox_id: input.mox_id.clone(),
            name: input.name.clone(),
            email: input.email.clone(),
            title: input.title.clone(),
            expertise: input.expertise.clone(),
            tier: input.tier.clone(),
            status: MemberStatus::Invited,
            joined_at: chrono::Utc::now(),
        };
        self.store.create_member(m.clone()).await;
        // 璇玑中，受邀参与者默认获得「专家」角色（作用域限定本璇玑）
        let mut bindings = self.store.get_bindings(&m.id).await;
        bindings.push(RoleBinding::mox(Role::Expert, &m.id, &input.mox_id));
        self.store.set_bindings(&m.id, bindings).await;
        self.bus.publish(DomainEvent::MemberInvited {
            member_id: m.id.clone(),
            mox_id: input.mox_id.clone(),
            by: by.to_string(),
        });
        Ok(m)
    }

    pub async fn activate(&self, member_id: &str, _by: &str) -> Result<Member> {
        // 复用状态机校验，保证 Invited → Active 是唯一合法激活路径（BR-21）
        let updated = self.set_status(member_id, MemberStatus::Active).await?;
        self.bus.publish(DomainEvent::MemberActivated {
            member_id: member_id.to_string(),
        });
        Ok(updated)
    }

    /// 成员状态迁移（BR-21）：受状态机约束，非法迁移返回 `InvalidState`
    pub async fn set_status(&self, member_id: &str, status: MemberStatus) -> Result<Member> {
        let before = self
            .store
            .get_member(member_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("成员 {member_id}")))?;
        let from = before.status;
        if from == status {
            // 幂等：重复设置同一状态不视为错误，也不重复发事件
            return Ok(before);
        }
        if !from.can_transition(status) {
            return Err(AppError::InvalidState(format!(
                "成员 {} 非法状态迁移: {} -> {}{}",
                member_id,
                from.label(),
                status.label(),
                if from.is_terminal() {
                    "（已退出为终态，需重新邀请）"
                } else {
                    ""
                }
            )));
        }
        let updated = self
            .store
            .update_member(member_id, |m| m.status = status)
            .await
            .ok_or_else(|| AppError::NotFound(format!("成员 {member_id}")))?;
        self.store
            .append_audit(AuditRecord {
                id: new_id("aud"),
                action: AuditAction::MemberStatusChange,
                actor: member_id.to_string(),
                resource: member_id.to_string(),
                permission: None,
                scope: format!("mox={}", updated.mox_id),
                detail: format!("{} -> {}", from.label(), status.label()),
                at: chrono::Utc::now(),
            })
            .await;
        self.bus.publish(DomainEvent::MemberStatusChanged {
            member_id: member_id.to_string(),
            status: format!("{status:?}"),
        });
        Ok(updated)
    }

    pub async fn update_profile(
        &self,
        member_id: &str,
        title: Option<String>,
        expertise: Option<Vec<String>>,
        tier: Option<Tier>,
    ) -> Result<Member> {
        let updated = self
            .store
            .update_member(member_id, |m| {
                if let Some(t) = title {
                    m.title = t;
                }
                if let Some(e) = expertise {
                    m.expertise = e;
                }
                if let Some(t) = tier {
                    m.tier = t;
                }
            })
            .await
            .ok_or_else(|| AppError::NotFound(format!("成员 {member_id}")))?;
        Ok(updated)
    }

    pub async fn get(&self, id: &str) -> Result<Member> {
        self.store
            .get_member(id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("成员 {id}")))
    }

    pub async fn list(&self, mox_id: &str) -> Vec<Member> {
        self.store.list_members(mox_id).await
    }

    pub async fn search(&self, mox_id: &str, expertise: &str) -> Vec<Member> {
        self.store
            .list_members(mox_id)
            .await
            .into_iter()
            .filter(|m| {
                m.expertise
                    .iter()
                    .any(|e| e.to_lowercase().contains(&expertise.to_lowercase()))
            })
            .collect()
    }
}
