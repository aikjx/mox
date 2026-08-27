// mox-kg-core 自研知识图谱核心引擎
// 基于Rust + RocksDB，与mox-dsql-core深度融合

pub mod dsl;
pub mod engine;
pub mod error;
pub mod model;
pub mod storage;

pub use dsl::DslParser;
pub use engine::QueryEngine;
pub use error::{KgError, KgResult};
pub use model::*;
pub use storage::GraphStorage;

use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// 知识图谱管理器（高层API）
pub struct KgManager {
    storage: GraphStorage,
    engine: QueryEngine,
    /// 内存缓存：热点顶点
    vertex_cache: Arc<RwLock<HashMap<String, Vertex>>>,
    /// 内存缓存：热点遍历结果
    traverse_cache: Arc<RwLock<HashMap<String, Vec<Vertex>>>>,
    /// 缓存最大条目数
    cache_max_size: usize,
}

impl KgManager {
    /// 打开或创建知识图谱数据库
    pub fn open<P: AsRef<Path>>(path: P) -> KgResult<Self> {
        let storage = GraphStorage::open(path)?;
        let engine = QueryEngine::new(storage.clone());
        Ok(Self {
            storage,
            engine,
            vertex_cache: Arc::new(RwLock::new(HashMap::new())),
            traverse_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_max_size: 10000,
        })
    }

    /// 打开内存模式（测试用）
    pub fn open_memory() -> KgResult<Self> {
        let storage = GraphStorage::open_memory()?;
        let engine = QueryEngine::new(storage.clone());
        Ok(Self {
            storage,
            engine,
            vertex_cache: Arc::new(RwLock::new(HashMap::new())),
            traverse_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_max_size: 10000,
        })
    }

    // ==================== 顶点操作 ====================

    /// 创建顶点
    pub fn create_vertex(&self, req: &CreateVertexRequest) -> KgResult<Vertex> {
        let vertex = Vertex::new(&req.id, &req.vertex_type, req.properties.clone());
        self.storage.put_vertex(&vertex)?;
        // 写入缓存
        self.cache_vertex(&vertex);
        Ok(vertex)
    }

    /// 创建或更新顶点
    pub fn upsert_vertex(&self, req: &CreateVertexRequest) -> KgResult<Vertex> {
        let vertex = Vertex::new(&req.id, &req.vertex_type, req.properties.clone());
        self.storage.upsert_vertex(&vertex)?;
        self.cache_vertex(&vertex);
        // 失效相关遍历缓存
        self.invalidate_traverse_cache(&vertex.id);
        Ok(vertex)
    }

    /// 获取顶点（带缓存）
    pub fn get_vertex(&self, vertex_id: &str) -> KgResult<Option<Vertex>> {
        // 查缓存
        if let Some(v) = self.vertex_cache.read().get(vertex_id) {
            return Ok(Some(v.clone()));
        }
        // 查存储
        if let Some(v) = self.storage.get_vertex(vertex_id)? {
            self.cache_vertex(&v);
            return Ok(Some(v));
        }
        Ok(None)
    }

    /// 按类型和ID获取顶点
    pub fn get_vertex_typed(&self, vertex_type: &str, vertex_id: &str) -> KgResult<Option<Vertex>> {
        let full_id = format!("{}:{}", vertex_type, vertex_id);
        self.get_vertex(&full_id)
    }

    /// 删除顶点（级联删除边）
    pub fn delete_vertex(&self, vertex_id: &str) -> KgResult<()> {
        self.storage.delete_vertex(vertex_id)?;
        // 清除缓存（删除顶点可能影响所有遍历缓存）
        self.vertex_cache.write().remove(vertex_id);
        self.traverse_cache.write().clear();
        Ok(())
    }

    /// 按类型列出顶点
    pub fn list_vertices(&self, vertex_type: &str, limit: Option<usize>, offset: Option<usize>) -> KgResult<Vec<Vertex>> {
        self.storage.list_vertices_by_type(vertex_type, limit, offset)
    }

    /// 统计顶点数量
    pub fn count_vertices(&self, vertex_type: &str) -> KgResult<usize> {
        self.storage.count_vertices_by_type(vertex_type)
    }

    // ==================== 边操作 ====================

    /// 创建边
    pub fn create_edge(&self, req: &CreateEdgeRequest) -> KgResult<Edge> {
        let properties = req.properties.clone().unwrap_or(serde_json::json!({}));
        let edge = Edge::new(&req.edge_type, &req.source, &req.target, properties);
        self.storage.put_edge(&edge)?;
        // 失效相关遍历缓存
        self.invalidate_traverse_cache(&edge.source);
        self.invalidate_traverse_cache(&edge.target);
        Ok(edge)
    }

