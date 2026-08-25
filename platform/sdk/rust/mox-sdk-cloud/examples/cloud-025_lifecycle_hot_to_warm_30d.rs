use mox_sdk_cloud::{Client, LifecycleRule};

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("telemetry").await.unwrap();
    c.lifecycle_put_rule("telemetry", LifecycleRule {
        id: "hot->warm@30d".into(),
        from_storage_class: "hot".into(),
        to_storage_class: "warm".into(),
        after_days: 30,
        prefix: "metrics/".into(),
    }).await.unwrap();
    let rules = c.lifecycle_list_rules("telemetry").await.unwrap();
    assert!(rules.iter().any(|r| r.after_days == 30 && r.to_storage_class == "warm"));
    println!("XJ-OK: cloud-025_lifecycle_hot_to_warm_30d");
}
