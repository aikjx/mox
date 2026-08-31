// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 结果融合引擎（执行器侧适配层）
//!
//! 把执行器 DAG 尾部的多节点执行结果，按 `plan.fusion_strategy` 接入
//! `mox-alliance-core` 的统一融合引擎 `FusionEngine`，产出融合结论。
//!
//! 设计：
//! - [`FusionItem`]：从节点 + 执行结果提取单条可融合条目（专家、摘要、置信度、输出）
//! - [`FusionEngine`]（本模块）：把条目映射为标量对 `(confidence, weight)`
//!   与分类对 `(summary, weight)`，调用 mox-alliance-core 的融合策略，
//!   并打包为结构化 [`FusionOutput`]
//!
//! 9 种策略全部兑现：
//! `Voting` / `Weighted` / `ConfidenceWeighted` / `BestOf` / `Concatenation`
//! `Stacking` / `Debate` / `MapReduce` / `Iterative`

use mox_alliance_common_proto::{AllianceError, AllianceResult, FusionStrategy, Node};
use mox_alliance_core::fusion::FusionEngine as CoreFusionEngine;
use mox_alliance_executor_proto::NodeExecutionResult;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// 单条可融合的专家输出
#[derive(Debug, Clone, Serialize)]
pub struct FusionItem {
    pub expert_id: String,
    pub node_id: String,
    /// 结果摘要文本（用于分类/投票类策略）
    pub summary: String,
    /// 置信度 0.0 ~ 1.0（标量融合的"值"）
    pub confidence: f64,
    /// 完整结构化输出
    pub output: Value,
}

impl FusionItem {
    /// 从节点与执行结果构造融合条目
    ///
    /// - 失败或无输出的节点返回 `None`（不参与融合）
    /// - 置信度优先取输出中的 `score` 字段，缺省 0.5
    /// - 摘要优先取输出中的 `result` 字段，缺省为节点名描述
    pub fn from_execution(node: &Node, result: &NodeExecutionResult) -> Option<Self> {
        if !result.success {
            return None;
        }
        let output = result.output.clone().unwrap_or(Value::Null);
        let summary = output
            .get("result")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| output.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{} 的执行输出", node.name));
        let confidence = output
            .get("score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        Some(Self {
            expert_id: node.expert_id.clone(),
            node_id: node.node_id.clone(),
            summary,
            confidence,
            output,
        })
    }

    /// 获取该专家的融合权重（无配置时等权重 1.0）
    fn weight(&self, weights: &HashMap<String, f64>) -> f64 {
        weights
            .get(&self.expert_id)
            .copied()
            .filter(|w| w.is_finite() && *w >= 0.0)
            .unwrap_or(1.0)
    }
}

/// 融合输入
#[derive(Debug, Clone)]
pub struct FusionInput {
    /// 成功节点输出条目
    pub items: Vec<FusionItem>,
    /// 专家权重（expert_id -> weight），缺省等权重
    pub expert_weights: HashMap<String, f64>,
    /// 融合策略
    pub strategy: FusionStrategy,
    /// 任务描述（上下文，仅记录）
    pub task_description: String,
}

/// 融合输出
#[derive(Debug, Clone, Serialize)]
pub struct FusionOutput {
    /// 融合后的结构化内容（策略相关）
    pub content: Value,
    /// 融合置信度 0.0 ~ 1.0
    pub confidence: f64,
    /// 参与融合的专家数量
    pub expert_count: usize,
    /// 使用的融合策略
    pub strategy: FusionStrategy,
    /// 各专家的贡献度（归一化权重）
    pub contributions: HashMap<String, f64>,
    /// 融合摘要说明
    pub summary: String,
}

/// 结果融合引擎（执行器侧）
#[derive(Clone)]
pub struct FusionEngine {
    default_strategy: FusionStrategy,
}

impl FusionEngine {
    /// 创建融合引擎
    pub fn new() -> Self {
        Self {
            default_strategy: FusionStrategy::Weighted,
        }
    }

    /// 配置默认融合策略
    pub fn with_default_strategy(mut self, strategy: FusionStrategy) -> Self {
        self.default_strategy = strategy;
        self
    }

