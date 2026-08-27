// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! # AI 对话会话管理
//!
//! 提供多轮对话能力，支持：
//! - 多会话并发管理（Session Registry）
//! - 图谱上下文注入（图谱感知对话）
//! - 对话历史管理（自动截断 + token 控制）
//! - 角色预设（System prompt 管理）

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::graph::MoxGraph;
use crate::providers::{AiError, AiProvider, ChatMessage, ChatRequest, ModelConfig, Role};

/// 单条消息
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
    fn to_chat_message(&self) -> ChatMessage {
        ChatMessage {
            role: self.role,
            content: self.content.clone(),
            name: None,
        }
    }
}

/// 对话历史
#[derive(Debug, Clone, Default)]
pub struct ChatHistory {
    messages: Vec<Message>,
    max_turns: usize,
}

impl ChatHistory {
    pub fn new(max_turns: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_turns,
        }
    }

    /// 添加用户消息
    pub fn push_user(&mut self, content: String) {
        self.messages.push(Message::user(content));
        self.trim();
    }

    /// 添加助手消息
    pub fn push_assistant(&mut self, content: String) {
        self.messages.push(Message::assistant(content));
    }

    /// 设置系统提示词
    pub fn set_system(&mut self, content: String) {
        // 删除已有的 system 消息
        self.messages.retain(|m| m.role != Role::System);
        self.messages.insert(0, Message::system(content));
    }

    /// 追加图谱上下文到最后一条 system 消息
    pub fn append_graph_context(&mut self, graph_desc: String) {
        let system_content = if let Some(msg) = self.messages.iter().find(|m| m.role == Role::System)
        {
            format!(
                "{}\n\n## 当前图谱上下文:\n{}",
                msg.content, graph_desc
            )
        } else {
            format!("## 当前图谱上下文:\n{}", graph_desc)
        };
        // 删除旧的 system，加入新的
        self.messages.retain(|m| m.role != Role::System);
        self.messages.insert(0, Message::system(system_content));
    }

    /// 获取最近 N 轮对话（不含 system）
    pub fn recent_turns(&self, n: usize) -> Vec<&Message> {
        let turns: Vec<_> = self
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .rev()
            .take(n * 2) // 每轮 user+assistant
            .collect();
        turns.into_iter().rev().collect()
    }

    /// 转换为 provider 需要的 ChatMessage 列表
    pub fn to_provider_messages(&self) -> Vec<ChatMessage> {
        self.messages.iter().map(|m| m.to_chat_message()).collect()
    }

    fn trim(&mut self) {
        if self.messages.iter().filter(|m| m.role != Role::System).count()
            > self.max_turns * 2
        {
            // 保留 system + 最近 max_turns*2 条
            let system = self
                .messages
                .iter()
                .filter(|m| m.role == Role::System)
                .cloned()
                .collect::<Vec<_>>();
            let non_system: Vec<_> = self
                .messages
                .iter()
                .filter(|m| m.role != Role::System)
                .cloned()
                .collect();
            let recent: Vec<Message> = non_system
                .into_iter()
                .rev()
                .take(self.max_turns * 2)
                .rev()
                .collect();
            self.messages.clear();
            self.messages.extend(system);
            self.messages.extend(recent);
        }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
}

/// 对话会话
#[derive(Debug)]
pub struct ChatSession {
    pub session_id: String,
    pub user_id: String,
    pub history: ChatHistory,
    system_prompt: Option<String>,
}

impl ChatSession {
    pub fn new(session_id: String, user_id: String, max_history_turns: usize) -> Self {
        Self {
            session_id,
            user_id,
            history: ChatHistory::new(max_history_turns),
            system_prompt: None,
        }
    }

    /// 设置系统提示词
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        self.history.set_system(prompt.clone());
        self.system_prompt = Some(prompt);
        self
    }

    /// 注入图谱上下文
    pub fn inject_graph_context(&mut self, graph: &MoxGraph, focus_hint: &str) {
        let desc = describe_graph_for_context(graph, focus_hint);
        self.history.append_graph_context(desc);
    }

    /// 注入自定义图谱描述
    pub fn inject_custom_context(&mut self, context: &str) {
        self.history.append_graph_context(context.to_string());
    }

    /// 发送用户消息，返回助手回复（async）
    pub async fn send<P: AiProvider + ?Sized>(
        &mut self,
        provider: &P,
        model: &str,
    ) -> Result<String, AiError> {
        let messages = self.history.to_provider_messages();
        let config = ModelConfig {
            model: model.into(),
            max_tokens: 4096,
            temperature: 0.7,
            ..Default::default()
        };
        let req = ChatRequest { messages, config };
        let response = provider.chat(&req).await?;
        self.history.push_assistant(response.content.clone());
        Ok(response.content)
    }

    /// 发送消息（自定义配置，async）
    pub async fn send_with<P: AiProvider + ?Sized>(
        &mut self,
        provider: &P,
        config: &ModelConfig,
    ) -> Result<String, AiError> {
        let messages = self.history.to_provider_messages();
        let req = ChatRequest { messages, config: config.clone() };
        let response = provider.chat(&req).await?;
        self.history.push_assistant(response.content.clone());
        Ok(response.content)
    }

    pub fn history(&self) -> &ChatHistory {
        &self.history
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }
}

