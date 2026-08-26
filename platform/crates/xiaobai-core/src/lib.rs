//! # xiaobai-core · 小白语音桌面助手 Rust 权威单源
//!
//! 与 Python xiaobai_voice.errors / operator.base 1:1 对齐，同时对接 mox 原生
//! 5 角色 RBAC、mox-expert PII 判据、operator-core 流水线抽象，根治 Python 版
//! "常量散落 10+ 文件 / PII 判据三处分叉" 等 P1/P4 缺陷。
//!
//! ## 对外权威模块
//! - [`errors`]：所有错误码（对齐 Python FR-13/FR-5）
//! - [`identity`]：OperatorIdentity 身份 + RBAC 角色→四级 clearance 映射
//! - [`hotword`]：FR-5 热词结构（word/score ∈ [0,100]/category）+ 格式校验
//! - [`rbac`]：4 级 clearance、require_level 过程宏风格函数（替代 Python 装饰器）
//! - [`operator`]：SystemOperator trait + OperatorCategory + ActionSignature
//! - [`engine`]：OperatorEngine 三策略（LocalFirst/CloudFallback/CloudOnly）统一调度
//! - [`protocol`]：voice_proxy JSON 信封消息协议（intent/audit/exec/ack/ping）
//! - [`constants`]：FR-14 常量归一化（敏感前缀表、扣分权重、维度优先级、歧义阈值、S6 周更周期）

pub mod errors;
pub mod identity;
pub mod hotword;
pub mod rbac;
pub mod operator;
pub mod engine;
pub mod protocol;
pub mod constants;

pub use constants::{XIAOBAI_CRATE_ID, XIAOBAI_ENGINE_NAME, XIAOBAI_PROTOCOL_VERSION, AMBIGUITY_THRESHOLD, S6_WEEKLY_CYCLE_MS, MAX_HOTWORD_LEN, HOTWORD_SCORE_MIN, HOTWORD_SCORE_MAX};
pub use errors::{XiaobaiError, XiaobaiResult};
pub use identity::OperatorIdentity;
pub use hotword::Hotword;
pub use rbac::{ClearanceLevel, check_clearance, DispatchMode};
pub use operator::{SystemOperator, OperatorCategory, ActionSignature, ActionParam, OperatorOutput};
pub use engine::{OperatorEngine, EngineConfig, DispatchIntentResult};
pub use protocol::{Envelope, EnvelopeKind, IntentPayload, AuditPayload, ExecPayload, AckPayload, VoiceProxyClient, VoiceProxyServerHandle};
