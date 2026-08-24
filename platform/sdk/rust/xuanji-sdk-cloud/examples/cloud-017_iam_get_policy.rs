use xuanji_sdk_cloud::{Client, IamPolicy};

#[tokio::main]
async fn main() {
    let c = Client::new();
    let doc = r#"{"Version":"1"}"#.to_string();
    let p = IamPolicy { name: "p1".into(), document: doc, version: "1".into() };
    c.iam_put_policy(p).await.unwrap();
    let got = c.iam_get_policy("p1").await.unwrap();
    assert_eq!(got.name, "p1");
    println!("XJ-OK: cloud-017_iam_get_policy");
}
