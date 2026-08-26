use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let chain = "cli-verified-chain";
    c.dbhc_append_blocks(chain, 200).await.unwrap(); // creates implicitly
    let verified = c.dbhc_verify_chain(chain).await.unwrap();
    assert!(verified, "dbhc chain verify must pass for honest append");
    println!("XJ-OK: cloud-030_dbhc_verify_cli_ok");
}
