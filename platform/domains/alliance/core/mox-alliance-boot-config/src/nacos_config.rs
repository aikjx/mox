//! # Nacos 配置中心存储（feature = "nacos"）
//!
//! 基于官方 **nacos-group/nacos-sdk-rust**（crates.io `nacos-sdk`）ConfigService：
//!
//! - 启动时 `get_config(dataId, group)` 拉取远程完整 yml；
//! - `add_listener` 注册 watch 监听，配置变更自动更新内存缓存并广播（热更新通道）；
//! - 经 [`ConfigStore`] trait 接入配置源链，与本地 yml 组成「远程优先、失败降级」链。
//!
//! 本模块仅在 `features = ["nacos"]` 时编译，默认不引入任何 SDK 依赖。

use std::sync::Arc;

use async_trait::async_trait;
use nacos_sdk::api::config::{ConfigChangeListener, ConfigResponse, ConfigServiceBuilder};
use nacos_sdk::api::props::ClientProps;
use tokio::sync::watch;

use crate::config_store::{ConfigStore, ConfigStoreError};
use crate::NacosSection;

/// 监听器：配置变更 → 更新内存缓存 + 广播（watch channel）
///
/// 注意：`notify` 由 nacos-sdk 的客户端线程池回调（非 tokio runtime 上下文），
/// 因此缓存锁必须用 **std::sync::Mutex**（任何线程可直接锁），不能用 tokio RwLock 的
/// `blocking_write()`（在无 runtime 线程上会 panic「Cannot block the current thread」）。
struct CacheListener {
    cache: Arc<std::sync::Mutex<Option<String>>>,
    tx: watch::Sender<Option<String>>,
}

impl ConfigChangeListener for CacheListener {
    fn notify(&self, resp: ConfigResponse) {
        let content = resp.content().clone();
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some(content.clone());
        }
        let _ = self.tx.send(Some(content));
        tracing::info!(
            data_id = %resp.data_id(),
            group = %resp.group(),
            "Nacos 配置已热更新（watch 命中）"
        );
    }
}

/// Nacos 配置中心存储：绑定单个 dataId，支持 watch 热更新。
pub struct NacosConfigStore {
    name: &'static str,
    data_id: String,
    group: String,
    cache: Arc<std::sync::Mutex<Option<String>>>,
    changed: watch::Receiver<Option<String>>,
}

impl NacosConfigStore {
    /// 连接 Nacos 并完成「首次拉取 + 注册 watch 监听」。
    ///
    /// - `enabled=false` 或 `data_id` 为空 → 返回 `Ok(None)`（不启用，不发起网络请求）；
    /// - 连接 / 首次拉取失败 → 返回 `Err`（由配置链告警降级到本地 yml）。
    pub async fn connect(section: &NacosSection) -> anyhow::Result<Option<Self>> {
        if !section.enabled || section.data_id.trim().is_empty() {
            return Ok(None);
        }

        let props = ClientProps::new()
            .server_addr(&section.server_addr)
            .namespace(&section.namespace)
            .app_name("mox-alliance");
        if !section.username.is_empty() {
            // 注意：nacos-sdk 认证（auth_username/auth_password）属于 auth-by-http feature；
            // 当前 boot-config 仅启用 config 能力（无鉴权直连），如 Nacos 服务端要求认证，
            // 需在 Cargo.toml 的 nacos-sdk 增加 auth-by-http feature 后在此接入。
            tracing::warn!(
                "NacosSection 配置了 username/password，但当前构建未启用 nacos-sdk auth-by-http 认证，将以无鉴权方式连接"
            );
        }
        let service = ConfigServiceBuilder::new(props).build().await?;

        // 首次拉取（远程完整配置，整体覆盖本地 yml）
        let resp = service
            .get_config(section.data_id.clone(), section.group.clone())
            .await?;
        let initial = Some(resp.content().clone());
        let cache = Arc::new(std::sync::Mutex::new(initial.clone()));
        let (tx, rx) = watch::channel(initial);

        // watch 热更新
        let listener = Arc::new(CacheListener {
            cache: cache.clone(),
            tx: tx.clone(),
        });
        service
            .add_listener(section.data_id.clone(), section.group.clone(), listener)
            .await?;

        tracing::info!(
            server = %section.server_addr,
            namespace = %section.namespace,
            data_id = %section.data_id,
            group = %section.group,
            "Nacos 配置中心已连接并注册 watch 监听"
        );
        Ok(Some(Self {
            name: "nacos",
            data_id: section.data_id.clone(),
            group: section.group.clone(),
            cache,
            changed: rx,
        }))
    }

    /// 订阅配置热更新通知（`watch::Receiver`；`Some` = 最新内容，`None` = 配置已删除）。
    pub fn changed(&self) -> &watch::Receiver<Option<String>> {
        &self.changed
    }
}

#[async_trait]
impl ConfigStore for NacosConfigStore {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn load_raw(&self, _key: &str) -> Result<Option<String>, ConfigStoreError> {
        // 返回 watch 维护的内存缓存（get_config 初拉 + add_listener 热更新）
        // std Mutex 快速短锁（回调线程与 async 均可用）
        let content = self.cache.lock().map(|g| g.clone()).unwrap_or(None);
        Ok(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_section_returns_none() {
        // enabled=false：不应发起任何网络请求，直接返回 Ok(None)
        let section = NacosSection::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(NacosConfigStore::connect(&section)).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn empty_data_id_returns_none() {
        let section = NacosSection {
            enabled: true,
            data_id: String::new(),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(NacosConfigStore::connect(&section)).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn unreachable_server_returns_err() {
        // 指向不可达地址：connect 必须返回 Err（而非静默成功），由配置链降级
        let section = NacosSection {
            enabled: true,
            server_addr: "127.0.0.1:1".to_string(),
            data_id: "mox-alliance-scheduler.yml".to_string(),
            ..Default::default()
        };
        let rt = tokio::runtime::Runtime::new().unwrap();
        let r = rt.block_on(NacosConfigStore::connect(&section));
        assert!(
            r.is_err(),
            "Nacos 不可达时必须显式报错，由配置链降级到本地 yml"
        );
    }
}
