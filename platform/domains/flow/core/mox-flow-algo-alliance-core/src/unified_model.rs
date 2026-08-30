// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 统一数据模型 — 图/对象/向量/张量统一表示与转换
//!
//! 知识图谱的数据和云盘的对象数据，在算法层面统一为 `UnifiedData`。
//! 算法只需要处理统一的数据抽象，无需关心底层是图存储还是对象存储。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 值类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueType {
    /// 空值
    Null,
    /// 布尔值
    Bool,
    /// 整数
    Int,
    /// 浮点数
    Float,
    /// 字符串
    String,
    /// 二进制数据
    Bytes,
    /// 向量
    Vector,
    /// 图数据
    Graph,
    /// 对象（键值对集合）
    Object,
    /// 列表
    List,
    /// 表/矩阵
    Table,
}

impl ValueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ValueType::Null => "null",
            ValueType::Bool => "bool",
            ValueType::Int => "int",
            ValueType::Float => "float",
            ValueType::String => "string",
            ValueType::Bytes => "bytes",
            ValueType::Vector => "vector",
            ValueType::Graph => "graph",
            ValueType::Object => "object",
            ValueType::List => "list",
            ValueType::Table => "table",
        }
    }
}

/// 统一数据引用（用于零拷贝传递）
pub type UnifiedDataRef = Arc<UnifiedData>;

/// 统一数据
///
/// 算法联盟中的所有算法都接受和返回 `UnifiedData`。
/// 这种统一抽象使得：
/// - 图算法可以直接处理对象数据（转换为二部图）
/// - 编码算法可以处理图数据的序列化结果
/// - 算法可以自由组合成流水线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnifiedData {
    /// 空数据
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
    /// 浮点向量
    Vector(Vec<f64>),
    /// 对象（属性集合）
    Object(HashMap<String, UnifiedData>),
    /// 列表
    List(Vec<UnifiedData>),
    /// 图数据（节点+边）
    Graph {
        /// 节点 ID 列表
        nodes: Vec<String>,
        /// 节点属性
        node_props: HashMap<String, HashMap<String, UnifiedData>>,
        /// 边列表 (源, 目标, 权重)
        edges: Vec<(String, String, f64)>,
        /// 边属性
        edge_props: HashMap<(String, String), HashMap<String, UnifiedData>>,
    },
    /// 表数据（行式）
    Table {
        /// 列名
        columns: Vec<String>,
        /// 行数据
        rows: Vec<Vec<UnifiedData>>,
    },
}

impl UnifiedData {
    /// 创建空数据
    pub fn null() -> Self {
        UnifiedData::Null
    }

    /// 数据类型
    pub fn value_type(&self) -> ValueType {
        match self {
            UnifiedData::Null => ValueType::Null,
            UnifiedData::Bool(_) => ValueType::Bool,
            UnifiedData::Int(_) => ValueType::Int,
            UnifiedData::Float(_) => ValueType::Float,
            UnifiedData::String(_) => ValueType::String,
            UnifiedData::Bytes(_) => ValueType::Bytes,
            UnifiedData::Vector(_) => ValueType::Vector,
            UnifiedData::Object(_) => ValueType::Object,
            UnifiedData::List(_) => ValueType::List,
            UnifiedData::Graph { .. } => ValueType::Graph,
            UnifiedData::Table { .. } => ValueType::Table,
        }
    }

