// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! flowopt —— 流程图优化 CLI
//!
//! 用法：
//!   flowopt demo                          运行内置政务场景演示
//!   flowopt optimize <flow.json> [--out DIR] [--no-repair] [--json]
//!   flowopt reverse  <script.py> [--out flow.json]
//!   flowopt mermaid  <flow.json>

use mox_ai_flow_svc::prelude::*;
use mox_ai_flow_svc::{dump_flow, load_flow, to_mermaid};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        return ExitCode::from(2);
    }
    let r = match args[0].as_str() {
        "demo" => cmd_demo(),
        "optimize" => cmd_optimize(&args[1..]),
        "reverse" => cmd_reverse(&args[1..]),
        "mermaid" => cmd_mermaid(&args[1..]),
        "-h" | "--help" | "help" => {
            usage();
            Ok(())
        }
        other => Err(anyhow::anyhow!("未知子命令: {other}")),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!(
        r#"flowopt —— 业务流程图 AI 优化器

  flowopt demo                                   内置政务 RPA 场景演示
  flowopt optimize <flow.json> [选项]            优化流程图
       --out <DIR>    输出生成的 Python 工程
       --no-repair    关闭自动修复
       --json         输出完整 JSON 报告
  flowopt reverse <script.py> [--out <flow.json>]  Python RPA 代码反生成流程图
  flowopt mermaid <flow.json>                     导出 Mermaid 流程图
"#
    );
}

fn cmd_demo() -> anyhow::Result<()> {
    let g = demo_flow();
    println!("== 原始流程（人工线性串联）==");
    println!("{}", to_mermaid(&g));

    let rep = optimize(&g, &OptimizeConfig::default());
    println!("== 优化报告 ==");
    println!("{}", rep.summary());

    println!("== 剪除的伪依赖 ==");
    for (a, b) in &rep.plan.removed_edges {
        println!("  {a} -> {b}  (无数据/副作用关系，可并行)");
    }

    println!("\n== 并行层 ==");
    for (i, l) in rep.plan.layers.iter().enumerate() {
        println!("  L{i}: {}", l.join(", "));
    }

    println!("\n== 关键路径 ==");
    for p in &rep.critical_path.critical_paths {
        println!("  {}", p.join(" -> "));
    }
    println!("  浮动时间排行:");
    let mut ts: Vec<&NodeTiming> = rep
        .critical_path
        .timings
        .iter()
        .filter(|t| t.duration_ms > 0)
        .collect();
    ts.sort_by_key(|t| t.total_float);
    for t in ts {
        println!(
            "    {:<10} dur={:>5}ms  float={:>5}ms  {}",
            t.id,
            t.duration_ms,
            t.total_float,
            if t.critical { "← 关键" } else { "" }
        );
    }

    println!("\n== 资源受限排程 ==");
    for s in &rep.schedule.slots {
        if s.finish_ms > s.start_ms {
            println!(
                "  [{:>5} - {:>5}] {:<10} pool={}",
                s.start_ms, s.finish_ms, s.id, s.pool
            );
        }
    }
    for p in &rep.schedule.pools {
        println!(
            "  池 {:<8} 容量={} 峰值={} 利用率={:.0}%",
            p.pool,
            p.capacity,
            p.peak,
            p.utilization * 100.0
        );
    }

    println!("\n== 冲突检测 ==");
    if rep.conflicts.conflicts.is_empty() {
        println!("  无");
    }
    for c in &rep.conflicts.conflicts {
        println!("  [{:?}/{:?}] {}", c.severity, c.kind, c.message);
    }

    println!("\n== 优化后流程图 ==");
    println!("{}", to_mermaid(&rep.optimized_graph));

    if let Some(code) = &rep.code {
        println!("== 生成代码 ==");
        for f in &code.files {
            println!("  {} ({} 行)", f.path, f.content.lines().count());
        }
        if let Some(sch) = code.file("generated/scheduler.py") {
            println!("\n---- scheduler.py 摘录 ----");
            for line in sch.content.lines().take(28) {
                println!("{line}");
            }
        }
    }
    Ok(())
}

