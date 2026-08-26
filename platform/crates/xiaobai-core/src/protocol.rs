//! voice_proxy JSON 信封协议（与 Python `AIS-FR13/V1.0` 字段级一致）
//!
//! 所有 WebSocket / HTTP 双向通信都套同一个 [`Envelope`]：
//!
//! ```json
//! {
//!   "version": "AIS-FR13/V1.0",
//!   "kind": "intent",
//!   "id": "uuid-v4",
//!   "ts_ms": 1787000000000,
//!   "sender": "desktop-ball-widget",
//!   "payload": { /* intent or audit or exec or ack */ }
//! }
//! ```
//!
//! 这样无论 HTTP POST / WS 文本帧，接收方都走同一段 JSON 解析，避免两套协议。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::constants::{XIAOBAI_PROTOCOL_VERSION, XIAOBAI_ENGINE_NAME};
use crate::errors::{XiaobaiError, XiaobaiResult};
use crate::rbac::DispatchMode;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeKind {
    /// 桌面端 → mox 联盟：发起一次意图（text + 身份 + 三策略模式）
    Intent,
    /// 执行结果（本地或远程算子都用这个回）
    Exec,
    /// 异步审计消息（LocalFirst 下本地执行完向联盟上报）
    Audit,
    /// ack（WS 心跳回包、意图接收确认、裁决前 lock 确认）
    Ack,
    /// ping（WS 心跳；BRIDGE_PING_INTERVAL_MS 周期）
    Ping,
    /// 错误：XB-001~XB-012 任意错误码；接收方按 http_status() 处理
    Error,
}

impl EnvelopeKind {
    pub fn as_str(self) -> &'static str {
        use EnvelopeKind::*;
        match self {
            Intent => "intent",
            Exec => "exec",
            Audit => "audit",
            Ack => "ack",
            Ping => "ping",
            Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub version: String,
    pub kind: EnvelopeKind,
    /// 全局唯一消息 ID（reply_to 用，防止重复执行幂等）
    pub id: String,
    /// 发消息时的毫秒时间戳（chrono::Utc::now().timestamp_millis）
    pub ts_ms: i64,
    /// 发送方标识："desktop-ball-widget"/"mox-alliance"/"xiaobai-engine"/"voice-proxy-ws"
    pub sender: String,
    /// 对应 kind 的具体 payload JSON
    pub payload: Value,
    /// 若为应答消息则填原请求 Envelope.id；否则 null（供审计链路关联）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
}

impl Envelope {
    /// 构造 intent 消息（BallWidget 调用 dispatch_intent 时生成）
    pub fn new_intent(
        sender: impl Into<String>,
        text: impl Into<String>,
        identity: &crate::identity::OperatorIdentity,
        mode: DispatchMode,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let payload = serde_json::to_value(IntentPayload {
            text: text.into(),
            identity: identity.clone(),
            mode,
            nonce: Uuid::new_v4().to_string(),
        })
        .unwrap_or(Value::Null);
        Self {
            version: XIAOBAI_PROTOCOL_VERSION.into(),
            kind: EnvelopeKind::Intent,
            id,
            ts_ms: chrono::Utc::now().timestamp_millis(),
            sender: sender.into(),
            payload,
            reply_to: None,
        }
    }

    /// 构造 ack 确认（WS 收到 intent 后立即返回，800ms 超时计时器清零）
    pub fn ack_reply(request: &Envelope, sender: impl Into<String>) -> Self {
        Self {
            version: XIAOBAI_PROTOCOL_VERSION.into(),
            kind: EnvelopeKind::Ack,
            id: Uuid::new_v4().to_string(),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            sender: sender.into(),
            payload: serde_json::to_value(AckPayload {
                received: true,
                stage: "pre_dispatch".into(),
                ttl_ms: None,
            })
            .unwrap_or(Value::Null),
            reply_to: Some(request.id.clone()),
        }
    }

    /// 构造错误信封（从 XB-001~XB-012 自动填 code/http_stauts）
    pub fn error_reply(request: &Envelope, sender: impl Into<String>, e: &XiaobaiError) -> Self {
        use serde_json::json;
        Self {
            version: XIAOBAI_PROTOCOL_VERSION.into(),
            kind: EnvelopeKind::Error,
            id: Uuid::new_v4().to_string(),
            ts_ms: chrono::Utc::now().timestamp_millis(),
            sender: sender.into(),
            payload: json!({
                "code": e.as_error_code(),
                "http_status": e.http_status(),
                "message": e.to_string(),
            }),
            reply_to: Some(request.id.clone()),
        }
    }

    pub fn decode_intent(&self) -> XiaobaiResult<IntentPayload> {
        serde_json::from_value(self.payload.clone()).map_err(|e| {
            XiaobaiError::InvalidArgument {
                action: "decode_intent_payload".into(),
                param: "payload".into(),
                value: e.to_string(),
                hint: "Intent 消息 payload 需含 text/identity/mode/nonce".into(),
            }
        })
    }
    pub fn decode_audit(&self) -> XiaobaiResult<AuditPayload> {
        serde_json::from_value(self.payload.clone()).map_err(|e| {
            XiaobaiError::InvalidArgument {
                action: "decode_audit_payload".into(),
                param: "payload".into(),
                value: e.to_string(),
                hint: "Audit 消息 payload 需含 trace_id/action/result/level".into(),
            }
        })
    }
}

// ==================== 四类 payload 定义 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPayload {
    pub text: String,
    pub identity: crate::identity::OperatorIdentity,
    pub mode: DispatchMode,
    /// 幂等随机串：重复 nonce 30s 内只执行一次，防止网络抖动双执行
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPayload {
    pub trace_id: String,
    pub action: String,
    pub identity_user_id: String,
    pub result: String,       // "permitted_denied" / "passed" / "rejected" / "unsupported"
    pub level: u8,            // 需要的 clearance level
    pub detail: String,       // 自由文本详情
    pub executed_at_ms: i64,
    pub source: String,       // "local_operator" / "remote_alliance"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecPayload {
    pub action: String,
    pub category: String,
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    pub fallbacks_used: Vec<String>,
    pub elapsed_ms: u64,
    /// 联盟裁决 verdict 文案（本地执行填 "local_first_direct" / "cloud_fallback_direct"）
    pub verdict: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckPayload {
    pub received: bool,
    /// 处理阶段：pre_dispatch / alliance_adjudicating / executing_local / executing_remote
    pub stage: String,
    /// 对于 Ping/Ack 的 TTL（若有）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
}

// ==================== Client/Server 抽象 ====================

/// voice_proxy 客户端（桌面端 BallWidget / xiaobai-asr 会用到）
#[async_trait]
pub trait VoiceProxyClient: Send + Sync {
    async fn send_intent(&self, intent: Envelope) -> XiaobaiResult<Envelope>;
    async fn send_audit(&self, audit: Envelope) -> XiaobaiResult<()>;
    async fn ping(&self) -> XiaobaiResult<u128>; // 返回 RTT 微秒
}

/// voice_proxy Server 句柄占位（LocalFirst 场景下，Engine 自己就是 Server，
/// 不需要走 WS——用这个本地 stub 省掉 IPC 开销；P2 VoiceProxyServer 反调桌面再扩）
#[derive(Debug, Default, Clone)]
pub struct VoiceProxyServerHandle {
    pub engine_name: String,
}
impl VoiceProxyServerHandle {
    pub fn new() -> Self {
        Self {
            engine_name: XIAOBAI_ENGINE_NAME.into(),
        }
    }
}
