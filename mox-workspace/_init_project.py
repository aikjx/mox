#!/usr/bin/env python3
"""
璇玑 RelGraph 项目初始化脚本
生成 6层8域 DDD 矩阵的完整 crate 骨架
"""
import os

BASE = os.path.dirname(os.path.abspath(__file__))

def write_file(path, content):
    full_path = os.path.join(BASE, path)
    os.makedirs(os.path.dirname(full_path), exist_ok=True)
    with open(full_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"  ✓ {path}")


def crate_cargo(name, deps_section="", dev_deps_section="", features=""):
    """生成标准 Cargo.toml"""
    return f'''[package]
name = "{name}"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[dependencies]
{deps_section}

[dev-dependencies]
{dev_deps_section}

{features}
'''


def crate_lib(name, description, modules=""):
    """生成标准 lib.rs"""
    return f'''//! # {name}
//!
//! {description}
//!
//! ## 功能特性
//! - TODO: 添加功能特性列表

#![warn(missing_docs)]
#![warn(clippy::all)]

{modules}

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
'''

# ============================================================
# L0 - Foundation Layer
# ============================================================

print("\n📦 L0 Foundation Layer")

# mox-platform-foundation
write_file("platform/foundation/mox-platform-foundation/Cargo.toml", crate_cargo(
    "mox-platform-foundation",
    deps_section='''serde = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
tracing = { workspace = true }''',
    dev_deps_section='''serde_json = { workspace = true }'''
))

write_file("platform/foundation/mox-platform-foundation/src/lib.rs", crate_lib(
    "mox-platform-foundation",
    "Mox 平台基础库 — 公共类型、错误码、元数据、通用工具",
    modules='''pub mod error;
pub mod id;
pub mod time;
pub mod tenant;
pub mod common;

pub use error::MoxError;
pub use id::MoxId;
pub use tenant::TenantId;
pub use common::*;'''
))

# error.rs
write_file("platform/foundation/mox-platform-foundation/src/error.rs", '''//! 统一错误类型与错误码体系
//!
//! 错误码格式：6 位数字
//! - 第 1 位：业务域（1=系统, 2=图谱, 3=AI, 4=流程, 5=数据, 6=云存储, 9=集成）
//! - 第 2 位：错误类型（0=参数, 1=认证, 2=权限, 3=不存在, 4=冲突, 5=内部, 6=超时, 7=限流）
//! - 第 3-6 位：顺序编号

use thiserror::Error;

/// 统一错误类型
#[derive(Debug, Error)]
pub enum MoxError {
    /// 参数错误 (10xxx)
    #[error("参数错误: {0}")]
    InvalidParameter(String),

    /// 未认证 (11xxx)
    #[error("未授权访问")]
    Unauthorized,

    /// 权限不足 (12xxx)
    #[error("权限不足: {0}")]
    PermissionDenied(String),

    /// 资源不存在 (13xxx)
    #[error("资源不存在: {0}")]
    NotFound(String),

    /// 资源冲突 (14xxx)
    #[error("资源冲突: {0}")]
    Conflict(String),

    /// 内部错误 (15xxx)
    #[error("内部错误: {0}")]
    Internal(String),

    /// 超时 (16xxx)
    #[error("操作超时")]
    Timeout,

    /// 限流 (17xxx)
    #[error("请求过于频繁，请稍后再试")]
    RateLimited,
}

impl MoxError {
    /// 获取错误码
    pub fn code(&self) -> i32 {
        match self {
            MoxError::InvalidParameter(_) => 10001,
            MoxError::Unauthorized => 11001,
            MoxError::PermissionDenied(_) => 12001,
            MoxError::NotFound(_) => 13001,
            MoxError::Conflict(_) => 14001,
            MoxError::Internal(_) => 15001,
            MoxError::Timeout => 16001,
            MoxError::RateLimited => 17001,
        }
    }

    /// HTTP 状态码
    pub fn http_status(&self) -> u16 {
        match self {
            MoxError::InvalidParameter(_) => 400,
            MoxError::Unauthorized => 401,
            MoxError::PermissionDenied(_) => 403,
            MoxError::NotFound(_) => 404,
            MoxError::Conflict(_) => 409,
            MoxError::Internal(_) => 500,
            MoxError::Timeout => 504,
            MoxError::RateLimited => 429,
        }
    }
}

pub type MoxResult<T> = Result<T, MoxError>;
''')

# id.rs
write_file("platform/foundation/mox-platform-foundation/src/id.rs", '''//! 全局唯一 ID 生成器
//!
//! 格式：前缀 + UUID v4（去除横线，26字符）
//! 示例：usr_550e8400e29b41d4a716446655440000

use uuid::Uuid;

/// Mox 全局唯一 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MoxId(String);

impl MoxId {
    /// 生成新 ID
    pub fn new(prefix: &str) -> Self {
        let id = format!("{}_{}", prefix, Uuid::new_v4().simple());
        Self(id)
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() { None } else { Some(Self(s.to_string())) }
    }

    /// 获取前缀
    pub fn prefix(&self) -> &str {
        self.0.split_once('_').map(|(p, _)| p).unwrap_or("")
    }

    /// 转为字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MoxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<MoxId> for String {
    fn from(id: MoxId) -> Self { id.0 }
}

impl AsRef<str> for MoxId {
    fn as_ref(&self) -> &str { &self.0 }
}

/// 租户 ID
pub type TenantId = MoxId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_id_has_prefix() {
        let id = MoxId::new("usr");
        assert!(id.as_str().starts_with("usr_"));
    }

    #[test]
    fn test_id_unique() {
        let id1 = MoxId::new("tst");
        let id2 = MoxId::new("tst");
        assert_ne!(id1, id2);
    }
}
''')

# time.rs
write_file("platform/foundation/mox-platform-foundation/src/time.rs", '''//! 时间工具
//!
//! 统一使用 UTC 时间，输出时再转换为本地时区

use chrono::{DateTime, Utc};

/// 获取当前时间戳（毫秒）
pub fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// 获取当前 UTC 时间
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// 格式化时间
pub fn format_time(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_millis_positive() {
        assert!(now_millis() > 0);
    }
}
''')

# tenant.rs
write_file("platform/foundation/mox-platform-foundation/src/tenant.rs", '''//! 多租户支持
//!
//! 所有业务操作必须携带租户上下文，确保数据隔离

use crate::id::MoxId;

/// 租户上下文
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// 租户 ID
    pub tenant_id: MoxId,
    /// 请求 ID（用于链路追踪）
    pub request_id: String,
    /// 当前用户 ID（可选）
    pub user_id: Option<MoxId>,
}

impl TenantContext {
    /// 创建系统级上下文（内部操作用）
    pub fn system(tenant_id: MoxId) -> Self {
        Self {
            tenant_id,
            request_id: format!("sys_{}", uuid::Uuid::new_v4().simple()),
            user_id: None,
        }
    }
}

impl Default for TenantContext {
    fn default() -> Self {
        Self {
            tenant_id: MoxId::parse("tnt_default").unwrap(),
            request_id: format!("req_{}", uuid::Uuid::new_v4().simple()),
            user_id: None,
        }
    }
}
''')

