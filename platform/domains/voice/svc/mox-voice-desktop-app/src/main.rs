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
//!   - 后台启动 voice_proxy :30010，供 Rust 网关 /voice/** 代理
//!
//! P2 语音闭环（全离线）：
//!   - Alt+X 真录音（cpal 16k）→ Paraformer ASR（sherpa-onnx）→ dispatch_text → Kokoro TTS 朗读
//!   - `--tts "文本"` 合成 WAV；`--vtest` 录音→ASR 链路测试（headless）
//!
//! ```bash
//! # GUI（默认）
//! cargo run -p mox-voice-desktop-app
//! # 一次性执行：文字端到端（headless 调试）
//! cargo run -p mox-voice-desktop-app -- --once "音量状态"
//! # TTS 合成（headless）
//! cargo run -p mox-voice-desktop-app -- --tts "你好，我是小白"
//! # 录音 → ASR 链路（headless，需麦克风）
//! cargo run -p mox-voice-desktop-app -- --vtest --vsecs 3
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
    /// 悬浮球 toast 提示
    Toast(String),
    /// 本地 TTS 不可用/失败时，触发浏览器拟人音兜底朗读（speechSynthesis）
    SpeakFallback(String),
}

// =============================== CLI ===============================
#[derive(Debug, Clone, Parser)]
#[command(
    name = "xiaobai-desktop",
    version,
    about = "🚀 桌面小白助手 · Rust 全栈 3D 悬浮球 + voice_proxy 30010 + 8 大类算子 + 语音闭环（Paraformer ASR / Kokoro TTS）",
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
    /// 语音 TTS 合成：合成文本为 WAV 输出到 --out（headless）
    #[arg(long)]
    tts: Option<String>,
    /// 语音链路测试：录音 N 秒 → ASR 转写 → 打印文本（headless）
    #[arg(long)]
    vtest: bool,
    /// ASR 文件测试：读 WAV → 16k → 识别 → 打印文本（headless，无麦克风验证）
    #[arg(long)]
    asrfile: Option<String>,
    /// --vtest 录音时长（秒）
    #[arg(long, default_value_t = 3.0)]
    vsecs: f64,
    /// 输出 WAV 路径（--tts / --vtest 用）
    #[arg(long, default_value = "voice_out.wav")]
    out: String,
    /// 诊断：在独立线程内做一次 TTS 合成（模拟 30010 跨线程调用）
    #[arg(long)]
    tts_thread: Option<String>,
    /// 不 spawn :30010 HTTP 服务
    #[arg(long)]
    no_server: bool,
}

/// 形象模型切换命令解析：`切换模型 X / 切换成 X / 变成 X / 换装 X / 换形象 X / 使用 X`。
/// 命中返回新形象模型（已切换），否则 None（交给算子 dispatch）。
fn try_switch_avatar(
    text: &str,
    avatars: &Arc<mox_voice_operator_svc::avatar::AvatarRegistry>,
) -> Option<mox_voice_operator_svc::avatar::Avatar> {
    let t = text.trim();
    let prefixes = [
        "切换模型", "切换为", "切换成", "切换到", "切换", "变成", "换成", "换装", "换形象", "换肤", "使用",
    ];
    let target = prefixes
        .iter()
        .find_map(|p| {
            if let Some(rest) = t.strip_prefix(p) {
                let r = rest
                    .trim()
                    .trim_matches(['"', '“', '”', '「', '」', '《', '》', ' ', '。', '！', '？', '，', '.']);
                if !r.is_empty() {
                    return Some(r.to_string());
                }
            }
            None
        })
        .unwrap_or_default();
    if target.is_empty() {
        return None;
    }
    // 精确 / 包含匹配 id 或 name
    for meta in avatars.list() {
        if meta.id == target
            || meta.name == target
            || meta.name.contains(&target)
            || target.contains(&meta.id)
        {
            if avatars.switch(&meta.id).is_ok() {
                return Some(avatars.current().clone());
            }
        }
    }
    None
}

