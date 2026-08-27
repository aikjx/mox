// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 示例：六维绑定 Registry（R06）+ PT-DOC 自生成（R08）端到端演示
//!
//! 运行：`cargo run -p primiflow-fusion --example registry_demo`
//!
//! 演示：
//! 1. 平台以 JSON 持久化注册表启动（跨重启复用历史绑定）；
//! 2. 多次 `synthesize` 累积六维绑定；
//! 3. 按代码节点反查需求（溯源）；
//! 4. 导出 PT-Primi 标准文档集（PT-DOC 01..10）；
//! 5. 打印平台级全局闸门判定。

use mox_flow_fusion_svc::ptdoc::PtdocSet;
use mox_flow_fusion_svc::sixdim::SixDimRegistry;
use mox_flow_fusion_svc::PrimiPlatform;
use std::path::PathBuf;

fn main() {
    let reg_path = PathBuf::from("target/registry_demo/registry.json");
    let doc_dir = PathBuf::from("target/registry_demo/ptdoc");
    let _ = std::fs::remove_dir_all(reg_path.parent().unwrap());

    // 以持久化注册表启动（首跑为空，重启后自动恢复）
    let mut platform = PrimiPlatform::with_persistence(reg_path.clone());

    println!("=== PrimiFlow 多维度融合归一化 · 一体化平台 ===\n");

    // 三次合成：均衡 / 探索分叉 / 超域拒绝
    let mut total_docs = 0;
    for (i, (req, s)) in [
        ("请抓取销售数据。清洗对账。生成图表报告。", 0.2_f64),
        ("监控生产日志，异常告警自动派单给值班工程师。", 0.6_f64),
        ("帮我写一首关于春天的诗", 0.2_f64), // 超域，应被拒
    ]
    .into_iter()
    .enumerate()
    {
        let rep = platform.synthesize_and_emit_docs(req, s, &doc_dir);
        total_docs = rep.ptdocs;
        let st = &rep.orchestration.status;
        println!(
            "[{i}] 需求={} | 状态={:?} | 新绑定={} | 闸门={} | ΣC={:.3}",
            req,
            st,
            rep.registered,
            if rep.gate.passed {
                "通过 ✅"
            } else {
                "未通过 ❌"
            },
            rep.gate.conservation.total_c
        );
    }

    // 注册表统计
    let stats = platform.registry.stats();
    println!("\n--- 六维绑定注册表统计 ---\n{}", stats.to_line());

    // 溯源：按代码节点反查需求
    if let Some(b) = platform.registry.bindings.first() {
        let code_id = &b.code;
        let hits = platform.registry.by_code(code_id);
        println!(
            "\n溯源：代码节点 {} 反查到 {} 条需求（首条：{}）",
            code_id,
            hits.len(),
            hits.first().map(|h| h.req_text.as_str()).unwrap_or("")
        );
    }

    // 平台级全局闸门
    let gate = platform.graph.full_gate();
    println!(
        "\n=== 平台级全局闸门：{}（守恒 {} / 绑定 {} / 治理 {}）===",
        if gate.passed {
            "通过 ✅"
        } else {
            "未通过 ❌"
        },
        mark(gate.conservation.passed),
        mark(gate.binding.passed),
        mark(gate.governance.passed)
    );

    // PT-DOC 概览
    let set = PtdocSet::generate(&platform.registry, &gate, &platform.graph);
    println!(
        "\n=== PT-DOC 标准文档集（{} 份，已导出至 {}）===",
        set.docs.len(),
        doc_dir.display()
    );
    for d in &set.docs {
        println!("  {} {}", d.code, d.title);
    }

    println!(
        "\n落盘注册表：{}（重启后 with_persistence 可恢复 {} 条绑定）",
        reg_path.display(),
        SixDimRegistry::load(&reg_path)
            .map(|r| r.len())
            .unwrap_or(0)
    );
    println!("本次共生成 {} 份 PT-DOC。", total_docs);
}

fn mark(b: bool) -> &'static str {
    if b {
        "✅"
    } else {
        "❌"
    }
}
