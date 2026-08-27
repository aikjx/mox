// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 多维度融合归一化一体化演示
//!
//! 运行：`cargo run -p primiflow-fusion --example fuse`

use mox_flow_fusion_svc::registry::CRATE_NAMES;
use mox_flow_fusion_svc::{fuse_all, PlatformReport, PrimiPlatform, UnifiedGraph};

fn main() {
    println!("════════ PrimiFlow 多维度融合归一化一体化 ════════\n");

    // 1) 融合全部 crate 能力 + 数据表 + 六维链
    let g: UnifiedGraph = fuse_all();
    println!(
        "[融合] {} 个 crate 能力域 → 统一图 {} 节点 / {} 边",
        CRATE_NAMES.len(),
        g.nodes.len(),
        g.edges.len()
    );

    // 2) 跑全局治理闸门（守恒 R07 + 六维绑定 A4 + GR-STD 8 闸门）
    let gate = g.full_gate();
    println!(
        "[闸门] 守恒残差 ε 上限 {:.0e} · 总 C={:.1} · 六维节点 {} · 结果 {}",
        mox_flow_fusion_svc::unified::GLOBAL_CONSERVATION_EPS,
        gate.conservation.total_c,
        gate.binding.six_dim_nodes,
        if gate.passed {
            "✅ 通过"
        } else {
            "❌ 未通过"
        }
    );
    if !gate.passed {
        for e in &gate.conservation.errors {
            println!("   ⚠ 守恒: {e}");
        }
        for e in &gate.binding.orphans {
            println!("   ⚠ 绑定: {e}");
        }
        for e in &gate.governance.errors {
            println!("   ⚠ 治理: {e}");
        }
    }

    // 3) 一体化合成：跑主链路 + 注册进统一图 + 闸门
    let mut platform = PrimiPlatform::new();
    println!("\n[一体化] 合成需求：「抓取销售数据 → 清洗对账 → 生成图表报告」");
    let rep: PlatformReport = platform.synthesize("请抓取销售数据。清洗对账。生成图表报告。", 0.2);
    println!("   {}", rep.orchestration.summary());
    println!(
        "   本次新注册绑定 {} · 整图闸门 {}",
        rep.registered,
        if rep.gate.passed { "✅" } else { "❌" }
    );

    // 4) 第二次同类需求：κ 复用命中
    let rep2 = platform.synthesize(
        "我想做一份电商经营分析报告，需要销售数据抓取和图表生成。",
        0.1,
    );
    println!(
        "[复用] 第二次同类需求：状态={:?} · 复用命中 {} 个历史资产",
        rep2.orchestration.status,
        rep2.orchestration.reuse_hits.len()
    );

    // 5) 导出融合图 Mermaid（可喂前端画布）
    println!(
        "\n[可视化] 融合统一图 Mermaid 已生成（{} 字节）",
        g.to_mermaid().len()
    );
    println!("════════ 融合归一化一体化完成 ════════");
}