// =============================== 对话分发（线程内） ===============================
/// 从 dispatch_text 的 JSON 结果中提取可朗读文本。
fn extract_speak_text(v: &serde_json::Value) -> String {
    // 1) execution.output（可能是 string / {output|text} / array）
    if let Some(exec) = v.get("execution") {
        if let Some(out) = exec.get("output") {
            if let Some(s) = out.as_str() {
                if !s.trim().is_empty() {
                    return s.trim().to_string();
                }
            }
            // 对象：取 output / text / message
            if let Some(s) = out
                .get("output")
                .or_else(|| out.get("text"))
                .and_then(|x| x.as_str())
            {
                if !s.trim().is_empty() {
                    return s.trim().to_string();
                }
            }
        }
    }
    // 2) error.message
    if let Some(msg) = v.pointer("/error/message").and_then(|m| m.as_str()) {
        if !msg.trim().is_empty() {
            return msg.trim().to_string();
        }
    }
    // 3) 顶层 message / text
    if let Some(s) = v.get("message").or_else(|| v.get("text")).and_then(|x| x.as_str()) {
        if !s.trim().is_empty() {
            return s.trim().to_string();
        }
    }
    v.to_string()
}

/// headless：文本 → dispatch → 提取回答 → TTS 合成 WAV
async fn run_tts_pipeline(
    text: String,
    out_path: &str,
) -> anyhow::Result<()> {
    let models = mox_voice_desktop_app::voice_engine::locate_models_dir()
        .ok_or_else(|| anyhow::anyhow!("未找到语音模型目录（models/voice）。"))?;
    let engine = mox_voice_desktop_app::VoiceEngine::new(&models)?;
    tracing::info!(target: "xiaobai_voice", "模型就绪：ASR+TTS，说话人数 {}", engine.speaker_count());

    use mox_voice_operator_svc::voice_server::{VoiceServiceConfig, XiaobaiVoiceService};
    let svc = XiaobaiVoiceService::new(VoiceServiceConfig::default())?;
    let answer = match svc.dispatch_text(&text, None, None).await {
        Ok(v) => extract_speak_text(&v),
        Err(e) => {
            // dispatch 兜底：无法识别/执行时朗读用户输入本身（保证 TTS 链路可用）
            tracing::warn!(target: "xiaobai_voice", "dispatch 失败，朗读原文本兜底: {e}");
            text.clone()
        }
    };

    let t0 = std::time::Instant::now();
    let sr = engine.synthesize_to_wav(&answer, std::path::Path::new(out_path))?;
    tracing::info!(target: "xiaobai_voice", "TTS 合成完成：{out_path}（耗时 {}s）", t0.elapsed().as_secs_f32());
    println!("{}", serde_json::json!({
        "input_text": text,
        "answer": answer,
        "wav": out_path,
        "sample_rate": sr,
        "elapsed_s": t0.elapsed().as_secs_f32(),
    }));
    Ok(())
}

/// headless：录音 N 秒 → ASR 转写 → 打印文本（可选合成）
fn run_voice_test(secs: f64, out_path: &str) -> anyhow::Result<()> {
    let models = mox_voice_desktop_app::voice_engine::locate_models_dir()
        .ok_or_else(|| anyhow::anyhow!("未找到语音模型目录（models/voice）。"))?;
    let engine = mox_voice_desktop_app::VoiceEngine::new(&models)?;

    println!("🎙 开始录音 {secs}s …（请对着麦克风说话）");
    let rec = mox_voice_desktop_app::Recorder::start()?;
    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
    let pcm = rec.stop(16000);
    let peak = mox_voice_desktop_app::voice_engine::peak_level(&pcm);
    println!("录音结束：{} 样本，峰值 {:.3}", pcm.len(), peak);
    if mox_voice_desktop_app::voice_engine::is_silent(&pcm, 0.01) {
        anyhow::bail!("录音基本为静音（峰值 {peak:.3} < 0.01），请检查麦克风");
    }

    let t0 = std::time::Instant::now();
    let text = engine.recognize(&pcm);
    println!("[ASR] 识别结果（{}ms）：{:?}", t0.elapsed().as_millis(), text);
    if text.is_empty() {
        anyhow::bail!("ASR 未识别到文本");
    }

    // 合成测试（读回识别文本，验证 TTS 闭环）
    let sr = engine.synthesize_to_wav(&text, std::path::Path::new(out_path))?;
    println!("[TTS] 已合成 {out_path}（sr={sr}）");
    println!("{}", serde_json::json!({
        "asr_text": text,
        "peak": peak,
        "wav": out_path,
        "sample_rate": sr,
        "asr_elapsed_ms": t0.elapsed().as_millis(),
    }));
    Ok(())
}

