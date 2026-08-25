use mox_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("pub-read").await.unwrap();
    c.set_bucket_acl("pub-read", "public-read").await.unwrap();
    let info = c.head_bucket("pub-read").await.unwrap();
    assert_eq!(info.acl, "public-read");
    println!("XJ-OK: cloud-005_set_bucket_acl");
}
