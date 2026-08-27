// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 桌面小白助手 · 二进制入口
//!
//! ```bash
//! # 启动 :3717 服务（前台）
//! cargo run -p xiaobai-desktop
//!
//! # 一次性执行：文字端到端
//! cargo run -p xiaobai-desktop -- --once "音量状态"
//!
//! # REPL 交互模式（每一行当作语音文本）
//! cargo run -p xiaobai-desktop -- --repl --bind 127.0.0.1:3718
//! ```

use std::io::{self, BufRead, Write};
use std::time::Duration;

use clap::Parser;
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use mox_voice_desktop_app::{DesktopApp, WidgetMode};
use mox_voice_operator_svc::server_3717::VoiceServiceConfig;
use mox_voice_core_svc::identity::{OperatorIdentity, RoleTag};
use mox_voice_core_svc::rbac::DispatchMode;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "xiaobai-desktop",
    version,
    about = "🚀 桌面小白助手 · voice_proxy 3717 桥接 + BallWidget 5 状态机 + 端到端算子调度（FR-13/8 大类 Rust 版）",
    long_about = None,
)]
struct Args {
    /// voice_proxy 绑定地址（端口 3717 与 Python FastAPI 版对齐）
    #[arg(long, default_value = "127.0.0.1:3717")]
    bind: String,

    /// 默认用户 id（用于 RBAC L0~L3 校验）
    #[arg(long, default_value = "desktop_user_001")]
    user_id: String,

    /// 默认角色：auditor|member|expert|coordinator|mox_admin（大小写/中文宽松）
    #[arg(long, default_value = "member")]
    role: String,

    /// 三策略调度模式：LocalFirst / CloudFallback / CloudOnly
    #[arg(long, default_value = "LocalFirst")]
    mode: String,

    /// UI 显示模式（仅写日志字段用，Slint 实装 P2）
    #[arg(long, value_enum, default_value_t = CliWidgetMode::FloatingBall)]
    widget_mode: CliWidgetMode,

    /// 交互 REPL：每读一行当作语音文本，输出 JSON（适合本地调试）
    #[arg(long)]
    repl: bool,

    /// 一次性执行：文本输入 → 执行 → 打印 JSON → 退出
    #[arg(long)]
    once: Option<String>,

    /// 一次性 + REPL 均给 dispatch 加超时（秒），防止平台命令挂死
    #[arg(long, default_value_t = 20)]
    timeout_secs: u64,

