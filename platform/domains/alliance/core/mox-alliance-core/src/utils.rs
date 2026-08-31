// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 通用工具函数

use std::time::SystemTime;

use chrono::{DateTime, Utc};

/// 生成短 ID（8 字符，基于 UUID v7 的前缀）
pub fn short_id() -> String {
    use uuid::Uuid;
    let id = Uuid::new_v4();
    let s = id.to_string();
    s[..8].to_string()
}

/// 获取当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 计算耗时（毫秒）
pub fn duration_ms(start: DateTime<Utc>) -> u64 {
    let now = Utc::now();
    let duration = now - start;
    duration.num_milliseconds().max(0) as u64
}

/// 截断字符串到指定长度
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut result: String = s.chars().take(max_len).collect();
        result.push_str("...");
        result
    }
}

/// 安全的字符串转 f64
pub fn safe_parse_f64(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok().filter(|f| f.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_short_id() {
        let id = short_id();
        assert_eq!(id.len(), 8);
    }

    #[test]
    fn test_now_ms() {
        let ms = now_ms();
        assert!(ms > 1_700_000_000_000); // 2023 年以后
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn test_safe_parse_f64() {
        assert_eq!(safe_parse_f64("3.14"), Some(3.14));
        assert_eq!(safe_parse_f64("  42  "), Some(42.0));
        assert_eq!(safe_parse_f64("abc"), None);
        assert_eq!(safe_parse_f64("NaN"), None);
    }
}