    /// 数据大小估计（字节）
    pub fn estimated_size(&self) -> usize {
        match self {
            UnifiedData::Null => 0,
            UnifiedData::Bool(_) => 1,
            UnifiedData::Int(_) => 8,
            UnifiedData::Float(_) => 8,
            UnifiedData::String(s) => s.len(),
            UnifiedData::Bytes(b) => b.len(),
            UnifiedData::Vector(v) => v.len() * 8,
            UnifiedData::Object(m) => m.iter().map(|(k, v)| k.len() + v.estimated_size()).sum(),
            UnifiedData::List(l) => l.iter().map(|v| v.estimated_size()).sum(),
            UnifiedData::Graph {
                nodes,
                node_props,
                edges,
                edge_props,
            } => {
                let node_size: usize = nodes.iter().map(|n| n.len()).sum();
                let node_prop_size: usize = node_props
                    .values()
                    .map(|m| m.values().map(|v| v.estimated_size()).sum::<usize>())
                    .sum();
                let edge_size = edges.len() * 24; // 2 string refs + f64
                let edge_prop_size: usize = edge_props
                    .values()
                    .map(|m| m.values().map(|v| v.estimated_size()).sum::<usize>())
                    .sum();
                node_size + node_prop_size + edge_size + edge_prop_size
            }
            UnifiedData::Table { columns, rows } => {
                let col_size: usize = columns.iter().map(|c| c.len()).sum();
                let row_size: usize = rows
                    .iter()
                    .map(|r| r.iter().map(|v| v.estimated_size()).sum::<usize>())
                    .sum();
                col_size + row_size
            }
        }
    }

    /// 转换为布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            UnifiedData::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 转换为整数
    pub fn as_int(&self) -> Option<i64> {
        match self {
            UnifiedData::Int(i) => Some(*i),
            UnifiedData::Float(f) => Some(*f as i64),
            _ => None,
        }
    }

