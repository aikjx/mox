//! 小白语音桌面权威错误码枚举
//!
//! 与 `xiaobai_voice/errors.py` 字段级 1:1 对齐：
//! - 保留 FR-13/FR-5 全部语义；新增 `OperatorUnsupported` 跨平台回退失败专用
//! - 全部实现 `thiserror::Error`，含上下文（动作名、缺失等级、拒绝原因、平台），可审计
//! - `#[serde(Serialize/Deserialize)]` 通过 `as_error_code()` 输出稳定 `XB-001` 机器码，
//!   避免桌面端 UI Toast 文案漂移。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum XiaobaiError {
    /// L3→L0 鉴权失败：缺失某个 clearance level
    #[error(
        "XB-001 PermissionDenied: action={action} required={required:?} identity={identity:?} reason={reason}"
    )]
    PermissionDenied {
        action: String,
        required: ClearanceLevelRepr,
        identity: String,
        reason: &'static str,
    },

    /// FR-5 热词格式校验失败：具体行号 + 非法字段值提示
    #[error("XB-002 HotwordsFormat: line={line} field={field} value={value} hint={hint}")]
    HotwordsFormat {
        line: usize,
        field: &'static str,
        value: String,
        hint: &'static str,
    },

    /// FR-5 S2 热词重建 recognizer 失败（sherpa-rs ContextConfig 反射或重建失败）
    #[error("XB-003 HotwordsReinstantiateFail: reason={0}")]
    HotwordsReinstantiateFail(String),

    /// 动作名未在 Engine 注册表命中
    #[error("XB-004 IntentUnknown: text={0}")]
    IntentUnknown(String),

    /// PPR 路由歧义（top1-top2 差值 < AMBIGUITY_THRESHOLD），需联盟裁决
    #[error("XB-005 IntentAmbiguous: text={text} top2={top2:?} delta={delta}")]
    IntentAmbiguous {
        text: String,
        top2: BTreeMap<String, f32>,
        delta: f32,
    },

    /// cloud_only 或 cloud_fallback 下 voice_proxy 桥断开（800ms 超时或 WS 1006 close）
    #[error("XB-006 BridgeDisconnected: mode={mode} target={target} elapsed_ms={elapsed_ms}")]
    BridgeDisconnected {
        mode: &'static str,
        target: String,
        elapsed_ms: u64,
    },

    /// 某算子在当前平台/缺依赖环境下全部回退链失败
    #[error("XB-007 OperatorUnsupported: category={category:?} action={action} platform={platform} fallbacks_used={fallbacks_used:?}")]
    OperatorUnsupported {
        category: String,
        action: String,
        platform: &'static str,
        fallbacks_used: Vec<String>,
    },

    /// 审计回调失败（S3/Syslog/Sink 任一审计通道返回 Err，由编排层记录不阻塞用户）
    #[error("XB-008 AuditCallbackFailed: sink={sink} reason={reason}")]
    AuditCallbackFailed { sink: &'static str, reason: String },

    /// 非法参数：参数类型不匹配 / 长度越界 / 路径不存在
    #[error("XB-009 InvalidArgument: action={action} param={param} value={value} hint={hint}")]
    InvalidArgument {
        action: String,
        param: String,
        value: String,
        hint: String,
    },

    /// 内部执行失败：Win32 API / CoreAudio / Shell API / send2trash 底层 Err
    /// （detail 命名避免被 thiserror 误当作 implicit #[error(source)] 源；错误类型不 impl StdError）
    #[error("XB-010 ExecutionError: category={category} action={action} detail={detail}")]
    ExecutionError {
        category: String,
        action: String,
        detail: String,
    },

    /// 联盟裁决事前否决（Parallelize ∧ MustSerialize 冲突 + 安全域一票否决）
    #[error("XB-011 AllianceRejected: verdict={verdict} reasons={reasons:?}")]
    AllianceRejected {
        verdict: &'static str,
        reasons: Vec<String>,
    },

    /// PII 敏感资源命中 → 强制提升到 L3；若身份不足转成 PermissionDenied
    #[error("XB-012 PiiLeakBlocked: resource={resource} required_level_lifted_to={required_level_lifted_to:?}")]
    PiiLeakBlocked {
        resource: String,
        required_level_lifted_to: ClearanceLevelRepr,
    },

    /// 透传 anyhow/第三方库内部错误（不进入错误码契约，仅兜底）
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl XiaobaiError {
    /// 稳定机器码（XB-001 ~ XB-012）——前端 Toast/埋点用，不随英文文案漂移
    pub fn as_error_code(&self) -> &'static str {
        use XiaobaiError::*;
        match self {
            PermissionDenied { .. } => "XB-001",
            HotwordsFormat { .. } => "XB-002",
            HotwordsReinstantiateFail(_) => "XB-003",
            IntentUnknown(_) => "XB-004",
            IntentAmbiguous { .. } => "XB-005",
            BridgeDisconnected { .. } => "XB-006",
            OperatorUnsupported { .. } => "XB-007",
            AuditCallbackFailed { .. } => "XB-008",
            InvalidArgument { .. } => "XB-009",
            ExecutionError { .. } => "XB-010",
            AllianceRejected { .. } => "XB-011",
            PiiLeakBlocked { .. } => "XB-012",
            Other(_) => "XB-999",
        }
    }

    /// HTTP 映射（REST/WS 协议使用）：联盟裁决拒绝 403、鉴权 401、未知 404、其余 500
    pub fn http_status(&self) -> u16 {
        use XiaobaiError::*;
        match self {
            PermissionDenied { .. } | AllianceRejected { .. } | PiiLeakBlocked { .. } => 403,
            IntentUnknown { .. } | OperatorUnsupported { .. } => 404,
            IntentAmbiguous { .. } => 300,
            BridgeDisconnected { .. } => 503,
            HotwordsFormat { .. } | InvalidArgument { .. } => 400,
            HotwordsReinstantiateFail(_) | AuditCallbackFailed { .. } | ExecutionError { .. } => 500,
            Other(_) => 500,
        }
    }
}

/// JSON 序列化友好的 clearance level 输出（枚举用整数 0~3）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClearanceLevelRepr(pub u8);

pub type XiaobaiResult<T> = Result<T, XiaobaiError>;
