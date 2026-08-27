// mox-kg-core 数据模型：顶点(Vertex)、边(Edge)、查询结果、DSL

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 顶点（实体节点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    /// 顶点ID，格式：{type}:{id}，如 "product:1"
    pub id: String,
    /// 顶点类型，如 "product"、"news"、"case"
    pub vertex_type: String,
    /// 顶点属性（JSON对象）
    pub properties: serde_json::Value,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

impl Vertex {
    pub fn new(id: impl Into<String>, vertex_type: impl Into<String>, properties: serde_json::Value) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: id.into(),
            vertex_type: vertex_type.into(),
            properties,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 生成顶点存储key
    pub fn storage_key(&self) -> String {
        format!("vertex:{}:{}", self.vertex_type, self.id)
    }

    /// 从存储key解析顶点ID和类型
    pub fn parse_key(key: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = key.splitn(3, ':').collect();
        if parts.len() == 3 && parts[0] == "vertex" {
            Some((parts[1].to_string(), parts[2].to_string()))
        } else {
            None
        }
    }
}

/// 边（关系）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// 边ID，格式：edge:{type}:{source}:{target}
    pub id: String,
    /// 边类型，如 "belongs_to"、"uses"、"related_to"
    pub edge_type: String,
    /// 源顶点ID
    pub source: String,
    /// 目标顶点ID
    pub target: String,
    /// 边属性（JSON对象）
    pub properties: serde_json::Value,
    /// 创建时间
    pub created_at: String,
}

impl Edge {
    pub fn new(
        edge_type: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        properties: serde_json::Value,
    ) -> Self {
        let edge_type = edge_type.into();
        let source = source.into();
        let target = target.into();
        let id = format!("edge:{}:{}:{}", edge_type, source, target);
        Self {
            id,
            edge_type,
            source,
            target,
            properties,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 生成边存储key
    pub fn storage_key(&self) -> String {
        format!("edge:{}:{}:{}", self.edge_type, self.source, self.target)
    }

    /// 出边索引key（按源点）
    pub fn out_index_key(&self) -> String {
        format!("idx:edge_out:{}:{}:{}", self.source, self.edge_type, self.target)
    }

    /// 入边索引key（按目标点）
    pub fn in_index_key(&self) -> String {
        format!("idx:edge_in:{}:{}:{}", self.target, self.edge_type, self.source)
    }
}

/// 遍历方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraverseDirection {
    /// 出边（从源点到目标点）
    Out,
    /// 入边（从目标点到源点）
    In,
    /// 双向
    Both,
}

/// 遍历结果（单跳）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraverseResult {
    /// 起始顶点
    pub source: Vertex,
    /// 遍历到的边
    pub edges: Vec<Edge>,
    /// 遍历到的顶点
    pub vertices: Vec<Vertex>,
}

/// 多跳路径结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    /// 路径上的顶点序列
    pub vertices: Vec<Vertex>,
    /// 路径上的边序列
    pub edges: Vec<Edge>,
    /// 路径长度（跳数）
    pub length: usize,
}

/// 查询结果（统一格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// 是否成功
    pub success: bool,
    /// 查询类型
    pub query_type: String,
    /// 顶点结果
    pub vertices: Vec<Vertex>,
    /// 边结果
    pub edges: Vec<Edge>,
    /// 路径结果
    pub paths: Vec<PathResult>,
    /// 聚合结果
    pub aggregations: Option<serde_json::Value>,
    /// 总数
    pub total: usize,
    /// 执行耗时（毫秒）
    pub duration_ms: u64,
    /// 错误信息
    pub error: Option<String>,
}

impl QueryResult {
    pub fn success(query_type: impl Into<String>) -> Self {
        Self {
            success: true,
            query_type: query_type.into(),
            vertices: vec![],
            edges: vec![],
            paths: vec![],
            aggregations: None,
            total: 0,
            duration_ms: 0,
            error: None,
        }
    }

    pub fn error(query_type: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            success: false,
            query_type: query_type.into(),
            vertices: vec![],
            edges: vec![],
            paths: vec![],
            aggregations: None,
            total: 0,
            duration_ms: 0,
            error: Some(msg.into()),
        }
    }
}

/// 顶点创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVertexRequest {
    pub id: String,
    pub vertex_type: String,
    pub properties: serde_json::Value,
}

/// 边创建请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEdgeRequest {
    pub edge_type: String,
    pub source: String,
    pub target: String,
    pub properties: Option<serde_json::Value>,
}

/// 遍历请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraverseRequest {
    pub vertex_id: String,
    pub direction: TraverseDirection,
    pub edge_types: Option<Vec<String>>,
    pub max_depth: Option<usize>,
    pub limit: Option<usize>,
}

/// DSL查询请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DslQueryRequest {
    /// DSL语句，如 "GET case -[uses]-> product WHERE product.id = 'product:1'"
    pub dsl: String,
    /// 查询参数（可选）
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// 企业官网实体类型常量
pub mod entity_types {
    pub const PRODUCT: &str = "product";
    pub const PRODUCT_CATEGORY: &str = "product_category";
    pub const NEWS: &str = "news";
    pub const NEWS_CATEGORY: &str = "news_category";
    pub const CASE: &str = "case";
    pub const TEAM: &str = "team";
    pub const CUSTOMER: &str = "customer";
    pub const FAQ: &str = "faq";
}

/// 企业官网关系类型常量
pub mod edge_types {
    pub const BELONGS_TO: &str = "belongs_to";
    pub const PARENT_OF: &str = "parent_of";
    pub const USES: &str = "uses";
    pub const BELONGS_TO_INDUSTRY: &str = "belongs_to_industry";
    pub const WROTE: &str = "wrote";
    pub const RESPONSIBLE_FOR: &str = "responsible_for";
    pub const RELATED_TO: &str = "related_to";
    pub const REFERENCES: &str = "references";
    pub const SIMILAR_TO: &str = "similar_to";
    pub const RECOMMENDS: &str = "recommends";
    pub const WORKS_WITH: &str = "works_with";
    pub const LOCATED_IN: &str = "located_in";
}
