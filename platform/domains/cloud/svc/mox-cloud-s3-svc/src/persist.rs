// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! S3 协议层 → store-core 持久化钩子。
//!
//! S3 协议层的 `InMemoryStorage` 是**测试双**（保持现有套件绿）；生产部署时通过
//! [`StoreCorePersist`] 将写 chokepoint（PutObject/CopyObject/CompleteMultipart/
//! DeleteObject/DeleteMultiple）的数据**镜像**到 `mox-cloud-store-core` 真实后端
//! （内容寻址 + 引用计数 GC + 原子写），实现"内存主路径 + 异步落盘"双写。
//!
//! 镜像语义：
//! - **尽力而为**：镜像失败只记录 `tracing::error`，不影响 S3 协议主路径
//!   （内存已写，读一致性由内存保证；store-core 侧由"目标空读回源"自愈）。
//! - **有界通道 + FIFO**：写请求经有界 `sync_channel` 投递给专属 writer 线程，
//!   天然保序；[`PersistSink::flush`] 提供同步等待点（测试/关停）。
//! - **零阻塞主路径**：writer 线程持有独立 current-thread tokio Runtime
//!   `block_on` store-core 异步接口，S3 axum 线程不被阻塞。
//!
//! `InMemoryStorage.persist = None` 时本模块零开销，现有测试行为完全不变。

use bytes::Bytes;
use mox_cloud_store_core::StoreBackend;
use parking_lot::Mutex;
use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::thread::{self, JoinHandle};

/// 写镜像后端契约（同步接口，供 S3 协议层写 chokepoint 调用）。
pub trait PersistSink: Send + Sync {
    /// 镜像写入对象（覆盖/新建）。
    fn mirror_put(&self, bucket: &str, key: &str, data: &[u8]);
    /// 镜像删除对象。
    fn mirror_delete(&self, bucket: &str, key: &str);
    /// 等待所有已投递的镜像请求落盘完成（测试/关停同步点）。
    fn flush(&self);
}

/// 镜像命令（FIFO 顺序）。
enum PersistCmd {
    Put { path: String, data: Vec<u8> },
    Delete { path: String },
    Flush { ack: Sender<()> },
}

/// 将写 chokepoint 镜像到 store-core 真实后端的实现。
pub struct StoreCorePersist {
    tx: Mutex<Option<SyncSender<PersistCmd>>>,
    worker: Option<JoinHandle<()>>,
}

impl StoreCorePersist {
    /// 以装配好的 store-core 后端构造持久化钩子。
    pub fn new(backend: StoreBackend) -> Self {
        let (tx, rx) = mpsc::sync_channel::<PersistCmd>(64);
        let worker = thread::Builder::new()
            .name("store-core-persist".into())
            .spawn(move || Self::worker_loop(rx, backend))
            .expect("spawn store-core persist worker");
        Self {
            tx: Mutex::new(Some(tx)),
            worker: Some(worker),
        }
    }

    /// writer 线程主循环：独立 runtime 消费命令并 block_on 落盘。
    fn worker_loop(rx: Receiver<PersistCmd>, backend: StoreBackend) {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("创建 store-core persist runtime 失败: {e}");
                return;
            }
        };
        while let Ok(cmd) = rx.recv() {
            match cmd {
                PersistCmd::Put { path, data } => {
                    if let Err(e) = rt.block_on(backend.object.put(
                        &path,
                        "application/octet-stream",
                        Bytes::from(data),
                    )) {
                        tracing::error!("镜像 put {path} 失败: {e}");
                    }
                }
                PersistCmd::Delete { path } => {
                    if let Err(e) = rt.block_on(backend.object.delete(&path)) {
                        tracing::error!("镜像 delete {path} 失败: {e}");
                    }
                }
                PersistCmd::Flush { ack } => {
                    let _ = ack.send(());
                }
            }
        }
    }
}

impl PersistSink for StoreCorePersist {
    fn mirror_put(&self, bucket: &str, key: &str, data: &[u8]) {
        let path = logical_path(bucket, key);
        if let Some(tx) = self.tx.lock().as_ref() {
            let _ = tx.send(PersistCmd::Put { path, data: data.to_vec() });
        }
    }

    fn mirror_delete(&self, bucket: &str, key: &str) {
        let path = logical_path(bucket, key);
        if let Some(tx) = self.tx.lock().as_ref() {
            let _ = tx.send(PersistCmd::Delete { path });
        }
    }

    fn flush(&self) {
        let (ack_tx, ack_rx) = mpsc::channel();
        let ok = self
            .tx
            .lock()
            .as_ref()
            .map(|tx| tx.send(PersistCmd::Flush { ack: ack_tx }).is_ok())
            .unwrap_or(false);
        if ok {
            let _ = ack_rx.recv();
        }
    }
}

impl Drop for StoreCorePersist {
    fn drop(&mut self) {
        // 置 None 丢弃发送端 → channel 关闭 → worker 的 recv 返回 Err → 线程退出
        *self.tx.get_mut() = None;
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

impl fmt::Debug for StoreCorePersist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StoreCorePersist {{ enabled: true }}")
    }
}

/// 逻辑路径：`{bucket}/{key}`（与 store-core / filer 桥接同构，FS/S3 可互换）。
/// store-core 侧以 sha256(key) 命名文件，天然免疫 `../` 路径穿越。
fn logical_path(bucket: &str, key: &str) -> String {
    format!(
        "{}/{}",
        bucket.trim_matches('/'),
        key.trim_start_matches('/')
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_cloud_store_core::{create_backend, BackendKind, StoreConfig};
    use std::sync::Arc;

    fn fs_backend(dir: &std::path::Path) -> StoreBackend {
        let cfg = StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir.to_path_buf(),
            ..Default::default()
        };
        create_backend(&cfg).unwrap()
    }

    #[test]
    fn mirror_put_then_read_back_from_store_core() {
        let dir = tempfile::tempdir().unwrap();
        let persist = StoreCorePersist::new(fs_backend(dir.path()));

        persist.mirror_put("docs", "a.md", "# 标题".as_bytes());
        persist.mirror_put("docs", "b.md", b"content-b");
        persist.mirror_delete("docs", "b.md");
        persist.flush();

        // 直接用 store-core 后端读回，验证镜像落盘
        let backend = fs_backend(dir.path());
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let got = rt.block_on(backend.object.get("docs/a.md")).unwrap();
        assert_eq!(&got[..], "# 标题".as_bytes());

        // b.md 已删除
        assert!(matches!(
            rt.block_on(backend.object.get("docs/b.md")),
            Err(mox_cloud_store_core::StoreError::NotFound { .. })
        ));
    }

    #[test]
    fn persist_sink_is_send_sync() {
        let dir = tempfile::tempdir().unwrap();
        let sink: Arc<dyn PersistSink> = Arc::new(StoreCorePersist::new(fs_backend(dir.path())));
        // Send + Sync 是 trait 对象约束；此处仅做编译期断言
        let _ = sink;
    }
}
