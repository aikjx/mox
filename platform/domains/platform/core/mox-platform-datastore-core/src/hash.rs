//! 版本链哈希计算
//!
//! 使用 SHA256 计算业务记录版本哈希，确保数据完整性与可追溯性。
//! 哈希输入: prev_hash + biz_id + version + data_json + user_id + timestamp

use serde_json::Value;
use sha2::{Digest, Sha256};

/// 计算业务记录版本哈希（SHA256 hex，64字符）
///
/// # 参数
/// - `prev_hash`: 前一版本哈希（首版本为 None）
/// - `biz_id`: 业务记录ID
/// - `version`: 版本号
/// - `data`: 业务数据 JSON
/// - `user_id`: 操作用户ID
/// - `timestamp`: 操作时间戳（ISO8601格式）
///
/// # 返回
/// 64字符的小写SHA256 hex字符串
pub fn compute_hash(
    prev_hash: Option<&str>,
    biz_id: &str,
    version: i64,
    data: &Value,
    user_id: &str,
    timestamp: &str,
) -> String {
    let mut hasher = Sha256::new();

    // 规范化数据 JSON（确保相同数据产生相同哈希）
    let data_str = normalize_json(data);

    // 按固定顺序拼接输入
    hasher.update(prev_hash.unwrap_or("GENESIS"));
    hasher.update(b"|");
    hasher.update(biz_id.as_bytes());
    hasher.update(b"|");
    hasher.update(version.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(data_str.as_bytes());
    hasher.update(b"|");
    hasher.update(user_id.as_bytes());
    hasher.update(b"|");
    hasher.update(timestamp.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}

/// 规范化 JSON 字符串（按键排序，去除空格）
fn normalize_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let inner: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", escape_json_string(k), normalize_json(v)))
                .collect();
            format!("{{{}}}", inner.join(","))
        }
        Value::Array(arr) => {
            let inner: Vec<String> = arr.iter().map(normalize_json).collect();
            format!("[{}]", inner.join(","))
        }
        Value::String(s) => format!("\"{}\"", escape_json_string(s)),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
    }
}

/// JSON 字符串转义
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if (c as u32) < 0x20 => result.push_str(&format!("\\u{:04x}", c as u32)),
            c => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let data = serde_json::json!({"k":"v"});
        let h1 = compute_hash(Some("abc"), "biz-001", 3, &data, "u1", "2024-01-01T00:00:00Z");
        let h2 = compute_hash(Some("abc"), "biz-001", 3, &data, "u1", "2024-01-01T00:00:00Z");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_hash_differs_on_input_change() {
        let data = serde_json::json!({"k":"v"});
        let h1 = compute_hash(Some("abc"), "biz-001", 3, &data, "u1", "2024-01-01T00:00:00Z");
        let h2 = compute_hash(Some("abc"), "biz-001", 4, &data, "u1", "2024-01-01T00:00:00Z");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_genesis_version() {
        let data = serde_json::json!({"k":"v"});
        let h = compute_hash(None, "biz-001", 1, &data, "u1", "2024-01-01T00:00:00Z");
        assert_eq!(h.len(), 64);
    }

    #[test]
    fn test_json_normalization_order_independent() {
        let a = serde_json::json!({"b":2,"a":1});
        let b = serde_json::json!({"a":1,"b":2});
        assert_eq!(normalize_json(&a), normalize_json(&b));
    }
}
