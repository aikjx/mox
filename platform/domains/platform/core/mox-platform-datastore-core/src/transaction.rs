// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use parking_lot::Mutex;

pub struct TxManager {
    pub conn: std::sync::Arc<Mutex<rusqlite::Connection>>,
    depth: Mutex<u32>,
}

impl TxManager {
    pub fn new(conn: std::sync::Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self {
            conn,
            depth: Mutex::new(0),
        }
    }

    pub fn run<R>(&self, f: impl FnOnce() -> anyhow::Result<R>) -> anyhow::Result<R> {
        let mut depth = self.depth.lock();
        let is_top = *depth == 0;
        let sp_name = format!("sp_{}", depth);
        *depth += 1;
        drop(depth);

        let conn = self.conn.lock();

        if is_top {
            conn.execute_batch("BEGIN IMMEDIATE")
                .map_err(|e| anyhow::anyhow!("BEGIN failed: {}", e))?;
        } else {
            conn.execute_batch(&format!("SAVEPOINT {}", sp_name))
                .map_err(|e| anyhow::anyhow!("SAVEPOINT {} failed: {}", sp_name, e))?;
        }
        drop(conn);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        let mut depth = self.depth.lock();
        *depth -= 1;
        let is_top_after = *depth == 0;
        drop(depth);

        let conn = self.conn.lock();

        match result {
            Ok(Ok(r)) => {
                if is_top_after {
                    conn.execute_batch("COMMIT")
                        .map_err(|e| anyhow::anyhow!("COMMIT failed: {}", e))?;
                } else {
                    conn.execute_batch(&format!("RELEASE SAVEPOINT {}", sp_name))
                        .map_err(|e| anyhow::anyhow!("RELEASE failed: {}", e))?;
                }
                Ok(r)
            }
            Ok(Err(e)) => {
                if is_top_after {
                    let _ = conn.execute_batch("ROLLBACK");
                } else {
                    let _ = conn.execute_batch(&format!("ROLLBACK TO SAVEPOINT {}", sp_name));
                    let _ = conn.execute_batch(&format!("RELEASE SAVEPOINT {}", sp_name));
                }
                Err(e)
            }
            Err(panic) => {
                if is_top_after {
                    let _ = conn.execute_batch("ROLLBACK");
                } else {
                    let _ = conn.execute_batch(&format!("ROLLBACK TO SAVEPOINT {}", sp_name));
                    let _ = conn.execute_batch(&format!("RELEASE SAVEPOINT {}", sp_name));
                }
                std::panic::resume_unwind(panic)
            }
        }
    }
}
