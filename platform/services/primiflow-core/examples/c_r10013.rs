//! example c_r10013：模块 Model::model 实际可执行（r4 需求 · model 子任务）
#[path = "out/c_r10013.rs"]
mod m;
fn main() {
    let s = m::Model::new();
    println!("[c_r10013] new={:?}", s);
    s.model();
    println!("[c_r10013] OK");
}
