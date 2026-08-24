use xuanji_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let data = b"hello cloud object storage".to_vec();
    let etag = c.put_object("bkt", "greeting.txt", data.clone()).await.unwrap();
    assert!(!etag.is_empty());
    println!("XJ-OK: cloud-006_put_object");
}
