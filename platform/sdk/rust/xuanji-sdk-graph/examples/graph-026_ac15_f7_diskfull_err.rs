use xuanji_sdk_graph::{Client, GraphError};

#[tokio::main]
async fn main() {
    let g = Client::new();
    // Baseline: without fault → OK
    let (bytes, _r) = g.ac15_f7_diskfull(1024).await.unwrap();
    assert_eq!(bytes, 1024);
    // Inject f7 fault → DiskFull error
    g.ac15_inject("f7").await.unwrap();
    let err = g.ac15_f7_diskfull(4096).await.unwrap_err();
    assert!(matches!(err, GraphError::DiskFull(_)), "got: {:?}", err);
    let report = g.ac15_report().await.unwrap();
    assert!(report.diskfull_triggered);
    println!("XJ-OK: graph-026_ac15_f7_diskfull_err");
}
