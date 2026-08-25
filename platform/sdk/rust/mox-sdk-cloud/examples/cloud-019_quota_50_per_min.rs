use mox_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.quota_set("tenant-x", 50, 10).await.unwrap();
    let q = c.quota_get("tenant-x").await.unwrap();
    assert_eq!(q.requests_per_minute, 50);
    c.quota_check("tenant-x", 1).await.unwrap();
    println!("XJ-OK: cloud-019_quota_50_per_min");
}
