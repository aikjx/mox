use xuanji_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.quota_set("api-v1", 1000, 10).await.unwrap();
    let q = c.quota_get("api-v1").await.unwrap();
    assert_eq!(q.burst, 10);
    // burst of 10 requests pass the check
    for _ in 0..10 {
        c.quota_check("api-v1", 1).await.unwrap();
    }
    println!("XJ-OK: cloud-020_quota_burst_10");
}
