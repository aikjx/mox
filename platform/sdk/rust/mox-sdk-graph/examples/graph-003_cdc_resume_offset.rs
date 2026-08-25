use mox_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.cdc_new_consumer("t", "c").await.unwrap();
    g.cdc_write_records(100, "ev").await.unwrap();
    // Skip first 50, resume at 50
    g.cdc_resume_offset("c", 50).await.unwrap();
    let r = g.cdc_next_blocking("c").await.unwrap();
    assert_eq!(r.offset, 50);
    println!("XJ-OK: graph-003_cdc_resume_offset");
}
