use mox_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.put_object("bkt", "trash.bin", b"scrap".to_vec()).await.unwrap();
    c.delete_object("bkt", "trash.bin").await.unwrap();
    let err = c.get_object("bkt", "trash.bin").await.unwrap_err();
    assert!(matches!(err, mox_sdk_cloud::CloudError::NotFound(_)));
    println!("XJ-OK: cloud-008_delete_object");
}
