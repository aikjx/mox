// =============================================================================
// 配置热更新（Config Watcher）
// =============================================================================

use crate::config::{Config, ConfigManager};
use crate::source::ConfigSource;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

// =============================================================================
// 监听事件
// =============================================================================

/// 配置变更事件
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// 事件类型
    pub event_type: WatchEventType,
    /// 变更的配置键
    pub changed_keys: Vec<String>,
    /// 新版本号
    pub new_version: u64,
    /// 旧版本号
    pub old_version: u64,
    /// 事件时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 监听事件类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventType {
    /// 配置已更新
    Updated,
    /// 配置已重载
    Reloaded,
    /// 配置源错误
    Error,
}

// =============================================================================
// 监听回调
// =============================================================================

/// 配置变更回调
pub type WatchCallback = Arc<dyn Fn(&WatchEvent) + Send + Sync>;

// =============================================================================
// 配置监听器
// =============================================================================

/// 配置监听器
///
/// 支持文件变更监听和远程轮询，配置变更时通知所有订阅者。
pub struct ConfigWatcher {
    manager: ConfigManager,
    source: Arc<dyn ConfigSource>,
    tx: broadcast::Sender<WatchEvent>,
    callbacks: parking_lot::RwLock<Vec<WatchCallback>>,
    poll_interval: Duration,
}

impl ConfigWatcher {
    /// 创建新的配置监听器
    pub fn new(manager: ConfigManager, source: Arc<dyn ConfigSource>) -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            manager,
            source,
            tx,
            callbacks: parking_lot::RwLock::new(vec![]),
            poll_interval: Duration::from_secs(30),
        }
    }

    /// 设置轮询间隔
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// 订阅配置变更事件
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.tx.subscribe()
    }

    /// 注册变更回调
    pub fn on_change(&self, callback: WatchCallback) {
        self.callbacks.write().push(callback);
    }

    /// 手动触发配置重载
    pub async fn reload(&self) -> Result<(), String> {
        let old_version = self.manager.version();

        match self.source.load().await {
            Ok(new_config) => {
                self.manager.merge(&new_config);
                let new_version = self.manager.version();

                let event = WatchEvent {
                    event_type: WatchEventType::Reloaded,
                    changed_keys: vec![],
                    new_version,
                    old_version,
                    timestamp: chrono::Utc::now(),
                };

                self.notify(&event);
                Ok(())
            }
            Err(e) => {
                let event = WatchEvent {
                    event_type: WatchEventType::Error,
                    changed_keys: vec![],
                    new_version: old_version,
                    old_version,
                    timestamp: chrono::Utc::now(),
                };
                self.notify(&event);
                Err(format!("配置重载失败: {}", e))
            }
        }
    }

    /// 启动后台轮询（需要在 tokio 运行时中调用）
    pub fn start_polling(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(self.poll_interval);
            loop {
                interval.tick().await;
                if let Err(e) = self.reload().await {
                    tracing::warn!("配置轮询失败: {}", e);
                }
            }
        })
    }

    /// 通知所有订阅者和回调
    fn notify(&self, event: &WatchEvent) {
        let _ = self.tx.send(event.clone());
        let callbacks = self.callbacks.read();
        for callback in callbacks.iter() {
            callback(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigValue;
    use crate::source::MemorySource;

    #[tokio::test]
    async fn watcher_reload() {
        let manager = ConfigManager::new("test");
        let mut source = MemorySource::new();
        source.set("key", ConfigValue::from("value"));

        let watcher = ConfigWatcher::new(manager.clone(), Arc::new(source));
        let mut rx = watcher.subscribe();

        // 执行重载
        watcher.reload().await.unwrap();

        // 验证配置已更新
        assert_eq!(manager.get_string("key"), Some("value".to_string()));

        // 验证事件已发送
        let event = rx.try_recv().unwrap();
        assert_eq!(event.event_type, WatchEventType::Reloaded);
    }

    #[tokio::test]
    async fn watcher_callback() {
        let manager = ConfigManager::new("test");
        let mut source = MemorySource::new();
        source.set("key", ConfigValue::from("value"));

        let watcher = Arc::new(ConfigWatcher::new(manager.clone(), Arc::new(source)));

        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        watcher.on_change(Arc::new(move |_event| {
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        }));

        watcher.reload().await.unwrap();

        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
