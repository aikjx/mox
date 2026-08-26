//! Sidecar 客户端模块：
//!   - node_sidecar：Node 127.0.0.1:3010 内部 API 调用（含 fallback 与指标）
//!
//! 导出规范（2026-08 企业级 lint 收敛）：
//! - 仅对外导出当前 bin/其他 crate 真实使用的符号（允许死代码在 node_sidecar 内部通过 `#[allow(dead_code)]` 保留
//!   为未来 sidecar 新端点做 SDK 预拓展）；
//! - SidecarError / SidecarMetrics 为 internal（仅 node_sidecar 内使用），不污染顶层 re-export；
//! - 顶层 re-export 的符号是本 crate 对外公开 SDK（即使某个 bin 文件恰好没 import，也保持 pub），
//!   因此统一允许 unused_imports。

pub mod node_sidecar;

// 以下为 runtime crate 公开 SDK 的顶层 re-exports（GraphAlgoResp / SidecarError
// / SidecarMetrics 目前仅在 node_sidecar 内使用，不在顶层 re-export）
#[allow(unused_imports)]
pub use node_sidecar::{
    GraphAlgoReq, GraphAlgoResp, IntentReq, IntentResp, NodeSidecarClient, SidecarError,
    SidecarMetrics, SidecarMetricsSnapshot,
};
