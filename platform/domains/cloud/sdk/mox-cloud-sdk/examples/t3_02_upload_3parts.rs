use mox_cloud_sdk::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let uid = client
        .create_multipart_upload("bk", "obj/3part.zip")
        .await
        .unwrap();
    let mut etags: Vec<PartEtag> = Vec::with_capacity(3);
    for n in 1..=3u16 {
        let size = (n as usize) * 256;
        let chunk = vec![n as u8; size];
        let pe = client
            .upload_part("bk", "obj/3part.zip", &uid, n, chunk)
            .await
            .unwrap();
        assert_eq!(pe.part_number, n);
        assert_eq!(pe.etag.len(), 16);
        etags.push(pe);
    }
    assert_eq!(etags.len(), 3);
    println!(
        "XJ-OK: t3_02_upload_3parts uid={} etags={:?}",
        uid,
        etags.iter().map(|e| &e.etag).collect::<Vec<_>>()
    );
}
