use xuanji_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("alpha").await.unwrap();
    c.create_bucket("beta").await.unwrap();
    c.create_bucket("gamma").await.unwrap();
    let list = c.list_buckets().await.unwrap();
    assert_eq!(list.len(), 3);
    println!("XJ-OK: cloud-003_list_buckets");
}
