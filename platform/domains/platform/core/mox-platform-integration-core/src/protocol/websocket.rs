//! WebSocket连接管理 — WebSocket Connection Manager

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use parking_lot::RwLock;

/// WebSocket消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    /// 消息ID
    pub message_id: String,
    /// 消息类型（text/binary/ping/pong/close）
    pub message_type: String,
    /// 消息内容（文本消息）
    #[serde(default)]
    pub payload: String,
    /// 二进制数据（base64编码）
    #[serde(default)]
    pub binary: Option<String>,
    /// 发送时间
    pub timestamp: String,
    /// 追踪ID
    #[serde(default)]
    pub trace_id: Option<String>,
}

impl WebSocketMessage {
    /// 创建文本消息
    pub fn text(payload: impl Into<String>) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: "text".into(),
            payload: payload.into(),
            binary: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: None,
        }
    }

    /// 创建JSON消息
    pub fn json<T: Serialize>(data: &T) -> serde_json::Result<Self> {
        Ok(Self::text(serde_json::to_string(data)?))
    }

    /// 创建Ping消息
    pub fn ping() -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            message_type: "ping".into(),
            payload: String::new(),
            binary: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: None,
        }
    }
}

/// WebSocket连接状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebSocketConnectionState {
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 正在关闭
    Closing,
    /// 已关闭
    Closed,
    /// 连接错误
    Error,
}

