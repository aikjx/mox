use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.put_object("archive", "contract-2024.pdf", b"pdf-signatures".to_vec()).await.unwrap();
    let one_year_seconds: u64 = 365 * 24 * 3600;
    c.worm_put_retention("archive", "contract-2024.pdf", "governance", one_year_seconds).await.unwrap();
    let w = c.worm_get("archive", "contract-2024.pdf").await.unwrap();
    assert_eq!(w.retain_until, one_year_seconds);
    println!("XJ-OK: cloud-022_worm_retention_1y");
}
