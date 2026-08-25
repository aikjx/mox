use mox_sdk_cloud::{Client, CloudError};

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.put_object("sec", "sealed.dat", b"immutable".to_vec()).await.unwrap();
    c.worm_put_retention("sec", "sealed.dat", "compliance", 315360000).await.unwrap(); // 10y
    // Trying to overwrite retention mode → WormLocked
    let err = c.worm_put_retention("sec", "sealed.dat", "governance", 1).await.unwrap_err();
    assert!(matches!(err, CloudError::WormLocked(_)), "got: {:?}", err);
    // Trying to delete → WormLocked
    let err2 = c.delete_object("sec", "sealed.dat").await.unwrap_err();
    assert!(matches!(err2, CloudError::WormLocked(_)), "got: {:?}", err2);
    println!("XJ-OK: cloud-024_worm_compliance_immutable");
}