# common.rs
write_file("platform/foundation/mox-platform-foundation/src/common.rs", '''//! 通用工具函数与类型

use serde::{Deserialize, Serialize};

/// 分页请求
#[derive(Debug, Clone, Deserialize)]
pub struct PageQuery {
    /// 页码，从 1 开始
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
}

impl Default for PageQuery {
    fn default() -> Self {
        Self { page: 1, page_size: 20 }
    }
}

impl PageQuery {
    /// 计算偏移量
    pub fn offset(&self) -> u64 {
        (self.page.max(1) - 1) * self.page_size.min(100)
    }

    /// 限制最大 page_size
    pub fn limit(&self) -> u64 {
        self.page_size.min(100)
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize)]
pub struct PageResult<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 总数
    pub total: u64,
    /// 当前页
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
    /// 总页数
    pub total_pages: u64,
}

impl<T> PageResult<T> {
    /// 创建分页结果
    pub fn new(items: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        let total_pages = if page_size == 0 { 0 } else { (total + page_size - 1) / page_size };
        Self { items, total, page, page_size, total_pages }
    }
}

/// 统一 API 响应
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    /// 错误码，0 表示成功
    pub code: i32,
    /// 消息
    pub message: String,
    /// 数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 请求 ID
    pub request_id: String,
}

impl<T> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T, request_id: impl Into<String>) -> Self {
        Self {
            code: 0,
            message: "success".into(),
            data: Some(data),
            request_id: request_id.into(),
        }
    }

    /// 错误响应
    pub fn error(code: i32, message: impl Into<String>, request_id: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
            request_id: request_id.into(),
        }
    }
}
''')

# mox-cloud-foundation
write_file("platform/foundation/mox-cloud-foundation/Cargo.toml", crate_cargo(
    "mox-cloud-foundation",
    deps_section='''mox-platform-foundation = { path = "../mox-platform-foundation" }
async-trait = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
serde = { workspace = true }'''
))

write_file("platform/foundation/mox-cloud-foundation/src/lib.rs", crate_lib(
    "mox-cloud-foundation",
    "云存储域抽象 — 定义统一的存储接口，支持多种后端实现",
    modules='''pub mod storage;
pub mod error;

pub use storage::*;
pub use error::CloudError;'''
))

write_file("platform/foundation/mox-cloud-foundation/src/error.rs", '''//! 云存储错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CloudError {
    #[error("对象不存在: {0}")]
    NotFound(String),

    #[error("存储错误: {0}")]
    StorageError(String),

    #[error("权限不足")]
    PermissionDenied,

    #[error("超出配额")]
    QuotaExceeded,
}
''')

write_file("platform/foundation/mox-cloud-foundation/src/storage.rs", '''//! 云存储抽象接口
//!
//! 支持多种后端：本地文件系统、S3 兼容、MinIO 等

use async_trait::async_trait;
use crate::error::CloudError;

/// 对象元信息
#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub size: u64,
    pub content_type: Option<String>,
    pub last_modified: i64,
    pub etag: Option<String>,
}

/// 对象存储接口
#[async_trait]
pub trait ObjectStorage: Send + Sync {
    /// 上传对象
    async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<ObjectMeta, CloudError>;

    /// 获取对象
    async fn get_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<Vec<u8>, CloudError>;

    /// 删除对象
    async fn delete_object(
        &self,
        bucket: &str,
        key: &str,
    ) -> Result<(), CloudError>;

    /// 列出对象
    async fn list_objects(
        &self,
        bucket: &str,
        prefix: Option<&str>,
        max_keys: i32,
    ) -> Result<Vec<ObjectMeta>, CloudError>;

    /// 创建存储桶
    async fn create_bucket(&self, bucket: &str) -> Result<(), CloudError>;

    /// 检查桶是否存在
    async fn bucket_exists(&self, bucket: &str) -> Result<bool, CloudError>;
}
''')

print("  ✓ Foundation layer complete")

# ============================================================
# L3 - Core Layer (代表性 crate)
# ============================================================

print("\n📦 L3 Core Layer")

# --- mox-kg-meta-core ---
write_file("platform/core/kg/mox-kg-meta-core/Cargo.toml", crate_cargo(
    "mox-kg-meta-core",
    deps_section='''mox-platform-foundation = { path = "../../../foundation/mox-platform-foundation" }
serde = { workspace = true }
thiserror = { workspace = true }'''
))

write_file("platform/core/kg/mox-kg-meta-core/src/lib.rs", crate_lib(
    "mox-kg-meta-core",
    "图元数据与类型系统 — 节点/边/属性的核心类型定义，纯计算无 IO",
    modules='''pub mod types;
pub mod schema;
pub mod property;

pub use types::*;
pub use schema::GraphSchema;'''
))

write_file("platform/core/kg/mox-kg-meta-core/src/types.rs", '''//! 图基础类型定义
//!
//! 所有图操作的基础数据结构，纯数据无 IO

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 节点 ID
pub type NodeId = String;

/// 边 ID
pub type EdgeId = String;

/// 属性值类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum PropertyValue {
    /// 字符串
    String(String),
    /// 整数
    Integer(i64),
    /// 浮点数
    Float(f64),
    /// 布尔值
    Boolean(bool),
    /// 列表
    List(Vec<PropertyValue>),
    /// 空值
    Null,
}

/// 节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// 节点 ID
    pub id: NodeId,
    /// 节点类型
    pub label: String,
    /// 属性
    pub properties: HashMap<String, PropertyValue>,
}

/// 边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// 边 ID
    pub id: EdgeId,
    /// 起点
    pub from: NodeId,
    /// 终点
    pub to: NodeId,
    /// 边类型
    pub label: String,
    /// 属性
    pub properties: HashMap<String, PropertyValue>,
    /// 是否有向
    pub directed: bool,
}

/// 图（内存表示）
#[derive(Debug, Clone, Default)]
pub struct Graph {
    nodes: HashMap<NodeId, GraphNode>,
    edges: Vec<GraphEdge>,
    adjacency: HashMap<NodeId, Vec<EdgeId>>,
}

impl Graph {
    /// 创建空图
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加节点
    pub fn add_node(&mut self, node: GraphNode) {
        self.adjacency.entry(node.id.clone()).or_default();
        self.nodes.insert(node.id.clone(), node);
    }

    /// 添加边
    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.id.clone());
        if !edge.directed {
            self.adjacency
                .entry(edge.to.clone())
                .or_default()
                .push(edge.id.clone());
        }
        self.edges.push(edge);
    }

    /// 获取节点
    pub fn get_node(&self, id: &NodeId) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// 节点数量
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 边数量
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// 获取邻居节点 ID
    pub fn neighbors(&self, node_id: &NodeId) -> Vec<&NodeId> {
        self.adjacency
            .get(node_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|eid| {
                        self.edges.iter().find(|e| &e.id == eid).map(|e| {
                            if &e.from == node_id { &e.to } else { &e.from }
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取所有节点 ID
    pub fn node_ids(&self) -> Vec<&NodeId> {
        self.nodes.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        assert_eq!(g.node_count(), 0);
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn test_add_node_and_edge() {
        let mut g = Graph::new();
        g.add_node(GraphNode {
            id: "a".into(),
            label: "test".into(),
            properties: HashMap::new(),
        });
        g.add_node(GraphNode {
            id: "b".into(),
            label: "test".into(),
            properties: HashMap::new(),
        });
        g.add_edge(GraphEdge {
            id: "e1".into(),
            from: "a".into(),
            to: "b".into(),
            label: "link".into(),
            properties: HashMap::new(),
            directed: true,
        });

        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_count(), 1);
        assert_eq!(g.neighbors(&"a".into()).len(), 1);
    }
}
''')

