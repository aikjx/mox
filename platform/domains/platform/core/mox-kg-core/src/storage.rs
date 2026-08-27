// mox-kg-core 存储层：基于SQLite的图存储（点表+边表+索引）
// 与mox-dsql-core技术栈统一，无需LLVM/clang原生依赖

use crate::error::{KgError, KgResult};
use crate::model::{Edge, TraverseDirection, Vertex};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;

/// 图存储引擎
pub struct GraphStorage {
    conn: Arc<parking_lot::Mutex<Connection>>,
}

impl GraphStorage {
    /// 打开或创建图数据库
    pub fn open<P: AsRef<Path>>(path: P) -> KgResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| KgError::StorageError(format!("open db: {e}")))?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(parking_lot::Mutex::new(conn)),
        })
    }

    /// 打开内存模式（测试用）
    pub fn open_memory() -> KgResult<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| KgError::StorageError(format!("open memory: {e}")))?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(parking_lot::Mutex::new(conn)),
        })
    }

    /// 初始化数据库表结构
    fn init_schema(conn: &Connection) -> KgResult<()> {
        // 顶点表
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS kg_vertex (
                id TEXT PRIMARY KEY,
                vertex_type TEXT NOT NULL,
                properties TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_vertex_type ON kg_vertex(vertex_type);

            CREATE TABLE IF NOT EXISTS kg_edge (
                id TEXT PRIMARY KEY,
                edge_type TEXT NOT NULL,
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                properties TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_edge_source ON kg_edge(source);
            CREATE INDEX IF NOT EXISTS idx_edge_target ON kg_edge(target);
            CREATE INDEX IF NOT EXISTS idx_edge_type ON kg_edge(edge_type);
            CREATE INDEX IF NOT EXISTS idx_edge_source_type ON kg_edge(source, edge_type);
            CREATE INDEX IF NOT EXISTS idx_edge_target_type ON kg_edge(target, edge_type);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_edge_unique ON kg_edge(edge_type, source, target);
            "#,
        )
        .map_err(|e| KgError::StorageError(format!("init schema: {e}")))?;
        Ok(())
    }

    // ==================== 顶点操作 ====================

    /// 插入顶点（如果已存在则返回错误）
    pub fn put_vertex(&self, vertex: &Vertex) -> KgResult<()> {
        let conn = self.conn.lock();
        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM kg_vertex WHERE id = ?1", params![vertex.id], |r| r.get::<_, i64>(0))
            .map_err(|e| KgError::StorageError(e.to_string()))?
            > 0;
        if exists {
            return Err(KgError::VertexAlreadyExists(vertex.id.clone()));
        }
        drop(conn);
        self.upsert_vertex(vertex)
    }

    /// 插入或更新顶点
    pub fn upsert_vertex(&self, vertex: &Vertex) -> KgResult<()> {
        let conn = self.conn.lock();
        let properties = serde_json::to_string(&vertex.properties)
            .map_err(|e| KgError::SerializeError(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO kg_vertex (id, vertex_type, properties, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![vertex.id, vertex.vertex_type, properties, vertex.created_at, vertex.updated_at],
        )
        .map_err(|e| KgError::StorageError(format!("upsert vertex: {e}")))?;
        Ok(())
    }

    /// 获取顶点
    pub fn get_vertex(&self, vertex_id: &str) -> KgResult<Option<Vertex>> {
        let conn = self.conn.lock();
        let result = conn
            .query_row(
                "SELECT id, vertex_type, properties, created_at, updated_at FROM kg_vertex WHERE id = ?1",
                params![vertex_id],
                Self::row_to_vertex,
            )
            .optional()
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        Ok(result)
    }

    /// 按类型和ID获取顶点
    pub fn get_vertex_typed(&self, vertex_type: &str, vertex_id: &str) -> KgResult<Option<Vertex>> {
        let full_id = format!("{}:{}", vertex_type, vertex_id);
        self.get_vertex(&full_id)
    }

    /// 删除顶点（级联删除相关边）
    pub fn delete_vertex(&self, vertex_id: &str) -> KgResult<()> {
        let conn = self.conn.lock();
        // 检查是否存在
        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM kg_vertex WHERE id = ?1", params![vertex_id], |r| r.get::<_, i64>(0))
            .map_err(|e| KgError::StorageError(e.to_string()))?
            > 0;
        if !exists {
            return Err(KgError::VertexNotFound(vertex_id.to_string()));
        }
        // 删除相关边（source或target）
        conn.execute("DELETE FROM kg_edge WHERE source = ?1 OR target = ?1", params![vertex_id])
            .map_err(|e| KgError::StorageError(format!("delete edges: {e}")))?;
        // 删除顶点
        conn.execute("DELETE FROM kg_vertex WHERE id = ?1", params![vertex_id])
            .map_err(|e| KgError::StorageError(format!("delete vertex: {e}")))?;
        Ok(())
    }

    /// 按类型列出所有顶点
    pub fn list_vertices_by_type(&self, vertex_type: &str, limit: Option<usize>, offset: Option<usize>) -> KgResult<Vec<Vertex>> {
        let conn = self.conn.lock();
        let limit = limit.unwrap_or(usize::MAX) as i64;
        let offset = offset.unwrap_or(0) as i64;
        let mut stmt = conn
            .prepare("SELECT id, vertex_type, properties, created_at, updated_at FROM kg_vertex WHERE vertex_type = ?1 ORDER BY id LIMIT ?2 OFFSET ?3")
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        let vertices = stmt
            .query_map(params![vertex_type, limit, offset], Self::row_to_vertex)
            .map_err(|e| KgError::StorageError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(vertices)
    }

    /// 统计某类型顶点数量
    pub fn count_vertices_by_type(&self, vertex_type: &str) -> KgResult<usize> {
        let conn = self.conn.lock();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kg_vertex WHERE vertex_type = ?1", params![vertex_type], |r| r.get(0))
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        Ok(count as usize)
    }

    /// 列出所有顶点
    pub fn list_all_vertices(&self) -> KgResult<Vec<Vertex>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, vertex_type, properties, created_at, updated_at FROM kg_vertex ORDER BY id")
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        let vertices = stmt
            .query_map([], Self::row_to_vertex)
            .map_err(|e| KgError::StorageError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(vertices)
    }

    // ==================== 边操作 ====================

    /// 插入边（如果已存在则返回错误）
    pub fn put_edge(&self, edge: &Edge) -> KgResult<()> {
        let conn = self.conn.lock();
        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM kg_edge WHERE id = ?1", params![edge.id], |r| r.get::<_, i64>(0))
            .map_err(|e| KgError::StorageError(e.to_string()))?
            > 0;
        if exists {
            return Err(KgError::EdgeAlreadyExists(edge.id.clone()));
        }
        drop(conn);
        self.upsert_edge(edge)
    }

    /// 插入或更新边
    pub fn upsert_edge(&self, edge: &Edge) -> KgResult<()> {
        let conn = self.conn.lock();
        let properties = serde_json::to_string(&edge.properties)
            .map_err(|e| KgError::SerializeError(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO kg_edge (id, edge_type, source, target, properties, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![edge.id, edge.edge_type, edge.source, edge.target, properties, edge.created_at],
        )
        .map_err(|e| KgError::StorageError(format!("upsert edge: {e}")))?;
        Ok(())
    }

    /// 获取边
    pub fn get_edge(&self, edge_type: &str, source: &str, target: &str) -> KgResult<Option<Edge>> {
        let id = format!("edge:{}:{}:{}", edge_type, source, target);
        let conn = self.conn.lock();
        let result = conn
            .query_row(
                "SELECT id, edge_type, source, target, properties, created_at FROM kg_edge WHERE id = ?1",
                params![id],
                Self::row_to_edge,
            )
            .optional()
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        Ok(result)
    }

    /// 删除边
    pub fn delete_edge(&self, edge_type: &str, source: &str, target: &str) -> KgResult<()> {
        let id = format!("edge:{}:{}:{}", edge_type, source, target);
        let conn = self.conn.lock();
        let affected = conn
            .execute("DELETE FROM kg_edge WHERE id = ?1", params![id])
            .map_err(|e| KgError::StorageError(format!("delete edge: {e}")))?;
        if affected == 0 {
            return Err(KgError::EdgeNotFound(id));
        }
        Ok(())
    }

    /// 遍历顶点的邻接边
    pub fn traverse_edges(
        &self,
        vertex_id: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
    ) -> KgResult<Vec<Edge>> {
        let conn = self.conn.lock();
        let mut edges = vec![];

        let directions = match direction {
            TraverseDirection::Out => vec![TraverseDirection::Out],
            TraverseDirection::In => vec![TraverseDirection::In],
            TraverseDirection::Both => vec![TraverseDirection::Out, TraverseDirection::In],
        };

        for dir in directions {
            let (col, param_col) = match dir {
                TraverseDirection::Out => ("source", "source"),
                TraverseDirection::In => ("target", "target"),
                _ => continue,
            };

            let sql = if let Some(types) = edge_types {
                if types.is_empty() {
                    format!("SELECT id, edge_type, source, target, properties, created_at FROM kg_edge WHERE {col} = ?1")
                } else {
                    let placeholders: Vec<String> = types.iter().map(|_| "?".to_string()).collect();
                    format!(
                        "SELECT id, edge_type, source, target, properties, created_at FROM kg_edge WHERE {col} = ?1 AND edge_type IN ({})",
                        placeholders.join(",")
                    )
                }
            } else {
                format!("SELECT id, edge_type, source, target, properties, created_at FROM kg_edge WHERE {col} = ?1")
            };

            let mut stmt = conn.prepare(&sql).map_err(|e| KgError::StorageError(e.to_string()))?;

            let params: Vec<rusqlite::types::Value> = if let Some(types) = edge_types {
                if types.is_empty() {
                    vec![rusqlite::types::Value::Text(vertex_id.to_string())]
                } else {
                    let mut p = vec![rusqlite::types::Value::Text(vertex_id.to_string())];
                    for t in types {
                        p.push(rusqlite::types::Value::Text(t.clone()));
                    }
                    p
                }
            } else {
                vec![rusqlite::types::Value::Text(vertex_id.to_string())]
            };

            let rows = stmt
                .query_map(rusqlite::params_from_iter(params.iter()), Self::row_to_edge)
                .map_err(|e| KgError::StorageError(e.to_string()))?;

            for row in rows {
                if let Ok(edge) = row {
                    edges.push(edge);
                }
            }
        }

        Ok(edges)
    }

    /// 遍历顶点的邻接顶点
    pub fn traverse_vertices(
        &self,
        vertex_id: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
    ) -> KgResult<Vec<Vertex>> {
        let edges = self.traverse_edges(vertex_id, direction, edge_types)?;
        let mut vertices = vec![];
        let mut seen = std::collections::HashSet::new();

        for edge in edges {
            let other_id = if edge.source == vertex_id {
                edge.target.clone()
            } else {
                edge.source.clone()
            };

            if seen.insert(other_id.clone()) {
                if let Some(v) = self.get_vertex(&other_id)? {
                    vertices.push(v);
                }
            }
        }

        Ok(vertices)
    }

    /// 多跳遍历（BFS）
    pub fn multi_hop_traverse(
        &self,
        start_vertex_id: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
        max_depth: usize,
    ) -> KgResult<Vec<(Vertex, usize)>> {
        let mut results = vec![];
        let mut visited = std::collections::HashSet::new();
        let mut current_level = vec![start_vertex_id.to_string()];
        visited.insert(start_vertex_id.to_string());

        for depth in 1..=max_depth {
            let mut next_level = vec![];
            for vid in &current_level {
                let neighbors = self.traverse_vertices(vid, direction, edge_types)?;
                for v in neighbors {
                    if visited.insert(v.id.clone()) {
                        results.push((v.clone(), depth));
                        next_level.push(v.id);
                    }
                }
            }
            current_level = next_level;
            if current_level.is_empty() {
                break;
            }
        }

        Ok(results)
    }

    /// 查找两顶点之间的路径（BFS最短路径）
    pub fn find_path(
        &self,
        source_id: &str,
        target_id: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
        max_depth: usize,
    ) -> KgResult<Option<Vec<(Vertex, Edge)>>> {
        if source_id == target_id {
            return Ok(Some(vec![]));
        }

        let mut visited: std::collections::HashMap<String, Option<(String, Edge)>> = std::collections::HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((source_id.to_string(), 0usize));
        visited.insert(source_id.to_string(), None);

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let edges = self.traverse_edges(&current, direction, edge_types)?;
            for edge in edges {
                let next = if edge.source == current {
                    edge.target.clone()
                } else {
                    edge.source.clone()
                };

                if visited.contains_key(&next) {
                    continue;
                }

                visited.insert(next.clone(), Some((current.clone(), edge.clone())));

                if next == target_id {
                    let mut path = vec![];
                    let mut cur = target_id.to_string();
                    while cur != source_id {
                        if let Some(Some((parent, e))) = visited.get(&cur) {
                            if let Some(v) = self.get_vertex(&cur)? {
                                path.push((v, e.clone()));
                            }
                            cur = parent.clone();
                        } else {
                            break;
                        }
                    }
                    path.reverse();
                    return Ok(Some(path));
                }

                queue.push_back((next, depth + 1));
            }
        }

        Ok(None)
    }

    /// 获取数据库统计信息
    pub fn stats(&self) -> KgResult<serde_json::Value> {
        let conn = self.conn.lock();
        let vertex_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kg_vertex", [], |r| r.get(0))
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        let edge_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kg_edge", [], |r| r.get(0))
            .map_err(|e| KgError::StorageError(e.to_string()))?;

        // 统计各类型顶点数
        let mut stmt = conn
            .prepare("SELECT vertex_type, COUNT(*) as cnt FROM kg_vertex GROUP BY vertex_type ORDER BY cnt DESC")
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        let mut type_counts = serde_json::Map::new();
        let rows = stmt
            .query_map([], |r| {
                let vertex_type: String = r.get(0)?;
                let cnt: i64 = r.get(1)?;
                Ok((vertex_type, cnt))
            })
            .map_err(|e| KgError::StorageError(e.to_string()))?;
        for row in rows {
            if let Ok((vt, cnt)) = row {
                type_counts.insert(vt, serde_json::json!(cnt));
            }
        }

        Ok(serde_json::json!({
            "vertex_count": vertex_count,
            "edge_count": edge_count,
            "vertex_types": type_counts,
        }))
    }

    // ==================== 行映射辅助 ====================

    fn row_to_vertex(r: &rusqlite::Row) -> rusqlite::Result<Vertex> {
        let properties_str: String = r.get(2)?;
        let properties: serde_json::Value = serde_json::from_str(&properties_str).unwrap_or(serde_json::json!({}));
        Ok(Vertex {
            id: r.get(0)?,
            vertex_type: r.get(1)?,
            properties,
            created_at: r.get(3)?,
            updated_at: r.get(4)?,
        })
    }

    fn row_to_edge(r: &rusqlite::Row) -> rusqlite::Result<Edge> {
        let properties_str: String = r.get(4)?;
        let properties: serde_json::Value = serde_json::from_str(&properties_str).unwrap_or(serde_json::json!({}));
        Ok(Edge {
            id: r.get(0)?,
            edge_type: r.get(1)?,
            source: r.get(2)?,
            target: r.get(3)?,
            properties,
            created_at: r.get(5)?,
        })
    }
}

impl Clone for GraphStorage {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}
