use xuanji_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("existential").await.unwrap();
    let info = c.head_bucket("existential").await.unwrap();
    assert_eq!(info.name, "existential");
    assert_eq!(info.acl, "private");
    println!("XJ-OK: cloud-004_head_bucket");
}
