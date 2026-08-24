//! xuanji —— 璇玑 CLI / HTTP 服务入口
//!
//! 子命令：
//!   xuanji serve [--port 8080]           启动独立 HTTP 服务 + Three.js 前端
//!   xuanji optimize <flow.json> [--out DIR]   跑全维优化并输出报告 JSON / 生成的 Python
//!   xuanji demo                          内置政务场景端到端演示（含可视化 DTO 校验）

use flow_ai::model::{FlowEdge, FlowNode, NodeKind, ToolKind};
use flow_ai::prelude::*;
use std::fs;
use std::process::ExitCode;
use xuanji_expert::context::{GovernContext, Principal, Tenant};
use xuanji_expert::pipeline::xuanji_optimize;
use xuanji_expert::server::{demo_topology, to_viz};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        usage();
        return ExitCode::from(2);
    }
    let r = match args[0].as_str() {
        "serve" => cmd_serve(&args[1..]),
        "optimize" => cmd_optimize(&args[1..]),
        "verify" => cmd_verify(&args[1..]),
        "bench" => cmd_bench(),
        "demo" => cmd_demo(),
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
        r#"┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
  🌀 璇玑 Xuanji Core · 璇玑智能中心
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
  全维设计·分析·开发·测试·修复·优化 → 流程图即开发产物

  xuanji serve [--port 8080]                 启动独立 HTTP 服务 + Three.js 前端
  xuanji optimize <flow.json> [--out DIR]    全维优化并输出报告
  xuanji verify <flow.json>                  璇玑验证（最高权限，退出码 0=通过 2=否决）
  xuanji bench                               多场景 Benchmark（量化加速比/剪伪依赖/冲突自愈/LLM削减）
  xuanji demo                                内置政务场景端到端演示

  HTTP 接口：
    POST /api/optimize    body: {{ "flow": <FlowGraph JSON>, "instruction": "..." }}
    GET  /                单文件 Three.js 力导向图前端
    GET  /api/health
"#
    );
}

/// 内置政务数据归集场景
fn gov_flow() -> FlowGraph {
    let mut g = FlowGraph::new("gov", "政务数据归集");
    g.add_node(FlowNode::new("s", "开始", NodeKind::Start));
    g.add_node(
        FlowNode::task("read", "读取公民库", ToolKind::Database, 300)
            .with_access(Access::read("db:citizen_info"))
            .with_access(Access::write("var:citizen")),
    );
    g.add_node(FlowNode::task("guard", "脱敏", ToolKind::Compute, 50).with_tag("desensitize"));
    g.add_node(FlowNode::task(
        "web1",
        "网办系统A填报",
        ToolKind::Browser,
        500,
    ));
    g.add_node(FlowNode::task(
        "web2",
        "网办系统B填报",
        ToolKind::Browser,
        400,
    ));
    g.add_node(
        FlowNode::task("merge", "汇总", ToolKind::Compute, 100)
            .with_access(Access::read("var:citizen")),
    );
    g.add_node(FlowNode::new("e", "结束", NodeKind::End));
    g.add_edge(FlowEdge::seq("s", "read"));
    g.add_edge(FlowEdge::seq("read", "guard"));
    g.add_edge(FlowEdge::seq("guard", "web1"));
    g.add_edge(FlowEdge::seq("guard", "web2"));
    g.add_edge(FlowEdge::seq("web1", "merge"));
    g.add_edge(FlowEdge::seq("web2", "merge"));
    g.add_edge(FlowEdge::seq("merge", "e"));
    g
}

fn default_ctx() -> GovernContext {
    let tenant = Tenant::new("gov-tenant", "ns-gov")
        .regulated(true)
        .with_pool("browser", 1);
    let principal = Principal::new("admin").with_roles(vec!["admin".into(), "editor".into()]);
    GovernContext::new(tenant, principal)
}

/// 多场景基准：用真实引擎量化核心收益，输出对齐表 + CSV
fn cmd_bench() -> anyhow::Result<()> {
    let rows = xuanji_expert::bench::run_benchmarks();
    println!("{}", xuanji_expert::bench::bench_table(&rows));
    let csv = xuanji_expert::bench::bench_csv(&rows);
    let path = std::env::temp_dir().join("xuanji_bench.csv");
    fs::write(&path, &csv)?;
    println!("\nCSV 已写出: {}", path.display());
    Ok(())
}

