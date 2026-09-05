//! 审计日志敏感数据脱敏模块
//!
//! 对存储在审计日志中的请求参数进行敏感信息脱敏，防止密码、身份证、
//! 手机号、邮箱、Token 等敏感数据明文落盘。
//!
//! 脱敏策略：
//! 1. 字段名黑名单匹配（不区分大小写）：直接替换为 "***"
//! 2. 正则模式匹配：对值进行部分掩码（手机号保留前3后4，邮箱保留首字符和域名）

use serde_json::Value;
use std::collections::HashSet;

/// 敏感字段名黑名单（不区分大小写匹配）
const SENSITIVE_FIELD_NAMES: &[&str] = &[
    "password",
    "pwd",
    "passwd",
    "secret",
    "token",
    "access_token",
    "refresh_token",
    "api_key",
    "apikey",
    "private_key",
    "id_card",
    "idcard",
    "identity_card",
    "credit_card",
    "card_number",
    "cvv",
    "cvc",
    "ssn",
    "bank_card",
    "bankcard",
    "authorization",
    "cookie",
    "session",
    "session_id",
    "otp",
    "verification_code",
    "verify_code",
    "sms_code",
];

/// 脱敏器
pub struct SensitiveMasker {
    /// 自定义敏感字段名集合
    custom_fields: HashSet<String>,
}

impl Default for SensitiveMasker {
    fn default() -> Self {
        Self::new()
    }
}

impl SensitiveMasker {
    /// 创建脱敏器
    pub fn new() -> Self {
        Self {
            custom_fields: HashSet::new(),
        }
    }

    /// 添加自定义敏感字段名
    pub fn with_custom_field(mut self, field: impl Into<String>) -> Self {
        self.custom_fields.insert(field.into().to_lowercase());
        self
    }

    /// 批量添加自定义敏感字段名
    pub fn with_custom_fields(mut self, fields: &[&str]) -> Self {
        for f in fields {
            self.custom_fields.insert(f.to_lowercase());
        }
        self
    }

    /// 判断字段名是否敏感
    fn is_sensitive_field(&self, field: &str) -> bool {
        let lower = field.to_lowercase();
        if self.custom_fields.contains(&lower) {
            return true;
        }
        SENSITIVE_FIELD_NAMES.iter().any(|name| {
            // 精确匹配或包含匹配（如 user_password 匹配 password）
            lower == *name || lower.contains(name)
        })
    }