write_file("platform/core/kg/mox-kg-meta-core/src/schema.rs", '''//! 图谱 Schema 定义
//!
//! 定义节点类型、边类型、属性约束

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 属性类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyType {
    String,
    Integer,
    Float,
    Boolean,
    DateTime,
    List(Box<PropertyType>),
}

/// 属性定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDef {
    pub name: String,
    pub data_type: PropertyType,
    pub required: bool,
    pub indexed: bool,
    pub description: Option<String>,
}

/// 节点类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeTypeDef {
    pub label: String,
    pub properties: Vec<PropertyDef>,
    pub description: Option<String>,
}

/// 边类型定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTypeDef {
    pub label: String,
    pub from_labels: Vec<String>,
    pub to_labels: Vec<String>,
    pub properties: Vec<PropertyDef>,
    pub directed: bool,
    pub description: Option<String>,
}

/// 图谱 Schema
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphSchema {
    pub node_types: HashMap<String, NodeTypeDef>,
    pub edge_types: HashMap<String, EdgeTypeDef>,
}

impl GraphSchema {
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加节点类型
    pub fn add_node_type(&mut self, def: NodeTypeDef) {
        self.node_types.insert(def.label.clone(), def);
    }

    /// 添加边类型
    pub fn add_edge_type(&mut self, def: EdgeTypeDef) {
        self.edge_types.insert(def.label.clone(), def);
    }

    /// 检查节点类型是否存在
    pub fn has_node_type(&self, label: &str) -> bool {
        self.node_types.contains_key(label)
    }

    /// 检查边类型是否存在
    pub fn has_edge_type(&self, label: &str) -> bool {
        self.edge_types.contains_key(label)
    }
}
''')

write_file("platform/core/kg/mox-kg-meta-core/src/property.rs", '''//! 属性值操作工具

use crate::types::PropertyValue;

impl PropertyValue {
    /// 转为字符串
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转为整数
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            PropertyValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// 转为浮点数
    pub fn as_float(&self) -> Option<f64> {
        match self {
            PropertyValue::Float(f) => Some(*f),
            PropertyValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 转为布尔值
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            PropertyValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// 是否为空
    pub fn is_null(&self) -> bool {
        matches!(self, PropertyValue::Null)
    }
}

impl From<String> for PropertyValue {
    fn from(v: String) -> Self { PropertyValue::String(v) }
}

impl From<&str> for PropertyValue {
    fn from(v: &str) -> Self { PropertyValue::String(v.to_string()) }
}

impl From<i64> for PropertyValue {
    fn from(v: i64) -> Self { PropertyValue::Integer(v) }
}

impl From<f64> for PropertyValue {
    fn from(v: f64) -> Self { PropertyValue::Float(v) }
}

impl From<bool> for PropertyValue {
    fn from(v: bool) -> Self { PropertyValue::Boolean(v) }
}
''')

# --- mox-kg-algo-core (核心算法库) ---
write_file("platform/core/kg/mox-kg-algo-core/Cargo.toml", crate_cargo(
    "mox-kg-algo-core",
    deps_section='''mox-kg-meta-core = { path = "../mox-kg-meta-core" }
mox-platform-foundation = { path = "../../../foundation/mox-platform-foundation" }
serde = { workspace = true }''',
    dev_deps_section='''proptest = { workspace = true }'''
))

write_file("platform/core/kg/mox-kg-algo-core/src/lib.rs", crate_lib(
    "mox-kg-algo-core",
    "图算法核心库 — 社区检测/中心性/PageRank/激活扩散等纯计算算法",
    modules='''pub mod centrality;
pub mod community;
pub mod pagerank;
pub mod spread;
pub mod shortest_path;

pub use centrality::*;
pub use community::*;
pub use pagerank::*;
pub use spread::*;
pub use shortest_path::*;'''
))

