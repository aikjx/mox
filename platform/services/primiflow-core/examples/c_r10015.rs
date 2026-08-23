//! example c_r10015：模块 Report::report 实际可执行（r4 需求 · report 子任务）
#[path = "out/c_r10015.rs"]
mod m;
fn main() {
    let s = m::Report::new();
    println!("[c_r10015] new={:?}", s);
    s.report();
    println!("[c_r10015] OK");
}
