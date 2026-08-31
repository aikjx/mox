// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! KG 连接器共享类型
//!
//! 集中定义所有连接器实现共用的数据结构，避免在各实现中重复定义。
//! 类型设计原则：
//!   - 与 kg-hub 服务端的 SearchHit 字段对齐，但本地定义以保持解耦
//!   - 不直接依赖 kg-hub crate（符合微服务架构）
//!   - 提供与 kg-sdk 类型的转换能力（当启用 sdk feature 时）

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ================== 搜索命中类型 ==================

/// 图谱搜索命中（与 kg-hub SearchHit 对齐，但本地定义以解耦）
///
/// 字段说明与 kg-hub 的 `SearchHit` 一一对应，便于 HTTP JSON 反序列化。
/// 注意：此处故意缺少 `evidence` 字段——expert-svc 侧不使用证据字段，
/// 若未来需要可直接添加，serde 会自动忽略多余字段（deny_unknown_fields=false）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSearchHit {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub layer: String,
    pub path: String,
    pub summary: String,
    pub score: f64,
    pub keyword_score: f64,
    pub vector_score: f64,
    pub graph_score: f64,
    pub matched_by: Vec<String>,
}

// ================== 专家匹配增强结果 ==================

/// 专家匹配增强结果
#[derive(Debug, Clone)]
pub struct ExpertGraphBoost {
    /// 专家 ID → 图谱增强分（0..1，越高表示该专家与查询的图谱关联度越高）
    pub boosts: BTreeMap<String, f64>,
    /// 是否使用了图谱（false 表示降级，boosts 全为 0）
    pub graph_used: bool,
}

// ================== 内部类型：API 响应包装 ==================

/// kg-hub 统一响应包装（内部使用，不对外导出）
#[derive(Debug, Deserialize)]
pub(crate) struct ApiResp<T> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GraphSearchHit 能正确序列化/反序列化
    #[test]
    fn graph_search_hit_serde_roundtrip() {
        let hit = GraphSearchHit {
            id: "node-1".into(),
            name: "测试节点".into(),
            kind: "Function".into(),
            layer: "L3".into(),
            path: "test/path".into(),
            summary: "测试摘要".into(),
            score: 0.95,
            keyword_score: 0.8,
            vector_score: 0.7,
            graph_score: 0.6,
            matched_by: vec!["keyword".into(), "vector".into()],
        };

        let json = serde_json::to_string(&hit).unwrap();
        let decoded: GraphSearchHit = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, hit.id);
        assert_eq!(decoded.name, hit.name);
        assert_eq!(decoded.score, hit.score);
        assert_eq!(decoded.matched_by, hit.matched_by);
    }

    /// GraphSearchHit 反序列化时忽略多余字段（如 evidence）
    #[test]
    fn graph_search_hit_tolerates_extra_fields() {
        let json = r#"{
            "id": "n1",
            "name": "test",
            "kind": "Doc",
            "layer": "L2",
            "path": "a/b",
            "summary": "s",
            "evidence": "extra field that should be ignored",
            "score": 0.5,
            "keyword_score": 0.4,
            "vector_score": 0.3,
            "graph_score": 0.2,
            "matched_by": ["keyword"]
        }"#;
        let hit: GraphSearchHit = serde_json::from_str(json).unwrap();
        assert_eq!(hit.id, "n1");
        assert_eq!(hit.score, 0.5);
    }

    /// ExpertGraphBoost 默认值正确
    #[test]
    fn expert_graph_boost_default_semantics() {
        let boost = ExpertGraphBoost {
            boosts: BTreeMap::new(),
            graph_used: false,
        };
        assert!(!boost.graph_used);
        assert!(boost.boosts.is_empty());
    }
}
