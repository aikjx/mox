// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let tok = c.sts_assume_role("arn:xj:iam:::role/sign-test", 900).await.unwrap();
    // Use the deterministic "valid" signature prefix
    let ok = c.sts_verify_signature(&tok.session_token, "sig-valid-demo").await.unwrap();
    assert!(ok);
    println!("XJ-OK: cloud-014_sts_token_signature_verify");
}
