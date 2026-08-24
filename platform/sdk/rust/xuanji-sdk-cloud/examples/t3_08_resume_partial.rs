use xuanji_sdk_cloud::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let bucket = "resume";
    let key = "resume/final.dat";

    // First attempt: start, upload 2 parts, abort (simulated failure)
    let uid1 = client.create_multipart_upload(bucket, key).await.unwrap();
    let _ = client
        .upload_part(bucket, key, &uid1, 1, vec![1u8; 200])
        .await
        .unwrap();
    let _ = client
        .upload_part(bucket, key, &uid1, 2, vec![2u8; 200])
        .await
        .unwrap();
    client.abort_multipart_upload(&uid1).await.unwrap();
    assert_eq!(client.list_multipart_uploads().await.unwrap().len(), 0);

    // Second attempt: resume by creating new upload, uploading all 4 parts to completion
    let uid2 = client.create_multipart_upload(bucket, key).await.unwrap();
    let mut parts: Vec<PartEtag> = Vec::with_capacity(4);
    let sizes: [usize; 4] = [200, 200, 300, 300];
    let mut total = 0usize;
    for (i, &sz) in sizes.iter().enumerate() {
        let n = (i + 1) as u16;
        let data = vec![n as u8; sz];
        total += sz;
        let pe = client
            .upload_part(bucket, key, &uid2, n, data)
            .await
            .unwrap();
        parts.push(pe);
    }
    let etag = client
        .complete_multipart_upload(bucket, key, &uid2, parts)
        .await
        .unwrap();
    let obj = client.get_object(bucket, key).await.unwrap();
    assert_eq!(obj.len(), total);
    assert_eq!(client.list_multipart_uploads().await.unwrap().len(), 0);

    println!(
        "XJ-OK: t3_08_resume_partial first_uid={} second_uid={} total_bytes={} etag={}",
        uid1, uid2, total, etag
    );
}
