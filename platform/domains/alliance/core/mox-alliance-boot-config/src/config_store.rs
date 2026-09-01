//! # 可插拔配置源（ConfigStore）
//!
//! 把「配置从哪里来」从「直接读 yml 文件」抽象为「配置源链」：
//!
//! ```text
//! 内置默认 < FileConfigStore(本地 yml) < NacosConfigStore(远程配置中心) < env
//! ```
//!
//! - [`ConfigStore`]：配置源抽象，按 key 返回原始配置文本。
//! - [`FileConfigStore`]：本地 `{base_dir}/{key}.yml`。
//! - [`MemoryConfigStore`]：内存内置默认 / 测试。
//! - [`ConfigStoreChain`]：按优先级逐源尝试，**容错降级**（上游 Err 时告警并落到下一源）。
//!
//! 本模块为纯本地实现，不引入任何异步运行时/外部 SDK 依赖。

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use thiserror::Error;

/// 配置源读取错误
#[derive(Debug, Error)]
pub enum ConfigStoreError {
    /// 配置源读取失败（I/O、网络、认证等）
    #[error("配置源[{store}]读取失败: {message}")]
    ReadFailed {
        /// 配置源名称
        store: &'static str,
        /// 失败原因
        message: String,
    },
}

/// 配置源抽象：按 `key` 返回原始配置文本。
///
/// - `Ok(Some(text))`：本配置源命中该 key。
/// - `Ok(None)`：本配置源无该 key（不视为错误）。
/// - `Err(e)`：读取失败（应由配置链告警后降级）。
#[async_trait]
pub trait ConfigStore: Send + Sync {
    /// 配置源名称（用于日志与错误定位）
    fn name(&self) -> &'static str;

    /// 按 key 读取原始配置文本。
    async fn load_raw(&self, key: &str) -> Result<Option<String>, ConfigStoreError>;
}

/// 本地文件配置源：`{base_dir}/{key}.yml`
#[derive(Debug, Clone)]
pub struct FileConfigStore {
    base_dir: PathBuf,
}

impl FileConfigStore {
    /// 创建文件配置源，`base_dir` 为 yml 文件所在目录。
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// 返回给定 key 对应的完整文件路径（`{base_dir}/{key}.yml`）。
    pub fn resolve_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{key}.yml"))
    }
}

#[async_trait]
impl ConfigStore for FileConfigStore {
    fn name(&self) -> &'static str {
        "file"
    }

    async fn load_raw(&self, key: &str) -> Result<Option<String>, ConfigStoreError> {
        let path = self.resolve_path(key);
        match std::fs::read_to_string(&path) {
            Ok(content) => Ok(Some(content)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ConfigStoreError::ReadFailed {
                store: self.name(),
                message: format!("{}: {e}", path.display()),
            }),
        }
    }
}

/// 内存配置源：内置默认值或测试桩。
#[derive(Debug, Clone)]
pub struct MemoryConfigStore {
    name: &'static str,
    content: String,
}

impl MemoryConfigStore {
    /// 创建内存配置源，任意 key 都返回同一份内容。
    pub fn new(name: &'static str, content: impl Into<String>) -> Self {
        Self {
            name,
            content: content.into(),
        }
    }
}

#[async_trait]
impl ConfigStore for MemoryConfigStore {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn load_raw(&self, _key: &str) -> Result<Option<String>, ConfigStoreError> {
        Ok(Some(self.content.clone()))
    }
}

/// 配置源链：按优先级逐源尝试，**容错降级**。
///
/// 规则（企业级配置高可用）：
/// - `Ok(Some)`：命中即返回（高优先级源优先）。
/// - `Ok(None)`：本源无此 key，尝试下一源。
/// - `Err`：告警并尝试下一源（上游配置中心不可达时自动降级到本地 yml）。
/// - 全部未命中返回 `Ok(None)`。
pub struct ConfigStoreChain {
    stores: Vec<Box<dyn ConfigStore>>,
}

impl ConfigStoreChain {
    /// 创建配置链。`stores` 顺序即优先级（高 → 低）。
    pub fn new(stores: Vec<Box<dyn ConfigStore>>) -> Self {
        Self { stores }
    }

