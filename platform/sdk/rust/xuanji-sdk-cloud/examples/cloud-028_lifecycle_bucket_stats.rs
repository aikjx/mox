use xuanji_sdk_cloud::{Client, LifecycleRule};

#[tokio::main]
async fn main() {
    let c = Client::new();
    for i in 0..10u8 {
        let key = format!("data/chunk_{:03}.bin", i);
        c.put_object("sb", &key, vec![i; (i as usize) * 64 + 64]).await.unwrap();
    }
    c.lifecycle_put_rule("sb", LifecycleRule {
        id: "r1".into(), from_storage_class: "hot".into(),
        to_storage_class: "warm".into(), after_days: 30, prefix: "data/".into(),
    }).await.unwrap();
    let stats = c.lifecycle_bucket_stats("sb").await.unwrap();
    assert_eq!(stats.bucket, "sb");
    assert!(stats.transitioned_last_30d > 0);
    println!("XJ-OK: cloud-028_lifecycle_bucket_stats");
}
