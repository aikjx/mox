// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! example c_r10006：模块 Report::report 实际可执行（r2 需求 · report 子任务）
#[path = "out/c_r10006.rs"]
mod m;
fn main() {
    let s = m::Report::new();
    println!("[c_r10006] new={:?}", s);
    s.report();
    println!("[c_r10006] OK");
}
