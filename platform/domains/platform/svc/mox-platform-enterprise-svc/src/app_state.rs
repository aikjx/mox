//! 应用共享状态：持有 4 个核心仓储 + rusqlite 连接
//!
//! 构造流程：打开 SQLite → 各 repo init_schema + 种子数据 → 返回 Arc<AppState>。

use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::Mutex;

use mox_platform_datastore_core::UniversalBizDAO;
use mox_platform_iam_core::IamRepository;
use mox_platform_meta_core::MetaRepository;
use mox_platform_orchestrator_core::Orchestrator;

pub struct AppState {
    pub iam: Arc<IamRepository>,
    pub meta: Arc<MetaRepository>,
    pub dao: Arc<UniversalBizDAO>,
    pub orch: Arc<Orchestrator>,
    pub db_conn: Arc<Mutex<rusqlite::Connection>>,
}

impl AppState {
    pub async fn open_memory_or_file(path: &str, install_industries: &[&str]) -> Result<Self> {
        let path = path.to_string();
        let industries: Vec<String> = install_industries.iter().map(|s| s.to_string()).collect();

        let db_conn = tokio::task::spawn_blocking(move || -> Result<Arc<Mutex<rusqlite::Connection>>> {
            let conn = if path == ":memory:" {
                rusqlite::Connection::open_in_memory()
            } else {
                rusqlite::Connection::open(&path)
            }
            .with_context(|| format!("open sqlite {} failed", path))?;
            conn.pragma_update(None, "journal_mode", "WAL").ok();
            conn.pragma_update(None, "foreign_keys", "ON").ok();
            Ok(Arc::new(Mutex::new(conn)))
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {}", e))??;

        let iam = Arc::new(IamRepository::new(db_conn.clone()));
        let meta = Arc::new(MetaRepository::new(db_conn.clone()));
        let dao = Arc::new(UniversalBizDAO::new(db_conn.clone()));

        let industries_ref: Vec<&str> = industries.iter().map(|s| s.as_str()).collect();
        let iam_cloned = iam.clone();
        let meta_cloned = meta.clone();
        let dao_cloned = dao.clone();

        tokio::task::spawn_blocking(move || -> Result<()> {
            iam_cloned.init_schema().context("iam init_schema")?;
            iam_cloned.seed().context("iam seed")?;
            meta_cloned
                .init_schema()
                .context("meta init_schema")?;
            meta_cloned
                .seed_industry(&industries_ref)
                .context("meta seed_industry")?;
            dao_cloned.init_schema().context("dao init_schema")?;
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking join error: {}", e))??;

        let orch = Arc::new(Orchestrator::new(meta.clone(), dao.clone()));
        orch.register_pipeline("default");

        Ok(Self {
            iam,
            meta,
            dao,
            orch,
            db_conn,
        })
    }
}
