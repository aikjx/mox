//! example c_r10001：模块 Fetch::fetch 实际可执行（r1 需求 · fetch 子任务）
#[path = "out/c_r10001.rs"]
mod m;
fn main() {
    let s = m::Fetch::new();
    println!("[c_r10001] new={:?}", s);
    s.fetch();
    println!("[c_r10001] OK");
}