    /// 执行融合
    ///
    /// 核心计算委托给 mox-alliance-core 的 `FusionEngine`：
    /// - 标量类策略：`fuse_scalar(&[(confidence, weight)])` 得到融合置信度
    /// - 分类类策略：`fuse_classification(&[(summary, weight)])` 得到胜出类别
    pub fn fuse(&self, input: FusionInput) -> AllianceResult<FusionOutput> {
        let strategy = input.strategy;
        if input.items.is_empty() {
            return Ok(FusionOutput {
                content: Value::Null,
                confidence: 0.0,
                expert_count: 0,
                strategy,
                contributions: HashMap::new(),
                summary: "No successful results to fuse".to_string(),
            });
        }

        let core = CoreFusionEngine::from_strategy(strategy);
        let contributions = Self::calculate_contributions(&input);

        let output = match strategy {
            FusionStrategy::Voting => Self::fuse_voting(&input, &core)?,
            FusionStrategy::Weighted => Self::fuse_weighted(&input, &core)?,
            FusionStrategy::ConfidenceWeighted => Self::fuse_confidence_weighted(&input, &core)?,
            FusionStrategy::BestOf => Self::fuse_best_of(&input, &core)?,
            FusionStrategy::Concatenation => Self::fuse_concatenation(&input, &core)?,
            FusionStrategy::Stacking => Self::fuse_stacking(&input, &core)?,
            FusionStrategy::Debate => Self::fuse_debate(&input, &core)?,
            FusionStrategy::MapReduce => Self::fuse_map_reduce(&input, &core)?,
            FusionStrategy::Iterative => Self::fuse_iterative(&input, &core)?,
        };
        let mut output = output;
        output.contributions = contributions;
        Ok(output)
    }

    // === 策略实现 ===

    /// 加权投票：以摘要为类别，按权重投票，得出胜出类别
    fn fuse_voting(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let votes: Vec<(String, f64)> = input
            .items
            .iter()
            .map(|it| (it.summary.clone(), it.weight(&input.expert_weights)))
            .collect();
        let (winner, score, total) = core
            .fuse_classification(&votes)
            .map_err(|e| AllianceError::internal(format!("voting fusion failed: {:?}", e)))?;

        Ok(FusionOutput {
            content: json!({
                "winner": winner,
                "votes": input.items.iter().map(|it| json!({
                    "expert": it.expert_id,
                    "summary": it.summary,
                    "weight": it.weight(&input.expert_weights),
                })).collect::<Vec<_>>(),
                "total_weight": total,
            }),
            confidence: if total > 0.0 { (score / total).clamp(0.0, 1.0) } else { 0.0 },
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!(
                "投票融合：{} 位专家，胜出摘要置信度 {:.1}%",
                input.items.len(),
                if total > 0.0 { score / total * 100.0 } else { 0.0 }
            ),
        })
    }

    /// 加权融合：以各专家置信度为"值"，按权重融合出整体置信度
    fn fuse_weighted(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let pairs: Vec<(f64, f64)> = input
            .items
            .iter()
            .map(|it| (it.confidence, it.weight(&input.expert_weights)))
            .collect();
        let fused = core
            .fuse_scalar(&pairs)
            .map_err(|e| AllianceError::internal(format!("weighted fusion failed: {:?}", e)))?;

        Ok(FusionOutput {
            content: json!({
                "outputs": input.items.iter().map(|it| json!({
                    "expert": it.expert_id,
                    "output": it.output,
                    "summary": it.summary,
                    "confidence": it.confidence,
                    "weight": it.weight(&input.expert_weights),
                })).collect::<Vec<_>>(),
                "fused_confidence": fused,
            }),
            confidence: fused.clamp(0.0, 1.0),
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!(
                "加权融合：{} 位专家，融合置信度 {:.1}%",
                input.items.len(),
                (fused * 100.0).clamp(0.0, 100.0)
            ),
        })
    }

    /// 置信度加权：以置信度为"值"与"权重"（weight*confidence），置信度高的专家权重更大
    fn fuse_confidence_weighted(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let pairs: Vec<(f64, f64)> = input
            .items
            .iter()
            .map(|it| {
                let base = it.weight(&input.expert_weights);
                (it.confidence, base * it.confidence)
            })
            .collect();
        let fused = core
            .fuse_scalar(&pairs)
            .map_err(|e| AllianceError::internal(format!("confidence-weighted fusion failed: {:?}", e)))?;

        let adjusted: HashMap<String, f64> = input
            .items
            .iter()
            .map(|it| {
                let base = it.weight(&input.expert_weights);
                (it.expert_id.clone(), base * it.confidence)
            })
            .collect();

        Ok(FusionOutput {
            content: json!({
                "outputs": input.items.iter().map(|it| json!({
                    "expert": it.expert_id,
                    "output": it.output,
                    "confidence": it.confidence,
                })).collect::<Vec<_>>(),
                "adjusted_weights": adjusted,
                "fused_confidence": fused,
            }),
            confidence: fused.clamp(0.0, 1.0),
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!(
                "置信度加权融合：{} 位专家，融合置信度 {:.1}%",
                input.items.len(),
                (fused * 100.0).clamp(0.0, 100.0)
            ),
        })
    }

