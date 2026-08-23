//! # Operator Core
//!
//! 算子统一系统核心库，实现六条数学公理：
//! 1. 万物皆算子 - Operator trait
//! 2. 系统状态高维向量 - StateVector
//! 3. 关联关系加权有向图 - 在graph-algorithms crate中实现
//! 4. 插件满足范畴论态射规则 - Category组合子
//! 5. 资源约束优化 - ResourceConstraints
//! 6. 扩展性闭包 - 算子代数运算

/// 璇玑系统 Crate 注册常量（图谱自同步契约：Rust 端显式声明 crate 身份）。
/// AIS 自动发现 / project-atlas self-sync / 图谱 CRATE_ID ↔ node.id 双向绑定基准。
pub const CRATE_ID: &str = "operator-core";

/// 璇玑系统 Crate 结构化元数据。
/// AIS 分层声明、owner 项目、能力清单、数据读写表契约。
#[derive(Debug, Clone, Copy)]
pub struct CrateMeta {
    /// 稳定唯一标识（v4 UUID，生成一次后永不变更，跨生命周期用于 atlas 关联）。
    pub uuid: &'static str,
    /// AIS 架构分层：L1接入/L2网关/L3域服务/L4核心算法/L5持久化/L6纯核心/L7工具。
    pub ais_layers: &'static [&'static str],
    /// 归属项目 id（与 project-registry.js PROJECTS.id 常量严格匹配）。
    pub owner_project: &'static str,
    /// 对外暴露能力列表（human-readable，用于图谱能力矩阵自描述）。
    pub capabilities: &'static [&'static str],
    /// 读取的持久化表名或 JSON data 文件名。
    pub data_tables_read: &'static [&'static str],
    /// 写入的持久化表名（L3/L4 应多为 empty，仅 L5 拥有写入）。
    pub data_tables_write: &'static [&'static str],
}

pub const CRATE_META: CrateMeta = CrateMeta {
    uuid: "a1f7c3e2-8b4d-4a5e-9c1d-2e3f4a5b6c7d",
    ais_layers: &["L4-Core", "L6-Kernel"],
    owner_project: "proj-graph-infra",
    capabilities: &[
        "Operator trait 统一算子抽象",
        "StateVector 高维状态向量",
        "Category 范畴论态射组合子",
        "ResourceConstraints 资源约束建模",
        "Monad 算子代数闭包",
        "Registry 算子注册表",
        "Conservation 守恒律引擎",
    ],
    data_tables_read: &["operators.json"],
    data_tables_write: &[],
};

use std::any::TypeId;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// L6 纯内核层：零外部依赖，仅 std。定义算子纯数据结构、trait、标量运算。
pub mod kernel;
/// L5 扩展层：为 kernel 纯类型提供 serde 派生实现 + 为 nalgebra 类型 impl kernel trait（DIP）。
pub mod kernel_ext;

pub mod types;
pub mod state;
pub mod operator;
pub mod category;
pub mod resource;
pub mod conservation;
pub mod monad;
pub mod engine;
pub mod registry;

pub use types::*;
pub use state::*;
pub use operator::*;
pub use category::*;
pub use resource::*;
pub use conservation::*;
pub use monad::*;
pub use registry::*;

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
