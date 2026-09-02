// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 内容寻址去重 · 引用计数索引（`refs/<xx>/<sha256>.json`）。
//!
//! 引用计数维护增量计数（put +1 / delete -1）；GC 以对象元数据扫描为准重建
//! 索引（`rebuild_from_objects`），修正崩溃/异常产生的漂移，保证企业级一致性。

use crate::{hash_prefix, key_file_name};
use mox_base_store_core::{StoreError, StoreResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 引用计数条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RefEntry {
    /// 当前引用数
    pub refcount: u64,
    /// 首次创建时间（epoch ms）
    pub created_ms: u64,
}

/// 引用计数管理器
#[derive(Clone)]
pub struct ChunkRefManager {
    data_dir: PathBuf,
}

/// 索引重建报告（用于 GC 前的漂移修正与审计）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RebuildReport {
    pub scanned_chunks: u64,
    pub rebuilt_entries: u64,
    pub removed_stale_entries: u64,
}

impl ChunkRefManager {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    fn ref_path(&self, sha: &str) -> PathBuf {
        self.data_dir
            .join("refs")
            .join(hash_prefix(sha))
            .join(format!("{sha}.json"))
    }

    async fn read_entry(&self, sha: &str) -> StoreResult<Option<RefEntry>> {
        let p = self.ref_path(sha);
        let raw = match tokio::fs::read(&p).await {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };
        serde_json::from_slice(&raw)
            .map(Some)
            .map_err(|e| StoreError::Checksum(format!("ref 解析失败 {}: {e}", p.display())))
    }

    async fn write_entry(&self, sha: &str, entry: &RefEntry) -> StoreResult<()> {
        let p = self.ref_path(sha);
        crate::fs_backend::atomic_write(&p, &serde_json::to_vec(entry).map_err(|e| StoreError::Other(e.to_string()))?).await
    }

    /// 引用 +1（不存在则创建）
    pub async fn add_ref(&self, sha: &str) -> StoreResult<()> {
        let mut e = self.read_entry(sha).await?.unwrap_or_default();
        if e.refcount == 0 {
            e.created_ms = now_ms();
        }
        e.refcount += 1;
        self.write_entry(sha, &e).await
    }

    /// 引用 -1（为 0 时保留条目，由 GC 负责物理删除）
    pub async fn remove_ref(&self, sha: &str) -> StoreResult<()> {
        let mut e = self.read_entry(sha).await?.unwrap_or_default();
        e.refcount = e.refcount.saturating_sub(1);
        self.write_entry(sha, &e).await
    }

    /// 当前引用数
    pub async fn refcount(&self, sha: &str) -> StoreResult<u64> {
        Ok(self.read_entry(sha).await?.map(|e| e.refcount).unwrap_or(0))
    }

    /// 以对象元数据扫描为准重建索引：修正漂移，返回报告。
    ///
    /// `referenced` 为当前对象元数据实际引用的 chunk 集合；
    /// - 为引用集内但索引缺失/为 0 的 chunk → 重建为 1
    /// - 为引用集外但索引 > 0 的 chunk → 清零（记录，供 GC 回收）
    pub async fn rebuild_from_objects(
        &self,
        referenced: &HashSet<String>,
        scanned_chunks: u64,
    ) -> StoreResult<RebuildReport> {
        let mut report = RebuildReport {
            scanned_chunks,
            ..Default::default()
        };
        for sha in referenced {
            let e = self.read_entry(sha).await?;
            let need_fix = e.as_ref().map(|e| e.refcount == 0).unwrap_or(true);
            if need_fix {
                self.write_entry(sha, &RefEntry { refcount: 1, created_ms: now_ms() })
                    .await?;
                report.rebuilt_entries += 1;
            }
        }
        // 扫描既有 ref 文件，将不在引用集的清零
        let refs_root = self.data_dir.join("refs");
        let mut stack = vec![refs_root.clone()];
        while let Some(dir) = stack.pop() {
            let mut rd = match tokio::fs::read_dir(&dir).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            while let Ok(Some(ent)) = rd.next_entry().await {
                if entry_is_dir(&ent).await {
                    stack.push(ent.path());
                } else if ent.file_name().to_string_lossy().ends_with(".json") {
                    let sha = ent.file_name().to_string_lossy().trim_end_matches(".json").to_string();
                    if !referenced.contains(&sha) {
                        if let Some(e) = self.read_entry(&sha).await? {
                            if e.refcount > 0 {
                                self.write_entry(&sha, &RefEntry { refcount: 0, created_ms: e.created_ms }).await?;
                                report.removed_stale_entries += 1;
                            }
                        }
                    }
                }
            }
        }
        Ok(report)
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 目录项是否为目录：`file_type` 失败时回退 `metadata`（再失败按文件处理）
pub(crate) async fn entry_is_dir(ent: &tokio::fs::DirEntry) -> bool {
    if let Ok(ft) = ent.file_type().await {
        return ft.is_dir();
    }
    std::fs::metadata(ent.path()).map(|m| m.is_dir()).unwrap_or(false)
}

/// 列出 data_dir 下全部 chunk 哈希（供 GC 扫描）
pub async fn list_chunks(data_dir: &Path) -> StoreResult<Vec<String>> {
    let mut out = Vec::new();
    let mut stack = vec![data_dir.join("chunks")];
    while let Some(dir) = stack.pop() {
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        while let Ok(Some(ent)) = rd.next_entry().await {
            if entry_is_dir(&ent).await {
                stack.push(ent.path());
            } else {
                // 跳过 tmp 文件
                if !ent.file_name().to_string_lossy().starts_with(".tmp-") {
                    out.push(ent.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }
    Ok(out)
}

/// 列出 data_dir 下全部对象元数据（逻辑 path, sha256）
pub async fn list_object_refs(data_dir: &Path) -> StoreResult<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut stack = vec![data_dir.join("objects")];
    while let Some(dir) = stack.pop() {
        let mut rd = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        while let Ok(Some(ent)) = rd.next_entry().await {
            if entry_is_dir(&ent).await {
                stack.push(ent.path());
            } else if ent.file_name().to_string_lossy().ends_with(".obj") {
                let raw = match tokio::fs::read(ent.path()).await {
                    Ok(r) => r,
                    Err(_) => continue,
                };
                if let Ok(meta) = serde_json::from_slice::<crate::fs_backend::ObjectMeta>(&raw) {
                    out.push((meta.path, meta.sha256));
                }
            }
        }
    }
    Ok(out)
}

/// 由逻辑 key 计算对象元数据文件名（复用防穿越编码）
pub fn object_file_for(key: &str) -> String {
    key_file_name(key)
}
