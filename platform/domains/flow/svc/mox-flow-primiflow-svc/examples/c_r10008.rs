//! example c_r10008：模块 Embed::embed 实际可执行（r3 需求 · embed 子任务）
#[path = "out/c_r10008.rs"]
mod m;
fn main() {
    let s = m::Embed::new();
    println!("[c_r10008] new={:?}", s);
    s.embed();
    println!("[c_r10008] OK");
}
