// mox-kg-core 自研知识图谱核心引擎
// 基于Rust + RocksDB，与mox-dsql-core深度融合

pub mod dsl;
pub mod engine;
pub mod error;
pub mod model;
pub mod storage;
pub mod manager;

pub use dsl::DslParser;
pub use engine::QueryEngine;
pub use error::{KgError, KgResult};
pub use manager::KgManager;
pub use model::*;
pub use storage::GraphStorage;

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
