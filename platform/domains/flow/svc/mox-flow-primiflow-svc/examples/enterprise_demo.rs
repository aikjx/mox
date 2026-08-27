// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! PrimiFlow 企业级端到端可运行示例
//!
//! 运行：`cargo run -p primiflow --example enterprise_demo`
//!
//! 对一组覆盖「均衡 / 紧急复用 / 探索分叉 / 超预算正则化」的代表性企业需求，
//! 跑完整 κ‑τ 闭环并产出：分步验证报告 + 涌现 DAG 可视化 + 六维溯源 + 文档自生成产物。
//! 退出码非 0 表示存在未通过项（可用于 CI 门禁）。

use mox_ai_flow_svc::primitive::{KnowledgeBase, PrimiEngine, ResourceBudget};
use mox_flow_primiflow_svc::{enterprise_specs, run_all};
use std::collections::HashMap;
use std::path::Path;

fn main() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/out");
    let _ = std::fs::remove_dir_all(&out);

    // 系统常数 C=10；资源预算总算力 2000ms（r4 会因此触发 ℛ̂ 正则化）
    let budget = ResourceBudget {
        total_ms: 2000,
        per_pool: HashMap::new(),
    };
    let mut engine = PrimiEngine::new(10.0, KnowledgeBase::new(), budget);

    println!("══════════════════════════════════════════════════════════════");
    println!(" PrimiFlow 企业级端到端验证 · κ‑τ 拓扑原语自涌现引擎");
    println!("══════════════════════════════════════════════════════════════");

    let specs = enterprise_specs();
    let reports = match run_all(&mut engine, &specs, &out) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("运行失败: {e}");
            std::process::exit(1);
        }
    };

    let mut all_ok = true;
    for rep in &reports {
        println!();
        print!("{rep}");
        if !rep.all_ok() {
            all_ok = false;
        }
    }

    // 资产沉淀：r1 成功注库后，后续需求命中知识库自动复用（κ 抬高）
    println!("\n══════════════════════════════════════════════════════════════");
    println!(
        " 资产知识库: 已固化 {} 个拓扑模板 · 累计拓扑荷 Q = {:.2}",
        engine.kb.stored.len(),
        engine.state.q
    );
    println!("══════════════════════════════════════════════════════════════");

    // 产物校验
    let files = [
        "graph.mmd",
        "trace_matrix.md",
        "ddl.sql",
        "schema.rs",
        "mod.rs",
    ];
    println!("\n文档自生成产物 (examples/out):");
    let mut artifacts_ok = true;
    for f in files {
        let p = out.join(f);
        let ok = p.exists() && std::fs::metadata(&p).map(|m| m.len() > 0).unwrap_or(false);
        if !ok {
            artifacts_ok = false;
        }
        println!("  [{}] {}", if ok { "OK" } else { "!!" }, f);
    }

    let code_count = std::fs::read_dir(&out)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter(|e| {
                    let n = e.file_name().to_string_lossy().into_owned();
                    n.ends_with(".rs") && n.starts_with("c_")
                })
                .count()
        })
        .unwrap_or(0);
    println!("  代码骨架模块: {code_count} 个 (c_*.rs)");

    let topo_count = std::fs::read_dir(&out)
        .map(|d| {
            d.filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .into_owned()
                        .starts_with("topo_")
                })
                .count()
        })
        .unwrap_or(0);
    println!("  涌现 DAG 可视化: {topo_count} 张 (topo_*.mmd)");

    let pass = all_ok
        && artifacts_ok
        && !engine.kb.stored.is_empty()
        && code_count >= 15
        && topo_count >= 4;

    println!("\n══════════════════════════════════════════════════════════════");
    if pass {
        println!(" ✅ 企业级端到端验证全部通过：可运行 / 守恒 / 溯源 / 文档自生成 全绿");
    } else {
        println!(" ❌ 存在未通过项，请检查上方 FAIL / !!");
    }
    println!("══════════════════════════════════════════════════════════════");

    if !pass {
        std::process::exit(2);
    }
}
