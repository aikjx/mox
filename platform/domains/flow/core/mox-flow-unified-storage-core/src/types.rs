// Copyright (c) 2026 璇玑 RelGraph · 统一存储引擎 (Unified Storage Engine)
// Licensed under the MIT License.

//! 统一存储类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 数据模型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataModel {
    /// 键值模型
    KeyValue,
    /// 图模型
    Graph,
    /// 对象模型
    Object,
}

impl DataModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataModel::KeyValue => "key_value",
            DataModel::Graph => "graph",
            DataModel::Object => "object",
        }
    }
}

/// 存储后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageBackend {
    /// 内存存储
    Memory,
    /// RocksDB 本地存储
    RocksDB,
    /// 分布式对象存储
    ObjectStorage,
    /// 混合存储（热数据内存 + 冷数据磁盘）
    Hybrid,
}

impl StorageBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageBackend::Memory => "memory",
            StorageBackend::RocksDB => "rocksdb",
            StorageBackend::ObjectStorage => "object_storage",
            StorageBackend::Hybrid => "hybrid",
        }
    }
}

/// 通用值类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    /// 空值
    Null,
    /// 布尔值
    Bool(bool),
    /// 整数
    Int(i64),
    /// 浮点数
    Float(f64),
    /// 字符串
    String(String),
    /// 二进制数据
    Bytes(Vec<u8>),
    /// 列表
    List(Vec<Value>),
    /// 对象/映射
    Object(HashMap<String, Value>),
}

impl Value {
    /// 估算值的大小（字节）
    pub fn estimated_size(&self) -> usize {
        match self {
            Value::Null => 1,
            Value::Bool(_) => 1,
            Value::Int(_) => 8,
            Value::Float(_) => 8,
            Value::String(s) => s.len() + 8,
            Value::Bytes(b) => b.len() + 8,
            Value::List(l) => l.iter().map(|v| v.estimated_size()).sum::<usize>() + 8,
            Value::Object(m) => {
                m.iter()
                    .map(|(k, v)| k.len() + v.estimated_size())
                    .sum::<usize>()
                    + 8
            }
        }
    }

    /// 转为字符串引用
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转为整数
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// 转为浮点数
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 转为布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 转为字节引用
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<Vec<u8>> for Value {
    fn from(b: Vec<u8>) -> Self {
        Value::Bytes(b)
    }
}

impl From<Vec<Value>> for Value {
    fn from(l: Vec<Value>) -> Self {
        Value::List(l)
    }
}

impl From<HashMap<String, Value>> for Value {
    fn from(m: HashMap<String, Value>) -> Self {
        Value::Object(m)
    }
}

/// 图节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// 节点 ID
    pub id: String,
    /// 节点标签/类型
    pub labels: Vec<String>,
    /// 节点属性
    pub properties: HashMap<String, Value>,
    /// 创建时间戳
    pub created_at: u64,
    /// 更新时间戳
    pub updated_at: u64,
}

impl GraphNode {
    /// 创建新节点
    pub fn new(id: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            id: id.to_string(),
            labels: Vec::new(),
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 添加标签
    pub fn with_label(mut self, label: &str) -> Self {
        self.labels.push(label.to_string());
        self
    }

    /// 设置属性
    pub fn with_property(mut self, key: &str, value: Value) -> Self {
        self.properties.insert(key.to_string(), value);
        self
    }

    /// 估算大小
    pub fn estimated_size(&self) -> usize {
        self.id.len()
            + self.labels.iter().map(|l| l.len()).sum::<usize>()
            + self
                .properties
                .iter()
                .map(|(k, v)| k.len() + v.estimated_size())
                .sum::<usize>()
            + 16 // timestamps
    }
}

/// 图边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// 边 ID（可选，通常由 src-type-dst 组成）
    pub id: String,
    /// 源节点 ID
    pub src_id: String,
    /// 目标节点 ID
    pub dst_id: String,
    /// 边类型
    pub edge_type: String,
    /// 边属性
    pub properties: HashMap<String, Value>,
    /// 权重
    pub weight: f64,
    /// 创建时间戳
    pub created_at: u64,
    /// 更新时间戳
    pub updated_at: u64,
}

impl GraphEdge {
    /// 创建新边
    pub fn new(src_id: &str, edge_type: &str, dst_id: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let id = format!("{}:{}:{}", src_id, edge_type, dst_id);
        Self {
            id,
            src_id: src_id.to_string(),
            dst_id: dst_id.to_string(),
            edge_type: edge_type.to_string(),
            properties: HashMap::new(),
            weight: 1.0,
            created_at: now,
            updated_at: now,
        }
    }

