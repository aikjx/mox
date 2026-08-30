// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS)
// Licensed under the MIT License.

//! 算法联盟核心类型定义

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 算法 ID
pub type AlgorithmId = String;

/// 算法类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlgorithmCategory {
    /// 图算法
    Graph,
    /// 编码算法（纠删码、压缩、加密等）
    Encoding,
    /// 优化算法
    Optimization,
    /// 统计分析
    Statistics,
    /// 机器学习/向量计算
    MachineLearning,
    /// 数据处理（清洗、转换等）
    DataProcessing,
    /// 流水线（组合算法）
    Pipeline,
    /// 其他
    Other,
}

impl AlgorithmCategory {
    /// 类别名称
    pub fn as_str(&self) -> &'static str {
        match self {
            AlgorithmCategory::Graph => "graph",
            AlgorithmCategory::Encoding => "encoding",
            AlgorithmCategory::Optimization => "optimization",
            AlgorithmCategory::Statistics => "statistics",
            AlgorithmCategory::MachineLearning => "machine_learning",
            AlgorithmCategory::DataProcessing => "data_processing",
            AlgorithmCategory::Pipeline => "pipeline",
            AlgorithmCategory::Other => "other",
        }
    }
}

impl fmt::Display for AlgorithmCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 算法状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlgorithmStatus {
    /// 可用
    Active,
    /// 弃用
    Deprecated,
    /// 实验性
    Experimental,
    /// 维护中
    Maintenance,
}

impl AlgorithmStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlgorithmStatus::Active => "active",
            AlgorithmStatus::Deprecated => "deprecated",
            AlgorithmStatus::Experimental => "experimental",
            AlgorithmStatus::Maintenance => "maintenance",
        }
    }
}

/// 计算模型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeModel {
    /// 批量同步并行 (Bulk Synchronous Parallel)
    BSP,
    /// 收集-应用-散射 (Gather-Apply-Scatter)
    GAS,
    /// 流式计算
    Streaming,
    /// SIMD 向量化
    SIMD,
    /// 单线程（默认）
    SingleThread,
    /// 多线程并行
    MultiThread,
    /// GPU 加速
    GPU,
}

impl ComputeModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComputeModel::BSP => "bsp",
            ComputeModel::GAS => "gas",
            ComputeModel::Streaming => "streaming",
            ComputeModel::SIMD => "simd",
            ComputeModel::SingleThread => "single_thread",
            ComputeModel::MultiThread => "multi_thread",
            ComputeModel::GPU => "gpu",
        }
    }
}

/// 数据形状描述
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataShape {
    /// 数据类型名
    pub type_name: String,
    /// 维度信息
    pub dimensions: Vec<u64>,
    /// 是否可选
    pub optional: bool,
}

impl DataShape {
    pub fn scalar(type_name: &str) -> Self {
        Self {
            type_name: type_name.to_string(),
            dimensions: vec![],
            optional: false,
        }
    }

    pub fn vector(type_name: &str, size: u64) -> Self {
        Self {
            type_name: type_name.to_string(),
            dimensions: vec![size],
            optional: false,
        }
    }

    pub fn graph() -> Self {
        Self {
            type_name: "graph".to_string(),
            dimensions: vec![],
            optional: false,
        }
    }

    pub fn object() -> Self {
        Self {
            type_name: "object".to_string(),
            dimensions: vec![],
            optional: false,
        }
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

/// 参数值类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParamValue {
    /// 整数
    Int(i64),
    /// 浮点数
    Float(f64),
    /// 布尔值
    Bool(bool),
    /// 字符串
    String(String),
    /// 整数列表
    IntList(Vec<i64>),
    /// 浮点数列表
    FloatList(Vec<f64>),
    /// 字符串列表
    StringList(Vec<String>),
    /// JSON 对象
    Json(serde_json::Value),
}

impl ParamValue {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            ParamValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            ParamValue::Float(v) => Some(*v),
            ParamValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParamValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ParamValue::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ParamValue::Int(_) => "int",
            ParamValue::Float(_) => "float",
            ParamValue::Bool(_) => "bool",
            ParamValue::String(_) => "string",
            ParamValue::IntList(_) => "int_list",
            ParamValue::FloatList(_) => "float_list",
            ParamValue::StringList(_) => "string_list",
            ParamValue::Json(_) => "json",
        }
    }
}

impl From<i64> for ParamValue {
    fn from(v: i64) -> Self {
        ParamValue::Int(v)
    }
}

impl From<f64> for ParamValue {
    fn from(v: f64) -> Self {
        ParamValue::Float(v)
    }
}

impl From<bool> for ParamValue {
    fn from(v: bool) -> Self {
        ParamValue::Bool(v)
    }
}