/// 在独立线程执行一次文本分发，关键节点向主线程推送状态。
/// 完成后对回答做第一层本地 TTS 朗读（套用形象模型性格措辞）；失败/无权重则回退浏览器拟人音兜底。
fn run_dispatch(
    text: String,
    engine: Option<Arc<mox_voice_desktop_app::VoiceEngine>>,
    avatars: Arc<mox_voice_operator_svc::avatar::AvatarRegistry>,
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
                    use mox_voice_operator_svc::voice_server::{VoiceServiceConfig, XiaobaiVoiceService};
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
        let _ = proxy.send_event(UserEvent::ChatResult(json.clone()));

        // ---- 回答朗读：第一层本地 TTS，失败/无权重 → 浏览器拟人音兜底 ----
        let speak_text = extract_speak_text(&serde_json::from_str::<serde_json::Value>(&json).unwrap_or_default());
        if !speak_text.trim().is_empty() {
            let decorated = avatars.decorate(&speak_text); // 性格措辞（前缀/后缀/语气）
            state.transition(BallWidgetState::Speak);
            let _ = proxy.send_event(UserEvent::SetState(BallWidgetState::Speak as u8));
            let ok = match &engine {
                Some(eng) => match eng.speak(&decorated) {
                    Ok(()) => true,
                    Err(e) => {
                        tracing::warn!(target: "xiaobai_voice", "本地 TTS 朗读失败，切浏览器兜底: {e:#}");
                        false
                    }
                },
                None => {
                    tracing::warn!(target: "xiaobai_voice", "语音权重缺失，走浏览器拟人音兜底");
                    false
                }
            };
            if !ok {
                let _ = proxy.send_event(UserEvent::SpeakFallback(decorated));
            }
        }

        state.transition(BallWidgetState::Idle);
        let _ = proxy.send_event(UserEvent::SetState(BallWidgetState::Idle as u8));
    });
}

