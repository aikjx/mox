// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::{Client, IamPolicy};

#[tokio::main]
async fn main() {
    let c = Client::new();
    let doc = r#"{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Action":"s3:*","Resource":"*"}]}"#.to_string();
    c.iam_put_policy(IamPolicy {
        name: "dev-full-access".into(),
        document: doc,
        version: "2012-10-17".into(),
    }).await.unwrap();
    println!("XJ-OK: cloud-016_iam_put_policy");
}