impl From<String> for ParamValue {
    fn from(v: String) -> Self {
        ParamValue::String(v)
    }
}

impl From<&str> for ParamValue {
    fn from(v: &str) -> Self {
        ParamValue::String(v.to_string())
    }
}

/// 参数规格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamSpec {
    /// 参数名
    pub name: String,
    /// 参数类型
    pub param_type: String,
    /// 是否必需
    pub required: bool,
    /// 默认值
    pub default: Option<ParamValue>,
    /// 描述
    pub description: String,
    /// 最小值（数值类型）
    pub min_value: Option<f64>,
    /// 最大值（数值类型）
    pub max_value: Option<f64>,
    /// 可选值列表
    pub options: Option<Vec<ParamValue>>,
}

impl ParamSpec {
    pub fn new(name: &str, param_type: &str, required: bool, description: &str) -> Self {
        Self {
            name: name.to_string(),
            param_type: param_type.to_string(),
            required,
            default: None,
            description: description.to_string(),
            min_value: None,
            max_value: None,
            options: None,
        }
    }

    pub fn with_default(mut self, default: impl Into<ParamValue>) -> Self {
        self.default = Some(default.into());
        self
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }

    /// 验证参数值是否符合规格
    pub fn validate(&self, value: &ParamValue) -> Result<(), String> {
        // 类型检查（简化）
        if self.param_type != value.type_name()
            && !((self.param_type == "number"
                && matches!(value, ParamValue::Int(_) | ParamValue::Float(_)))
                || (self.param_type == "list"
                    && matches!(
                        value,
                        ParamValue::IntList(_)
                            | ParamValue::FloatList(_)
                            | ParamValue::StringList(_)
                    )))
        {
            return Err(format!(
                "type mismatch: expected {}, got {}",
                self.param_type,
                value.type_name()
            ));
        }

        // 范围检查
        if let (Some(min), Some(num)) = (self.min_value, value.as_float()) {
            if num < min {
                return Err(format!("value {} is less than min {}", num, min));
            }
        }
        if let (Some(max), Some(num)) = (self.max_value, value.as_float()) {
            if num > max {
                return Err(format!("value {} is greater than max {}", num, max));
            }
        }

        Ok(())
    }
}

/// 算法基本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmInfo {
    /// 算法 ID
    pub id: String,
    /// 算法名称
    pub name: String,
    /// 算法类别
    pub category: AlgorithmCategory,
    /// 版本号
    pub version: String,
    /// 描述
    pub description: String,
    /// 状态
    pub status: AlgorithmStatus,
}

/// 算法特征
#[derive(Debug, Clone, Default)]
pub struct AlgorithmTraits {
    /// 是否支持增量计算
    pub supports_incremental: bool,
    /// 是否支持分布式执行
    pub supports_distributed: bool,
    /// 是否支持 GPU 加速
    pub supports_gpu: bool,
    /// 是否确定性算法
    pub is_deterministic: bool,
    /// 时间复杂度描述
    pub time_complexity: Option<String>,
    /// 空间复杂度描述
    pub space_complexity: Option<String>,
}

use crate::unified_model::UnifiedData;
use crate::compute_engine::ComputeEngine;
use std::sync::Arc;

/// 算法 trait — 所有算法必须实现
#[async_trait::async_trait]
pub trait Algorithm: Send + Sync {
    /// 算法 ID
    fn id(&self) -> &str;

    /// 算法名称
    fn name(&self) -> &str;

    /// 算法类别
    fn category(&self) -> AlgorithmCategory;

    /// 版本号
    fn version(&self) -> &str;

    /// 描述
    fn description(&self) -> &str;

    /// 状态
    fn status(&self) -> AlgorithmStatus {
        AlgorithmStatus::Active
    }

    /// 算法特征
    fn traits(&self) -> AlgorithmTraits {
        AlgorithmTraits::default()
    }

    /// 输入规格
    fn input_spec(&self) -> Vec<DataShape>;

    /// 输出规格
    fn output_spec(&self) -> Vec<DataShape>;

    /// 参数规格
    fn param_specs(&self) -> Vec<ParamSpec>;

    /// 支持的计算模型
    fn supported_compute_models(&self) -> Vec<ComputeModel> {
        vec![ComputeModel::SingleThread]
    }

    /// 执行算法
    async fn execute(
        &self,
        input: UnifiedData,
        params: IndexMap<String, ParamValue>,
        compute_engine: Arc<ComputeEngine>,
    ) -> crate::error::AlgoResult<UnifiedData>;

    /// 获取算法基本信息
    fn info(&self) -> AlgorithmInfo {
        AlgorithmInfo {
            id: self.id().to_string(),
            name: self.name().to_string(),
            category: self.category(),
            version: self.version().to_string(),
            description: self.description().to_string(),
            status: self.status(),
        }
    }
}
