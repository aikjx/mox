use mox_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let (ms, report) = g.ac15_f13_lag_spike(12_500).await.unwrap();
    assert_eq!(ms, 12_500);
    assert_eq!(report.lag_spike_ms, 12_500);
    assert_eq!(report.fault_tag, "f13");
    println!("XJ-OK: graph-029_ac15_f13_lag_spike");
}
