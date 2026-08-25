use mox_sdk_cloud::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let roles = [
        "arn:xj:iam:::role/level1",
        "arn:xj:iam:::role/level2",
        "arn:xj:iam:::role/level3",
    ];
    let chain = c.sts_assume_chain(&roles, 900).await.unwrap();
    assert_eq!(chain.len(), 3);
    for (t, r) in chain.iter().zip(roles.iter()) {
        assert!(t.access_key.contains(&r.replace(':', "-")));
    }
    println!("XJ-OK: cloud-015_sts_assume_chain");
}
