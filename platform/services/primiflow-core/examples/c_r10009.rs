//! example c_r10009：模块 Cluster::cluster 实际可执行（r3 需求 · cluster 子任务）
#[path = "out/c_r10009.rs"]
mod m;
fn main() {
    let s = m::Cluster::new();
    println!("[c_r10009] new={:?}", s);
    s.cluster();
    println!("[c_r10009] OK");
}
