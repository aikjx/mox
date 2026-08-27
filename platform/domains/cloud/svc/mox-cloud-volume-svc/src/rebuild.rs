// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! EC rebuild job – given a manifest, list of missing shard ids and a root
//! directory, reconstructs the missing shards, writes them back, updates the
//! manifest crc64 and bumps the `REBUILD_COUNT` metric.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use crate::error::{VolumeError, VolumeResult};
use crate::fs_layout::{manifest_path, shard_path};
use crate::manifest::{crc64_ecma, EcManifest};
use crate::metrics::REBUILD_COUNT;
use crate::profile::EcProfile;
use crate::reed_solomon::{ReedSolomonEngine, RSResult, RSError};

/// High-level job descriptor used by the rebuild orchestrator.
#[derive(Debug, Clone)]
pub struct RebuildJob {
    pub mountpath: PathBuf,
    pub bucket_prefix: String,
    pub oid: String,
    /// Missing shard indices we want to materialise back on disk.
    pub missing_shard_ids: Vec<usize>,
}

impl RebuildJob {
    pub fn new<P: AsRef<Path>>(
        mountpath: P,
        bucket_prefix: impl Into<String>,
        oid: impl Into<String>,
        missing_shard_ids: Vec<usize>,
    ) -> Self {
        Self {
            mountpath: mountpath.as_ref().to_path_buf(),
            bucket_prefix: bucket_prefix.into(),
            oid: oid.into(),
            missing_shard_ids,
        }
    }

    /// Load the manifest for this object.
    pub fn load_manifest(&self) -> VolumeResult<EcManifest> {
        let path = manifest_path(&self.mountpath, &self.bucket_prefix, &self.oid);
        let raw = fs::read(&path).map_err(|e| VolumeError::IOError(format!("read manifest: {e}")))?;
        serde_json::from_slice(&raw).map_err(|e| VolumeError::Internal(format!("decode manifest: {e}")))
    }

    /// Run the rebuild.  Returns the number of shard files written.
    /// Metric `mox_ec_rebuild_count` is incremented by one on success.
    pub fn run(&self) -> VolumeResult<usize> {
        let manifest = self.load_manifest()?;
        let profile = EcProfile::new(
            manifest.data_shards,
            manifest.parity_shards,
            crate::profile::DEFAULT_MIN_OBJ_SIZE,
        )
        .map_err(|e| VolumeError::Internal(format!("bad manifest profile: {e}")))?;
        let total = profile.total_shards();
        // Read all shards we can find.
        let mut slots: Vec<Option<Vec<u8>>> = vec![None; total];
        for i in 0..total {
            let p = shard_path(&self.mountpath, &self.bucket_prefix, &self.oid, i);
            if p.exists() {
                let data =
                    fs::read(&p).map_err(|e| VolumeError::IOError(format!("read shard {i}: {e}")))?;
                slots[i] = Some(data);
            }
        }
        let engine = ReedSolomonEngine::new();
        let rebuilt = engine
            .reconstruct_shards(&profile, &slots)
            .map_err(|e| VolumeError::RebuildFailed(format!("reconstruct: {e}")))?;
        // Validate against manifest.crc64 by reassembling user bytes then
        // truncating to `manifest.original_size` (the byte count recorded at
        // encode-time, before EC zero-padding).
        let _shard_size = rebuilt
            .first()
            .map(|v| v.len())
            .ok_or_else(|| VolumeError::Internal("empty shard set".into()))?;
        let padded_bytes: Vec<u8> = rebuilt
            .iter()
            .take(profile.data_shards as usize)
            .flat_map(|s| s.clone())
            .collect();
        let original_size = manifest.original_size as usize;
        let trimmed: Vec<u8> = if original_size == 0 {
            // Legacy manifests may default original_size to 0.  Fall back to
            // stripping trailing zero bytes one by one until the CRC matches
            // (up to the padded size).
            let mut found = None;
            if crc64_ecma(&padded_bytes) == manifest.crc64 {
                padded_bytes
            } else {
                let mut cur = padded_bytes.clone();
                while !cur.is_empty() && cur[cur.len() - 1] == 0 {
                    cur.pop();
                    if crc64_ecma(&cur) == manifest.crc64 {
                        found = Some(cur.clone());
                        break;
                    }
                }
                found.ok_or_else(|| {
                    VolumeError::CrcMismatch(format!(
                        "rebuilt crc64={:#018x} != manifest crc64={:#018x}",
                        crc64_ecma(&padded_bytes),
                        manifest.crc64
                    ))
                })?
            }
        } else {
            if original_size > padded_bytes.len() {
                return Err(VolumeError::CrcMismatch(format!(
                    "original_size {original_size} > padded bytes {}",
                    padded_bytes.len()
                )));
            }
            let trimmed = &padded_bytes[..original_size];
            let got = crc64_ecma(trimmed);
            if got != manifest.crc64 {
                return Err(VolumeError::CrcMismatch(format!(
                    "rebuilt crc64={got:#018x} != manifest crc64={:#018x}",
                    manifest.crc64
                )));
            }
            trimmed.to_vec()
        };
        // The validated bytes are no longer used directly – drop the binding.
        let _ = trimmed;

        // Write the missing shards back.
        let mut written = 0usize;
        for &idx in &self.missing_shard_ids {
            if idx >= rebuilt.len() {
                return Err(VolumeError::RebuildFailed(format!(
                    "missing shard id {idx} out of range 0..{}",
                    rebuilt.len()
                )));
            }
            let p = shard_path(&self.mountpath, &self.bucket_prefix, &self.oid, idx);
            if let Some(parent) = p.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| VolumeError::IOError(format!("mkdir {parent:?}: {e}")))?;
            }
            fs::write(&p, &rebuilt[idx])
                .map_err(|e| VolumeError::IOError(format!("write shard {idx}: {e}")))?;
            written += 1;
        }
        // Recompute aggregate crc64 after write and update manifest.
        let mut new_manifest = manifest.clone();
        let mut agg = 0u64;
        for shard in &rebuilt {
            agg = crate::manifest::crc64_ecma_update(agg, shard);
        }
        new_manifest.crc64 = agg;
        new_manifest.shard_count = rebuilt.len() as u16;
        let manifest_bytes = serde_json::to_vec_pretty(&new_manifest)
            .map_err(|e| VolumeError::Internal(format!("encode manifest: {e}")))?;
        fs::write(manifest_path(&self.mountpath, &self.bucket_prefix, &self.oid), manifest_bytes)
            .map_err(|e| VolumeError::IOError(format!("write manifest: {e}")))?;
        REBUILD_COUNT.fetch_add(1, Ordering::SeqCst);
        Ok(written)
    }
}

