use mox_cloud_sdk::{Client, CloudError};

#[tokio::main]
async fn main() {
    let c = Client::new();
    let err = c.sts_assume_role("arn:xj:iam:::role/admin", 3600).await.unwrap_err();
    assert!(matches!(err, CloudError::StsRejected(_)), "got: {:?}", err);
    println!("XJ-OK: cloud-013_sts_assume_3600s_reject");
}