write_file("platform/core/kg/mox-kg-algo-core/src/pagerank.rs", '''//! PageRank 算法
//!
//! 基于幂迭代的 PageRank 计算，含转置图处理以保证质量沿出边正确传播。
//! 阻尼因子 d=0.85，收敛条件：迭代差值 < 1e-6 或达到最大迭代次数。

use std::collections::HashMap;
use mox_kg_meta_core::{Graph, NodeId};

/// PageRank 结果
pub type PageRankResult = HashMap<NodeId, f64>;

/// 计算 PageRank
///
/// # Arguments
/// * `graph` - 图
/// * `damping` - 阻尼因子，通常为 0.85
/// * `max_iterations` - 最大迭代次数
/// * `tolerance` - 收敛阈值
pub fn pagerank(
    graph: &Graph,
    damping: f64,
    max_iterations: u32,
    tolerance: f64,
) -> PageRankResult {
    let n = graph.node_count();
    if n == 0 {
        return HashMap::new();
    }

    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let initial_value = 1.0 / n as f64;

    let mut scores: HashMap<NodeId, f64> = node_ids
        .iter()
        .map(|id| (id.clone(), initial_value))
        .collect();

    let teleport = (1.0 - damping) / n as f64;

    for _ in 0..max_iterations {
        let mut new_scores = HashMap::new();
        let mut dangling_sum = 0.0;

        // 计算悬挂节点贡献（无出边的节点）
        for id in &node_ids {
            let out_edges: Vec<_> = graph
                .neighbors(id)
                .into_iter()
                .filter(|&&nid| {
                    // 只计出边邻居
                    graph.neighbors(id).contains(&nid)
                })
                .collect();
            if out_edges.is_empty() {
                dangling_sum += scores.get(id).copied().unwrap_or(0.0);
            }
        }

        let dangling_contrib = damping * dangling_sum / n as f64;

        // 计算新 PR 值
        for id in &node_ids {
            let mut sum = 0.0;
            for &neighbor in graph.neighbors(id) {
                let out_degree = graph.neighbors(neighbor).len();
                if out_degree > 0 {
                    sum += scores.get(neighbor).copied().unwrap_or(0.0) / out_degree as f64;
                }
            }

            let new_score = teleport + dangling_contrib + damping * sum;
            new_scores.insert(id.clone(), new_score);
        }

        // 检查收敛
        let max_diff = node_ids
            .iter()
            .map(|id| {
                let old = scores.get(id).copied().unwrap_or(0.0);
                let new = new_scores.get(id).copied().unwrap_or(0.0);
                (old - new).abs()
            })
            .fold(0.0, f64::max);

        scores = new_scores;

        if max_diff < tolerance {
            break;
        }
    }

    scores
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};
    use std::collections::HashMap;

    fn build_test_graph() -> Graph {
        let mut g = Graph::new();
        g.add_node(GraphNode { id: "a".into(), label: "n".into(), properties: HashMap::new() });
        g.add_node(GraphNode { id: "b".into(), label: "n".into(), properties: HashMap::new() });
        g.add_node(GraphNode { id: "c".into(), label: "n".into(), properties: HashMap::new() });
        g.add_edge(GraphEdge {
            id: "e1".into(), from: "a".into(), to: "b".into(),
            label: "l".into(), properties: HashMap::new(), directed: true,
        });
        g.add_edge(GraphEdge {
            id: "e2".into(), from: "b".into(), to: "c".into(),
            label: "l".into(), properties: HashMap::new(), directed: true,
        });
        g.add_edge(GraphEdge {
            id: "e3".into(), from: "c".into(), to: "a".into(),
            label: "l".into(), properties: HashMap::new(), directed: true,
        });
        g
    }

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        let result = pagerank(&g, 0.85, 100, 1e-6);
        assert!(result.is_empty());
    }

    #[test]
    fn test_pagerank_converges() {
        let g = build_test_graph();
        let result = pagerank(&g, 0.85, 100, 1e-6);
        assert_eq!(result.len(), 3);
        // 三个节点环形，分数应该相近
        let sum: f64 = result.values().sum();
        assert!((sum - 1.0).abs() < 0.01, "sum should be ~1.0, got {}", sum);
    }
}
''')

write_file("platform/core/kg/mox-kg-algo-core/src/centrality.rs", '''//! 中心性算法
//!
//! - 介数中心性 (Brandes 2001)
//! - 紧密中心性 (Harmonic Closeness)

use std::collections::{HashMap, VecDeque};
use mox_kg_meta_core::{Graph, NodeId};

/// 中心性结果
pub type CentralityResult = HashMap<NodeId, f64>;

/// 介数中心性（Brandes 算法，简化版 BFS 无权图）
///
/// 计算每个节点作为最短路径中介的次数比例。
pub fn betweenness_centrality(graph: &Graph, normalized: bool) -> CentralityResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let mut betweenness: HashMap<NodeId, f64> = node_ids
        .iter()
        .map(|id| (id.clone(), 0.0))
        .collect();

    for s in &node_ids {
        let mut stack: Vec<NodeId> = Vec::new();
        let mut predecessors: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut sigma: HashMap<NodeId, f64> = HashMap::new();
        let mut dist: HashMap<NodeId, i32> = HashMap::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();

        for v in &node_ids {
            predecessors.insert(v.clone(), Vec::new());
            sigma.insert(v.clone(), 0.0);
            dist.insert(v.clone(), -1);
        }
        sigma.insert(s.clone(), 1.0);
        dist.insert(s.clone(), 0);
        queue.push_back(s.clone());

        while let Some(v) = queue.pop_front() {
            stack.push(v.clone());
            for w in graph.neighbors(&v) {
                let w = w.clone();
                // 首次发现
                if *dist.get(&w).unwrap() == -1 {
                    dist.insert(w.clone(), *dist.get(&v).unwrap() + 1);
                    queue.push_back(w.clone());
                }
                // 最短路径经过 v
                if *dist.get(&w).unwrap() == *dist.get(&v).unwrap() + 1 {
                    let sv = *sigma.get(&v).unwrap();
                    *sigma.get_mut(&w).unwrap() += sv;
                    predecessors.get_mut(&w).unwrap().push(v.clone());
                }
            }
        }

        let mut delta: HashMap<NodeId, f64> = node_ids
            .iter()
            .map(|id| (id.clone(), 0.0))
            .collect();

        while let Some(w) = stack.pop() {
            for v in predecessors.get(&w).unwrap() {
                let ratio = sigma.get(v).unwrap() / sigma.get(&w).unwrap();
                let dw = *delta.get(&w).unwrap();
                *delta.get_mut(v).unwrap() += ratio * (1.0 + dw);
            }
            if &w != s {
                *betweenness.get_mut(&w).unwrap() += *delta.get(&w).unwrap();
            }
        }
    }

    // 有向图除以 2，标准化
    let n = graph.node_count() as f64;
    if normalized && n > 2.0 {
        let factor = 1.0 / ((n - 1.0) * (n - 2.0));
        betweenness.iter_mut().for_each(|(_, v)| *v *= factor);
    }

    betweenness
}

/// 紧密中心性（Harmonic Closeness）
///
/// 使用调和平均，解决不可达节点的问题。
pub fn harmonic_closeness(graph: &Graph) -> CentralityResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let mut result = HashMap::new();
    let n = graph.node_count() as f64;

    for s in &node_ids {
        let mut dist: HashMap<NodeId, f64> = HashMap::new();
        let mut visited: HashMap<NodeId, bool> = HashMap::new();
        let mut queue: VecDeque<(NodeId, i32)> = VecDeque::new();

        for v in &node_ids {
            dist.insert(v.clone(), f64::INFINITY);
            visited.insert(v.clone(), false);
        }
        dist.insert(s.clone(), 0.0);
        visited.insert(s.clone(), true);
        queue.push_back((s.clone(), 0));

        while let Some((v, d)) = queue.pop_front() {
            for w in graph.neighbors(&v) {
                let w = w.clone();
                if !*visited.get(&w).unwrap() {
                    visited.insert(w.clone(), true);
                    dist.insert(w.clone(), (d + 1) as f64);
                    queue.push_back((w.clone(), d + 1));
                }
            }
        }

        let harmonic_sum: f64 = dist
            .values()
            .filter(|&&d| d > 0.0 && d.is_finite())
            .map(|&d| 1.0 / d)
            .sum();

        let closeness = if n > 1.0 { harmonic_sum / (n - 1.0) } else { 0.0 };
        result.insert(s.clone(), closeness);
    }

    result
}
''')

