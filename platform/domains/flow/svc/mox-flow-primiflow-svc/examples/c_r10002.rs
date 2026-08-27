// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! example c_r10002：模块 Clean::clean 实际可执行（r1 需求 · clean 子任务）
#[path = "out/c_r10002.rs"]
mod m;
fn main() {
    let s = m::Clean::new();
    println!("[c_r10002] new={:?}", s);
    s.clean();
    println!("[c_r10002] OK");
}
