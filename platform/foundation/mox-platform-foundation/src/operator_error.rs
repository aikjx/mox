/// 系统核心错误类型
#[derive(Debug, thiserror::Error)]
pub enum OperatorError {
    #[error("类型不匹配: 期望 {expected:?}, 得到 {actual:?}")]
    TypeMismatch { expected: std::any::TypeId, actual: std::any::TypeId },

    #[error("守恒律违反: {law} - 残差 {residual} 超过阈值 {threshold}")]
    ConservationViolation {
        law: String,
        residual: f64,
        threshold: f64,
    },

    #[error("资源不足: 需要 {required}, 可用 {available}")]
    ResourceExhausted { required: String, available: String },

    #[error("算子组合错误: {0}")]
    CompositionError(String),

    #[error("WASM插件错误: {0}")]
    WasmError(String),

    #[error("执行错误: {0}")]
    ExecutionError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OperatorError>;
