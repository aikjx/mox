// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! PrimiFlow-Fusion 企业级服务入口
//!
//! 用法：
//! - `primiflow-fusion serve [--config path.json] [--addr 0.0.0.0:8080]`  启动 REST 服务
//! - `primiflow-fusion verify`                                             跑全局治理闸门（供 CI 门禁，通过退出 0，否则 1）

use std::process::exit;

use mox_flow_fusion_svc::config::Config;
use mox_flow_fusion_svc::observability;
use mox_flow_fusion_svc::platform::PrimiPlatform;
use mox_flow_fusion_svc::server::{new_state, serve};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("serve");

    match sub {
        "verify" => verify_cmd(),
        "serve" => serve_cmd(&args).await,
        other => {
            eprintln!("未知子命令：{other}（可选：serve / verify）");
            exit(2);
        }
    }
}

/// 解析 `--config <path>` 与 `--addr <addr>` 参数
fn parse_args(args: &[String]) -> (Option<String>, Option<String>) {
    let mut config = None;
    let mut addr = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--config" => {
                if i + 1 < args.len() {
                    config = Some(args[i + 1].clone());
                }
                i += 2;
            }
            "--addr" => {
                if i + 1 < args.len() {
                    addr = Some(args[i + 1].clone());
                }
                i += 2;
            }
            _ => i += 1,
        }
    }
    (config, addr)
}

/// `verify` 子命令：加载平台、跑全局治理闸门，把结果打印并以退出码反映（CI 门禁用）
fn verify_cmd() {
    let cfg = match Config::load(None) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置错误：{e}");
            exit(2);
        }
    };
    if let Err(e) = observability::init(&cfg.log_level, cfg.json_log) {
        eprintln!("可观测性初始化失败（非致命）：{e}");
    }

    let platform = match &cfg.persistence_path {
        Some(p) => PrimiPlatform::with_persistence(p.clone()),
        None => PrimiPlatform::new(),
    };
    let gate = platform.graph.full_gate();

    println!(
        "全局治理闸门（守恒 R07 / 六维零孤儿 A4 / GR-STD 8 闸门）：{}",
        if gate.passed {
            "✅ 通过"
        } else {
            "❌ 未通过"
        }
    );
    if !gate.conservation.passed {
        println!("  · 守恒残差错误：{:?}", gate.conservation.errors);
    }
    if !gate.binding.passed {
        println!("  · 六维绑定孤儿：{:?}", gate.binding.orphans);
    }
    if !gate.governance.passed {
        println!("  · 关图治理错误：{:?}", gate.governance.errors);
    }

    exit(if gate.passed { 0 } else { 1 });
}

/// `serve` 子命令：加载配置 → 初始化日志 → 确保文档目录 → 监听端口
async fn serve_cmd(args: &[String]) {
    let (config_path, addr_arg) = parse_args(args);
    let mut cfg = match Config::load(config_path.as_deref()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("配置错误：{e}");
            exit(2);
        }
    };
    if let Some(a) = addr_arg {
        cfg.bind_addr = a;
    }
    if let Err(e) = cfg.validate() {
        eprintln!("配置校验失败：{e}");
        exit(2);
    }
    if let Err(e) = observability::init(&cfg.log_level, cfg.json_log) {
        eprintln!("可观测性初始化失败（非致命）：{e}");
    }

    // 确保文档目录存在（PT-DOC 导出目标）
    if let Err(e) = std::fs::create_dir_all(&cfg.docs_dir) {
        eprintln!("无法创建 docs_dir {}：{e}", cfg.docs_dir.display());
        exit(2);
    }

    let state = new_state(cfg);
    let addr = state.config.bind_addr.clone();
    eprintln!("PrimiFlow-Fusion 启动：addr={addr}");
    if let Err(e) = serve(state, &addr).await {
        eprintln!("服务启动失败：{e}");
        exit(1);
    }
}
