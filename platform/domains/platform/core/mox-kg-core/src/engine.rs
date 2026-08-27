// mox-kg-core 查询引擎：执行DSL查询、多跳遍历、聚合、路径查找

use crate::dsl::{DslParser, DslQuery, DslQueryType};
use crate::error::{KgError, KgResult};
use crate::model::{Edge, PathResult, QueryResult, TraverseDirection, Vertex};
use crate::storage::GraphStorage;
use std::time::Instant;

/// 图查询引擎
pub struct QueryEngine {
    storage: GraphStorage,
}

impl QueryEngine {
    pub fn new(storage: GraphStorage) -> Self {
        Self { storage }
    }

    /// 执行DSL查询
    pub fn execute_dsl(&self, dsl: &str) -> KgResult<QueryResult> {
        let start = Instant::now();
        let query = match DslParser::parse(dsl) {
            Ok(q) => q,
            Err(e) => {
                let mut result = QueryResult::error("dsl", e.to_string());
                result.duration_ms = start.elapsed().as_millis() as u64;
                return Ok(result);
            }
        };

        let mut result = match query.query_type {
            DslQueryType::Get => self.execute_get(&query)?,
            DslQueryType::Match => self.execute_match(&query)?,
            DslQueryType::Search => self.execute_search(&query)?,
            DslQueryType::Count => self.execute_count(&query)?,
            DslQueryType::Delete => self.execute_delete(&query)?,
        };

        result.duration_ms = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    /// 执行GET查询
    fn execute_get(&self, query: &DslQuery) -> KgResult<QueryResult> {
        let mut result = QueryResult::success("get");

        if query.path_segments.is_empty() {
            // 简单查询：按类型+条件获取顶点
            let vertices = self.query_vertices_simple(query)?;
            result.total = vertices.len();
            result.vertices = vertices;
        } else {
            // 多跳路径查询
            let vertices = self.query_vertices_path(query)?;
            result.total = vertices.len();
            result.vertices = vertices;
        }

        Ok(result)
    }

    /// 简单顶点查询
    fn query_vertices_simple(&self, query: &DslQuery) -> KgResult<Vec<Vertex>> {
        let target_type = &query.target_type;

        let vertices = if target_type == "*" {
            // 遍历所有类型（简化实现：遍历所有vertex前缀）
            self.list_all_vertices()?
        } else {
            self.storage.list_vertices_by_type(target_type, None, None)?
        };

        // 应用WHERE条件过滤
        let mut filtered: Vec<Vertex> = vertices
            .into_iter()
            .filter(|v| self.match_conditions(v, &query.conditions))
            .collect();

        // 排序
        if let Some(order_field) = &query.order_by {
            filtered.sort_by(|a, b| {
                let va = self.get_field_value(a, order_field);
                let vb = self.get_field_value(b, order_field);
                let cmp = va.cmp(&vb);
                if query.order_desc { cmp.reverse() } else { cmp }
            });
        }

        // 分页
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        let result: Vec<Vertex> = filtered.into_iter().skip(offset).take(limit).collect();

        Ok(result)
    }

    /// 多跳路径查询
    fn query_vertices_path(&self, query: &DslQuery) -> KgResult<Vec<Vertex>> {
        // 分离WHERE条件：
        // - start_conditions: 不带前缀的条件 + 类型等于起始顶点类型的条件
        // - typed_conditions: 类型不等于起始顶点类型的带前缀条件
        let mut start_conditions: Vec<crate::dsl::WhereCondition> = vec![];
        let mut typed_conditions: Vec<&crate::dsl::WhereCondition> = vec![];

        for c in &query.conditions {
            if let Some(pos) = c.field.find('.') {
                let cond_type = &c.field[..pos];
                if cond_type == query.target_type {
                    // 类型等于起始顶点类型，作为起始条件
                    let mut owned = c.clone();
                    owned.field = c.field[pos + 1..].to_string(); // 去掉类型前缀
                    start_conditions.push(owned);
                } else {
                    // 类型不等于起始顶点类型，作为路径中的类型条件
                    typed_conditions.push(c);
                }
            } else {
                // 不带前缀的条件，作为起始条件
                start_conditions.push(c.clone());
            }
        }

        // 获取起始顶点（应用不带前缀的条件）
        let start_vertices = if query.target_type == "*" {
            self.list_all_vertices()?
        } else {
            self.storage.list_vertices_by_type(&query.target_type, None, None)?
        };

        let start_vertices: Vec<Vertex> = start_vertices
            .into_iter()
            .filter(|v| self.match_conditions(v, &start_conditions))
            .collect();

        // 对每个起始顶点，检查是否满足完整路径条件，返回满足条件的起始顶点
        let mut result_vertices = vec![];
        let mut seen = std::collections::HashSet::new();

        for start in &start_vertices {
            if self.has_valid_path(start, &query.path_segments, &typed_conditions) {
                if seen.insert(start.id.clone()) {
                    result_vertices.push(start.clone());
                }
            }
        }

        // 分页
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        let result: Vec<Vertex> = result_vertices.into_iter().skip(offset).take(limit).collect();

        Ok(result)
    }

    /// 检查从起始顶点出发是否存在满足条件的完整路径
    fn has_valid_path(
        &self,
        start: &Vertex,
        segments: &[crate::dsl::PathSegment],
        typed_conditions: &[&crate::dsl::WhereCondition],
    ) -> bool {
        if segments.is_empty() {
            return true;
        }

        let segment = &segments[0];
        let edge_types = if segment.edge_type == "*" {
            None
        } else {
            Some(vec![segment.edge_type.clone()])
        };

        let neighbors = match self.storage.traverse_vertices(
            &start.id,
            segment.direction,
            edge_types.as_deref(),
        ) {
            Ok(v) => v,
            Err(_) => return false,
        };

        for v in neighbors {
            // 过滤目标类型
            if segment.target_type != "*" && v.vertex_type != segment.target_type {
                continue;
            }

            // 检查针对该类型的WHERE条件
            let type_matches = typed_conditions.iter().all(|c| {
                if let Some(pos) = c.field.find('.') {
                    let cond_type = &c.field[..pos];
                    if cond_type == v.vertex_type {
                        let field = &c.field[pos + 1..];
                        return self.match_single_condition_field(&v, field, c);
                    }
                }
                true
            });

            if !type_matches {
                continue;
            }

            // 递归检查剩余路径段
            if self.has_valid_path(&v, &segments[1..], typed_conditions) {
                return true;
            }
        }

        false
    }

    /// 检查顶点单个字段是否满足条件
    fn match_single_condition_field(&self, vertex: &Vertex, field: &str, cond: &crate::dsl::WhereCondition) -> bool {
        let value = if field == "id" {
            vertex.id.clone()
        } else if field == "vertex_type" || field == "type" {
            vertex.vertex_type.clone()
        } else {
            self.get_field_value(vertex, field)
        };
        self.match_single_condition(&value, cond)
    }

    /// 执行MATCH查询（路径匹配）
    fn execute_match(&self, query: &DslQuery) -> KgResult<QueryResult> {
        let mut result = QueryResult::success("match");

        if query.path_segments.is_empty() {
            return Err(KgError::DslParseError("MATCH query requires path".to_string()));
        }

        // 简化实现：找到所有满足路径的顶点序列
        let start_vertices = if query.target_type == "*" {
            self.list_all_vertices()?
        } else {
            self.storage.list_vertices_by_type(&query.target_type, None, None)?
        };

        let start_vertices: Vec<Vertex> = start_vertices
            .into_iter()
            .filter(|v| self.match_conditions(v, &query.conditions))
            .collect();

        let mut paths = vec![];

        for start in &start_vertices {
            if let Some(path) = self.find_path_from(start, &query.path_segments) {
                paths.push(path);
            }
        }

        result.total = paths.len();
        result.paths = paths;
        Ok(result)
    }

    /// 从起始顶点查找路径
    fn find_path_from(&self, start: &Vertex, segments: &[crate::dsl::PathSegment]) -> Option<PathResult> {
        let mut vertices = vec![start.clone()];
        let mut edges = vec![];
        let mut current_id = start.id.clone();

        for segment in segments {
            let edge_types = if segment.edge_type == "*" {
                None
            } else {
                Some(vec![segment.edge_type.clone()])
            };

            let neighbors = self.storage.traverse_edges(
                &current_id,
                segment.direction,
                edge_types.as_deref(),
            ).ok()?;

            // 找到第一个匹配目标类型的边
            let mut found = None;
            for edge in neighbors {
                let target_id = if edge.source == current_id {
                    edge.target.clone()
                } else {
                    edge.source.clone()
                };

                if let Some(v) = self.storage.get_vertex(&target_id).ok()? {
                    if segment.target_type == "*" || v.vertex_type == segment.target_type {
                        found = Some((v, edge));
                        break;
                    }
                }
            }

            match found {
                Some((v, e)) => {
                    current_id = v.id.clone();
                    vertices.push(v);
                    edges.push(e);
                }
                None => return None,
            }
        }

        Some(PathResult {
            vertices,
            edges,
            length: segments.len(),
        })
    }

    /// 执行SEARCH查询（全文搜索）
    fn execute_search(&self, query: &DslQuery) -> KgResult<QueryResult> {
        let mut result = QueryResult::success("search");

        let keyword = query.search_keyword.as_deref().unwrap_or("");
        let field = query.search_field.as_deref().unwrap_or("name");

        let vertices = if query.target_type == "*" {
            self.list_all_vertices()?
        } else {
            self.storage.list_vertices_by_type(&query.target_type, None, None)?
        };

        let mut matched: Vec<Vertex> = vertices
            .into_iter()
            .filter(|v| {
                let val = self.get_field_value(v, field);
                val.to_lowercase().contains(&keyword.to_lowercase())
            })
            .collect();

        // 排序（按匹配度，简化为按名称）
        matched.sort_by(|a, b| {
            let va = self.get_field_value(a, field);
            let vb = self.get_field_value(b, field);
            let sa = va.matches(keyword).count();
            let sb = vb.matches(keyword).count();
            sb.cmp(&sa)
        });

        let limit = query.limit.unwrap_or(50);
        result.total = matched.len();
        result.vertices = matched.into_iter().take(limit).collect();

        Ok(result)
    }

    /// 执行COUNT查询
    fn execute_count(&self, query: &DslQuery) -> KgResult<QueryResult> {
        let mut result = QueryResult::success("count");

        let vertices = if query.target_type == "*" {
            self.list_all_vertices()?
        } else {
            self.storage.list_vertices_by_type(&query.target_type, None, None)?
        };

        let count = vertices
            .iter()
            .filter(|v| self.match_conditions(v, &query.conditions))
            .count();

        result.total = count;
        result.aggregations = Some(serde_json::json!({ "count": count }));

        Ok(result)
    }

    /// 执行DELETE查询
    fn execute_delete(&self, query: &DslQuery) -> KgResult<QueryResult> {
        let mut result = QueryResult::success("delete");

        let vertices = if query.target_type == "*" {
            self.list_all_vertices()?
        } else {
            self.storage.list_vertices_by_type(&query.target_type, None, None)?
        };

        let to_delete: Vec<Vertex> = vertices
            .into_iter()
            .filter(|v| self.match_conditions(v, &query.conditions))
            .collect();

        let count = to_delete.len();
        for v in &to_delete {
            let _ = self.storage.delete_vertex(&v.id);
        }

        result.total = count;
        result.aggregations = Some(serde_json::json!({ "deleted": count }));

        Ok(result)
    }

    // ==================== 辅助方法 ====================

    /// 列出所有顶点
    fn list_all_vertices(&self) -> KgResult<Vec<Vertex>> {
        self.storage.list_all_vertices()
    }

    /// 检查顶点是否满足所有WHERE条件
    fn match_conditions(&self, vertex: &Vertex, conditions: &[crate::dsl::WhereCondition]) -> bool {
        for cond in conditions {
            // 支持 field.subfield 格式（如 product.id）
            let field = if cond.field.contains('.') {
                cond.field.split('.').last().unwrap().to_string()
            } else {
                cond.field.clone()
            };

            // 特殊字段：id, vertex_type
            let value = if field == "id" {
                vertex.id.clone()
            } else if field == "vertex_type" || field == "type" {
                vertex.vertex_type.clone()
            } else {
                self.get_field_value(vertex, &field)
            };

            if !self.match_single_condition(&value, cond) {
                return false;
            }
        }
        true
    }

    /// 匹配单个条件
    fn match_single_condition(&self, value: &str, cond: &crate::dsl::WhereCondition) -> bool {
        match cond.operator.as_str() {
            "=" => value == cond.value,
            "!=" => value != cond.value,
            ">" => value.parse::<f64>().unwrap_or(0.0) > cond.value.parse::<f64>().unwrap_or(0.0),
            "<" => value.parse::<f64>().unwrap_or(0.0) < cond.value.parse::<f64>().unwrap_or(0.0),
            ">=" => value.parse::<f64>().unwrap_or(0.0) >= cond.value.parse::<f64>().unwrap_or(0.0),
            "<=" => value.parse::<f64>().unwrap_or(0.0) <= cond.value.parse::<f64>().unwrap_or(0.0),
            "CONTAINS" => value.to_lowercase().contains(&cond.value.to_lowercase()),
            "IN" => {
                let values: Vec<&str> = cond.value.split(',').map(|s| s.trim()).collect();
                values.contains(&value)
            }
            _ => true,
        }
    }

    /// 获取顶点字段值（字符串形式）
    fn get_field_value(&self, vertex: &Vertex, field: &str) -> String {
        if field == "id" {
            return vertex.id.clone();
        }
        if field == "vertex_type" || field == "type" {
            return vertex.vertex_type.clone();
        }

        match vertex.properties.get(field) {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::Bool(b)) => b.to_string(),
            Some(serde_json::Value::Null) => String::new(),
            _ => String::new(),
        }
    }

    /// 直接遍历（非DSL）
    pub fn traverse(
        &self,
        vertex_id: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
        max_depth: usize,
    ) -> KgResult<Vec<(Vertex, usize)>> {
        self.storage.multi_hop_traverse(vertex_id, direction, edge_types, max_depth)
    }

    /// 查找路径
    pub fn find_path(
        &self,
        source: &str,
        target: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
        max_depth: usize,
    ) -> KgResult<Option<PathResult>> {
        let path = self.storage.find_path(source, target, direction, edge_types, max_depth)?;
        Ok(path.map(|p| PathResult {
            vertices: p.iter().map(|(v, _)| v.clone()).collect(),
            edges: p.iter().map(|(_, e)| e.clone()).collect(),
            length: p.len(),
        }))
    }
}
