//! Storage administration adapter. Contracts belong to mox-cloud-api; backend implementations stay here.
use mox_cloud_api::admin::{GcReport, MigrateReport, StorageAdmin, StorageStats, StorageStatus, VerifyReport};
use mox_cloud_api::{CloudApiError, CloudApiResult};
use async_trait::async_trait;
use mox_base_store_core::StoreError;
use mox_cloud_store_core::{
    collect_store_stats, create_backend, list_object_refs, BackendKind, GarbageCollector,
    KeyPathCodec, S3ClientConfig, StoreBackend, StoreConfig,
};
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// 可热切换的后端持有者
struct BackendCell {
    backend: Mutex<StoreBackend>,
    last_gc_ms: Mutex<u64>,
    error_count: Mutex<u64>,
    last_error: Mutex<Option<String>>,
    /// GC 宽限期（秒），默认 30 天
    grace_secs: u64,
}

/// 管理实现：持有当前后端 + 运维指标。
#[derive(Clone)]
pub struct StoreAdmin {
    cell: Arc<BackendCell>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn store_err(e: StoreError) -> CloudApiError {
    CloudApiError::Storage(e.to_string())
}

fn backend_kind_name(k: &BackendKind) -> &'static str {
    match k {
        BackendKind::Fs => "fs",
        BackendKind::S3 => "s3",
        BackendKind::Minio => "minio",
        BackendKind::Oss => "oss",
    }
}

fn health_for(err_count: u64) -> &'static str {
    if err_count > 0 {
        "degraded"
    } else {
        "healthy"
    }
}

