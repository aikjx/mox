use xuanji_sdk_cloud::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let names = ["alpha", "beta", "gamma", "delta"];
    for (i, n) in names.iter().enumerate() {
        let uid = client
            .create_multipart_upload("lb", format!("file/{}.bin", n))
            .await
            .unwrap();
        if i % 2 == 0 {
            let _ = client
                .upload_part("lb", &format!("file/{}.bin", n), &uid, 1, vec![0u8; 64])
                .await
                .unwrap();
        }
    }
    let list = client.list_multipart_uploads().await.unwrap();
    assert_eq!(list.len(), 4, "expected 4 active uploads");
    let with_parts: Vec<_> = list.iter().filter(|m| m.parts_count > 0).collect();
    assert_eq!(with_parts.len(), 2, "2 uploads should have a part uploaded");
    println!(
        "XJ-OK: t3_05_list_uploads total={} with_parts={}",
        list.len(),
        with_parts.len()
    );
}
