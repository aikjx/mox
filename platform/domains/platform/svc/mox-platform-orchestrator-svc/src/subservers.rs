// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 子服务聚合（Phase 1 收敛）
//!
//! 将此前并行的四套 axum server 收敛为库，由 operator-server（mox_platform_orchestrator_svc）唯一对外暴露：
//! - [`PREFIX_MOX_VIZ`]    ← mox-expert  治理可视化 + 一键闭环演示
//! - [`PREFIX_MOX_SYSTEM`] ← mox-system  成员/任务/RBAC/审计/WebSocket 协同
//! - [`PREFIX_PRIMIFLOW`]     ← primiflow      六维溯源拓扑引擎（server feature）
//! - [`PREFIX_FUSION`]        ← primiflow-fusion 融合合成/注册/落库/闸门
//!
//! 各子服务可用环境变量独立关闭（默认全部启用）：
//! `OUS_ENABLE_MOX_SYSTEM` / `OUS_ENABLE_MOX_VIZ` / `OUS_ENABLE_PRIMIFLOW` / `OUS_ENABLE_FUSION`
//! （取 `0`/`false`/`off`/`no` 时关闭）。
//!
//! 鉴权边界（与 `main::auth_middleware` 配合）：
//! - [`PASSTHROUGH_PREFIXES`]：由子服务自带成员令牌 RBAC 鉴权，网关透传；
//! - [`GATEWAY_PREFIXES`]：子服务无自带鉴权，由网关 `OUS_API_TOKEN` 统一保护。

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;

/// 挂载前缀（对外路径）
pub const PREFIX_MOX_SYSTEM: &str = "/mox-system";
pub const PREFIX_MOX_VIZ: &str = "/mox-viz";
pub const PREFIX_PRIMIFLOW: &str = "/primiflow";
pub const PREFIX_FUSION: &str = "/fusion";

/// 透传前缀：由子服务自带鉴权（成员令牌 RBAC）
pub const PASSTHROUGH_PREFIXES: [&str; 1] = [PREFIX_MOX_SYSTEM];
/// 网关保护前缀：由 `OUS_API_TOKEN` 统一鉴权
pub const GATEWAY_PREFIXES: [&str; 3] = [PREFIX_MOX_VIZ, PREFIX_PRIMIFLOW, PREFIX_FUSION];

/// 聚合结果
pub struct SubServers {
    /// (前缀, `Router<()>`) 列表，由 main 逐个 `nest`
    pub routers: Vec<(&'static str, Router)>,
    /// 启动说明（打日志用）
    pub notes: Vec<String>,
}

fn sub_enabled(name: &str) -> bool {
    match std::env::var(format!("OUS_ENABLE_{name}")) {
        Ok(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        ),
        Err(_) => true,
    }
}

/// 构建并初始化全部已启用的子服务（初始化失败的服务会跳过并记录说明，不阻断 mox_platform_orchestrator_svc）
pub async fn build() -> SubServers {
    let mut out = SubServers {
        routers: Vec::new(),
        notes: Vec::new(),
    };

    // 1) mox-expert：治理可视化 + 一键闭环演示（原独立服务）
    if sub_enabled("MOX_VIZ") {
        let state = mox_ai_expert_svc::server::AppState::new_state();
        out.routers
            .push((PREFIX_MOX_VIZ, mox_ai_expert_svc::server::router(state)));
        out.notes.push(format!(
            "  [聚合] mox-viz → {PREFIX_MOX_VIZ}（治理可视化 + 闭环演示）"
        ));
    }

    // 2) mox-system：成员/任务/RBAC/审计/WS 协同（原 :3000 独立服务）
    if sub_enabled("MOX_SYSTEM") {
        let cfg = mox_platform_system_core::config::AppConfig::load();
        match mox_platform_system_core::MoxSystem::with_config(cfg).await {
            Ok(sys) => {
                let sys = Arc::new(sys);
                if sys.store.mox_count().await == 0 {
                    match sys
                        .bootstrap("默认璇玑", "系统管理员", "admin@mox.io")
                        .await
                    {
                        Ok((mox, _admin, token)) => {
                            out.notes.push(format!(
                                "  [聚合] mox-system 首次引导：璇玑「{}」id={}，管理员令牌={}",
                                mox.name, mox.id, token
                            ));
                        }
                        Err(e) => out
                            .notes
                            .push(format!("  [聚合] mox-system 引导失败（跳过）: {e}")),
                    }
                } else {
                    out.notes
                        .push("  [聚合] mox-system 已有数据，跳过引导".into());
                }
                let _reactor = sys.start_reactor();
                out.routers
                    .push((PREFIX_MOX_SYSTEM, mox_platform_system_core::server::app(sys)));
                out.notes.push(format!(
                    "  [聚合] mox-system → {PREFIX_MOX_SYSTEM}（成员/任务/RBAC/审计/WS）"
                ));
            }
            Err(e) => out
                .notes
                .push(format!("  [聚合] mox-system 启动失败（已跳过）: {e}")),
        }
    }

    // 3) primiflow：六维溯源 + 拓扑正则化/固化（原 server feature 独立服务）
    if sub_enabled("PRIMIFLOW") {
        let out_dir = std::env::var("OUS_PRIMIFLOW_OUT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./primiflow_runtime"));
        let state = mox_flow_primiflow_svc::server::new_state(
            out_dir,
            mox_flow_primiflow_svc::persistence::Persistence::memory(),
        );
        out.routers.push((
            PREFIX_PRIMIFLOW,
            mox_flow_primiflow_svc::server::build_router(state),
        ));
        out.notes.push(format!(
            "  [聚合] primiflow → {PREFIX_PRIMIFLOW}（六维溯源拓扑引擎）"
        ));
    }

