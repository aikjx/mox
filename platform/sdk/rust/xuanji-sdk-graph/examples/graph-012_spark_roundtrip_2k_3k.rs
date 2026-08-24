use xuanji_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let (nodes, edges) = g.spark_inc_roundtrip(2000, 3000).await.unwrap();
    assert!(nodes >= 2000);
    assert!(edges >= 3000);
    let stats = g.spark_stats().await.unwrap();
    assert_eq!(stats.roundtrips, 1);
    println!("XJ-OK: graph-012_spark_roundtrip_2k_3k");
}
