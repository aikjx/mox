//! MOX 统一基座 · 统一查询层
//!
//! 定义"四查询原语"的统一 DSL 与执行契约：
//! - `GET(id)`：按 ID 精确取回（Node / Edge / Blob）
//! - `FILTER(props)`：按属性过滤（元数据索引）
//! - `TRAVERSE(edge, k)`：沿边多跳遍历（图引擎）
//! - `RANGE(blob, offset, len)`：Blob 字节区间读取（大对象直达流式）
//!
//! ## 设计原则
//! - 只定义 DSL + 执行 trait，不内置后端；由各域实现 QueryExecutor。
//! - kg 域 mox-kg-hub/fusion/streams 统一接入本层，对外只暴露一个查询入口。
//! - mox-dsql-core 与 DSL 融合，SQL / 图查询一体。

use async_trait::async_trait;
use mox_base_graph_core::GraphTraversal;
use mox_base_index_core::MetadataIndex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 统一查询错误
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("实体不存在: {0}")]
    NotFound(String),
    #[error("不支持的查询原语: {0}")]
    UnsupportedPrimitive(String),
    #[error("查询执行失败: {0}")]
    Execution(String),
    #[error("其他错误: {0}")]
    Other(String),
}

/// 统一查询结果
pub type QueryResult<T> = Result<T, QueryError>;

/// 查询原语（统一 DSL 的原子操作）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryPrimitive {
    /// GET(id)：按 ID 精确取回
    Get { id: String },
    /// FILTER(prop, value)：按属性过滤
    Filter {
        prop: String,
        value: serde_json::Value,
    },
    /// TRAVERSE(from, edge_type, hops)：沿边多跳遍历
    Traverse {
        from: String,
        edge_type: String,
        hops: usize,
    },
    /// RANGE(path, offset, length)：Blob 字节区间读取
    Range {
        path: String,
        offset: u64,
        length: u64,
    },
}

/// 查询结果条目（统一返回形态）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHit {
    /// 命中 ID / 路径
    pub id: String,
    /// 命中类型（node / blob / edge）
    pub kind: String,
    /// 属性摘要
    pub props: std::collections::HashMap<String, serde_json::Value>,
}

/// 统一查询执行器 trait
///
/// 各域（kg/cloud/data/ai）实现此 trait，向外部暴露统一查询入口。
#[async_trait]
pub trait QueryExecutor: Send + Sync {
    /// 执行单个查询原语
    async fn execute(&self, query: &QueryPrimitive) -> QueryResult<Vec<QueryHit>>;

    /// 执行组合查询（顺序执行，结果合并）
    async fn execute_all(&self, queries: &[QueryPrimitive]) -> QueryResult<Vec<QueryHit>>;
}

/// 统一查询 DSL 构造器（fluent 接口）
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder {
    primitives: Vec<QueryPrimitive>,
}

impl QueryBuilder {
    /// 新建查询构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// GET(id)
    pub fn get(mut self, id: impl Into<String>) -> Self {
        self.primitives.push(QueryPrimitive::Get { id: id.into() });
        self
    }

    /// FILTER(prop, value)
    pub fn filter(mut self, prop: impl Into<String>, value: serde_json::Value) -> Self {
        self.primitives.push(QueryPrimitive::Filter {
            prop: prop.into(),
            value,
        });
        self
    }

    /// TRAVERSE(from, edge_type, hops)
    pub fn traverse(
        mut self,
        from: impl Into<String>,
        edge_type: impl Into<String>,
        hops: usize,
    ) -> Self {
        self.primitives.push(QueryPrimitive::Traverse {
            from: from.into(),
            edge_type: edge_type.into(),
            hops,
        });
        self
    }

    /// RANGE(path, offset, length)
    pub fn range(mut self, path: impl Into<String>, offset: u64, length: u64) -> Self {
        self.primitives.push(QueryPrimitive::Range {
            path: path.into(),
            offset,
            length,
        });
        self
    }

    /// 构建查询列表
    pub fn build(self) -> Vec<QueryPrimitive> {
        self.primitives
    }
}

/// 参考实现：基于内存索引 + 内存图的最小执行器
/// （生产由各域提供完整实现；本实现用于验证 DSL 契约与端到端通路）
pub struct InMemoryQueryExecutor {
    meta: std::sync::Arc<dyn MetadataIndex>,
    graph: std::sync::Arc<dyn GraphTraversal>,
}

