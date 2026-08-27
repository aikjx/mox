// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! example c_r10007：模块 Pull::pull 实际可执行（r3 需求 · pull 子任务）
#[path = "out/c_r10007.rs"]
mod m;
fn main() {
    let s = m::Pull::new();
    println!("[c_r10007] new={:?}", s);
    s.pull();
    println!("[c_r10007] OK");
}