    /// 创建或更新边
    pub fn upsert_edge(&self, req: &CreateEdgeRequest) -> KgResult<Edge> {
        let properties = req.properties.clone().unwrap_or(serde_json::json!({}));
        let edge = Edge::new(&req.edge_type, &req.source, &req.target, properties);
        self.storage.upsert_edge(&edge)?;
        self.invalidate_traverse_cache(&edge.source);
        self.invalidate_traverse_cache(&edge.target);
        Ok(edge)
    }

    /// 获取边
    pub fn get_edge(&self, edge_type: &str, source: &str, target: &str) -> KgResult<Option<Edge>> {
        self.storage.get_edge(edge_type, source, target)
    }

    /// 删除边
    pub fn delete_edge(&self, edge_type: &str, source: &str, target: &str) -> KgResult<()> {
        self.storage.delete_edge(edge_type, source, target)?;
        self.invalidate_traverse_cache(source);
        self.invalidate_traverse_cache(target);
        Ok(())
    }

    // ==================== 遍历操作 ====================

    /// 单跳遍历邻接顶点（带缓存）
    pub fn traverse(
        &self,
        vertex_id: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
    ) -> KgResult<Vec<Vertex>> {
        let cache_key = format!(
            "{}:{:?}:{}",
            vertex_id,
            direction,
            edge_types.map(|t| t.join(",")).unwrap_or_default()
        );

        // 查缓存
        if let Some(v) = self.traverse_cache.read().get(&cache_key) {
            return Ok(v.clone());
        }

        let vertices = self.storage.traverse_vertices(vertex_id, direction, edge_types)?;

        // 写入缓存
        if self.traverse_cache.read().len() < self.cache_max_size {
            self.traverse_cache.write().insert(cache_key, vertices.clone());
        }

        Ok(vertices)
    }

    /// 多跳遍历
    pub fn multi_hop_traverse(
        &self,
        start_vertex_id: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
        max_depth: usize,
    ) -> KgResult<Vec<(Vertex, usize)>> {
        self.storage.multi_hop_traverse(start_vertex_id, direction, edge_types, max_depth)
    }

    /// 查找两顶点之间的路径
    pub fn find_path(
        &self,
        source: &str,
        target: &str,
        direction: TraverseDirection,
        edge_types: Option<&[String]>,
        max_depth: usize,
    ) -> KgResult<Option<PathResult>> {
        self.engine.find_path(source, target, direction, edge_types, max_depth)
    }

    // ==================== DSL查询 ====================

    /// 执行DSL查询
    pub fn query_dsl(&self, dsl: &str) -> KgResult<QueryResult> {
        self.engine.execute_dsl(dsl)
    }

    /// 执行DSL查询（带参数）
    pub fn query_dsl_with_params(&self, dsl: &str, params: &HashMap<String, serde_json::Value>) -> KgResult<QueryResult> {
        // 简单参数替换：{{param}} → value
        let mut resolved = dsl.to_string();
        for (key, value) in params {
            let placeholder = format!("{{{{{}}}}}", key);
            let value_str = match value {
                serde_json::Value::String(s) => s.clone(),
                _ => value.to_string(),
            };
            resolved = resolved.replace(&placeholder, &value_str);
        }
        self.engine.execute_dsl(&resolved)
    }

    // ==================== 企业官网实体模型 ====================

