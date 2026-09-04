// =============================================================================
// 质量分与等级（ABCD 四级门禁）
// =============================================================================
// 跨端对齐：Python 和 前端必须使用相同的等级阈值和语义。
// =============================================================================

use serde::{Deserialize, Serialize};

// =============================================================================
// 质量等级
// =============================================================================

/// 质量等级（A/B/C/D 四级门禁）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum QualityGrade {
    /// 优秀 - 通过，优质交付
    A,
    /// 良好 - 通过，标准交付
    B,
    /// 合格 - 有条件通过，可重试优化
    C,
    /// 不合格 - 阻断，必须修复后重新提交
    D,
}

impl QualityGrade {
    /// 等级标签
    pub fn label(&self) -> &'static str {
        match self {
            QualityGrade::A => "优秀",
            QualityGrade::B => "良好",
            QualityGrade::C => "合格",
            QualityGrade::D => "不合格",
        }
    }

    /// 是否通过门禁
    pub fn passed(&self) -> bool {
        matches!(self, QualityGrade::A | QualityGrade::B)
    }

    /// 是否需要重试（C级可重试优化）
    pub fn retryable(&self) -> bool {
        matches!(self, QualityGrade::C)
    }

    /// 是否阻断（D级必须修复）
    pub fn blocked(&self) -> bool {
        matches!(self, QualityGrade::D)
    }

    /// 等级颜色（前端用）
    pub fn color(&self) -> &'static str {
        match self {
            QualityGrade::A => "#10b981", // 绿
            QualityGrade::B => "#06b6d4", // 青
            QualityGrade::C => "#f59e0b", // 黄
            QualityGrade::D => "#ef4444", // 红
        }
    }
}

impl std::fmt::Display for QualityGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

// =============================================================================
// 等级阈值（SSOT）
// =============================================================================

/// 质量等级阈值配置
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GateThresholds {
    /// A 级最低分
    pub a: f64,
    /// B 级最低分
    pub b: f64,
    /// C 级最低分
    pub c: f64,
}

/// 默认等级阈值（SSOT，跨端必须一致）
pub const GATE_THRESHOLDS: GateThresholds = GateThresholds {
    a: 0.85,
    b: 0.70,
    c: 0.50,
};

impl Default for GateThresholds {
    fn default() -> Self {
        GATE_THRESHOLDS
    }
}

impl GateThresholds {
    /// 根据分数计算等级
    pub fn grade_from_score(&self, score: f64) -> QualityGrade {
        let s = score.clamp(0.0, 1.0);
        if s >= self.a {
            QualityGrade::A
        } else if s >= self.b {
            QualityGrade::B
        } else if s >= self.c {
            QualityGrade::C
        } else {
            QualityGrade::D
        }
    }
}

// =============================================================================
// 质量分（值对象）
// =============================================================================

/// 质量分（0.0 - 1.0）
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct QualityScore(f64);

impl QualityScore {
    /// 创建质量分（自动 clamp 到 0-1）
    pub fn new(score: f64) -> Self {
        Self(score.clamp(0.0, 1.0))
    }

    /// 获取分数值
    pub fn value(&self) -> f64 {
        self.0
    }

    /// 计算等级
    pub fn grade(&self) -> QualityGrade {
        GATE_THRESHOLDS.grade_from_score(self.0)
    }

    /// 百分比形式（0-100）
    pub fn percentage(&self) -> f64 {
        self.0 * 100.0
    }
}

impl Default for QualityScore {
    fn default() -> Self {
        Self(0.0)
    }
}

impl From<f64> for QualityScore {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for QualityScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

// =============================================================================
// 门禁结果
// =============================================================================

/// 质量门禁结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    /// 总分（0.0 - 1.0）
    pub score: f64,
    /// 等级
    pub grade: QualityGrade,
    /// 是否通过
    pub passed: bool,
    /// 各维度得分（维度名 -> 分数）
    #[serde(default)]
    pub dimensions: std::collections::BTreeMap<String, f64>,
    /// 阻断原因（D级时必填）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
    /// 改进建议（C级时提供）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
    /// 评估耗时（毫秒）
    #[serde(default)]
    pub latency_ms: u64,
}

impl GateResult {
    /// 创建通过结果
    pub fn pass(score: f64, grade: QualityGrade) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            grade,
            passed: grade.passed(),
            dimensions: Default::default(),
            block_reason: None,
            suggestions: vec![],
            latency_ms: 0,
        }
    }

    /// 创建阻断结果
    pub fn block(score: f64, reason: impl Into<String>) -> Self {
        Self {
            score: score.clamp(0.0, 1.0),
            grade: QualityGrade::D,
            passed: false,
            dimensions: Default::default(),
            block_reason: Some(reason.into()),
            suggestions: vec![],
            latency_ms: 0,
        }
    }

    /// 从分数自动计算等级
    pub fn from_score(score: f64) -> Self {
        let grade = GATE_THRESHOLDS.grade_from_score(score);
        Self::pass(score, grade)
    }

    /// 添加维度得分
    pub fn with_dimension(mut self, name: impl Into<String>, score: f64) -> Self {
        self.dimensions.insert(name.into(), score.clamp(0.0, 1.0));
        self
    }

    /// 添加改进建议
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grade_thresholds_boundary() {
        assert_eq!(GATE_THRESHOLDS.grade_from_score(0.85), QualityGrade::A);
        assert_eq!(GATE_THRESHOLDS.grade_from_score(0.84), QualityGrade::B);
        assert_eq!(GATE_THRESHOLDS.grade_from_score(0.70), QualityGrade::B);
        assert_eq!(GATE_THRESHOLDS.grade_from_score(0.69), QualityGrade::C);
        assert_eq!(GATE_THRESHOLDS.grade_from_score(0.50), QualityGrade::C);
        assert_eq!(GATE_THRESHOLDS.grade_from_score(0.49), QualityGrade::D);
        assert_eq!(GATE_THRESHOLDS.grade_from_score(0.0), QualityGrade::D);
        assert_eq!(GATE_THRESHOLDS.grade_from_score(1.0), QualityGrade::A);
    }

    #[test]
    fn grade_passed_logic() {
        assert!(QualityGrade::A.passed());
        assert!(QualityGrade::B.passed());
        assert!(!QualityGrade::C.passed());
        assert!(!QualityGrade::D.passed());
        assert!(QualityGrade::C.retryable());
        assert!(QualityGrade::D.blocked());
    }

    #[test]
    fn quality_score_clamp() {
        assert_eq!(QualityScore::new(1.5).value(), 1.0);
        assert_eq!(QualityScore::new(-0.5).value(), 0.0);
        assert_eq!(QualityScore::new(0.75).value(), 0.75);
    }

    #[test]
    fn gate_result_from_score() {
        let result = GateResult::from_score(0.90);
        assert_eq!(result.grade, QualityGrade::A);
        assert!(result.passed);
    }

    #[test]
    fn gate_result_block() {
        let result = GateResult::block(0.30, "安mox 模块化系统架构维度不达标");
        assert_eq!(result.grade, QualityGrade::D);
        assert!(!result.passed);
        assert_eq!(result.block_reason, Some("安mox 模块化系统架构维度不达标".to_string()));
    }

    #[test]
    fn quality_grade_serialization() {
        let json = serde_json::to_string(&QualityGrade::A).unwrap();
        assert_eq!(json, "\"A\"");
        let parsed: QualityGrade = serde_json::from_str("\"B\"").unwrap();
        assert_eq!(parsed, QualityGrade::B);
    }
}
