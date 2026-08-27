//! 数据脱敏Trait — Data Masker
//!
//! 企业级数据脱敏抽象，可替换脱敏算法：
//! 内置实现支持手机号/身份证/邮箱/银行卡/姓名/地址等。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 脱敏级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaskLevel {
    /// 不脱敏
    None,
    /// 部分脱敏（保留首尾）
    Partial,
    /// 完全脱敏（全部替换）
    Full,
    /// 哈希脱敏（不可逆）
    Hash,
}

impl MaskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            MaskLevel::None => "none",
            MaskLevel::Partial => "partial",
            MaskLevel::Full => "full",
            MaskLevel::Hash => "hash",
        }
    }
}

/// 脱敏结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskResult {
    /// 脱敏后的值
    pub masked_value: String,
    /// 原始值的哈希（用于验证，可选）
    #[serde(default)]
    pub original_hash: Option<String>,
    /// 使用的脱敏级别
    pub level: MaskLevel,
    /// 是否被脱敏
    pub masked: bool,
}

/// 数据脱敏器Trait
#[async_trait]
pub trait DataMasker: Send + Sync {
    /// 脱敏字符串值
    async fn mask(&self, value: &str, field_type: &str, level: MaskLevel) -> MaskResult;

    /// 脱敏JSON值（递归处理敏感字段）
    async fn mask_json(&self, value: &serde_json::Value, sensitive_fields: &[String], level: MaskLevel) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut result = serde_json::Map::new();
                for (k, v) in map {
                    if sensitive_fields.iter().any(|f| f == k || k.contains(f)) {
                        if let Some(s) = v.as_str() {
                            let masked = self.mask(s, k, level).await;
                            result.insert(k.clone(), serde_json::Value::String(masked.masked_value));
                        } else {
                            result.insert(k.clone(), v.clone());
                        }
                    } else {
                        result.insert(k.clone(), self.mask_json(v, sensitive_fields, level).await);
                    }
                }
                serde_json::Value::Object(result)
            }
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for v in arr {
                    result.push(self.mask_json(v, sensitive_fields, level).await);
                }
                serde_json::Value::Array(result)
            }
            _ => value.clone(),
        }
    }

    /// 检查字段是否需要脱敏
    fn is_sensitive_field(&self, field_name: &str) -> bool {
        let sensitive = ["phone", "mobile", "id_card", "idcard", "email", "mail",
            "bank_card", "bankcard", "password", "secret", "token", "api_key",
            "address", "addr", "name", "real_name", "id_number"];
        let lower = field_name.to_lowercase();
        sensitive.iter().any(|s| lower.contains(s))
    }

    /// 获取支持的字段类型列表
    fn supported_field_types(&self) -> Vec<String> {
        vec!["phone".into(), "id_card".into(), "email".into(), "bank_card".into(),
             "name".into(), "address".into(), "default".into()]
    }
}

/// 便捷：创建脱敏结果
pub fn mask_result(value: impl Into<String>, level: MaskLevel, masked: bool) -> MaskResult {
    MaskResult {
        masked_value: value.into(),
        original_hash: None,
        level,
        masked,
    }
}
