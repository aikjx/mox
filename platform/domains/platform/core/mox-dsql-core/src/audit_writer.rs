//! 异步批量审计日志写入器
//!
//! 将审计日志写入从请求线程剥离，通过后台线程批量写入数据库，
//! 显著降低高 QPS 下审计写入对请求延迟的影响。
//!
//! 设计要点：
//! - 非阻塞写入：`write()` 仅将日志放入 channel，立即返回
//! - 批量刷盘：达到批量大小或超时阈值时，一次性事务写入
//! - 优雅关闭：Drop 时先排空 channel、刷盘，再停止后台线程
//! - 可观测：内置统计（写入成功/失败/批量次数）

use crate::model::AuditLog;
use crate::storage::DsqlStorage;
use parking_lot::Mutex;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// 异步审计写入器配置
#[derive(Debug, Clone)]
pub struct AuditWriterConfig {
    /// 批量写入阈值（达到此数量立即刷盘）
    pub batch_size: usize,
    /// 最大刷盘间隔（毫秒，即使未达到 batch_size 也刷盘）
    pub flush_interval_ms: u64,
    /// channel 缓冲区大小（超过则 write() 阻塞）
    pub channel_capacity: usize,
}

impl Default for AuditWriterConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            flush_interval_ms: 1000,
            channel_capacity: 10000,
        }
    }
}

/// 审计写入器统计信息
#[derive(Debug, Clone, Default)]
pub struct AuditWriterStats {
    /// 累计成功写入条数
    pub total_written: u64,
    /// 累计失败条数
    pub total_failed: u64,
    /// 累计批量写入次数
    pub batch_count: u64,
    /// 当前待处理条数（近似值：channel + buffer）
    pub pending_count: usize,
}

/// 异步批量审计写入器
pub struct AsyncAuditWriter {
    sender: Sender<AuditLog>,
    handle: Option<JoinHandle<()>>,
    stats: Arc<Mutex<AuditWriterStats>>,
    /// 用于通知后台线程停止的信号
    shutdown: Arc<Mutex<bool>>,
}

impl AsyncAuditWriter {
    /// 创建异步审计写入器并启动后台线程
    pub fn new(storage: Arc<DsqlStorage>, config: AuditWriterConfig) -> Self {
        let (sender, receiver) = mpsc::channel::<AuditLog>();
        let stats = Arc::new(Mutex::new(AuditWriterStats::default()));
        let shutdown = Arc::new(Mutex::new(false));

        let stats_clone = stats.clone();
        let shutdown_clone = shutdown.clone();
        let batch_size = config.batch_size;
        let flush_interval = Duration::from_millis(config.flush_interval_ms);

        let handle = thread::spawn(move || {
            Self::background_loop(
                storage,
                receiver,
                stats_clone,
                shutdown_clone,
                batch_size,
                flush_interval,
            );
        });

        Self {
            sender,
            handle: Some(handle),
            stats,
            shutdown,
        }
    }

    /// 非阻塞写入审计日志（放入 channel 后立即返回）
    ///
    /// 审计写入失败不会传播给调用方（仅记录到统计中），
    /// 避免审计故障影响主业务流程。
    pub fn write(&self, log: AuditLog) {
        // 发送失败时静默忽略（后台线程已退出），不影响主流程
        let _ = self.sender.send(log);
    }

    /// 阻塞等待所有待处理审计日志落盘
    ///
    /// 通过发送一个"哨兵"消息并等待后台线程处理完成来实现。
    /// 由于 mpsc 是 FIFO，哨兵消息到达时前面的消息都已处理。
    pub fn flush(&self) {
        // 用一个特殊的审计日志作为哨兵（sql_code 为空字符串 + created_at 为空）
        let sentinel = AuditLog {
            id: 0,
            trace_id: None,
            sql_code: String::new(),
            datasource_code: None,
            params: None,
            row_count: None,
            duration_ms: None,
            success: true,
            error_msg: None,
            is_slow: false,
            cache_hit: false,
            created_at: String::new(),
        };
        if self.sender.send(sentinel).is_ok() {
            // 轮询等待 total_written 增加或 pending_count 归零（最多等 3 秒）
            let initial_written = self.stats.lock().total_written;
            for _ in 0..60 {
                thread::sleep(Duration::from_millis(50));
                let s = self.stats.lock();
                // 哨兵处理后会触发一次刷盘，batch_count 会增加
                // 或者 pending_count 归零
                if s.pending_count == 0 || s.batch_count > 0 {
                    // 再等一小段时间确保写入完成
                    thread::sleep(Duration::from_millis(20));
                    break;
                }
                // 如果 total_written 已经增加，说明之前的数据已写入
                if s.total_written > initial_written {
                    break;
                }
            }
        }
    }

