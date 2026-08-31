//! ETL 流水线定义
//!
//! 定义数据抽取、转换、加载的完整流水线

use serde::{Deserialize, Serialize};

/// 流水线状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    /// 未运行
    Idle,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 流水线配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// 流水线 ID
    pub id: String,
    /// 流水线名称
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 数据源 ID
    pub source_id: String,
    /// 目标图谱 ID
    pub target_graph_id: String,
    /// 映射规则
    pub mappings: Vec<MappingRule>,
    /// 调度配置
    pub schedule: Option<ScheduleConfig>,
}

/// 映射规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MappingRule {
    /// 规则 ID
    pub id: String,
    /// 源字段
    pub source_field: String,
    /// 目标属性
    pub target_property: String,
    /// 转换表达式
    pub transform: Option<String>,
    /// 映射类型
    pub rule_type: MappingType,
}

/// 映射类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingType {
    /// 直接映射
    Direct,
    /// 节点映射
    Node,
    /// 边映射
    Edge,
    /// 属性映射
    Property,
}

/// 调度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Cron 表达式
    pub cron_expression: String,
    /// 是否启用
    pub enabled: bool,
}

/// 流水线执行统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    /// 抽取记录数
    pub extracted: usize,
    /// 转换记录数
    pub transformed: usize,
    /// 加载记录数
    pub loaded: usize,
    /// 失败记录数
    pub failed: usize,
    /// 开始时间（毫秒时间戳）
    pub start_time: Option<i64>,
    /// 结束时间（毫秒时间戳）
    pub end_time: Option<i64>,
}
