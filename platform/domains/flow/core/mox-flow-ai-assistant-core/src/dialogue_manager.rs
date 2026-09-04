// Copyright (c) 2026 璇玑 RelGraph · AI对话mox 模块化系统架构自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 对话管理器
//!
//! 管理多轮对话、上下文维护、会话状态

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::{AiError, AiResult};
use crate::types::*;

/// 对话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationState {
    /// 空闲
    Idle,
    /// 理解意图中
    Understanding,
    /// 任务执行中
    Executing,
    /// 等待用户确认
    WaitingForConfirmation,
    /// 等待用户输入
    WaitingForInput,
    /// 已完成
    Completed,
    /// 已失败
    Failed,
}

/// 对话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// 对话 ID
    pub id: String,
    /// 用户 ID
    pub user_id: String,
    /// 对话标题
    pub title: String,
    /// 状态
    pub state: ConversationState,
    /// 消息历史
    pub messages: Vec<ConversationMessage>,
    /// 上下文变量
    pub context: HashMap<String, serde_json::Value>,
    /// 关联的任务 ID 列表
    pub task_ids: Vec<String>,
    /// 当前意图
    pub current_intent: Option<IntentType>,
    /// 创建时间
    pub created_at: u64,
    /// 最后活跃时间
    pub last_active_at: u64,
    /// 消息总数
    pub message_count: u64,
}

impl Conversation {
    /// 创建新对话
    pub fn new(user_id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            title: "新对话".to_string(),
            state: ConversationState::Idle,
            messages: Vec::new(),
            context: HashMap::new(),
            task_ids: Vec::new(),
            current_intent: None,
            created_at: now_ms(),
            last_active_at: now_ms(),
            message_count: 0,
        }
    }

    /// 添加消息
    pub fn add_message(&mut self, message: ConversationMessage) {
        self.messages.push(message);
        self.message_count += 1;
        self.last_active_at = now_ms();
    }

    /// 设置上下文变量
    pub fn set_context(&mut self, key: &str, value: serde_json::Value) {
        self.context.insert(key.to_string(), value);
    }

    /// 获取上下文变量
    pub fn get_context(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }

    /// 获取最近 N 条消息
    pub fn recent_messages(&self, n: usize) -> &[ConversationMessage] {
        let len = self.messages.len();
        let start = if n >= len { 0 } else { len - n };
        &self.messages[start..]
    }
}

/// 对话管理器
pub struct DialogueManager {
    /// 对话表
    conversations: RwLock<HashMap<String, Conversation>>,
    /// 用户对话索引：user_id -> Vec<conversation_id>
    user_conversations: RwLock<HashMap<String, Vec<String>>>,
    /// 最大历史消息数
    max_history: usize,
}

impl DialogueManager {
    /// 创建对话管理器
    pub fn new() -> Self {
        Self {
            conversations: RwLock::new(HashMap::new()),
            user_conversations: RwLock::new(HashMap::new()),
            max_history: 100,
        }
    }

    /// 创建新对话
    pub fn create_conversation(&self, user_id: &str) -> Conversation {
        let conv = Conversation::new(user_id);

        self.user_conversations
            .write()
            .entry(user_id.to_string())
            .or_default()
            .push(conv.id.clone());
        self.conversations
            .write()
            .insert(conv.id.clone(), conv.clone());

        conv
    }

    /// 获取对话
    pub fn get_conversation(&self, conversation_id: &str) -> Option<Conversation> {
        self.conversations.read().get(conversation_id).cloned()
    }

    /// 添加用户消息
    pub fn add_user_message(
        &self,
        conversation_id: &str,
        content: &str,
    ) -> AiResult<ConversationMessage> {
        let mut conversations = self.conversations.write();
        let conv = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AiError::NotFound("conversation not found".to_string()))?;

        let msg = ConversationMessage::user(content);
        conv.add_message(msg.clone());
        conv.state = ConversationState::Understanding;