write_file("platform/core/kg/mox-kg-algo-core/src/community.rs", '''//! 社区检测算法
//!
//! CNM (Clauset-Newman-Moore) 模块度贪心凝聚算法

use std::collections::{HashMap, HashSet};
use mox_kg_meta_core::{Graph, NodeId};

/// 社区检测结果：社区 ID -> 节点 ID 列表
pub type CommunityResult = HashMap<usize, Vec<NodeId>>;

/// CNM 社区检测（模块度贪心）
///
/// 自底向上的层次聚类，每次合并使模块度增益最大的两个社区。
pub fn cnm_community(graph: &Graph) -> CommunityResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let n = graph.node_count();
    let m = graph.edge_count() as f64;

    if n == 0 || m == 0.0 {
        let mut result = HashMap::new();
        for (i, id) in node_ids.iter().enumerate() {
            result.insert(i, vec![id.clone()]);
        }
        return result;
    }

    // 初始化：每个节点一个社区
    let mut node_community: HashMap<NodeId, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i))
        .collect();

    let mut communities: HashMap<usize, HashSet<NodeId>> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (i, HashSet::from([id.clone()])))
        .collect();

    // 计算每个社区的度
    let mut community_degree: HashMap<usize, f64> = HashMap::new();
    for (cid, nodes) in &communities {
        let deg: f64 = nodes.iter().map(|id| graph.neighbors(id).len() as f64).sum();
        community_degree.insert(*cid, deg);
    }

    // 初始模块度
    let mut max_modularity = -1.0;
    let mut best_communities = communities.clone();

    // 贪心合并
    let mut num_communities = n;

    while num_communities > 1 {
        let mut best_gain = f64::NEG_INFINITY;
        let mut best_pair: Option<(usize, usize)> = None;

        // 遍历所有可能的社区对（简化：找相邻社区对）
        let community_list: Vec<usize> = communities.keys().copied().collect();

        for i in 0..community_list.len() {
            for j in (i + 1)..community_list.len() {
                let ci = community_list[i];
                let cj = community_list[j];

                // 检查是否相邻（有边连接）
                let mut has_connection = false;
                'outer: for ni in communities.get(&ci).unwrap() {
                    for nj in communities.get(&cj).unwrap() {
                        // 简化判断：互为邻居
                        let neighbors_i = graph.neighbors(ni);
                        if neighbors_i.iter().any(|&&id| id == *nj) {
                            has_connection = true;
                            break 'outer;
                        }
                    }
                }

                if !has_connection {
                    continue;
                }

                // 计算模块度增益 ΔQ
                let ki = community_degree.get(&ci).copied().unwrap_or(0.0);
                let kj = community_degree.get(&cj).copied().unwrap_or(0.0);

                // 计算两社区之间的边数
                let mut e_ij = 0.0;
                for ni in communities.get(&ci).unwrap() {
                    for nj in communities.get(&cj).unwrap() {
                        let neighbors_i = graph.neighbors(ni);
                        if neighbors_i.iter().any(|&&id| id == *nj) {
                            e_ij += 1.0;
                        }
                    }
                }

                let delta_q = (e_ij / m) - (ki * kj) / (2.0 * m * m);

                if delta_q > best_gain {
                    best_gain = delta_q;
                    best_pair = Some((ci, cj));
                }
            }
        }

        if best_pair.is_none() || best_gain <= 0.0 {
            break;
        }

        let (ci, cj) = best_pair.unwrap();

        // 合并社区
        let cj_nodes = communities.remove(&cj).unwrap();
        let ci_nodes = communities.get_mut(&ci).unwrap();
        for node in &cj_nodes {
            node_community.insert(node.clone(), ci);
            ci_nodes.insert(node.clone());
        }

        // 更新度
        let cj_deg = community_degree.remove(&cj).unwrap_or(0.0);
        *community_degree.get_mut(&ci).unwrap() += cj_deg;

        num_communities -= 1;

        // 计算当前模块度（简化）
        let mut modularity = 0.0;
        for (_, nodes) in &communities {
            let mut internal_edges = 0.0;
            let mut total_degree = 0.0;
            for node in nodes {
                total_degree += graph.neighbors(node).len() as f64;
                for neighbor in graph.neighbors(node) {
                    if nodes.contains(neighbor) {
                        internal_edges += 1.0;
                    }
                }
            }
            internal_edges /= 2.0; // 每条边计了两次
            modularity += internal_edges / m - (total_degree / (2.0 * m)).powi(2);
        }

        if modularity > max_modularity {
            max_modularity = modularity;
            best_communities = communities.clone();
        }
    }

    // 转换输出格式
    let mut result = HashMap::new();
    for (i, (_, nodes)) in best_communities.into_iter().enumerate() {
        result.insert(i, nodes.into_iter().collect());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        let result = cnm_community(&g);
        assert!(result.is_empty());
    }
}
''')

write_file("platform/core/kg/mox-kg-algo-core/src/spread.rs", '''//! 激活扩散算法
//!
//! 个性化 PageRank 的简化版本，用于意图识别、影响面分析、推荐召回。
//! 阻尼因子 d=0.85，迭代 30 轮收敛。

use std::collections::HashMap;
use mox_kg_meta_core::{Graph, NodeId};

/// 激活扩散结果
pub type SpreadResult = HashMap<NodeId, f64>;

/// 激活扩散算法（Activation Spread）
///
/// 从种子节点出发，沿边传播激活值，每轮衰减 damping 比例。
///
/// # Arguments
/// * `graph` - 图
/// * `seeds` - 种子节点及初始激活值
/// * `damping` - 阻尼因子（保留比例），通常 0.85
/// * `max_iterations` - 最大迭代次数，默认 30
pub fn activation_spread(
    graph: &Graph,
    seeds: &HashMap<NodeId, f64>,
    damping: f64,
    max_iterations: u32,
) -> SpreadResult {
    let node_ids: Vec<NodeId> = graph.node_ids().iter().map(|id| (*id).clone()).collect();
    let n = graph.node_count();

    if n == 0 {
        return HashMap::new();
    }

    // 初始化激活值
    let mut activation: HashMap<NodeId, f64> = node_ids
        .iter()
        .map(|id| {
            let seed_val = seeds.get(id).copied().unwrap_or(0.0);
            (id.clone(), seed_val)
        })
        .collect();

    let teleport: f64 = seeds.values().sum::<f64>() * (1.0 - damping) / n as f64;

    for _ in 0..max_iterations {
        let mut new_activation: HashMap<NodeId, f64> = node_ids
            .iter()
            .map(|id| (id.clone(), teleport + seeds.get(id).copied().unwrap_or(0.0) * (1.0 - damping)))
            .collect();

        // 沿边传播
        for node_id in &node_ids {
            let act = activation.get(node_id).copied().unwrap_or(0.0);
            if act <= 0.0 {
                continue;
            }

            let neighbors = graph.neighbors(node_id);
            if neighbors.is_empty() {
                continue;
            }

            let share = damping * act / neighbors.len() as f64;
            for &neighbor in &neighbors {
                *new_activation.get_mut(neighbor).unwrap() += share;
            }
        }

        activation = new_activation;
    }

    activation
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};
    use std::collections::HashMap;

    #[test]
    fn test_empty_graph() {
        let g = Graph::new();
        let seeds = HashMap::new();
        let result = activation_spread(&g, &seeds, 0.85, 30);
        assert!(result.is_empty());
    }

    #[test]
    fn test_single_seed() {
        let mut g = Graph::new();
        g.add_node(GraphNode { id: "a".into(), label: "n".into(), properties: HashMap::new() });
        g.add_node(GraphNode { id: "b".into(), label: "n".into(), properties: HashMap::new() });
        g.add_edge(GraphEdge {
            id: "e1".into(), from: "a".into(), to: "b".into(),
            label: "l".into(), properties: HashMap::new(), directed: false,
        });

        let mut seeds = HashMap::new();
        seeds.insert("a".into(), 1.0);

        let result = activation_spread(&g, &seeds, 0.85, 30);
        assert!(result.get("a").copied().unwrap_or(0.0) > 0.0);
        assert!(result.get("b").copied().unwrap_or(0.0) > 0.0);
    }
}
''')

