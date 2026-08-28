// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 桌面小白助手 · 二进制入口（Rust 全栈 · WebView2 3D 悬浮球）
//!
//! 形态：无边框透明置顶悬浮球（WebGL2 活体拓扑几何球）：
//!   - 点击悬浮球 → 展开对话面板，快速文本对话（8 类算子本地执行）
//!   - 按住拖动 → 移动悬浮球
//!   - 全局热键：Alt+X 录音开关（Listen 状态）/ Alt+Q 隐藏显示
//!   - 后台启动 voice_proxy :3717，供 Rust 网关 /voice/** 代理
//!
//! ```bash
//! # GUI（默认）
//! cargo run -p xiaobai-desktop
//! # 一次性执行：文字端到端（headless 调试）
//! cargo run -p xiaobai-desktop -- --once "音量状态"
//! # REPL 交互模式
//! cargo run -p xiaobai-desktop -- --repl
//! ```

use std::sync::Arc;

use clap::Parser;
use serde_json::json;
use tracing_subscriber::EnvFilter;

use mox_voice_desktop_app::{ball_widget::BallWidgetState, global_hotkeys::HotkeyAction};

use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
};

// =============================== 窗口尺寸 ===============================
const BALL_W: f64 = 150.0;
const BALL_H: f64 = 150.0;
const PANEL_W: f64 = 470.0;
const PANEL_H: f64 = 690.0;

// =============================== 用户事件 ===============================
enum UserEvent {
    /// JS → Rust 的原始 IPC 消息
    Ipc(String),
    /// 对话 dispatch 结果（JSON 字符串）
    ChatResult(String),
    /// 状态推送（BallWidgetState repr u8）
    SetState(u8),
    /// 热键：录音开关
    ToggleRecord,
    /// 热键：隐藏/显示悬浮球
    ToggleWidget,
}

// =============================== CLI ===============================
#[derive(Debug, Clone, Parser)]
#[command(
    name = "xiaobai-desktop",
    version,
    about = "🚀 桌面小白助手 · Rust 全栈 3D 悬浮球 + voice_proxy 3717 + 8 大类算子（FR-13）",
)]
struct Args {
    /// 默认用户 id（RBAC L0~L3）
    #[arg(long, default_value = "desktop_user_001")]
    user_id: String,
    /// 默认角色
    #[arg(long, default_value = "member")]
    role: String,
    /// 一次性执行：文本 → 执行 → 打印 JSON → 退出（headless）
    #[arg(long)]
    once: Option<String>,
    /// REPL 交互模式（headless）
    #[arg(long)]
    repl: bool,
    /// 不 spawn :3717 HTTP 服务
    #[arg(long)]
    no_server: bool,
}

// =============================== 对话分发（线程内） ===============================
/// 在独立线程执行一次文本分发，关键节点向主线程推送状态。
fn run_dispatch(
    text: String,
    proxy: EventLoopProxy<UserEvent>,
    state: Arc<mox_voice_desktop_app::ball_widget::StateController>,
) {
    std::thread::spawn(move || {
        state.transition(BallWidgetState::Think);
        let _ = proxy.send_event(UserEvent::SetState(BallWidgetState::Think as u8));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let json = match rt {
            Ok(rt) => {
                let fut = async {
                    use mox_voice_operator_svc::server_3717::{VoiceServiceConfig, XiaobaiVoiceService};
                    let svc = XiaobaiVoiceService::new(VoiceServiceConfig::default())?;
                    svc.dispatch_text(&text, None, None).await
                };
                match rt.block_on(fut) {
                    Ok(v) => {
                        let exec_ok = v
                            .get("execution")
                            .and_then(|e| e.get("ok"))
                            .and_then(|o| o.as_bool())
                            .unwrap_or(false);
                        if exec_ok {
                            state.transition(BallWidgetState::Executing);
                            let _ = proxy.send_event(UserEvent::SetState(BallWidgetState::Executing as u8));
                        }
                        v.to_string()
                    }
                    Err(e) => json!({ "error": { "message": e.to_string() } }).to_string(),
                }
            }
            Err(e) => json!({ "error": { "message": format!("tokio 初始化失败: {e}") } }).to_string(),
        };
        let _ = proxy.send_event(UserEvent::ChatResult(json));
        state.transition(BallWidgetState::Idle);
        let _ = proxy.send_event(UserEvent::SetState(BallWidgetState::Idle as u8));
    });
}