    /// 初始化企业官网实体关系模型（创建示例数据）
    pub fn init_enterprise_website_model(&self) -> KgResult<()> {
        // 1. 创建产品分类
        let categories: Vec<(&str, &str, Option<String>)> = vec![
            ("cat_phone", "手机", None),
            ("cat_laptop", "笔记本电脑", None),
            ("cat_accessory", "配件", None),
            ("cat_electronics", "电子产品", None),
        ];

        for (id, name, parent) in &categories {
            self.upsert_vertex(&CreateVertexRequest {
                id: id.to_string(),
                vertex_type: entity_types::PRODUCT_CATEGORY.to_string(),
                properties: serde_json::json!({
                    "name": name,
                    "parent_id": parent,
                    "sort": 0,
                    "status": 1
                }),
            })?;
        }

        // 建立分类层级关系
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::PARENT_OF.to_string(),
            source: "cat_electronics".to_string(),
            target: "cat_phone".to_string(),
            properties: None,
        })?;
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::PARENT_OF.to_string(),
            source: "cat_electronics".to_string(),
            target: "cat_laptop".to_string(),
            properties: None,
        })?;
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::PARENT_OF.to_string(),
            source: "cat_electronics".to_string(),
            target: "cat_accessory".to_string(),
            properties: None,
        })?;

        // 2. 创建产品
        let products = vec![
            ("product_1", "iPhone 15 Pro", "cat_phone", 8999.0, 100, "ACTIVE"),
            ("product_2", "MacBook Pro 14", "cat_laptop", 14999.0, 50, "ACTIVE"),
            ("product_3", "AirPods Pro 2", "cat_accessory", 1899.0, 200, "ACTIVE"),
            ("product_4", "iPad Air", "cat_electronics", 4799.0, 0, "OUT_OF_STOCK"),
        ];

        for (id, name, category, price, stock, status) in &products {
            self.upsert_vertex(&CreateVertexRequest {
                id: id.to_string(),
                vertex_type: entity_types::PRODUCT.to_string(),
                properties: serde_json::json!({
                    "name": name,
                    "category_id": category,
                    "price": price,
                    "stock": stock,
                    "status": status,
                    "views": 0,
                    "is_recommend": true,
                    "is_new": false,
                    "is_hot": *stock > 50
                }),
            })?;

            // 产品→分类关系
            self.upsert_edge(&CreateEdgeRequest {
                edge_type: edge_types::BELONGS_TO.to_string(),
                source: id.to_string(),
                target: category.to_string(),
                properties: None,
            })?;
        }

        // 产品相似关系
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::SIMILAR_TO.to_string(),
            source: "product_1".to_string(),
            target: "product_4".to_string(),
            properties: Some(serde_json::json!({ "similarity": 0.85 })),
        })?;

        // 3. 创建客户案例
        let cases = vec![
            ("case_1", "某银行数字化转型", "招商银行", "金融", "ACTIVE"),
            ("case_2", "某电商平台架构升级", "阿里巴巴", "电商", "ACTIVE"),
            ("case_3", "某制造企业MES系统", "比亚迪", "制造", "ACTIVE"),
        ];

        for (id, title, customer, industry, status) in &cases {
            self.upsert_vertex(&CreateVertexRequest {
                id: id.to_string(),
                vertex_type: entity_types::CASE.to_string(),
                properties: serde_json::json!({
                    "title": title,
                    "customer_name": customer,
                    "industry": industry,
                    "status": status,
                    "views": 0
                }),
            })?;
        }

        // 案例→产品关系（使用了哪些产品）
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::USES.to_string(),
            source: "case_1".to_string(),
            target: "product_2".to_string(),
            properties: Some(serde_json::json!({ "usage_count": 50 })),
        })?;
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::USES.to_string(),
            source: "case_2".to_string(),
            target: "product_1".to_string(),
            properties: Some(serde_json::json!({ "usage_count": 100 })),
        })?;
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::USES.to_string(),
            source: "case_3".to_string(),
            target: "product_3".to_string(),
            properties: Some(serde_json::json!({ "usage_count": 200 })),
        })?;

        // 4. 创建新闻
        let news = vec![
            ("news_1", "公司发布新品iPhone 15 Pro", "产品发布", "ACTIVE"),
            ("news_2", "MacBook Pro荣获设计大奖", "公司动态", "ACTIVE"),
            ("news_3", "行业趋势：AI终端的未来", "行业资讯", "ACTIVE"),
        ];

        for (id, title, category, status) in &news {
            self.upsert_vertex(&CreateVertexRequest {
                id: id.to_string(),
                vertex_type: entity_types::NEWS.to_string(),
                properties: serde_json::json!({
                    "title": title,
                    "category": category,
                    "status": status,
                    "views": 0,
                    "publish_time": "2026-08-27"
                }),
            })?;
        }

        // 新闻→产品关系（相关产品）
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::RELATED_TO.to_string(),
            source: "news_1".to_string(),
            target: "product_1".to_string(),
            properties: None,
        })?;
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::RELATED_TO.to_string(),
            source: "news_2".to_string(),
            target: "product_2".to_string(),
            properties: None,
        })?;

        // 5. 创建团队成员
        let team = vec![
            ("team_1", "张三", "CTO", "技术部"),
            ("team_2", "李四", "产品总监", "产品部"),
            ("team_3", "王五", "架构师", "技术部"),
        ];

        for (id, name, position, department) in &team {
            self.upsert_vertex(&CreateVertexRequest {
                id: id.to_string(),
                vertex_type: entity_types::TEAM.to_string(),
                properties: serde_json::json!({
                    "name": name,
                    "position": position,
                    "department": department,
                    "status": 1
                }),
            })?;
        }

        // 团队→产品关系（负责）
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::RESPONSIBLE_FOR.to_string(),
            source: "team_1".to_string(),
            target: "product_2".to_string(),
            properties: None,
        })?;
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::RESPONSIBLE_FOR.to_string(),
            source: "team_2".to_string(),
            target: "product_1".to_string(),
            properties: None,
        })?;

        // 团队协作关系
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::WORKS_WITH.to_string(),
            source: "team_1".to_string(),
            target: "team_3".to_string(),
            properties: None,
        })?;

        // 6. 创建FAQ
        let faqs = vec![
            ("faq_1", "iPhone 15 Pro支持5G吗？", "支持，支持全网通5G"),
            ("faq_2", "MacBook Pro续航多久？", "最长可达18小时"),
        ];

        for (id, question, answer) in &faqs {
            self.upsert_vertex(&CreateVertexRequest {
                id: id.to_string(),
                vertex_type: entity_types::FAQ.to_string(),
                properties: serde_json::json!({
                    "question": question,
                    "answer": answer,
                    "status": 1
                }),
            })?;
        }

        // FAQ→产品关系（引用）
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::REFERENCES.to_string(),
            source: "faq_1".to_string(),
            target: "product_1".to_string(),
            properties: None,
        })?;
        self.upsert_edge(&CreateEdgeRequest {
            edge_type: edge_types::REFERENCES.to_string(),
            source: "faq_2".to_string(),
            target: "product_2".to_string(),
            properties: None,
        })?;

        Ok(())
    }

    // ==================== 统计与监控 ====================

    /// 获取图谱统计信息
    pub fn stats(&self) -> KgResult<serde_json::Value> {
        self.storage.stats()
    }

    /// 性能基准测试
    pub fn benchmark(&self, iterations: usize) -> KgResult<serde_json::Value> {
        // 1. 点查询性能
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = self.get_vertex("product_1")?;
        }
        let point_query_ms = start.elapsed().as_micros() as f64 / iterations as f64;

        // 2. 1跳遍历性能
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = self.traverse("product_1", TraverseDirection::Out, None)?;
        }
        let one_hop_ms = start.elapsed().as_micros() as f64 / iterations as f64;

        // 3. 2跳遍历性能
        let start = Instant::now();
        for _ in 0..(iterations / 10).max(1) {
            let _ = self.multi_hop_traverse("case_1", TraverseDirection::Out, None, 2)?;
        }
        let two_hop_ms = start.elapsed().as_micros() as f64 / (iterations as f64 / 10.0);

        // 4. DSL查询性能
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = self.query_dsl("GET product WHERE status = 'ACTIVE'")?;
        }
        let dsl_ms = start.elapsed().as_micros() as f64 / iterations as f64;

        Ok(serde_json::json!({
            "iterations": iterations,
            "point_query_avg_us": point_query_ms,
            "one_hop_traverse_avg_us": one_hop_ms,
            "two_hop_traverse_avg_us": two_hop_ms,
            "dsl_query_avg_us": dsl_ms,
            "qps_estimate": {
                "point_query": (1_000_000.0 / point_query_ms) as u64,
                "one_hop": (1_000_000.0 / one_hop_ms) as u64,
                "dsl": (1_000_000.0 / dsl_ms) as u64,
            }
        }))
    }

    // ==================== 缓存管理 ====================

    /// 清除所有缓存
    pub fn clear_cache(&self) {
        self.vertex_cache.write().clear();
        self.traverse_cache.write().clear();
    }

    /// 获取缓存状态
    pub fn cache_status(&self) -> serde_json::Value {
        serde_json::json!({
            "vertex_cache_size": self.vertex_cache.read().len(),
            "traverse_cache_size": self.traverse_cache.read().len(),
            "max_size": self.cache_max_size,
        })
    }

    // ==================== 内部方法 ====================

    fn cache_vertex(&self, vertex: &Vertex) {
        let mut cache = self.vertex_cache.write();
        if cache.len() < self.cache_max_size {
            cache.insert(vertex.id.clone(), vertex.clone());
        }
    }

    fn invalidate_traverse_cache(&self, vertex_id: &str) {
        let mut cache = self.traverse_cache.write();
        cache.retain(|k, _| !k.starts_with(&format!("{}:", vertex_id)));
    }
}

