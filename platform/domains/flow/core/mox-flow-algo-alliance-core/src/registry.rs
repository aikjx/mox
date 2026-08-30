// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 算法注册表 — 统一注册、发现、版本管理所有算法

use crate::error::{AlgoError, AlgoResult};
use crate::types::{Algorithm, AlgorithmCategory, AlgorithmId, AlgorithmInfo, AlgorithmStatus};
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// 算法注册表
///
/// 统一管理所有算法的注册、发现、版本控制。
/// 支持按类别、状态、名称等多维度查询。
pub struct AlgorithmRegistry {
    /// 已注册的算法
    algorithms: RwLock<IndexMap<AlgorithmId, Arc<dyn Algorithm>>>,
    /// 类别索引
    category_index: RwLock<IndexMap<AlgorithmCategory, Vec<AlgorithmId>>>,
}

impl AlgorithmRegistry {
    /// 创建空的算法注册表
    pub fn new() -> Self {
        Self {
            algorithms: RwLock::new(IndexMap::new()),
            category_index: RwLock::new(IndexMap::new()),
        }
    }

    /// 注册算法
    pub fn register(&self, algorithm: impl Algorithm + 'static) -> AlgoResult<()> {
        let id = algorithm.id().to_string();
        let category = algorithm.category();

        if self.algorithms.read().contains_key(&id) {
            return Err(AlgoError::InvalidParameter {
                param: "id".to_string(),
                reason: format!("algorithm with id '{}' already registered", id),
            });
        }

        self.algorithms.write().insert(id.clone(), Arc::new(algorithm));

        self.category_index
            .write()
            .entry(category)
            .or_default()
            .push(id);

        Ok(())
    }

    /// 注册 Arc 包装的算法
    pub fn register_arc(&self, algorithm: Arc<dyn Algorithm>) -> AlgoResult<()> {
        let id = algorithm.id().to_string();
        let category = algorithm.category();

        if self.algorithms.read().contains_key(&id) {
            return Err(AlgoError::InvalidParameter {
                param: "id".to_string(),
                reason: format!("algorithm with id '{}' already registered", id),
            });
        }

        self.algorithms.write().insert(id.clone(), algorithm);

        self.category_index
            .write()
            .entry(category)
            .or_default()
            .push(id);

        Ok(())
    }

    /// 获取算法
    pub fn get(&self, id: &str) -> AlgoResult<Arc<dyn Algorithm>> {
        self.algorithms
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AlgoError::AlgorithmNotFound(id.to_string()))
    }

    /// 检查算法是否存在
    pub fn contains(&self, id: &str) -> bool {
        self.algorithms.read().contains_key(id)
    }

    /// 注销算法
    pub fn unregister(&self, id: &str) -> AlgoResult<()> {
        let algo = self
            .algorithms
            .write()
            .shift_remove(id)
            .ok_or_else(|| AlgoError::AlgorithmNotFound(id.to_string()))?;

        // 从类别索引中移除
        let category = algo.category();
        if let Some(ids) = self.category_index.write().get_mut(&category) {
            ids.retain(|x| x != id);
        }

        Ok(())
    }

    /// 已注册算法数量
    pub fn count(&self) -> usize {
        self.algorithms.read().len()
    }

    /// 列出所有算法信息
    pub fn list_all(&self) -> Vec<AlgorithmInfo> {
        self.algorithms
            .read()
            .values()
            .map(|a| a.info())
            .collect()
    }

    /// 按类别列出算法
    pub fn list_by_category(&self, category: AlgorithmCategory) -> Vec<AlgorithmInfo> {
        let ids = self
            .category_index
            .read()
            .get(&category)
            .cloned()
            .unwrap_or_default();

        let algos = self.algorithms.read();
        ids.iter()
            .filter_map(|id| algos.get(id).map(|a| a.info()))
            .collect()
    }

    /// 按状态列出算法
    pub fn list_by_status(&self, status: AlgorithmStatus) -> Vec<AlgorithmInfo> {
        self.algorithms
            .read()
            .values()
            .filter(|a| a.status() == status)
            .map(|a| a.info())
            .collect()
    }

    /// 搜索算法（名称或描述包含关键词）
    pub fn search(&self, keyword: &str) -> Vec<AlgorithmInfo> {
        let kw = keyword.to_lowercase();
        self.algorithms
            .read()
            .values()
            .filter(|a| {
                a.name().to_lowercase().contains(&kw)
                    || a.description().to_lowercase().contains(&kw)
                    || a.id().to_lowercase().contains(&kw)
            })
            .map(|a| a.info())
            .collect()
    }

    /// 获取所有类别及其算法数量
    pub fn category_stats(&self) -> IndexMap<AlgorithmCategory, usize> {
        let mut stats = IndexMap::new();
        for category in self.category_index.read().keys() {
            if let Some(ids) = self.category_index.read().get(category) {
                stats.insert(*category, ids.len());
            }
        }
        stats
    }
}

