//! # Operator Core
//!
//! 算子统一系统核心库，实现六条数学公理：
//! 1. 万物皆算子 - Operator trait
//! 2. 系统状态高维向量 - StateVector
//! 3. 关联关系加权有向图 - 在graph-algorithms crate中实现
//! 4. 插件满足范畴论态射规则 - Category组合子
//! 5. 资源约束优化 - ResourceConstraints
//! 6. 扩展性闭包 - 算子代数运算

use std::any::TypeId;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod types;
pub mod state;
pub mod operator;
pub mod category;
pub mod resource;
pub mod conservation;
pub mod monad;
pub mod engine;

pub use types::*;
pub use state::*;
pub use operator::*;
pub use category::*;
pub use resource::*;
pub use conservation::*;
pub use monad::*;

/// 系统核心错误类型
#[derive(Debug, Error)]
pub enum OperatorError {
    #[error("类型不匹配: 期望 {expected:?}, 得到 {actual:?}")]
    TypeMismatch {
        expected: TypeId,
        actual: TypeId,
    },

    #[error("守恒律违反: {law} - 残差 {residual} 超过阈值 {threshold}")]
    ConservationViolation {
        law: String,
        residual: f64,
        threshold: f64,
    },

    #[error("资源不足: 需要 {required}, 可用 {available}")]
    ResourceExhausted {
        required: String,
        available: String,
    },

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

/// 系统全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// 状态空间维度
    pub state_dimension: usize,
    /// 残差阈值
    pub residual_threshold: f64,
    /// 最大CPU使用率
    pub max_cpu_usage: f64,
    /// 最大内存使用(字节)
    pub max_memory_bytes: u64,
    /// 最大执行时间(毫秒)
    pub max_execution_time_ms: u64,
    /// 是否启用守恒律检查
    pub enable_conservation_check: bool,
    /// 是否启用类型检查
    pub enable_type_check: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            state_dimension: 1024,
            residual_threshold: 1e-10,
            max_cpu_usage: 1.0,
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_execution_time_ms: 30000,
            enable_conservation_check: true,
            enable_type_check: true,
        }
    }
}

/// 算子执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub config: SystemConfig,
    pub trace_id: String,
    pub resources: ResourceUsage,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            config: SystemConfig::default(),
            trace_id: uuid::Uuid::new_v4().to_string(),
            resources: ResourceUsage::default(),
            metadata: HashMap::new(),
        }
    }
}

/// 算子执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output_state: Option<StateVector>,
    pub residual: f64,
    pub resources_used: ResourceUsage,
    pub execution_time_ms: u64,
    pub logs: Vec<String>,
    pub error: Option<String>,
}

/// 算子元数据
///
/// `input_type` / `output_type` 使用 `TypeIdentifier` 而非原始字符串，
/// 使得算子类型可以参与编译期检查（通过 `TypeCheck` trait），
/// 同时保留 `name` 字段供人类可读的描述。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub input_type: TypeIdentifier,
    pub output_type: TypeIdentifier,
    pub resource_cost: ResourceCost,
    pub author: String,
    pub tags: Vec<String>,
}

impl OperatorMetadata {
    /// 从实现了 `TypeCheck` 的算子构造元数据，自动填充类型标识
    pub fn from_operator<O: crate::types::TypeCheck>(op: &O, id: String, name: String) -> Self {
        Self {
            id,
            name,
            version: "1.0.0".to_string(),
            description: String::new(),
            input_type: op.input_type(),
            output_type: op.output_type(),
            resource_cost: ResourceCost::default(),
            author: "System".to_string(),
            tags: Vec::new(),
        }
    }

    /// 构造后链式设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 构造后链式设置版本
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// 构造后链式设置作者
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// 构造后链式设置标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 构造后链式设置资源消耗
    pub fn with_resource_cost(mut self, cost: ResourceCost) -> Self {
        self.resource_cost = cost;
        self
    }
}

/// 生成唯一算子ID
pub fn generate_operator_id() -> String {
    format!("op-{}", &uuid::Uuid::new_v4().to_string()[..8])
}

/// 初始化日志
pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .with_thread_ids(true)
        .init();
}

// 重导出uuid
pub use uuid;
