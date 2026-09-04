// =============================================================================
// 配置验证器（Config Validator）
// =============================================================================

use crate::config::{Config, ConfigValue};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// 验证结果
// =============================================================================

/// 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// 是否通过
    pub valid: bool,
    /// 验证错误列表
    pub errors: Vec<ValidationError>,
    /// 验证警告列表
    pub warnings: Vec<ValidationError>,
    /// 验证耗时（毫秒）
    pub duration_ms: u64,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: vec![],
            warnings: vec![],
            duration_ms: 0,
        }
    }

    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            warnings: vec![],
            duration_ms: 0,
        }
    }

    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: ValidationError) {
        self.warnings.push(warning);
    }
}

/// 验证错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// 配置键
    pub key: String,
    /// 错误消息
    pub message: String,
    /// 错误级别
    pub level: ValidationLevel,
}

/// 验证级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationLevel {
    /// 错误（必须修复）
    Error,
    /// 警告（建议修复）
    Warning,
    /// 信息
    Info,
}

// =============================================================================
// 配置规则
// =============================================================================

/// 配置验证规则
#[derive(Debug, Clone)]
pub struct ValidationRule {
    /// 配置键（支持通配符 *）
    pub key: String,
    /// 是否必填
    pub required: bool,
    /// 期望类型
    pub expected_type: Option<&'static str>,
    /// 最小值（数值类型）
    pub min: Option<f64>,
    /// 最大值（数值类型）
    pub max: Option<f64>,
    /// 枚举值（字符串类型）
    pub enum_values: Option<Vec<String>>,
    /// 自定义验证函数
    pub custom_validator: Option<fn(&ConfigValue) -> Result<(), String>>,
}

impl ValidationRule {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            required: false,
            expected_type: None,
            min: None,
            max: None,
            enum_values: None,
            custom_validator: None,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn with_type(mut self, ty: &'static str) -> Self {
        self.expected_type = Some(ty);
        self
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn with_enum(mut self, values: Vec<String>) -> Self {
        self.enum_values = Some(values);
        self
    }
}

// =============================================================================
// 配置验证器
// =============================================================================

/// 配置验证器
pub struct ConfigValidator {
    rules: Vec<ValidationRule>,
}

impl ConfigValidator {
    pub fn new() -> Self {
        Self { rules: vec![] }
    }

    pub fn add_rule(mut self, rule: ValidationRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn add_rules(mut self, rules: Vec<ValidationRule>) -> Self {
        self.rules.extend(rules);
        self
    }

    /// 验证配置
    pub fn validate(&self, config: &Config) -> ValidationResult {
        let start = std::time::Instant::now();
        let mut result = ValidationResult::success();

        for rule in &self.rules {
            let value = config.get(&rule.key);

            // 必填检查
            if rule.required && value.is_none() {
                result.add_error(ValidationError {
                    key: rule.key.clone(),
                    message: format!("配置项 '{}' 是必填的", rule.key),
                    level: ValidationLevel::Error,
                });
                continue;
            }

            // 非必填且不存在，跳过
            let Some(value) = value else { continue };

            // 类型检查
            if let Some(expected_type) = rule.expected_type {
                if value.type_name() != expected_type {
                    result.add_error(ValidationError {
                        key: rule.key.clone(),
                        message: format!(
                            "配置项 '{}' 类型不匹配：期望 '{}'，实际 '{}'",
                            rule.key,
                            expected_type,
                            value.type_name()
                        ),
                        level: ValidationLevel::Error,
                    });
                    continue;
                }
            }

            // 范围检查
            if let (Some(min), Some(max)) = (rule.min, rule.max) {
                if let Some(num) = value.as_f64() {
                    if num < min || num > max {
                        result.add_error(ValidationError {
                            key: rule.key.clone(),
                            message: format!(
                                "配置项 '{}' 值 {} 超出范围 [{}, {}]",
                                rule.key, num, min, max
                            ),
                            level: ValidationLevel::Error,
                        });
                    }
                }
            }

            // 枚举检查
            if let Some(enum_values) = &rule.enum_values {
                if let Some(s) = value.as_str() {
                    if !enum_values.contains(&s.to_string()) {
                        result.add_error(ValidationError {
                            key: rule.key.clone(),
                            message: format!(
                                "配置项 '{}' 值 '{}' 不在允许的枚举值中: {:?}",
                                rule.key, s, enum_values
                            ),
                            level: ValidationLevel::Error,
                        });
                    }
                }
            }

            // 自定义验证
            if let Some(validator) = rule.custom_validator {
                if let Err(msg) = validator(value) {
                    result.add_error(ValidationError {
                        key: rule.key.clone(),
                        message: format!("配置项 '{}' 验证失败: {}", rule.key, msg),
                        level: ValidationLevel::Error,
                    });
                }
            }
        }

        result.duration_ms = start.elapsed().as_millis() as u64;
        result
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_required() {
        let validator = ConfigValidator::new()
            .add_rule(ValidationRule::new("required.key").required());

        let config = Config::new();
        let result = validator.validate(&config);

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].key, "required.key");
    }

    #[test]
    fn validator_type_check() {
        let validator = ConfigValidator::new()
            .add_rule(ValidationRule::new("port").with_type("integer"));

        let mut config = Config::new();
        config.set("port", ConfigValue::from("not-a-number"));

        let result = validator.validate(&config);
        assert!(!result.valid);
        assert!(result.errors[0].message.contains("类型不匹配"));
    }

    #[test]
    fn validator_range_check() {
        let validator = ConfigValidator::new()
            .add_rule(ValidationRule::new("port").with_range(1.0, 65535.0));

        let mut config = Config::new();
        config.set("port", ConfigValue::from(70000i64));

        let result = validator.validate(&config);
        assert!(!result.valid);
        assert!(result.errors[0].message.contains("超出范围"));
    }

    #[test]
    fn validator_enum_check() {
        let validator = ConfigValidator::new()
            .add_rule(ValidationRule::new("env").with_enum(vec!["dev".to_string(), "prod".to_string()]));

        let mut config = Config::new();
        config.set("env", ConfigValue::from("staging"));

        let result = validator.validate(&config);
        assert!(!result.valid);
        assert!(result.errors[0].message.contains("不在允许的枚举值中"));
    }

    #[test]
    fn validator_success() {
        let validator = ConfigValidator::new()
            .add_rule(ValidationRule::new("port").required().with_type("integer").with_range(1.0, 65535.0));

        let mut config = Config::new();
        config.set("port", ConfigValue::from(8080i64));

        let result = validator.validate(&config);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }
}
