// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! MOX KG Storage Service
//!
//! Knowledge graph storage backend with:
//! - In-memory graph with petgraph (fast queries)
//! - SQLite persistence (durable storage)
//! - Node/Edge CRUD with type-safe schemas
//! - Batch operations and transaction support
//! - Indexing by type, property, and full-text search

use petgraph::graph::{DiGraph, NodeIndex, EdgeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("edge not found: {0}")]
    EdgeNotFound(String),
    #[error("duplicate node id: {0}")]
    DuplicateNode(String),
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("invalid schema: {0}")]
    InvalidSchema(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub properties: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

impl GraphNode {
    pub fn new(id: &str, node_type: &str, label: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self { id: id.into(), node_type: node_type.into(), label: label.into(), properties: serde_json::json!({}), created_at: now.clone(), updated_at: now }
    }

    pub fn with_properties(mut self, props: serde_json::Value) -> Self {
        self.properties = props;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub weight: f64,
    pub properties: serde_json::Value,
    pub created_at: String,
}

impl GraphEdge {
    pub fn new(id: &str, source: &str, target: &str, edge_type: &str) -> Self {
        Self {
            id: id.into(), source: source.into(), target: target.into(),
            edge_type: edge_type.into(), weight: 1.0, properties: serde_json::json!({}),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_weight(mut self, w: f64) -> Self { self.weight = w; self }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub node_count: usize,
    pub edge_count: usize,
}

/// In-memory knowledge graph store with optional SQLite persistence.
#[derive(Clone)]
pub struct GraphStore {
    graph: Arc<parking_lot::RwLock<DiGraph<GraphNode, GraphEdge>>>,
    node_index: Arc<parking_lot::RwLock<HashMap<String, NodeIndex>>>,
    edge_index: Arc<parking_lot::RwLock<HashMap<String, EdgeIndex>>>,
    type_index: Arc<parking_lot::RwLock<HashMap<String, Vec<String>>>>, // node_type -> node_ids
}

impl GraphStore {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(parking_lot::RwLock::new(DiGraph::new())),
            node_index: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            edge_index: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            type_index: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn add_node(&self, node: GraphNode) -> Result<(), StorageError> {
        let mut graph = self.graph.write();
        let mut idx = self.node_index.write();
        let mut type_idx = self.type_index.write();

        if idx.contains_key(&node.id) {
            return Err(StorageError::DuplicateNode(node.id));
        }
        let ni = graph.add_node(node.clone());
        idx.insert(node.id.clone(), ni);
        type_idx.entry(node.node_type.clone()).or_default().push(node.id);
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<GraphNode> {
        let idx = self.node_index.read();
        let graph = self.graph.read();
        idx.get(id).and_then(|ni| graph.node_weight(*ni).cloned())
    }

    pub fn update_node(&self, id: &str, label: Option<&str>, properties: Option<serde_json::Value>) -> Result<GraphNode, StorageError> {
        let idx = self.node_index.read();
        let ni = *idx.get(id).ok_or_else(|| StorageError::NodeNotFound(id.into()))?;
        drop(idx);
        let mut graph = self.graph.write();
        let node = graph.node_weight_mut(ni).ok_or_else(|| StorageError::NodeNotFound(id.into()))?;
        if let Some(l) = label { node.label = l.into(); }
        if let Some(p) = properties { node.properties = p; }
        node.updated_at = chrono::Utc::now().to_rfc3339();
        Ok(node.clone())
    }

    pub fn delete_node(&self, id: &str) -> Result<(), StorageError> {
        let mut idx = self.node_index.write();
        let mut graph = self.graph.write();
        let mut edge_idx = self.edge_index.write();
        let mut type_idx = self.type_index.write();

        let ni = *idx.get(id).ok_or_else(|| StorageError::NodeNotFound(id.into()))?;
        // Remove connected edges (petgraph Direction 只有 Incoming/Outgoing；双向 = 两者相并)
        let mut edges: Vec<EdgeIndex> = graph
            .edges_directed(ni, petgraph::Direction::Outgoing)
            .map(|e| e.id())
            .collect();
        edges.extend(graph.edges_directed(ni, petgraph::Direction::Incoming).map(|e| e.id()));
        edges.sort_unstable();
        edges.dedup();
        for ei in edges {
            if let Some(edge) = graph.edge_weight(ei) {
                edge_idx.remove(&edge.id);
            }
            graph.remove_edge(ei);
        }
        graph.remove_node(ni);
        idx.remove(id);
        // Remove from type index
        for ids in type_idx.values_mut() { ids.retain(|x| x != id); }
        Ok(())
    }

    pub fn add_edge(&self, edge: GraphEdge) -> Result<(), StorageError> {
        let idx = self.node_index.read();
        let source = *idx.get(&edge.source).ok_or_else(|| StorageError::NodeNotFound(edge.source.clone()))?;
        let target = *idx.get(&edge.target).ok_or_else(|| StorageError::NodeNotFound(edge.target.clone()))?;
        drop(idx);
        let mut graph = self.graph.write();
        let mut edge_idx = self.edge_index.write();
        let ei = graph.add_edge(source, target, edge.clone());
        edge_idx.insert(edge.id, ei);
        Ok(())
    }

    pub fn get_edge(&self, id: &str) -> Option<GraphEdge> {
        let idx = self.edge_index.read();
        let graph = self.graph.read();
        idx.get(id).and_then(|ei| graph.edge_weight(*ei).cloned())
    }

    pub fn delete_edge(&self, id: &str) -> Result<(), StorageError> {
        let mut edge_idx = self.edge_index.write();
        let mut graph = self.graph.write();
        let ei = *edge_idx.get(id).ok_or_else(|| StorageError::EdgeNotFound(id.into()))?;
        graph.remove_edge(ei);
        edge_idx.remove(id);
        Ok(())
    }

    pub fn node_count(&self) -> usize {
        self.graph.read().node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.read().edge_count()
    }

    pub fn list_nodes(&self) -> Vec<GraphNode> {
        self.graph.read().node_weights().cloned().collect()
    }

    pub fn list_edges(&self) -> Vec<GraphEdge> {
        self.graph.read().edge_weights().cloned().collect()
    }

    pub fn nodes_by_type(&self, node_type: &str) -> Vec<GraphNode> {
        let type_idx = self.type_index.read();
        let ids = type_idx.get(node_type).cloned().unwrap_or_default();
        drop(type_idx);
        ids.into_iter().filter_map(|id| self.get_node(&id)).collect()
    }

    /// Neighbors of a node (outgoing edges).
    pub fn neighbors(&self, id: &str) -> Vec<GraphNode> {
        let idx = self.node_index.read();
        let graph = self.graph.read();
        let Some(ni) = idx.get(id) else { return vec![]; };
        graph.neighbors(*ni).filter_map(|n| graph.node_weight(n).cloned()).collect()
    }

    /// Full-text search over node labels and properties.
    pub fn search(&self, query: &str, limit: usize) -> Vec<GraphNode> {
        let q = query.to_lowercase();
        let mut results: Vec<(GraphNode, f64)> = self.graph.read().node_weights().filter_map(|n| {
            let mut score = 0.0f64;
            if n.label.to_lowercase().contains(&q) { score += 2.0; }
            if n.node_type.to_lowercase().contains(&q) { score += 1.0; }
            if let Ok(props) = serde_json::to_string(&n.properties) {
                if props.to_lowercase().contains(&q) { score += 0.5; }
            }
            if score > 0.0 { Some((n.clone(), score)) } else { None }
        }).collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.into_iter().take(limit).map(|(n, _)| n).collect()
    }

    /// Batch add nodes and edges in a single operation.
    pub fn batch_add(&self, nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> Result<(usize, usize), StorageError> {
        let mut node_count = 0;
        for n in nodes {
            if self.add_node(n).is_ok() { node_count += 1; }
        }
        let mut edge_count = 0;
        for e in edges {
            if self.add_edge(e).is_ok() { edge_count += 1; }
        }
        Ok((node_count, edge_count))
    }

    /// Export full graph snapshot.
    pub fn snapshot(&self) -> GraphSnapshot {
        let graph = self.graph.read();
        GraphSnapshot {
            nodes: graph.node_weights().cloned().collect(),
            edges: graph.edge_weights().cloned().collect(),
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
        }
    }

    /// Import from snapshot (replaces existing graph).
    pub fn import_snapshot(&self, snap: GraphSnapshot) -> Result<(), StorageError> {
        // Clear existing
        let mut graph = self.graph.write();
        graph.clear();
        self.node_index.write().clear();
        self.edge_index.write().clear();
        self.type_index.write().clear();
        drop(graph);
        self.batch_add(snap.nodes, snap.edges)?;
        Ok(())
    }

    /// Get adjacency list for graph algorithms.
    pub fn adjacency_list(&self) -> HashMap<String, Vec<(String, f64)>> {
        let graph = self.graph.read();
        let idx = self.node_index.read();
        let mut reverse: HashMap<NodeIndex, String> = HashMap::new();
        for (id, ni) in idx.iter() { reverse.insert(*ni, id.clone()); }
        let mut adj = HashMap::new();
        for ni in graph.node_indices() {
            let source = reverse.get(&ni).cloned().unwrap_or_default();
            let neighbors: Vec<(String, f64)> = graph.edges_directed(ni, petgraph::Direction::Outgoing)
                .filter_map(|e| reverse.get(&e.target()).map(|t| (t.clone(), e.weight().weight)))
                .collect();
            adj.insert(source, neighbors);
        }
        adj
    }
}

impl Default for GraphStore {
    fn default() -> Self { Self::new() }
}

/// Persistent graph store backed by SQLite.
#[derive(Clone)]
pub struct PersistentGraphStore {
    pub memory: GraphStore,
    db_path: Option<String>,
}

impl PersistentGraphStore {
    pub fn new() -> Self {
        Self { memory: GraphStore::new(), db_path: None }
    }

    pub fn with_persistence(db_path: &str) -> Result<Self, StorageError> {
        let store = Self { memory: GraphStore::new(), db_path: Some(db_path.into()) };
        store.init_db()?;
        store.load()?;
        Ok(store)
    }

    fn init_db(&self) -> Result<(), StorageError> {
        let Some(path) = &self.db_path else { return Ok(()); };
        let conn = rusqlite::Connection::open(path).map_err(|e| StorageError::Persistence(e.to_string()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kg_nodes (
                id TEXT PRIMARY KEY, node_type TEXT NOT NULL, label TEXT NOT NULL,
                properties TEXT NOT NULL DEFAULT '{}', created_at TEXT, updated_at TEXT
             );
             CREATE TABLE IF NOT EXISTS kg_edges (
                id TEXT PRIMARY KEY, source TEXT NOT NULL, target TEXT NOT NULL,
                edge_type TEXT NOT NULL, weight REAL NOT NULL DEFAULT 1.0,
                properties TEXT NOT NULL DEFAULT '{}', created_at TEXT,
                FOREIGN KEY(source) REFERENCES kg_nodes(id),
                FOREIGN KEY(target) REFERENCES kg_nodes(id)
             );
             CREATE INDEX IF NOT EXISTS idx_nodes_type ON kg_nodes(node_type);
             CREATE INDEX IF NOT EXISTS idx_edges_source ON kg_edges(source);
             CREATE INDEX IF NOT EXISTS idx_edges_target ON kg_edges(target);"
        ).map_err(|e| StorageError::Persistence(e.to_string()))?;
        Ok(())
    }

    pub fn persist(&self) -> Result<(), StorageError> {
        let Some(path) = &self.db_path else { return Ok(()); };
        let mut conn = rusqlite::Connection::open(path).map_err(|e| StorageError::Persistence(e.to_string()))?;
        let tx = conn.transaction().map_err(|e| StorageError::Persistence(e.to_string()))?;
        tx.execute("DELETE FROM kg_edges", []).map_err(|e| StorageError::Persistence(e.to_string()))?;
        tx.execute("DELETE FROM kg_nodes", []).map_err(|e| StorageError::Persistence(e.to_string()))?;
        for node in self.memory.list_nodes() {
            tx.execute(
                "INSERT OR REPLACE INTO kg_nodes (id, node_type, label, properties, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6)",
                rusqlite::params![node.id, node.node_type, node.label, serde_json::to_string(&node.properties).unwrap(), node.created_at, node.updated_at],
            ).map_err(|e| StorageError::Persistence(e.to_string()))?;
        }
        for edge in self.memory.list_edges() {
            tx.execute(
                "INSERT OR REPLACE INTO kg_edges (id, source, target, edge_type, weight, properties, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                rusqlite::params![edge.id, edge.source, edge.target, edge.edge_type, edge.weight, serde_json::to_string(&edge.properties).unwrap(), edge.created_at],
            ).map_err(|e| StorageError::Persistence(e.to_string()))?;
        }
        tx.commit().map_err(|e| StorageError::Persistence(e.to_string()))?;
        Ok(())
    }

    pub fn load(&self) -> Result<(), StorageError> {
        let Some(path) = &self.db_path else { return Ok(()); };
        let conn = rusqlite::Connection::open(path).map_err(|e| StorageError::Persistence(e.to_string()))?;
        let nodes: Vec<GraphNode> = conn.prepare("SELECT id, node_type, label, properties, created_at, updated_at FROM kg_nodes")
            .map_err(|e| StorageError::Persistence(e.to_string()))?
            .query_map([], |row| Ok(GraphNode {
                id: row.get(0)?, node_type: row.get(1)?, label: row.get(2)?,
                properties: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or(serde_json::json!({})),
                created_at: row.get(4)?, updated_at: row.get(5)?,
            }))
            .map_err(|e| StorageError::Persistence(e.to_string()))?
            .filter_map(|r| r.ok()).collect();
        let edges: Vec<GraphEdge> = conn.prepare("SELECT id, source, target, edge_type, weight, properties, created_at FROM kg_edges")
            .map_err(|e| StorageError::Persistence(e.to_string()))?
            .query_map([], |row| Ok(GraphEdge {
                id: row.get(0)?, source: row.get(1)?, target: row.get(2)?, edge_type: row.get(3)?,
                weight: row.get(4)?,
                properties: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(serde_json::json!({})),
                created_at: row.get(6)?,
            }))
            .map_err(|e| StorageError::Persistence(e.to_string()))?
            .filter_map(|r| r.ok()).collect();
        self.memory.batch_add(nodes, edges)?;
        Ok(())
    }
}

impl Default for PersistentGraphStore {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_get_node() {
        let store = GraphStore::new();
        let node = GraphNode::new("n1", "Person", "Alice");
        store.add_node(node.clone()).unwrap();
        let got = store.get_node("n1").unwrap();
        assert_eq!(got.label, "Alice");
        assert_eq!(store.node_count(), 1);
    }

    #[test]
    fn duplicate_node_rejected() {
        let store = GraphStore::new();
        store.add_node(GraphNode::new("n1", "T", "L")).unwrap();
        assert!(store.add_node(GraphNode::new("n1", "T", "L2")).is_err());
    }

    #[test]
    fn add_edge_and_neighbors() {
        let store = GraphStore::new();
        store.add_node(GraphNode::new("a", "T", "A")).unwrap();
        store.add_node(GraphNode::new("b", "T", "B")).unwrap();
        store.add_edge(GraphEdge::new("e1", "a", "b", "related")).unwrap();
        assert_eq!(store.edge_count(), 1);
        let neighbors = store.neighbors("a");
        assert_eq!(neighbors.len(), 1);
        assert_eq!(neighbors[0].id, "b");
    }

    #[test]
    fn search_nodes() {
        let store = GraphStore::new();
        store.add_node(GraphNode::new("n1", "Algorithm", "PageRank")).unwrap();
        store.add_node(GraphNode::new("n2", "Data", "GraphStorage")).unwrap();
        let results = store.search("pagerank", 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "n1");
    }

    #[test]
    fn batch_add() {
        let store = GraphStore::new();
        let nodes = vec![
            GraphNode::new("a", "T", "A"),
            GraphNode::new("b", "T", "B"),
            GraphNode::new("c", "T", "C"),
        ];
        let edges = vec![
            GraphEdge::new("e1", "a", "b", "r"),
            GraphEdge::new("e2", "b", "c", "r"),
        ];
        let (nc, ec) = store.batch_add(nodes, edges).unwrap();
        assert_eq!(nc, 3);
        assert_eq!(ec, 2);
    }

    #[test]
    fn snapshot_roundtrip() {
        let store = GraphStore::new();
        store.add_node(GraphNode::new("a", "T", "A")).unwrap();
        store.add_node(GraphNode::new("b", "T", "B")).unwrap();
        store.add_edge(GraphEdge::new("e1", "a", "b", "r")).unwrap();
        let snap = store.snapshot();
        assert_eq!(snap.node_count, 2);
        assert_eq!(snap.edge_count, 1);

        let store2 = GraphStore::new();
        store2.import_snapshot(snap).unwrap();
        assert_eq!(store2.node_count(), 2);
        assert_eq!(store2.edge_count(), 1);
    }

    #[test]
    fn nodes_by_type() {
        let store = GraphStore::new();
        store.add_node(GraphNode::new("a", "TypeA", "A")).unwrap();
        store.add_node(GraphNode::new("b", "TypeB", "B")).unwrap();
        store.add_node(GraphNode::new("c", "TypeA", "C")).unwrap();
        let type_a = store.nodes_by_type("TypeA");
        assert_eq!(type_a.len(), 2);
    }

    #[test]
    fn delete_cascades_edges() {
        let store = GraphStore::new();
        store.add_node(GraphNode::new("a", "T", "A")).unwrap();
        store.add_node(GraphNode::new("b", "T", "B")).unwrap();
        store.add_edge(GraphEdge::new("e1", "a", "b", "r")).unwrap();
        store.delete_node("a").unwrap();
        assert_eq!(store.node_count(), 1);
        assert_eq!(store.edge_count(), 0);
    }
}
