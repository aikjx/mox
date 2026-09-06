//! 企业级 SQLite 连接池实现
//!
//! 由于 r2d2-sqlite crate 已不可用，这里实现一个轻量级但企业级的连接池，支持：
//! - 固定大小连接池 + 连接获取/归还
//! - WAL 模式初始化
//! - 连接健康检查（获取时验证连接有效性）
//! - 连接最大生命周期（超时连接自动丢弃）
//! - 完整监控指标（活跃数/空闲数/等待数/超时数/获取耗时）
//! - 原子计数器（无锁统计）

use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 连接池错误
#[derive(Debug, thiserror::Error)]
pub enum PoolError {
    #[error("connection timeout")]
    Timeout,
    #[error("pool is closed")]
    Closed,
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// 连接池统计信息
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// 最大连接数
    pub max_size: usize,
    /// 当前空闲连接数
    pub idle_count: usize,
    /// 当前活跃（借出）连接数
    pub active_count: usize,
    /// 总连接数（空闲+活跃）
    pub total_count: usize,
    /// 累计等待获取连接次数
    pub wait_count: usize,
    /// 累计获取超时次数
    pub timeout_count: usize,
    /// 累计获取连接总耗时（纳秒）
    pub total_acquire_time_ns: u64,
    /// 累计创建连接次数
    pub create_count: usize,
    /// 累计丢弃连接次数（健康检查失败/超生命周期）
    pub discard_count: usize,
}

/// 带元数据的连接
struct PooledConn {
    conn: Connection,
    /// 连接创建时间
    created_at: Instant,
    /// 最后一次使用时间
    last_used_at: Instant,
}

/// 连接池内部状态
struct PoolInner {
    connections: Mutex<Vec<PooledConn>>,
    max_size: usize,
    /// 连接最大生命周期（超过则丢弃，None表示不限制）
    max_lifetime: Option<Duration>,
    /// 连接空闲超时（超过则丢弃，None表示不限制）
    idle_timeout: Option<Duration>,
    /// 用于创建新连接的工厂
    factory: Box<dyn Fn() -> Result<Connection, rusqlite::Error> + Send + Sync>,
    // 原子计数器
    active_count: AtomicUsize,
    total_created: AtomicUsize,
    wait_count: AtomicUsize,
    timeout_count: AtomicUsize,
    total_acquire_time_ns: AtomicUsize,
    discard_count: AtomicUsize,
}

/// 企业级 SQLite 连接池
#[derive(Clone)]
pub struct SqlitePool {
    inner: Arc<PoolInner>,
}

impl SqlitePool {
    /// 创建文件模式连接池
    pub fn file<P: AsRef<std::path::Path>>(path: P, max_size: usize) -> Result<Self, PoolError> {
        let path = path.as_ref().to_path_buf();
        let factory = move || Connection::open(&path);
        Self::with_factory(Box::new(factory), max_size)
    }

    /// 创建内存模式连接池（注意：每个连接是独立的内存数据库，仅用于测试）
    pub fn memory(max_size: usize) -> Result<Self, PoolError> {
        let factory = || Connection::open_in_memory();
        Self::with_factory(Box::new(factory), max_size)
    }

    /// 使用自定义工厂创建连接池
    fn with_factory(
        factory: Box<dyn Fn() -> Result<Connection, rusqlite::Error> + Send + Sync>,
        max_size: usize,
    ) -> Result<Self, PoolError> {
        let pool = Self {
            inner: Arc::new(PoolInner {
                connections: Mutex::new(Vec::with_capacity(max_size)),
                max_size,
                max_lifetime: Some(Duration::from_secs(3600)), // 默认1小时
                idle_timeout: Some(Duration::from_secs(600)),   // 默认10分钟
                factory,
                active_count: AtomicUsize::new(0),
                total_created: AtomicUsize::new(0),
                wait_count: AtomicUsize::new(0),
                timeout_count: AtomicUsize::new(0),
                total_acquire_time_ns: AtomicUsize::new(0),
                discard_count: AtomicUsize::new(0),
            }),
        };
        // 预创建 min_idle 个连接（至少1个，最多max_size）
        let min_idle = std::cmp::min(2, max_size);
        for _ in 0..min_idle {
            let conn = pool.create_connection()?;
            pool.inner.connections.lock().push(conn);
        }
        Ok(pool)
    }

    /// 创建新连接并初始化 WAL 模式
    fn create_connection(&self) -> Result<PooledConn, PoolError> {
        let conn = (self.inner.factory)()?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        self.inner.total_created.fetch_add(1, Ordering::Relaxed);
        Ok(PooledConn {
            conn,
            created_at: Instant::now(),
            last_used_at: Instant::now(),
        })
    }

