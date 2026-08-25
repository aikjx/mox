use mox_sdk_cloud::{Client, CloudError, IamPolicy};

#[tokio::main]
async fn main() {
    let c = Client::new();
    // A deny-first policy that explicitly denies deleting the "prod-sealed" bucket
    let deny_doc = r#"{"Effect":"Deny","Action":"s3:DeleteBucket","Resource":"prod-sealed"}"#.to_string();
    c.iam_put_policy(IamPolicy {
        name: "seal-guard".into(),
        document: deny_doc,
        version: "2012-10-17".into(),
    }).await.unwrap();
    let err = c.iam_eval_policy(&["seal-guard"], "s3:DeleteBucket", "prod-sealed").await.unwrap_err();
    assert!(matches!(err, CloudError::IamDeny(_)), "got: {:?}", err);
    println!("XJ-OK: cloud-018_iam_eval_deny_first");
}