    /// 启动时不 spawn :3717 HTTP 服务（纯进程内 in-process 模式）
    #[arg(long)]
    no_server: bool,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum CliWidgetMode {
    FloatingBall,
    TrayOnly,
    Sidebar,
}
impl From<CliWidgetMode> for WidgetMode {
    fn from(c: CliWidgetMode) -> Self {
        match c {
            CliWidgetMode::FloatingBall => WidgetMode::FloatingBall,
            CliWidgetMode::TrayOnly => WidgetMode::TrayOnly,
            CliWidgetMode::Sidebar => WidgetMode::Sidebar,
        }
    }
}

// ---------- helper: 宽松解析 DispatchMode ----------
fn parse_mode(s: &str) -> DispatchMode {
    match s.trim().to_ascii_lowercase().as_str() {
        "local" | "localfirst" | "local_first" => DispatchMode::LocalFirst,
        "cloud" | "cloudfallback" | "cloud_fallback" => DispatchMode::CloudFallback,
        "cloudonly" | "cloud_only" => DispatchMode::CloudOnly,
        _ => DispatchMode::LocalFirst,
    }
}

// ---------- helper: 打印 banner ----------
fn print_banner(args: &Args, addr: &Option<String>) {
    println!(
        r#"
╔══════════════════════════════════════════════════════════════╗
║   🚀  桌面小白助手  xiaobai-desktop  ·  Rust 版                ║
╠══════════════════════════════════════════════════════════════╣
║   协议:   AIS-FR13/V1.0  JSON 信封                           ║
║   算子:   App / File / Volume / Input × Network / Display    ║
║             Browser / Notify   (8 大类, 跨平台回退链)          ║
║   引擎:   OperatorEngine  ·  RBAC L0-L3  ·  PPR 激活扩散     ║
║   热词:   S1 ContextConfig  ·  S2 重建  ·  S3 Levenshtein    ║
╠══════════════════════════════════════════════════════════════╣
║   bind:   {:<48}  ║
║   user:   {:<48}  ║
║   role:   {:<48}  ║
║   mode:   {:<48}  ║
╚══════════════════════════════════════════════════════════════╝
"#,
        addr.as_deref().unwrap_or("<no-server>"),
        args.user_id,
        args.role,
        args.mode,
    );
}

// ---------- pretty print dispatch result ----------
fn pretty_result(text: &str, v: &serde_json::Value, elapsed_ms: u128) {
    let action = v
        .get("intent")
        .and_then(|i| i.get("action"))
        .and_then(|a| a.as_str())
        .unwrap_or("?");
    let cat = v
        .get("intent")
        .and_then(|i| i.get("category"))
        .and_then(|c| c.as_str())
        .unwrap_or("?");
    let level = v
        .get("intent")
        .and_then(|i| i.get("required_level"))
        .and_then(|l| l.as_u64())
        .unwrap_or(0);
    let exec_ok = v
        .get("execution")
        .and_then(|e| e.get("ok"))
        .and_then(|o| o.as_bool())
        .unwrap_or(false);
    let verdict = v
        .get("intent")
        .and_then(|i| i.get("verdict"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    println!(
        "  ✅ [{elapsed_ms:>5}ms]  {:?}  →  {}::{}  (L{level})  ok={exec_ok}  verdict={verdict}",
        text, cat, action
    );
    if let Some(output) = v.get("execution").and_then(|e| e.get("output")) {
        if !output.is_null() {
            let s = serde_json::to_string_pretty(output).unwrap_or_default();
            if s.lines().count() <= 16 {
                println!("{}", s.lines().map(|l| format!("     │ {l}")).collect::<Vec<_>>().join("\n"));
            } else {
                println!("     │ (output too long: {} lines, use --once --output=json 看完整 JSON)", s.lines().count());
            }
        }
    }
}

// ---------- small dispatcher ----------
async fn run_once(app: &DesktopApp, text: &str, timeout: Duration) -> anyhow::Result<()> {
    let t0 = std::time::Instant::now();
    match tokio::time::timeout(timeout, app.dispatch_local_text(text)).await {
        Ok(Ok(v)) => {
            pretty_result(text, &v, t0.elapsed().as_millis());
            Ok(())
        }
        Ok(Err(e)) => {
            let ms = t0.elapsed().as_millis();
            println!("  ❌ [{ms:>5}ms]  {:?}  →  XB 错误: {e:#}", text);
            Err(anyhow::anyhow!("{e}"))
        }
        Err(_) => {
            let ms = t0.elapsed().as_millis();
            println!("  ⏰ [{ms:>5}ms]  {:?}  →  超时 > {}s (已自动中断)", text, timeout.as_secs());
            Err(anyhow::anyhow!("timeout"))
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1) Tracing：RUST_LOG=info 默认
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,xiaobai_audit=info,warn")),
        )
        .with_target(false)
        .init();

    // 2) CLI
    let args = Args::parse();
    let mode = parse_mode(&args.mode);
    let role_tag = RoleTag::parse_loose(&args.role).unwrap_or(RoleTag::Member);
    let identity = OperatorIdentity::new(&args.user_id, role_tag, false);

    // 3) Build config / spawn :3717 server
    let mut app = DesktopApp::new();
    app.mode = WidgetMode::from(args.widget_mode);

    let server_addr = if args.no_server {
        None
    } else {
        let mut cfg = VoiceServiceConfig::default();
        cfg.bind = args.bind.parse()?;
        cfg.default_identity = identity.clone();
        cfg.default_mode = mode.clone();

        let addr = cfg.bind.to_string();
        // 后台线程 spawn
        match app.spawn_voice_server_background() {
            Ok(_) => {
                // 给 HTTP 服务 200ms 起起来（便于后续 --once REPL 若走 HTTP 也可连）
                tokio::time::sleep(Duration::from_millis(200)).await;
                Some(addr)
            }
            Err(e) => {
                tracing::warn!("voice_proxy :3717 启动失败，降级为 in-process 模式: {e}");
                None
            }
        }
    };

    print_banner(&args, &server_addr);

    // 4) Pre-flight：先测一次 health 级别的 in-process dispatch（保证 pipeline 通）
    tracing::info!(target: "xiaobai_cli", "preflight: list_registered_actions via XiaobaiVoiceService");
    {
        use mox_voice_operator_svc::server_3717::XiaobaiVoiceService;
        let pre_cfg = VoiceServiceConfig::default();
        match XiaobaiVoiceService::new(pre_cfg) {
            Ok(svc) => {
                let actions = svc.engine.list_registered_actions();
                tracing::info!(target: "xiaobai_cli", "已注册算子动作: {} 个 (覆盖 8 大类)", actions.len());
            }
            Err(e) => {
                tracing::warn!("preflight new XiaobaiVoiceService 失败: {e}");
            }
        }
    }

    let timeout = Duration::from_secs(args.timeout_secs);

    // 5) Execution mode
    if let Some(text) = args.once {
        run_once(&app, text.trim(), timeout).await.ok();
        return Ok(());
    }

    if args.repl {
        println!("💬  REPL 模式：每输入一行回车执行，空行退出。示例：音量调到 50 ｜ ping 百度 ｜ 列屏幕");
        let stdin = io::stdin();
        let mut lines = stdin.lock().lines();
        loop {
            print!("xiaobai> ");
            io::stdout().flush().ok();
            match lines.next() {
                Some(Ok(line)) => {
                    let t = line.trim().to_string();
                    if t.is_empty() {
                        println!("👋  退出");
                        break;
                    }
                    run_once(&app, &t, timeout).await.ok();
                }
                _ => break,
            }
        }
        return Ok(());
    }

    // 6) 默认：前台常驻（等待 Ctrl+C）—— 实际 Slint UI 实装 P2 阶段替换这里
    println!("✅  voice_proxy 已启动，HTTP 监听：{}", server_addr.as_deref().unwrap_or("<未启用>"));
    println!("   健康检查:   curl http://{}/health", server_addr.as_deref().unwrap_or("127.0.0.1:3717"));
    println!("   文本分发:   curl -X POST http://{}/v1/dispatch_text -H 'Content-Type: application/json' -d '{{\"text\":\"音量状态\"}}'",
             server_addr.as_deref().unwrap_or("127.0.0.1:3717"));
    println!("   Ctrl+C 退出");

    tokio::signal::ctrl_c().await?;
    println!("\n👋  退出");
    Ok(())
}
