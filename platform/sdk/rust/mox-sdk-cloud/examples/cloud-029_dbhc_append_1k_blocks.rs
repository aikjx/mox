use mox_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let chain = "audit-ledger-prod";
    c.dbhc_create_chain(chain).await.unwrap();
    let final_idx = c.dbhc_append_blocks(chain, 1000).await.unwrap();
    assert_eq!(final_idx, 1000); // genesis(0) + 1000 appended
    println!("XJ-OK: cloud-029_dbhc_append_1k_blocks");
}