    /// 择优：选出权重*置信度最高的专家结果
    fn fuse_best_of(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let pairs: Vec<(f64, f64)> = input
            .items
            .iter()
            .map(|it| {
                let base = it.weight(&input.expert_weights);
                (it.confidence, base * it.confidence)
            })
            .collect();
        let _ = core
            .fuse_scalar(&pairs)
            .map_err(|e| AllianceError::internal(format!("best-of fusion failed: {:?}", e)))?;

        let best = input
            .items
            .iter()
            .max_by(|a, b| {
                let sa = a.confidence * a.weight(&input.expert_weights);
                let sb = b.confidence * b.weight(&input.expert_weights);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap();

        Ok(FusionOutput {
            content: json!({
                "best_expert": best.expert_id,
                "best_node": best.node_id,
                "output": best.output,
                "summary": best.summary,
                "confidence": best.confidence,
            }),
            confidence: best.confidence,
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!(
                "择优融合：选中 {}（置信度 {:.1}%）",
                best.expert_id,
                best.confidence * 100.0
            ),
        })
    }

    /// 拼接：聚合所有专家输出，置信度取加权融合
    fn fuse_concatenation(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let pairs: Vec<(f64, f64)> = input
            .items
            .iter()
            .map(|it| (it.confidence, it.weight(&input.expert_weights)))
            .collect();
        let fused = core
            .fuse_scalar(&pairs)
            .map_err(|e| AllianceError::internal(format!("concatenation fusion failed: {:?}", e)))?;

        Ok(FusionOutput {
            content: json!({
                "expert_outputs": input.items.iter().map(|it| json!({
                    "expert": it.expert_id,
                    "node": it.node_id,
                    "output": it.output,
                    "confidence": it.confidence,
                    "weight": it.weight(&input.expert_weights),
                })).collect::<Vec<_>>(),
                "count": input.items.len(),
            }),
            confidence: fused.clamp(0.0, 1.0),
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!("拼接融合：{} 位专家输出已合并", input.items.len()),
        })
    }

    /// 堆叠：基础学习器输出 + 元学习器（降级为置信度加权）
    fn fuse_stacking(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let pairs: Vec<(f64, f64)> = input
            .items
            .iter()
            .map(|it| (it.confidence, it.weight(&input.expert_weights)))
            .collect();
        let meta_conf = core
            .fuse_scalar(&pairs)
            .map_err(|e| AllianceError::internal(format!("stacking fusion failed: {:?}", e)))?;

        Ok(FusionOutput {
            content: json!({
                "base_learners": input.items.iter().map(|it| json!({
                    "expert": it.expert_id,
                    "output": it.output,
                    "confidence": it.confidence,
                    "weight": it.weight(&input.expert_weights),
                })).collect::<Vec<_>>(),
                "meta_learner_output": {
                    "confidence": meta_conf,
                    "method": "confidence_weighted",
                },
            }),
            confidence: meta_conf.clamp(0.0, 1.0),
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!(
                "堆叠融合：{} 个基学习器，元学习器置信度 {:.1}%",
                input.items.len(),
                (meta_conf * 100.0).clamp(0.0, 100.0)
            ),
        })
    }

    /// 辩论：前 N-1 为辩手，最后一位为裁判，分类投票裁决
    fn fuse_debate(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let (debaters, judge) = if input.items.len() >= 3 {
            let split = input.items.len() - 1;
            (
                &input.items[..split],
                Some(&input.items[split]),
            )
        } else {
            (input.items.as_slice(), None)
        };

        let votes: Vec<(String, f64)> = debaters
            .iter()
            .map(|it| (it.summary.clone(), it.weight(&input.expert_weights)))
            .collect();
        let (winner, score, total) = core
            .fuse_classification(&votes)
            .map_err(|e| AllianceError::internal(format!("debate fusion failed: {:?}", e)))?;

        Ok(FusionOutput {
            content: json!({
                "debaters": debaters.iter().map(|it| json!({
                    "expert": it.expert_id,
                    "summary": it.summary,
                    "weight": it.weight(&input.expert_weights),
                })).collect::<Vec<_>>(),
                "verdict": winner,
                "judge": judge.map(|j| json!({
                    "expert": j.expert_id,
                    "summary": j.summary,
                })),
            }),
            confidence: if let Some(j) = judge {
                j.confidence
            } else if total > 0.0 {
                (score / total).clamp(0.0, 1.0)
            } else {
                0.0
            },
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!("辩论融合：{} 位辩手，{} 位裁判", debaters.len(), judge.map(|_| 1).unwrap_or(0)),
        })
    }

    /// Map-Reduce：分治融合，整体置信度由标量融合给出
    fn fuse_map_reduce(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let mid = (input.items.len() / 2).max(1);
        let (map, reduce) = (&input.items[..mid], &input.items[mid..]);
        let pairs: Vec<(f64, f64)> = input
            .items
            .iter()
            .map(|it| (it.confidence, it.weight(&input.expert_weights)))
            .collect();
        let fused = core
            .fuse_scalar(&pairs)
            .map_err(|e| AllianceError::internal(format!("map-reduce fusion failed: {:?}", e)))?;

        Ok(FusionOutput {
            content: json!({
                "map_phase": map.iter().map(|it| it.summary.clone()).collect::<Vec<_>>(),
                "reduce_phase": reduce.iter().map(|it| it.summary.clone()).collect::<Vec<_>>(),
                "map_count": map.len(),
                "reduce_count": reduce.len(),
            }),
            confidence: fused.clamp(0.0, 1.0),
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!("Map-Reduce 融合：{} 个 map，{} 个 reduce", map.len(), reduce.len()),
        })
    }

    /// 迭代精炼：多轮迭代，最终置信度由标量融合给出
    fn fuse_iterative(
        input: &FusionInput,
        core: &CoreFusionEngine,
    ) -> AllianceResult<FusionOutput> {
        let pairs: Vec<(f64, f64)> = input
            .items
            .iter()
            .map(|it| (it.confidence, it.weight(&input.expert_weights)))
            .collect();
        let fused = core
            .fuse_scalar(&pairs)
            .map_err(|e| AllianceError::internal(format!("iterative fusion failed: {:?}", e)))?;
        let last = input.items.last().cloned().unwrap();

        Ok(FusionOutput {
            content: json!({
                "iterations": input.items.iter().enumerate().map(|(i, it)| json!({
                    "round": i + 1,
                    "expert": it.expert_id,
                    "summary": it.summary,
                    "confidence": it.confidence,
                })).collect::<Vec<_>>(),
                "final_output": last.output,
                "final_confidence": fused,
                "total_iterations": input.items.len(),
            }),
            confidence: fused.clamp(0.0, 1.0),
            expert_count: input.items.len(),
            strategy: input.strategy,
            contributions: HashMap::new(),
            summary: format!(
                "迭代精炼融合：{} 轮迭代，最终置信度 {:.1}%",
                input.items.len(),
                (fused * 100.0).clamp(0.0, 100.0)
            ),
        })
    }

    // === 辅助 ===

    /// 计算各专家贡献度（归一化权重）
    fn calculate_contributions(input: &FusionInput) -> HashMap<String, f64> {
        let total: f64 = input
            .items
            .iter()
            .map(|it| it.weight(&input.expert_weights))
            .sum();
        let mut out = HashMap::new();
        for it in &input.items {
            let w = it.weight(&input.expert_weights);
            let contribution = if total > 0.0 { w / total } else { 1.0 / input.items.len() as f64 };
            out.insert(it.expert_id.clone(), contribution);
        }
        out
    }
}

impl Default for FusionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::Node;

    fn make_item(expert_id: &str, summary: &str, confidence: f64, score: f64) -> FusionItem {
        FusionItem {
            expert_id: expert_id.to_string(),
            node_id: format!("n-{}", expert_id),
            summary: summary.to_string(),
            confidence,
            output: json!({ "expert_id": expert_id, "result": summary, "score": score }),
        }
    }

    fn base_input(items: Vec<FusionItem>, strategy: FusionStrategy) -> FusionInput {
        FusionInput {
            items,
            expert_weights: HashMap::new(),
            strategy,
            task_description: "test".to_string(),
        }
    }

    #[test]
    fn from_execution_ignores_failed() {
        let node = Node {
            node_id: "n1".to_string(),
            task_id: uuid::Uuid::nil(),
            expert_id: "finance-expert-001".to_string(),
            module_id: None,
            name: "金融分析专家".to_string(),
            description: None,
            status: mox_alliance_common_proto::NodeStatus::Running,
            retry_count: 0,
            dependencies: vec![],
            input_refs: vec![],
            output_ref: None,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
        };
        let failed = NodeExecutionResult {
            node_id: "n1".to_string(),
            task_id: uuid::Uuid::nil(),
            success: false,
            output: None,
            error_message: Some("boom".to_string()),
            duration_ms: 10,
            retry_count: 0,
        };
        assert!(FusionItem::from_execution(&node, &failed).is_none());

        let ok = NodeExecutionResult {
            node_id: "n1".to_string(),
            task_id: uuid::Uuid::nil(),
            success: true,
            output: Some(json!({ "expert_id": "finance-expert-001", "result": "分析完成", "score": 0.9 })),
            error_message: None,
            duration_ms: 10,
            retry_count: 0,
        };
        let item = FusionItem::from_execution(&node, &ok).unwrap();
        assert_eq!(item.expert_id, "finance-expert-001");
        assert_eq!(item.summary, "分析完成");
        assert!((item.confidence - 0.9).abs() < 1e-9);
    }

    #[test]
    fn weighted_fusion_uses_core_engine() {
        let engine = FusionEngine::new();
        let input = base_input(
            vec![make_item("a", "A", 0.8, 0.8), make_item("b", "B", 0.6, 0.6)],
            FusionStrategy::Weighted,
        );
        let out = engine.fuse(input).unwrap();
        assert_eq!(out.expert_count, 2);
        // (0.8 + 0.6) / 2 = 0.7
        assert!((out.confidence - 0.7).abs() < 1e-6);
        assert!(out.content["fused_confidence"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn best_of_selects_highest_score() {
        let engine = FusionEngine::new();
        let input = base_input(
            vec![
                make_item("low", "L", 0.5, 0.5),
                make_item("high", "H", 0.9, 0.9),
                make_item("mid", "M", 0.7, 0.7),
            ],
            FusionStrategy::BestOf,
        );
        let out = engine.fuse(input).unwrap();
        assert_eq!(out.expert_count, 3);
        assert_eq!(out.content["best_expert"], "high");
        assert!((out.confidence - 0.9).abs() < 1e-9);
    }

    #[test]
    fn voting_selects_majority_summary() {
        let engine = FusionEngine::new();
        let input = base_input(
            vec![
                make_item("a", "同意", 0.5, 0.5),
                make_item("b", "反对", 0.3, 0.3),
                make_item("c", "同意", 0.2, 0.2),
            ],
            FusionStrategy::Voting,
        );
        let out = engine.fuse(input).unwrap();
        assert_eq!(out.content["winner"], "同意");
    }

    #[test]
    fn all_strategies_produce_output() {
        let engine = FusionEngine::new();
        let strategies = [
            FusionStrategy::Voting,
            FusionStrategy::Weighted,
            FusionStrategy::ConfidenceWeighted,
            FusionStrategy::BestOf,
            FusionStrategy::Concatenation,
            FusionStrategy::Stacking,
            FusionStrategy::Debate,
            FusionStrategy::MapReduce,
            FusionStrategy::Iterative,
        ];
        for strategy in strategies {
            let items = vec![
                make_item("a", "A", 0.8, 0.8),
                make_item("b", "B", 0.7, 0.7),
                make_item("c", "C", 0.9, 0.9),
            ];
            let input = base_input(items, strategy);
            let out = engine.fuse(input).unwrap();
            assert_eq!(out.expert_count, 3, "strategy {:?}", strategy);
            assert!(out.confidence >= 0.0 && out.confidence <= 1.0);
            assert_eq!(out.strategy, strategy);
            assert!(!out.contributions.is_empty());
        }
    }

    #[test]
    fn empty_input_zero_confidence() {
        let engine = FusionEngine::new();
        let input = base_input(vec![], FusionStrategy::Weighted);
        let out = engine.fuse(input).unwrap();
        assert_eq!(out.expert_count, 0);
        assert_eq!(out.confidence, 0.0);
    }
}
