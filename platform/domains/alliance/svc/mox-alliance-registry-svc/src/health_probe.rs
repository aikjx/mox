// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家实例后台主动健康探测
//!
//! 与被动心跳租约回收互补：被动方式只能在「心跳停止 + 租约到期」后摘除实例；
//! 主动探测周期访问注册时登记的 `health_check_url`，可在实例仍在心跳但实际
//! 业务端点不可用时提前标记 [`InstanceStatus::Unhealthy`]，并在恢复时自动回册。
//!
//! 设计要点：
//! - [`HealthProbe`] trait 可注入——生产用 [`HttpHealthProbe`]（reqwest GET，
//!   2xx 视为健康），测试用 [`MockHealthProbe`] 注入固定结果，不依赖真实网络。
//! - 探测失败只标记 Unhealthy，**不立即摘除**；连续失败由心跳租约到期后 reaper
//!   兜底摘除，避免单次抖动误杀。
//! - 探测成功且当前为 Unhealthy 时自动回册为 Active（Draining 保持，不自动复活）。
//! - D8（2026-10-02）：`health_probe_enabled` 默认开启，[`crate::server`] 默认即启动本
//!   任务；设 `MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=0/false/no/off` 可关闭，行为回退历史
//!   「仅被动心跳租约」。

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mox_alliance_registry_proto::InstanceStatus;
use tracing::{debug, warn};

use crate::storage::RegistryStore;

/// 健康探测器（可注入，便于测试不依赖网络）
#[async_trait]
pub trait HealthProbe: Send + Sync {
    /// 探测指定健康检查 URL；返回 `true` 表示健康（2xx），`false` 表示不健康。
    async fn probe(&self, url: &str) -> bool;
}

/// 默认 HTTP 探测实现：reqwest GET，2xx → 健康，其余/超时/错误 → 不健康。
pub struct HttpHealthProbe {
    client: reqwest::Client,
}

impl HttpHealthProbe {
    /// 构造带超时的 HTTP 客户端。`timeout_ms` 为整请求超时，连接超时收紧到其一半
    /// （下限 500ms），端点不可达时快速失败，不拖慢探测周期。
    pub fn new(timeout_ms: u64) -> Result<Self, reqwest::Error> {
        let connect = Duration::from_millis((timeout_ms / 2).clamp(500, timeout_ms.max(500)));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms.max(500)))
            .connect_timeout(connect)
            .build()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl HealthProbe for HttpHealthProbe {
    async fn probe(&self, url: &str) -> bool {
        match self.client.get(url).send().await {
            Ok(resp) => {
                let ok = resp.status().is_success();
                if !ok {
                    debug!(url, status = %resp.status(), "主动探测返回非 2xx");
                }
                ok
            }
            Err(e) => {
                debug!(url, error = %e, "主动探测请求失败");
                false
            }
        }
    }
}

/// 测试用 Mock 探测器：按 URL 返回预设结果，缺省视为不健康。
#[derive(Clone, Default)]
pub struct MockHealthProbe {
    results: Arc<Mutex<HashMap<String, bool>>>,
}

impl MockHealthProbe {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置某 URL 的探测结果。
    pub fn set(&self, url: &str, ok: bool) {
        self.results.lock().unwrap().insert(url.to_string(), ok);
    }
}

#[async_trait]
impl HealthProbe for MockHealthProbe {
    async fn probe(&self, url: &str) -> bool {
        self.results
            .lock()
            .unwrap()
            .get(url)
            .copied()
            .unwrap_or(false)
    }
}