/// Convenience helper that encodes an object and writes its shards + manifest
/// into the EC layout.  Returns the manifest that was written.
pub fn encode_and_write(
    mountpath: &Path,
    bucket_prefix: &str,
    oid: &str,
    profile: &EcProfile,
    tier: crate::manifest::StorageTier,
    user_bytes: &[u8],
) -> RSResult<EcManifest> {
    let engine = ReedSolomonEngine::new();
    let shards = if profile.is_replica(user_bytes.len() as u64) {
        // replica: skip EC, just make 3 identical copies labelled as shards
        // 0..3 so `EcProfile.data_shards=2, parity=1` still fits; but since
        // tier/replica flag is up to the caller we simply reuse data_shards+1
        // copies of the original bytes (or total_shards = data + parity for
        // consistency).
        let total = profile.total_shards();
        vec![user_bytes.to_vec(); total]
    } else {
        engine.encode(profile, user_bytes)?
    };

    // Build dir + write shards.
    let dir = crate::fs_layout::ec_object_dir(mountpath, bucket_prefix, oid);
    fs::create_dir_all(&dir).map_err(|e| RSError::InvalidInput(format!("mkdir: {e}")))?;
    for (i, s) in shards.iter().enumerate() {
        let p = shard_path(mountpath, bucket_prefix, oid, i);
        fs::write(p, s).map_err(|e| RSError::InvalidInput(format!("write shard: {e}")))?;
    }

    // Aggregate crc64 over the shards in order.
    let user_crc = crc64_ecma(user_bytes);
    let manifest = EcManifest {
        oid: oid.to_string(),
        bid: bucket_prefix.to_string(),
        crc64: user_crc,
        shard_count: shards.len() as u16,
        data_shards: profile.data_shards,
        parity_shards: profile.parity_shards,
        created_at_ms: now_ms(),
        tier,
        original_size: user_bytes.len() as u64,
    };
    let manifest_path = manifest_path(mountpath, bucket_prefix, oid);
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| RSError::InvalidInput(format!("encode manifest: {e}")))?;
    fs::write(manifest_path, bytes)
        .map_err(|e| RSError::InvalidInput(format!("write manifest: {e}")))?;
    Ok(manifest)
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_write_and_rebuild_small() {
        let tmp = tempfile::tempdir().unwrap();
        let mount = tmp.path();
        // Force EC path (not replica) by using min_obj_size = 0 or a payload
        // larger than DEFAULT_MIN_OBJ_SIZE; we use 128 KiB here.
        let profile = EcProfile::new(4, 2, 0).unwrap();
        let data = (0..=255u8).cycle().take(128 * 1024).collect::<Vec<_>>();
        let oid = "obj01";
        let bucket = "demo";
        let man = encode_and_write(
            mount,
            bucket,
            oid,
            &profile,
            crate::manifest::StorageTier::Hot,
            &data,
        )
        .unwrap();
        assert_eq!(man.shard_count, 6);
        // delete shards 0 and 4 (mix of data + parity)
        fs::remove_file(shard_path(mount, bucket, oid, 0)).unwrap();
        fs::remove_file(shard_path(mount, bucket, oid, 4)).unwrap();
        let job = RebuildJob::new(mount, bucket, oid, vec![0, 4]);
        let written = job.run().unwrap();
        assert_eq!(written, 2);
        // check rebuilt shards readable + manifest updated with agg crc
        assert!(shard_path(mount, bucket, oid, 0).exists());
        assert!(shard_path(mount, bucket, oid, 4).exists());
        let new = job.load_manifest().unwrap();
        assert_ne!(new.crc64, man.crc64); // switched to aggregate crc64
    }
}
