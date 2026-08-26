use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let info = c.create_bucket("my-first-bucket").await.unwrap();
    assert_eq!(info.name, "my-first-bucket");
    println!("XJ-OK: cloud-001_create_bucket");
}
