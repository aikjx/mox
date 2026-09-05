// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 子服务聚合（Phase 1 收敛）
//!
//! 将四个可选领域模块按部署清单装配到 operator-server；各领域仍可独立部署：
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
    pub report: mox_platform_module_core::StartupReport,
}

/// Explicit deployment composition; initialization outcomes are recorded, never assumed healthy.
fn default_modules() -> Vec<mox_platform_module_core::ModuleSpec> {
    [("mox-viz", "MOX_VIZ", PREFIX_MOX_VIZ), ("mox-system", "MOX_SYSTEM", PREFIX_MOX_SYSTEM),
     ("primiflow", "PRIMIFLOW", PREFIX_PRIMIFLOW), ("fusion", "FUSION", PREFIX_FUSION)]
        .into_iter().filter(|(_, env, _)| {
            !std::env::var(format!("OUS_ENABLE_{env}")).map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "false" | "off" | "no")).unwrap_or(false)
        }).map(|(id, _, prefix)| mox_platform_module_core::ModuleSpec {
            id: id.into(), contract_major: 1, required: true, dependencies: vec![], route_prefix: Some(prefix.into()),
        }).collect()
}

pub async fn build() -> anyhow::Result<SubServers> {
    let specs = match std::env::var("MOX_MODULES_CONFIG") {
        Ok(path) => serde_json::from_slice(&std::fs::read(path)?)?,
        Err(_) => default_modules(),
    };
    build_with_specs(specs).await
}

pub async fn build_with_specs(specs: Vec<mox_platform_module_core::ModuleSpec>) -> anyhow::Result<SubServers> {
    use mox_platform_module_core::ModulePlan;
    // Route ownership and contract versions are validated before any service initialization.
    for spec in &specs {
        let prefix = match spec.id.as_str() {
            "mox-viz" => PREFIX_MOX_VIZ, "mox-system" => PREFIX_MOX_SYSTEM,
            "primiflow" => PREFIX_PRIMIFLOW, "fusion" => PREFIX_FUSION,
            other => anyhow::bail!("unknown embedded module: {other}"),
        };
        anyhow::ensure!(spec.contract_major == 1 && spec.route_prefix.as_deref() == Some(prefix), "unsupported contract or route for {}", spec.id);
    }
    let plan = ModulePlan::new(specs)?;
    let order = plan.order().to_vec();
    let mut startup = plan.startup();
    let mut routers = Vec::new();
    let mut notes = Vec::new();
    for id in order {
        if let Err(error) = startup.can_start(&id) {
            startup.failed(&id, error.to_string())?;
            notes.push(format!("module {id}: skipped because dependencies are unavailable"));
            continue;
        }
        let result: anyhow::Result<(&'static str, Router)> = async {
            match id.as_str() {
                "mox-viz" => {
                    let state = mox_ai_expert_svc::server::AppState::new_state();
                    Ok((PREFIX_MOX_VIZ, mox_ai_expert_svc::server::router(state)))
                }
                "mox-system" => {
                    let cfg = mox_platform_system_core::config::AppConfig::load();
                    let sys = Arc::new(mox_platform_system_core::MoxSystem::with_config(cfg).await?);
                    anyhow::ensure!(sys.store.mox_count().await > 0,
                        "mox-system has no initialized organization; provision it explicitly using MoxSystem::bootstrap before serving, or omit this module from the deployment");
                    // Initialization must not create an administrator or print a bearer token.
                    let _reactor = sys.start_reactor();
                    Ok((PREFIX_MOX_SYSTEM, mox_platform_system_core::server::app(sys)))
                }
                "primiflow" => {
                    let out_dir = std::env::var("OUS_PRIMIFLOW_OUT").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("./primiflow_runtime"));
                    let state = mox_flow_primiflow_svc::server::new_state(out_dir, mox_flow_primiflow_svc::persistence::Persistence::memory());
                    Ok((PREFIX_PRIMIFLOW, mox_flow_primiflow_svc::server::build_router(state)))
                }
                "fusion" => {
                    let state = mox_flow_fusion_svc::server::new_state(mox_flow_fusion_svc::config::Config::default());
                    Ok((PREFIX_FUSION, mox_flow_fusion_svc::server::build_router(state)))
                }
                _ => unreachable!("validated before initialization"),
            }
        }.await;
        match result {
            Ok(router) => {
                match startup.ready(&id) {
                    Ok(()) => { routers.push(router); notes.push(format!("module {id}: initialized")); }
                    Err(error) => { startup.failed(&id, error.to_string())?; }
                }
            }
            Err(error) => { startup.failed(&id, error.to_string())?; notes.push(format!("module {id}: initialization failed: {error}")); }
        }
    }
    let report = startup.report();
    anyhow::ensure!(report.ready, "required modules failed to initialize: {:?}", report.modules);
    Ok(SubServers { routers, notes, report })
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
    pub required: bool, // External service inventory only; startup policy is declared in ModuleSpec.
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
            purpose: "专家联盟 6 阶段mox 模块化系统架构分析引擎（Rust crate，内嵌于本 mox_platform_orchestrator_svc）",
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

    #[tokio::test]
    async fn deployment_rejects_unknown_module_before_initialization() {
        let specs = vec![mox_platform_module_core::ModuleSpec { id: "unknown".into(), contract_major: 1,
            required: true, dependencies: vec![], route_prefix: Some("/unknown".into()) }];
        assert!(build_with_specs(specs).await.is_err());
    }

    #[tokio::test]
    async fn empty_deployment_does_not_mount_or_initialize_services() {
        let result = build_with_specs(vec![]).await.unwrap();
        assert!(result.routers.is_empty());
        assert!(result.report.ready);
        assert!(!result.report.degraded);
    }

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

