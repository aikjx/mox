//! 子服务聚合（Phase 1 收敛）
//!
//! 将此前并行的四套 axum server 收敛为库，由 operator-server（runtime）唯一对外暴露：
//! - [`PREFIX_XUANJI_VIZ`]    ← xuanji-expert  治理可视化 + 一键闭环演示
//! - [`PREFIX_XUANJI_SYSTEM`] ← xuanji-system  成员/任务/RBAC/审计/WebSocket 协同
//! - [`PREFIX_PRIMIFLOW`]     ← primiflow      六维溯源拓扑引擎（server feature）
//! - [`PREFIX_FUSION`]        ← primiflow-fusion 融合合成/注册/落库/闸门
//!
//! 各子服务可用环境变量独立关闭（默认全部启用）：
//! `OUS_ENABLE_XUANJI_SYSTEM` / `OUS_ENABLE_XUANJI_VIZ` / `OUS_ENABLE_PRIMIFLOW` / `OUS_ENABLE_FUSION`
//! （取 `0`/`false`/`off`/`no` 时关闭）。
//!
//! 鉴权边界（与 `main::auth_middleware` 配合）：
//! - [`PASSTHROUGH_PREFIXES`]：由子服务自带成员令牌 RBAC 鉴权，网关透传；
//! - [`GATEWAY_PREFIXES`]：子服务无自带鉴权，由网关 `OUS_API_TOKEN` 统一保护。

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;

/// 挂载前缀（对外路径）
pub const PREFIX_XUANJI_SYSTEM: &str = "/xuanji-system";
pub const PREFIX_XUANJI_VIZ: &str = "/xuanji-viz";
pub const PREFIX_PRIMIFLOW: &str = "/primiflow";
pub const PREFIX_FUSION: &str = "/fusion";

/// 透传前缀：由子服务自带鉴权（成员令牌 RBAC）
pub const PASSTHROUGH_PREFIXES: [&str; 1] = [PREFIX_XUANJI_SYSTEM];
/// 网关保护前缀：由 `OUS_API_TOKEN` 统一鉴权
pub const GATEWAY_PREFIXES: [&str; 3] = [PREFIX_XUANJI_VIZ, PREFIX_PRIMIFLOW, PREFIX_FUSION];

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

/// 构建并初始化全部已启用的子服务（初始化失败的服务会跳过并记录说明，不阻断 runtime）
pub async fn build() -> SubServers {
    let mut out = SubServers {
        routers: Vec::new(),
        notes: Vec::new(),
    };

    // 1) xuanji-expert：治理可视化 + 一键闭环演示（原独立服务）
    if sub_enabled("XUANJI_VIZ") {
        let state = xuanji_expert::server::AppState::new_state();
        out.routers
            .push((PREFIX_XUANJI_VIZ, xuanji_expert::server::router(state)));
        out.notes.push(format!(
            "  [聚合] xuanji-viz → {PREFIX_XUANJI_VIZ}（治理可视化 + 闭环演示）"
        ));
    }

    // 2) xuanji-system：成员/任务/RBAC/审计/WS 协同（原 :3000 独立服务）
    if sub_enabled("XUANJI_SYSTEM") {
        let cfg = xuanji_system::config::AppConfig::load();
        match xuanji_system::XuanjiSystem::with_config(cfg).await {
            Ok(sys) => {
                let sys = Arc::new(sys);
                if sys.store.xuanji_count().await == 0 {
                    match sys
                        .bootstrap("默认璇玑", "系统管理员", "admin@xuanji.io")
                        .await
                    {
                        Ok((xuanji, _admin, token)) => {
                            out.notes.push(format!(
                                "  [聚合] xuanji-system 首次引导：璇玑「{}」id={}，管理员令牌={}",
                                xuanji.name, xuanji.id, token
                            ));
                        }
                        Err(e) => out
                            .notes
                            .push(format!("  [聚合] xuanji-system 引导失败（跳过）: {e}")),
                    }
                } else {
                    out.notes
                        .push("  [聚合] xuanji-system 已有数据，跳过引导".into());
                }
                let _reactor = sys.start_reactor();
                out.routers
                    .push((PREFIX_XUANJI_SYSTEM, xuanji_system::server::app(sys)));
                out.notes.push(format!(
                    "  [聚合] xuanji-system → {PREFIX_XUANJI_SYSTEM}（成员/任务/RBAC/审计/WS）"
                ));
            }
            Err(e) => out
                .notes
                .push(format!("  [聚合] xuanji-system 启动失败（已跳过）: {e}")),
        }
    }

    // 3) primiflow：六维溯源 + 拓扑正则化/固化（原 server feature 独立服务）
    if sub_enabled("PRIMIFLOW") {
        let out_dir = std::env::var("OUS_PRIMIFLOW_OUT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./primiflow_runtime"));
        let state = primiflow_core::server::new_state(
            out_dir,
            primiflow_core::persistence::Persistence::memory(),
        );
        out.routers.push((
            PREFIX_PRIMIFLOW,
            primiflow_core::server::build_router(state),
        ));
        out.notes.push(format!(
            "  [聚合] primiflow → {PREFIX_PRIMIFLOW}（六维溯源拓扑引擎）"
        ));
    }

    // 4) primiflow-fusion：融合合成/注册/落库/闸门（原独立服务）
    if sub_enabled("FUSION") {
        let state =
            primiflow_fusion::server::new_state(primiflow_fusion::config::Config::default());
        out.routers
            .push((PREFIX_FUSION, primiflow_fusion::server::build_router(state)));
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
            url: "http://127.0.0.1:3717".into(),
            health: "/voice/health".into(),
            required: true,
            timeout_ms: 1500,
        },
        Subserver {
            name: "xuanji-expert-alliance",
            purpose: "专家联盟 6 阶段全维分析引擎（Rust crate，内嵌于本 runtime）",
            url: "builtin://xuanji-expert/alliance".into(),
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
    fn at_least_two_subservers_and_voice_3717() {
        let list = registered_subservers();
        assert!(list.len() >= 2);
        let voice = list
            .iter()
            .find(|s| s.name == "xiaobai_voice")
            .expect("voice必须注册");
        assert!(
            voice.url.contains("3717"),
            "voice URL 必须是 3717：{}",
            voice.url
        );
        assert!(voice.health.starts_with("/voice/"));
        let alliance = list
            .iter()
            .find(|s| s.name == "xuanji-expert-alliance")
            .expect("alliance必须注册");
        assert!(alliance.health.starts_with("/ai/engine/"));
    }
}

