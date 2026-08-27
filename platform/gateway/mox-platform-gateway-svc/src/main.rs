//! MOX 企业级单二进制入口（mox-server.exe）
//!
//! # 用法
//! ```powershell
//! # 默认 0.0.0.0:8080
//! cargo run -p mox-platform-gateway-svc
//! # 或自定义端口
//! ./target/release/mox-server --bind 127.0.0.1 --port 9000
//! ```
//!
//! Ctrl-C 优雅退出。

use mox_platform_gateway_svc::serve_forever;
use std::process::ExitCode;

fn parse_args() -> (String, u16) {
    let mut bind = "0.0.0.0".to_string();
    let mut port: u16 = 8080;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bind" | "-b" => {
                if let Some(v) = args.next() { bind = v; }
            }
            "--port" | "-p" => {
                if let Some(v) = args.next() {
                    if let Ok(n) = v.parse::<u16>() { port = n; }
                }
            }
            "--single-node" | "server" => { /* 兼容历史 CLI 子命令 */ }
            "-h" | "--help" => {
                println!("Usage: mox-server [--bind ADDR] [--port PORT]\n\
                          Default: 0.0.0.0:8080 (全面接管 backend-node 3000/3001/3002)");
                std::process::exit(0);
            }
            other => eprintln!("[mox-server] ⚠️  忽略未知参数: {other} (用 --help 查看用法)"),
        }
    }
    (bind, port)
}

fn main() -> ExitCode {
    let (bind, port) = parse_args();

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4))
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("mox-server: tokio runtime 构建失败: {e}");
            return ExitCode::from(1);
        }
    };

    rt.block_on(async move {
        match serve_forever(&bind, port).await {
            Ok(()) => ExitCode::from(0),
            Err(e) => {
                eprintln!("mox-server: 致命错误: {e}");
                ExitCode::from(1)
            }
        }
    })
}
