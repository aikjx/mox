//! 图谱 Schema 定义
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
