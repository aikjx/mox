use mox_sdk_cloud::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let bucket = "bigdata";
    let key = "reports/2026/q2/large.csv";
    let uid = client.create_multipart_upload(bucket, key).await.unwrap();
    const CHUNK: usize = 64 * 1024; // 64 KiB per part
    let mut parts: Vec<PartEtag> = Vec::with_capacity(10);
    let mut total: usize = 0;
    for n in 1..=10u16 {
        let mut data = vec![(n & 0xFF) as u8; CHUNK];
        // Vary first 8 bytes to make each part unique
        for (i, b) in data.iter_mut().take(8).enumerate() {
            *b = (n as usize + i) as u8;
        }
        total += data.len();
        let pe = client
            .upload_part(bucket, key, &uid, n, data)
            .await
            .unwrap();
        parts.push(pe);
    }
    assert_eq!(parts.len(), 10);
    let final_etag = client
        .complete_multipart_upload(bucket, key, &uid, parts)
        .await
        .unwrap();
    let obj = client.get_object(bucket, key).await.unwrap();
    assert_eq!(obj.len(), total);
    assert_eq!(total, 10 * CHUNK);
    println!(
        "XJ-OK: t3_07_upload_10parts_big parts=10 total_bytes={} etag={}",
        total, final_etag
    );
}
