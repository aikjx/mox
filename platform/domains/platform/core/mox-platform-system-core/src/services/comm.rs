// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 通信机制服务
//!
//! 负责频道管理、消息发送、通知等领域逻辑。

use std::sync::Arc;

use crate::error::*;
use crate::event::{DomainEvent, EventBus};
use crate::model::*;
use crate::store::Store;

#[derive(Clone)]
pub struct CommService {
    pub store: Arc<Store>,
    pub bus: EventBus,
}

impl CommService {
    pub fn new(store: Arc<Store>, bus: EventBus) -> Self {
        Self { store, bus }
    }

    pub async fn create_channel(
        &self,
        mox_id: &str,
        kind: ChannelKind,
        name: &str,
        members: Vec<String>,
    ) -> Channel {
        let ch = Channel {
            id: new_id("chan"),
            mox_id: mox_id.to_string(),
            kind,
            name: name.to_string(),
            members,
        };
        self.store.create_channel(ch.clone()).await;
        ch
    }

    pub async fn send_message(
        &self,
        channel_id: &str,
        sender_id: &str,
        body: &str,
        kind: MessageKind,
    ) -> Result<Message> {
        let ch = self
            .store
            .get_channel(channel_id)
            .await
            .ok_or_else(|| AppError::NotFound(format!("频道 {channel_id}")))?;
        // 私信/任务频道要求发送者是频道成员（系统消息除外）
        if kind == MessageKind::Chat && !ch.members.contains(&sender_id.to_string()) {
            return Err(AppError::Forbidden(format!(
                "发送者 {sender_id} 不在频道 {} 中",
                ch.name
            )));
        }
        let msg = Message {
            id: new_id("msg"),
            channel_id: channel_id.to_string(),
            sender_id: sender_id.to_string(),
            body: body.to_string(),
            kind,
            created_at: chrono::Utc::now(),
        };
        self.store.add_message(msg.clone()).await;
        self.bus.publish(DomainEvent::MessagePosted {
            channel_id: channel_id.to_string(),
            message_id: msg.id.clone(),
            sender_id: sender_id.to_string(),
        });
        Ok(msg)
    }

    pub async fn list_messages(&self, channel_id: &str) -> Vec<Message> {
        let mut v = self.store.list_messages(channel_id).await;
        v.sort_by_key(|a| a.created_at);
        v
    }

    /// 生成一条成员通知（供编排层反应器调用）
    pub async fn notify(
        &self,
        member_id: &str,
        title: &str,
        body: &str,
        related_task: Option<&str>,
    ) {
        let n = Notification {
            id: new_id("ntf"),
            member_id: member_id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            read: false,
            related_task: related_task.map(|s| s.to_string()),
            created_at: chrono::Utc::now(),
        };
        self.store.add_notification(n).await;
    }

    pub async fn list_notifications(&self, member_id: &str) -> Vec<Notification> {
        self.store.list_notifications(member_id).await
    }

    pub async fn mark_read(&self, id: &str, member_id: &str) -> Result<()> {
        if self.store.mark_notification_read(id, member_id).await {
            Ok(())
        } else {
            Err(AppError::NotFound(format!("通知 {id}")))
        }
    }
}