    /// 在链头追加一个配置源（更高优先级）。
    pub fn prepend(mut self, store: Box<dyn ConfigStore>) -> Self {
        self.stores.insert(0, store);
        self
    }
}

#[async_trait]
impl ConfigStore for ConfigStoreChain {
    fn name(&self) -> &'static str {
        "chain"
    }

    async fn load_raw(&self, key: &str) -> Result<Option<String>, ConfigStoreError> {
        for store in &self.stores {
            match store.load_raw(key).await {
                Ok(Some(content)) => return Ok(Some(content)),
                Ok(None) => continue,
                Err(e) => {
                    tracing::warn!(
                        store = store.name(),
                        key = key,
                        err = %e,
                        "配置源读取失败，自动降级到下一来源"
                    );
                    continue;
                }
            }
        }
        Ok(None)
    }
}

/// 从配置源链读取 yml 文本（key = 配置名，如 `alliance-scheduler`）。
///
/// 返回 `Ok(Some(text))` 表示某配置源命中；`Ok(None)` 表示全链未命中（调用方决定是否用内置默认）。
pub async fn load_yaml_from_chain(
    chain: &dyn ConfigStore,
    key: &str,
) -> anyhow::Result<Option<String>> {
    chain.load_raw(key).await.map_err(|e| anyhow::anyhow!(e))
}

/// 判断文件是否存在于磁盘（配置校验辅助）。
pub fn file_exists(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn temp_dir() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "mox_boot_cfg_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn file_store_hit_and_miss() {
        let dir = temp_dir();
        let path = dir.join("scheduler.yml");
        std::fs::write(&path, "server:\n  port: 3100\n").unwrap();

        let store = FileConfigStore::new(&dir);
        let hit = store.load_raw("scheduler").await.unwrap();
        assert_eq!(hit, Some("server:\n  port: 3100\n".to_string()));

        let miss = store.load_raw("not-exist").await.unwrap();
        assert_eq!(miss, None);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn memory_store_returns_content() {
        let store = MemoryConfigStore::new("test", "a: 1\n");
        assert_eq!(store.load_raw("any").await.unwrap(), Some("a: 1\n".to_string()));
    }

    /// 链：高优先级命中 → 不再查低优先级
    #[tokio::test]
    async fn chain_returns_first_hit() {
        let high = MemoryConfigStore::new("high", "from-high");
        let low = MemoryConfigStore::new("low", "from-low");
        let chain = ConfigStoreChain::new(vec![Box::new(high), Box::new(low)]);
        assert_eq!(chain.load_raw("k").await.unwrap(), Some("from-high".to_string()));
    }

    /// 链：高优先级 None → 落到低优先级
    #[tokio::test]
    async fn chain_falls_through_on_miss() {
        // MemoryConfigStore 总是 Some；用"不存在文件"模拟 miss
        let dir = temp_dir();
        let file = FileConfigStore::new(&dir); // 空目录 → None
        let low = MemoryConfigStore::new("low", "from-low");
        let chain = ConfigStoreChain::new(vec![Box::new(file), Box::new(low)]);
        assert_eq!(chain.load_raw("k").await.unwrap(), Some("from-low".to_string()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// 链：上游读取失败 → 告警降级到下一源（不阻断）
    #[tokio::test]
    async fn chain_degrades_on_error() {
        struct FailingStore;
        #[async_trait]
        impl ConfigStore for FailingStore {
            fn name(&self) -> &'static str {
                "failing"
            }
            async fn load_raw(&self, _key: &str) -> Result<Option<String>, ConfigStoreError> {
                Err(ConfigStoreError::ReadFailed {
                    store: "failing",
                    message: "模拟网络不可达".into(),
                })
            }
        }
        let low = MemoryConfigStore::new("low", "from-low");
        let chain = ConfigStoreChain::new(vec![Box::new(FailingStore), Box::new(low)]);
        let got = chain.load_raw("k").await.unwrap();
        assert_eq!(got, Some("from-low".to_string()));
    }

    /// 链：全链未命中 → Ok(None)
    #[tokio::test]
    async fn chain_all_miss_returns_none() {
        let dir = temp_dir();
        let chain = ConfigStoreChain::new(vec![Box::new(FileConfigStore::new(&dir))]);
        assert_eq!(chain.load_raw("k").await.unwrap(), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn chain_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn ConfigStore>>();
    }
}
