//! # Nacos 注册中心（feature = "naming"）
//!
//! 基于官方 **nacos-group/nacos-sdk-rust**（crates.io `nacos-sdk`）NamingService：
//!
//! - 服务启动时 `register_instance(service_name, group, instance)` 注册自身实例；
//! - 服务退出时 `deregister_instance` 注销（显式优雅注销，避免僵尸实例）；
//! - `ServiceInstance` 含 ip / port / weight / healthy / enabled / ephemeral / metadata，
//!   满足 PORT-NORM-001 的服务发现接入。
//!
//! 本模块仅在 `features = ["naming"]` 时编译（`naming` 隐含 `nacos` config 能力）。
//! `NamingSection` 结构本身**始终可解析**（不依赖 SDK），默认 `enabled=false`。

use std::collections::HashMap;

use nacos_sdk::api::naming::{NamingService, NamingServiceBuilder, ServiceInstance};
use nacos_sdk::api::props::ClientProps;

use crate::NacosSection;

/// Nacos 注册中心引导段（bootstrap）。
///
/// 语义：`nacos.enabled=true` 且 `naming.enabled=true` 时，服务启动把自己注册到
/// Nacos，让其他服务（或 Nacos 网关）通过服务名发现本实例。失败仅告警不阻断启动。
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct NamingSection {
    /// 是否启用服务注册（默认关闭）
    pub enabled: bool,
    /// 注册的服务名（如 `mox-alliance-scheduler`）
    pub service_name: String,
    /// 分组（默认 `DEFAULT_GROUP`）
    pub group: String,
    /// 对外注册 IP（默认 `127.0.0.1`；生产按实际绑定网卡配置）
    pub ip: String,
    /// 对外注册端口（默认 0，由调用方填实际监听端口）
    pub port: u16,
    /// 负载权重（默认 1.0）
    pub weight: f64,
    /// 附加元数据（`key=value` 形式，如 `protocol=http`、`domain=alliance`）
    pub metadata: Vec<String>,
}

impl Default for NamingSection {
    fn default() -> Self {
        Self {
            enabled: false,
            service_name: String::new(),
            group: "DEFAULT_GROUP".to_string(),
            ip: "127.0.0.1".to_string(),
            port: 0,
            weight: 1.0,
            metadata: Vec::new(),
        }
    }
}

impl NamingSection {
    /// 将 `key=value` 元数据列表解析为 HashMap
    fn metadata_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for kv in &self.metadata {
            if let Some((k, v)) = kv.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            } else {
                map.insert(kv.trim().to_string(), String::new());
            }
        }
        map
    }
}

/// Nacos 注册中心句柄：持 NamingService + 注册实例信息，供注册/注销。
///
/// 注意：nacos-sdk 0.8 的 `NamingService` 是**具体 struct**（非 trait），builder `build()`
/// 返回它；因此直接持有，无需 `Arc<dyn ...>`。
pub struct NamingRegistry {
    service: NamingService,
    service_name: String,
    group: Option<String>,
    instance: ServiceInstance,
}

impl NamingRegistry {
    /// 连接 Nacos 注册中心。
    ///
    /// - `nacos.enabled=false` 或 `naming.enabled=false` 或 `service_name` 为空 → `Ok(None)`；
    /// - 连接失败 → `Ok(None)`（告警降级：注册中心不可用不阻断服务启动）。
    pub async fn connect(
        nacos: &NacosSection,
        naming: &NamingSection,
    ) -> anyhow::Result<Option<Self>> {
        if !nacos.enabled || !naming.enabled || naming.service_name.trim().is_empty() {
            return Ok(None);
        }

        let props = ClientProps::new()
            .server_addr(&nacos.server_addr)
            .namespace(&nacos.namespace)
            .app_name("mox-alliance");
        let service = match NamingServiceBuilder::new(props).build().await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    err = %e,
                    server = %nacos.server_addr,
                    "Nacos 注册中心连接失败，跳过服务注册（服务仍正常启动）"
                );
                return Ok(None);
            }
        };

        let instance = ServiceInstance {
            instance_id: None,
            ip: naming.ip.clone(),
            port: naming.port as i32,
            weight: naming.weight,
            healthy: true,
            enabled: true,
            ephemeral: true,
            cluster_name: Some("DEFAULT".to_string()),
            service_name: Some(naming.service_name.clone()),
            metadata: naming.metadata_map(),
        };

        tracing::info!(
            service = %naming.service_name,
            group = %naming.group,
            ip = %naming.ip,
            port = %naming.port,
            "Nacos 注册中心已连接（NamingService）"
        );
        Ok(Some(Self {
            service,
            service_name: naming.service_name.clone(),
            group: Some(naming.group.clone()),
            instance,
        }))
    }

    /// 注册自身实例到 Nacos。失败仅告警（不阻断）。
    pub async fn register(&self) {
        match self
            .service
            .register_instance(self.service_name.clone(), self.group.clone(), self.instance.clone())
            .await
        {
            Ok(_) => tracing::info!(
                service = %self.service_name,
                ip = %self.instance.ip,
                port = %self.instance.port,
                "已注册到 Nacos 注册中心"
            ),
            Err(e) => tracing::warn!(err = %e, "Nacos 服务注册失败（服务继续运行）"),
        }
    }

    /// 注销自身实例（服务退出时优雅注销）。
    pub async fn deregister(&self) {
        let _ = self
            .service
            .deregister_instance(self.service_name.clone(), self.group.clone(), self.instance.clone())
            .await;
        tracing::info!(service = %self.service_name, "已从 Nacos 注册中心注销");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_section_returns_none() {
        let nacos = NacosSection::default(); // enabled=false
        let naming = NamingSection::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(NamingRegistry::connect(&nacos, &naming)).unwrap();
        assert!(r.is_none(), "未启用时不发起任何网络请求");
    }

    #[test]
    fn naming_disabled_returns_none() {
        let nacos = NacosSection {
            enabled: true,
            ..Default::default()
        };
        let naming = NamingSection::default(); // enabled=false
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(NamingRegistry::connect(&nacos, &naming)).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn empty_service_name_returns_none() {
        let nacos = NacosSection {
            enabled: true,
            ..Default::default()
        };
        let naming = NamingSection {
            enabled: true,
            service_name: String::new(),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(NamingRegistry::connect(&nacos, &naming)).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn metadata_parsing() {
        let naming = NamingSection {
            metadata: vec!["protocol=http".into(), "domain=alliance".into(), "noeq".into()],
            ..Default::default()
        };
        let m = naming.metadata_map();
        assert_eq!(m.get("protocol").map(String::as_str), Some("http"));
        assert_eq!(m.get("domain").map(String::as_str), Some("alliance"));
        assert_eq!(m.get("noeq").map(String::as_str), Some(""));
    }

    /// 不可达注册中心：connect 返回 Ok(None)（告警降级，不阻断启动）
    #[test]
    fn unreachable_server_degrades() {
        let nacos = NacosSection {
            enabled: true,
            server_addr: "127.0.0.1:1".to_string(),
            ..Default::default()
        };
        let naming = NamingSection {
            enabled: true,
            service_name: "mox-alliance-test".into(),
            port: 3100,
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(NamingRegistry::connect(&nacos, &naming)).unwrap();
        assert!(r.is_none(), "注册中心不可达应降级（Ok(None)），不阻断服务启动");
    }
}