/// 从环境变量装配 S3 配置（MOX_S3_ENDPOINT / MOX_S3_REGION / MOX_S3_ACCESS_KEY /
/// MOX_S3_SECRET_KEY / MOX_S3_BUCKET）。
fn s3_config_from_env() -> Option<S3ClientConfig> {
    let endpoint = std::env::var("MOX_S3_ENDPOINT").ok()?;
    Some(S3ClientConfig {
        endpoint,
        region: std::env::var("MOX_S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
        access_key: std::env::var("MOX_S3_ACCESS_KEY").unwrap_or_default(),
        secret_key: std::env::var("MOX_S3_SECRET_KEY").unwrap_or_default(),
        bucket: std::env::var("MOX_S3_BUCKET").unwrap_or_else(|_| "kb".into()),
        force_path_style: true,
    })
}

/// 依据环境装配后端：`FILE_BACKEND`（fs|s3|minio|oss）+ `MOX_STORE_DATA_DIR`。
pub fn assemble_backend() -> CloudApiResult<StoreBackend> {
    let kind = std::env::var("FILE_BACKEND").unwrap_or_else(|_| "fs".into());
    let data_dir = std::env::var("MOX_STORE_DATA_DIR").unwrap_or_else(|_| "./data/store".into());
    let cfg = StoreConfig {
        kind: BackendKind::from_str_ci(&kind).map_err(store_err)?,
        data_dir: PathBuf::from(data_dir),
        s3: s3_config_from_env(),
        ..Default::default()
    };
    create_backend(&cfg).map_err(store_err)
}

impl StoreAdmin {
    /// 用现有后端构造管理实例（默认 GC 宽限期 30 天；测试 / 运行时注入用）
    pub fn new(backend: StoreBackend) -> Self {
        Self::with_grace(backend, 30 * 24 * 3600)
    }

    /// 指定 GC 宽限期（秒）构造，供测试 / 运维快速回收
    pub fn with_grace(backend: StoreBackend, grace_secs: u64) -> Self {
        Self {
            cell: Arc::new(BackendCell {
                backend: Mutex::new(backend),
                last_gc_ms: Mutex::new(0),
                error_count: Mutex::new(0),
                last_error: Mutex::new(None),
                grace_secs,
            }),
        }
    }

    /// 依据环境变量装配后端并构造管理实例
    pub fn assemble() -> CloudApiResult<Self> {
        Ok(Self::new(assemble_backend()?))
    }

    /// 记录一次错误（供 status 健康度）
    fn record_error(&self, msg: &str) {
        let mut n = self.cell.error_count.lock();
        *n += 1;
        let mut le = self.cell.last_error.lock();
        *le = Some(msg.to_string());
    }

    /// 构建指定 kind 的后端（沿用当前 data_dir + 环境 S3 配置）
    fn build_kind(&self, kind_str: &str, data_dir: &PathBuf) -> CloudApiResult<StoreBackend> {
        let kind = BackendKind::from_str_ci(kind_str).map_err(store_err)?;
        let cfg = StoreConfig {
            kind,
            data_dir: data_dir.clone(),
            s3: s3_config_from_env(),
            ..Default::default()
        };
        create_backend(&cfg).map_err(store_err)
    }

    /// 解析后端描述符 `kind[@data_dir]`；缺省 data_dir 用当前后端目录
    fn parse_descriptor(&self, desc: &str) -> CloudApiResult<(String, PathBuf)> {
        if let Some((kind, dir)) = desc.split_once('@') {
            Ok((kind.trim().to_string(), PathBuf::from(dir.trim())))
        } else {
            let dir = self.cell.backend.lock().data_dir.clone();
            Ok((desc.trim().to_string(), dir))
        }
    }
}

#[async_trait]
impl StorageAdmin for StoreAdmin {
    async fn status(&self) -> CloudApiResult<StorageStatus> {
        let (backend, err_count, last_error, last_gc) = {
            let b = self.cell.backend.lock();
            (
                b.clone(),
                *self.cell.error_count.lock(),
                self.cell.last_error.lock().clone(),
                *self.cell.last_gc_ms.lock(),
            )
        };
        let st = match collect_store_stats(&backend.data_dir).await {
            Ok(s) => s,
            Err(e) => {
                self.record_error(&e.to_string());
                return Err(store_err(e));
            }
        };
        Ok(StorageStatus {
            backend: backend_kind_name(&backend.kind).to_string(),
            health: health_for(err_count).to_string(),
            data_dir: backend.data_dir.display().to_string(),
            object_count: st.object_count,
            chunk_count: st.chunk_count,
            version_count: st.version_count,
            kv_count: st.kv_count,
            data_bytes: st.chunks_bytes,
            logical_bytes: st.logical_bytes,
            dedup_ratio: st.dedup_ratio(),
            last_gc_ms: last_gc,
            error_count: err_count,
            last_error,
        })
    }

    async fn stats(&self) -> CloudApiResult<StorageStats> {
        let data_dir = self.cell.backend.lock().data_dir.clone();
        let st = collect_store_stats(&data_dir).await.map_err(store_err)?;
        Ok(StorageStats {
            object_count: st.object_count,
            chunk_count: st.chunk_count,
            version_count: st.version_count,
            kv_count: st.kv_count,
            physical_bytes: st.chunks_bytes,
            logical_bytes: st.logical_bytes,
            dedup_ratio: st.dedup_ratio(),
            ref_total: st.ref_total,
        })
    }

    async fn verify(&self) -> CloudApiResult<VerifyReport> {
        let data_dir = self.cell.backend.lock().data_dir.clone();
        let t0 = now_ms();
        let mut report = VerifyReport::default();
        let refs = match list_object_refs(&data_dir).await {
            Ok(r) => r,
            Err(e) => return Err(store_err(e)),
        };
        report.objects_checked = refs.len() as u64;
        for (path, sha) in refs {
            let cp = KeyPathCodec::chunk_path(&data_dir, &sha);
            match tokio::fs::metadata(&cp).await {
                Ok(m) if m.len() > 0 => report.objects_ok += 1,
                Ok(_) => {
                    report.missing += 1;
                    report.errors.push(format!("{path}: chunk 空文件 {sha}"));
                }
                Err(_) => {
                    report.missing += 1;
                    report.errors.push(format!("{path}: chunk 缺失 {sha}"));
                }
            }
        }
        report.duration_ms = now_ms().saturating_sub(t0);
        Ok(report)
    }

    async fn gc(&self, dry_run: bool) -> CloudApiResult<GcReport> {
        let (data_dir, grace_secs) = {
            let b = self.cell.backend.lock();
            (b.data_dir.clone(), self.cell.grace_secs)
        };
        let t0 = now_ms();
        let gc = GarbageCollector::with_grace(data_dir, grace_secs);
        let r = gc.collect(dry_run).await.map_err(store_err)?;
        if !dry_run {
            *self.cell.last_gc_ms.lock() = now_ms();
        }
        Ok(GcReport {
            dry_run,
            chunks_scanned: r.chunks_scanned,
            soft_purged: r.soft_purged,
            hard_deleted: r.hard_deleted,
            bytes_freed: r.bytes_freed,
            duration_ms: now_ms().saturating_sub(t0),
            warnings: r.warnings,
        })
    }

    async fn switch_backend(&self, target: &str) -> CloudApiResult<StorageStatus> {
        let dir = self.cell.backend.lock().data_dir.clone();
        let new_backend = self.build_kind(target, &dir)?;
        *self.cell.backend.lock() = new_backend;
        self.status().await
    }

    async fn migrate(&self, source: &str, target: &str) -> CloudApiResult<MigrateReport> {
        let (src_kind, src_dir) = self.parse_descriptor(source)?;
        let (dst_kind, dst_dir) = self.parse_descriptor(target)?;
        let src = self.build_kind(&src_kind, &src_dir)?;
        let dst = self.build_kind(&dst_kind, &dst_dir)?;
        let t0 = now_ms();
        let mut report = MigrateReport {
            source: format!("{src_kind}@{}", src_dir.display()),
            target: format!("{dst_kind}@{}", dst_dir.display()),
            ..Default::default()
        };
        let refs = match list_object_refs(&src.data_dir).await {
            Ok(r) => r,
            Err(e) => return Err(store_err(e)),
        };
        report.objects_total = refs.len() as u64;
        for (path, _sha) in refs {
            match src.object.get(&path).await {
                Ok(data) => {
                    match dst
                        .object
                        .put(&path, "application/octet-stream", data.clone())
                        .await
                    {
                        Ok(_) => {
                            report.objects_ok += 1;
                            report.bytes_migrated += data.len() as u64;
                        }
                        Err(e) => {
                            report.objects_failed += 1;
                            report.errors.push(format!("{path}: {e}"));
                        }
                    }
                }
                Err(e) => {
                    report.objects_failed += 1;
                    report.errors.push(format!("{path}: {e}"));
                }
            }
        }
        report.duration_ms = now_ms().saturating_sub(t0);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    fn fs_admin(dir: &std::path::Path) -> StoreAdmin {
        fs_admin_grace(dir, 0)
    }

    fn fs_admin_grace(dir: &std::path::Path, grace_secs: u64) -> StoreAdmin {
        let cfg = StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dir.to_path_buf(),
            ..Default::default()
        };
        StoreAdmin::with_grace(create_backend(&cfg).unwrap(), grace_secs)
    }

    #[tokio::test]
    async fn status_and_stats_report_dedup() {
        let dir = tempfile::tempdir().unwrap();
        let admin = fs_admin(dir.path());
        let be = admin.cell.backend.lock().clone();
        be.object
            .put("a.txt", "text/plain", Bytes::from_static(b"shared"))
            .await
            .unwrap();
        be.object
            .put("b.txt", "text/plain", Bytes::from_static(b"shared"))
            .await
            .unwrap();

        let st = admin.status().await.unwrap();
        assert_eq!(st.backend, "fs");
        assert_eq!(st.health, "healthy");
        assert_eq!(st.object_count, 2);
        assert_eq!(st.chunk_count, 1, "去重后唯一块应为 1");
        assert!(st.dedup_ratio >= 2.0);

        let stats = admin.stats().await.unwrap();
        assert_eq!(stats.object_count, 2);
        assert_eq!(stats.chunk_count, 1);
        assert_eq!(stats.ref_total, 2);
    }

    #[tokio::test]
    async fn verify_and_gc_workflow() {
        let dir = tempfile::tempdir().unwrap();
        let admin = fs_admin(dir.path());
        let be = admin.cell.backend.lock().clone();
        be.object
            .put("keep.txt", "text/plain", Bytes::from_static(b"keep me"))
            .await
            .unwrap();
        be.object
            .put("gone.txt", "text/plain", Bytes::from_static(b"delete me"))
            .await
            .unwrap();
        be.object.delete("gone.txt").await.unwrap();

        let v = admin.verify().await.unwrap();
        assert_eq!(v.objects_checked, 1);
        assert_eq!(v.objects_ok, 1);

        // dry-run 预览
        let g = admin.gc(true).await.unwrap();
        assert!(g.dry_run);
        assert!(g.hard_deleted >= 1);

        // 实跑
        let g = admin.gc(false).await.unwrap();
        assert_eq!(g.hard_deleted, 1);
        let st = admin.status().await.unwrap();
        assert!(st.last_gc_ms > 0);
    }

    #[tokio::test]
    async fn switch_backend_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let admin = fs_admin(dir.path());
        let st = admin.switch_backend("fs").await.unwrap();
        assert_eq!(st.backend, "fs");
        assert_eq!(st.health, "healthy");
    }

    #[tokio::test]
    async fn migrate_copies_objects_between_dirs() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let admin = fs_admin(src.path());
        let be = admin.cell.backend.lock().clone();
        be.object
            .put("doc/a.md", "text/markdown", Bytes::from_static("# 迁移".as_bytes()))
            .await
            .unwrap();

        let src_desc = format!("fs@{}", src.path().display());
        let dst_desc = format!("fs@{}", dst.path().display());
        let r = admin.migrate(&src_desc, &dst_desc).await.unwrap();
        assert_eq!(r.objects_total, 1);
        assert_eq!(r.objects_ok, 1);
        assert_eq!(r.objects_failed, 0);

        // 目标可读到完整内容
        let dst_be = create_backend(&StoreConfig {
            kind: BackendKind::Fs,
            data_dir: dst.path().to_path_buf(),
            ..Default::default()
        })
        .unwrap();
        let got = dst_be.object.get("doc/a.md").await.unwrap();
        assert_eq!(&got[..], "# 迁移".as_bytes());
    }
}