fn cmd_optimize(args: &[String]) -> anyhow::Result<()> {
    let path = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("缺少 flow.json 路径"))?;
    let out_dir = flag_value(args, "--out");
    let cfg = OptimizeConfig {
        auto_repair: !args.iter().any(|a| a == "--no-repair"),
        emit_code: true,
        fast_path_threshold: 0.15,
    };
    let g = load_flow(&fs::read_to_string(path)?)?;
    let rep = optimize(&g, &cfg);

    if args.iter().any(|a| a == "--json") {
        println!("{}", serde_json::to_string_pretty(&rep)?);
    } else {
        print!("{}", rep.summary());
        for c in &rep.conflicts.conflicts {
            println!("  [{:?}] {}", c.severity, c.message);
        }
    }

    if let Some(dir) = out_dir {
        let code = rep
            .code
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("未生成代码"))?;
        if code.rejected {
            anyhow::bail!(
                "存在阻断级冲突，拒绝出码:\n  - {}",
                code.reject_reasons.join("\n  - ")
            );
        }
        for f in &code.files {
            let p = Path::new(&dir).join(&f.path);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&p, &f.content)?;
        }
        println!("已输出 {} 个文件到 {}", code.files.len(), dir);
    }
    Ok(())
}

fn cmd_reverse(args: &[String]) -> anyhow::Result<()> {
    let path = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("缺少 .py 路径"))?;
    let src = fs::read_to_string(path)?;
    let stem = Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "legacy".into());
    let res = reverse_from_python(&src, &stem);
    println!(
        "反解析节点数: {}  边数: {}",
        res.graph.nodes.len(),
        res.graph.edges.len()
    );
    for gap in &res.gaps {
        println!("  [缺陷补全] {gap}");
    }
    println!("\n{}", to_mermaid(&res.graph));
    if let Some(out) = flag_value(args, "--out") {
        fs::write(&out, dump_flow(&res.graph)?)?;
        println!("已写入 {out}");
    }
    Ok(())
}

fn cmd_mermaid(args: &[String]) -> anyhow::Result<()> {
    let path = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("缺少 flow.json 路径"))?;
    let g = load_flow(&fs::read_to_string(path)?)?;
    println!("{}", to_mermaid(&g));
    Ok(())
}

fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// 内置演示：政务 RPA 数据归集
fn demo_flow() -> FlowGraph {
    let mut g = FlowGraph::new("gov-demo", "政务数据归集流水线");
    g.pools.push(ResourcePool {
        name: "browser".into(),
        capacity: 1,
    });
    g.pools.push(ResourcePool {
        name: "db".into(),
        capacity: 2,
    });

    g.add_node(FlowNode::new("start", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("excel", "读取台账Excel", ToolKind::File, 300)
            .with_access(Access::read("file:ledger.xlsx"))
            .with_access(Access::write("var:ledger"))
            .idempotent(true),
    );
    g.add_node(
        FlowNode::task("web1", "省平台取数", ToolKind::Browser, 500)
            .with_access(Access::write("var:prov"))
            .idempotent(true),
    );
    g.add_node(
        FlowNode::task("web2", "市平台取数", ToolKind::Browser, 400)
            .with_access(Access::write("var:city"))
            .idempotent(true),
    );
    g.add_node(
        FlowNode::task("db", "查询公民信息", ToolKind::Database, 350)
            .with_access(Access::read("db:citizen_info"))
            .with_access(Access::write("var:citizen"))
            .transactional(true)
            .idempotent(true),
    );
    g.add_node(
        FlowNode::task("classify", "意图分类", ToolKind::Llm, 150)
            .with_access(Access::read("var:ledger"))
            .with_access(Access::write("var:intent")),
    );
    g.add_node(
        FlowNode::task("merge", "汇总归集", ToolKind::Compute, 120)
            .with_access(Access::read("var:ledger"))
            .with_access(Access::read("var:prov"))
            .with_access(Access::read("var:city"))
            .with_access(Access::read("var:citizen"))
            .with_access(Access::read("var:intent"))
            .with_access(Access::write("file:result.xlsx")),
    );
    g.add_node(FlowNode::new("end", "结束", NodeKind::End));

    for (a, b) in [
        ("start", "excel"),
        ("excel", "web1"),
        ("web1", "web2"),
        ("web2", "db"),
        ("db", "classify"),
        ("classify", "merge"),
        ("merge", "end"),
    ] {
        g.add_edge(FlowEdge::seq(a, b));
    }

    g.rules.push(ExpertRule {
        id: "GOV-SEC-001".into(),
        description: "公民敏感数据出库前必须脱敏".into(),
        severity: Severity::Blocking,
        resource_prefixes: vec!["db:citizen_".into()],
        tool_kinds: vec![],
        required_guard_tags: vec!["desensitize".into()],
    });
    g
}
