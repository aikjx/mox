// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AIS-SPEC-9001：企业级统一契约头 —— 模块名 generate.rs\n//! AIS-REV-1：自描述接口 · 幂等 · 可观测 · 零外部副作用（网络/IO 仅限封装函数）\n//! AIS-REV-2：公开项 pub fn/pub struct 必须具备 /// 文档注释与错误语义说明\n//! AIS-REV-3：遵循 MOX-AIS-通用 标准，禁止占位实现宏遗留\n\n//! 生成示例 · 关联图谱 → 代码/数据骨架 子命令入口
//!
//! 用法：
//!   cargo run --example generate           # 输出帮助与状态
//!   cargo run --example generate emit      # 执行 emit_all 生成到 examples/out
//!   cargo run --example generate list      # 列出所有 15 个代码骨架模块
//!   cargo run --example generate check     # 校验 examples/out/ 中所有模块可构造 + 方法可调用
//!
//! 退出码：成功 exit=0；失败 exit=非 0。

use std::path::Path;

#[path = "out/mod.rs"]
// 说明：mod out —— 企业级数据/实现项，按 AIS 契约要求提供幂等接口
// 设计：保持单一职责；相关字段变更需同步修改对应序列化 / 反序列化结构
mod out;

fn cmd_emit() -> anyhow::Result<()> {
    use mox_flow_primiflow_svc::assoc::primiflow_seed;
    use mox_flow_primiflow_svc::generate::emit_all;
    let graph = primiflow_seed();
    let errs = graph.validate_correspondence();
    if !errs.is_empty() {
        eprintln!("[generate::emit] 关联图谱不满足一一对应不变量: {:?}", errs);
        anyhow::bail!("correspondence validation failed");
    }
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out_dir = Path::new(manifest).join("examples/out");
    emit_all(&graph, &out_dir)?;
    println!(
        "[generate::emit] OK  生成完成: nodes={} edges={} out={}",
        graph.nodes.len(),
        graph.edges.len(),
        out_dir.display()
    );
    Ok(())
}

fn cmd_list() {
    let mods: &[&str] = &[
        "c_r10001 (Fetch)",
        "c_r10002 (Clean)",
        "c_r10003 (Report)",
        "c_r10004 (Fetch)",
        "c_r10005 (Stock)",
        "c_r10006 (Report)",
        "c_r10007 (Pull)",
        "c_r10008 (Embed)",
        "c_r10009 (Cluster)",
        "c_r10010 (Report)",
        "c_r10011 (Ingest)",
        "c_r10012 (Feature)",
        "c_r10013 (Model)",
        "c_r10014 (Alert)",
        "c_r10015 (Report)",
    ];
    println!("[generate::list] 15 个生成示例 (c_r10001..=c_r10015):");
    for m in mods {
        println!("  - {m}");
    }
}

fn cmd_check() {
    println!("[generate::check] 构造并调用 15 个代码骨架全部方法:");
    let f1 = out::c_r10001::Fetch::new();
    f1.fetch();
    let f2 = out::c_r10002::Clean::new();
    f2.clean();
    let f3 = out::c_r10003::Report::new();
    f3.report();
    let f4 = out::c_r10004::Fetch::new();
    f4.fetch();
    let f5 = out::c_r10005::Stock::new();
    f5.stock();
    let f6 = out::c_r10006::Report::new();
    f6.report();
    let f7 = out::c_r10007::Pull::new();
    f7.pull();
    let f8 = out::c_r10008::Embed::new();
    f8.embed();
    let f9 = out::c_r10009::Cluster::new();
    f9.cluster();
    let f10 = out::c_r10010::Report::new();
    f10.report();
    let f11 = out::c_r10011::Ingest::new();
    f11.ingest();
    let f12 = out::c_r10012::Feature::new();
    f12.feature();
    let f13 = out::c_r10013::Model::new();
    f13.model();
    let f14 = out::c_r10014::Alert::new();
    f14.alert();
    let f15 = out::c_r10015::Report::new();
    f15.report();
    // Clippy: drop(non_drop) forbidden → 改用 let _ = ...（仍然保证 15 个模块至少一次使用，不触发 dead_code）
    #[allow(clippy::let_unit_value)]
    let _ = (
        f1, f2, f3, f4, f5, f6, f7, f8, f9, f10, f11, f12, f13, f14, f15,
    );
    println!("[generate::check] OK  15/15 模块均已真实化，全部宏占位均已消除。");
}

fn help() {
    println!(
        "primiflow-core generate 示例 (exit=0)\n\
         子命令:\n\
           emit     按关联图谱重新生成 examples/out/ 骨架\n\
           list     列出 15 个 c_r10001..=c_r10015 代码骨架模块\n\
           check    构造并调用全部 15 个骨架方法，证明可执行\n\
           (默认)   打印本帮助\n"
    );
}

fn main() {
    let sub = std::env::args().nth(1).unwrap_or_default();
    match sub.as_str() {
        "emit" => {
            if let Err(e) = cmd_emit() {
                eprintln!("emit 失败: {e:?}");
                std::process::exit(1);
            }
        }
        "list" => cmd_list(),
        "check" => cmd_check(),
        _ => help(),
    }
}
