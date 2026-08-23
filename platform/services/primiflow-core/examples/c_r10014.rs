//! example c_r10014：模块 Alert::alert 实际可执行（r4 需求 · alert 子任务）
#[path = "out/c_r10014.rs"]
mod m;
fn main() {
    let s = m::Alert::new();
    println!("[c_r10014] new={:?}", s);
    s.alert();
    println!("[c_r10014] OK");
}
