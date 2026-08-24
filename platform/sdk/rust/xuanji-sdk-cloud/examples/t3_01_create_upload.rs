use xuanji_sdk_cloud::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let uid = client
        .create_multipart_upload("my-bucket", "data/large-file.bin")
        .await
        .expect("create multipart upload");
    assert!(!uid.is_empty(), "upload_id must not be empty");
    assert!(
        uid.starts_with("mpu-"),
        "upload_id must start with mpu- prefix, got {}",
        uid
    );
    println!("XJ-OK: t3_01_create_upload id={}", uid);
}