/// 语音闭环：pcm → Paraformer ASR → dispatch_text → 回答 → Kokoro TTS → 播放。
/// 后台线程执行，状态通过 proxy 回传主线程。
fn run_voice_loop(
    engine: Arc<mox_voice_desktop_app::VoiceEngine>,
    pcm: Vec<i16>,
    avatars: Arc<mox_voice_operator_svc::avatar::AvatarRegistry>,
    proxy: EventLoopProxy<UserEvent>,
    state: Arc<mox_voice_desktop_app::ball_widget::StateController>,
) {
    std::thread::spawn(move || {
        // ---- 1. ASR ----
        state.transition(BallWidgetState::Think);
        let _ = proxy.send_event(UserEvent::SetState(BallWidgetState::Think as u8));
        let text = engine.recognize(&pcm);
        if text.trim().is_empty() {
            state.transition(BallWidgetState::Idle);
            let _ = proxy.send_event(UserEvent::SetState(0));
            let _ = proxy.send_event(UserEvent::Toast("未识别到语音，请再试一次".into()));
            return;
        }

        // ---- 2. dispatch（本地算子执行） ----
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let json = match rt {
            Ok(rt) => {
                let fut = async {
                    use mox_voice_operator_svc::voice_server::{VoiceServiceConfig, XiaobaiVoiceService};
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
                        v
                    }
                    Err(e) => json!({ "error": { "message": e.to_string() } }),
                }
            }
            Err(e) => json!({ "error": { "message": format!("tokio 初始化失败: {e}") } }),
        };

        let speak_text = extract_speak_text(&json);
        let _ = proxy.send_event(UserEvent::ChatResult(json.to_string()));

        // ---- 3. TTS 朗读（第一层本地；失败 → 浏览器拟人音兜底；套性格措辞）----
        if !speak_text.trim().is_empty() {
            let decorated = avatars.decorate(&speak_text);
            state.transition(BallWidgetState::Speak);
            let _ = proxy.send_event(UserEvent::SetState(BallWidgetState::Speak as u8));
            if let Err(e) = engine.speak(&decorated) {
                tracing::warn!(target: "xiaobai_voice", "本地 TTS 朗读失败，切浏览器兜底: {e:#}");
                let _ = proxy.send_event(UserEvent::SpeakFallback(decorated));
            }
        }

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

    // ---- P2 语音引擎（启动时加载一次；模型缺失则录音时提示）----
    let engine = match mox_voice_desktop_app::voice_engine::locate_models_dir() {
        Some(dir) => match mox_voice_desktop_app::VoiceEngine::new(&dir) {
            Ok(e) => {
                tracing::info!(target: "xiaobai_voice", "语音引擎就绪（模型: {dir:?}，说话人 {}）", e.speaker_count());
                Some(Arc::new(e))
            }
            Err(e) => {
                tracing::warn!(target: "xiaobai_voice", "语音引擎初始化失败: {e:#}（Alt+X 将回退旧占位）");
                let _ = webview.evaluate_script("window.__toast('语音引擎未加载，Alt+X 暂不可用')");
                None
            }
        },
        None => {
            tracing::warn!(target: "xiaobai_voice", "未找到语音模型目录，语音闭环禁用");
            let _ = webview.evaluate_script("window.__toast('未找到语音模型，Alt+X 暂不可用')");
            None
        }
    };

    // ---- 形象模型注册表（AIS-FR14：视觉/语音/性格 三维一体，可无限切换）----
    let avatars = Arc::new(
        mox_voice_operator_svc::avatar::AvatarRegistry::load_dir(
            &mox_voice_operator_svc::avatar::locate_avatar_dir(),
        )
        .expect("形象模型注册表加载失败"),
    );
    // 语音通道：应用当前模型音色
    if let Some(eng) = &engine {
        avatars.apply_voice(eng);
    }
    tracing::info!(target: "xiaobai_avatar", "形象模型注册表就绪（{} 个），当前: {}",
        avatars.list().len(), avatars.current().name);
    // 视觉通道：把当前形象参数推给 UI
    let cur_visual = serde_json::to_string(&avatars.current().visual).unwrap_or_else(|_| "{}".into());
    let _ = webview.evaluate_script(&format!("window.__applyAvatar({cur_visual})"));

    // ---- voice_proxy :30010 后台启动（供 Rust 网关 /voice/** 代理，注入语音引擎）----
    if !args.no_server {
        let cfg = mox_voice_operator_svc::voice_server::VoiceServiceConfig::default();
        let addr = cfg.bind.to_string();
        let addr_cb = addr.clone();
        let engine_for_voice = engine.clone();
        std::thread::Builder::new()
            .name("voice-server".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("tokio runtime 创建失败");
                rt.block_on(async move {
                    use mox_voice_operator_svc::voice_server::{build_router, VoiceServiceConfig, XiaobaiVoiceService};
                    let svc = Arc::new(XiaobaiVoiceService::new(cfg.clone()).expect("XiaobaiVoiceService 初始化失败"));
                    if let Some(ve) = &engine_for_voice {
                        svc.attach_voice(ve.clone());
                        tracing::info!(target: "xiaobai_voice", "30010 已挂载语音引擎（/v1/tts、/v1/asr 可用）");
                    }
                    let app = build_router(svc);
                    let listener = tokio::net::TcpListener::bind(cfg.bind).await
                        .expect("30010 bind 失败");
                    tracing::info!(target: "xiaobai_cli", service = "voice-desktop", addr = %cfg.bind, "TCP listener bound");
                    tracing::info!(target: "xiaobai_cli", "voice_proxy :30010 后台启动 → {addr_cb}");
                    if let Err(e) = axum::serve(
                        listener,
                        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                    )
                    .with_graceful_shutdown(async {
                        let _ = tokio::signal::ctrl_c().await;
                        tracing::info!(target: "xiaobai_cli", "voice-desktop shutdown signal received");
                    })
                    .await
                    {
                        eprintln!("voice 30010 服务异常退出: {e:#}");
                    }
                });
            })?;
        tracing::info!(target: "xiaobai_cli", "voice_proxy :30010 线程已派发 → {addr}");
    }

    // ---- 事件循环 ----
    let mut widget_visible = true;
    let mut recording: Option<mox_voice_desktop_app::Recorder> = None;

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
                        let text = rest.to_string();
                        // 形象模型切换命令（全维联动：语音 + 性格 + 视觉）
                        if let Some(avatar) = try_switch_avatar(&text, &avatars) {
                            if let Some(eng) = &engine {
                                avatars.apply_voice(eng);
                            }
                            let visual = serde_json::to_string(&avatar.visual).unwrap_or_else(|_| "{}".into());
                            let _ = webview.evaluate_script(&format!("window.__applyAvatar({visual})"));
                            let out = format!(
                                "已切换为「{}」· {}\n语音音色 sid={} · 性格「{}」",
                                avatar.name, avatar.desc, avatar.voice.tts_sid, avatar.persona.style
                            );
                            let resp = serde_json::json!({
                                "intent": { "action": "switch_avatar", "category": "Avatar", "verdict": "avatar_switched" },
                                "execution": { "ok": true, "output": out },
                                "avatar": avatar
                            });
                            let _ = webview.evaluate_script(&format!(
                                "window.__chatResponse({:?})",
                                resp.to_string()
                            ));
                            // 朗读问候（性格措辞）
                            if !avatar.persona.greeting.trim().is_empty() {
                                match &engine {
                                    Some(eng) => {
                                        let _ = eng.speak(&avatar.persona.greeting);
                                    }
                                    None => {
                                        let _ = proxy
                                            .send_event(UserEvent::SpeakFallback(avatar.persona.greeting.clone()));
                                    }
                                }
                            }
                        } else {
                            run_dispatch(text, engine.clone(), avatars.clone(), proxy.clone(), state.clone());
                        }
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
                    // P2 真实闭环：首按开始录音（Listen），再按停止 → ASR → dispatch → TTS
                    if recording.is_none() {
                        match mox_voice_desktop_app::Recorder::start() {
                            Ok(rec) => {
                                recording = Some(rec);
                                state.transition(BallWidgetState::Listen);
                                let _ = webview.evaluate_script("window.__setState(1)");
                                let _ = webview.evaluate_script("window.__toast('🎙 录音中… 再按 Alt+X 结束')");
                            }
                            Err(e) => {
                                let _ = webview
                                    .evaluate_script(&format!("window.__toast('录音设备不可用: {e}')"));
                            }
                        }
                    } else {
                        let rec = recording.take().unwrap();
                        let pcm = rec.stop(16000);
                        let peak = mox_voice_desktop_app::voice_engine::peak_level(&pcm);
                        if pcm.is_empty() || mox_voice_desktop_app::voice_engine::is_silent(&pcm, 0.01) {
                            state.transition(BallWidgetState::Idle);
                            let _ = webview.evaluate_script("window.__setState(0)");
                            let _ = webview.evaluate_script(&format!(
                                "window.__toast('未听到声音（峰值 {peak:.2}），请再试')"
                            ));
                        }
                        match &engine {
                            Some(eng) => {
                                let eng = eng.clone();
                                run_voice_loop(eng, pcm, avatars.clone(), proxy.clone(), state.clone());
                            }
                            None => {
                                state.transition(BallWidgetState::Idle);
                                let _ = webview.evaluate_script("window.__setState(0)");
                                let _ = webview.evaluate_script("window.__toast('语音引擎未加载')");
                            }
                        }
                    }
                }
                UserEvent::ToggleWidget => {
                    widget_visible = !widget_visible;
                    window.set_visible(widget_visible);
                }
                UserEvent::Toast(msg) => {
                    let _ = webview.evaluate_script(&format!("window.__toast({msg:?})"));
                }
                UserEvent::SpeakFallback(text) => {
                    // 本地 TTS 不可用/失败：切浏览器拟人音兜底朗读
                    let _ = webview.evaluate_script(&format!("window.__speakFallback({text:?})"));
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
    use mox_voice_operator_svc::voice_server::{VoiceServiceConfig, XiaobaiVoiceService};
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

    // headless：TTS 合成
    if let Some(text) = args.tts {
        return run_tts_pipeline(text, &args.out).await;
    }
    // 诊断：独立线程内 TTS 合成（模拟 30010 跨线程调用）
    if let Some(text) = args.tts_thread {
        let models = mox_voice_desktop_app::voice_engine::locate_models_dir()
            .ok_or_else(|| anyhow::anyhow!("未找到语音模型目录（models/voice）。"))?;
        let engine = Arc::new(mox_voice_desktop_app::VoiceEngine::new(&models)?);
        let t = text.clone();
        let handle = std::thread::spawn(move || {
            let t0 = std::time::Instant::now();
            match engine.synthesize(&t) {
                Ok((samples, sr)) => {
                    println!("线程内合成成功：{} 样本 sr={} 耗时{}ms", samples.len(), sr, t0.elapsed().as_millis());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("线程内合成失败: {e:#}");
                    Err::<(), anyhow::Error>(e)
                }
            }
        });
        handle.join().map_err(|_| anyhow::anyhow!("线程 panic"))??;
        return Ok(());
    }
    // headless：WAV → ASR 识别
    if let Some(wav) = args.asrfile {
        let models = mox_voice_desktop_app::voice_engine::locate_models_dir()
            .ok_or_else(|| anyhow::anyhow!("未找到语音模型目录（models/voice）。"))?;
        let engine = mox_voice_desktop_app::VoiceEngine::new(&models)?;
        let t0 = std::time::Instant::now();
        let text = engine.recognize_wav_file(std::path::Path::new(&wav))?;
        println!("{}", serde_json::json!({
            "wav": wav,
            "asr_text": text,
            "elapsed_ms": t0.elapsed().as_millis(),
        }));
        return Ok(());
    }
    // headless：录音 → ASR 链路
    if args.vtest {
        return run_voice_test(args.vsecs, &args.out);
    }

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

    run_gui(args)
}