    // 4) primiflow-fusion：融合合成/注册/落库/闸门（原独立服务）
    if sub_enabled("FUSION") {
        let state =
            mox_flow_fusion_svc::server::new_state(mox_flow_fusion_svc::config::Config::default());
        out.routers
            .push((PREFIX_FUSION, mox_flow_fusion_svc::server::build_router(state)));
        out.notes.push(format!(
            "  [聚合] primiflow-fusion → {PREFIX_FUSION}（融合合成/注册/落库/闸门）"
        ));
    }

    out
}

// ========================
// 子服务注册表（T8，FR-GW-05）
// ========================
// 启动时上报"已注册的子服务清单"到日志与 metrics。
// —— 保留上文 Phase 1 聚合逻辑，此处追加注册表 + 打印 + TDD 测试。

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subserver {
    pub name: &'static str,
    pub purpose: &'static str,
    pub url: String,
    pub health: String,
    pub required: bool, // true = 启动失败也不影响主进程（降级）；目前全部 true 保持弹性
    pub timeout_ms: u64,
}

pub fn registered_subservers() -> Vec<Subserver> {
    vec![
        Subserver {
            name: "xiaobai_voice",
            purpose: "ASR (Paraformer-zh + sherpa-onnx) / TTS (CosyVoice2 Apache-2.0 默认 / Fish-S2-Pro 需 Research License)",
            url: "http://127.0.0.1:30010".into(),
            health: "/voice/health".into(),
            required: true,
            timeout_ms: 1500,
        },
        Subserver {
            name: "mox-expert-alliance",
            purpose: "专家联盟 6 阶段全维分析引擎（Rust crate，内嵌于本 mox_platform_orchestrator_svc）",
            url: "builtin://mox-expert/alliance".into(),
            health: "/ai/engine/alliance/capabilities".into(),
            required: true,
            timeout_ms: 800,
        },
    ]
}

/// 打印子服务注册到 stderr（main 启动时调用一次即可）
pub fn print_subserver_registry() {
    eprintln!("\n========== Platform Subservers (FR-GW-05) ==========");
    for s in registered_subservers() {
        eprintln!(
            "  • {:26} | required={} | {} | health={}",
            s.name, s.required, s.url, s.health
        );
    }
    eprintln!("=====================================================\n");
}

/// TDD 友好：获取超时 Duration
pub fn timeout(s: &Subserver) -> Duration {
    Duration::from_millis(s.timeout_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_two_subservers_and_voice() {
        let list = registered_subservers();
        assert!(list.len() >= 2);
        let voice = list
            .iter()
            .find(|s| s.name == "xiaobai_voice")
            .expect("voice必须注册");
        assert!(
            voice.url.contains("30010"),
            "voice URL 必须是 30010：{}",
            voice.url
        );
        assert!(voice.health.starts_with("/voice/"));
        let alliance = list
            .iter()
            .find(|s| s.name == "mox-expert-alliance")
            .expect("alliance必须注册");
        assert!(alliance.health.starts_with("/ai/engine/"));
    }
}

