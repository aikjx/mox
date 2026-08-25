use mox_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let tok = c.sts_assume_role("arn:xj:iam:::role/read-only", 900).await.unwrap();
    assert_eq!(tok.duration_secs, 900);
    assert!(!tok.session_token.is_empty());
    println!("XJ-OK: cloud-012_sts_assume_900s_ok");
}