    /// 转换为浮点数
    pub fn as_float(&self) -> Option<f64> {
        match self {
            UnifiedData::Float(f) => Some(*f),
            UnifiedData::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 转换为字符串引用
    pub fn as_str(&self) -> Option<&str> {
        match self {
            UnifiedData::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转换为向量引用
    pub fn as_vector(&self) -> Option<&[f64]> {
        match self {
            UnifiedData::Vector(v) => Some(v),
            _ => None,
        }
    }

    /// 转换为字节引用
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            UnifiedData::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// 是否为 null
    pub fn is_null(&self) -> bool {
        matches!(self, UnifiedData::Null)
    }

    /// 包装为 Arc
    pub fn into_arc(self) -> Arc<Self> {
        Arc::new(self)
    }

    // === 图数据快捷方法 ===

    /// 获取图的节点数
    pub fn graph_node_count(&self) -> Option<usize> {
        match self {
            UnifiedData::Graph { nodes, .. } => Some(nodes.len()),
            _ => None,
        }
    }

    /// 获取图的边数
    pub fn graph_edge_count(&self) -> Option<usize> {
        match self {
            UnifiedData::Graph { edges, .. } => Some(edges.len()),
            _ => None,
        }
    }

    // === 类型转换 ===

    /// 将对象数据转换为图（属性图）
    pub fn object_to_graph(&self) -> Option<UnifiedData> {
        match self {
            UnifiedData::Object(props) => {
                let mut nodes = vec!["root".to_string()];
                let mut node_props = HashMap::new();
                node_props.insert("root".to_string(), HashMap::new());
                let mut edges = Vec::new();

                for (key, value) in props {
                    let node_id = format!("prop_{}", key);
                    nodes.push(node_id.clone());
                    let mut props_map = HashMap::new();
                    props_map.insert("value".to_string(), value.clone());
                    node_props.insert(node_id.clone(), props_map);
                    edges.push(("root".to_string(), node_id, 1.0));
                }

                Some(UnifiedData::Graph {
                    nodes,
                    node_props,
                    edges,
                    edge_props: HashMap::new(),
                })
            }
            _ => None,
        }
    }

    /// 将列表数据转换为向量
    pub fn list_to_vector(&self) -> Option<UnifiedData> {
        match self {
            UnifiedData::List(list) => {
                let mut vec = Vec::with_capacity(list.len());
                for item in list {
                    match item.as_float() {
                        Some(f) => vec.push(f),
                        None => return None,
                    }
                }
                Some(UnifiedData::Vector(vec))
            }
            _ => None,
        }
    }
}

// === 便捷构造函数 ===

impl From<bool> for UnifiedData {
    fn from(v: bool) -> Self {
        UnifiedData::Bool(v)
    }
}

impl From<i64> for UnifiedData {
    fn from(v: i64) -> Self {
        UnifiedData::Int(v)
    }
}

impl From<i32> for UnifiedData {
    fn from(v: i32) -> Self {
        UnifiedData::Int(v as i64)
    }
}

impl From<f64> for UnifiedData {
    fn from(v: f64) -> Self {
        UnifiedData::Float(v)
    }
}

impl From<String> for UnifiedData {
    fn from(v: String) -> Self {
        UnifiedData::String(v)
    }
}

impl From<&str> for UnifiedData {
    fn from(v: &str) -> Self {
        UnifiedData::String(v.to_string())
    }
}

impl From<Vec<u8>> for UnifiedData {
    fn from(v: Vec<u8>) -> Self {
        UnifiedData::Bytes(v)
    }
}

impl From<Vec<f64>> for UnifiedData {
    fn from(v: Vec<f64>) -> Self {
        UnifiedData::Vector(v)
    }
}

impl From<Vec<UnifiedData>> for UnifiedData {
    fn from(v: Vec<UnifiedData>) -> Self {
        UnifiedData::List(v)
    }
}

impl From<HashMap<String, UnifiedData>> for UnifiedData {
    fn from(v: HashMap<String, UnifiedData>) -> Self {
        UnifiedData::Object(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        assert_eq!(UnifiedData::null().value_type(), ValueType::Null);
        assert_eq!(UnifiedData::from(true).value_type(), ValueType::Bool);
        assert_eq!(UnifiedData::from(42i64).value_type(), ValueType::Int);
        assert_eq!(UnifiedData::from(3.14).value_type(), ValueType::Float);
        assert_eq!(UnifiedData::from("hello").value_type(), ValueType::String);
        assert_eq!(
            UnifiedData::from(vec![1u8, 2, 3]).value_type(),
            ValueType::Bytes
        );
        assert_eq!(
            UnifiedData::from(vec![1.0, 2.0]).value_type(),
            ValueType::Vector
        );
    }

    #[test]
    fn test_as_conversions() {
        assert_eq!(UnifiedData::from(true).as_bool(), Some(true));
        assert_eq!(UnifiedData::from(42i64).as_int(), Some(42));
        assert_eq!(UnifiedData::from(3.14).as_float(), Some(3.14));
        assert_eq!(
            UnifiedData::from("hello").as_str(),
            Some("hello")
        );
        assert_eq!(
            UnifiedData::from(vec![1.0, 2.0]).as_vector(),
            Some(&[1.0, 2.0][..])
        );
    }

    #[test]
    fn test_estimated_size() {
        assert_eq!(UnifiedData::null().estimated_size(), 0);
        assert_eq!(UnifiedData::from(42i64).estimated_size(), 8);
        assert_eq!(UnifiedData::from("hi").estimated_size(), 2);
        assert_eq!(UnifiedData::from(vec![1.0, 2.0, 3.0]).estimated_size(), 24);
    }

    #[test]
    fn test_graph_data() {
        let mut node_props = HashMap::new();
        node_props.insert("a".to_string(), HashMap::new());
        node_props.insert("b".to_string(), HashMap::new());

        let graph = UnifiedData::Graph {
            nodes: vec!["a".to_string(), "b".to_string()],
            node_props,
            edges: vec![("a".to_string(), "b".to_string(), 1.0)],
            edge_props: HashMap::new(),
        };

        assert_eq!(graph.value_type(), ValueType::Graph);
        assert_eq!(graph.graph_node_count(), Some(2));
        assert_eq!(graph.graph_edge_count(), Some(1));
    }

    #[test]
    fn test_object_to_graph() {
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), UnifiedData::from("test"));
        obj.insert("count".to_string(), UnifiedData::from(42i64));

        let data = UnifiedData::Object(obj);
        let graph = data.object_to_graph().unwrap();

        assert_eq!(graph.graph_node_count(), Some(3)); // root + 2 props
        assert_eq!(graph.graph_edge_count(), Some(2));
    }
}
