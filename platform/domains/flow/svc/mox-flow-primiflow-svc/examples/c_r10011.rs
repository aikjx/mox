//! example c_r10011：模块 Ingest::ingest 实际可执行（r4 需求 · ingest 子任务）
#[path = "out/c_r10011.rs"]
mod m;
fn main() {
    let s = m::Ingest::new();
    println!("[c_r10011] new={:?}", s);
    s.ingest();
    println!("[c_r10011] OK");
}
