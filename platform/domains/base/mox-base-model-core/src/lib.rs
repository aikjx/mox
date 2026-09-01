//! MOX 统一基座 · 统一模型层
//!
//! 定义知识图谱 / 云盘 / 知识库 / 对象存储共用的统一数据模型原语：
//! - [`Node`]：图谱节点（实体 / 文档 / 目录 / 对象均可建模为 Node）
//! - [`Edge`]：关系边（引用 / 包含 / 语义关联）
//! - [`Blob`]：物理二进制对象（大对象走 RANGE 直达流式通道，不因图谱化牺牲吞吐）
//! - [`Id`]：统一 ID 空间（域前缀 + UUID，跨域可路由）
//!
//! ## 设计原则
//! - **只定义模型，不内置后端**：本 crate 是纯数据契约，不依赖任何存储实现。
//! - 各域（kg/cloud/data/ai/flow）的模型对齐/复用本 crate 的 Node/Edge/Blob，
//!   不再自建存储模型（消除多套 unified 并存的 God Module 反模式）。
//! - 依赖方向单向：域 → mox-base-model-core。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 统一 ID：域前缀 + UUID，跨域可路由
///
/// 例：`kg:node:550e8400-e29b-41d4-a716-446655440000`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id {
    /// 域前缀（kg / cloud / data / ai / flow / base）
    pub domain: String,
    /// 实体类型（node / edge / blob）
    pub kind: EntityKind,
    /// UUID
    pub uuid: Uuid,
}

/// 实体类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    /// 节点（实体 / 文档 / 目录 / 对象）
    Node,
    /// 关系边
    Edge,
    /// 物理二进制对象
    Blob,
}

impl Id {
    /// 构造统一 ID
    pub fn new(domain: impl Into<String>, kind: EntityKind) -> Self {
        Self {
            domain: domain.into(),
            kind,
            uuid: Uuid::new_v4(),
        }
    }

    /// 路由键：域 + 类型，用于分区 / 寻址
    pub fn routing_key(&self) -> String {
        format!("{}:{}", self.domain, match self.kind {
            EntityKind::Node => "node",
            EntityKind::Edge => "edge",
            EntityKind::Blob => "blob",
        })
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.domain, self.kind_label(), self.uuid)
    }
}

impl Id {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            EntityKind::Node => "node",
            EntityKind::Edge => "edge",
            EntityKind::Blob => "blob",
        }
    }
}

/// 统一节点：图谱中的实体 / 文档 / 目录 / 对象统一建模
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// 统一 ID
    pub id: Id,
    /// 节点类型（如 expert / document / directory / object / module）
    pub node_type: String,
    /// 属性（统一 props，schema 由 data 域 mox-data-standards-core 注入约束）
    pub props: HashMap<String, serde_json::Value>,
    /// 创建时间（epoch ms）
    pub created_at_ms: u64,
    /// 更新时间（epoch ms）
    pub updated_at_ms: u64,
    /// 版本号（生命周期管理，配合 mox-base-lifecycle-core）
    pub version: u64,
}

impl Node {
    /// 构造节点（自动生成 ID 与时间戳）
    pub fn new(domain: impl Into<String>, node_type: impl Into<String>) -> Self {
        let now = now_ms();
        Self {
            id: Id::new(domain, EntityKind::Node),
            node_type: node_type.into(),
            props: HashMap::new(),
            created_at_ms: now,
            updated_at_ms: now,
            version: 1,
        }
    }

    /// 设置属性（保持 API 简洁）
    pub fn with_prop(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.props.insert(key.into(), value);
        self
    }
}

/// 统一关系边：引用 / 包含 / 语义关联
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// 边 ID
    pub id: Id,
    /// 边类型（reference / contains / semantic）
    pub edge_type: String,
    /// 起点节点 ID
    pub from: Id,
    /// 终点节点 ID
    pub to: Id,
    /// 边属性（权重 / 置信度 / 关系说明）
    pub props: HashMap<String, serde_json::Value>,
    /// 创建时间（epoch ms）
    pub created_at_ms: u64,
}

impl Edge {
    /// 构造边
    pub fn new(domain: impl Into<String>, edge_type: impl Into<String>, from: Id, to: Id) -> Self {
        Self {
            id: Id::new(domain, EntityKind::Edge),
            edge_type: edge_type.into(),
            from,
            to,
            props: HashMap::new(),
            created_at_ms: now_ms(),
        }
    }
}

/// 统一物理二进制对象（大对象走 RANGE 直达流式通道）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blob {
    /// Blob ID
    pub id: Id,
    /// 逻辑路径（如 `kg/expert/xxx/avatar.png`）
    pub path: String,
    /// 内容类型（MIME）
    pub content_type: String,
    /// 字节大小
    pub size_bytes: u64,
    /// 内容寻址哈希（SHA-256 hex，用于去重，配合 mox-base-lifecycle-core）
    pub sha256: Option<String>,
    /// 创建时间（epoch ms）
    pub created_at_ms: u64,
}

impl Blob {
    /// 构造 Blob 元数据
    pub fn new(domain: impl Into<String>, path: impl Into<String>, content_type: impl Into<String>) -> Self {
        Self {
            id: Id::new(domain, EntityKind::Blob),
            path: path.into(),
            content_type: content_type.into(),
            size_bytes: 0,
            sha256: None,
            created_at_ms: now_ms(),
        }
    }
}

/// 当前时间（epoch ms）
fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_routing_key_works() {
        let id = Id::new("kg", EntityKind::Node);
        assert_eq!(id.routing_key(), "kg:node");
        assert!(id.uuid.to_string().len() == 36);
    }

    #[test]
    fn node_uniform_model_works() {
        let n = Node::new("kg", "expert")
            .with_prop("name", serde_json::json!("张三"));
        assert_eq!(n.node_type, "expert");
        assert_eq!(n.props["name"], serde_json::json!("张三"));
        assert_eq!(n.version, 1);
        assert!(n.created_at_ms > 0);
    }

    #[test]
    fn edge_links_two_nodes() {
        let a = Node::new("kg", "expert");
        let b = Node::new("kg", "module");
        let e = Edge::new("kg", "contains", a.id.clone(), b.id.clone());
        assert_eq!(e.edge_type, "contains");
        assert_eq!(e.from, a.id);
        assert_eq!(e.to, b.id);
    }

    #[test]
    fn blob_metadata_works() {
        let b = Blob::new("cloud", "kg/expert/avatar.png", "image/png");
        assert_eq!(b.content_type, "image/png");
        assert_eq!(b.size_bytes, 0);
        assert!(b.sha256.is_none());
    }
}