impl Default for AlgorithmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComputeModel, DataShape, ParamSpec, ParamValue};
    use crate::unified_model::UnifiedData;
    use async_trait::async_trait;
    use indexmap::IndexMap;
    use std::sync::Arc;

    struct TestAlgo {
        id: String,
        name: String,
        category: AlgorithmCategory,
    }

    #[async_trait]
    impl Algorithm for TestAlgo {
        fn id(&self) -> &str {
            &self.id
        }
        fn name(&self) -> &str {
            &self.name
        }
        fn category(&self) -> AlgorithmCategory {
            self.category
        }
        fn version(&self) -> &str {
            "1.0.0"
        }
        fn description(&self) -> &str {
            "test algorithm"
        }
        fn input_spec(&self) -> Vec<DataShape> {
            vec![DataShape::graph()]
        }
        fn output_spec(&self) -> Vec<DataShape> {
            vec![DataShape::scalar("f64")]
        }
        fn param_specs(&self) -> Vec<ParamSpec> {
            vec![]
        }
        async fn execute(
            &self,
            _input: UnifiedData,
            _params: IndexMap<String, ParamValue>,
            _compute_engine: Arc<crate::compute_engine::ComputeEngine>,
        ) -> AlgoResult<UnifiedData> {
            Ok(UnifiedData::null())
        }
    }

    #[test]
    fn test_register_and_get() {
        let registry = AlgorithmRegistry::new();
        let algo = TestAlgo {
            id: "test.pagerank".to_string(),
            name: "PageRank".to_string(),
            category: AlgorithmCategory::Graph,
        };

        assert!(registry.register(algo).is_ok());
        assert_eq!(registry.count(), 1);

        let found = registry.get("test.pagerank").unwrap();
        assert_eq!(found.name(), "PageRank");
    }

    #[test]
    fn test_duplicate_register() {
        let registry = AlgorithmRegistry::new();
        let algo1 = TestAlgo {
            id: "test.algo".to_string(),
            name: "Algo1".to_string(),
            category: AlgorithmCategory::Graph,
        };
        let algo2 = TestAlgo {
            id: "test.algo".to_string(),
            name: "Algo2".to_string(),
            category: AlgorithmCategory::Graph,
        };

        assert!(registry.register(algo1).is_ok());
        assert!(registry.register(algo2).is_err());
    }

    #[test]
    fn test_list_by_category() {
        let registry = AlgorithmRegistry::new();

        registry
            .register(TestAlgo {
                id: "g1".to_string(),
                name: "G1".to_string(),
                category: AlgorithmCategory::Graph,
            })
            .unwrap();
        registry
            .register(TestAlgo {
                id: "e1".to_string(),
                name: "E1".to_string(),
                category: AlgorithmCategory::Encoding,
            })
            .unwrap();

        assert_eq!(registry.list_by_category(AlgorithmCategory::Graph).len(), 1);
        assert_eq!(registry.list_by_category(AlgorithmCategory::Encoding).len(), 1);
    }

    #[test]
    fn test_search() {
        let registry = AlgorithmRegistry::new();
        registry
            .register(TestAlgo {
                id: "test.pagerank".to_string(),
                name: "PageRank".to_string(),
                category: AlgorithmCategory::Graph,
            })
            .unwrap();

        let results = registry.search("page");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_unregister() {
        let registry = AlgorithmRegistry::new();
        registry
            .register(TestAlgo {
                id: "test.algo".to_string(),
                name: "Test".to_string(),
                category: AlgorithmCategory::Graph,
            })
            .unwrap();

        assert_eq!(registry.count(), 1);
        assert!(registry.unregister("test.algo").is_ok());
        assert_eq!(registry.count(), 0);
        assert!(registry.get("test.algo").is_err());
    }
}
