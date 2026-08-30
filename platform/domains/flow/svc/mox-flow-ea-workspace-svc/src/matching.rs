// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! 专家匹配引擎
//!
//! 使用 mox-unified-algo-core 的相似度与排序算法，
//! 实现跨域专家匹配（技能向量 + 知识图谱关联 + 历史评价）。

use crate::error::WorkspaceResult;
use crate::types::*;
use mox_unified_algo_core::algorithms::similarity::CosineSimilarity;
use mox_unified_algo_core::traits::SimilarityAlgo;

/// 专家匹配引擎
pub struct ExpertMatchingEngine {
    cosine: CosineSimilarity,
}

impl ExpertMatchingEngine {
    /// 创建匹配引擎
    pub fn new() -> Self {
        Self {
            cosine: CosineSimilarity::default(),
        }
    }

    /// 匹配专家
    ///
    /// 综合评分 = 技能相似度 × 0.4 + 经验匹配 × 0.3 + 可用性 × 0.2 + 历史评分 × 0.1
    pub fn match_experts(
        &self,
        required_skills: &[String],
        experts: &[ExpertProfile],
        limit: usize,
    ) -> WorkspaceResult<Vec<ExpertMatchResult>> {
        let mut results: Vec<ExpertMatchResult> = experts
            .iter()
            .map(|expert| {
                let skill_overlap = self.compute_skill_overlap(required_skills, &expert.skills);
                let skill_score = self.compute_skill_similarity(required_skills, &expert.skills);
                let exp_score = (expert.completed_tasks as f64).min(100.0) / 100.0;
                let avail_score = self.availability_score(&expert.availability);
                let rating_score = expert.rating / 5.0;

                let weights = [0.4, 0.3, 0.2, 0.1];
                let scores = [skill_score, exp_score, avail_score, rating_score];

                let match_score: f64 = scores
                    .iter()
                    .zip(weights.iter())
                    .map(|(s, w)| s * w)
                    .sum();

                let match_reasons = self.generate_match_reasons(
                    required_skills,
                    &skill_overlap,
                    skill_score,
                    expert,
                );

                ExpertMatchResult {
                    expert: expert.clone(),
                    match_score,
                    skill_overlap,
                    match_reasons,
                }
            })
            .collect();

        // 按匹配度降序排列
        results.sort_by(|a, b| {
            b.match_score
                .partial_cmp(&a.match_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);

        Ok(results)
    }

    /// 计算技能重叠
    fn compute_skill_overlap(&self, required: &[String], expert_skills: &[String]) -> Vec<String> {
        required
            .iter()
            .filter(|s| expert_skills.contains(s))
            .cloned()
            .collect()
    }

    /// 计算技能相似度（Jaccard 相似度）
    fn compute_skill_similarity(&self, required: &[String], expert_skills: &[String]) -> f64 {
        if required.is_empty() && expert_skills.is_empty() {
            return 1.0;
        }
        let intersection = self.compute_skill_overlap(required, expert_skills).len() as f64;
        let union_set: std::collections::HashSet<&str> = required
            .iter()
            .chain(expert_skills.iter())
            .map(|s| s.as_str())
            .collect();
        let union = union_set.len() as f64;
        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }

    /// 可用性评分
    fn availability_score(&self, avail: &ExpertAvailability) -> f64 {
        match avail {
            ExpertAvailability::Available => 1.0,
            ExpertAvailability::Busy => 0.5,
            ExpertAvailability::Offline => 0.2,
            ExpertAvailability::OnLeave => 0.1,
        }
    }

    /// 生成匹配理由
    fn generate_match_reasons(
        &self,
        required: &[String],
        overlap: &[String],
        skill_score: f64,
        expert: &ExpertProfile,
    ) -> Vec<String> {
        let mut reasons = Vec::new();

        if !overlap.is_empty() {
            reasons.push(format!(
                "技能匹配：{}（{}/{}）",
                overlap.join("、"),
                overlap.len(),
                required.len()
            ));
        }

        if skill_score > 0.7 {
            reasons.push("技能高度匹配".to_string());
        }

        if expert.rating >= 4.5 {
            reasons.push(format!("专家评分 {:.1} 分（优秀）", expert.rating));
        }

        if expert.completed_tasks > 50 {
            reasons.push(format!("已完成 {} 个任务（经验丰富）", expert.completed_tasks));
        }

        reasons
    }

    /// 基于向量的专家匹配（使用嵌入向量 + 余弦相似度）
    pub fn match_by_embedding(
        &self,
        query_vector: &[f64],
        experts: &[ExpertProfile],
        limit: usize,
    ) -> WorkspaceResult<Vec<ExpertMatchResult>> {
        let mut results: Vec<ExpertMatchResult> = experts
            .iter()
            .filter(|expert| !expert.skill_vector.is_empty())
            .filter_map(|expert| {
                let score = self.cosine.similarity(
                    &query_vector.to_vec(),
                    &expert.skill_vector,
                );
                Some(ExpertMatchResult {
                    expert: expert.clone(),
                    match_score: score,
                    skill_overlap: vec![],
                    match_reasons: vec![format!("向量相似度 {:.1}%", score * 100.0)],
                })
            })
            .collect();

        results.sort_by(|a, b| {
            b.match_score
                .partial_cmp(&a.match_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(limit);

        Ok(results)
    }
}

impl Default for ExpertMatchingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_experts() -> Vec<ExpertProfile> {
        use chrono::Utc;
        vec![
            ExpertProfile {
                id: "e1".into(),
                name: "张博士".into(),
                avatar: None,
                title: "首席算法工程师".into(),
                organization: "璇玑研究院".into(),
                skills: vec!["知识图谱".into(), "图算法".into(), "RAG".into(), "NLP".into()],
                domains: vec!["AI".into(), "KG".into()],
                rating: 4.8,
                completed_tasks: 128,
                availability: ExpertAvailability::Available,
                hourly_rate: Some(800.0),
                description: "资深图算法专家".into(),
                skill_vector: vec![0.9, 0.8, 0.7, 0.6, 0.5],
            },
            ExpertProfile {
                id: "e2".into(),
                name: "李工".into(),
                avatar: None,
                title: "高级后端工程师".into(),
                organization: "璇玑平台部".into(),
                skills: vec!["Rust".into(), "分布式系统".into(), "数据库".into()],
                domains: vec!["Platform".into()],
                rating: 4.5,
                completed_tasks: 86,
                availability: ExpertAvailability::Busy,
                hourly_rate: Some(600.0),
                description: "分布式系统专家".into(),
                skill_vector: vec![0.3, 0.4, 0.8, 0.9, 0.7],
            },
            ExpertProfile {
                id: "e3".into(),
                name: "王教授".into(),
                avatar: None,
                title: "AI 研究科学家".into(),
                organization: "某高校".into(),
                skills: vec!["深度学习".into(), "知识图谱".into(), "图神经网络".into(), "RAG".into()],
                domains: vec!["AI".into(), "Research".into()],
                rating: 4.9,
                completed_tasks: 42,
                availability: ExpertAvailability::Available,
                hourly_rate: Some(1000.0),
                description: "GNN 领域权威".into(),
                skill_vector: vec![0.85, 0.9, 0.6, 0.4, 0.3],
            },
        ]
    }

    #[test]
    fn test_match_experts_by_skills() {
        let engine = ExpertMatchingEngine::new();
        let experts = mock_experts();
        let required = vec!["知识图谱".to_string(), "RAG".to_string(), "图算法".to_string()];

        let results = engine.match_experts(&required, &experts, 5).unwrap();

        assert!(!results.is_empty());
        assert!(results[0].match_score >= results.last().unwrap().match_score);

        // 张博士应该匹配度最高（匹配3个技能）
        assert_eq!(results[0].expert.name, "张博士");
        assert!(results[0].match_score > 0.5);
    }

    #[test]
    fn test_match_limit() {
        let engine = ExpertMatchingEngine::new();
        let experts = mock_experts();
        let required = vec!["Rust".to_string()];

        let results = engine.match_experts(&required, &experts, 2).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_skill_overlap() {
        let engine = ExpertMatchingEngine::new();
        let required = vec!["知识图谱".into(), "RAG".into()];
        let skills = vec!["知识图谱".into(), "NLP".into(), "RAG".into()];
        let overlap = engine.compute_skill_overlap(&required, &skills);
        assert_eq!(overlap.len(), 2);
    }

    #[test]
    fn test_skill_similarity() {
        let engine = ExpertMatchingEngine::new();
        let required = vec!["A".into(), "B".into(), "C".into()];
        let skills = vec!["B".into(), "C".into(), "D".into()];
        let sim = engine.compute_skill_similarity(&required, &skills);
        // 交集: B, C = 2, 并集: A, B, C, D = 4, Jaccard = 0.5
        assert!((sim - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_match_reasons_generated() {
        let engine = ExpertMatchingEngine::new();
        let experts = mock_experts();
        let required = vec!["知识图谱".to_string(), "RAG".to_string()];

        let results = engine.match_experts(&required, &experts, 3).unwrap();
        assert!(!results[0].match_reasons.is_empty());
    }

    #[test]
    fn test_match_by_embedding() {
        let engine = ExpertMatchingEngine::new();
        let experts = mock_experts();
        let query = vec![0.9, 0.8, 0.7, 0.6, 0.5];

        let results = engine.match_by_embedding(&query, &experts, 5).unwrap();
        assert!(!results.is_empty());
        // 张博士的向量最接近查询向量
        assert_eq!(results[0].expert.name, "张博士");
        assert!(results[0].match_score > 0.9);
    }
}
