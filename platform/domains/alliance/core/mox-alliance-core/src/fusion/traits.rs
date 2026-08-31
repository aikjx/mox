// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 融合策略抽象 Trait
//!
//! 定义统一的融合策略接口，所有融合策略都实现此 trait，
//! 便于通过 FusionEngine 统一调度。

use crate::fusion::error::FusionResult;

/// 融合策略统一接口
///
/// 所有融合策略都实现此 trait，提供统一的 `fuse` 方法。
/// 输入是多个带评分的候选项，输出是融合后的结果。
///
/// # 设计原则
/// - **纯函数**：输入确定则输出确定，无副作用
/// - **零 IO**：不涉及任何外部资源访问
/// - **泛型化**：支持多种输入类型，方便不同场景使用
///
/// # 类型参数
/// - `Item` - 候选项类型，需可克隆用于结果返回
pub trait FusionStrategy<Item: Clone> {
    /// 策略名称（用于日志和调试）
    fn name(&self) -> &'static str;

    /// 执行融合
    ///
    /// # Arguments
    /// * `candidates` - 候选项列表，每个元素是 (内容, 基础分数/权重) 元组
    ///
    /// # Returns
    /// 融合后的结果列表，按分数降序排列
    ///
    /// # Errors
    /// 当输入无效（空列表、权重异常等）时返回 `FusionError`
    fn fuse(&self, candidates: &[(Item, f64)]) -> FusionResult<Vec<(Item, f64)>>;
}

/// 标量融合策略
///
/// 适用于将多个数值型结果融合为单个数值的策略。
/// 与 `FusionStrategy` 不同，此 trait 返回单个值而非排序列表。
pub trait ScalarFusionStrategy {
    /// 策略名称
    fn name(&self) -> &'static str;

    /// 执行标量融合
    ///
    /// # Arguments
    /// * `values` - 值与权重/置信度列表
    ///
    /// # Returns
    /// 融合后的单个标量值
    fn fuse_scalar(&self, values: &[(f64, f64)]) -> FusionResult<f64>;
}

/// 分类融合策略
///
/// 适用于分类/投票场景，从多个离散选项中选出最优。
pub trait ClassificationFusionStrategy<Category: Eq + std::hash::Hash + Clone> {
    /// 策略名称
    fn name(&self) -> &'static str;

    /// 执行分类融合
    ///
    /// # Arguments
    /// * `votes` - 投票列表，每个元素是 (类别, 权重/置信度)
    ///
    /// # Returns
    /// (胜出类别, 最终得分, 总权重)
    fn fuse_classification(
        &self,
        votes: &[(Category, f64)],
    ) -> FusionResult<(Category, f64, f64)>;
}