        Ok(msg)
    }

    /// 添加助手消息
    pub fn add_assistant_message(
        &self,
        conversation_id: &str,
        content: &str,
    ) -> AiResult<ConversationMessage> {
        let mut conversations = self.conversations.write();
        let conv = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AiError::NotFound("conversation not found".to_string()))?;

        let msg = ConversationMessage::assistant(content);
        conv.add_message(msg.clone());

        Ok(msg)
    }

    /// 更新对话状态
    pub fn update_state(&self, conversation_id: &str, state: ConversationState) -> AiResult<()> {
        let mut conversations = self.conversations.write();
        let conv = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AiError::NotFound("conversation not found".to_string()))?;

        conv.state = state;
        Ok(())
    }

    /// 设置当前意图
    pub fn set_current_intent(&self, conversation_id: &str, intent: IntentType) -> AiResult<()> {
        let mut conversations = self.conversations.write();
        let conv = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AiError::NotFound("conversation not found".to_string()))?;

        conv.current_intent = Some(intent);
        Ok(())
    }

    /// 关联任务
    pub fn add_task(&self, conversation_id: &str, task_id: &str) -> AiResult<()> {
        let mut conversations = self.conversations.write();
        let conv = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AiError::NotFound("conversation not found".to_string()))?;

        conv.task_ids.push(task_id.to_string());
        Ok(())
    }

    /// 获取用户的所有对话
    pub fn list_user_conversations(&self, user_id: &str) -> Vec<Conversation> {
        let ids = self
            .user_conversations
            .read()
            .get(user_id)
            .cloned()
            .unwrap_or_default();
        let conversations = self.conversations.read();
        let mut result: Vec<Conversation> = ids
            .iter()
            .filter_map(|id| conversations.get(id).cloned())
            .collect();
        // 按最后活跃时间倒序
        result.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
        result
    }

    /// 更新对话标题
    pub fn update_title(&self, conversation_id: &str, title: &str) -> AiResult<()> {
        let mut conversations = self.conversations.write();
        let conv = conversations
            .get_mut(conversation_id)
            .ok_or_else(|| AiError::NotFound("conversation not found".to_string()))?;

        conv.title = title.to_string();
        Ok(())
    }

    /// 对话总数
    pub fn conversation_count(&self) -> usize {
        self.conversations.read().len()
    }

    /// 设置最大历史消息数
    pub fn set_max_history(&mut self, max: usize) {
        self.max_history = max;
    }
}

impl Default for DialogueManager {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> u64 {
    crate::types::now_ms()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_conversation() {
        let manager = DialogueManager::new();
        let conv = manager.create_conversation("user1");
        assert_eq!(conv.user_id, "user1");
        assert_eq!(conv.state, ConversationState::Idle);
        assert_eq!(manager.conversation_count(), 1);
    }

    #[test]
    fn test_add_messages() {
        let manager = DialogueManager::new();
        let conv = manager.create_conversation("user1");

        manager
            .add_user_message(&conv.id, "你好")
            .unwrap();
        manager
            .add_assistant_message(&conv.id, "你好！有什么可以帮你的？")
            .unwrap();

        let conv = manager.get_conversation(&conv.id).unwrap();
        assert_eq!(conv.message_count, 2);
        assert_eq!(conv.messages.len(), 2);
        assert_eq!(conv.messages[0].content, "你好");
        assert_eq!(conv.messages[1].content, "你好！有什么可以帮你的？");
    }

    #[test]
    fn test_update_state() {
        let manager = DialogueManager::new();
        let conv = manager.create_conversation("user1");

        manager
            .update_state(&conv.id, ConversationState::Executing)
            .unwrap();

        let conv = manager.get_conversation(&conv.id).unwrap();
        assert_eq!(conv.state, ConversationState::Executing);
    }

    #[test]
    fn test_set_intent() {
        let manager = DialogueManager::new();
        let conv = manager.create_conversation("user1");

        manager
            .set_current_intent(&conv.id, IntentType::GraphQuery)
            .unwrap();

        let conv = manager.get_conversation(&conv.id).unwrap();
        assert_eq!(conv.current_intent, Some(IntentType::GraphQuery));
    }

    #[test]
    fn test_list_user_conversations() {
        let manager = DialogueManager::new();

        manager.create_conversation("user1");
        manager.create_conversation("user1");
        manager.create_conversation("user2");

        let user1_convs = manager.list_user_conversations("user1");
        assert_eq!(user1_convs.len(), 2);

        let user2_convs = manager.list_user_conversations("user2");
        assert_eq!(user2_convs.len(), 1);
    }

    #[test]
    fn test_context_variables() {
        let manager = DialogueManager::new();
        let conv = manager.create_conversation("user1");

        // 直接修改上下文需要通过 write
        {
            let mut conversations = manager.conversations.write();
            let c = conversations.get_mut(&conv.id).unwrap();
            c.set_context("key", serde_json::json!("value"));
        }

        let conv = manager.get_conversation(&conv.id).unwrap();
        assert_eq!(conv.get_context("key").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn test_recent_messages() {
        let mut conv = Conversation::new("user1");
        for i in 0..5 {
            conv.add_message(ConversationMessage::user(&format!("msg {}", i)));
        }

        let recent = conv.recent_messages(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].content, "msg 2");
    }

    #[test]
    fn test_update_title() {
        let manager = DialogueManager::new();
        let conv = manager.create_conversation("user1");

        manager.update_title(&conv.id, "图谱分析对话").unwrap();

        let conv = manager.get_conversation(&conv.id).unwrap();
        assert_eq!(conv.title, "图谱分析对话");
    }

    #[test]
    fn test_add_task() {
        let manager = DialogueManager::new();
        let conv = manager.create_conversation("user1");

        manager.add_task(&conv.id, "task-123").unwrap();

        let conv = manager.get_conversation(&conv.id).unwrap();
        assert_eq!(conv.task_ids.len(), 1);
        assert_eq!(conv.task_ids[0], "task-123");
    }
}
