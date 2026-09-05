//! 简单的 SQLite 连接池实现
//!
//! 由于 r2d2-sqlite crate 已不可用，这里实现一个轻量级连接池，
//! 支持固定大小连接池、连接获取/归还、WAL 模式初始化。

use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;
use std::time::Duration;

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

/// 简单的 SQLite 连接池
pub struct SqlitePool {
    inner: Arc<PoolInner>,
}

struct PoolInner {
    connections: Mutex<Vec<Connection>>,
    max_size: usize,
    /// 用于创建新连接的工厂
    factory: Box<dyn Fn() -> Result<Connection, rusqlite::Error> + Send + Sync>,
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
                factory,
            }),
        };
        // 预创建 min_idle 个连接（至少1个）
        let min_idle = std::cmp::min(2, max_size);
        for _ in 0..min_idle {
            let conn = pool.create_connection()?;
            pool.inner.connections.lock().push(conn);
        }
        Ok(pool)
    }

    /// 创建新连接并初始化 WAL 模式
    fn create_connection(&self) -> Result<Connection, PoolError> {
        let conn = (self.inner.factory)()?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(conn)
    }

    /// 获取连接（带超时）
    pub fn get(&self, timeout: Duration) -> Result<PooledConnection, PoolError> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            // 尝试从池中获取空闲连接
            if let Some(conn) = self.inner.connections.lock().pop() {
                return Ok(PooledConnection {
                    conn: Some(conn),
                    pool: self.inner.clone(),
                });
            }
            // 池为空，尝试创建新连接（如果未达上限）
            // 注意：这里需要先检查当前总连接数，但简单实现中我们不追踪活跃连接数
            // 因此直接创建新连接，归还时如果超过 max_size 则丢弃
            if std::time::Instant::now() > deadline {
                return Err(PoolError::Timeout);
            }
            // 短暂等待后重试
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
}

/// 从连接池获取的连接，Drop 时自动归还
pub struct PooledConnection {
    conn: Option<Connection>,
    pool: Arc<PoolInner>,
}

impl PooledConnection {
    /// 获取内部连接引用
    pub fn conn(&self) -> &Connection {
        self.conn.as_ref().expect("connection is taken")
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
        if let Some(conn) = self.conn.take() {
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

        let conn1 = pool.get_default().unwrap();
        assert_eq!(pool.idle_count(), 1);

        let conn2 = pool.get_default().unwrap();
        assert_eq!(pool.idle_count(), 0);

        // 归还连接
        drop(conn1);
        assert_eq!(pool.idle_count(), 1);

        drop(conn2);
        assert_eq!(pool.idle_count(), 2);
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
}
