//! 事务管理器
//!
//! 支持嵌套事务（通过 SQLite SAVEPOINT 实现），提供事务性闭包执行能力。
//! 内层事务失败时回滚到保存点，不影响外层事务。

use parking_lot::Mutex;
use rusqlite::Connection;
use std::sync::Arc;

/// 事务管理器
///
/// 包装数据库连接，提供嵌套事务支持。
/// 使用 SQLite SAVEPOINT 实现嵌套事务语义。
#[derive(Clone)]
pub struct TxManager {
    conn: Arc<Mutex<Connection>>,
    depth: Arc<Mutex<u32>>,
}

impl TxManager {
    /// 创建新的事务管理器
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self {
            conn,
            depth: Arc::new(Mutex::new(0)),
        }
    }

    /// 在事务中执行闭包
    ///
    /// - 外层调用：开启 BEGIN 事务
    /// - 内层调用：创建 SAVEPOINT
    /// - 闭包返回 Ok：提交（外层）或释放保存点（内层）
    /// - 闭包返回 Err：回滚（外层）或回滚到保存点（内层）
    pub fn run<F, T>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce() -> anyhow::Result<T>,
    {
        let mut depth = self.depth.lock();
        let is_outer = *depth == 0;
        *depth += 1;
        drop(depth);

        let savepoint_name = format!("mox_sp_{}", std::process::id());

        {
            let conn = self.conn.lock();
            if is_outer {
                conn.execute_batch("BEGIN")?;
            } else {
                conn.execute_batch(&format!("SAVEPOINT {}", savepoint_name))?;
            }
        }

        let result = f();

        let mut depth = self.depth.lock();
        *depth -= 1;
        let is_outer_commit = *depth == 0;
        drop(depth);

        match &result {
            Ok(_) => {
                let conn = self.conn.lock();
                if is_outer_commit {
                    conn.execute_batch("COMMIT")?;
                } else {
                    conn.execute_batch(&format!("RELEASE SAVEPOINT {}", savepoint_name))?;
                }
            }
            Err(_) => {
                let conn = self.conn.lock();
                if is_outer_commit {
                    let _ = conn.execute_batch("ROLLBACK");
                } else {
                    let _ = conn.execute_batch(&format!("ROLLBACK TO SAVEPOINT {}", savepoint_name));
                    let _ = conn.execute_batch(&format!("RELEASE SAVEPOINT {}", savepoint_name));
                }
            }
        }

        result
    }

    /// 获取当前事务嵌套深度
    pub fn current_depth(&self) -> u32 {
        *self.depth.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tx_commit() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let tx = TxManager::new(conn.clone());

        conn.lock().execute_batch("CREATE TABLE t(id INTEGER PRIMARY KEY, v TEXT)").unwrap();

        let result: anyhow::Result<()> = tx.run(|| {
            conn.lock().execute("INSERT INTO t(v) VALUES(?)", rusqlite::params!["a"])?;
            Ok(())
        });
        assert!(result.is_ok());

        let count: i64 = conn.lock().query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_tx_rollback() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let tx = TxManager::new(conn.clone());

        conn.lock().execute_batch("CREATE TABLE t(id INTEGER PRIMARY KEY, v TEXT)").unwrap();

        let result: anyhow::Result<()> = tx.run(|| {
            conn.lock().execute("INSERT INTO t(v) VALUES(?)", rusqlite::params!["a"])?;
            anyhow::bail!("force rollback")
        });
        assert!(result.is_err());

        let count: i64 = conn.lock().query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_nested_tx_inner_rollback() {
        let conn = Arc::new(Mutex::new(Connection::open_in_memory().unwrap()));
        let tx = TxManager::new(conn.clone());

        conn.lock().execute_batch("CREATE TABLE t(id INTEGER PRIMARY KEY, v TEXT)").unwrap();

        let result: anyhow::Result<()> = tx.run(|| {
            conn.lock().execute("INSERT INTO t(v) VALUES(?)", rusqlite::params!["outer"])?;

            let inner: anyhow::Result<()> = tx.run(|| {
                conn.lock().execute("INSERT INTO t(v) VALUES(?)", rusqlite::params!["inner"])?;
                anyhow::bail!("inner rollback")
            });
            assert!(inner.is_err());

            Ok(())
        });
        assert!(result.is_ok());

        // 外层提交，内层回滚 → 只有 outer 记录
        let count: i64 = conn.lock().query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let v: String = conn.lock().query_row("SELECT v FROM t LIMIT 1", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "outer");
    }
}