// =============================== GUI 主入口 ===============================
fn run_gui(args: Args) -> anyhow::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // ---- 悬浮球窗口：无边框 · 透明 · 置顶 ----
    let mut wb = tao::window::WindowBuilder::new()
        .with_title("小白 · 璇玑伙伴")
        .with_inner_size(LogicalSize::new(BALL_W, BALL_H))
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top(true)
        .with_resizable(false);
    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowBuilderExtWindows;
        wb = wb.with_undecorated_shadow(false);
    }
    let window = Arc::new(wb.build(&event_loop)?);

    // ---- WebView：加载 3D 悬浮球 UI ----
    let html = include_str!("../ui/index.html");
    let ipc_proxy = proxy.clone();
    let builder = wry::WebViewBuilder::new()
        .with_transparent(true)
        .with_html(html)
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let body = req.body().clone();
            let _ = ipc_proxy.send_event(UserEvent::Ipc(body));
        });
    let webview = builder.build(&window)?;

    // ---- 全局热键 ----
    let state = Arc::new(mox_voice_desktop_app::ball_widget::StateController::new());
    let hk_manager = global_hotkey::GlobalHotKeyManager::new().ok();
    let mut hotkeys: Vec<(global_hotkey::hotkey::HotKey, HotkeyAction)> = Vec::new();
    if let Some(mgr) = &hk_manager {
        use global_hotkey::hotkey::HotKey;
        let binds = mox_voice_desktop_app::global_hotkeys::HotkeyBindings::with_defaults();
        for (action, combo) in &binds.bindings {
            let parsed = parse_combo(combo);
            if let Some((code, mods)) = parsed {
                let hk = HotKey::new(Some(mods), code);
                if mgr.register(hk).is_ok() {
                    hotkeys.push((hk, *action));
                }
            }
        }
    }

    // 初始推送状态
    let _ = webview.evaluate_script("window.__setState(0)");

    // ---- 事件循环 ----
    let mut widget_visible = true;

    event_loop.run(move |event, _el, control_flow| {
        *control_flow = ControlFlow::Wait;

        // 全局热键事件
        if let Ok(ev) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if ev.state() == global_hotkey::HotKeyState::Pressed {
                for (hk, action) in &hotkeys {
                    if hk.id() == ev.id() {
                        match action {
                            HotkeyAction::ToggleRecord => {
                                let _ = proxy.send_event(UserEvent::ToggleRecord);
                            }
                            HotkeyAction::ToggleWidgetVisible => {
                                let _ = proxy.send_event(UserEvent::ToggleWidget);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::UserEvent(ev) => match ev {
                UserEvent::Ipc(msg) => {
                    if let Some(rest) = msg.strip_prefix("chat:") {
                        run_dispatch(rest.to_string(), proxy.clone(), state.clone());
                    } else {
                        match msg.as_str() {
                            "open-panel" => {
                                let _ = window.set_inner_size(LogicalSize::new(PANEL_W, PANEL_H));
                            }
                            "close-panel" => {
                                let _ = window.set_inner_size(LogicalSize::new(BALL_W, BALL_H));
                            }
                            "drag" => {
                                let _ = window.drag_window();
                            }
                            "ready" => {
                                let _ = webview
                                    .evaluate_script("window.__setState(0)");
                            }
                            _ => {}
                        }
                    }
                }
                UserEvent::ChatResult(json_str) => {
                    let _ = webview
                        .evaluate_script(&format!("window.__chatResponse({json_str:?})"));
                }
                UserEvent::SetState(n) => {
                    let _ = webview.evaluate_script(&format!("window.__setState({n})"));
                }
                UserEvent::ToggleRecord => {
                    // P2：接入真实录音 + ASR。当前先切 Listen 状态并提示。
                    state.transition(BallWidgetState::Listen);
                    let _ = webview.evaluate_script("window.__setState(1)");
                    let _ = webview.evaluate_script("window.__toast('🎙 正在录音…(ASR 接入 P2)')");
                    let st = state.clone();
                    let px = proxy.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1600));
                        st.transition(BallWidgetState::Idle);
                        let _ = px.send_event(UserEvent::SetState(0));
                    });
                }
                UserEvent::ToggleWidget => {
                    widget_visible = !widget_visible;
                    window.set_visible(widget_visible);
                }
            },

            _ => {}
        }
    });
}

