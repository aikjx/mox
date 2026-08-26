use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.put_object("hold", "evidence.log", b"audit trail".to_vec()).await.unwrap();
    c.worm_set_legal_hold("hold", "evidence.log", true).await.unwrap();
    let w1 = c.worm_get("hold", "evidence.log").await.unwrap();
    assert!(w1.legal_hold);
    c.worm_set_legal_hold("hold", "evidence.log", false).await.unwrap();
    let w2 = c.worm_get("hold", "evidence.log").await.unwrap();
    assert!(!w2.legal_hold);
    println!("XJ-OK: cloud-023_worm_legal_hold_on_off");
}
