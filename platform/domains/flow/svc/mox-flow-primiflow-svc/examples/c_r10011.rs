// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! example c_r10011：模块 Ingest::ingest 实际可执行（r4 需求 · ingest 子任务）
#[path = "out/c_r10011.rs"]
mod m;
fn main() {
    let s = m::Ingest::new();
    println!("[c_r10011] new={:?}", s);
    s.ingest();
    println!("[c_r10011] OK");
}
