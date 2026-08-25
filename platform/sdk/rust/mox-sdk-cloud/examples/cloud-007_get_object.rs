use mox_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let payload = b"get-me-please".to_vec();
    c.put_object("bkt", "needle.txt", payload.clone()).await.unwrap();
    let got = c.get_object("bkt", "needle.txt").await.unwrap();
    assert_eq!(got, payload);
    println!("XJ-OK: cloud-007_get_object");
}
