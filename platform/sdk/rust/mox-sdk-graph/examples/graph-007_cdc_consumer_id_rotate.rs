use mox_sdk_graph::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    g.cdc_new_consumer("t", "id-v1").await.unwrap();
    g.cdc_write_records(10, "r").await.unwrap();
    g.cdc_resume_offset("id-v1", 3).await.unwrap();
    let rotated = g.cdc_rotate_consumer("id-v1", "id-v2").await.unwrap();
    assert_eq!(rotated.id, "id-v2");
    assert_eq!(rotated.offset, 3);
    // old id is gone
    let err = g.cdc_get_consumer("id-v1").await.unwrap_err();
    assert!(matches!(err, mox_sdk_graph::GraphError::NotFound(_)));
    println!("XJ-OK: graph-007_cdc_consumer_id_rotate");
}