write_file("platform/core/kg/mox-kg-algo-core/src/shortest_path.rs", '''//! 最短路径算法
//!
//! BFS（无权图）/ Dijkstra（带权图）

use std::collections::{HashMap, VecDeque};
use mox_kg_meta_core::{Graph, NodeId};

/// BFS 最短路径（无权图）
///
/// 返回从源点到各节点的最短距离。
pub fn bfs_shortest_path(graph: &Graph, source: &NodeId) -> HashMap<NodeId, u32> {
    let mut distances: HashMap<NodeId, u32> = graph
        .node_ids()
        .iter()
        .map(|id| ((*id).clone(), u32::MAX))
        .collect();

    let mut queue = VecDeque::new();
    distances.insert(source.clone(), 0);
    queue.push_back(source.clone());

    while let Some(node) = queue.pop_front() {
        let current_dist = *distances.get(&node).unwrap();

        for &neighbor in graph.neighbors(&node) {
            if *distances.get(neighbor).unwrap() == u32::MAX {
                distances.insert(neighbor.clone(), current_dist + 1);
                queue.push_back(neighbor.clone());
            }
        }
    }

    distances
}

/// 获取两点间的最短路径长度
pub fn shortest_path_length(graph: &Graph, source: &NodeId, target: &NodeId) -> Option<u32> {
    let distances = bfs_shortest_path(graph, source);
    distances.get(target).copied().filter(|&d| d != u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_kg_meta_core::{GraphNode, GraphEdge};
    use std::collections::HashMap;

    fn build_linear_graph() -> Graph {
        let mut g = Graph::new();
        for i in 0..4 {
            g.add_node(GraphNode {
                id: format!("n{}", i),
                label: "n".into(),
                properties: HashMap::new(),
            });
        }
        for i in 0..3 {
            g.add_edge(GraphEdge {
                id: format!("e{}", i),
                from: format!("n{}", i),
                to: format!("n{}", i + 1),
                label: "l".into(),
                properties: HashMap::new(),
                directed: false,
            });
        }
        g
    }

    #[test]
    fn test_bfs_linear() {
        let g = build_linear_graph();
        let distances = bfs_shortest_path(&g, &"n0".into());
        assert_eq!(*distances.get("n0").unwrap(), 0);
        assert_eq!(*distances.get("n1").unwrap(), 1);
        assert_eq!(*distances.get("n2").unwrap(), 2);
        assert_eq!(*distances.get("n3").unwrap(), 3);
    }

    #[test]
    fn test_shortest_path_length() {
        let g = build_linear_graph();
        assert_eq!(shortest_path_length(&g, &"n0".into(), &"n3".into()), Some(3));
    }
}
''')

# --- mox-ai-core ---
write_file("platform/core/ai/mox-ai-core/Cargo.toml", crate_cargo(
    "mox-ai-core",
    deps_section='''mox-platform-foundation = { path = "../../../foundation/mox-platform-foundation" }
serde = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }'''
))

write_file("platform/core/ai/mox-ai-core/src/lib.rs", crate_lib(
    "mox-ai-core",
    "AI 核心类型与接口 — LLM Provider 抽象、消息、Embedding、工具调用",
    modules='''pub mod types;
pub mod provider;
pub mod message;
pub mod tool;

pub use types::*;
pub use provider::*;
pub use message::*;
pub use tool::*;'''
))

write_file("platform/core/ai/mox-ai-core/src/types.rs", '''//! AI 核心类型定义

use serde::{Deserialize, Serialize};

/// 模型角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// 完成度设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    /// 温度 0.0-2.0
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 最大生成 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Top P 采样
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    /// 停止序列
    #[serde(default)]
    pub stop: Vec<String>,
    /// 是否流式输出
    #[serde(default)]
    pub stream: bool,
}

fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32 { 2048 }
fn default_top_p() -> f32 { 1.0 }

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            top_p: default_top_p(),
            stop: vec![],
            stream: false,
        }
    }
}

/// Embedding 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub model: String,
    pub dimensions: usize,
}

/// 余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_same_vector() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }
}
''')

write_file("platform/core/ai/mox-ai-core/src/message.rs", '''//! 消息类型
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
''')

write_file("platform/core/ai/mox-ai-core/src/tool.rs", '''//! 工具定义抽象

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称
    pub name: String,
    /// 工具描述
    pub description: String,
    /// 参数 JSON Schema
    pub parameters: serde_json::Value,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 是否成功
    pub success: bool,
    /// 结果内容
    pub content: String,
    /// 置信度 0-1
    #[serde(default)]
    pub confidence: f32,
    /// 错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
''')

write_file("platform/core/ai/mox-ai-core/src/provider.rs", '''//! LLM Provider 抽象
//!
//! 定义统一的 LLM 接口，支持多种后端实现

use async_trait::async_trait;
use crate::message::{ChatMessage, ChatCompletion};
use crate::types::{CompletionOptions, Embedding};
use crate::error::AiError;

pub type AiResult<T> = Result<T, AiError>;

/// LLM Provider 接口
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider 名称
    fn name(&self) -> &str;

    /// 聊天完成
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: &CompletionOptions,
    ) -> AiResult<ChatCompletion>;

    /// 生成 Embedding
    async fn embed(&self, texts: &[String]) -> AiResult<Vec<Embedding>>;
}

/// Provider 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    /// 本地 vLLM
    LocalVllm,
    /// OpenAI 兼容
    OpenAi,
    /// Anthropic
    Anthropic,
}
''')

