//! example c_r10012：模块 Feature::feature 实际可执行（r4 需求 · feature 子任务）
#[path = "out/c_r10012.rs"]
mod m;
fn main() {
    let s = m::Feature::new();
    println!("[c_r10012] new={:?}", s);
    s.feature();
    println!("[c_r10012] OK");
}
