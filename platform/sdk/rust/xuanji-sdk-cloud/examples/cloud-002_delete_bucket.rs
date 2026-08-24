use xuanji_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("doomed-bucket").await.unwrap();
    c.delete_bucket("doomed-bucket").await.unwrap();
    let list = c.list_buckets().await.unwrap();
    assert!(!list.iter().any(|b| b.name == "doomed-bucket"));
    println!("XJ-OK: cloud-002_delete_bucket");
}