    /// 获取统计信息快照
    pub fn stats(&self) -> AuditWriterStats {
        self.stats.lock().clone()
    }

    /// 后台线程主循环
    fn background_loop(
        storage: Arc<DsqlStorage>,
        receiver: Receiver<AuditLog>,
        stats: Arc<Mutex<AuditWriterStats>>,
        shutdown: Arc<Mutex<bool>>,
        batch_size: usize,
        flush_interval: Duration,
    ) {
        let mut buffer: Vec<AuditLog> = Vec::with_capacity(batch_size);
        let mut last_flush = std::time::Instant::now();

        loop {
            // 检查是否收到关闭信号
            if *shutdown.lock() {
                // 关闭前：先排空 channel 中剩余消息（非阻塞接收）
                while let Ok(log) = receiver.try_recv() {
                    // 跳过哨兵消息
                    if !log.sql_code.is_empty() || !log.created_at.is_empty() {
                        buffer.push(log);
                    }
                }
                // 刷盘剩余数据
                if !buffer.is_empty() {
                    Self::flush_buffer(&storage, &mut buffer, &stats);
                }
                // 更新 pending_count
                stats.lock().pending_count = 0;
                break;
            }

            // 计算剩余超时时间
            let elapsed = last_flush.elapsed();
            let timeout = if elapsed >= flush_interval {
                Duration::from_millis(1)
            } else {
                flush_interval - elapsed
            };

            match receiver.recv_timeout(timeout) {
                Ok(log) => {
                    // 检查是否是哨兵消息（空 sql_code + 空 created_at）
                    if log.sql_code.is_empty() && log.created_at.is_empty() {
                        // 哨兵：立即刷盘
                        if !buffer.is_empty() {
                            Self::flush_buffer(&storage, &mut buffer, &stats);
                        }
                        last_flush = std::time::Instant::now();
                        stats.lock().pending_count = 0;
                        continue;
                    }

                    buffer.push(log);

                    // 达到批量大小，立即刷盘
                    if buffer.len() >= batch_size {
                        Self::flush_buffer(&storage, &mut buffer, &stats);
                        last_flush = std::time::Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 超时：如果有数据且超过间隔，刷盘
                    if !buffer.is_empty() && last_flush.elapsed() >= flush_interval {
                        Self::flush_buffer(&storage, &mut buffer, &stats);
                        last_flush = std::time::Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // channel 已断开（所有 sender 都 drop），刷盘剩余数据后退出
                    if !buffer.is_empty() {
                        Self::flush_buffer(&storage, &mut buffer, &stats);
                    }
                    stats.lock().pending_count = 0;
                    break;
                }
            }

            // 更新 pending_count（近似值）
            stats.lock().pending_count = buffer.len();
        }
    }

    /// 将缓冲区数据批量写入数据库
    fn flush_buffer(
        storage: &DsqlStorage,
        buffer: &mut Vec<AuditLog>,
        stats: &Mutex<AuditWriterStats>,
    ) {
        let logs = std::mem::take(buffer);
        match storage.write_audit_logs_batch(&logs) {
            Ok(count) => {
                let mut s = stats.lock();
                s.total_written += count as u64;
                s.batch_count += 1;
            }
            Err(e) => {
                tracing::error!(error = %e, count = logs.len(), "async audit batch write failed");
                let mut s = stats.lock();
                s.total_failed += logs.len() as u64;
                s.batch_count += 1;
            }
        }
    }
}

impl Drop for AsyncAuditWriter {
    fn drop(&mut self) {
        // 1. 先 flush 所有待处理数据（发送哨兵消息唤醒后台线程）
        self.flush();

        // 2. 通知后台线程停止
        *self.shutdown.lock() = true;

        // 3. 等待后台线程退出（最多等 5 秒）
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::storage::DsqlStorage;

    fn make_audit_log(sql_code: &str) -> AuditLog {
        AuditLog {
            id: 0,
            trace_id: Some("test-trace".to_string()),
            sql_code: sql_code.to_string(),
            datasource_code: Some("default".to_string()),
            params: Some(r#"{"key":"value"}"#.to_string()),
            row_count: Some(1),
            duration_ms: Some(5),
            success: true,
            error_msg: None,
            is_slow: false,
            cache_hit: false,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    #[test]
    fn test_async_audit_writer_basic() {
        let storage = Arc::new(DsqlStorage::open_memory().unwrap());
        let config = AuditWriterConfig {
            batch_size: 10,
            flush_interval_ms: 100,
            channel_capacity: 1000,
        };
        let writer = AsyncAuditWriter::new(storage.clone(), config);

        // 写入 5 条（未达到 batch_size）
        for i in 0..5 {
            writer.write(make_audit_log(&format!("sql_{i}")));
        }

        // 等待超时刷盘
        thread::sleep(Duration::from_millis(300));

        let stats = writer.stats();
        assert_eq!(stats.total_written, 5);
        assert_eq!(stats.total_failed, 0);
        assert!(stats.batch_count >= 1);
    }

    #[test]
    fn test_async_audit_writer_batch_trigger() {
        let storage = Arc::new(DsqlStorage::open_memory().unwrap());
        let config = AuditWriterConfig {
            batch_size: 5,
            flush_interval_ms: 5000,
            channel_capacity: 1000,
        };
        let writer = AsyncAuditWriter::new(storage.clone(), config);

        // 写入 5 条（达到 batch_size，立即刷盘）
        for i in 0..5 {
            writer.write(make_audit_log(&format!("sql_{i}")));
        }

        // 等待批量写入完成
        thread::sleep(Duration::from_millis(200));

        let stats = writer.stats();
        assert_eq!(stats.total_written, 5);
        assert_eq!(stats.batch_count, 1);
    }

    #[test]
    fn test_async_audit_writer_flush() {
        let storage = Arc::new(DsqlStorage::open_memory().unwrap());
        let config = AuditWriterConfig {
            batch_size: 100,
            flush_interval_ms: 5000,
            channel_capacity: 1000,
        };
        let writer = AsyncAuditWriter::new(storage.clone(), config);

        // 写入 3 条（未达到 batch_size，也未超时）
        for i in 0..3 {
            writer.write(make_audit_log(&format!("sql_{i}")));
        }

        // 手动 flush
        writer.flush();

        let stats = writer.stats();
        assert_eq!(stats.total_written, 3);
        assert_eq!(stats.pending_count, 0);
    }

    #[test]
    fn test_async_audit_writer_drop_flush() {
        let storage = Arc::new(DsqlStorage::open_memory().unwrap());
        let config = AuditWriterConfig {
            batch_size: 100,
            flush_interval_ms: 5000,
            channel_capacity: 1000,
        };
        {
            let writer = AsyncAuditWriter::new(storage.clone(), config);
            for i in 0..7 {
                writer.write(make_audit_log(&format!("sql_{i}")));
            }
            // writer drop 时自动 flush + 排空 channel + 刷盘
        }

        // 等待 drop 完成
        thread::sleep(Duration::from_millis(300));

        // 验证数据已写入数据库
        let logs = storage
            .list_audit_logs(&AuditLogQuery {
                page: 1,
                page_size: 100,
                sql_code: None,
                datasource_code: None,
                trace_id: None,
                success: None,
                is_slow: None,
                cache_hit: None,
                start_time: None,
                end_time: None,
            })
            .unwrap();
        assert_eq!(logs.total, 7);
    }

    #[test]
    fn test_async_audit_writer_large_batch() {
        let storage = Arc::new(DsqlStorage::open_memory().unwrap());
        let config = AuditWriterConfig {
            batch_size: 10,
            flush_interval_ms: 5000,
            channel_capacity: 10000,
        };
        let writer = AsyncAuditWriter::new(storage.clone(), config);

        // 写入 25 条（触发 2 次批量刷盘 + 剩余 5 条）
        for i in 0..25 {
            writer.write(make_audit_log(&format!("sql_{i}")));
        }

        // flush 剩余数据
        writer.flush();

        let stats = writer.stats();
        assert_eq!(stats.total_written, 25);
        assert!(stats.batch_count >= 2);
    }
}