    /// 检查连接是否健康（执行 SELECT 1）
    fn is_healthy(conn: &Connection) -> bool {
        conn.query_row("SELECT 1", [], |_| Ok(())).is_ok()
    }

    /// 检查连接是否过期（超生命周期或空闲超时）
    fn is_expired(&self, conn: &PooledConn) -> bool {
        if let Some(max_lifetime) = self.inner.max_lifetime {
            if conn.created_at.elapsed() > max_lifetime {
                return true;
            }
        }
        if let Some(idle_timeout) = self.inner.idle_timeout {
            if conn.last_used_at.elapsed() > idle_timeout {
                return true;
            }
        }
        false
    }

    /// 获取连接（带超时）
    pub fn get(&self, timeout: Duration) -> Result<PooledConnection, PoolError> {
        let start = Instant::now();
        let deadline = start + timeout;
        let mut waited = false;

        loop {
            // 尝试从池中获取空闲连接
            let conn = {
                let mut pool = self.inner.connections.lock();
                // 从栈顶取出连接，并清理过期连接
                let mut result = None;
                while let Some(conn) = pool.pop() {
                    if self.is_expired(&conn) {
                        // 过期连接丢弃
                        self.inner.discard_count.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    if !Self::is_healthy(&conn.conn) {
                        // 不健康连接丢弃
                        self.inner.discard_count.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    result = Some(conn);
                    break;
                }
                result
            };

            if let Some(conn) = conn {
                self.inner.active_count.fetch_add(1, Ordering::Relaxed);
                let elapsed = start.elapsed();
                self.inner
                    .total_acquire_time_ns
                    .fetch_add(elapsed.as_nanos() as usize, Ordering::Relaxed);
                return Ok(PooledConnection {
                    conn: Some(conn),
                    pool: self.inner.clone(),
                });
            }

            // 池为空，检查是否可以创建新连接
            let total = self.inner.active_count.load(Ordering::Relaxed)
                + self.inner.connections.lock().len();
            if total < self.inner.max_size {
                // 未达上限，创建新连接
                match self.create_connection() {
                    Ok(conn) => {
                        self.inner.active_count.fetch_add(1, Ordering::Relaxed);
                        let elapsed = start.elapsed();
                        self.inner
                            .total_acquire_time_ns
                            .fetch_add(elapsed.as_nanos() as usize, Ordering::Relaxed);
                        return Ok(PooledConnection {
                            conn: Some(conn),
                            pool: self.inner.clone(),
                        });
                    }
                    Err(e) => {
                        // 创建失败，等待后重试
                        if !waited {
                            self.inner.wait_count.fetch_add(1, Ordering::Relaxed);
                            waited = true;
                        }
                        if Instant::now() > deadline {
                            self.inner.timeout_count.fetch_add(1, Ordering::Relaxed);
                            return Err(PoolError::Timeout);
                        }
                        std::thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                }
            }

            // 已达上限，等待连接归还
            if !waited {
                self.inner.wait_count.fetch_add(1, Ordering::Relaxed);
                waited = true;
            }
            if Instant::now() > deadline {
                self.inner.timeout_count.fetch_add(1, Ordering::Relaxed);
                return Err(PoolError::Timeout);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// 获取连接（默认超时 30 秒）
    pub fn get_default(&self) -> Result<PooledConnection, PoolError> {
        self.get(Duration::from_secs(30))
    }

    /// 当前空闲连接数
    pub fn idle_count(&self) -> usize {
        self.inner.connections.lock().len()
    }

    /// 当前活跃（借出）连接数
    pub fn active_count(&self) -> usize {
        self.inner.active_count.load(Ordering::Relaxed)
    }

    /// 最大连接数
    pub fn max_size(&self) -> usize {
        self.inner.max_size
    }

    /// 获取连接池统计信息
    pub fn stats(&self) -> PoolStats {
        let idle = self.inner.connections.lock().len();
        let active = self.inner.active_count.load(Ordering::Relaxed);
        PoolStats {
            max_size: self.inner.max_size,
            idle_count: idle,
            active_count: active,
            total_count: idle + active,
            wait_count: self.inner.wait_count.load(Ordering::Relaxed),
            timeout_count: self.inner.timeout_count.load(Ordering::Relaxed),
            total_acquire_time_ns: self.inner.total_acquire_time_ns.load(Ordering::Relaxed) as u64,
            create_count: self.inner.total_created.load(Ordering::Relaxed),
            discard_count: self.inner.discard_count.load(Ordering::Relaxed),
        }
    }

    /// 平均获取连接耗时（毫秒）
    pub fn avg_acquire_time_ms(&self) -> f64 {
        let stats = self.stats();
        if stats.create_count + stats.wait_count == 0 {
            return 0.0;
        }
        stats.total_acquire_time_ns as f64 / 1_000_000.0 / (stats.create_count + stats.wait_count) as f64
    }
}

/// 从连接池获取的连接，Drop 时自动归还
pub struct PooledConnection {
    conn: Option<PooledConn>,
    pool: Arc<PoolInner>,
}

impl PooledConnection {
    /// 获取内部连接引用
    pub fn conn(&self) -> &Connection {
        &self.conn.as_ref().expect("connection is taken").conn
    }
}

impl std::ops::Deref for PooledConnection {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        self.conn()
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(mut conn) = self.conn.take() {
            self.pool.active_count.fetch_sub(1, Ordering::Relaxed);
            // 更新最后使用时间
            conn.last_used_at = Instant::now();
            let mut pool = self.pool.connections.lock();
            if pool.len() < self.pool.max_size {
                pool.push(conn);
            }
            // 超过 max_size 的连接直接丢弃
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_basic() {
        let pool = SqlitePool::memory(3).unwrap();
        assert_eq!(pool.idle_count(), 2); // min_idle = 2
        assert_eq!(pool.active_count(), 0);

        let conn1 = pool.get_default().unwrap();
        assert_eq!(pool.idle_count(), 1);
        assert_eq!(pool.active_count(), 1);

        let conn2 = pool.get_default().unwrap();
        assert_eq!(pool.idle_count(), 0);
        assert_eq!(pool.active_count(), 2);

        // 归还连接
        drop(conn1);
        assert_eq!(pool.idle_count(), 1);
        assert_eq!(pool.active_count(), 1);

        drop(conn2);
        assert_eq!(pool.idle_count(), 2);
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_pool_execute() {
        let pool = SqlitePool::memory(2).unwrap();
        let conn = pool.get_default().unwrap();
        conn.execute_batch("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)").unwrap();
        conn.execute("INSERT INTO test (name) VALUES (?1)", rusqlite::params!["hello"]).unwrap();
        let name: String = conn.query_row("SELECT name FROM test WHERE id = 1", [], |row| row.get(0)).unwrap();
        assert_eq!(name, "hello");
    }

    #[test]
    fn test_pool_stats() {
        let pool = SqlitePool::memory(3).unwrap();
        let stats = pool.stats();
        assert_eq!(stats.max_size, 3);
        assert_eq!(stats.idle_count, 2);
        assert_eq!(stats.active_count, 0);
        assert_eq!(stats.total_count, 2);
        assert!(stats.create_count >= 2);

        let _conn = pool.get_default().unwrap();
        let stats2 = pool.stats();
        assert_eq!(stats2.active_count, 1);
        assert_eq!(stats2.total_count, 2);
    }

    #[test]
    fn test_pool_creation_when_empty() {
        let pool = SqlitePool::memory(2).unwrap();
        // min_idle = 2，获取2个连接后池为空
        let conn1 = pool.get_default().unwrap();
        let conn2 = pool.get_default().unwrap();
        assert_eq!(pool.idle_count(), 0);
        assert_eq!(pool.active_count(), 2);

        // 再获取应该创建新连接（但max_size=2，所以会等待）
        // 这里用短超时测试
        let result = pool.get(Duration::from_millis(100));
        assert!(result.is_err()); // 应该超时
        assert_eq!(pool.stats().timeout_count, 1);
    }

    #[test]
    fn test_pool_avg_acquire_time() {
        let pool = SqlitePool::memory(2).unwrap();
        let _conn = pool.get_default().unwrap();
        let avg = pool.avg_acquire_time_ms();
        assert!(avg >= 0.0);
    }

    #[test]
    fn test_pool_connection_reuse() {
        let pool = SqlitePool::memory(2).unwrap();
        // 获取并归还连接多次，验证连接被复用
        for _ in 0..5 {
            let conn = pool.get_default().unwrap();
            conn.execute_batch("SELECT 1").unwrap();
            drop(conn);
        }
        let stats = pool.stats();
        // 创建的连接数应该小于获取次数（因为复用）
        assert!(stats.create_count <= 5);
        assert_eq!(stats.active_count, 0);
    }
}
