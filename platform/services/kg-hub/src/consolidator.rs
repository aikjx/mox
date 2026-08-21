//! 轨迹固化器 — 将引擎执行轨迹转化为长期知识
//!
//! 三段固化流程，对应认知科学中记忆固化的三个层次：
//!
//! | 阶段 | 方法 | 认知学对应 | 产出 |
//! |---|---|---|---|
//! | 保存情景记忆 | [`TraceConsolidator::save_episodic`] | 情景记忆 (Episodic) | 带时间戳的执行快照 |
//! | 抽取实体关系 | [`TraceConsolidator::extract_to_kg`] | 语义记忆 (Semantic) | 知识图谱增量 |
//! | 固化操作模式 | [`TraceConsolidator::distill_to_operator`] | 程序性记忆 (Procedural) | 可复用的算子/模板 |
//!
//! 设计为无副作用（当前阶段仅留痕与日志），便于在引擎 CONSOLIDATE 阶段
//! 安全接入——即使固化器异常，也不会阻断主流程。

use serde::{Deserialize, Serialize};

/// 引擎执行轨迹 — 固化器的唯一输入
///
/// 从 [`ai-agent::engine::EngineContext`] 转换而来，
/// 携带一次完整引擎周期中感知、行动、反思、生成的全部上下文。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngineTrace {
    /// 原始任务描述
    pub task: String,
    /// 感知阶段收集的观察
    pub observations: Vec<String>,
    /// ACT 阶段的执行结果
    pub action_results: Vec<String>,
    /// REFLECT 阶段的反思条目
    pub reflections: Vec<String>,
    /// GENERATE 阶段的最终输出
    pub generated_output: Option<String>,
}

/// 固化结果 — 三段固化的汇总快照
///
/// 每段均有独立的成功标志和计数，便于上层做审计与告警。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidationResult {
    /// 情景记忆是否保存成功
    pub episodic_saved: bool,
    /// 图谱抽取是否成功
    pub kg_extracted: bool,
    /// 程序性记忆是否固化成功
    pub operator_distilled: bool,
    /// 保存的情景条目数
    pub episodic_count: usize,
    /// 抽取的实体数
    pub kg_entities: usize,
    /// 抽取的关系数
    pub kg_relations: usize,
    /// 固化的操作模式数
    pub operator_count: usize,
    /// 人类可读的汇总描述
    pub summary: String,
}

/// 轨迹固化器
///
/// 无状态设计（当前阶段），所有依赖通过方法参数注入。
/// 后续可扩展为持有 [`crate::KgHub`] 引用以实现真正的图谱写入。
#[derive(Debug, Default)]
pub struct TraceConsolidator;

impl TraceConsolidator {
    pub fn new() -> Self {
        Self
    }

    /// 执行完整的三段固化流程
    pub fn consolidate(&self, trace: &EngineTrace) -> ConsolidationResult {
        let (episodic_saved, episodic_count) = self.save_episodic(trace);
        let (kg_extracted, kg_entities, kg_relations) = self.extract_to_kg(trace);
        let (operator_distilled, operator_count) = self.distill_to_operator(trace);

        let summary = format!(
            "情景 {} 条 | 图谱 {} 实体 / {} 关系 | 程序性 {} 条",
            episodic_count, kg_entities, kg_relations, operator_count
        );

        tracing::info!(
            target: "consolidator",
            task = %trace.task,
            episodic_saved,
            kg_extracted,
            operator_distilled,
            "固化完成: {}", summary
        );

        ConsolidationResult {
            episodic_saved,
            kg_extracted,
            operator_distilled,
            episodic_count,
            kg_entities,
            kg_relations,
            operator_count,
            summary,
        }
    }

    /// ① 保存情景记忆
    ///
    /// 将本次引擎执行的感知、行动、反思作为一个带时间锚点的
    /// "情景快照" 持久化。情景记忆是后续图谱抽取的原始素材。
    fn save_episodic(&self, trace: &EngineTrace) -> (bool, usize) {
        let count = trace.observations.len() + trace.action_results.len();
        tracing::debug!(
            target: "consolidator",
            task = %trace.task,
            "save_episodic: 保存情景快照 {} 条 (观察 {} + 行动 {})",
            count,
            trace.observations.len(),
            trace.action_results.len()
        );
        (true, count)
    }

    /// ② 抽取实体关系到知识图谱
    ///
    /// 从情景记忆中识别实体（反思条目 → 实体候选）和关系（行动结果 → 边候选），
    /// 合并进 KG-Hub 的统一图谱。
    fn extract_to_kg(&self, trace: &EngineTrace) -> (bool, usize, usize) {
        let entities = trace.reflections.len();
        let relations = trace.action_results.len();
        tracing::debug!(
            target: "consolidator",
            task = %trace.task,
            "extract_to_kg: 抽取 {} 实体 / {} 关系",
            entities,
            relations
        );
        (true, entities, relations)
    }

    /// ③ 固化为程序性记忆
    ///
    /// 当引擎产出了最终输出（`generated_output` 为 `Some`），
    /// 说明本次执行形成了一个可复用的操作模式，应被提炼为
    /// 程序性记忆（算子/工作流模板）。
    fn distill_to_operator(&self, trace: &EngineTrace) -> (bool, usize) {
        let count = if trace.generated_output.is_some() {
            1
        } else {
            0
        };
        tracing::debug!(
            target: "consolidator",
            task = %trace.task,
            "distill_to_operator: 固化程序性记忆 {} 条",
            count
        );
        (true, count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_trace() -> EngineTrace {
        EngineTrace {
            task: "分析销售数据".to_string(),
            observations: vec!["感知到数据源 sales.csv".into()],
            action_results: vec!["执行统计查询".into(), "生成图表".into()],
            reflections: vec!["数据完整".into(), "图表清晰".into()],
            generated_output: Some("销售分析报告".to_string()),
        }
    }

    #[test]
    fn full_consolidation_records_all_stages() {
        let c = TraceConsolidator::new();
        let r = c.consolidate(&sample_trace());
        assert!(r.episodic_saved);
        assert!(r.kg_extracted);
        assert!(r.operator_distilled);
        assert_eq!(r.episodic_count, 3);
        assert_eq!(r.kg_entities, 2);
        assert_eq!(r.kg_relations, 2);
        assert_eq!(r.operator_count, 1);
        assert!(!r.summary.is_empty());
    }

    #[test]
    fn no_output_yields_zero_operator() {
        let trace = EngineTrace {
            generated_output: None,
            ..sample_trace()
        };
        let c = TraceConsolidator::new();
        let r = c.consolidate(&trace);
        assert_eq!(r.operator_count, 0);
    }

    #[test]
    fn empty_trace_still_succeeds() {
        let trace = EngineTrace::default();
        let c = TraceConsolidator::new();
        let r = c.consolidate(&trace);
        assert!(r.episodic_saved);
        assert_eq!(r.episodic_count, 0);
        assert_eq!(r.kg_entities, 0);
        assert_eq!(r.operator_count, 0);
    }
}