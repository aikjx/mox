use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn compute_hash(
    prev_hash: Option<&str>,
    biz_id: &str,
    version: i64,
    snapshot_after: &Value,
    operator: &str,
    created_at: &str,
) -> String {
    let mut hasher = Sha256::new();
    if let Some(prev) = prev_hash {
        hasher.update(prev.as_bytes());
    } else {
        hasher.update(b"GENESIS");
    }
    hasher.update(b"|");
    hasher.update(biz_id.as_bytes());
    hasher.update(b"|");
    hasher.update(version.to_string().as_bytes());
    hasher.update(b"|");
    let snap_str = serde_json::to_string(snapshot_after).unwrap_or_default();
    hasher.update(snap_str.as_bytes());
    hasher.update(b"|");
    hasher.update(operator.as_bytes());
    hasher.update(b"|");
    hasher.update(created_at.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}