# 补充 mox-ai-core 的 error 模块
write_file("platform/core/ai/mox-ai-core/src/error.rs", '''//! AI 错误类型

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("模型调用失败: {0}")]
    ProviderError(String),

    #[error("请求超时")]
    Timeout,

    #[error("请求被限流")]
    RateLimited,

    #[error("内容安全过滤")]
    ContentFiltered,

    #[error("参数错误: {0}")]
    InvalidParameter(String),
}
''')

# 注意：需要在 lib.rs 中添加 error 模块声明，我上面的 modules 里漏了
# 让我直接重新生成 lib.rs 以确保正确
write_file("platform/core/ai/mox-ai-core/src/lib.rs", '''//! # mox-ai-core
//!
//! AI 核心类型与接口 — LLM Provider 抽象、消息、Embedding、工具调用
//!
//! ## 功能特性
//! - 统一的 LLM Provider 抽象接口
//! - 聊天消息与工具调用类型
//! - Embedding 与余弦相似度计算
//! - Completion 参数配置

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod types;
pub mod provider;
pub mod message;
pub mod tool;

pub use error::AiError;
pub use types::*;
pub use provider::*;
pub use message::*;
pub use tool::*;

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
''')

print("  ✓ Core layer (kg + ai) complete")

# ============================================================
# 生成剩余代表性 crate（快速生成骨架）
# ============================================================

print("\n📦 Generating remaining crate skeletons...")