// =============================== 热键组合解析 ===============================
fn parse_combo(
    combo: &str,
) -> Option<(
    global_hotkey::hotkey::Code,
    global_hotkey::hotkey::Modifiers,
)> {
    use global_hotkey::hotkey::{Code, Modifiers};
    let parts: Vec<&str> = combo.split('+').map(|s| s.trim()).collect();
    let mut mods = Modifiers::empty();
    let mut code = None;
    for p in parts {
        match p.to_ascii_lowercase().as_str() {
            "alt" => mods.insert(Modifiers::ALT),
            "ctrl" | "control" => mods.insert(Modifiers::CONTROL),
            "shift" => mods.insert(Modifiers::SHIFT),
            "meta" | "win" | "super" => mods.insert(Modifiers::META),
            "x" => code = Some(Code::KeyX),
            "q" => code = Some(Code::KeyQ),
            "s" => code = Some(Code::KeyS),
            _ => {}
        }
    }
    code.map(|c| (c, mods))
}

// =============================== headless 调试 ===============================
async fn run_once_debug(text: &str) -> Result<serde_json::Value, mox_voice_core_svc::errors::XiaobaiError> {
    use mox_voice_operator_svc::server_3717::{VoiceServiceConfig, XiaobaiVoiceService};
    let svc = XiaobaiVoiceService::new(VoiceServiceConfig::default())?;
    svc.dispatch_text(text, None, None).await
}

// =============================== main ===============================
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,xiaobai_audit=info,warn")),
        )
        .with_target(false)
        .init();

    let args = Args::parse();

    // headless 调试模式
    if let Some(text) = args.once {
        let t0 = std::time::Instant::now();
        match run_once_debug(&text).await {
            Ok(v) => {
                println!("{}", serde_json::to_string_pretty(&v)?);
                println!("elapsed: {:?}", t0.elapsed());
            }
            Err(e) => eprintln!("XB 错误: {e:#}"),
        }
        return Ok(());
    }
    if args.repl {
        println!("💬 REPL 模式（headless）：输入一行执行，空行退出。");
        use std::io::BufRead;
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let l = line?.trim().to_string();
            if l.is_empty() {
                break;
            }
            match run_once_debug(&l).await {
                Ok(v) => println!("{}", v),
                Err(e) => eprintln!("XB 错误: {e:#}"),
            }
        }
        return Ok(());
    }

    // GUI 默认：先用主 tokio runtime 后台启动 voice_proxy :3717（供 Rust 网关 /voice/** 代理）
    if !args.no_server {
        let cfg = mox_voice_operator_svc::server_3717::VoiceServiceConfig::default();
        let addr = cfg.bind.to_string();
        tokio::spawn(async move {
            if let Err(e) = mox_voice_operator_svc::server_3717::serve(cfg).await {
                eprintln!("voice 3717 服务异常退出: {e:#}");
            }
        });
        tracing::info!(target: "xiaobai_cli", "voice_proxy :3717 后台启动 → {addr}");
    }

    run_gui(args)
}
