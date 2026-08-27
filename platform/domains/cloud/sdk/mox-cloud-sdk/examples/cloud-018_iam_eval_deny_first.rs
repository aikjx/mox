// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::{Client, CloudError, IamPolicy};

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
