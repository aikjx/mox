//! example c_r10005：模块 Stock::stock 实际可执行（r2 需求 · stock 子任务）
#[path = "out/c_r10005.rs"]
mod m;
fn main() {
    let s = m::Stock::new();
    println!("[c_r10005] new={:?}", s);
    s.stock();
    println!("[c_r10005] OK");
}