fn cmd_demo() -> anyhow::Result<()> {
    let g = gov_flow();
    let ctx = default_ctx();
    let rep = xuanji_optimize(&g, &ctx);

    println!("=== 七专家评分 ===");
    for (e, s) in &rep.expert_scores {
        println!("  {:>12} : {:.2}", e, s);
    }
    println!("=== 治理闸门 ===");
    println!("  状态: {:?}  批准: {}", rep.gate.status, rep.gate.approved);
    println!("  原因: {}", rep.gate.reason);
    println!("=== 优化收益 ===");
    println!("{}", rep.optimization.summary());

    // 可视化 DTO 校验（前端契约）
    let topo = demo_topology();
    let viz = to_viz(&rep, Some(&topo));
    println!("=== 可视化 DTO ===");
    println!(
        "  流程图节点: {}  边: {}",
        viz.flow_nodes.len(),
        viz.flow_edges.len()
    );
    println!(
        "  关系网实体: {}  关系: {}",
        viz.entities.len(),
        viz.relations.len()
    );
    println!("  关键路径: {:?}", viz.critical_path);
    println!("  复用路径: {:?}", viz.reuse_path);
    println!(
        "  冲突标红节点: {}",
        viz.flow_nodes
            .iter()
            .filter(|n| n.highlight == "conflict")
            .count()
    );
    println!(
        "  关键路径高亮节点: {}",
        viz.flow_nodes
            .iter()
            .filter(|n| n.highlight == "critical")
            .count()
    );
    Ok(())
}

fn cmd_optimize(args: &[String]) -> anyhow::Result<()> {
    let mut path: Option<String> = None;
    let mut out: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out = args.get(i).cloned();
            }
            other if !other.starts_with("--") && path.is_none() => path = Some(other.to_string()),
            _ => {}
        }
        i += 1;
    }
    let path = path.ok_or_else(|| anyhow::anyhow!("缺少 flow.json 路径"))?;
    let json = fs::read_to_string(&path)?;
    let g: FlowGraph = serde_json::from_str(&json)?;
    let ctx = default_ctx();
    let rep = xuanji_optimize(&g, &ctx);

    println!("{}", rep.optimization.summary());
    println!("治理: {:?} 批准={}", rep.gate.status, rep.gate.approved);

    if let Some(dir) = out {
        fs::create_dir_all(&dir)?;
        // 写报告
        fs::write(
            format!("{dir}/report.json"),
            serde_json::to_string_pretty(&rep.optimization)?,
        )?;
        // 写生成的代码
        if let Some(code) = &rep.optimization.code {
            for f in &code.files {
                let fp = format!("{dir}/{}", f.path);
                if let Some(parent) = std::path::Path::new(&fp).parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(fp, &f.content)?;
            }
        }
        // 写可视化 DTO
        let topo = demo_topology();
        let viz = to_viz(&rep, Some(&topo));
        fs::write(
            format!("{dir}/viz.json"),
            serde_json::to_string_pretty(&viz)?,
        )?;
        println!("已写出到 {dir}/ （report.json / 生成代码 / viz.json）");
    }
    Ok(())
}

fn cmd_verify(args: &[String]) -> anyhow::Result<()> {
    let path = args.iter().find(|a| !a.starts_with("--")).cloned();
    let path = path.ok_or_else(|| anyhow::anyhow!("缺少 flow.json 路径"))?;
    let json = fs::read_to_string(&path)?;
    let g: FlowGraph = serde_json::from_str(&json)?;
    let ctx = default_ctx();
    let rep = xuanji_optimize(&g, &ctx);
    let v = &rep.algo;
    println!("=== 璇玑验证网关（最高权限）===");
    for c in &v.checks {
        let mark = if c.passed { "OK" } else { "FAIL" };
        let lvl = if c.blocking { "阻断" } else { "告警" };
        println!("  [{}] {} ({}) - {}", mark, c.name, lvl, c.detail);
    }
    println!("{}", v.summary);
    println!("治理闸门: 批准={} 否决={}", rep.gate.approved, v.vetoed);
    if v.vetoed {
        std::process::exit(2);
    }
    Ok(())
}

fn cmd_serve(args: &[String]) -> anyhow::Result<()> {
    let mut port = 8080u16;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--port" {
            i += 1;
            if let Some(p) = args.get(i).and_then(|s| s.parse().ok()) {
                port = p;
            }
        }
        i += 1;
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let topo = demo_topology();
        let state = xuanji_expert::server::AppState {
            topo: std::sync::Arc::new(tokio::sync::Mutex::new(Some(topo))),
            live: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
            current_exec: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        };
        let app =
            xuanji_expert::server::router(state).layer(tower_http::cors::CorsLayer::permissive());
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("🌀 璇玑 智能中心已就绪: http://{addr}");
        println!("  前端:        http://{addr}/");
        println!("  API:         POST http://{addr}/api/optimize");
        axum::serve(listener, app).await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}
