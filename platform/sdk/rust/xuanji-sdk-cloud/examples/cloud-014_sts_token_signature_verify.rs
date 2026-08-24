use xuanji_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let tok = c.sts_assume_role("arn:xj:iam:::role/sign-test", 900).await.unwrap();
    // Use the deterministic "valid" signature prefix
    let ok = c.sts_verify_signature(&tok.session_token, "sig-valid-demo").await.unwrap();
    assert!(ok);
    println!("XJ-OK: cloud-014_sts_token_signature_verify");
}