    /// 设置权重
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// 设置属性
    pub fn with_property(mut self, key: &str, value: Value) -> Self {
        self.properties.insert(key.to_string(), value);
        self
    }

    /// 估算大小
    pub fn estimated_size(&self) -> usize {
        self.id.len()
            + self.src_id.len()
            + self.dst_id.len()
            + self.edge_type.len()
            + self
                .properties
                .iter()
                .map(|(k, v)| k.len() + v.estimated_size())
                .sum::<usize>()
            + 8 // weight
            + 16 // timestamps
    }
}

/// 对象元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMeta {
    /// 对象键（路径）
    pub key: String,
    /// 对象大小（字节）
    pub size: u64,
    /// 内容类型
    pub content_type: String,
    /// 自定义元数据
    pub metadata: HashMap<String, String>,
    /// 创建时间戳
    pub created_at: u64,
    /// 最后修改时间戳
    pub last_modified: u64,
    /// ETag
    pub etag: String,
    /// 存储类别
    pub storage_class: String,
    /// 版本 ID
    pub version_id: Option<String>,
}

impl ObjectMeta {
    /// 创建新的对象元数据
    pub fn new(key: &str, size: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            key: key.to_string(),
            size,
            content_type: "application/octet-stream".to_string(),
            metadata: HashMap::new(),
            created_at: now,
            last_modified: now,
            etag: String::new(),
            storage_class: "standard".to_string(),
            version_id: None,
        }
    }

    /// 设置内容类型
    pub fn with_content_type(mut self, content_type: &str) -> Self {
        self.content_type = content_type.to_string();
        self
    }

    /// 添加元数据
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

/// 存储统计信息
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// 总键数（KV 模型）
    pub total_keys: u64,
    /// 总节点数（图模型）
    pub total_nodes: u64,
    /// 总边数（图模型）
    pub total_edges: u64,
    /// 总对象数（对象模型）
    pub total_objects: u64,
    /// 已用字节数
    pub used_bytes: u64,
    /// 总容量字节数
    pub capacity_bytes: u64,
    /// 读操作次数
    pub read_ops: u64,
    /// 写操作次数
    pub write_ops: u64,
    /// 删除操作次数
    pub delete_ops: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
}

impl StorageStats {
    /// 使用率
    pub fn usage_ratio(&self) -> f64 {
        if self.capacity_bytes == 0 {
            0.0
        } else {
            self.used_bytes as f64 / self.capacity_bytes as f64
        }
    }

    /// 缓存命中率
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total as f64
        }
    }
}

/// 范围查询选项
#[derive(Debug, Clone)]
pub struct RangeOptions {
    /// 起始键（包含）
    pub start: Option<String>,
    /// 结束键（包含）
    pub end: Option<String>,
    /// 前缀匹配
    pub prefix: Option<String>,
    /// 最大返回数量
    pub limit: Option<usize>,
    /// 反向迭代
    pub reverse: bool,
}

impl Default for RangeOptions {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
            prefix: None,
            limit: None,
            reverse: false,
        }
    }
}

impl RangeOptions {
    /// 创建带前缀的范围选项
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: Some(prefix.to_string()),
            ..Default::default()
        }
    }

    /// 设置 limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// 图遍历方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    /// 出边
    Out,
    /// 入边
    In,
    /// 双向
    Both,
}

impl EdgeDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeDirection::Out => "out",
            EdgeDirection::In => "in",
            EdgeDirection::Both => "both",
        }
    }
}

/// 对象列表选项
#[derive(Debug, Clone)]
pub struct ListObjectsOptions {
    /// 前缀
    pub prefix: Option<String>,
    /// 分隔符（用于模拟目录）
    pub delimiter: Option<String>,
    /// 起始标记
    pub marker: Option<String>,
    /// 最大返回数量
    pub max_keys: usize,
}

impl Default for ListObjectsOptions {
    fn default() -> Self {
        Self {
            prefix: None,
            delimiter: None,
            marker: None,
            max_keys: 1000,
        }
    }
}

/// 对象列表结果
#[derive(Debug, Clone)]
pub struct ListObjectsResult {
    /// 对象列表
    pub objects: Vec<ObjectMeta>,
    /// 通用前缀（目录）
    pub common_prefixes: Vec<String>,
    /// 是否被截断
    pub is_truncated: bool,
    /// 下一个标记
    pub next_marker: Option<String>,
}
