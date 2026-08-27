//! 数据脱敏 — 敏感数据脱敏处理
//!
//! 支持：手机号、身份证、邮箱、银行卡、姓名、地址、自定义正则

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 脱敏级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaskLevel {
    /// 不脱敏
    None,
    /// 轻度脱敏（保留首尾）
    Low,
    /// 中度脱敏（保留首/尾各1位）
    Medium,
    /// 高度脱敏（全部替换）
    High,
    /// 自定义（按规则）
    Custom,
}

impl Default for MaskLevel {
    fn default() -> Self { MaskLevel::Medium }
}

/// 数据脱敏器
pub struct DataMasker {
    level: MaskLevel,
    mask_char: char,
}

impl DataMasker {
    pub fn new(level: MaskLevel) -> Self {
        Self { level, mask_char: '*' }
    }

    pub fn with_mask_char(mut self, c: char) -> Self {
        self.mask_char = c;
        self
    }

    /// 脱敏手机号（11位：138****1234）
    pub fn mask_phone(&self, phone: &str) -> String {
        match self.level {
            MaskLevel::None => phone.to_string(),
            MaskLevel::Low => {
                if phone.len() >= 7 {
                    format!("{}{}{}", &phone[..3], "*".repeat(phone.len() - 7), &phone[phone.len()-4..])
                } else { self.mask_all(phone) }
            }
            MaskLevel::Medium => {
                if phone.len() >= 7 {
                    format!("{}****{}", &phone[..3], &phone[phone.len()-4..])
                } else { self.mask_all(phone) }
            }
            MaskLevel::High => self.mask_all(phone),
            MaskLevel::Custom => self.mask_custom(phone),
        }
    }

    /// 脱敏身份证号（18位：110***********1234）
    pub fn mask_id_card(&self, id: &str) -> String {
        match self.level {
            MaskLevel::None => id.to_string(),
            MaskLevel::Low | MaskLevel::Medium => {
                if id.len() >= 10 {
                    format!("{}{}{}", &id[..3], "*".repeat(id.len() - 7), &id[id.len()-4..])
                } else { self.mask_all(id) }
            }
            MaskLevel::High => self.mask_all(id),
            MaskLevel::Custom => self.mask_custom(id),
        }
    }

    /// 脱敏邮箱（a***@example.com）
    pub fn mask_email(&self, email: &str) -> String {
        match self.level {
            MaskLevel::None => email.to_string(),
            MaskLevel::Low | MaskLevel::Medium => {
                if let Some((local, domain)) = email.split_once('@') {
                    if local.len() <= 2 {
                        format!("{}***@{}", &local[..1.min(local.len())], domain)
                    } else {
                        format!("{}***@{}", &local[..1], domain)
                    }
                } else { self.mask_all(email) }
            }
            MaskLevel::High => {
                if let Some((_, domain)) = email.split_once('@') {
                    format!("***@{}", domain)
                } else { self.mask_all(email) }
            }
            MaskLevel::Custom => self.mask_custom(email),
        }
    }

    /// 脱敏银行卡号（16-19位：6222 **** **** 1234）
    pub fn mask_bank_card(&self, card: &str) -> String {
        match self.level {
            MaskLevel::None => card.to_string(),
            MaskLevel::Low | MaskLevel::Medium => {
                let digits: String = card.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() >= 8 {
                    format!("{} **** **** {}", &digits[..4], &digits[digits.len()-4..])
                } else { self.mask_all(card) }
            }
            MaskLevel::High => self.mask_all(card),
            MaskLevel::Custom => self.mask_custom(card),
        }
    }

    /// 脱敏姓名（张*、张*明）
    pub fn mask_name(&self, name: &str) -> String {
        match self.level {
            MaskLevel::None => name.to_string(),
            MaskLevel::Low | MaskLevel::Medium => {
                let chars: Vec<char> = name.chars().collect();
                match chars.len() {
                    0 => String::new(),
                    1 => name.to_string(),
                    2 => format!("{}{}", chars[0], self.mask_char),
                    _ => {
                        let middle: String = chars[1..chars.len()-1].iter().map(|_| self.mask_char).collect();
                        format!("{}{}{}", chars[0], middle, chars[chars.len()-1])
                    }
                }
            }
            MaskLevel::High => self.mask_all(name),
            MaskLevel::Custom => self.mask_custom(name),
        }
    }

    /// 脱敏地址（保留省市，隐藏详细地址）
    pub fn mask_address(&self, address: &str) -> String {
        match self.level {
            MaskLevel::None => address.to_string(),
            MaskLevel::Low | MaskLevel::Medium => {
                // 简化：保留前10个字符
                if address.chars().count() > 10 {
                    format!("{}***", address.chars().take(10).collect::<String>())
                } else { address.to_string() }
            }
            MaskLevel::High => self.mask_all(address),
            MaskLevel::Custom => self.mask_custom(address),
        }
    }

    /// 全部替换为掩码字符
    fn mask_all(&self, s: &str) -> String {
        self.mask_char.to_string().repeat(s.chars().count())
    }

    /// 自定义脱敏（按正则规则，简化实现）
    fn mask_custom(&self, s: &str) -> String {
        // 自定义规则：保留首尾各1位
        let chars: Vec<char> = s.chars().collect();
        if chars.len() <= 2 { return self.mask_all(s); }
        let middle: String = chars[1..chars.len()-1].iter().map(|_| self.mask_char).collect();
        format!("{}{}{}", chars[0], middle, chars[chars.len()-1])
    }

    /// 自动识别并脱敏（根据内容模式）
    pub fn auto_mask(&self, text: &str) -> String {
        static PHONE_RE: OnceLock<Regex> = OnceLock::new();
        static ID_RE: OnceLock<Regex> = OnceLock::new();
        static EMAIL_RE: OnceLock<Regex> = OnceLock::new();

        let phone_re = PHONE_RE.get_or_init(|| Regex::new(r"1[3-9]\d{9}").unwrap());
        let id_re = ID_RE.get_or_init(|| Regex::new(r"\d{17}[\dXx]").unwrap());
        let email_re = EMAIL_RE.get_or_init(|| Regex::new(r"[\w.-]+@[\w.-]+\.\w+").unwrap());

        let mut result = text.to_string();
        result = email_re.replace_all(&result, |caps: &regex::Captures| self.mask_email(&caps[0])).to_string();
        result = phone_re.replace_all(&result, |caps: &regex::Captures| self.mask_phone(&caps[0])).to_string();
        result = id_re.replace_all(&result, |caps: &regex::Captures| self.mask_id_card(&caps[0])).to_string();
        result
    }
}

impl Default for DataMasker {
    fn default() -> Self { Self::new(MaskLevel::Medium) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_phone() {
        let masker = DataMasker::new(MaskLevel::Medium);
        assert_eq!(masker.mask_phone("13812345678"), "138****5678");
    }

    #[test]
    fn test_mask_email() {
        let masker = DataMasker::new(MaskLevel::Medium);
        assert_eq!(masker.mask_email("test@example.com"), "t***@example.com");
    }

    #[test]
    fn test_mask_name() {
        let masker = DataMasker::new(MaskLevel::Medium);
        assert_eq!(masker.mask_name("张三"), "张*");
        assert_eq!(masker.mask_name("张小明"), "张*明");
    }

    #[test]
    fn test_auto_mask() {
        let masker = DataMasker::new(MaskLevel::Medium);
        let text = "联系电话13812345678，邮箱test@example.com";
        let result = masker.auto_mask(text);
        assert!(result.contains("138****5678"));
        assert!(result.contains("t***@example.com"));
    }
}
