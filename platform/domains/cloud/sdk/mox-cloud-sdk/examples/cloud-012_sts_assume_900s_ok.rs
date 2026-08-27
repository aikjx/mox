// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let tok = c.sts_assume_role("arn:xj:iam:::role/read-only", 900).await.unwrap();
    assert_eq!(tok.duration_secs, 900);
    assert!(!tok.session_token.is_empty());
    println!("XJ-OK: cloud-012_sts_assume_900s_ok");
}
