//! OA/ERP 集成适配器核心模型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 连接器类型（支持的 OA/ERP 系统）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorType {
    /// SAP ERP
    Sap,
    /// Oracle ERP / Oracle HCM
    Oracle,
    /// Workday HCM
    Workday,
    /// 北森 HR
    Beisen,
    /// 用友 NC / U8
    Yonyou,
    /// 金蝶 EAS / K3
    Kingdee,
    /// 钉钉
    DingTalk,
    /// 企业微信
    WeCom,
    /// 飞书
    Feishu,
    /// 通用 REST API
    GenericRest,
    /// 通用数据库直连
    GenericJdbc,
}

impl ConnectorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sap => "sap",
            Self::Oracle => "oracle",
            Self::Workday => "workday",
            Self::Beisen => "beisen",
            Self::Yonyou => "yonyou",
            Self::Kingdee => "kingdee",
            Self::DingTalk => "dingtalk",
            Self::WeCom => "wecom",
            Self::Feishu => "feishu",
            Self::GenericRest => "generic_rest",
            Self::GenericJdbc => "generic_jdbc",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Sap => "SAP ERP",
            Self::Oracle => "Oracle ERP/HCM",
            Self::Workday => "Workday HCM",
            Self::Beisen => "北森 HR",
            Self::Yonyou => "用友 NC/U8",
            Self::Kingdee => "金蝶 EAS/K3",
            Self::DingTalk => "钉钉",
            Self::WeCom => "企业微信",
            Self::Feishu => "飞书",
            Self::GenericRest => "通用 REST API",
            Self::GenericJdbc => "通用数据库直连",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::Sap | Self::Oracle | Self::Yonyou | Self::Kingdee => "ERP",
            Self::Workday | Self::Beisen => "HCM",
            Self::DingTalk | Self::WeCom | Self::Feishu => "协同办公",
            Self::GenericRest | Self::GenericJdbc => "通用",
        }
    }
}

/// 连接器状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorStatus {
    /// 草稿
    Draft,
    /// 已启用
    Active,
    /// 已停用
    Inactive,
    /// 连接失败
    Error,
    /// 测试中
    Testing,
}

/// 同步方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    /// 从 OA/ERP 同步到 MOX（拉取）
    Pull,
    /// 从 MOX 同步到 OA/ERP（推送）
    Push,
    /// 双向同步
    Bidirectional,
}

/// 同步任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncTaskStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 部分成功
    PartialSuccess,
    /// 已取消
    Cancelled,
}

/// 连接器配置（存储在数据库中）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorConfig {
    /// 连接器 ID
    pub connector_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 连接器名称
    pub name: String,
    /// 连接器类型
    pub connector_type: ConnectorType,
    /// 连接器状态
    pub status: ConnectorStatus,
    /// 连接配置（URL/账号/密码/API Key 等，加密存储）
    pub connection: HashMap<String, String>,
    /// 认证配置（OAuth2/API Key/用户名密码 等）
    pub auth: HashMap<String, String>,
    /// 字段映射配置（源字段 -> 目标字段）
    pub field_mappings: Vec<FieldMapping>,
    /// 同步配置（增量/全量/定时）
    pub sync_config: SyncConfig,
    /// 描述
    pub description: Option<String>,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
    /// 最后测试连接时间
    pub last_test_at: Option<String>,
    /// 最后同步时间
    pub last_sync_at: Option<String>,
}

/// 字段映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    /// 源字段名（OA/ERP 侧）
    pub source_field: String,
    /// 目标字段名（MOX 侧）
    pub target_field: String,
    /// 字段类型
    pub field_type: String,
    /// 是否必填
    pub required: bool,
    /// 默认值
    pub default_value: Option<String>,
    /// 转换表达式（可选，用于值转换）
    pub transform_expr: Option<String>,
    /// 描述
    pub description: Option<String>,
}