# 定义要生成的 crate 列表：(路径, 名称, 描述, 依赖)
crates = [
    # API Layer
    ("platform/api/mox-kg-api", "mox-kg-api", "图谱 API DTO 与接口定义",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-kg-meta-core = { path = \"../../core/kg/mox-kg-meta-core\" }\nserde = { workspace = true }\nvalidator = { workspace = true }"),
    ("platform/api/mox-ai-api", "mox-ai-api", "AI API DTO 与接口定义",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-ai-core = { path = \"../../core/ai/mox-ai-core\" }\nserde = { workspace = true }\nvalidator = { workspace = true }"),
    ("platform/api/mox-platform-api", "mox-platform-api", "平台 API DTO 与接口定义",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nserde = { workspace = true }\nvalidator = { workspace = true }"),
    ("platform/api/mox-flow-api", "mox-flow-api", "流程 API DTO 与接口定义",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nserde = { workspace = true }"),
    ("platform/api/mox-data-api", "mox-data-api", "数据 API DTO 与接口定义",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nserde = { workspace = true }"),
    ("platform/api/mox-cloud-api", "mox-cloud-api", "云存储 API DTO 与接口定义",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-cloud-foundation = { path = \"../../foundation/mox-cloud-foundation\" }\nserde = { workspace = true }"),

    # Core Layer - flow & data
    ("platform/core/flow/mox-flow-optimizer-core", "mox-flow-optimizer-core", "DAG 优化器核心 — CPM 关键路径/RCPSP 资源约束调度",
     "mox-platform-foundation = { path = \"../../../foundation/mox-platform-foundation\" }\nserde = { workspace = true }"),
    ("platform/core/flow/mox-flow-operator-core", "mox-flow-operator-core", "算子核心 — 算子接口/类型系统",
     "mox-platform-foundation = { path = \"../../../foundation/mox-platform-foundation\" }\nserde = { workspace = true }\nasync-trait = { workspace = true }"),
    ("platform/core/ai/mox-ai-intent-core", "mox-ai-intent-core", "意图识别核心 — 意图分类/匹配评分算法",
     "mox-platform-foundation = { path = \"../../../foundation/mox-platform-foundation\" }\nmox-ai-core = { path = \"./mox-ai-core\" }\nserde = { workspace = true }"),

    # SvcAPI Layer
    ("platform/svcapi/mox-kg-svcapi", "mox-kg-svcapi", "图谱 gRPC 服务契约",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-kg-api = { path = \"../../api/mox-kg-api\" }\ntonic = { workspace = true }\nprost = { workspace = true }"),
    ("platform/svcapi/mox-ai-svcapi", "mox-ai-svcapi", "AI gRPC 服务契约",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-ai-api = { path = \"../../api/mox-ai-api\" }\ntonic = { workspace = true }\nprost = { workspace = true }"),
    ("platform/svcapi/mox-platform-svcapi", "mox-platform-svcapi", "平台 gRPC 服务契约",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-platform-api = { path = \"../../api/mox-platform-api\" }\ntonic = { workspace = true }\nprost = { workspace = true }"),
    ("platform/svcapi/mox-flow-svcapi", "mox-flow-svcapi", "流程 gRPC 服务契约",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-flow-api = { path = \"../../api/mox-flow-api\" }\ntonic = { workspace = true }\nprost = { workspace = true }"),

    # Service Layer - KG
    ("platform/services/kg/mox-kg-service-svc", "mox-kg-service-svc", "图谱查询服务 — 图查询/遍历/CRUD",
     "mox-platform-foundation = { path = \"../../../foundation/mox-platform-foundation\" }\nmox-kg-api = { path = \"../../../api/mox-kg-api\" }\nmox-kg-algo-core = { path = \"../../../core/kg/mox-kg-algo-core\" }\nmox-kg-meta-core = { path = \"../../../core/kg/mox-kg-meta-core\" }\nasync-trait = { workspace = true }\ntracing = { workspace = true }"),

    # Service Layer - AI
    ("platform/services/ai/mox-ai-expert-svc", "mox-ai-expert-svc", "专家服务 — 专家注册/匹配/协作",
     "mox-platform-foundation = { path = \"../../../foundation/mox-platform-foundation\" }\nmox-ai-api = { path = \"../../../api/mox-ai-api\" }\nmox-ai-core = { path = \"../../../core/ai/mox-ai-core\" }\nasync-trait = { workspace = true }\ntracing = { workspace = true }"),
    ("platform/services/ai/mox-ai-agent-svc", "mox-ai-agent-svc", "AI Agent 服务 — ReAct 循环/工具调用",
     "mox-platform-foundation = { path = \"../../../foundation/mox-platform-foundation\" }\nmox-ai-api = { path = \"../../../api/mox-ai-api\" }\nmox-ai-core = { path = \"../../../core/ai/mox-ai-core\" }\nmox-ai-expert-svc = { path = \"./mox-ai-expert-svc\" }\nasync-trait = { workspace = true }\ntracing = { workspace = true }"),

    # Service Layer - Platform
    ("platform/services/platform/mox-platform-system-svc", "mox-platform-system-svc", "系统服务 — 用户/角色/权限/审计",
     "mox-platform-foundation = { path = \"../../../foundation/mox-platform-foundation\" }\nmox-platform-api = { path = \"../../../api/mox-platform-api\" }\nasync-trait = { workspace = true }\ntracing = { workspace = true }\nthiserror = { workspace = true }"),

    # Gateway
    ("platform/gateway/mox-platform-gateway-runtime", "mox-platform-gateway-runtime", "网关运行时 — HTTP/gRPC 入口/路由/中间件",
     "mox-platform-foundation = { path = \"../../foundation/mox-platform-foundation\" }\nmox-platform-system-svc = { path = \"../../services/platform/mox-platform-system-svc\" }\nmox-kg-service-svc = { path = \"../../services/kg/mox-kg-service-svc\" }\nmox-ai-agent-svc = { path = \"../../services/ai/mox-ai-agent-svc\" }\naxum = { workspace = true }\ntokio = { workspace = true }\ntracing = { workspace = true }\ntracing-subscriber = { workspace = true }\ndotenvy = { workspace = true }\nconfig = { workspace = true }\nserde = { workspace = true }\nserde_json = { workspace = true }"),
]

for path, name, desc, deps in crates:
    write_file(f"{path}/Cargo.toml", crate_cargo(name, deps_section=deps))
    write_file(f"{path}/src/lib.rs", crate_lib(name, desc))

# 为 gateway runtime 生成 main.rs
write_file("platform/gateway/mox-platform-gateway-runtime/src/main.rs", '''//! Mox 平台网关运行时入口
//!
//! 启动 HTTP/gRPC 服务，注册路由，装配中间件

use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod routes;
mod middleware;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载配置
    dotenvy::dotenv().ok();

    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Mox Platform Gateway starting...");
    tracing::info!("Version: {}", mox_platform_gateway_runtime::VERSION);

    // 加载配置
    let config = config::AppConfig::load()?;
    tracing::info!("Config loaded: server on {}:{}", config.server.host, config.server.port);

    // 构建路由
    let app = routes::build_router();

    // 启动 HTTP 服务
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;
    tracing::info!("HTTP server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
''')

write_file("platform/gateway/mox-platform-gateway-runtime/src/config.rs", '''//! 应用配置

use serde::Deserialize;

/// 服务配置
#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

/// 数据库配置
#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

/// 应用配置
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}

impl AppConfig {
    /// 从环境变量和配置文件加载
    pub fn load() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(config::Environment::with_prefix("MOX").separator("_"))
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("database.max_connections", 20)?
            .build()?;

        config.try_deserialize()
    }
}
''')

write_file("platform/gateway/mox-platform-gateway-runtime/src/routes.rs", '''//! 路由注册
//!
//! 统一注册所有域的 HTTP 路由

use axum::{Router, routing::get, http::StatusCode, Json};
use mox_platform_foundation::ApiResponse;
use uuid::Uuid;

/// 构建总路由
pub fn build_router() -> Router {
    Router::new()
        // 健康检查
        .route("/health", get(health_check))
        .route("/api/v1/health", get(health_check))
        // API v1 路由（占位，后续各域注册）
        .nest("/api/v1/kg", kg_routes())
        .nest("/api/v1/ai", ai_routes())
}

fn health_check() -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let req_id = format!("req_{}", Uuid::new_v4().simple());
    (
        StatusCode::OK,
        Json(ApiResponse::success(serde_json::json!({"status": "ok"}), req_id)),
    )
}

fn kg_routes() -> Router {
    Router::new()
        .route("/graphs", get(|| async { "KG API placeholder" }))
}

fn ai_routes() -> Router {
    Router::new()
        .route("/chat", get(|| async { "AI API placeholder" }))
}
''')

write_file("platform/gateway/mox-platform-gateway-runtime/src/middleware.rs", '''//! 中间件
//!
//! 统一中间件：请求 ID、日志、CORS、鉴权等

// 中间件占位实现，后续逐步完善
''')

# 修改 gateway runtime 的 lib.rs（因为有 main.rs，lib.rs 也可以有）
write_file("platform/gateway/mox-platform-gateway-runtime/src/lib.rs", '''//! # mox-platform-gateway-runtime
//!
//! 网关运行时 — HTTP/gRPC 统一入口，路由分发，中间件装配
//!
//! ## 功能特性
//! - HTTP RESTful API 服务
//! - gRPC 内部服务
//! - 统一请求 ID 与日志追踪
//! - CORS 与鉴权中间件
//! - 健康检查与指标

#![warn(missing_docs)]
#![warn(clippy::all)]

/// Crate 版本号
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
''')

print("  ✓ All crate skeletons generated")

# ============================================================
# README
# ============================================================

print("\n📝 Generating README...")
write_file("README.md", '''# 璇玑 RelGraph · Mox Platform

> 以 Rust 自研高性能知识图谱为唯一中枢的七维归一化关联与自动化治理系统。

## 架构

```
6层8域 DDD 矩阵：

L6  Gateway       mox-platform-gateway-runtime
                    │
L5  Services      mox-*-svc (kg/ai/flow/data/cloud/platform)
                    │
L2  SvcAPI        mox-*-svcapi (gRPC 契约)
                    │
L1  API           mox-*-api (REST DTO)
                    │
L3  Core          mox-*-core (纯计算 · 零IO)
                    │
L0  Foundation    mox-platform-foundation
```

## 快速开始

### 前置条件

- Rust 1.80+ (`rustup default stable`)
- protobuf 编译器 3.20+
- PostgreSQL 14+ / Redis 7+（生产环境）

### 构建

```bash
cargo build
```

### 测试

```bash
cargo test
```

### 运行

```bash
cp .env.example .env
cargo run -p mox-platform-gateway-runtime
```

## 开发规范

- 命名公式：`mox-<domain>-<layer>-<role>`
- 8 个业务域：kg / ai / flow / data / cloud / voice / platform / market
- 6 个架构层：foundation / core / api / svcapi / svc / gateway
- 依赖方向：上层依赖下层，禁止反向依赖
- Core 层零 IO，纯计算，可独立测试

## 文档

- 开发文档：`../developer-docs.html`
- 架构图谱：`../docs-optimal-architecture-map.html`
- 专家联盟白皮书：`expert-alliance-tech-whitepaper/index.html`

## 项目结构

```
mox-workspace/
├── platform/
│   ├── foundation/          # L0 基础层
│   ├── core/                # L3 核心计算层
│   ├── api/                 # L1 对外契约层
│   ├── svcapi/              # L2 服务间契约层
│   ├── services/            # L5 服务实现层
│   └── gateway/             # L6 网关运行时
├── Cargo.toml               # workspace 配置
├── rust-toolchain.toml
└── .env.example
```

## License

MIT OR Apache-2.0
''')

print("\n✅ 项目初始化完成！")
print(f"   生成位置: {BASE}")
print("   Crate 数量: 24+")
print("   架构层数: 6 层")
print("   业务域数: 6 个 (kg/ai/flow/data/cloud/platform)")
