//! # voice_proxy Rust 版 3717 服务（Feature = server-3717）
//!
//! 提供 Python `service/app.py FastAPI` 在 `:3717` 的等价 1:1 HTTP + WS 端点，
//! 同时内部直接挂载：
//! - xiaobai-intent 规则路由（PPR）
//! - xiaobai-core RBAC + OperatorEngine
//! - xiaobai-operators 4 大类系统算子
//! - xiaobai-asr FR-5 热词三层注入（S1/S2/S3）
//! - xiaobai-core::protocol AIS-FR13/V1.0 JSON 信封协议
//!
//! 端到端入口：`dispatch_text("打开微信")` → Engine.dispatch_intent → 真实操控电脑

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json as AxumJson, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};

use mox_voice_asr_svc::HotwordInjector;
use mox_voice_core_svc::constants::AMBIGUITY_THRESHOLD;
use mox_voice_core_svc::engine::{AuditFn, EngineConfig, OperatorEngine, XiaobaiResult};
use mox_voice_core_svc::errors::XiaobaiError;
use mox_voice_core_svc::identity::{OperatorIdentity, RoleTag};
use mox_voice_core_svc::protocol::Envelope;
use mox_voice_core_svc::rbac::DispatchMode;
use mox_voice_intent_svc::DefaultRouter;

use crate::register_all_defaults;

// ===== Service =====

/// 启动配置（对应 Python UvicornSettings + 鉴权配置）
#[derive(Debug, Clone)]
pub struct VoiceServiceConfig {
    pub bind: SocketAddr,
    pub default_identity: OperatorIdentity,
    pub default_mode: DispatchMode,
}

impl Default for VoiceServiceConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:3717".parse().unwrap(),
            default_identity: OperatorIdentity::new("voice_user_3717", RoleTag::Member, false),
            default_mode: DispatchMode::LocalFirst,
        }
    }
}

/// 组合 Service（Engine + HotwordInjector）
pub struct XiaobaiVoiceService {
    pub engine: Arc<OperatorEngine>,
    pub hotwords: Arc<HotwordInjector>,
    pub config: VoiceServiceConfig,
}

impl XiaobaiVoiceService {
    pub fn new(config: VoiceServiceConfig) -> Result<Self, XiaobaiError> {
        let router: Arc<dyn mox_voice_core_svc::engine::IntentRouter> = Arc::new(DefaultRouter::new());
        let log_audit: AuditFn = Arc::new(|payload| {
            info!(target: "xiaobai_audit", trace_id = %payload.trace_id, action = %payload.action);
            Ok(())
        });
        let mut engine_config = EngineConfig::default();
        engine_config.mode = config.default_mode.clone();
        engine_config.audit_fn = log_audit;
        let engine = Arc::new(OperatorEngine::new(engine_config, router));
        register_all_defaults(&engine);
        Ok(Self { engine, hotwords: Arc::new(HotwordInjector::new()), config })
    }

    /// 文本端到端：热词 S3 修正 → Engine dispatch_intent（内部走 PPR 路由 + RBAC + 算子）
    pub async fn dispatch_text(
        &self,
        text: &str,
        identity: Option<OperatorIdentity>,
        mode: Option<DispatchMode>,
    ) -> XiaobaiResult<Value> {
        // S3 post-hoc（如果设置了热词）
        let (corrected, _hits) = self
            .hotwords
            .apply_post_hoc(text)
            .unwrap_or_else(|_| (text.to_string(), Vec::new()));
        let identity = identity.unwrap_or_else(|| self.config.default_identity.clone());
        // 注意：EngineConfig 的 mode 在构造时已设定；这里 mode 参数传入仅用于审计/统计
        let _mode = mode.unwrap_or_else(|| self.config.default_mode.clone());
        let result = self.engine.dispatch_intent(&corrected, &identity).await?;
        Ok(json!({
            "intent": {
                "action": result.action,
                "category": format!("{:?}", result.category),
                "executed_where": result.executed_where,
                "required_level": result.required_level.0,
                "verdict": result.verdict,
            },
            "execution": {
                "ok": true,
                "output": result.output,
            },
            "trace_id": result.trace_id,
            "total_elapsed_ms": result.total_elapsed_ms,
            "corrected_text": corrected,
        }))
    }
}

// ===== HTTP Handlers =====