/// 执行一轮探测：遍历所有登记了 `health_check_url` 的实例，按结果调整状态。
///
/// - 探测失败且当前为 Active → 标记 Unhealthy（不摘除）。
/// - 探测成功且当前为 Unhealthy → 回册为 Active。
/// - Draining/Expired 等其他状态不被探测改变（Draining 由人工/心跳显式控制）。
pub async fn run_probe_cycle(registry: &RegistryStore, probe: &dyn HealthProbe) {
    // 取全部状态实例：必须看到 Unhealthy 才能判定恢复。
    let targets: Vec<(String, String)> = registry
        .all_instances()
        .into_iter()
        .filter_map(|i| i.health_check_url.map(|u| (i.id, u)))
        .collect();

    for (id, url) in targets {
        let ok = probe.probe(&url).await;
        // 实例可能在探测期间被注销/摘除，取不到视为已不存在，跳过。
        let current = registry.get(&id).map(|i| i.status);
        match (ok, current) {
            (false, Some(InstanceStatus::Active)) => {
                warn!(instance = %id, url, "主动探测失败，标记 Unhealthy");
                registry.set_status(&id, InstanceStatus::Unhealthy);
            }
            (true, Some(InstanceStatus::Unhealthy)) => {
                debug!(instance = %id, url, "主动探测恢复，回册 Active");
                registry.set_status(&id, InstanceStatus::Active);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use mox_alliance_registry_proto::{RegisteredInstance, RegisterRequest};

    fn registered(id: &str, health_url: Option<&str>) -> RegisteredInstance {
        let req = RegisterRequest {
            id: Some(id.to_string()),
            name: format!("expert-{id}"),
            version: "1.0.0".to_string(),
            endpoint: format!("http://127.0.0.1:9000/{id}"),
            health_check_url: health_url.map(|s| s.to_string()),
            capabilities: vec!["code".to_string()],
            domain: Some("code".to_string()),
            weight: 1.0,
            load_current: 0,
            load_capacity: 100,
            lease_seconds: 15,
            metadata: Default::default(),
        };
        let now = Utc::now();
        RegisteredInstance {
            id: req.id.unwrap(),
            name: req.name,
            version: req.version,
            endpoint: req.endpoint,
            health_check_url: req.health_check_url,
            capabilities: req.capabilities,
            domain: req.domain,
            weight: req.weight,
            load_current: req.load_current,
            load_capacity: req.load_capacity,
            status: InstanceStatus::Active,
            registered_at: now,
            last_heartbeat_at: now,
            lease_seconds: req.lease_seconds,
            metadata: req.metadata,
        }
    }

    #[tokio::test]
    async fn probe_failure_marks_unhealthy_then_recovers_active() {
        let store = RegistryStore::new_memory();
        let url = "http://127.0.0.1:9001/health";
        store.register(registered("i1", Some(url)));

        let mock = MockHealthProbe::new();
        // 默认未设置 → 不健康
        run_probe_cycle(&store, &mock).await;
        assert_eq!(
            store.get("i1").unwrap().status,
            InstanceStatus::Unhealthy,
            "探测失败应标记 Unhealthy"
        );

        // 恢复：探测成功 → 回册 Active
        mock.set(url, true);
        run_probe_cycle(&store, &mock).await;
        assert_eq!(
            store.get("i1").unwrap().status,
            InstanceStatus::Active,
            "探测恢复应回册 Active"
        );
    }

    #[tokio::test]
    async fn healthy_active_instance_untouched() {
        let store = RegistryStore::new_memory();
        let url = "http://127.0.0.1:9002/health";
        store.register(registered("ok", Some(url)));
        let mock = MockHealthProbe::new();
        mock.set(url, true);
        run_probe_cycle(&store, &mock).await;
        assert_eq!(store.get("ok").unwrap().status, InstanceStatus::Active);
    }

    #[tokio::test]
    async fn instances_without_health_url_skipped() {
        let store = RegistryStore::new_memory();
        // 无 health_check_url：不应被探测触及状态
        store.register(registered("nourl", None));
        let mock = MockHealthProbe::new();
        run_probe_cycle(&store, &mock).await;
        assert_eq!(store.get("nourl").unwrap().status, InstanceStatus::Active);
    }

    #[test]
    fn probe_default_config_enables_task_d8() {
        // D8（2026-10-02）：默认配置下主动健康探测开启，server 启动后台探测任务；
        // 生产可由 MOX_ALLIANCE_REGISTRY_PROBE_ENABLED=0/false/no/off 关闭回退被动租约。
        let cfg = crate::Config::default();
        assert!(cfg.health_probe_enabled, "D8：主动健康探测应默认开启");
        assert_eq!(cfg.health_probe_interval_ms, 30_000);
        assert_eq!(cfg.health_probe_timeout_ms, 5_000);
    }
}
