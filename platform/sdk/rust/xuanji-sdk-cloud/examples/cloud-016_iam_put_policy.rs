use xuanji_sdk_cloud::{Client, IamPolicy};

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