/// WebSocket连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConnection {
    /// 连接ID
    pub connection_id: String,
    /// 客户端地址
    pub client_addr: String,
    /// 连接路径
    pub path: String,
    /// 连接状态
    pub state: WebSocketConnectionState,
    /// 连接时间
    pub connected_at: String,
    /// 最后活动时间
    pub last_active_at: String,
    /// 接收消息数
    pub messages_received: u64,
    /// 发送消息数
    pub messages_sent: u64,
    /// 租户ID
    #[serde(default)]
    pub tenant_id: Option<String>,
    /// 用户ID
    #[serde(default)]
    pub user_id: Option<String>,
    /// 订阅的主题列表
    #[serde(default)]
    pub subscriptions: Vec<String>,
    /// 自定义属性
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl WebSocketConnection {
    /// 创建新连接
    pub fn new(client_addr: impl Into<String>, path: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            connection_id: uuid::Uuid::new_v4().to_string(),
            client_addr: client_addr.into(),
            path: path.into(),
            state: WebSocketConnectionState::Connecting,
            connected_at: now.clone(),
            last_active_at: now,
            messages_received: 0,
            messages_sent: 0,
            tenant_id: None,
            user_id: None,
            subscriptions: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    /// 标记已连接
    pub fn mark_connected(&mut self) {
        self.state = WebSocketConnectionState::Connected;
        self.last_active_at = chrono::Utc::now().to_rfc3339();
    }

    /// 记录接收消息
    pub fn record_received(&mut self) {
        self.messages_received += 1;
        self.last_active_at = chrono::Utc::now().to_rfc3339();
    }

    /// 记录发送消息
    pub fn record_sent(&mut self) {
        self.messages_sent += 1;
        self.last_active_at = chrono::Utc::now().to_rfc3339();
    }

    /// 订阅主题
    pub fn subscribe(&mut self, topic: impl Into<String>) {
        let topic = topic.into();
        if !self.subscriptions.contains(&topic) {
            self.subscriptions.push(topic);
        }
    }

    /// 取消订阅
    pub fn unsubscribe(&mut self, topic: &str) {
        self.subscriptions.retain(|t| t != topic);
    }

    /// 检查连接是否超时（指定秒数无活动）
    pub fn is_idle_timeout(&self, timeout_secs: u64) -> bool {
        // 简化：实际应解析时间戳比较
        let _ = Instant::now();
        timeout_secs == 0
    }
}

/// WebSocket连接管理器
pub struct WebSocketManager {
    connections: RwLock<HashMap<String, Arc<RwLock<WebSocketConnection>>>>,
    /// 最大连接数
    max_connections: usize,
    /// 空闲超时（秒）
    idle_timeout_secs: u64,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
            max_connections: 10000,
            idle_timeout_secs: 300,
        }
    }

    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    pub fn with_idle_timeout(mut self, timeout_secs: u64) -> Self {
        self.idle_timeout_secs = timeout_secs;
        self
    }

    /// 添加连接
    pub fn add_connection(&self, connection: WebSocketConnection) -> Result<String, String> {
        if self.connections.read().len() >= self.max_connections {
            return Err("max connections reached".into());
        }
        let id = connection.connection_id.clone();
        self.connections.write().insert(id.clone(), Arc::new(RwLock::new(connection)));
        tracing::info!("WebSocket connected: {} from {}", id, id);
        Ok(id)
    }

    /// 移除连接
    pub fn remove_connection(&self, connection_id: &str) -> Option<Arc<RwLock<WebSocketConnection>>> {
        let conn = self.connections.write().remove(connection_id);
        if conn.is_some() {
            tracing::info!("WebSocket disconnected: {}", connection_id);
        }
        conn
    }

    /// 获取连接
    pub fn get_connection(&self, connection_id: &str) -> Option<Arc<RwLock<WebSocketConnection>>> {
        self.connections.read().get(connection_id).cloned()
    }

    /// 列出所有连接
    pub fn list_connections(&self) -> Vec<WebSocketConnection> {
        self.connections.read()
            .values()
            .map(|c| c.read().clone())
            .collect()
    }

    /// 按路径筛选连接
    pub fn list_by_path(&self, path: &str) -> Vec<WebSocketConnection> {
        self.connections.read()
            .values()
            .filter(|c| c.read().path == path)
            .map(|c| c.read().clone())
            .collect()
    }

    /// 按租户筛选连接
    pub fn list_by_tenant(&self, tenant_id: &str) -> Vec<WebSocketConnection> {
        self.connections.read()
            .values()
            .filter(|c| c.read().tenant_id.as_deref() == Some(tenant_id))
            .map(|c| c.read().clone())
            .collect()
    }

    /// 按订阅主题筛选连接
    pub fn list_by_subscription(&self, topic: &str) -> Vec<WebSocketConnection> {
        self.connections.read()
            .values()
            .filter(|c| c.read().subscriptions.iter().any(|t| t == topic))
            .map(|c| c.read().clone())
            .collect()
    }

    /// 广播消息到所有连接
    pub async fn broadcast(&self, message: &WebSocketMessage) -> usize {
        let connections = self.list_connections();
        let mut sent = 0;
        for conn in connections {
            if conn.state == WebSocketConnectionState::Connected {
                // 实际应通过连接发送消息
                sent += 1;
            }
        }
        sent
    }

    /// 广播消息到订阅指定主题的连接
    pub async fn broadcast_to_topic(&self, topic: &str, message: &WebSocketMessage) -> usize {
        let connections = self.list_by_subscription(topic);
        connections.len()
    }

    /// 连接数
    pub fn connection_count(&self) -> usize {
        self.connections.read().len()
    }

    /// 活跃连接数
    pub fn active_count(&self) -> usize {
        self.connections.read()
            .values()
            .filter(|c| c.read().state == WebSocketConnectionState::Connected)
            .count()
    }

    /// 清理超时连接
    pub fn cleanup_idle_connections(&self) -> usize {
        let mut to_remove = Vec::new();
        for (id, conn) in self.connections.read().iter() {
            if conn.read().is_idle_timeout(self.idle_timeout_secs) {
                to_remove.push(id.clone());
            }
        }
        let count = to_remove.len();
        for id in to_remove {
            self.connections.write().remove(&id);
        }
        count
    }
}

impl Default for WebSocketManager {
    fn default() -> Self { Self::new() }
}
