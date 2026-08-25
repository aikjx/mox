//! 专家联盟 6 阶段管线 · 硬约束常量（HC-2, HC-5, HC-8, HC-9）
//!
//! 本文件所有值为项目记忆 + TOP-MASTER 企业级基线锁死常量。
//! 禁止做任何修改；修改必须走 ADR 申请流程并由算法联盟 + 产品联盟同时放行。

/// 激活扩散方法：个性化 PageRank 特例 method=spread（HC-2）
pub const SPREAD_METHOD: &str = "spread";
/// 激活扩散阻尼因子 d（HC-2）：必须 = 0.85
pub const SPREAD_DAMPING: f64 = 0.85;
/// 激活扩散收敛轮数（HC-2）：必须 = 30
pub const SPREAD_ROUNDS: u32 = 30;

/// RRF（倒数秩融合）k 值（HC-8 家族固定）
pub const RRF_K: u32 = 60;
/// 激活扩散在 RRF 融合中的权重（HC-8 家族固定）：最终分数 = (1-sw)*RRF_keyword + sw*RRF_spread
pub const SPREAD_WEIGHT: f64 = 0.7;

/// 质量门禁 A 级阈值：综合分 ≥ 0.90
pub const GATE_THRESHOLD_A: f64 = 0.90;
/// 质量门禁 B 级阈值：综合分 ≥ 0.80
pub const GATE_THRESHOLD_B: f64 = 0.80;
/// 质量门禁 C 级阈值：综合分 ≥ 0.70（< 0.70 = D）
pub const GATE_THRESHOLD_C: f64 = 0.70;

/// 单轮辩论最大 token 数（EAF-STD 4.3）：不得超过
pub const DEBATE_MAX_TOKENS_PER_ROUND: usize = 900;
/// 单专家并行咨询超时秒数（EAF-STD 4.3 超时隔离）：超过则跳过
pub const EXPERT_TIMEOUT_SECS: u64 = 60;

/// 多目标加权评估统一公式（HC-8）：任何评分计算的权重、变量名不得替换
pub const QUALITY_FORMULA: &str =
    "0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability";

/// 7 类基准任务分类（HC-9，缺任何一项 = 企业基线不通过）
pub const INTENT_CLASSES: [&str; 7] = [
    "math",         // 数学
    "logic",        // 逻辑
    "knowledge",    // 知识
    "code",         // 代码
    "chinese",      // 中文
    "timeliness",   // 时效
    "instruction",  // 指令
];

/// 6 阶段管线名（用于事件与审计的稳定字符串）
pub const PHASE_NAMES: [&str; 7] = [
    "intent",       // 01 意图识别
    "team",         // 02 组队路由
    "debate",       // 03 并行咨询 + 辩论
    "synthesize",   // 04 归一合成
    "gate",         // 05 质量门禁
    "learn",        // 06 指标学习
    "done",         // 07 终态
];

#[cfg(test)]
mod tests {
    use super::*;

    /// 基线锁定：以上常量值若被改动，此测试必须失败（防漂移 CI 闸）
    #[test]
    fn hard_constants_locked_no_drift() {
        // HC-2: 激活扩散三件套
        assert_eq!(SPREAD_METHOD, "spread");
        assert!((SPREAD_DAMPING - 0.85).abs() < f64::EPSILON);
        assert_eq!(SPREAD_ROUNDS, 30);
        // HC-8 家族：RRF + spread_weight
        assert_eq!(RRF_K, 60);
        assert!((SPREAD_WEIGHT - 0.7).abs() < f64::EPSILON);
        // 门禁阈值（硬编码 A/B/C）
        assert!((GATE_THRESHOLD_A - 0.90).abs() < f64::EPSILON);
        assert!((GATE_THRESHOLD_B - 0.80).abs() < f64::EPSILON);
        assert!((GATE_THRESHOLD_C - 0.70).abs() < f64::EPSILON);
        // EAF-STD 4.3
        assert_eq!(DEBATE_MAX_TOKENS_PER_ROUND, 900);
        assert_eq!(EXPERT_TIMEOUT_SECS, 60);
        // HC-8 统一公式不得被修改文案
        assert_eq!(
            QUALITY_FORMULA,
            "0.55×Quality + 0.20×Speed + 0.10×TokenEfficiency + 0.15×Stability"
        );
        // HC-9 7 类基准完整
        assert_eq!(INTENT_CLASSES.len(), 7);
        assert_eq!(INTENT_CLASSES[0], "math");
        assert_eq!(INTENT_CLASSES[6], "instruction");
        // 6 阶段 + done
        assert_eq!(PHASE_NAMES.len(), 7);
    }

    /// 7 类基准完整度：7 条不同字符串，无重复
    #[test]
    fn intent_classes_7_distinct() {
        use std::collections::HashSet;
        let set: HashSet<&str> = INTENT_CLASSES.iter().copied().collect();
        assert_eq!(set.len(), 7);
    }
}
