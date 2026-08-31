//! 数据归一化类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 数据值类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DataValue {
    /// 字符串
    String(String),
    /// 整数
    Integer(i64),
    /// 浮点数
    Float(f64),
    /// 布尔值
    Boolean(bool),
    /// 日期时间（Unix 毫秒时间戳）
    DateTime(i64),
    /// 列表
    List(Vec<DataValue>),
    /// 空值
    Null,
}

/// 数据行
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataRow {
    /// 字段值映射
    pub fields: HashMap<String, DataValue>,
}

impl DataRow {
    /// 创建空行
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置字段值
    pub fn set(&mut self, field: impl Into<String>, value: DataValue) {
        self.fields.insert(field.into(), value);
    }

    /// 获取字段值
    pub fn get(&self, field: &str) -> Option<&DataValue> {
        self.fields.get(field)
    }

    /// 检查字段是否存在且非空
    pub fn has(&self, field: &str) -> bool {
        self.fields.get(field).map(|v| !matches!(v, DataValue::Null)).unwrap_or(false)
    }
}

/// 数据集
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DataSet {
    /// 数据行列表
    pub rows: Vec<DataRow>,
    /// 字段名列表（有序）
    pub schema: Vec<String>,
}

impl DataSet {
    /// 创建空数据集
    pub fn new() -> Self {
        Self::default()
    }

    /// 行数
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// 字段数
    pub fn field_count(&self) -> usize {
        self.schema.len()
    }
}
