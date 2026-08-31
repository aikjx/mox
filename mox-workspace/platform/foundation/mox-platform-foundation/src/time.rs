//! 时间工具
//!
//! 统一使用 UTC 时间，输出时再转换为本地时区

use chrono::{DateTime, Utc};

/// 获取当前时间戳（毫秒）
pub fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

/// 获取当前 UTC 时间
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// 格式化时间
pub fn format_time(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_millis_positive() {
        assert!(now_millis() > 0);
    }
}