/// 同步配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// 同步方向
    pub direction: SyncDirection,
    /// 同步模式：full（全量）/ incremental（增量）
    pub mode: String,
    /// 同步频率：manual（手动）/ cron（定时）/ event（事件触发）
    pub frequency: String,
    /// Cron 表达式（定时同步时使用）
    pub cron_expression: Option<String>,
    /// 增量字段名（增量同步时使用，如 updated_at）
    pub incremental_field: Option<String>,
    /// 批量大小
    pub batch_size: i32,
    /// 失败重试次数
    pub retry_count: i32,
    /// 失败重试间隔（秒）
    pub retry_interval: i32,
    /// 冲突解决策略：source_wins / target_wins / manual
    pub conflict_strategy: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            direction: SyncDirection::Pull,
            mode: "incremental".to_string(),
            frequency: "manual".to_string(),
            cron_expression: None,
            incremental_field: Some("updated_at".to_string()),
            batch_size: 100,
            retry_count: 3,
            retry_interval: 60,
            conflict_strategy: "source_wins".to_string(),
        }
    }
}

/// 同步任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    /// 任务 ID
    pub task_id: String,
    /// 连接器 ID
    pub connector_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 任务名称
    pub name: String,
    /// 同步实体：organization / user / department / role / 自定义
    pub entity: String,
    /// 任务状态
    pub status: SyncTaskStatus,
    /// 同步方向
    pub direction: SyncDirection,
    /// 同步模式
    pub mode: String,
    /// 触发方式：manual / cron / event / api
    pub trigger_type: String,
    /// 开始时间
    pub started_at: Option<String>,
    /// 结束时间
    pub finished_at: Option<String>,
    /// 总记录数
    pub total_count: i64,
    /// 成功数
    pub success_count: i64,
    /// 失败数
    pub failed_count: i64,
    /// 跳过数
    pub skipped_count: i64,
    /// 错误信息
    pub error_message: Option<String>,
    /// 详细日志（存储在文件或数据库中）
    pub log_path: Option<String>,
    /// 创建人
    pub created_by: Option<String>,
    /// 创建时间
    pub created_at: String,
}

/// 同步日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncLogRecord {
    /// 日志 ID
    pub log_id: String,
    /// 任务 ID
    pub task_id: String,
    /// 连接器 ID
    pub connector_id: String,
    /// 实体类型
    pub entity: String,
    /// 源记录 ID
    pub source_id: String,
    /// 目标记录 ID
    pub target_id: Option<String>,
    /// 操作类型：create / update / delete / skip / error
    pub operation: String,
    /// 状态：success / failed / skipped
    pub status: String,
    /// 错误信息
    pub error_message: Option<String>,
    /// 原始数据（JSON）
    pub raw_data: Option<String>,
    /// 时间戳
    pub timestamp: String,
}

/// 连接器测试结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorTestResult {
    /// 是否成功
    pub success: bool,
    /// 响应时间（毫秒）
    pub response_time_ms: i64,
    /// 消息
    pub message: String,
    /// 详细信息
    pub details: Option<HashMap<String, String>>,
}

/// 创建连接器请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConnectorRequest {
    pub name: String,
    pub connector_type: ConnectorType,
    pub connection: HashMap<String, String>,
    pub auth: HashMap<String, String>,
    pub field_mappings: Option<Vec<FieldMapping>>,
    pub sync_config: Option<SyncConfig>,
    pub description: Option<String>,
}

/// 更新连接器请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConnectorRequest {
    pub name: Option<String>,
    pub status: Option<ConnectorStatus>,
    pub connection: Option<HashMap<String, String>>,
    pub auth: Option<HashMap<String, String>>,
    pub field_mappings: Option<Vec<FieldMapping>>,
    pub sync_config: Option<SyncConfig>,
    pub description: Option<String>,
}

/// 触发同步请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSyncRequest {
    /// 同步实体
    pub entity: String,
    /// 同步模式：full / incremental
    pub mode: Option<String>,
    /// 同步方向
    pub direction: Option<SyncDirection>,
    /// 自定义参数
    pub params: Option<HashMap<String, String>>,
}

/// 支持的连接器类型列表（用于前端展示）
pub fn supported_connector_types() -> Vec<ConnectorType> {
    vec![
        ConnectorType::Sap,
        ConnectorType::Oracle,
        ConnectorType::Workday,
        ConnectorType::Beisen,
        ConnectorType::Yonyou,
        ConnectorType::Kingdee,
        ConnectorType::DingTalk,
        ConnectorType::WeCom,
        ConnectorType::Feishu,
        ConnectorType::GenericRest,
        ConnectorType::GenericJdbc,
    ]
}