    /// 对 JSON 值进行递归脱敏
    pub fn mask_json(&self, value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut masked = serde_json::Map::new();
                for (key, val) in map {
                    if self.is_sensitive_field(key) {
                        masked.insert(key.clone(), Value::String("***".to_string()));
                    } else {
                        masked.insert(key.clone(), self.mask_json(val));
                    }
                }
                Value::Object(masked)
            }
            Value::Array(arr) => {
                Value::Array(arr.iter().map(|v| self.mask_json(v)).collect())
            }
            Value::String(s) => Value::String(self.mask_string(s)),
            other => other.clone(),
        }
    }

    /// 对字符串值进行正则模式脱敏
    fn mask_string(&self, s: &str) -> String {
        // 身份证脱敏（18位，保留前6后4）—— 必须在手机号之前，避免手机号正则匹配身份证中的数字段
        let idcard_re = regex::Regex::new(r"[1-9]\d{5}(19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]").unwrap();
        let s = idcard_re.replace_all(s, |caps: &regex::Captures| {
            let id = &caps[0];
            format!("{}********{}", &id[..6], &id[14..])
        }).to_string();

        // 手机号脱敏（11位，保留前3后4）
        let phone_re = regex::Regex::new(r"1[3-9]\d{9}").unwrap();
        let s = phone_re.replace_all(&s, |caps: &regex::Captures| {
            let phone = &caps[0];
            format!("{}****{}", &phone[..3], &phone[7..])
        }).to_string();

        // 邮箱脱敏（保留首字符和@后域名）
        let email_re = regex::Regex::new(r"([a-zA-Z0-9._%+-]+)@([a-zA-Z0-9.-]+\.[a-zA-Z]{2,})").unwrap();
        let s = email_re.replace_all(&s, |caps: &regex::Captures| {
            let user = &caps[1];
            let domain = &caps[2];
            if user.len() <= 1 {
                format!("*@{}", domain)
            } else {
                format!("{}***@{}", &user[..1], domain)
            }
        }).to_string();

        // 银行卡号脱敏（16-19位，保留前4后4）
        let bankcard_re = regex::Regex::new(r"\b\d{16,19}\b").unwrap();
        let s = bankcard_re.replace_all(&s, |caps: &regex::Captures| {
            let card = &caps[0];
            let len = card.len();
            format!("{}********{}", &card[..4], &card[len-4..])
        }).to_string();

        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_password_field_masking() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "username": "alice",
            "password": "secret123",
            "profile": {
                "age": 30,
                "pwd": "another_secret"
            }
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["username"], "alice");
        assert_eq!(masked["password"], "***");
        assert_eq!(masked["profile"]["age"], 30);
        assert_eq!(masked["profile"]["pwd"], "***");
    }

    #[test]
    fn test_token_field_masking() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            "refresh_token": "refresh_secret_token",
            "api_key": "sk-1234567890"
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["access_token"], "***");
        assert_eq!(masked["refresh_token"], "***");
        assert_eq!(masked["api_key"], "***");
    }

    #[test]
    fn test_phone_number_masking() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "phone": "13812345678",
            "contact": "我的电话是13987654321请联系"
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["phone"], "138****5678");
        assert!(masked["contact"].as_str().unwrap().contains("139****4321"));
    }

    #[test]
    fn test_id_card_masking() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "id_card": "110101199001011234"
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["id_card"], "110101********1234");
    }

    #[test]
    fn test_email_masking() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "email": "alice@example.com",
            "short": "a@test.com"
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["email"], "a***@example.com");
        assert_eq!(masked["short"], "*@test.com");
    }

    #[test]
    fn test_bank_card_masking() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "bank_card": "6222021234567890123"
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["bank_card"], "6222********0123");
    }

    #[test]
    fn test_nested_array_masking() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "users": [
                {"name": "alice", "password": "pass1"},
                {"name": "bob", "password": "pass2"}
            ]
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["users"][0]["password"], "***");
        assert_eq!(masked["users"][1]["password"], "***");
        assert_eq!(masked["users"][0]["name"], "alice");
    }

    #[test]
    fn test_custom_sensitive_field() {
        let masker = SensitiveMasker::new()
            .with_custom_field("my_secret_field")
            .with_custom_fields(&["internal_key", "confidential_data"]);
        let input = json!({
            "my_secret_field": "secret",
            "internal_key": "key123",
            "confidential_data": "data",
            "normal_field": "normal"
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["my_secret_field"], "***");
        assert_eq!(masked["internal_key"], "***");
        assert_eq!(masked["confidential_data"], "***");
        assert_eq!(masked["normal_field"], "normal");
    }

    #[test]
    fn test_non_sensitive_data_preserved() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "user_id": 12345,
            "username": "alice",
            "age": 30,
            "is_active": true,
            "roles": ["admin", "user"],
            "metadata": null
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["user_id"], 12345);
        assert_eq!(masked["username"], "alice");
        assert_eq!(masked["age"], 30);
        assert_eq!(masked["is_active"], true);
        assert_eq!(masked["roles"][0], "admin");
        assert!(masked["metadata"].is_null());
    }

    #[test]
    fn test_field_name_case_insensitive() {
        let masker = SensitiveMasker::new();
        let input = json!({
            "Password": "secret",
            "PWD": "secret2",
            "AccessToken": "token"
        });
        let masked = masker.mask_json(&input);
        assert_eq!(masked["Password"], "***");
        assert_eq!(masked["PWD"], "***");
        assert_eq!(masked["AccessToken"], "***");
    }
}