/// 对话会话注册表（支持多会话并发）
#[derive(Debug, Default)]
pub struct SessionRegistry {
    sessions: RwLock<HashMap<String, Arc<RwLock<ChatSession>>>>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// 创建或获取会话
    pub fn get_or_create(
        &self,
        session_id: &str,
        user_id: &str,
        max_turns: usize,
    ) -> Arc<RwLock<ChatSession>> {
        {
            let sessions = self.sessions.read().unwrap();
            if let Some(s) = sessions.get(session_id) {
                return s.clone();
            }
        }
        let session = Arc::new(RwLock::new(ChatSession::new(
            session_id.into(),
            user_id.into(),
            max_turns,
        )));
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session_id.into(), session.clone());
        session
    }

    /// 删除会话
    pub fn remove(&self, session_id: &str) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(session_id);
    }

    /// 会话数量
    pub fn len(&self) -> usize {
        self.sessions.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.read().unwrap().is_empty()
    }
}

/// 将图谱对象转换为人类可读的上下文描述
pub fn describe_graph_for_context(graph: &MoxGraph, focus_hint: &str) -> String {
    let stats = graph.stats();
    let nodes: Vec<_> = graph.nodes.values().take(20).collect();

    let node_list = nodes
        .iter()
        .map(|n| format!("  - [{}] {}", n.id, n.label))
        .collect::<Vec<_>>()
        .join("\n");

    let edge_list = graph
        .edges
        .values()
        .take(30)
        .map(|e| format!("  - {} --「{}」--> {}", e.from, e.relation_type, e.to))
        .collect::<Vec<_>>()
        .join("\n");

    let focus = if focus_hint.is_empty() {
        "无特定聚焦"
    } else {
        focus_hint
    };
    format!(
        "【图谱概览】节点 {} 个，边 {} 条。\n\
         【聚焦】{}\n\
         【节点】\n{}\n\
         【关系】\n{}",
        stats.node_count, stats.edge_count, focus, node_list, edge_list,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_history_trim() {
        let mut history = ChatHistory::new(2); // 最多 2 轮 = 4 条非 system
        history.set_system("system prompt".into());
        for i in 0..10 {
            history.push_user(format!("user {}", i));
            history.push_assistant(format!("assistant {}", i));
        }
        // system + 5 条非 system（trim 在 push_user 时保留 4 条，随后 push_assistant 加 1 条）
        assert_eq!(history.messages().len(), 6);
        assert_eq!(history.messages()[0].role, Role::System);
    }

    #[test]
    fn test_session_registry() {
        let registry = SessionRegistry::new();
        let s1 = registry.get_or_create("sess-1", "user-1", 10);
        let s1_again = registry.get_or_create("sess-1", "user-1", 10);
        assert!(Arc::ptr_eq(&s1, &s1_again));
        assert_eq!(registry.len(), 1);

        let _s2 = registry.get_or_create("sess-2", "user-2", 10);
        assert_eq!(registry.len(), 2);

        registry.remove("sess-1");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_describe_graph_for_context() {
        let mut graph = MoxGraph::new();
        graph.add_node(crate::graph::GraphNode::new(
            crate::graph::NodeId::new("A"),
            "TypeA",
        ));
        graph.add_node(crate::graph::GraphNode::new(
            crate::graph::NodeId::new("B"),
            "TypeB",
        ));
        graph.add_edge(crate::graph::GraphEdge::new(
            crate::graph::NodeId::new("A"),
            crate::graph::NodeId::new("B"),
            "relates_to",
        ));

        let desc = describe_graph_for_context(&graph, "测试聚焦");
        assert!(desc.contains("节点 2 个"));
        assert!(desc.contains("边 1 条"));
        assert!(desc.contains("测试聚焦"));
        assert!(desc.contains("TypeA"));
        assert!(desc.contains("relates_to"));
    }

    #[test]
    fn test_chat_session_system_prompt() {
        let session = ChatSession::new("s1".into(), "u1".into(), 10)
            .with_system_prompt("你是助手");
        assert_eq!(session.system_prompt(), Some("你是助手"));
        assert_eq!(session.history().messages()[0].role, Role::System);
    }
}
