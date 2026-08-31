//! 数据源连接器抽象
//!
//! 定义统一的数据源接入接口，支持多种数据源类型

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::HubResult;

/// 数据源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSourceType {
    /// 关系型数据库
    RelationalDatabase,
    /// NoSQL 数据库
    NosqlDatabase,
    /// CSV 文件
    CsvFile,
    /// JSON 文件
    JsonFile,
    /// API 接口
    ApiEndpoint,
    /// Excel 文件
    ExcelFile,
    /// 消息队列
    MessageQueue,
}

/// 数据源配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceConfig {
    /// 数据源 ID
    pub id: String,
    /// 数据源名称
    pub name: String,
    /// 数据源类型
    pub source_type: DataSourceType,
    /// 连接字符串 / URL
    pub connection_string: String,
    /// 额外配置（JSON）
    pub options: Option<serde_json::Value>,
}

/// 数据记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRecord {
    /// 记录 ID
    pub id: String,
    /// 字段值
    pub fields: std::collections::HashMap<String, serde_json::Value>,
}

/// 数据源连接器接口
#[async_trait]
pub trait DataConnector: Send + Sync {
    /// 连接器名称
    fn name(&self) -> &str;

    /// 数据源类型
    fn source_type(&self) -> DataSourceType;

    /// 测试连接
    async fn test_connection(&self, config: &DataSourceConfig) -> HubResult<bool>;

    /// 抽取数据
    async fn extract(&self, config: &DataSourceConfig, query: &str) -> HubResult<Vec<DataRecord>>;

    /// 流式抽取数据
    async fn extract_stream(
        &self,
        config: &DataSourceConfig,
        query: &str,
        batch_size: usize,
    ) -> HubResult<Vec<DataRecord>>;

    /// 获取表/集合列表
    async fn list_tables(&self, config: &DataSourceConfig) -> HubResult<Vec<String>>;

    /// 获取字段信息
    async fn get_schema(&self, config: &DataSourceConfig, table: &str) -> HubResult<Vec<FieldInfo>>;
}

/// 字段信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    /// 字段名
    pub name: String,
    /// 字段类型
    pub data_type: String,
    /// 是否可为空
    pub nullable: bool,
    /// 描述
    pub description: Option<String>,
}