#[derive(Debug, Deserialize)]
struct DispatchTextRequest {
    text: String,
    identity: Option<Value>,
    mode: Option<String>,
    apply_hotwords_s3: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SetHotwordsRequest {
    hotwords: Value, // Vec<Hotword> 或 Vec<{word, score}> 宽松解析
}

#[derive(Debug, Deserialize)]
struct PostHocRequest {
    text: String,
}

fn make_mode(mode: Option<&str>, default: &DispatchMode) -> DispatchMode {
    match mode {
        Some("local") | Some("LocalFirst") => DispatchMode::LocalFirst,
        Some("cloud") | Some("CloudFallback") => DispatchMode::CloudFallback,
        Some("cloud_only") | Some("CloudOnly") => DispatchMode::CloudOnly,
        _ => default.clone(),
    }
}

type AppState = Arc<XiaobaiVoiceService>;

fn xb_err_to_parts(err: &XiaobaiError) -> (&'static str, String, u16) {
    use XiaobaiError::*;
    let code = match err {
        PermissionDenied { .. } => "XB-001",
        IntentUnknown { .. } => "XB-002",
        IntentAmbiguous { .. } => "XB-003",
        BridgeDisconnected { .. } => "XB-004",
        AllianceRejected { .. } => "XB-005",
        HotwordsFormat { .. } => "XB-006",
        HotwordsReinstantiateFail(_) => "XB-007",
        InvalidArgument { .. } => "XB-008",
        OperatorUnsupported { .. } => "XB-009",
        AuditCallbackFailed { .. } => "XB-010",
        ExecutionError { .. } => "XB-011",
        PiiLeakBlocked { .. } => "XB-012",
        Other(_) => "XB-999",
    };
    let http = err.http_status();
    (code, format!("{err}"), http)
}

struct XiErr(XiaobaiError);
impl From<XiaobaiError> for XiErr {
    fn from(e: XiaobaiError) -> Self { XiErr(e) }
}

impl IntoResponse for XiErr {
    fn into_response(self) -> Response {
        let (code, msg, http_status) = xb_err_to_parts(&self.0);
        let status = axum::http::StatusCode::from_u16(http_status).unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        (status, AxumJson(json!({
            "error": { "code": code, "message": msg, "detail": null }
        }))).into_response()
    }
}

async fn health(State(svc): State<AppState>) -> Response {
    let actions = svc.engine.list_registered_actions();
    (StatusCode::OK, AxumJson(json!({
        "status": "ok",
        "service": "xiaobai-voice-rs",
        "version": "AIS-FR13/V1.0",
        "bind": svc.config.bind.to_string(),
        "default_identity": svc.config.default_identity.user_id,
        "default_mode": format!("{:?}", svc.config.default_mode),
        "operators_registered": actions.len(),
        "actions": actions.iter().map(|(n, c, l)| json!({"name": n, "category": format!("{:?}", c), "clearance": l})).collect::<Vec<_>>(),
        "hotwords_count": svc.hotwords.hotwords().len(),
        "hotword_file": svc.hotwords.hotword_file_path().map(|p| p.display().to_string()),
    }))).into_response()
}

async fn dispatch_text(
    State(svc): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    AxumJson(req): AxumJson<DispatchTextRequest>,
) -> Response {
    tracing::info!(target: "xiaobai_http", from = %addr, text_len = req.text.len(), apply_s3 = ?req.apply_hotwords_s3);
    let identity = match req.identity.clone() {
        Some(v) => match serde_json::from_value::<OperatorIdentity>(v) {
            Ok(id) => Some(id),
            Err(e) => return XiErr::from(XiaobaiError::InvalidArgument {
                action: "dispatch_text".into(),
                param: "identity".into(),
                value: req.identity.map(|v| v.to_string()).unwrap_or_default(),
                hint: format!("identity JSON 解析失败：{e}"),
            }).into_response(),
        },
        None => None,
    };
    let mode = make_mode(req.mode.as_deref(), &svc.config.default_mode);
    // 先做 S3 可选（apply_hotwords_s3=true 默认开）
    let text = if req.apply_hotwords_s3.unwrap_or(true) {
        match svc.hotwords.apply_post_hoc(&req.text) {
            Ok((t, _h)) => t,
            Err(_e) => req.text.clone(),
        }
    } else {
        req.text.clone()
    };
    match svc.dispatch_text(&text, identity, Some(mode)).await {
        Ok(v) => (StatusCode::OK, AxumJson(v)).into_response(),
        Err(e) => XiErr::from(e).into_response(),
    }
}

async fn set_hotwords(State(svc): State<AppState>, AxumJson(req): AxumJson<SetHotwordsRequest>) -> Response {
    use mox_voice_core_svc::hotword::Hotword;
    // 宽松解析：Vec<Hotword> 或 Vec<{word, score}>
    let raw_vec: Vec<Value> = match req.hotwords {
        Value::Array(arr) => arr,
        Value::Object(obj) if obj.contains_key("hotwords") => {
            obj.get("hotwords").and_then(|v| v.as_array()).cloned().unwrap_or_default()
        }
        _ => return XiErr::from(XiaobaiError::InvalidArgument {
            action: "set_hotwords".into(), param: "hotwords".into(),
            value: req.hotwords.to_string(),
            hint: "需要数组 [{word,score}, ...]".into(),
        }).into_response(),
    };
    let mut hws = Vec::with_capacity(raw_vec.len());
    for (i, v) in raw_vec.iter().enumerate() {
        let hw = match serde_json::from_value::<Hotword>(v.clone()) {
            Ok(h) => h,
            Err(_) => {
                let word = v.get("word").and_then(|s| s.as_str()).unwrap_or("").to_string();
                let score = v.get("score").and_then(|s| s.as_f64()).unwrap_or(0.5) as f32;
                if word.is_empty() {
                    return XiErr::from(XiaobaiError::InvalidArgument {
                        action: "set_hotwords".into(), param: format!("hotwords[{i}]").into(),
                        value: v.to_string(), hint: "缺少 word 字段".into(),
                    }).into_response();
                }
                let h = Hotword::new(&word).with_score(score);
                h
            }
        };
        hws.push(hw);
    }
    match svc.hotwords.set_hotwords(&hws) {
        Ok((cleaned, report)) => (StatusCode::OK, AxumJson(json!({
            "accepted": cleaned.len(),
            "cleaned": cleaned,
            "report": report.to_json(),
        }))).into_response(),
        Err(e) => XiErr::from(e).into_response(),
    }
}

async fn get_hotwords(State(svc): State<AppState>) -> Response {
    let list = svc.hotwords.hotwords();
    let path = svc.hotwords.hotword_file_path().map(|p| p.display().to_string());
    (StatusCode::OK, AxumJson(json!({
        "count": list.len(),
        "hotwords": list,
        "hotword_file": path,
    }))).into_response()
}

async fn post_hoc(State(svc): State<AppState>, AxumJson(req): AxumJson<PostHocRequest>) -> Response {
    match svc.hotwords.apply_post_hoc(&req.text) {
        Ok((t, hits)) => (StatusCode::OK, AxumJson(json!({
            "original": req.text,
            "corrected": t,
            "hits": hits,
        }))).into_response(),
        Err(e) => XiErr::from(e).into_response(),
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(svc): State<AppState>) -> Response {
    ws.on_upgrade(move |_socket| async move {
        // 基础占位：WS 协议实现对应 Envelope JSON 消息解析
        // 完整 FR-13 WS 消息循环在 P2 迭代补充
        let _ = Arc::try_unwrap(svc);
    })
}

// ===== Router / Start =====

pub fn build_router(svc: Arc<XiaobaiVoiceService>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/dispatch_text", post(dispatch_text))
        .route("/v1/hotwords", get(get_hotwords).post(set_hotwords))
        .route("/v1/hotwords/post_hoc", post(post_hoc))
        .route("/ws", get(ws_handler))
        .with_state(svc)
}

/// 阻塞启动：等价 Python `uvicorn.run(app, host=..., port=3717)`
pub async fn serve(config: VoiceServiceConfig) -> XiaobaiResult<()> {
    let svc = Arc::new(XiaobaiVoiceService::new(config.clone())?);
    let app = build_router(svc);
    let listener = tokio::net::TcpListener::bind(config.bind).await.map_err(|e| XiaobaiError::ExecutionError {
        category: "server".into(), action: "serve_3717".into(), detail: format!("bind 失败：{e}"),
    })?;
    info!(target: "xiaobai_server", "xiaobai-voice-rs listening on {}", config.bind);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| XiaobaiError::ExecutionError {
            category: "server".into(), action: "serve_3717".into(), detail: format!("serve error：{e}"),
        })?;
    Ok(())
}

/// 同步阻塞启动（给 BallWidget spawn 线程用；内部创建 current_thread mox_platform_orchestrator_svc）
pub fn run_service_blocking(config: VoiceServiceConfig) -> XiaobaiResult<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| XiaobaiError::ExecutionError {
            category: "server".into(), action: "run_service_blocking".into(),
            detail: format!("创建 tokio mox_platform_orchestrator_svc 失败：{e}"),
        })?;
    rt.block_on(serve(config))
}

// ===== 协议导出（便于外部消费 AIS-FR13/V1.0）=====
pub use mox_voice_core_svc::protocol::{Envelope as AisEnvelope, EnvelopeKind as AisMessageType};
