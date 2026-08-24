use xuanji_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let entries = ["rule-a", "rule-b", "rule-c", "rule-d"];
    let (n, report) = g.ac15_f14_audit_cb(&entries).await.unwrap();
    assert_eq!(n, 4);
    assert_eq!(report.audit_entries.len(), 4);
    assert!(report.callback_fired);
    for (i, e) in report.audit_entries.iter().enumerate() {
        assert!(e.contains(entries[i]), "missing entry in audit trail: {e}");
    }
    println!("XJ-OK: graph-030_ac15_f14_audit_cb");
}
