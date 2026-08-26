//! example c_r10004：模块 Fetch::fetch 实际可执行（r2 需求 · fetch 子任务）
#[path = "out/c_r10004.rs"]
mod m;
fn main() {
    let s = m::Fetch::new();
    println!("[c_r10004] new={:?}", s);
    s.fetch();
    println!("[c_r10004] OK");
}
