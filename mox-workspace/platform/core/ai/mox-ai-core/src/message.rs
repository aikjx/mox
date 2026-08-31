//! 消息类型
//!
//! 对话消息、工具调用消息等

use serde::{Deserialize, Serialize};
use crate::Role;

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// 角色
    pub role: Role,
    /// 内容
    pub content: String,
    /// 工具调用 ID（当 role=tool 时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// 工具调用列表（当 role=assistant 时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 完成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletion {
    pub message: ChatMessage,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: String,
}

/// Token 使用量
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
