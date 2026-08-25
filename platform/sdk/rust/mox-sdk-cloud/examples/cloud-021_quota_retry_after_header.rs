use mox_sdk_cloud::{Client, CloudError};

#[tokio::main]
async fn main() {
    let c = Client::new();
    // 0 rpm → disabled → QuotaExceeded with retry-after header
    c.quota_set("blocked-scope", 0, 0).await.unwrap();
    match c.quota_check("blocked-scope", 1).await.unwrap_err() {
        CloudError::QuotaExceeded(retry_after) => {
            assert!(retry_after > 0, "retry-after header must be > 0");
        }
        other => panic!("expected QuotaExceeded, got {:?}", other),
    }
    println!("XJ-OK: cloud-021_quota_retry_after_header");
}
