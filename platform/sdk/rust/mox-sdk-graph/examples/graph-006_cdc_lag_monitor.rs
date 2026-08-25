use mox_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.cdc_new_consumer("t", "lagmon").await.unwrap();
    let reported = g.cdc_lag_sample("lagmon", 2500).await.unwrap(); // 2.5s spike
    assert_eq!(reported, 2500);
    let c = g.cdc_get_consumer("lagmon").await.unwrap();
    assert_eq!(c.last_lag_ms, 2500);
    println!("XJ-OK: graph-006_cdc_lag_monitor");
}
