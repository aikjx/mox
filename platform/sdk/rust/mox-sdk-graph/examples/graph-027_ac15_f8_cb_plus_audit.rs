use mox_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let (fired, report) = g.ac15_f8_cb_audit("tx-commit-event").await.unwrap();
    assert!(fired);
    assert!(report.callback_fired);
    assert_eq!(report.fault_tag, "f8");
    assert_eq!(report.audit_entries.len(), 1);
    assert!(report.audit_entries[0].contains("tx-commit-event"));
    println!("XJ-OK: graph-027_ac15_f8_cb_plus_audit");
}
