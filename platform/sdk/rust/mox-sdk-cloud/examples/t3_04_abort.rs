use mox_sdk_cloud::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let uid = client
        .create_multipart_upload("abkt", "abort/file.tmp")
        .await
        .unwrap();
    let _pe = client
        .upload_part("abkt", "abort/file.tmp", &uid, 1, b"partial".to_vec())
        .await
        .unwrap();
    let before = client.list_multipart_uploads().await.unwrap();
    assert_eq!(before.len(), 1);
    client.abort_multipart_upload(&uid).await.unwrap();
    let after = client.list_multipart_uploads().await.unwrap();
    assert_eq!(after.len(), 0, "aborted upload must be removed from list");
    println!("XJ-OK: t3_04_abort removed={} remaining={}", uid, after.len());
}