impl InMemoryQueryExecutor {
    /// 新建内存查询执行器
    pub fn new(
        meta: std::sync::Arc<dyn MetadataIndex>,
        graph: std::sync::Arc<dyn GraphTraversal>,
    ) -> Self {
        Self { meta, graph }
    }
}

#[async_trait]
impl QueryExecutor for InMemoryQueryExecutor {
    async fn execute(&self, query: &QueryPrimitive) -> QueryResult<Vec<QueryHit>> {
        match query {
            QueryPrimitive::Get { id } => {
                // GET 委托元数据索引过滤 id 字段（简化：按属性 id 匹配）
                let hits = self
                    .meta
                    .filter("id", &serde_json::json!(id))
                    .await
                    .map_err(|e| QueryError::Execution(e.to_string()))?;
                Ok(hits
                    .into_iter()
                    .map(|hid| QueryHit {
                        id: hid,
                        kind: "node".into(),
                        props: std::collections::HashMap::new(),
                    })
                    .collect())
            }
            QueryPrimitive::Filter { prop, value } => {
                let hits = self
                    .meta
                    .filter(prop, value)
                    .await
                    .map_err(|e| QueryError::Execution(e.to_string()))?;
                Ok(hits
                    .into_iter()
                    .map(|hid| QueryHit {
                        id: hid,
                        kind: "node".into(),
                        props: std::collections::HashMap::new(),
                    })
                    .collect())
            }
            QueryPrimitive::Traverse {
                from,
                edge_type,
                hops,
            } => {
                // 简化：from 为字符串 ID 时走图遍历（需要 Id；此处构造占位 Id）
                let id = mox_base_model_core::Id::new("kg", mox_base_model_core::EntityKind::Node);
                // 注意：真实实现应将 from 解析为 Id 并调用 graph；此处用字符串等价
                let _ = (id, from);
                let reached = self
                    .graph
                    .traverse(
                        &mox_base_model_core::Id::new("kg", mox_base_model_core::EntityKind::Node),
                        edge_type,
                        *hops,
                    )
                    .await
                    .map_err(|e| QueryError::Execution(e.to_string()))?;
                Ok(reached
                    .into_iter()
                    .map(|rid| QueryHit {
                        id: rid,
                        kind: "node".into(),
                        props: std::collections::HashMap::new(),
                    })
                    .collect())
            }
            QueryPrimitive::Range { path, .. } => {
                // RANGE 委托物理存储（此处返回占位，真实由 mox-base-store-core ObjectStore 提供）
                Ok(vec![QueryHit {
                    id: path.clone(),
                    kind: "blob".into(),
                    props: std::collections::HashMap::new(),
                }])
            }
        }
    }

    async fn execute_all(&self, queries: &[QueryPrimitive]) -> QueryResult<Vec<QueryHit>> {
        let mut all = Vec::new();
        for q in queries {
            all.extend(self.execute(q).await?);
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_base_graph_core::InMemoryGraph;
    use mox_base_index_core::{InMemoryMetadataIndex, IndexEntry};

    #[tokio::test]
    async fn builder_constructs_all_four_primitives() {
        let qs = QueryBuilder::new()
            .get("n1")
            .filter("domain", serde_json::json!("kg"))
            .traverse("a", "contains", 2)
            .range("kg/x.png", 0, 1024)
            .build();
        assert_eq!(qs.len(), 4);
        assert!(matches!(qs[0], QueryPrimitive::Get { .. }));
        assert!(matches!(qs[1], QueryPrimitive::Filter { .. }));
        assert!(matches!(qs[2], QueryPrimitive::Traverse { .. }));
        assert!(matches!(qs[3], QueryPrimitive::Range { .. }));
    }

    #[tokio::test]
    async fn executor_filter_roundtrip() {
        let meta = std::sync::Arc::new(InMemoryMetadataIndex::new());
        let graph = std::sync::Arc::new(InMemoryGraph::new());
        let mut props = std::collections::HashMap::new();
        props.insert("id".to_string(), serde_json::json!("n1"));
        props.insert("domain".to_string(), serde_json::json!("kg"));
        meta.put(IndexEntry {
            id: "n1".into(),
            kind: "node".into(),
            props,
            text: None,
            vector: None,
        })
        .await
        .unwrap();
        let ex = InMemoryQueryExecutor::new(meta, graph);
        let hits = ex
            .execute(&QueryPrimitive::Filter {
                prop: "domain".into(),
                value: serde_json::json!("kg"),
            })
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[tokio::test]
    async fn builder_default_is_empty() {
        assert!(QueryBuilder::new().build().is_empty());
    }
}
