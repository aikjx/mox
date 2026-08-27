// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! example c_r10003：模块 Report::report 实际可执行（r1 需求 · report 子任务）
#[path = "out/c_r10003.rs"]
mod m;
fn main() {
    let s = m::Report::new();
    println!("[c_r10003] new={:?}", s);
    s.report();
    println!("[c_r10003] OK");
}