impl Clone for KgManager {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
            engine: QueryEngine::new(self.storage.clone()),
            vertex_cache: Arc::clone(&self.vertex_cache),
            traverse_cache: Arc::clone(&self.traverse_cache),
            cache_max_size: self.cache_max_size,
        }
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> KgManager {
        let kg = KgManager::open_memory().unwrap();
        kg.init_enterprise_website_model().unwrap();
        kg
    }

    #[test]
    fn test_create_and_get_vertex() {
        let kg = setup();
        let v = kg.get_vertex("product_1").unwrap().unwrap();
        assert_eq!(v.vertex_type, "product");
        assert_eq!(v.properties["name"], "iPhone 15 Pro");
    }

    #[test]
    fn test_traverse_one_hop() {
        let kg = setup();
        // 产品→分类（出边）
        let neighbors = kg.traverse("product_1", TraverseDirection::Out, None).unwrap();
        assert!(neighbors.iter().any(|v| v.id == "cat_phone"));
    }

    #[test]
    fn test_traverse_inbound() {
        let kg = setup();
        // 分类→产品（入边）
        let neighbors = kg.traverse("cat_phone", TraverseDirection::In, None).unwrap();
        assert!(neighbors.iter().any(|v| v.id == "product_1"));
    }

    #[test]
    fn test_dsl_simple_get() {
        let kg = setup();
        let result = kg.query_dsl("GET product WHERE status = 'ACTIVE'").unwrap();
        assert!(result.success);
        assert!(result.total >= 3);
    }

    #[test]
    fn test_dsl_one_hop() {
        let kg = setup();
        // 查找使用了product_1的案例
        let result = kg.query_dsl("GET case -[uses]-> product WHERE product.id = 'product_1'").unwrap();
        assert!(result.success);
        assert!(result.vertices.iter().any(|v| v.id == "case_2"));
    }

    #[test]
    fn test_dsl_two_hop() {
        let kg = setup();
        // 查找金融行业案例使用的产品（GET后面是返回类型product，用入边语法）
        let result = kg.query_dsl("GET product <-[uses]- case WHERE case.industry = '金融'").unwrap();
        assert!(result.success);
        assert!(result.vertices.iter().any(|v| v.id == "product_2"));
    }

    #[test]
    fn test_dsl_search() {
        let kg = setup();
        let result = kg.query_dsl("SEARCH product WHERE name CONTAINS 'iPhone'").unwrap();
        assert!(result.success);
        assert!(result.vertices.iter().any(|v| v.id == "product_1"));
    }

    #[test]
    fn test_dsl_count() {
        let kg = setup();
        let result = kg.query_dsl("COUNT product WHERE status = 'ACTIVE'").unwrap();
        assert!(result.success);
        assert_eq!(result.total, 3);
    }

    #[test]
    fn test_find_path() {
        let kg = setup();
        // 案例→产品→分类 路径
        let path = kg.find_path(
            "case_1",
            "cat_laptop",
            TraverseDirection::Out,
            None,
            3,
        ).unwrap();
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.length, 2);
    }

    #[test]
    fn test_multi_hop_traverse() {
        let kg = setup();
        // 从案例出发，2跳遍历（案例→产品→分类）
        let results = kg.multi_hop_traverse("case_1", TraverseDirection::Out, None, 2).unwrap();
        assert!(results.iter().any(|(v, d)| v.id == "product_2" && *d == 1));
        assert!(results.iter().any(|(v, d)| v.id == "cat_laptop" && *d == 2));
    }

    #[test]
    fn test_stats() {
        let kg = setup();
        let stats = kg.stats().unwrap();
        assert!(stats["vertex_count"].as_u64().unwrap() >= 15);
        assert!(stats["edge_count"].as_u64().unwrap() >= 15);
    }

    #[test]
    fn test_delete_vertex_cascade() {
        let kg = setup();
        // 删除产品前，分类有入边
        let neighbors_before = kg.traverse("cat_phone", TraverseDirection::In, None).unwrap();
        assert!(neighbors_before.iter().any(|v| v.id == "product_1"));

        // 删除产品
        kg.delete_vertex("product_1").unwrap();

        // 删除后，分类的入边应该减少
        let neighbors_after = kg.traverse("cat_phone", TraverseDirection::In, None).unwrap();
        assert!(!neighbors_after.iter().any(|v| v.id == "product_1"));
    }

    #[test]
    fn test_benchmark() {
        let kg = setup();
        let result = kg.benchmark(100).unwrap();
        assert!(result["point_query_avg_us"].as_f64().unwrap() > 0.0);
        assert!(result["dsl_query_avg_us"].as_f64().unwrap() > 0.0);
    }
}
