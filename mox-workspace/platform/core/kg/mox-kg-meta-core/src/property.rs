//! 属性值操作工具

use crate::types::PropertyValue;

impl PropertyValue {
    /// 转为字符串
    pub fn as_string(&self) -> Option<&str> {
        match self {
            PropertyValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转为整数
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            PropertyValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// 转为浮点数
    pub fn as_float(&self) -> Option<f64> {
        match self {
            PropertyValue::Float(f) => Some(*f),
            PropertyValue::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// 转为布尔值
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            PropertyValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// 是否为空
    pub fn is_null(&self) -> bool {
        matches!(self, PropertyValue::Null)
    }
}

impl From<String> for PropertyValue {
    fn from(v: String) -> Self { PropertyValue::String(v) }
}

impl From<&str> for PropertyValue {
    fn from(v: &str) -> Self { PropertyValue::String(v.to_string()) }
}

impl From<i64> for PropertyValue {
    fn from(v: i64) -> Self { PropertyValue::Integer(v) }
}

impl From<f64> for PropertyValue {
    fn from(v: f64) -> Self { PropertyValue::Float(v) }
}

impl From<bool> for PropertyValue {
    fn from(v: bool) -> Self { PropertyValue::Boolean(v) }
}
